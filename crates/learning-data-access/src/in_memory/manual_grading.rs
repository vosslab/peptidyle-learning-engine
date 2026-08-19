//! In-memory implementation of the server-only manual-evaluation contract.

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use objects::Sha256Digest;
use question_model::{
    AttemptResult, AttemptStatus, CourseMembershipRole, FeedbackContent, ScoringStatus,
};

use super::*;
use crate::{
    EvaluationRevision, ManualEvaluationRecord, ManualEvaluationStatus, ManualGradeReceipt,
    ManualGradingStore, SetManualGradeCommand, SubmitPendingManualQuestionAttemptCommand,
    manual_grading::request_digest,
};

/// Minimal private idempotency state. The retained digest proves an exact
/// retry without retaining any past grade or learner response.
#[derive(Debug, Clone)]
pub(super) struct MemoryManualGradeReceipt {
    pub(super) actor: UserId,
    pub(super) attempt: QuestionAttemptId,
    pub(super) expected_revision: EvaluationRevision,
    pub(super) resulting_revision: EvaluationRevision,
    pub(super) scoring_generation: ScoringGeneration,
    pub(super) request_sha256: Sha256Digest,
    pub(super) occurred_at: ActivityTimestamp,
}

impl MemoryManualGradeReceipt {
    fn public(&self, action: crate::ManualGradeActionId) -> ManualGradeReceipt {
        ManualGradeReceipt {
            action,
            attempt: self.attempt,
            resulting_revision: self.resulting_revision,
            scoring_generation: self.scoring_generation,
            occurred_at: self.occurred_at,
        }
    }
}

#[async_trait]
impl ManualGradingStore for MemoryStore {
    async fn get_manual_evaluation_with_response_for_edit(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<(ManualEvaluationRecord, StudentResponse)>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let attempt_record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt_record.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        state
            .courses
            .get(&(tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if super::entitlement::current_course_role(&state, tenant, assignment.course_id, actor)
            != Some(CourseMembershipRole::Instructor)
        {
            return Err(StoreError::NotFound);
        }
        let evaluation = state.manual_evaluations.get(&(tenant, attempt)).cloned();
        let response = projected_attempt(&state, tenant, attempt_record).response;
        Ok(evaluation.zip(response))
    }
    async fn submit_pending_manual_question_attempt(
        &self,
        context: TenantContext,
        command: SubmitPendingManualQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        let mut state = self.write_state()?;
        submit_pending_manual_question_attempt_locked(&mut state, context, command)
    }

    async fn get_manual_evaluation_for_edit(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ManualEvaluationRecord>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let attempt_record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt_record.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        state
            .courses
            .get(&(tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if super::entitlement::current_course_role(&state, tenant, assignment.course_id, actor)
            != Some(CourseMembershipRole::Instructor)
        {
            return Err(StoreError::NotFound);
        }
        Ok(state.manual_evaluations.get(&(tenant, attempt)).cloned())
    }

    async fn set_manual_grade(
        &self,
        context: TenantContext,
        command: SetManualGradeCommand,
    ) -> Result<ManualGradeReceipt, StoreError> {
        let mut state = self.write_state()?;
        set_memory_manual_grade(&mut state, context, command)
    }
}

fn submit_pending_manual_question_attempt_locked(
    state: &mut State,
    context: TenantContext,
    command: SubmitPendingManualQuestionAttemptCommand,
) -> Result<SubmissionRecord, StoreError> {
    let tenant = context.tenant_id();
    let base = state
        .attempts
        .get(&(tenant, command.attempt))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    require_attempt_owner(state, tenant, &base, command.actor)?;
    if let Some(matches_request) = state
        .submissions
        .get(&(tenant, command.attempt))
        .map(|stored| stored.key == command.idempotency_key && stored.response == command.response)
    {
        return if matches_request {
            super::runs::load_submission_record(state, tenant, &base)?.ok_or_else(|| {
                StoreError::Unavailable("submission receipt disappeared during replay".to_string())
            })
        } else {
            Err(StoreError::Conflict)
        };
    }
    if projected_attempt(state, tenant, &base).status != AttemptStatus::InProgress {
        return Err(StoreError::Conflict);
    }
    // Validate the issued snapshot before any receipt, attempt, or run
    // mutation. The pending-grade receipt copies it without reconstruction.
    let presentation = super::runs::load_issued_presentation(state, tenant, &base)?;
    let run = state
        .runs
        .get(&(tenant, base.run))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::Conflict);
    }
    let enrollment = enrollment_record(state, tenant, run.enrollment)?;
    let assignment = assignment_record(state, tenant, enrollment.assignment)?;
    require_course_records_accessible(state, tenant, assignment.course_id)?;
    let published = state
        .published
        .get(&(base.problem, base.question_version))
        .ok_or(StoreError::NotFound)?;
    let authored_policy = published.question.timing_policy;
    let submitted_at = state.authoritative_time;
    let mut submitted = projected_attempt(state, tenant, &base);
    submitted.response = Some(command.response.clone());
    submitted.status = AttemptStatus::NeedsManualGrading;
    submitted.result = None;
    submitted.timer.submitted_at = Some(submitted_at);
    let disclosure = super::feedback::current_disclosure_input(
        state,
        tenant,
        &assignment,
        command.attempt,
        submitted.timer.submitted_at,
    )?;
    let effective_policy =
        state
            .attempt_timing
            .get(&(tenant, command.attempt))
            .map_or(authored_policy, |timing| {
                timing
                    .effective_deadline
                    .map_or(TimingPolicy::Untimed, |_| TimingPolicy::PerQuestion {
                        seconds: 1,
                        grace_seconds: timing.effective_grace_seconds,
                    })
            });
    let verdict = timer_verdict(&TimerEvaluation {
        policy: effective_policy,
        timer: submitted.timer,
        evaluated_at: submitted_at,
        pause_extension_millis: 0,
    })
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    if verdict == TimerVerdict::TimedOut {
        return Err(StoreError::TimedOut);
    }
    let previous = state
        .summaries
        .get(&(tenant, enrollment.id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let summary = project_summary(
        &previous,
        domain::scoring::RunTransition::QuestionAttemptRecorded { at: submitted_at },
        grade_policy(&assignment),
    )?;
    let record = SubmissionRecord {
        attempt: submitted.clone(),
        run: run.clone(),
        summary: summary.clone(),
        feedback: private_feedback_record(FeedbackContent::default())?,
        presentation,
        disclosure,
    };
    state.submissions.insert(
        (tenant, command.attempt),
        StoredSubmission {
            key: command.idempotency_key,
            response: command.response,
            record: record.clone(),
        },
    );
    state
        .attempt_current
        .insert((tenant, command.attempt), submitted);
    state.manual_evaluations.insert(
        (tenant, command.attempt),
        ManualEvaluationRecord {
            tenant,
            attempt: command.attempt,
            revision: EvaluationRevision::INITIAL,
            status: ManualEvaluationStatus::NeedsManualGrading,
            credit: None,
            evaluated_at: submitted_at,
        },
    );
    state.runs.insert((tenant, run.id), run);
    state
        .summaries
        .insert((tenant, summary.enrollment), summary);
    complete_memory_attempt_timing_job(state, tenant, command.attempt);
    Ok(record)
}

fn set_memory_manual_grade(
    state: &mut State,
    context: TenantContext,
    command: SetManualGradeCommand,
) -> Result<ManualGradeReceipt, StoreError> {
    let tenant = context.tenant_id();
    let base = state
        .attempts
        .get(&(tenant, command.attempt))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let mut run = state
        .runs
        .get(&(tenant, base.run))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(state, tenant, run.enrollment)?;
    let assignment = assignment_record(state, tenant, enrollment.assignment)?;
    require_course_records_accessible(state, tenant, assignment.course_id)?;
    state
        .courses
        .get(&(tenant, assignment.course_id))
        .ok_or(StoreError::NotFound)?;
    if super::entitlement::current_course_role(state, tenant, assignment.course_id, command.actor)
        != Some(CourseMembershipRole::Instructor)
    {
        return Err(StoreError::NotFound);
    }
    let digest = request_digest(&command);
    if let Some(receipt) = state.manual_grade_actions.get(&(tenant, command.action)) {
        return if receipt.actor == command.actor
            && receipt.attempt == command.attempt
            && receipt.expected_revision == command.expected_revision
            && receipt.request_sha256 == digest
        {
            Ok(receipt.public(command.action))
        } else {
            Err(StoreError::Conflict)
        };
    }
    let prior = state
        .manual_evaluations
        .get(&(tenant, command.attempt))
        .cloned()
        .ok_or(StoreError::Conflict)?;
    let previous_attempt = projected_attempt(state, tenant, &base);
    if previous_attempt.response.is_none()
        || !state.submissions.contains_key(&(tenant, command.attempt))
        || !matches!(
            previous_attempt.status,
            AttemptStatus::NeedsManualGrading | AttemptStatus::Submitted
        )
        || prior.status != ManualEvaluationStatus::NeedsManualGrading
            && prior.status != ManualEvaluationStatus::Graded
    {
        return Err(StoreError::Conflict);
    }
    if prior.revision != command.expected_revision {
        return Err(StoreError::Conflict);
    }
    let correct = command.credit.as_decimal() == &BigDecimal::from(1);
    let credit = command.credit.try_as_f64()?;
    let result = AttemptResult {
        correct,
        points_earned: credit,
        points_possible: 1.0,
    };
    crate::validate_attempt_result(result)?;
    let now = state.authoritative_time;
    let resulting_revision = prior.revision.next()?;
    let record = ManualEvaluationRecord {
        tenant,
        attempt: command.attempt,
        revision: resulting_revision,
        status: ManualEvaluationStatus::Graded,
        credit: Some(command.credit.clone()),
        evaluated_at: now,
    };
    let mut graded = previous_attempt.clone();
    graded.status = AttemptStatus::Submitted;
    graded.result = Some(result);
    let scoring_key = (tenant, assignment.id);
    let (current_generation, _) = state
        .assignment_scoring
        .get(&scoring_key)
        .copied()
        .ok_or(StoreError::NotFound)?;
    let generation = current_generation.next().ok_or(StoreError::Conflict)?;
    let job = loop {
        let candidate = crate::JobId::generate()?;
        if !state.jobs.contains_key(&candidate) {
            break candidate;
        }
    };
    let run_items = state
        .run_items
        .get(&(tenant, run.id))
        .cloned()
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    let attempts = state
        .attempts
        .values()
        .filter(|attempt| attempt.tenant == tenant && attempt.run == run.id)
        .map(|attempt| {
            if attempt.id == graded.id {
                graded.clone()
            } else {
                projected_attempt(state, tenant, attempt)
            }
        })
        .collect::<Vec<_>>();
    let questions = current_run_questions(&assignment, &run_items, &attempts, &graded)?;
    if let Some(score) = completed_run_score(&questions, assignment.policies.completion)? {
        if run.completed_at.is_none() {
            run.completed_at = Some(now);
        }
        run.score = Some(score);
    }
    state.jobs.insert(
        job,
        StoredJob {
            tenant,
            payload: crate::JobPayload::RecalculateAssignment {
                assignment: assignment.id,
                generation,
            },
            state: JobState::Ready,
            available_at: now,
            lease_token: None,
            lease_expires_at: None,
            attempt_count: 0,
            max_attempts: 10,
            failure: None,
        },
    );
    state
        .assignment_scoring
        .insert(scoring_key, (generation, ScoringStatus::Recalculating));
    state.attempt_current.insert((tenant, graded.id), graded);
    state
        .manual_evaluations
        .insert((tenant, command.attempt), record);
    state.manual_grade_actions.insert(
        (tenant, command.action),
        MemoryManualGradeReceipt {
            actor: command.actor,
            attempt: command.attempt,
            expected_revision: command.expected_revision,
            resulting_revision,
            scoring_generation: generation,
            request_sha256: digest,
            occurred_at: now,
        },
    );
    state.runs.insert((tenant, run.id), run);
    Ok(ManualGradeReceipt {
        action: command.action,
        attempt: command.attempt,
        resulting_revision,
        scoring_generation: generation,
        occurred_at: now,
    })
}
