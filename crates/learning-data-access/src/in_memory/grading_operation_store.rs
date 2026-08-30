//! Instructor-only automated-grading operation capability for Memory.

use async_trait::async_trait;
#[cfg(feature = "test-support")]
use question_model::GradingOperationReason;
use question_model::{
    AssignmentId, CourseId, GradingOperationAction, GradingOperationState, ScoringStatus, UserId,
};
#[cfg(feature = "test-support")]
use uuid::Uuid;

use super::{MemoryStore, StoredJob};
use crate::{
    ActorContext, GradingExecution, GradingExecutionGeneration, GradingOperation,
    GradingOperationActionReceipt, GradingOperationRevision, GradingOperationStore,
    ListInstructorGradingOperationsCommand, MAX_INSTRUCTOR_GRADING_RETRY_COUNT,
    RecalculateAssignmentCommand, RetryGradingOperationCommand, StoreError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum MemoryGradingOperationIntent {
    Retry {
        course: CourseId,
        assignment: AssignmentId,
        operation: question_model::GradingOperationReference,
        expected_revision: GradingOperationRevision,
    },
    Recalculate {
        course: CourseId,
        assignment: AssignmentId,
        expected_assignment_revision: question_model::AssignmentRevision,
    },
}

#[derive(Debug, Clone)]
pub(super) struct MemoryGradingOperationAction {
    actor: UserId,
    intent: MemoryGradingOperationIntent,
    receipt: GradingOperationActionReceipt,
}

pub(super) type MemoryGradingOperationActions =
    std::collections::BTreeMap<crate::GradingOperationActionId, MemoryGradingOperationAction>;

/// Seeds the smallest durable exception state needed by route-boundary tests.
///
/// The `test-support` feature is the explicit non-production fixture seam for
/// the in-memory backend. Keeping this setup beside the operation store lets
/// server tests exercise the real retry transaction without manufacturing
/// browser-facing records or reaching into private state.
#[cfg(feature = "test-support")]
impl MemoryStore {
    pub fn seed_retryable_grading_operation_for_test(
        &self,
        course: CourseId,
        assignment: AssignmentId,
        operation: question_model::GradingOperationReference,
        attempt: question_model::QuestionAttemptId,
        submission: crate::AcceptedSubmissionId,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        state.automated_grading_executions.insert(
            attempt,
            GradingExecution {
                submission,
                generation: GradingExecutionGeneration::INITIAL,
                state: crate::GradingExecutionState::Exception,
                job: crate::JobId::from_uuid(Uuid::from_u128(0x7f00_0000_0000_0001)),
                retry_count: 0,
            },
        );
        state.automated_grading_operations.insert(
            operation,
            GradingOperation {
                course,
                assignment,
                reference: operation,
                target: crate::GradingOperationTarget::SubmissionRecovery { submission },
                reason: GradingOperationReason::GraderExecutionFailure,
                state: GradingOperationState::Actionable,
                revision: crate::GradingOperationRevision::INITIAL,
                next_action: Some(GradingOperationAction::Retry),
            },
        );
        Ok(())
    }
}

#[async_trait]
impl GradingOperationStore for MemoryStore {
    async fn list_instructor_grading_operations(
        &self,
        context: ActorContext,
        command: ListInstructorGradingOperationsCommand,
    ) -> Result<crate::Page<crate::InstructorGradingOperationRow>, StoreError> {
        let state = self.read_state()?;
        super::grading_operations::require_instructor_operation_authority(
            &state,
            context,
            command.session,
            command.course,
            command.assignment,
        )?;
        let mut rows = state
            .automated_grading_operations
            .values()
            .filter(|operation| {
                operation.course == command.course && operation.assignment == command.assignment
            })
            .map(|operation| {
                super::grading_operations::operation_row(&state, *operation, command.group_by)
            })
            .collect::<Result<Vec<_>, _>>()?;
        rows.sort_by_key(|row| {
            (
                crate::operation_group_key(&row.group),
                row.operation.reference,
            )
        });
        super::grading_operations::page_rows(
            rows,
            command.course,
            command.assignment,
            command.group_by,
            command.page,
        )
    }

    async fn retry_instructor_grading_operation(
        &self,
        context: ActorContext,
        command: RetryGradingOperationCommand,
    ) -> Result<GradingOperationActionReceipt, StoreError> {
        let mut state = self.write_state()?;
        let actor = super::grading_operations::require_instructor_operation_authority(
            &state,
            context,
            command.session,
            command.course,
            command.assignment,
        )?;
        let intent = retry_intent(&command);
        if let Some(stored) = state
            .instructor_grading_operation_actions
            .get(&command.action)
        {
            return replay(stored, actor, &intent);
        }
        let operation = state
            .automated_grading_operations
            .get(&command.operation)
            .copied()
            .filter(|operation| {
                operation.course == command.course && operation.assignment == command.assignment
            })
            .ok_or(StoreError::NotFound)?;
        if operation.revision != command.expected_revision
            || operation.next_action != Some(GradingOperationAction::Retry)
        {
            return Err(StoreError::Conflict);
        }
        let question_attempt = match operation.target {
            crate::GradingOperationTarget::SubmissionRecovery { submission } => state
                .automated_grading_executions
                .iter()
                .find_map(|(attempt, execution)| {
                    (super::activity::attempt_belongs_to_course(&state, *attempt)
                        && execution.submission == submission)
                        .then_some(*attempt)
                })
                .ok_or(StoreError::NotFound)?,
            _ => return Err(StoreError::Conflict),
        };
        let execution = state
            .automated_grading_executions
            .get(&question_attempt)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if execution.state != crate::GradingExecutionState::Exception
            || execution.retry_count >= MAX_INSTRUCTOR_GRADING_RETRY_COUNT
        {
            return Err(StoreError::Conflict);
        }
        let generation = GradingExecutionGeneration::from_u64(
            execution
                .generation
                .as_u64()
                .checked_add(1)
                .ok_or(StoreError::Conflict)?,
        )
        .ok_or(StoreError::Conflict)?;
        let revision = super::grading_operations::next_operation_revision(operation.revision)?;
        let job = crate::JobId::from_uuid(command.action.as_uuid());
        if state.jobs.contains_key(&job) {
            return Err(StoreError::Conflict);
        }
        let receipt = GradingOperationActionReceipt::Retry {
            action: command.action,
            operation: command.operation,
            resulting_operation_revision: revision,
            safe_category: crate::GradingOperationReceiptSafeCategory::InstructorRetry,
            occurred_at: state.authoritative_time,
        };
        let occurred_at = state.authoritative_time;
        state.automated_grading_executions.insert(
            question_attempt,
            GradingExecution {
                generation,
                state: crate::GradingExecutionState::Ready,
                job,
                retry_count: execution
                    .retry_count
                    .checked_add(1)
                    .ok_or(StoreError::Conflict)?,
                ..execution
            },
        );
        state.automated_grading_evaluations.insert(
            question_attempt,
            question_model::SubmissionEvaluationStatus::AutomatedPending,
        );
        state
            .automated_grading_execution_receipts
            .entry(question_attempt)
            .or_default()
            .push(crate::GradingExecutionReceipt {
                submission: execution.submission,
                generation,
                resulting_state: crate::GradingExecutionState::Ready,
                safe_category: crate::GradingExecutionReceiptSafeCategory::InstructorRetry,
                actor: Some(actor),
                worker: None,
                occurred_at,
            });
        state.jobs.insert(
            job,
            StoredJob {
                payload: crate::JobPayload::GradeAcceptedSubmission {
                    attempt: question_attempt,
                    submission: execution.submission,
                    execution_generation: generation,
                },
                state: crate::JobState::Ready,
                available_at: occurred_at,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: crate::ACCEPTED_SUBMISSION_JOB_MAX_ATTEMPTS,
                failure: None,
            },
        );
        state.automated_grading_operations.insert(
            command.operation,
            GradingOperation {
                revision,
                state: GradingOperationState::ActionInProgress,
                next_action: None,
                ..operation
            },
        );
        state.instructor_grading_operation_actions.insert(
            command.action,
            MemoryGradingOperationAction {
                actor,
                intent,
                receipt,
            },
        );
        Ok(receipt)
    }

    async fn recalculate_instructor_assignment(
        &self,
        context: ActorContext,
        command: RecalculateAssignmentCommand,
    ) -> Result<GradingOperationActionReceipt, StoreError> {
        let mut state = self.write_state()?;
        let actor = super::grading_operations::require_instructor_operation_authority(
            &state,
            context,
            command.session,
            command.course,
            command.assignment,
        )?;
        let intent = recalculation_intent(&command);
        if let Some(stored) = state
            .instructor_grading_operation_actions
            .get(&command.action)
        {
            return replay(stored, actor, &intent);
        }
        let assignment_revision = state
            .assignment_revisions
            .get(&command.assignment)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if assignment_revision != command.expected_assignment_revision {
            return Err(StoreError::Conflict);
        }
        let (_, status) = state
            .assignment_scoring
            .get(&command.assignment)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if !matches!(status, ScoringStatus::Current | ScoringStatus::Failed) {
            return Err(StoreError::Conflict);
        }
        let invalidation = super::scoring_invalidation::request_scoring_invalidation(
            &mut state,
            command.course,
            command.assignment,
            crate::ScoringInvalidationOrigin::instructor_recalculation(
                crate::ScoringInvalidationOriginId::from_uuid(command.action.as_uuid()),
                actor,
            ),
            crate::JobId::from_uuid(command.action.as_uuid()),
        )?;
        let receipt = GradingOperationActionReceipt::Recalculation {
            action: command.action,
            operation: invalidation.operation,
            resulting_operation_revision: GradingOperationRevision::INITIAL,
            assignment_revision,
            scoring_generation: invalidation.generation,
            safe_category: crate::GradingOperationReceiptSafeCategory::InstructorRecalculation,
            occurred_at: state.authoritative_time,
        };
        state.instructor_grading_operation_actions.insert(
            command.action,
            MemoryGradingOperationAction {
                actor,
                intent,
                receipt,
            },
        );
        Ok(receipt)
    }
}

fn retry_intent(command: &RetryGradingOperationCommand) -> MemoryGradingOperationIntent {
    MemoryGradingOperationIntent::Retry {
        course: command.course,
        assignment: command.assignment,
        operation: command.operation,
        expected_revision: command.expected_revision,
    }
}
fn recalculation_intent(command: &RecalculateAssignmentCommand) -> MemoryGradingOperationIntent {
    MemoryGradingOperationIntent::Recalculate {
        course: command.course,
        assignment: command.assignment,
        expected_assignment_revision: command.expected_assignment_revision,
    }
}
fn replay(
    stored: &MemoryGradingOperationAction,
    actor: UserId,
    intent: &MemoryGradingOperationIntent,
) -> Result<GradingOperationActionReceipt, StoreError> {
    (stored.actor == actor && &stored.intent == intent)
        .then_some(stored.receipt)
        .ok_or(StoreError::Conflict)
}
