//! In-memory current course item-analysis projection.

use std::collections::BTreeMap;

use async_trait::async_trait;
use domain::item_analysis::{
    AssignmentItemAnalysis, CourseItemAnalysisReport, ItemAnalysisMetricInput,
    calculate_item_analysis_metrics,
};
use question_model::{
    AssignmentId, AssignmentItemId, AssignmentRun, AttemptStatus, CourseId, CourseRole,
    ProblemVersionRef, ScoringGeneration, ScoringStatus, TenantId, UserRole,
};

use super::*;
use crate::SessionSubject;

#[derive(Debug, Clone)]
struct ItemAggregate {
    graded_credits: Vec<f64>,
    graded_correct: Vec<bool>,
    rest_of_run_credits: Vec<f64>,
    unanswered_attempt_count: u32,
    pending_manual_attempt_count: u32,
    completion_times_millis: Vec<u64>,
}

impl ItemAggregate {
    fn new() -> Self {
        Self {
            graded_credits: Vec::new(),
            graded_correct: Vec::new(),
            rest_of_run_credits: Vec::new(),
            unanswered_attempt_count: 0,
            pending_manual_attempt_count: 0,
            completion_times_millis: Vec::new(),
        }
    }
}

#[async_trait]
impl crate::CourseItemAnalysisStore for MemoryStore {
    async fn course_item_analysis(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        assignment: AssignmentId,
    ) -> Result<Option<CourseItemAnalysisReport>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let Some(assignment_record) = state.assignments.get(&(tenant, assignment)) else {
            return Ok(None);
        };
        let Some(subject) = active_analysis_session(&state, context, session) else {
            return Ok(None);
        };
        let authorized = subject.roles().contains(&UserRole::Administrator)
            || state
                .courses
                .get(&(tenant, course))
                .and_then(|record| record.role_for(subject.user()))
                == Some(CourseRole::Instructor);
        if assignment_record.course_id != course
            || !course_records_accessible(&state, tenant, course)
            || !authorized
        {
            return Ok(None);
        }
        let Some(mut report) = state.item_analysis.get(&(tenant, assignment)).cloned() else {
            return Ok(None);
        };
        let Some((generation, status)) = state.assignment_scoring.get(&(tenant, assignment)) else {
            return Err(StoreError::NotFound);
        };
        report.recent_rescoring =
            *generation != report.source_scoring_generation || *status != ScoringStatus::Current;
        Ok(Some(report))
    }
}

fn active_analysis_session(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
) -> Option<&SessionSubject> {
    let stored = state.sessions.get(&session)?;
    (!stored.revoked
        && stored.record.expires_at > state.authoritative_time
        && stored.record.subject.tenant() == context.tenant_id())
    .then_some(&stored.record.subject)
}

#[async_trait]
impl crate::CourseItemAnalysisWorkerStore for MemoryStore {
    async fn prepare_course_item_analysis(
        &self,
        context: TenantContext,
        command: crate::CourseItemAnalysisWorkerCommand,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let expected = JobPayload::RecalculateCourseItemAnalysis {
            assignment: command.assignment,
            generation: command.generation,
        };
        let claim_active = state.jobs.get(&command.job).is_some_and(|job| {
            job.tenant == context.tenant_id()
                && job.state == JobState::Leased
                && job.lease_token == Some(command.lease)
                && job
                    .lease_expires_at
                    .is_some_and(|expiry| expiry > state.authoritative_time)
                && job.payload == expected
        });
        if !claim_active {
            return Err(StoreError::Conflict);
        }
        if state
            .assignment_scoring
            .get(&(context.tenant_id(), command.assignment))
            != Some(&(command.generation, ScoringStatus::Current))
        {
            return Ok(());
        }
        let report = build_memory_course_item_analysis(
            &state,
            context.tenant_id(),
            command.assignment,
            command.generation,
        )?;
        state.item_analysis_staging.insert(
            command.job,
            PreparedCourseItemAnalysis {
                tenant: context.tenant_id(),
                assignment: command.assignment,
                generation: command.generation,
                report,
            },
        );
        Ok(())
    }

    async fn commit_course_item_analysis(
        &self,
        context: TenantContext,
        command: crate::CourseItemAnalysisWorkerCommand,
    ) -> Result<crate::CourseItemAnalysisCommitOutcome, StoreError> {
        let mut state = self.write_state()?;
        let expected = JobPayload::RecalculateCourseItemAnalysis {
            assignment: command.assignment,
            generation: command.generation,
        };
        let claim_active = state.jobs.get(&command.job).is_some_and(|job| {
            job.tenant == context.tenant_id()
                && job.state == JobState::Leased
                && job.lease_token == Some(command.lease)
                && job
                    .lease_expires_at
                    .is_some_and(|expiry| expiry > state.authoritative_time)
                && job.payload == expected
        });
        if !claim_active {
            return Ok(crate::CourseItemAnalysisCommitOutcome::ClaimNoLongerActive);
        }
        let current = state
            .assignment_scoring
            .get(&(context.tenant_id(), command.assignment))
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
        if prepared.tenant != context.tenant_id()
            || prepared.assignment != command.assignment
            || prepared.generation != command.generation
        {
            return Err(StoreError::Conflict);
        }
        state
            .item_analysis
            .insert((context.tenant_id(), command.assignment), prepared.report);
        super::queue::complete_memory_job(&mut state, command.job)?;
        Ok(crate::CourseItemAnalysisCommitOutcome::Committed)
    }
}

fn build_memory_course_item_analysis(
    state: &State,
    tenant: TenantId,
    assignment_id: AssignmentId,
    generation: ScoringGeneration,
) -> Result<CourseItemAnalysisReport, StoreError> {
    let assignment = state
        .assignments
        .get(&(tenant, assignment_id))
        .ok_or(StoreError::NotFound)?;
    if state.assignment_scoring.get(&(tenant, assignment_id))
        != Some(&(generation, ScoringStatus::Current))
    {
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
        .filter(|record| record.tenant == tenant && record.assignment == assignment_id)
    {
        let Some(run) = latest_run_for_enrollment(state, tenant, enrollment.id) else {
            continue;
        };
        if run_is_active(state, tenant, run)? {
            in_progress_run_count = in_progress_run_count.saturating_add(1);
            continue;
        }
        completed_run_count = completed_run_count.saturating_add(1);
        let completion_millis = terminal_run_elapsed_millis(state, tenant, run)?;
        if run.completed_at.is_some()
            && let Some(score) = run.score.filter(|score| score.is_finite())
        {
            completed_scores.push(score);
        }
        if let Some(elapsed) = completion_millis {
            completion_times.push(elapsed);
        }
        aggregate_run(state, tenant, run, completion_millis, &mut aggregates)?;
    }
    let analyzed_at = state.authoritative_time;
    let mut incomplete_manual_grading = false;
    let items = aggregates
        .into_iter()
        .map(
            |((assignment_item, reference), aggregate)| -> Result<AssignmentItemAnalysis, StoreError> {
                incomplete_manual_grading |= aggregate.pending_manual_attempt_count > 0;
                let metrics = calculate_item_analysis_metrics(&ItemAnalysisMetricInput {
                    graded_credits: aggregate.graded_credits,
                    graded_correct: aggregate.graded_correct,
                    rest_of_run_credits: aggregate.rest_of_run_credits,
                    unanswered_attempt_count: aggregate.unanswered_attempt_count,
                    pending_manual_attempt_count: aggregate.pending_manual_attempt_count,
                    completion_times_millis: aggregate.completion_times_millis,
                })
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
                Ok(AssignmentItemAnalysis {
                    tenant,
                    course: assignment.course_id,
                    assignment: assignment_id,
                    assignment_item,
                    reference,
                    source_scoring_generation: generation,
                    analyzed_at,
                    graded_attempt_count: metrics.graded_attempt_count,
                    unanswered_attempt_count: metrics.response_distribution.unanswered,
                    pending_manual_attempt_count: metrics.response_distribution.pending_manual,
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
        tenant,
        course: assignment.course_id,
        assignment: assignment_id,
        source_scoring_generation: generation,
        analyzed_at,
        completed_run_count,
        in_progress_run_count,
        incomplete_manual_grading,
        recent_rescoring: false,
        assignment_average_score: mean(&completed_scores),
        average_completion_time_millis: mean_u64(&completion_times),
        items,
    })
}

fn latest_run_for_enrollment(
    state: &State,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Option<&AssignmentRun> {
    state
        .runs
        .values()
        .filter(|run| run.tenant == tenant && run.enrollment == enrollment)
        .max_by_key(|run| (run.run_number, run.id))
}

/// A terminal cohort may still contain a pending manual evaluation. Only a
/// missing or actively open current attempt suppresses prior runs from the
/// current-only analysis cohort.
fn run_is_active(state: &State, tenant: TenantId, run: &AssignmentRun) -> Result<bool, StoreError> {
    let items = state
        .run_items
        .get(&(tenant, run.id))
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    Ok(items.iter().any(|item| {
        latest_current_attempt(state, tenant, run.id, item.issued_position)
            .is_none_or(|attempt| attempt.status == AttemptStatus::InProgress)
    }))
}

fn aggregate_run(
    state: &State,
    tenant: TenantId,
    run: &AssignmentRun,
    completion_millis: Option<u64>,
    aggregates: &mut BTreeMap<(AssignmentItemId, ProblemVersionRef), ItemAggregate>,
) -> Result<(), StoreError> {
    let items = state
        .run_items
        .get(&(tenant, run.id))
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    let latest_attempts = items
        .iter()
        .map(|item| {
            (
                item.issued_position,
                latest_current_attempt(state, tenant, run.id, item.issued_position),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let graded_observations = latest_attempts
        .values()
        .filter_map(|attempt| {
            attempt
                .as_ref()
                .and_then(|attempt| graded_observation(state, tenant, attempt))
        })
        .collect::<Vec<_>>();
    let total_credit = graded_observations
        .iter()
        .map(|(credit, _)| credit)
        .sum::<f64>();
    for item in items {
        let aggregate = aggregates
            .entry((item.assignment_item, item.reference))
            .or_insert_with(ItemAggregate::new);
        let attempt = latest_attempts
            .get(&item.issued_position)
            .expect("run item positions are unique")
            .clone();
        match attempt.as_ref() {
            None => {
                aggregate.unanswered_attempt_count =
                    aggregate.unanswered_attempt_count.saturating_add(1)
            }
            Some(attempt)
                if matches!(
                    attempt.status,
                    AttemptStatus::Cleared | AttemptStatus::Exempt
                ) => {}
            Some(attempt)
                if attempt.status == AttemptStatus::NeedsManualGrading
                    && state
                        .manual_evaluations
                        .get(&(tenant, attempt.id))
                        .is_some_and(|evaluation| {
                            evaluation.status == crate::ManualEvaluationStatus::NeedsManualGrading
                        }) =>
            {
                aggregate.pending_manual_attempt_count =
                    aggregate.pending_manual_attempt_count.saturating_add(1);
            }
            Some(attempt) => {
                if let Some((credit, correct)) = graded_observation(state, tenant, attempt) {
                    aggregate.graded_credits.push(credit);
                    aggregate.graded_correct.push(correct);
                    aggregate.rest_of_run_credits.push(total_credit - credit);
                } else {
                    aggregate.unanswered_attempt_count =
                        aggregate.unanswered_attempt_count.saturating_add(1);
                }
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
    tenant: TenantId,
    run: question_model::RunId,
    position: u32,
) -> Option<QuestionAttempt> {
    state
        .attempts
        .values()
        .filter(|attempt| {
            attempt.tenant == tenant
                && attempt.run == run
                && attempt.assignment_position == position
        })
        .map(|attempt| projected_attempt(state, tenant, attempt))
        .max_by_key(|attempt| (attempt.timer.issued_at, attempt.id))
}

fn graded_observation(
    state: &State,
    tenant: TenantId,
    attempt: &QuestionAttempt,
) -> Option<(f64, bool)> {
    if !matches!(
        attempt.status,
        AttemptStatus::Submitted | AttemptStatus::AutoSubmitted
    ) {
        return None;
    }
    let result = attempt.result?;
    if !result.points_earned.is_finite() || !result.points_possible.is_finite() {
        return None;
    }
    let manual_credit = state
        .manual_evaluations
        .get(&(tenant, attempt.id))
        .filter(|evaluation| evaluation.status == crate::ManualEvaluationStatus::Graded)
        .and_then(|evaluation| evaluation.credit.as_ref())
        .and_then(|credit| credit.try_as_f64().ok());
    let credit = if let Some(credit) = manual_credit {
        credit
    } else if result.points_possible > 0.0 {
        result.points_earned / result.points_possible
    } else if result.correct {
        1.0
    } else {
        0.0
    };
    (credit.is_finite() && (-1_000.0..=1_000.0).contains(&credit))
        .then_some((credit, result.correct))
}

fn terminal_run_elapsed_millis(
    state: &State,
    tenant: TenantId,
    run: &AssignmentRun,
) -> Result<Option<u64>, StoreError> {
    let items = state
        .run_items
        .get(&(tenant, run.id))
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    let submitted_at = items
        .iter()
        .filter_map(|item| {
            latest_current_attempt(state, tenant, run.id, item.issued_position)
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
    tenant: TenantId,
    assignment: AssignmentId,
    generation: ScoringGeneration,
) -> Result<(), StoreError> {
    if state.jobs.contains_key(&job) {
        return Err(StoreError::Conflict);
    }
    state.jobs.insert(
        job,
        StoredJob {
            tenant,
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
