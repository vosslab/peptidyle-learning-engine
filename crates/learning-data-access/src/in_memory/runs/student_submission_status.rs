//! Route-bound Student submission-status reads for deterministic Memory tests.

use async_trait::async_trait;
use question_model::{CourseMembershipRole, SubmissionEvaluationStatus};

use super::super::*;
use crate::{
    ActorContext, GradingExecutionState, StoreError, StudentSubmissionStatusRead,
    StudentWorkRoutingBinding, SubmissionReceiptRead,
};

#[async_trait]
impl crate::StudentSubmissionStatusStore for MemoryStore {
    async fn student_submission_status(
        &self,
        context: ActorContext,
        actor: UserId,
        binding: StudentWorkRoutingBinding,
        attempt: QuestionAttemptId,
    ) -> Result<StudentSubmissionStatusRead, StoreError> {
        read(self, context, actor, binding, attempt).await
    }
}

/// Reads one closed Student status after establishing the complete nested
/// Student-work witness under the same immutable Memory state lock.
pub(super) async fn read(
    store: &MemoryStore,
    context: ActorContext,
    actor: UserId,
    binding: StudentWorkRoutingBinding,
    attempt_id: QuestionAttemptId,
) -> Result<StudentSubmissionStatusRead, StoreError> {
    let state = store.read_state()?;
    if context.user_id() != actor {
        return Err(StoreError::NotFound);
    }

    // ASVS V8.2.2/V8.3.1: route values are assertions, not authority.  Check
    // current student entitlement before the opaque attempt, then bind the
    // run and enrollment back to that exact course/assignment/actor tuple.
    super::super::entitlement::active_membership_for(&state, binding.course, actor)
        .filter(|membership| {
            membership.role == CourseMembershipRole::Student && membership.student.is_some()
        })
        .ok_or(StoreError::NotFound)?;
    let assignment = assignment_record(&state, binding.assignment)?;
    if assignment.course_id != binding.course {
        return Err(StoreError::NotFound);
    }
    require_course_records_accessible(&state, binding.course)?;
    let domain::entitlement::EntitlementDecision::Granted(grant) =
        super::super::entitlement::evaluate_locked(
            &state,
            actor,
            binding.course,
            binding.assignment,
        )?
    else {
        return Err(StoreError::NotFound);
    };
    let attempt = state
        .attempts
        .get(&attempt_id)
        .ok_or(StoreError::NotFound)?;
    let run = state.runs.get(&attempt.run).ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(&state, run.enrollment)?;
    if enrollment.assignment != binding.assignment
        || enrollment.user != actor
        || enrollment.student != grant.student()
    {
        return Err(StoreError::NotFound);
    }

    // `load_submission_record` is the canonical immutable receipt reader.
    // It validates a completed aggregate before disclosure and never rebuilds
    // Student work from mutable catalog state.
    let receipt = super::load_submission_record(&state, attempt)?;
    let execution = state.automated_grading_executions.get(&attempt_id);
    let evaluation = state.automated_grading_evaluations.get(&attempt_id);
    status_from_durable_state(&state, receipt, execution, evaluation)
}

/// Retains the legacy flat receipt read while its route-bound successor owns
/// the stronger nested witness above.  Keeping the body here prevents the
/// `RunStore` implementation from exceeding the repository source limit.
pub(super) fn submission_record(
    store: &MemoryStore,
    context: ActorContext,
    actor: UserId,
    attempt_id: QuestionAttemptId,
) -> Result<SubmissionReceiptRead, StoreError> {
    let state = store.read_state()?;
    if context.user_id() != actor {
        return Err(StoreError::NotFound);
    }
    let attempt = state
        .attempts
        .get(&attempt_id)
        .ok_or(StoreError::NotFound)?;
    let run = state.runs.get(&attempt.run).ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(&state, run.enrollment)?;
    let assignment = assignment_record(&state, enrollment.assignment)?;
    require_course_records_accessible(&state, assignment.course_id)?;
    require_attempt_owner(&state, attempt, actor)?;
    super::load_submission_record(&state, attempt)
}

fn status_from_durable_state(
    state: &State,
    receipt: SubmissionReceiptRead,
    execution: Option<&crate::GradingExecution>,
    evaluation: Option<&SubmissionEvaluationStatus>,
) -> Result<StudentSubmissionStatusRead, StoreError> {
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
            Ok(StudentSubmissionStatusRead::AcceptedPending(pending))
        }
        (
            SubmissionReceiptRead::AcceptedPending(pending),
            Some(execution),
            Some(SubmissionEvaluationStatus::AutomatedException),
        ) if execution.state == GradingExecutionState::Exception => {
            Ok(StudentSubmissionStatusRead::InstructorAttention(pending))
        }
        (
            SubmissionReceiptRead::Completed(record),
            Some(execution),
            Some(SubmissionEvaluationStatus::Graded | SubmissionEvaluationStatus::Exempt),
        ) if execution.state == GradingExecutionState::Completed => {
            let next_pending = completed_successor_is_eligible(state, &record)?;
            Ok(StudentSubmissionStatusRead::Completed {
                record,
                next_pending,
            })
        }
        // Every partial, legacy, superseded, or contradictory combination is
        // unavailable. A Student must never receive a reconstructed receipt
        // or a reason that reveals worker/private evaluation state.
        _ => Err(StoreError::Unavailable(
            "Student submission status has no coherent durable aggregate".to_string(),
        )),
    }
}

/// Mirrors the read-only eligibility half of Student successor delivery. A
/// status read reports a pending successor only when the immutable run plan
/// still permits `start_or_resume_run` to issue one; it never writes a link.
fn completed_successor_is_eligible(
    state: &State,
    record: &crate::SubmissionRecord,
) -> Result<bool, StoreError> {
    let run_items = state
        .run_items
        .get(&record.run.id)
        .ok_or(StoreError::Unavailable(
            "completed submission has no immutable run items".to_string(),
        ))?;
    let attempts = state
        .attempts
        .values()
        .filter(|attempt| attempt.run == record.run.id)
        .map(|attempt| super::super::projected_attempt(state, attempt))
        .collect::<Vec<_>>();
    let questions = run_items
        .iter()
        .map(|item| {
            let question = state
                .published
                .get(&(item.reference.problem, item.reference.version))
                .ok_or(StoreError::Unavailable(
                    "run item has no immutable published question".to_string(),
                ))?;
            if question.problem != item.reference.problem
                || question.version != item.reference.version
            {
                return Err(StoreError::Unavailable(
                    "run item question identity is incoherent".to_string(),
                ));
            }
            Ok((item.reference, question.question.clone()))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    crate::successor_is_eligible(
        &record.run,
        state
            .submission_next_attempts
            .contains_key(&record.attempt.id),
        run_items,
        &attempts,
        &questions,
    )
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

    fn test_status(
        receipt: SubmissionReceiptRead,
        execution: Option<&crate::GradingExecution>,
        evaluation: Option<&SubmissionEvaluationStatus>,
    ) -> Result<StudentSubmissionStatusRead, StoreError> {
        status_from_durable_state(&State::default(), receipt, execution, evaluation)
    }

    #[test]
    fn pending_and_attention_states_use_only_the_closed_answer_free_projection() {
        for state in [
            GradingExecutionState::Ready,
            GradingExecutionState::Running,
            GradingExecutionState::RetryWait,
        ] {
            assert_eq!(
                test_status(
                    SubmissionReceiptRead::AcceptedPending(pending()),
                    Some(&execution(state)),
                    Some(&SubmissionEvaluationStatus::AutomatedPending),
                ),
                Ok(StudentSubmissionStatusRead::AcceptedPending(pending())),
            );
        }
        assert_eq!(
            test_status(
                SubmissionReceiptRead::AcceptedPending(pending()),
                Some(&execution(GradingExecutionState::Exception)),
                Some(&SubmissionEvaluationStatus::AutomatedException),
            ),
            Ok(StudentSubmissionStatusRead::InstructorAttention(pending())),
        );
    }

    #[test]
    fn missing_or_contradictory_status_never_fabricates_a_student_projection() {
        assert_eq!(
            test_status(SubmissionReceiptRead::Missing, None, None),
            Err(StoreError::NotFound),
        );
        assert!(matches!(
            test_status(
                SubmissionReceiptRead::AcceptedPending(pending()),
                Some(&execution(GradingExecutionState::Ready)),
                Some(&SubmissionEvaluationStatus::AutomatedException),
            ),
            Err(StoreError::Unavailable(_))
        ));
    }
}
