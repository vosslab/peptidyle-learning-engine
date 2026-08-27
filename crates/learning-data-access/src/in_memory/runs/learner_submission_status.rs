//! Route-bound learner submission-status reads for deterministic Memory tests.

use async_trait::async_trait;
use question_model::{CourseMembershipRole, SubmissionEvaluationStatus};

use super::super::*;
use crate::{
    GradingExecutionState, LearnerSubmissionStatusRead, LearnerWorkRoutingBinding, StoreError,
    SubmissionReceiptRead, TenantContext,
};

#[async_trait]
impl crate::LearnerSubmissionStatusStore for MemoryStore {
    async fn learner_submission_status(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
    ) -> Result<LearnerSubmissionStatusRead, StoreError> {
        read(self, context, actor, binding, attempt).await
    }
}

/// Reads one closed learner status after establishing the complete nested
/// learner-work witness under the same immutable Memory state lock.
pub(super) async fn read(
    store: &MemoryStore,
    context: TenantContext,
    actor: UserId,
    binding: LearnerWorkRoutingBinding,
    attempt_id: QuestionAttemptId,
) -> Result<LearnerSubmissionStatusRead, StoreError> {
    let state = store.read_state()?;
    let tenant = context.tenant_id();

    // ASVS V8.2.2/V8.3.1: route values are assertions, not authority.  Check
    // current student entitlement before the opaque attempt, then bind the
    // run and enrollment back to that exact course/assignment/actor tuple.
    super::super::entitlement::active_membership_for(&state, tenant, binding.course, actor)
        .filter(|membership| {
            membership.role == CourseMembershipRole::Student && membership.student.is_some()
        })
        .ok_or(StoreError::NotFound)?;
    let assignment = assignment_record(&state, tenant, binding.assignment)?;
    if assignment.course_id != binding.course {
        return Err(StoreError::NotFound);
    }
    require_course_records_accessible(&state, tenant, binding.course)?;
    let domain::entitlement::EntitlementDecision::Granted(grant) =
        super::super::entitlement::evaluate_locked(
            &state,
            tenant,
            actor,
            binding.course,
            binding.assignment,
        )?
    else {
        return Err(StoreError::NotFound);
    };
    let attempt = state
        .attempts
        .get(&(tenant, attempt_id))
        .ok_or(StoreError::NotFound)?;
    let run = state
        .runs
        .get(&(tenant, attempt.run))
        .ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
    if enrollment.assignment != binding.assignment
        || enrollment.user != actor
        || enrollment.student != grant.student()
    {
        return Err(StoreError::NotFound);
    }

    // `load_submission_record` is the canonical immutable receipt reader.
    // It validates a completed aggregate before disclosure and never rebuilds
    // learner work from mutable catalog state.
    let receipt = super::load_submission_record(&state, tenant, attempt)?;
    let execution = state
        .automated_grading_executions
        .get(&(tenant, attempt_id));
    let evaluation = state
        .automated_grading_evaluations
        .get(&(tenant, attempt_id));
    status_from_durable_state(receipt, execution, evaluation)
}

/// Retains the legacy flat receipt read while its route-bound successor owns
/// the stronger nested witness above.  Keeping the body here prevents the
/// `RunStore` implementation from exceeding the repository source limit.
pub(super) fn submission_record(
    store: &MemoryStore,
    context: TenantContext,
    actor: UserId,
    attempt_id: QuestionAttemptId,
) -> Result<SubmissionReceiptRead, StoreError> {
    let state = store.read_state()?;
    let tenant = context.tenant_id();
    let attempt = state
        .attempts
        .get(&(tenant, attempt_id))
        .ok_or(StoreError::NotFound)?;
    let run = state
        .runs
        .get(&(tenant, attempt.run))
        .ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
    let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
    require_course_records_accessible(&state, tenant, assignment.course_id)?;
    require_attempt_owner(&state, tenant, attempt, actor)?;
    super::load_submission_record(&state, tenant, attempt)
}

fn status_from_durable_state(
    receipt: SubmissionReceiptRead,
    execution: Option<&crate::GradingExecution>,
    evaluation: Option<&SubmissionEvaluationStatus>,
) -> Result<LearnerSubmissionStatusRead, StoreError> {
    match (receipt, execution, evaluation) {
        (SubmissionReceiptRead::Missing, _, _) => Err(StoreError::NotFound),
        (
            SubmissionReceiptRead::AcceptedPending(pending),
            Some(execution),
            Some(SubmissionEvaluationStatus::AutomatedPending),
        ) if matches!(
            execution.state,
            GradingExecutionState::Ready
                | GradingExecutionState::Running
                | GradingExecutionState::RetryWait
        ) =>
        {
            Ok(LearnerSubmissionStatusRead::AcceptedPending(pending))
        }
        (
            SubmissionReceiptRead::AcceptedPending(pending),
            Some(execution),
            Some(SubmissionEvaluationStatus::AutomatedException),
        ) if execution.state == GradingExecutionState::Exception => {
            Ok(LearnerSubmissionStatusRead::InstructorAttention(pending))
        }
        (
            SubmissionReceiptRead::AcceptedPending(pending),
            Some(_),
            Some(SubmissionEvaluationStatus::NeedsManualGrading),
        ) => Ok(LearnerSubmissionStatusRead::InstructorAttention(pending)),
        (
            SubmissionReceiptRead::Completed(record),
            Some(execution),
            Some(SubmissionEvaluationStatus::Graded | SubmissionEvaluationStatus::Exempt),
        ) if execution.state == GradingExecutionState::Completed => {
            Ok(LearnerSubmissionStatusRead::Completed(record))
        }
        // Every partial, legacy, superseded, or contradictory combination is
        // unavailable.  A learner must never receive a reconstructed receipt
        // or a reason that reveals worker/private evaluation state.
        _ => Err(StoreError::Unavailable(
            "learner submission status has no coherent durable aggregate".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AcceptedSubmissionPending, GradingExecutionGeneration, JobId};
    use question_model::QuestionAttemptId;
    use uuid::Uuid;

    fn pending() -> AcceptedSubmissionPending {
        AcceptedSubmissionPending::new(QuestionAttemptId::from_uuid(Uuid::from_u128(91)))
    }

    fn execution(state: GradingExecutionState) -> crate::GradingExecution {
        crate::GradingExecution {
            submission: crate::AcceptedSubmissionId::from_uuid(Uuid::from_u128(92)),
            generation: GradingExecutionGeneration::INITIAL,
            state,
            job: JobId::from_uuid(Uuid::from_u128(93)),
            retry_count: 0,
        }
    }

    #[test]
    fn pending_and_attention_states_use_only_the_closed_answer_free_projection() {
        for state in [
            GradingExecutionState::Ready,
            GradingExecutionState::Running,
            GradingExecutionState::RetryWait,
        ] {
            assert_eq!(
                status_from_durable_state(
                    SubmissionReceiptRead::AcceptedPending(pending()),
                    Some(&execution(state)),
                    Some(&SubmissionEvaluationStatus::AutomatedPending),
                ),
                Ok(LearnerSubmissionStatusRead::AcceptedPending(pending())),
            );
        }
        assert_eq!(
            status_from_durable_state(
                SubmissionReceiptRead::AcceptedPending(pending()),
                Some(&execution(GradingExecutionState::Exception)),
                Some(&SubmissionEvaluationStatus::AutomatedException),
            ),
            Ok(LearnerSubmissionStatusRead::InstructorAttention(pending())),
        );
        assert_eq!(
            status_from_durable_state(
                SubmissionReceiptRead::AcceptedPending(pending()),
                Some(&execution(GradingExecutionState::Ready)),
                Some(&SubmissionEvaluationStatus::NeedsManualGrading),
            ),
            Ok(LearnerSubmissionStatusRead::InstructorAttention(pending())),
        );
    }

    #[test]
    fn missing_or_contradictory_status_never_fabricates_a_learner_projection() {
        assert_eq!(
            status_from_durable_state(SubmissionReceiptRead::Missing, None, None),
            Err(StoreError::NotFound),
        );
        assert!(matches!(
            status_from_durable_state(
                SubmissionReceiptRead::AcceptedPending(pending()),
                Some(&execution(GradingExecutionState::Ready)),
                Some(&SubmissionEvaluationStatus::AutomatedException),
            ),
            Err(StoreError::Unavailable(_))
        ));
    }
}
