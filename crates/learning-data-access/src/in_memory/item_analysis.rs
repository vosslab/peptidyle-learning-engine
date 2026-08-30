//! In-memory current course item-analysis projection.

use std::collections::BTreeMap;

use async_trait::async_trait;
use domain::item_analysis::{
    AssignmentItemAnalysis, CourseItemAnalysisReport, ItemAnalysisMetricInput,
    calculate_item_analysis_metrics,
};
use question_model::{
    AssignmentId, AssignmentItemId, AssignmentRun, AttemptStatus, CourseId, CourseMembershipRole,
    ProblemVersionRef, ScoringGeneration, ScoringStatus, StudentClassStatistics,
    SubmissionEvaluationStatus, UserId,
};

use super::*;
use crate::{ActorContext, SessionSubject};

#[derive(Debug, Clone)]
struct ItemAggregate {
    graded_credits: Vec<f64>,
    graded_correct: Vec<bool>,
    rest_of_run_credits: Vec<f64>,
    unanswered_attempt_count: u32,
    unscored_attempt_count: u32,
    completion_times_millis: Vec<u64>,
}

impl ItemAggregate {
    fn new() -> Self {
        Self {
            graded_credits: Vec::new(),
            graded_correct: Vec::new(),
            rest_of_run_credits: Vec::new(),
            unanswered_attempt_count: 0,
            unscored_attempt_count: 0,
            completion_times_millis: Vec::new(),
        }
    }
}

#[async_trait]
impl crate::CourseItemAnalysisStore for MemoryStore {
    async fn course_item_analysis(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        course: CourseId,
        assignment: AssignmentId,
    ) -> Result<Option<CourseItemAnalysisReport>, StoreError> {
        let state = self.read_state()?;
        let Some(assignment_record) = state.assignments.get(&assignment) else {
            return Ok(None);
        };
        let Some(subject) = active_analysis_session(&state, context, session) else {
            return Ok(None);
        };
        let authorized = state.courses.contains_key(&course)
            && super::entitlement::current_course_role(&state, course, subject.user())
                == Some(CourseMembershipRole::Instructor);
        if assignment_record.course_id != course
            || !course_records_accessible(&state, course)
            || !authorized
        {
            return Ok(None);
        }
        let Some(mut report) = state.item_analysis.get(&assignment).cloned() else {
            return Ok(None);
        };
        let Some((generation, status)) = state.assignment_scoring.get(&assignment) else {
            return Err(StoreError::NotFound);
        };
        report.recent_rescoring =
            *generation != report.source_scoring_generation || *status != ScoringStatus::Current;
        Ok(Some(report))
    }

    async fn student_class_statistics(
        &self,
        context: ActorContext,
        student: UserId,
        course: CourseId,
        assignment: AssignmentId,
    ) -> Result<StudentClassStatistics, StoreError> {
        let state = self.read_state()?;
        super::entitlement::require_current_assignment_entitlement(
            &state, student, course, assignment,
        )?;
        require_course_records_accessible(&state, course)?;
        let Some(mut report) = state.item_analysis.get(&assignment).cloned() else {
            return Ok(StudentClassStatistics::InsufficientEvidence);
        };
        let Some((generation, status)) = state.assignment_scoring.get(&assignment) else {
            return Ok(StudentClassStatistics::InsufficientEvidence);
        };
        report.recent_rescoring =
            *generation != report.source_scoring_generation || *status != ScoringStatus::Current;
        Ok(StudentClassStatistics::from_current_analysis(
            report.completed_run_count,
            report.incomplete_scoring,
            report.recent_rescoring,
            report.assignment_average_score,
        ))
    }
}

fn active_analysis_session(
    state: &State,
    context: ActorContext,
    session: SessionTokenHash,
) -> Option<&SessionSubject> {
    super::sessions::active_subject(state, context, session)
}

#[async_trait]
impl crate::CourseItemAnalysisWorkerStore for MemoryStore {
    async fn prepare_course_item_analysis(
        &self,
        command: crate::CourseItemAnalysisWorkerCommand,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let expected = JobPayload::RecalculateCourseItemAnalysis {
            assignment: command.assignment,
            generation: command.generation,
        };
        let claimed = state.jobs.get(&command.job).is_some_and(|job| {
            (job.state == JobState::Leased
                && job.lease_token == Some(command.lease)
                && job
                    .lease_expires_at
                    .is_some_and(|expiry| expiry > state.authoritative_time)
                && job.payload == expected)
        });
        if !claimed {
            return Err(StoreError::Conflict);
        }
        if state.assignment_scoring.get(&command.assignment)
            != Some(&(command.generation, ScoringStatus::Current))
        {
            return Ok(());
        }
        let report =
            build_memory_course_item_analysis(&state, command.assignment, command.generation)?;
        state.item_analysis_staging.insert(
            command.job,
            PreparedCourseItemAnalysis {
                assignment: command.assignment,
                generation: command.generation,
                report,
            },
        );
        Ok(())
    }

    async fn commit_course_item_analysis(
        &self,
        command: crate::CourseItemAnalysisWorkerCommand,
    ) -> Result<crate::CourseItemAnalysisCommitOutcome, StoreError> {
        let mut state = self.write_state()?;
        let expected = JobPayload::RecalculateCourseItemAnalysis {
            assignment: command.assignment,
            generation: command.generation,
        };
        let claimed = state.jobs.get(&command.job).is_some_and(|job| {
            (job.state == JobState::Leased
                && job.lease_token == Some(command.lease)
                && job
                    .lease_expires_at
                    .is_some_and(|expiry| expiry > state.authoritative_time)
                && job.payload == expected)
        });
        if !claimed {
            return Ok(crate::CourseItemAnalysisCommitOutcome::ClaimNoLongerActive);
        }
        let current = state
            .assignment_scoring
            .get(&command.assignment)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current != (command.generation, ScoringStatus::Current) {
            state.item_analysis_staging.remove(&command.job);
            super::queue::complete_memory_job(&mut state, command.job)?;
            return Ok(crate::CourseItemAnalysisCommitOutcome::Superseded);
        }
        let prepared = state
            .item_analysis_staging
            .remove(&command.job)
            .ok_or(StoreError::Conflict)?;
        if prepared.assignment != command.assignment || prepared.generation != command.generation {
            return Err(StoreError::Conflict);
        }
        state
            .item_analysis
            .insert(command.assignment, prepared.report);
        super::queue::complete_memory_job(&mut state, command.job)?;
        Ok(crate::CourseItemAnalysisCommitOutcome::Committed)
    }
}

fn build_memory_course_item_analysis(
    state: &State,
    assignment_id: AssignmentId,
    generation: ScoringGeneration,
) -> Result<CourseItemAnalysisReport, StoreError> {
    let assignment = state
        .assignments
        .get(&assignment_id)
        .ok_or(StoreError::NotFound)?;
    if state.assignment_scoring.get(&assignment_id) != Some(&(generation, ScoringStatus::Current)) {
        return Err(StoreError::Conflict);
    }
    let mut aggregates = BTreeMap::<(AssignmentItemId, ProblemVersionRef), ItemAggregate>::new();
    let mut completed_scores = Vec::new();
    let mut completion_times = Vec::new();
    let mut completed_run_count = 0_u32;
    let mut in_progress_run_count = 0_u32;
    for enrollment in state
        .enrollments
        .values()
        .filter(|record| record.assignment == assignment_id)
    {
        let Some(run) = latest_run_for_enrollment(state, enrollment.id) else {
            continue;
        };
        if run_is_active(state, run)? {
            in_progress_run_count = in_progress_run_count.saturating_add(1);
            continue;
        }
        completed_run_count = completed_run_count.saturating_add(1);
        let completion_millis = terminal_run_elapsed_millis(state, run)?;
        if run.completed_at.is_some()
            && let Some(score) = run.score.filter(|score| score.is_finite())
        {
            completed_scores.push(score);
        }
        if let Some(elapsed) = completion_millis {
            completion_times.push(elapsed);
        }
        aggregate_run(state, run, completion_millis, &mut aggregates)?;
    }
    let analyzed_at = state.authoritative_time;
    let mut incomplete_scoring = false;
    let items = aggregates
        .into_iter()
        .map(
            |((assignment_item, reference), aggregate)| -> Result<AssignmentItemAnalysis, StoreError> {
                incomplete_scoring |= aggregate.unscored_attempt_count > 0;
                let metrics = calculate_item_analysis_metrics(&ItemAnalysisMetricInput {
                    graded_credits: aggregate.graded_credits,
                    graded_correct: aggregate.graded_correct,
                    rest_of_run_credits: aggregate.rest_of_run_credits,
                    unanswered_attempt_count: aggregate.unanswered_attempt_count,
                    unscored_attempt_count: aggregate.unscored_attempt_count,
                    completion_times_millis: aggregate.completion_times_millis,
                })
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
                Ok(AssignmentItemAnalysis {
                    course: assignment.course_id,
                    assignment: assignment_id,
                    assignment_item,
                    reference,
                    source_scoring_generation: generation,
                    analyzed_at,
                    graded_attempt_count: metrics.graded_attempt_count,
                    unanswered_attempt_count: metrics.response_distribution.unanswered,
                    unscored_attempt_count: aggregate.unscored_attempt_count,
                    difficulty: metrics.difficulty,
                    average_credit: metrics.average_credit,
                    credit_standard_deviation: metrics.credit_standard_deviation,
                    discrimination: metrics.discrimination,
                    response_distribution: metrics.response_distribution,
                    average_completion_time_millis: metrics.average_completion_time_millis,
                })
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CourseItemAnalysisReport {
        course: assignment.course_id,
        assignment: assignment_id,
        source_scoring_generation: generation,
        analyzed_at,
        completed_run_count,
        in_progress_run_count,
        incomplete_scoring,
        recent_rescoring: false,
        assignment_average_score: (!incomplete_scoring)
            .then(|| mean(&completed_scores))
            .flatten(),
        average_completion_time_millis: mean_u64(&completion_times),
        items,
    })
}

fn latest_run_for_enrollment(state: &State, enrollment: EnrollmentId) -> Option<&AssignmentRun> {
    state
        .runs
        .values()
        .filter(|run| run.enrollment == enrollment)
        .max_by_key(|run| (run.run_number, run.id))
}

/// A terminal cohort may still contain a pending automated evaluation. Only a
/// missing or actively open current attempt suppresses prior runs from the
/// current-only analysis cohort.
fn run_is_active(state: &State, run: &AssignmentRun) -> Result<bool, StoreError> {
    let items = state
        .run_items
        .get(&run.id)
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    Ok(items.iter().any(|item| {
        latest_current_attempt(state, run.id, item.issued_position)
            .is_none_or(|attempt| attempt.status == AttemptStatus::InProgress)
    }))
}

fn aggregate_run(
    state: &State,
    run: &AssignmentRun,
    completion_millis: Option<u64>,
    aggregates: &mut BTreeMap<(AssignmentItemId, ProblemVersionRef), ItemAggregate>,
) -> Result<(), StoreError> {
    let items = state
        .run_items
        .get(&run.id)
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    let observations = items
        .iter()
        .map(|item| {
            let attempt = latest_current_attempt(state, run.id, item.issued_position);
            classify_item_observation(state, attempt.as_ref())
                .map(|observation| (item, observation))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let total_credit = observations
        .iter()
        .filter_map(|(_, observation)| match observation {
            ItemObservation::Graded(credit, _) => Some(credit),
            ItemObservation::Excluded | ItemObservation::Unscored | ItemObservation::Unanswered => {
                None
            }
        })
        .sum::<f64>();
    for (item, observation) in observations {
        let aggregate = aggregates
            .entry((item.assignment_item, item.reference))
            .or_insert_with(ItemAggregate::new);
        match observation {
            ItemObservation::Excluded => {}
            ItemObservation::Unanswered => {
                aggregate.unanswered_attempt_count =
                    aggregate.unanswered_attempt_count.saturating_add(1)
            }
            ItemObservation::Unscored => {
                aggregate.unscored_attempt_count =
                    aggregate.unscored_attempt_count.saturating_add(1);
            }
            ItemObservation::Graded(credit, correct) => {
                aggregate.graded_credits.push(credit);
                aggregate.graded_correct.push(correct);
                aggregate.rest_of_run_credits.push(total_credit - credit);
            }
        }
        if let Some(elapsed) = completion_millis {
            aggregate.completion_times_millis.push(elapsed);
        }
    }
    Ok(())
}

fn latest_current_attempt(
    state: &State,
    run: question_model::RunId,
    position: u32,
) -> Option<QuestionAttempt> {
    state
        .attempts
        .values()
        .filter(|attempt| attempt.run == run && attempt.assignment_position == position)
        .map(|attempt| projected_attempt(state, attempt))
        .max_by_key(|attempt| (attempt.timer.issued_at, attempt.id))
}

enum ItemObservation {
    Excluded,
    Graded(f64, bool),
    Unscored,
    Unanswered,
}

/// Derives one closed item-analysis observation from current attempt state,
/// canonical automated evaluation state, and immutable result evidence.
fn classify_item_observation(
    state: &State,
    attempt: Option<&QuestionAttempt>,
) -> Result<ItemObservation, StoreError> {
    let Some(attempt) = attempt else {
        return Ok(ItemObservation::Unanswered);
    };
    if matches!(
        attempt.status,
        AttemptStatus::Cleared | AttemptStatus::Exempt
    ) {
        return Ok(ItemObservation::Excluded);
    }
    if !matches!(
        attempt.status,
        AttemptStatus::Submitted | AttemptStatus::AutoSubmitted
    ) {
        return Err(StoreError::Unavailable(
            "non-terminal attempt entered automated item analysis".to_string(),
        ));
    }
    // A forced terminal submission has no response to grade. It remains an
    // intentional unanswered aggregate observation rather than a fabricated
    // automated evaluation tuple.
    if attempt.status == AttemptStatus::AutoSubmitted && attempt.response.is_none() {
        return Ok(ItemObservation::Unanswered);
    }
    let evaluation = state
        .automated_grading_evaluations
        .get(&attempt.id)
        .ok_or_else(|| {
            StoreError::Unavailable(
                "submitted attempt lacks automated evaluation state".to_string(),
            )
        })?;
    let execution = state
        .automated_grading_executions
        .get(&attempt.id)
        .ok_or_else(|| {
            StoreError::Unavailable("submitted attempt lacks automated execution state".to_string())
        })?;
    match (execution.state, evaluation) {
        (
            crate::GradingExecutionState::Ready
            | crate::GradingExecutionState::Running
            | crate::GradingExecutionState::RetryWait,
            SubmissionEvaluationStatus::AutomatedPending,
        )
        | (
            crate::GradingExecutionState::Exception,
            SubmissionEvaluationStatus::AutomatedException,
        ) => {
            return Ok(ItemObservation::Unscored);
        }
        (_, SubmissionEvaluationStatus::Exempt) => {
            return Err(StoreError::Unavailable(
                "submitted attempt has exempt automated evaluation state".to_string(),
            ));
        }
        (crate::GradingExecutionState::Completed, SubmissionEvaluationStatus::Graded) => {}
        _ => {
            return Err(StoreError::Unavailable(
                "automated execution and evaluation states are inconsistent".to_string(),
            ));
        }
    }
    let evidence = state
        .automated_grading_result_evidence
        .get(&attempt.id)
        .ok_or_else(|| {
            StoreError::Unavailable(
                "graded automated evaluation lacks immutable result evidence".to_string(),
            )
        })?;
    let result = attempt.result.ok_or_else(|| {
        StoreError::Unavailable("graded automated evaluation lacks result projection".to_string())
    })?;
    if evidence.result != result {
        return Err(StoreError::Unavailable(
            "automated evaluation result projection is inconsistent".to_string(),
        ));
    }
    if !result.points_earned.is_finite() || !result.points_possible.is_finite() {
        return Err(StoreError::Unavailable(
            "automated evaluation result is invalid".to_string(),
        ));
    }
    let credit = if result.points_possible > 0.0 {
        result.points_earned / result.points_possible
    } else if result.correct {
        1.0
    } else {
        0.0
    };
    if !credit.is_finite() || !(-1_000.0..=1_000.0).contains(&credit) {
        return Err(StoreError::Unavailable(
            "automated evaluation credit is invalid".to_string(),
        ));
    }
    Ok(ItemObservation::Graded(credit, result.correct))
}

fn terminal_run_elapsed_millis(
    state: &State,
    run: &AssignmentRun,
) -> Result<Option<u64>, StoreError> {
    let items = state
        .run_items
        .get(&run.id)
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    let submitted_at = items
        .iter()
        .filter_map(|item| {
            latest_current_attempt(state, run.id, item.issued_position)
                .and_then(|attempt| attempt.timer.submitted_at)
        })
        .max();
    let Some(submitted_at) = submitted_at else {
        return Ok(None);
    };
    Ok(submitted_at
        .as_unix_millis()
        .checked_sub(run.started_at.as_unix_millis())
        .and_then(|elapsed| u64::try_from(elapsed).ok()))
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn mean_u64(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let total = values.iter().map(|value| u128::from(*value)).sum::<u128>();
    u64::try_from(total / values.len() as u128).ok()
}

/// Reserves one durable transactional-outbox job before scoring publication.
/// The worker later computes analytics asynchronously, so grading never waits
/// for analysis while a successful publication cannot lose its rebuild work.
pub(super) fn allocate_course_item_analysis_job(state: &State) -> Result<crate::JobId, StoreError> {
    for _ in 0..16 {
        let job = crate::JobId::generate()?;
        if !state.jobs.contains_key(&job) {
            return Ok(job);
        }
    }
    Err(StoreError::Unavailable(
        "could not allocate a unique item-analysis job".to_string(),
    ))
}

pub(super) fn enqueue_course_item_analysis_after_scoring(
    state: &mut State,
    job: crate::JobId,
    assignment: AssignmentId,
    generation: ScoringGeneration,
) -> Result<(), StoreError> {
    if state.jobs.contains_key(&job) {
        return Err(StoreError::Conflict);
    }
    state.jobs.insert(
        job,
        StoredJob {
            payload: JobPayload::RecalculateCourseItemAnalysis {
                assignment,
                generation,
            },
            state: JobState::Ready,
            available_at: state.authoritative_time,
            lease_token: None,
            lease_expires_at: None,
            attempt_count: 0,
            max_attempts: 10,
            failure: None,
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::in_memory::grading_execution_worker::completion_tests::seed_complete_issued_execution;
    use crate::{
        CourseItemAnalysisWorkerCommand, CourseItemAnalysisWorkerStore, JobClaimFilter,
        JobLeaseDuration,
    };
    use question_model::{ActivityTimestamp, AttemptResult, StudentResponse};
    use uuid::Uuid;

    #[tokio::test]
    async fn response_bearing_incoherent_automated_tuple_cannot_stage_or_publish_analysis() {
        let store = MemoryStore::default();
        let (_scope, _student, attempt, _) = seed_complete_issued_execution(&store);
        let assignment = AssignmentId::from_uuid(Uuid::from_u128(75_004));
        let run = RunId::from_uuid(Uuid::from_u128(75_006));
        let generation = ScoringGeneration::INITIAL;
        let job = JobId::from_uuid(Uuid::from_u128(75_099));

        {
            let mut state = store
                .write_state()
                .expect("inject contradictory Memory state");
            {
                let attempt = state.attempts.get_mut(&attempt).expect("issued attempt");
                attempt.status = AttemptStatus::Submitted;
                attempt.response = Some(StudentResponse::Numeric { value: 42.0 });
                attempt.result = Some(AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                });
                attempt.timer.submitted_at = Some(ActivityTimestamp::from_unix_millis(1_001));
            }
            state.runs.get_mut(&run).expect("issued run").completed_at =
                Some(ActivityTimestamp::from_unix_millis(1_001));
            state
                .automated_grading_evaluations
                .insert(attempt, SubmissionEvaluationStatus::Graded);
            enqueue_course_item_analysis_after_scoring(&mut state, job, assignment, generation)
                .expect("queue item-analysis rebuild");
        }

        let claim = store
            .claim_next_job(
                &JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).expect("bounded test lease"),
            )
            .await
            .expect("claim item-analysis job")
            .expect("queued item-analysis job");
        let command = CourseItemAnalysisWorkerCommand {
            job,
            lease: claim.lease_token,
            assignment,
            generation,
        };
        assert!(matches!(
            store.prepare_course_item_analysis(command).await,
            Err(StoreError::Unavailable(_))
        ));
        let state = store.read_state().expect("verify no report was published");
        assert!(!state.item_analysis.contains_key(&assignment));
        assert!(!state.item_analysis_staging.contains_key(&job));
    }
}
