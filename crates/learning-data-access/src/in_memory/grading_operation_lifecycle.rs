//! Shared Memory projection for authoritative worker lifecycle transitions.
//!
//! The execution and scoring workers remain the only state authorities.  This
//! module only reflects their already-fenced terminal outcome into an existing
//! Instructor operation thread, mirroring migration 2026081862.

use question_model::{
    AssignmentId, CourseId, GradingOperationAction, GradingOperationReason, GradingOperationState,
    ScoringGeneration, ScoringStatus,
};

use super::State;
use crate::{GradingOperation, StoreError};

/// Closes every in-progress submission recovery thread for this completed
/// immutable submission.  The target identity binds the projection to the
/// worker-owned execution rather than to an Instructor request.
pub(super) fn close_completed_submission_operation(
    state: &mut State,
    submission: crate::AcceptedSubmissionId,
) -> Result<(), StoreError> {
    let references = state
        .automated_grading_operations
        .iter()
        .filter_map(|(reference, operation)| {
            (operation.target == crate::GradingOperationTarget::SubmissionRecovery { submission }
                && operation.state == GradingOperationState::ActionInProgress)
                .then_some(*reference)
        })
        .collect::<Vec<_>>();
    for reference in references {
        let operation = state
            .automated_grading_operations
            .get_mut(&reference)
            .expect("selected grading operation remains present");
        operation.revision = next_revision(operation.revision)?;
        operation.state = GradingOperationState::Completed;
        operation.next_action = None;
    }
    Ok(())
}

/// Creates the first submission recovery thread or reopens its exact prior
/// thread after a later terminal worker exception.  An immutable submission
/// has one recovery thread in the Memory model, matching the database trigger
/// that locks and refreshes the existing target before accepting a new row.
pub(super) fn reopen_submission_operation(
    state: &mut State,
    course: CourseId,
    assignment: AssignmentId,
    submission: crate::AcceptedSubmissionId,
    reason: GradingOperationReason,
) -> Result<(), StoreError> {
    let references = state
        .automated_grading_operations
        .iter()
        .filter_map(|(reference, operation)| {
            (operation.course == course
                && operation.assignment == assignment
                && operation.target
                    == crate::GradingOperationTarget::SubmissionRecovery { submission })
            .then_some(*reference)
        })
        .collect::<Vec<_>>();
    if references.len() > 1 {
        return Err(StoreError::InvalidRecord(
            "submission recovery has multiple operation threads".to_string(),
        ));
    }
    if let Some(reference) = references.first().copied() {
        let operation = state
            .automated_grading_operations
            .get_mut(&reference)
            .expect("selected grading operation remains present");
        operation.revision = next_revision(operation.revision)?;
        operation.reason = reason;
        operation.state = GradingOperationState::Actionable;
        operation.next_action = Some(GradingOperationAction::Retry);
        return Ok(());
    }

    let reference = next_operation_reference(state)?;
    state.automated_grading_operations.insert(
        reference,
        GradingOperation {
            course,
            assignment,
            reference,
            target: crate::GradingOperationTarget::SubmissionRecovery { submission },
            reason,
            state: GradingOperationState::Actionable,
            revision: crate::GradingOperationRevision::INITIAL,
            next_action: Some(GradingOperationAction::Retry),
        },
    );
    Ok(())
}

/// Reflects one exact scoring-generation publication into its existing
/// Instructor-requested recalculation thread.  Background recalculations that
/// have no Instructor thread remain purely worker-owned and create no UI work.
pub(super) fn project_assignment_scoring_operation(
    state: &mut State,
    assignment: AssignmentId,
    generation: ScoringGeneration,
    status: ScoringStatus,
) -> Result<(), StoreError> {
    if status == ScoringStatus::Recalculating {
        return Ok(());
    }
    let references = state
        .automated_grading_operations
        .iter()
        .filter_map(|(reference, operation)| {
            (operation.assignment == assignment
                && operation.target
                    == crate::GradingOperationTarget::AssignmentScoringGeneration {
                        requested_generation: generation,
                    }
                && operation.state == GradingOperationState::ActionInProgress)
                .then_some(*reference)
        })
        .collect::<Vec<_>>();
    for reference in references {
        let operation = state
            .automated_grading_operations
            .get_mut(&reference)
            .expect("selected grading operation remains present");
        operation.revision = next_revision(operation.revision)?;
        match status {
            ScoringStatus::Current => {
                operation.state = GradingOperationState::Completed;
                operation.next_action = None;
            }
            ScoringStatus::Failed => {
                operation.reason = GradingOperationReason::ScoringRecalculationFailed;
                operation.state = GradingOperationState::Actionable;
                operation.next_action = Some(GradingOperationAction::Recalculate);
            }
            ScoringStatus::Recalculating => {}
        }
    }
    Ok(())
}

pub(super) fn next_operation_reference(
    state: &State,
) -> Result<question_model::GradingOperationReference, StoreError> {
    state
        .automated_grading_operations
        .keys()
        .map(|reference| reference.number())
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .and_then(|number| question_model::GradingOperationReference::new(u64::from(number)))
        .ok_or(StoreError::Conflict)
}

fn next_revision(
    revision: crate::GradingOperationRevision,
) -> Result<crate::GradingOperationRevision, StoreError> {
    revision
        .as_u64()
        .checked_add(1)
        .and_then(crate::GradingOperationRevision::from_u64)
        .ok_or(StoreError::Conflict)
}
