//! Deterministic in-memory automated-grading acceptance boundary.

use async_trait::async_trait;
use question_model::{
    CourseMembershipRole, GradingOperationReference, IssuedAttemptCapabilityV1,
    SubmissionEvaluationStatus,
};

use super::*;
use crate::{
    AcceptedSubmission, AcceptedSubmissionCommand, AcceptedSubmissionExecution,
    AcceptedSubmissionExecutionClaim, AcceptedSubmissionExecutionStore, AcceptedSubmissionId,
    AutomatedGradingStore, GradingExecution, GradingExecutionGeneration, GradingExecutionReceipt,
    GradingOperation, StoreError, TenantContext,
};

#[async_trait]
impl AutomatedGradingStore for MemoryStore {
    async fn accept_automated_submission(
        &self,
        context: TenantContext,
        command: AcceptedSubmissionCommand,
    ) -> Result<AcceptedSubmission, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let canonical_response = crate::canonical_student_response_json(&command.response)?;
        let response_sha256 = objects::Sha256Digest::compute(canonical_response.as_bytes());
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        if assignment.course_id != command.course
            || assignment.id != command.assignment
            || enrollment.user != command.actor
            || super::entitlement::current_course_role(
                &state,
                tenant,
                command.course,
                command.actor,
            ) != Some(CourseMembershipRole::Student)
        {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, command.course)?;
        if let Some(stored) = state.submissions.get(&(tenant, command.attempt)) {
            let Some(accepted) = stored.accepted_pending() else {
                return Err(StoreError::Conflict);
            };
            let metadata_matches = accepted.actor == command.actor
                && accepted.course == command.course
                && accepted.assignment == command.assignment
                && accepted.idempotency_key == command.idempotency_key
                && accepted.request_sha256 == response_sha256;
            if metadata_matches
                && stored_submission_matches_canonical(
                    &state,
                    tenant,
                    command.attempt,
                    &command.response,
                    &canonical_response,
                    response_sha256,
                )?
            {
                return Ok(accepted.clone());
            }
            return Err(StoreError::Conflict);
        }
        if projected_attempt(&state, tenant, &attempt).status
            != question_model::AttemptStatus::InProgress
        {
            return Err(StoreError::Conflict);
        }
        let accepted_at = state.authoritative_time;
        // The accepted-work record is the common durable timing witness for
        // every worker backend. Validate it before storing private response
        // material so a later evaluator cannot complete work that expired at
        // the server-owned acceptance boundary.
        let timing = state
            .attempt_timing
            .get(&(tenant, command.attempt))
            .ok_or_else(|| {
                StoreError::Unavailable("issued timing authority is missing".to_string())
            })?;
        let effective_policy = timing
            .effective_deadline
            .map_or(TimingPolicy::Untimed, |_| TimingPolicy::PerQuestion {
                seconds: 1,
                grace_seconds: timing.effective_grace_seconds,
            });
        let verdict = timer_verdict(&TimerEvaluation {
            policy: effective_policy,
            timer: projected_attempt(&state, tenant, &attempt).timer,
            evaluated_at: accepted_at,
            pause_extension_millis: 0,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        if verdict == TimerVerdict::TimedOut {
            return Err(StoreError::TimedOut);
        }
        let submission = AcceptedSubmissionId::from_uuid(command.attempt.as_uuid());
        let accepted = AcceptedSubmission {
            tenant,
            course: command.course,
            assignment: command.assignment,
            attempt: command.attempt,
            submission,
            actor: command.actor,
            idempotency_key: command.idempotency_key.clone(),
            request_sha256: response_sha256,
            accepted_at,
        };
        let execution = GradingExecution {
            submission,
            generation: GradingExecutionGeneration::INITIAL,
            state: crate::GradingExecutionState::Ready,
            job: command.execution_job,
            retry_count: 0,
        };
        if state.jobs.contains_key(&command.execution_job) {
            return Err(StoreError::Conflict);
        }
        let private_response = StoredPrivateSubmissionResponse {
            canonical_text: canonical_response,
            sha256: response_sha256,
            response: command.response,
        };
        state.submissions.insert(
            (tenant, command.attempt),
            StoredSubmission {
                key: command.idempotency_key,
                state: StoredSubmissionState::AcceptedPending(accepted.clone()),
            },
        );
        state
            .private_submission_responses
            .insert((tenant, command.attempt), private_response);
        // Keep Memory's current lifecycle projection aligned with the atomic
        // PostgreSQL broker transition. The accepted response stays only in
        // `private_submission_responses`; this answer-free projection records
        // that the learner can no longer submit a second response. ASVS
        // 2.3.1/2.3.3.
        let mut current_attempt = projected_attempt(&state, tenant, &attempt);
        current_attempt.status = question_model::AttemptStatus::Submitted;
        current_attempt.timer.submitted_at = Some(accepted_at);
        state
            .attempt_current
            .insert((tenant, command.attempt), current_attempt);
        complete_memory_attempt_timing_job(&mut state, tenant, command.attempt);
        state
            .automated_grading_executions
            .insert((tenant, command.attempt), execution);
        state.automated_grading_evaluations.insert(
            (tenant, command.attempt),
            SubmissionEvaluationStatus::AutomatedPending,
        );
        state.jobs.insert(
            command.execution_job,
            StoredJob {
                tenant,
                payload: crate::JobPayload::GradeAcceptedSubmission {
                    attempt: command.attempt,
                    submission,
                    execution_generation: GradingExecutionGeneration::INITIAL,
                },
                state: crate::JobState::Ready,
                available_at: accepted_at,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: 3,
                failure: None,
            },
        );
        state
            .automated_grading_execution_receipts
            .entry((tenant, command.attempt))
            .or_default()
            .push(GradingExecutionReceipt {
                submission,
                generation: GradingExecutionGeneration::INITIAL,
                resulting_state: crate::GradingExecutionState::Ready,
                worker: None,
                occurred_at: accepted_at,
            });
        Ok(accepted)
    }

    async fn automated_grading_execution(
        &self,
        context: TenantContext,
        submission: AcceptedSubmissionId,
    ) -> Result<Option<GradingExecution>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        Ok(state
            .automated_grading_executions
            .iter()
            .find_map(|((stored_tenant, _), execution)| {
                (*stored_tenant == tenant && execution.submission == submission)
                    .then_some(*execution)
            }))
    }

    async fn automated_grading_operation(
        &self,
        context: TenantContext,
        course: CourseId,
        reference: GradingOperationReference,
    ) -> Result<Option<GradingOperation>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        Ok(state
            .automated_grading_operations
            .get(&(tenant, reference))
            .copied()
            .filter(|operation| operation.course == course))
    }

    async fn record_automated_grading_execution_receipt(
        &self,
        context: TenantContext,
        receipt: GradingExecutionReceipt,
        resulting_evaluation: SubmissionEvaluationStatus,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let (attempt, generation_matches) = state
            .automated_grading_executions
            .iter_mut()
            .find_map(|((stored_tenant, attempt), execution)| {
                (*stored_tenant == tenant && execution.submission == receipt.submission)
                    .then_some((*attempt, execution.generation == receipt.generation))
            })
            .ok_or(StoreError::NotFound)?;
        if !generation_matches {
            return Err(StoreError::Conflict);
        }
        let execution = state
            .automated_grading_executions
            .get_mut(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        execution.state = receipt.resulting_state;
        state
            .automated_grading_evaluations
            .insert((tenant, attempt), resulting_evaluation);
        state
            .automated_grading_execution_receipts
            .entry((tenant, attempt))
            .or_default()
            .push(receipt);
        Ok(())
    }
}

#[async_trait]
impl AcceptedSubmissionExecutionStore for MemoryStore {
    async fn load_accepted_submission_for_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
    ) -> Result<AcceptedSubmissionExecution, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        if claim.tenant != tenant {
            return Err(StoreError::Conflict);
        }
        let now = state.authoritative_time;
        let job = state.jobs.get(&claim.job).ok_or(StoreError::NotFound)?;
        if job.tenant != tenant
            || job.state != crate::JobState::Leased
            || job.lease_token != Some(claim.lease_token)
            || !job.lease_expires_at.is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        let crate::JobPayload::GradeAcceptedSubmission {
            attempt,
            submission,
            execution_generation,
        } = job.payload
        else {
            return Err(StoreError::Conflict);
        };
        if submission != claim.submission || execution_generation != claim.execution_generation {
            return Err(StoreError::Conflict);
        }
        let execution = state
            .automated_grading_executions
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        if execution.submission != claim.submission
            || execution.generation != claim.execution_generation
            || execution.job != claim.job
            || execution.state != crate::GradingExecutionState::Running
            || state
                .automated_grading_execution_workers
                .get(&(tenant, attempt))
                != Some(&claim.worker)
        {
            return Err(StoreError::Conflict);
        }
        let stored = state
            .submissions
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        let accepted = stored.accepted_pending().ok_or(StoreError::Conflict)?;
        if accepted.submission != claim.submission || accepted.tenant != tenant {
            return Err(StoreError::Conflict);
        }
        let private = state
            .private_submission_responses
            .get(&(tenant, attempt))
            .ok_or_else(|| {
                StoreError::Unavailable("accepted response authority is missing".to_string())
            })?;
        let canonical = crate::canonical_student_response_json(&private.response)?;
        if canonical != private.canonical_text
            || objects::Sha256Digest::compute(canonical.as_bytes()) != private.sha256
            || private.sha256 != accepted.request_sha256
        {
            return Err(StoreError::Unavailable(
                "accepted response authority disagrees with immutable metadata".to_string(),
            ));
        }
        let prepared = load_prepared_accepted_submission(&state, tenant, attempt)?;
        Ok(AcceptedSubmissionExecution {
            accepted: accepted.clone(),
            response: private.response.clone(),
            prepared: Box::new(prepared),
        })
    }
}

fn load_prepared_accepted_submission(
    state: &State,
    tenant: question_model::TenantId,
    attempt_id: question_model::QuestionAttemptId,
) -> Result<crate::PreparedQuestionSubmission, StoreError> {
    let attempt = state
        .attempts
        .get(&(tenant, attempt_id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let issued_question_snapshot = state
        .attempt_issued_question_snapshots
        .get(&(tenant, attempt_id))
        .cloned()
        .ok_or_else(|| {
            StoreError::Unavailable("issued question snapshot is missing".to_string())
        })?;
    let flat_capability = state
        .attempt_flat_grading_capabilities
        .get(&(tenant, attempt_id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt flat grading capability is missing".to_string())
        })?;
    let webwork_capability = state
        .attempt_webwork_grading_capabilities
        .get(&(tenant, attempt_id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt WeBWorK grading capability is missing".to_string())
        })?;
    let qti_capability = state
        .attempt_qti_grading_capabilities
        .get(&(tenant, attempt_id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt QTI grading capability is missing".to_string())
        })?;
    super::runs::validate_issued_question_snapshot(
        &issued_question_snapshot,
        &attempt,
        flat_capability,
        webwork_capability,
        qti_capability,
        state
            .attempt_presentation_snapshots
            .get(&(tenant, attempt_id)),
    )?;
    let presentation = super::runs::load_issued_presentation(state, tenant, &attempt)?;
    let presentation_binding = state
        .attempt_presentations
        .get(&(tenant, attempt_id))
        .copied();
    let grading_envelope = state
        .attempt_grading_envelopes
        .get(&(tenant, attempt_id))
        .cloned();
    let flat_grading = super::runs::load_issued_flat_grading(state, tenant, &attempt)?;
    let webwork_grading = super::runs::load_issued_webwork_grading(state, tenant, &attempt)?;
    let issued_qti_grading =
        super::runs::load_issued_qti_grading(state, tenant, &attempt, &issued_question_snapshot)?;
    crate::validate_issued_flat_grading(
        issued_question_snapshot.question(),
        if presentation.is_some() {
            crate::PresentationCapability::EnvelopeV1
        } else {
            crate::PresentationCapability::NotApplicable
        },
        if matches!(
            attempt.issued_capability,
            IssuedAttemptCapabilityV1::FlatPresentation
        ) {
            crate::FlatGradingCapability::Required
        } else {
            crate::FlatGradingCapability::NotApplicable
        },
        flat_grading.as_ref(),
    )?;
    crate::validate_issued_webwork_grading(
        issued_question_snapshot.question(),
        if matches!(
            attempt.issued_capability,
            IssuedAttemptCapabilityV1::WebworkPresentation
        ) {
            crate::WebworkGradingCapability::Required
        } else {
            crate::WebworkGradingCapability::NotApplicable
        },
        webwork_grading.as_ref(),
    )?;
    crate::validate_issued_qti_grading(
        issued_question_snapshot.question(),
        if matches!(
            attempt.issued_capability,
            IssuedAttemptCapabilityV1::QtiPresentation
        ) {
            crate::QtiGradingCapability::Required
        } else {
            crate::QtiGradingCapability::NotApplicable
        },
        issued_qti_grading.as_ref(),
    )?;
    Ok(crate::PreparedQuestionSubmission {
        attempt,
        issued_question_snapshot,
        presentation_binding,
        presentation,
        grading_envelope,
        flat_grading,
        webwork_grading,
        issued_qti_grading,
        webwork_replay: state
            .webwork_grade_replay
            .get(&(tenant, attempt_id))
            .cloned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn accepted_pending_submission_has_no_fabricated_completed_receipt() {
        let tenant = question_model::TenantId::from_uuid(Uuid::from_u128(301));
        let submission = AcceptedSubmissionId::from_uuid(Uuid::from_u128(302));
        let stored = StoredSubmission {
            key: crate::SubmissionIdempotencyKey::parse("accepted-pending")
                .expect("bounded idempotency key"),
            state: StoredSubmissionState::AcceptedPending(AcceptedSubmission {
                tenant,
                course: CourseId::from_uuid(Uuid::from_u128(303)),
                assignment: AssignmentId::from_uuid(Uuid::from_u128(304)),
                attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(305)),
                submission,
                actor: UserId::from_uuid(Uuid::from_u128(306)),
                idempotency_key: crate::SubmissionIdempotencyKey::parse("accepted-pending")
                    .expect("bounded idempotency key"),
                request_sha256: objects::Sha256Digest::compute(b"accepted"),
                accepted_at: ActivityTimestamp::from_unix_millis(0),
            }),
        };

        assert!(stored.completed_record_opt().is_none());
        assert_eq!(
            stored
                .accepted_pending()
                .map(|accepted| accepted.submission),
            Some(submission)
        );
        let debug = format!("{stored:?}");
        assert!(!debug.contains("accepted-pending"));
        assert!(!debug.contains("88"));
        assert!(!debug.contains("response"));
    }

    #[test]
    fn private_response_identity_requires_canonical_text_digest_and_typed_value() {
        let tenant = question_model::TenantId::from_uuid(Uuid::from_u128(401));
        let attempt = QuestionAttemptId::from_uuid(Uuid::from_u128(402));
        let response = question_model::StudentResponse::Numeric { value: 88.0 };
        let private = StoredPrivateSubmissionResponse::from_response(response.clone())
            .expect("closed response serializes");
        let mut state = State::default();
        state
            .private_submission_responses
            .insert((tenant, attempt), private.clone());

        assert!(
            stored_submission_matches_response(&state, tenant, attempt, &response)
                .expect("stored private response")
        );
        assert!(
            !stored_submission_matches_response(
                &state,
                tenant,
                attempt,
                &question_model::StudentResponse::Numeric { value: 89.0 },
            )
            .expect("stored private response")
        );
        let debug = format!("{private:?}");
        assert!(!debug.contains("88"));
        assert!(!debug.contains(&private.canonical_text));
        assert!(debug.contains("[SERVER-ONLY]"));
    }
}
