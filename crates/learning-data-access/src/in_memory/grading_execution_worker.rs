//! Sealed accepted-submission worker state machine for Memory conformance.

use async_trait::async_trait;
use question_model::{
    ActivityTimestamp, GradingOperationReason, QuestionAttemptId, SubmissionEvaluationStatus,
    TenantId,
};
#[cfg(test)]
use uuid::Uuid;

use super::*;
use crate::submission_completion::AcceptedSubmissionCompletionPlan;
use crate::{
    AcceptedSubmission, AcceptedSubmissionCommitError, AcceptedSubmissionCompletionInput,
    AcceptedSubmissionExecution, AcceptedSubmissionExecutionClaim,
    AcceptedSubmissionExecutionDisposition, AcceptedSubmissionExecutionFastPathClaimStore,
    AcceptedSubmissionExecutionLoadError, AcceptedSubmissionExecutionOutcome,
    AcceptedSubmissionExecutionRecoveryClaimStore, AcceptedSubmissionExecutionStore,
    AcceptedSubmissionExecutionTarget, GradingExecutionReceipt, JobLeaseDuration, JobPayload,
    JobState, StoreError, TenantContext, WorkerId,
};

struct SuccessfulEvaluationPlan {
    completion: AcceptedSubmissionCompletionPlan,
}

#[async_trait]
impl AcceptedSubmissionExecutionRecoveryClaimStore for MemoryStore {
    async fn claim_next_accepted_submission_execution(
        &self,
        worker: WorkerId,
        lease: JobLeaseDuration,
    ) -> Result<Option<AcceptedSubmissionExecutionClaim>, StoreError> {
        self.claim_accepted_submission_execution(None, worker, lease)
    }
}

#[async_trait]
impl AcceptedSubmissionExecutionFastPathClaimStore for MemoryStore {
    async fn claim_exact_accepted_submission_execution(
        &self,
        target: AcceptedSubmissionExecutionTarget,
        worker: WorkerId,
        lease: JobLeaseDuration,
    ) -> Result<Option<AcceptedSubmissionExecutionClaim>, StoreError> {
        self.claim_accepted_submission_execution(Some(target), worker, lease)
    }
}

#[async_trait]
impl AcceptedSubmissionExecutionStore for MemoryStore {
    async fn load_accepted_submission_for_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
    ) -> Result<AcceptedSubmissionExecution, AcceptedSubmissionExecutionLoadError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        if claim.tenant != tenant {
            return Err(AcceptedSubmissionExecutionLoadError::Conflict);
        }
        let now = state.authoritative_time;
        let job = state
            .jobs
            .get(&claim.job)
            .ok_or(AcceptedSubmissionExecutionLoadError::NotFound)?;
        if job.tenant != tenant
            || job.state != crate::JobState::Leased
            || job.lease_token != Some(claim.lease_token)
            || !job.lease_expires_at.is_some_and(|expiry| expiry > now)
        {
            return Err(AcceptedSubmissionExecutionLoadError::Conflict);
        }
        let crate::JobPayload::GradeAcceptedSubmission {
            attempt,
            submission,
            execution_generation,
        } = job.payload
        else {
            return Err(AcceptedSubmissionExecutionLoadError::Conflict);
        };
        if submission != claim.submission || execution_generation != claim.execution_generation {
            return Err(AcceptedSubmissionExecutionLoadError::Conflict);
        }
        let execution = state
            .automated_grading_executions
            .get(&(tenant, attempt))
            .ok_or(AcceptedSubmissionExecutionLoadError::NotFound)?;
        if execution.submission != claim.submission
            || execution.generation != claim.execution_generation
            || execution.job != claim.job
            || execution.state != crate::GradingExecutionState::Running
            || state
                .automated_grading_execution_workers
                .get(&(tenant, attempt))
                != Some(&claim.worker)
        {
            return Err(AcceptedSubmissionExecutionLoadError::Conflict);
        }
        let stored = state
            .submissions
            .get(&(tenant, attempt))
            .ok_or(AcceptedSubmissionExecutionLoadError::NotFound)?;
        let accepted = stored
            .accepted_pending()
            .ok_or(AcceptedSubmissionExecutionLoadError::Conflict)?;
        if accepted.submission != claim.submission || accepted.tenant != tenant {
            return Err(AcceptedSubmissionExecutionLoadError::Conflict);
        }
        let private = state
            .private_submission_responses
            .get(&(tenant, attempt))
            .ok_or(AcceptedSubmissionExecutionLoadError::IssuedEvidenceIntegrity)?;
        let canonical = crate::canonical_student_response_json(&private.response)
            .map_err(|_| AcceptedSubmissionExecutionLoadError::IssuedEvidenceIntegrity)?;
        if canonical != private.canonical_text
            || objects::Sha256Digest::compute(canonical.as_bytes()) != private.sha256
            || private.sha256 != accepted.request_sha256
        {
            return Err(AcceptedSubmissionExecutionLoadError::IssuedEvidenceIntegrity);
        }
        let prepared =
            super::grading_operations::load_prepared_accepted_submission(&state, tenant, attempt)
                .map_err(|_| AcceptedSubmissionExecutionLoadError::IssuedEvidenceIntegrity)?;
        Ok(AcceptedSubmissionExecution {
            accepted: accepted.clone(),
            response: private.response.clone(),
            prepared: Box::new(prepared),
        })
    }

    async fn commit_or_fail_accepted_submission_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
        outcome: AcceptedSubmissionExecutionOutcome,
    ) -> Result<AcceptedSubmissionExecutionDisposition, AcceptedSubmissionCommitError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let tenant = context.tenant_id();
        if claim.tenant != tenant {
            return Ok(AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive);
        }
        let Some((attempt, accepted)) = active_claim_attempt(&state, tenant, claim, now) else {
            return Ok(AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive);
        };
        let success_plan = match &outcome {
            AcceptedSubmissionExecutionOutcome::Evaluated { grade } => {
                Some(prepare_successful_evaluation(&state, &accepted, grade)?)
            }
            _ => None,
        };
        let terminal_failure = match outcome {
            AcceptedSubmissionExecutionOutcome::TimedOut
            | AcceptedSubmissionExecutionOutcome::TransientFailure => {
                crate::JobFailureKind::TimedOut
            }
            _ => crate::JobFailureKind::Permanent,
        };
        let outcome_reason = match &outcome {
            AcceptedSubmissionExecutionOutcome::DeterministicFailure { reason } => Some(*reason),
            AcceptedSubmissionExecutionOutcome::TransientFailure
            | AcceptedSubmissionExecutionOutcome::TimedOut => {
                Some(GradingOperationReason::RetryExhausted)
            }
            AcceptedSubmissionExecutionOutcome::TerminalFailure => {
                Some(GradingOperationReason::GraderExecutionFailure)
            }
            AcceptedSubmissionExecutionOutcome::Evaluated { .. } => None,
        };
        let (state_after, evaluation, disposition, evidence, safe_category) = match outcome {
            AcceptedSubmissionExecutionOutcome::Evaluated { grade } => (
                crate::GradingExecutionState::Completed,
                SubmissionEvaluationStatus::Graded,
                AcceptedSubmissionExecutionDisposition::Committed,
                Some(grade.evidence),
                crate::GradingExecutionReceiptSafeCategory::Graded,
            ),
            AcceptedSubmissionExecutionOutcome::DeterministicFailure { reason } => (
                crate::GradingExecutionState::Exception,
                SubmissionEvaluationStatus::AutomatedException,
                AcceptedSubmissionExecutionDisposition::Terminal,
                None,
                execution_failure_category(reason),
            ),
            AcceptedSubmissionExecutionOutcome::TerminalFailure => (
                crate::GradingExecutionState::Exception,
                SubmissionEvaluationStatus::AutomatedException,
                AcceptedSubmissionExecutionDisposition::Terminal,
                None,
                crate::GradingExecutionReceiptSafeCategory::GraderExecutionFailure,
            ),
            AcceptedSubmissionExecutionOutcome::TransientFailure
            | AcceptedSubmissionExecutionOutcome::TimedOut => {
                let exhausted = state
                    .jobs
                    .get(&claim.job)
                    .is_some_and(|job| job.attempt_count >= job.max_attempts);
                if exhausted {
                    (
                        crate::GradingExecutionState::Exception,
                        SubmissionEvaluationStatus::AutomatedException,
                        AcceptedSubmissionExecutionDisposition::Terminal,
                        None,
                        crate::GradingExecutionReceiptSafeCategory::RetryExhausted,
                    )
                } else {
                    (
                        crate::GradingExecutionState::RetryWait,
                        SubmissionEvaluationStatus::AutomatedPending,
                        AcceptedSubmissionExecutionDisposition::Rescheduled,
                        None,
                        crate::GradingExecutionReceiptSafeCategory::DependencyRetry,
                    )
                }
            }
        };
        let terminal_reason = (disposition == AcceptedSubmissionExecutionDisposition::Terminal)
            .then(|| outcome_reason.expect("terminal outcome reason"));
        let retry_available_at =
            if disposition == AcceptedSubmissionExecutionDisposition::Rescheduled {
                let attempt_count = state
                    .jobs
                    .get(&claim.job)
                    .ok_or(StoreError::NotFound)?
                    .attempt_count;
                Some(super::queue::add_job_seconds(
                    now,
                    super::queue::retry_delay_seconds(attempt_count),
                )?)
            } else {
                None
            };
        let committed = disposition == AcceptedSubmissionExecutionDisposition::Committed;
        if committed {
            if let Some(contributions) = success_plan
                .as_ref()
                .expect("evaluated plan")
                .completion
                .statistics
                .as_deref()
            {
                super::stage_statistics_contributions(
                    &mut state,
                    accepted.tenant,
                    success_plan
                        .as_ref()
                        .expect("evaluated plan")
                        .completion
                        .enrollment
                        .id,
                    success_plan
                        .as_ref()
                        .expect("evaluated plan")
                        .completion
                        .receipt
                        .run
                        .id,
                    contributions,
                )?;
            }
            apply_successful_evaluation(
                &mut state,
                &accepted,
                success_plan.expect("evaluated plan"),
            )?;
        }
        let execution = state
            .automated_grading_executions
            .get_mut(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        execution.state = state_after;
        state
            .automated_grading_evaluations
            .insert((tenant, attempt), evaluation);
        state
            .automated_grading_execution_workers
            .remove(&(tenant, attempt));
        if let Some(evidence) = evidence {
            state
                .automated_grading_result_evidence
                .insert((tenant, attempt), evidence);
        }
        let job = state
            .jobs
            .get_mut(&claim.job)
            .expect("active claim retains its leased job");
        job.lease_token = None;
        job.lease_expires_at = None;
        match disposition {
            AcceptedSubmissionExecutionDisposition::Committed => {
                job.state = JobState::Completed;
            }
            AcceptedSubmissionExecutionDisposition::Rescheduled => {
                job.state = JobState::Ready;
                job.available_at = retry_available_at.expect("prepared retry availability");
            }
            AcceptedSubmissionExecutionDisposition::Terminal => {
                job.state = JobState::Dead;
                job.failure = Some(terminal_failure);
            }
            AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive => unreachable!(),
        }
        let _ = job;
        if committed {
            super::grading_operation_lifecycle::close_completed_submission_operation(
                &mut state,
                tenant,
                accepted.submission,
            )?;
        }
        if let Some(reason) = terminal_reason {
            super::grading_operation_lifecycle::reopen_submission_operation(
                &mut state,
                tenant,
                accepted.course,
                accepted.assignment,
                accepted.submission,
                reason,
            )?;
        }
        state
            .automated_grading_execution_receipts
            .entry((tenant, attempt))
            .or_default()
            .push(GradingExecutionReceipt {
                submission: claim.submission,
                generation: claim.execution_generation,
                resulting_state: state_after,
                safe_category,
                actor: None,
                worker: Some(claim.worker),
                occurred_at: now,
            });
        Ok(disposition)
    }
}

fn prepare_successful_evaluation(
    state: &State,
    accepted: &AcceptedSubmission,
    grade: &crate::AcceptedSubmissionGrade,
) -> Result<SuccessfulEvaluationPlan, StoreError> {
    let expected = crate::canonical_attempt_result_json(grade.evidence.result)?;
    if grade.evidence.canonical_json_version != expected.canonical_json_version
        || grade.evidence.canonical_json != expected.canonical_json
        || grade.evidence.sha256 != expected.sha256
    {
        return Err(StoreError::InvalidRecord(
            "automated result evidence is not the canonical typed result".to_string(),
        ));
    }
    crate::validate_attempt_result(grade.evidence.result)?;
    let base = state
        .attempts
        .get(&(accepted.tenant, accepted.attempt))
        .ok_or(StoreError::NotFound)?;
    let submitted = projected_attempt(state, accepted.tenant, base);
    let run = state
        .runs
        .get(&(accepted.tenant, submitted.run))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::Conflict);
    }
    let enrollment = enrollment_record(state, accepted.tenant, run.enrollment)?;
    let assignment = assignment_record(state, accepted.tenant, enrollment.assignment)?;
    if assignment.id != accepted.assignment || assignment.course_id != accepted.course {
        return Err(StoreError::Conflict);
    }
    let summary = state
        .summaries
        .get(&(accepted.tenant, enrollment.id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let run_items = state
        .run_items
        .get(&(accepted.tenant, run.id))
        .cloned()
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    let attempts = state
        .attempts
        .values()
        .filter(|attempt| attempt.tenant == accepted.tenant && attempt.run == run.id)
        .map(|attempt| {
            if attempt.id == submitted.id {
                submitted.clone()
            } else {
                projected_attempt(state, accepted.tenant, attempt)
            }
        })
        .collect::<Vec<_>>();
    let completion =
        crate::plan_accepted_submission_completion(AcceptedSubmissionCompletionInput {
            base_attempt: submitted,
            grade: grade.clone(),
            assignment: assignment.clone(),
            run,
            enrollment,
            previous_summary: summary,
            run_items,
            attempts,
            accepted_at: accepted.accepted_at,
            presentation: super::runs::load_issued_presentation(state, accepted.tenant, base)?,
        })?;
    Ok(SuccessfulEvaluationPlan { completion })
}

fn apply_successful_evaluation(
    state: &mut State,
    accepted: &AcceptedSubmission,
    plan: SuccessfulEvaluationPlan,
) -> Result<(), StoreError> {
    super::scoring_invalidation::request_scoring_invalidation(
        state,
        accepted.tenant,
        accepted.course,
        accepted.assignment,
        crate::ScoringInvalidationOrigin::accepted_submission_completion(
            crate::ScoringInvalidationOriginId::from_uuid(accepted.submission.as_uuid()),
        ),
        crate::accepted_submission_recalculation_job(accepted.submission),
    )?;
    state.attempt_current.insert(
        (accepted.tenant, accepted.attempt),
        plan.completion.receipt.attempt.clone(),
    );
    state.runs.insert(
        (accepted.tenant, plan.completion.receipt.run.id),
        plan.completion.receipt.run.clone(),
    );
    state.enrollments.insert(
        (accepted.tenant, plan.completion.enrollment.id),
        plan.completion.enrollment,
    );
    state.summaries.insert(
        (accepted.tenant, plan.completion.receipt.summary.enrollment),
        plan.completion.receipt.summary.clone(),
    );
    let stored = state
        .submissions
        .get_mut(&(accepted.tenant, accepted.attempt))
        .expect("active accepted claim retains its submission receipt");
    stored.state = StoredSubmissionState::Completed(Box::new(plan.completion.receipt));
    Ok(())
}

pub(super) fn converge_expired_exhausted_claims(
    state: &mut State,
    now: ActivityTimestamp,
) -> Result<(), StoreError> {
    let exhausted = state
        .jobs
        .iter()
        .filter_map(|(job_id, job)| {
            let JobPayload::GradeAcceptedSubmission {
                attempt,
                submission,
                execution_generation,
            } = job.payload
            else {
                return None;
            };
            (job.state == JobState::Leased
                && job.lease_expires_at.is_some_and(|expiry| expiry <= now)
                && job.attempt_count >= job.max_attempts)
                .then_some((
                    *job_id,
                    job.tenant,
                    attempt,
                    submission,
                    execution_generation,
                ))
        })
        .collect::<Vec<_>>();
    for (job_id, tenant, attempt, submission, generation) in exhausted {
        let accepted = state
            .submissions
            .get(&(tenant, attempt))
            .and_then(StoredSubmission::accepted_pending)
            .cloned();
        let Some(execution) = state
            .automated_grading_executions
            .get_mut(&(tenant, attempt))
        else {
            continue;
        };
        if execution.submission != submission
            || execution.generation != generation
            || execution.job != job_id
            || execution.state != crate::GradingExecutionState::Running
        {
            continue;
        }
        let worker = state
            .automated_grading_execution_workers
            .remove(&(tenant, attempt))
            .ok_or(StoreError::Conflict)?;
        execution.state = crate::GradingExecutionState::Exception;
        state.automated_grading_evaluations.insert(
            (tenant, attempt),
            SubmissionEvaluationStatus::AutomatedException,
        );
        if let Some(job) = state.jobs.get_mut(&job_id) {
            job.state = JobState::Dead;
            job.lease_token = None;
            job.lease_expires_at = None;
            job.failure = Some(crate::JobFailureKind::TimedOut);
        }
        state
            .automated_grading_execution_receipts
            .entry((tenant, attempt))
            .or_default()
            .push(GradingExecutionReceipt {
                submission,
                generation,
                resulting_state: crate::GradingExecutionState::Exception,
                safe_category: crate::GradingExecutionReceiptSafeCategory::RetryExhausted,
                actor: None,
                worker: Some(worker),
                occurred_at: now,
            });
        if let Some(accepted) = accepted {
            super::grading_operation_lifecycle::reopen_submission_operation(
                state,
                tenant,
                accepted.course,
                accepted.assignment,
                submission,
                GradingOperationReason::RetryExhausted,
            )?;
        }
    }
    Ok(())
}

fn execution_failure_category(
    reason: GradingOperationReason,
) -> crate::GradingExecutionReceiptSafeCategory {
    match reason {
        GradingOperationReason::GraderContractFailure => {
            crate::GradingExecutionReceiptSafeCategory::GraderContractFailure
        }
        GradingOperationReason::IssuedEvidenceIntegrity => {
            crate::GradingExecutionReceiptSafeCategory::IssuedEvidenceIntegrity
        }
        GradingOperationReason::GraderExecutionFailure
        | GradingOperationReason::RetryExhausted
        | GradingOperationReason::InstructorRequestedRecalculation
        | GradingOperationReason::ScoringRecalculationRequested
        | GradingOperationReason::ScoringRecalculationFailed => {
            crate::GradingExecutionReceiptSafeCategory::GraderExecutionFailure
        }
    }
}

fn active_claim_attempt(
    state: &State,
    tenant: TenantId,
    claim: AcceptedSubmissionExecutionClaim,
    now: ActivityTimestamp,
) -> Option<(QuestionAttemptId, AcceptedSubmission)> {
    let job = state.jobs.get(&claim.job)?;
    let JobPayload::GradeAcceptedSubmission {
        attempt,
        submission,
        execution_generation,
    } = job.payload
    else {
        return None;
    };
    let execution = state.automated_grading_executions.get(&(tenant, attempt))?;
    let accepted = state
        .submissions
        .get(&(tenant, attempt))?
        .accepted_pending()?;
    (job.tenant == tenant
        && job.state == JobState::Leased
        && job.lease_token == Some(claim.lease_token)
        && job.lease_expires_at.is_some_and(|expiry| expiry > now)
        && submission == claim.submission
        && execution_generation == claim.execution_generation
        && execution.submission == claim.submission
        && execution.generation == claim.execution_generation
        && execution.job == claim.job
        && execution.state == crate::GradingExecutionState::Running
        && state
            .automated_grading_execution_workers
            .get(&(tenant, attempt))
            == Some(&claim.worker)
        && accepted.tenant == tenant
        && accepted.submission == claim.submission)
        .then_some((attempt, accepted.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AcceptedSubmissionExecutionFastPathClaimStore,
        AcceptedSubmissionExecutionRecoveryClaimStore, AcceptedSubmissionExecutionTarget,
        AcceptedSubmissionId, GradingExecution, GradingExecutionGeneration,
    };
    use question_model::UserId;

    fn seed_claimable_execution(
        store: &MemoryStore,
        max_attempts: u16,
    ) -> (TenantId, AcceptedSubmissionId) {
        let tenant = TenantId::from_uuid(Uuid::from_u128(501));
        let attempt = QuestionAttemptId::from_uuid(Uuid::from_u128(502));
        let submission = AcceptedSubmissionId::from_uuid(Uuid::from_u128(503));
        let course = CourseId::from_uuid(Uuid::from_u128(504));
        let assignment = AssignmentId::from_uuid(Uuid::from_u128(505));
        let job = crate::JobId::from_uuid(Uuid::from_u128(506));
        let accepted = AcceptedSubmission {
            tenant,
            course,
            assignment,
            attempt,
            submission,
            actor: UserId::from_uuid(Uuid::from_u128(507)),
            idempotency_key: crate::SubmissionIdempotencyKey::parse("execution-claim")
                .expect("key"),
            request_sha256: objects::Sha256Digest::compute(b"accepted"),
            accepted_at: ActivityTimestamp::from_unix_millis(1_000),
        };
        let mut state = store.write_state().expect("memory state");
        state.authoritative_time = ActivityTimestamp::from_unix_millis(1_000);
        state.submissions.insert(
            (tenant, attempt),
            StoredSubmission {
                key: accepted.idempotency_key.clone(),
                state: StoredSubmissionState::AcceptedPending(accepted),
            },
        );
        state.automated_grading_executions.insert(
            (tenant, attempt),
            GradingExecution {
                submission,
                generation: GradingExecutionGeneration::INITIAL,
                state: crate::GradingExecutionState::Ready,
                job,
                retry_count: 0,
            },
        );
        state.automated_grading_evaluations.insert(
            (tenant, attempt),
            SubmissionEvaluationStatus::AutomatedPending,
        );
        state.assignment_scoring.insert(
            (tenant, assignment),
            (ScoringGeneration::INITIAL, ScoringStatus::Current),
        );
        state.jobs.insert(
            job,
            StoredJob {
                tenant,
                payload: JobPayload::GradeAcceptedSubmission {
                    attempt,
                    submission,
                    execution_generation: GradingExecutionGeneration::INITIAL,
                },
                state: JobState::Ready,
                available_at: ActivityTimestamp::from_unix_millis(1_000),
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts,
                failure: None,
            },
        );
        (tenant, submission)
    }

    fn seeded_target() -> AcceptedSubmissionExecutionTarget {
        AcceptedSubmissionExecutionTarget {
            tenant: TenantId::from_uuid(Uuid::from_u128(501)),
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(502)),
            submission: AcceptedSubmissionId::from_uuid(Uuid::from_u128(503)),
            job: crate::JobId::from_uuid(Uuid::from_u128(506)),
        }
    }

    #[tokio::test]
    async fn generic_and_exact_claims_contend_for_the_same_eligible_execution() {
        let lease = JobLeaseDuration::from_seconds(30).expect("lease");
        let exact_first = MemoryStore::default();
        seed_claimable_execution(&exact_first, 2);
        let target = seeded_target();
        let exact_claim = exact_first
            .claim_exact_accepted_submission_execution(
                target,
                WorkerId::from_uuid(Uuid::from_u128(508)),
                lease,
            )
            .await
            .expect("exact claim")
            .expect("exact winner");
        assert_eq!(
            (exact_claim.tenant, exact_claim.job, exact_claim.submission,),
            (target.tenant, target.job, target.submission)
        );
        assert!(
            exact_first
                .claim_next_accepted_submission_execution(
                    WorkerId::from_uuid(Uuid::from_u128(509)),
                    lease,
                )
                .await
                .expect("recovery contender")
                .is_none()
        );

        let recovery_first = MemoryStore::default();
        seed_claimable_execution(&recovery_first, 2);
        let recovery_claim = recovery_first
            .claim_next_accepted_submission_execution(
                WorkerId::from_uuid(Uuid::from_u128(510)),
                lease,
            )
            .await
            .expect("recovery claim")
            .expect("recovery winner");
        assert_eq!(
            (
                recovery_claim.tenant,
                recovery_claim.job,
                recovery_claim.submission,
            ),
            (target.tenant, target.job, target.submission)
        );
        assert!(
            recovery_first
                .claim_exact_accepted_submission_execution(
                    target,
                    WorkerId::from_uuid(Uuid::from_u128(511)),
                    lease,
                )
                .await
                .expect("exact contender")
                .is_none()
        );
    }

    #[tokio::test]
    async fn claim_commit_and_stale_tuple_are_fenced() {
        let store = MemoryStore::default();
        let (tenant, submission) = seed_claimable_execution(&store, 2);
        let worker = WorkerId::from_uuid(Uuid::from_u128(508));
        let lease = JobLeaseDuration::from_seconds(30).expect("lease");
        let claim = store
            .claim_next_accepted_submission_execution(worker, lease)
            .await
            .expect("claim")
            .expect("winner");
        assert_eq!(claim.tenant, tenant);
        assert!(
            store
                .claim_next_accepted_submission_execution(
                    WorkerId::from_uuid(Uuid::from_u128(509)),
                    lease
                )
                .await
                .expect("second")
                .is_none()
        );
        let mut stale = claim;
        stale.worker = WorkerId::from_uuid(Uuid::from_u128(510));
        assert_eq!(
            store
                .commit_or_fail_accepted_submission_execution(
                    TenantContext::from_authenticated_session(tenant),
                    stale,
                    AcceptedSubmissionExecutionOutcome::TerminalFailure
                )
                .await
                .expect("stale"),
            AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive
        );
        assert_eq!(
            store
                .commit_or_fail_accepted_submission_execution(
                    TenantContext::from_authenticated_session(tenant),
                    claim,
                    AcceptedSubmissionExecutionOutcome::TerminalFailure
                )
                .await
                .expect("commit"),
            AcceptedSubmissionExecutionDisposition::Terminal
        );
        let state = store.read_state().expect("state");
        assert!(
            state
                .automated_grading_executions
                .values()
                .any(|execution| execution.submission == submission
                    && execution.state == crate::GradingExecutionState::Exception)
        );
        assert!(!state.jobs.values().any(|job| matches!(
            job.payload,
            JobPayload::RecalculateAssignment { assignment, .. }
                if assignment == AssignmentId::from_uuid(Uuid::from_u128(505))
        )));
        assert_eq!(
            state
                .assignment_scoring
                .get(&(tenant, AssignmentId::from_uuid(Uuid::from_u128(505)),)),
            Some(&(ScoringGeneration::INITIAL, ScoringStatus::Current))
        );
    }

    #[tokio::test]
    async fn controlled_time_reclaims_then_converges_expired_exhaustion() {
        let store = MemoryStore::default();
        let (tenant, submission) = seed_claimable_execution(&store, 2);
        let worker = WorkerId::from_uuid(Uuid::from_u128(511));
        let lease = JobLeaseDuration::from_seconds(1).expect("lease");
        let first = store
            .claim_next_accepted_submission_execution(worker, lease)
            .await
            .expect("first")
            .expect("claim");
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_001))
            .expect("time");
        let _second = store
            .claim_next_accepted_submission_execution(worker, lease)
            .await
            .expect("reclaim")
            .expect("reclaimed");
        assert_eq!(
            store
                .commit_or_fail_accepted_submission_execution(
                    TenantContext::from_authenticated_session(tenant),
                    first,
                    AcceptedSubmissionExecutionOutcome::TerminalFailure,
                )
                .await
                .expect("expired fence"),
            AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive
        );
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(3_002))
            .expect("time");
        assert!(
            store
                .claim_next_accepted_submission_execution(worker, lease)
                .await
                .expect("exhaust")
                .is_none()
        );
        let state = store.read_state().expect("state");
        assert!(
            state
                .automated_grading_executions
                .values()
                .any(|execution| execution.submission == submission
                    && execution.state == crate::GradingExecutionState::Exception)
        );
        assert!(
            state
                .automated_grading_execution_receipts
                .values()
                .flatten()
                .any(|receipt| receipt.submission == submission
                    && receipt.resulting_state == crate::GradingExecutionState::Exception)
        );
    }
}

#[cfg(test)]
#[path = "grading_execution_worker_completion_tests.rs"]
pub(crate) mod completion_tests;
#[cfg(test)]
#[path = "runs/student_submission_status_tests.rs"]
mod student_submission_status_tests;
