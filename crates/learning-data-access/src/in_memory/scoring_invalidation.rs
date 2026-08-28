//! Canonical Memory capability for derived-score invalidation.
//!
//! This module owns the one state transition from immutable cause evidence to
//! a new scoring generation, queued worker job, and Instructor-safe operation
//! thread. Callers retain ownership of their source mutation only.

use std::collections::BTreeMap;

use question_model::{
    AssignmentId, CourseId, GradingOperationReason, GradingOperationState, ScoringGeneration,
    ScoringStatus, TenantId,
};

use super::{State, StoredJob, grading_operation_lifecycle};
use crate::{
    GradingOperation, GradingOperationRevision, GradingOperationTarget, JobId, JobPayload,
    JobState, ScoringInvalidationOrigin, StoreError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MemoryScoringInvalidation {
    pub origin: ScoringInvalidationOrigin,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub generation: ScoringGeneration,
    pub job: JobId,
    pub operation: question_model::GradingOperationReference,
}

pub(super) type MemoryScoringInvalidations = BTreeMap<
    (
        TenantId,
        crate::ScoringInvalidationOriginKind,
        crate::ScoringInvalidationOriginId,
    ),
    MemoryScoringInvalidation,
>;

/// Resolves definition-edit generation semantics before any durable job exists.
/// A semantic edit with no derived scores still advances its fence, while a
/// recalculation-capable edit leaves advancement to the atomic capability.
pub(super) fn definition_scoring_state(
    generation: ScoringGeneration,
    status: ScoringStatus,
    scoring_changed: bool,
    has_results: bool,
) -> Result<(ScoringGeneration, ScoringStatus, bool), StoreError> {
    let requires_invalidation = scoring_changed && has_results;
    if requires_invalidation {
        return Ok((generation, ScoringStatus::Current, true));
    }
    if scoring_changed {
        return generation
            .next()
            .map(|next| (next, ScoringStatus::Current, false))
            .ok_or(StoreError::Conflict);
    }
    Ok((generation, status, false))
}

/// Atomically records one immutable invalidation cause and makes its exact
/// generation eligible for worker-owned recalculation.
///
/// Exact replay returns the original durable result. A new origin advances the
/// scoring generation, supersedes older active generation threads, then inserts
/// the one 1830-compatible job and its safe operation projection.
pub(super) fn request_scoring_invalidation(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    origin: ScoringInvalidationOrigin,
    job: JobId,
) -> Result<MemoryScoringInvalidation, StoreError> {
    if matches!(
        origin.kind,
        crate::ScoringInvalidationOriginKind::AcceptedSubmissionCompletion
    ) != origin.actor.is_none()
    {
        return Err(StoreError::InvalidRecord(
            "scoring invalidation origin actor does not match its authority kind".to_string(),
        ));
    }
    let expected_reason = match origin.kind {
        crate::ScoringInvalidationOriginKind::InstructorRecalculation => {
            GradingOperationReason::InstructorRequestedRecalculation
        }
        crate::ScoringInvalidationOriginKind::AssignmentDefinition
        | crate::ScoringInvalidationOriginKind::ManualGrade
        | crate::ScoringInvalidationOriginKind::LearnerSupport
        | crate::ScoringInvalidationOriginKind::AcceptedSubmissionCompletion => {
            GradingOperationReason::ScoringRecalculationRequested
        }
    };
    let origin_key = (tenant, origin.kind, origin.id);
    if let Some(existing) = state.scoring_invalidations.get(&origin_key).copied() {
        if existing.origin == origin
            && existing.course == course
            && existing.assignment == assignment
        {
            return Ok(existing);
        }
        return Err(StoreError::Conflict);
    }
    let key = (tenant, assignment);
    let (current_generation, _) = state
        .assignment_scoring
        .get(&key)
        .copied()
        .ok_or(StoreError::NotFound)?;
    let generation = current_generation.next().ok_or(StoreError::Conflict)?;
    let operation = grading_operation_lifecycle::next_operation_reference(state, tenant)?;
    if state.jobs.contains_key(&job) {
        return Err(StoreError::Conflict);
    }
    supersede_older_active_operations(state, tenant, assignment, generation)?;
    let invalidation = MemoryScoringInvalidation {
        origin,
        course,
        assignment,
        generation,
        job,
        operation,
    };
    state
        .assignment_scoring
        .insert(key, (generation, ScoringStatus::Recalculating));
    state.jobs.insert(
        job,
        StoredJob {
            tenant,
            payload: JobPayload::RecalculateAssignment {
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
    state.automated_grading_operations.insert(
        (tenant, operation),
        GradingOperation {
            tenant,
            course,
            assignment,
            reference: operation,
            target: GradingOperationTarget::AssignmentScoringGeneration {
                requested_generation: generation,
            },
            reason: expected_reason,
            state: GradingOperationState::ActionInProgress,
            revision: GradingOperationRevision::INITIAL,
            next_action: None,
        },
    );
    state.scoring_invalidations.insert(origin_key, invalidation);
    Ok(invalidation)
}

fn supersede_older_active_operations(
    state: &mut State,
    tenant: TenantId,
    assignment: AssignmentId,
    next_generation: ScoringGeneration,
) -> Result<(), StoreError> {
    let revisions = state
        .automated_grading_operations
        .iter()
        .filter_map(|((stored_tenant, reference), operation)| {
            let GradingOperationTarget::AssignmentScoringGeneration {
                requested_generation,
            } = operation.target
            else {
                return None;
            };
            (*stored_tenant == tenant
                && operation.assignment == assignment
                && matches!(
                    operation.state,
                    GradingOperationState::ActionInProgress | GradingOperationState::Actionable
                )
                && requested_generation.value() < next_generation.value())
            .then_some((*reference, operation.revision))
        })
        .map(|(reference, revision)| {
            revision
                .as_u64()
                .checked_add(1)
                .and_then(GradingOperationRevision::from_u64)
                .map(|next_revision| (reference, next_revision))
                .ok_or(StoreError::Conflict)
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (reference, revision) in revisions {
        let operation = state
            .automated_grading_operations
            .get_mut(&(tenant, reference))
            .expect("selected scoring operation remains present");
        operation.revision = revision;
        operation.state = GradingOperationState::Superseded;
        operation.next_action = None;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{
        GradingOperationAction, GradingOperationReason, GradingOperationState, UserId,
    };
    use uuid::Uuid;

    fn tenant() -> TenantId {
        TenantId::from_uuid(Uuid::from_u128(1))
    }

    fn assignment() -> AssignmentId {
        AssignmentId::from_uuid(Uuid::from_u128(2))
    }

    fn course() -> CourseId {
        CourseId::from_uuid(Uuid::from_u128(3))
    }

    fn origin(id: u128) -> ScoringInvalidationOrigin {
        ScoringInvalidationOrigin::manual_grade(
            crate::ScoringInvalidationOriginId::from_uuid(Uuid::from_u128(id)),
            UserId::from_uuid(Uuid::from_u128(4)),
        )
    }

    #[test]
    fn origin_families_hold_only_stable_identity_and_authority() {
        let actor = UserId::from_uuid(Uuid::from_u128(5));
        let id = crate::ScoringInvalidationOriginId::from_uuid(Uuid::from_u128(6));
        assert_eq!(
            ScoringInvalidationOrigin::instructor_recalculation(id, actor).actor,
            Some(actor)
        );
        assert_eq!(
            ScoringInvalidationOrigin::manual_grade(id, actor).actor,
            Some(actor)
        );
        assert_eq!(
            ScoringInvalidationOrigin::learner_support(id, actor).actor,
            Some(actor)
        );
        assert!(
            ScoringInvalidationOrigin::accepted_submission_completion(id)
                .actor
                .is_none()
        );
    }

    #[test]
    fn definition_change_without_scores_advances_only_its_generation_fence() {
        let (generation, status, requires_invalidation) = definition_scoring_state(
            ScoringGeneration::INITIAL,
            ScoringStatus::Current,
            true,
            false,
        )
        .expect("next generation");
        assert_eq!(
            generation,
            ScoringGeneration::new(2).expect("positive generation")
        );
        assert_eq!(status, ScoringStatus::Current);
        assert!(!requires_invalidation);
    }

    #[test]
    fn exact_origin_replays_and_newer_origin_supersedes_active_generation() {
        let mut state = State::default();
        state.assignment_scoring.insert(
            (tenant(), assignment()),
            (ScoringGeneration::INITIAL, ScoringStatus::Current),
        );
        let first = request_scoring_invalidation(
            &mut state,
            tenant(),
            course(),
            assignment(),
            origin(7),
            JobId::from_uuid(Uuid::from_u128(7)),
        )
        .expect("first origin");
        let replay = request_scoring_invalidation(
            &mut state,
            tenant(),
            course(),
            assignment(),
            origin(7),
            JobId::from_uuid(Uuid::from_u128(7)),
        )
        .expect("exact replay");
        assert_eq!(replay, first);
        let second = request_scoring_invalidation(
            &mut state,
            tenant(),
            course(),
            assignment(),
            origin(8),
            JobId::from_uuid(Uuid::from_u128(8)),
        )
        .expect("new origin");
        assert!(second.generation.value() > first.generation.value());
        assert_eq!(
            state.automated_grading_operations[&(tenant(), first.operation)].state,
            GradingOperationState::Superseded
        );
    }

    #[test]
    fn terminal_projection_changes_only_the_matching_generation_thread() {
        let mut state = State::default();
        state.assignment_scoring.insert(
            (tenant(), assignment()),
            (ScoringGeneration::INITIAL, ScoringStatus::Current),
        );
        let first = request_scoring_invalidation(
            &mut state,
            tenant(),
            course(),
            assignment(),
            origin(9),
            JobId::from_uuid(Uuid::from_u128(9)),
        )
        .expect("first origin");
        let second = request_scoring_invalidation(
            &mut state,
            tenant(),
            course(),
            assignment(),
            origin(10),
            JobId::from_uuid(Uuid::from_u128(10)),
        )
        .expect("second origin");
        grading_operation_lifecycle::project_assignment_scoring_operation(
            &mut state,
            tenant(),
            assignment(),
            first.generation,
            ScoringStatus::Failed,
        )
        .expect("stale failure projection");
        grading_operation_lifecycle::project_assignment_scoring_operation(
            &mut state,
            tenant(),
            assignment(),
            second.generation,
            ScoringStatus::Current,
        )
        .expect("current success projection");
        assert_eq!(
            state.automated_grading_operations[&(tenant(), first.operation)].state,
            GradingOperationState::Superseded
        );
        assert_eq!(
            state.automated_grading_operations[&(tenant(), second.operation)].state,
            GradingOperationState::Completed
        );
    }

    #[test]
    fn exact_failed_generation_reopens_its_own_operation() {
        let mut state = State::default();
        state.assignment_scoring.insert(
            (tenant(), assignment()),
            (ScoringGeneration::INITIAL, ScoringStatus::Current),
        );
        let invalidation = request_scoring_invalidation(
            &mut state,
            tenant(),
            course(),
            assignment(),
            origin(11),
            JobId::from_uuid(Uuid::from_u128(11)),
        )
        .expect("origin");
        grading_operation_lifecycle::project_assignment_scoring_operation(
            &mut state,
            tenant(),
            assignment(),
            invalidation.generation,
            ScoringStatus::Failed,
        )
        .expect("failure projection");
        let operation = state.automated_grading_operations[&(tenant(), invalidation.operation)];
        assert_eq!(
            operation.reason,
            GradingOperationReason::ScoringRecalculationFailed
        );
        assert_eq!(operation.state, GradingOperationState::Actionable);
        assert_eq!(
            operation.next_action,
            Some(GradingOperationAction::Recalculate)
        );
        request_scoring_invalidation(
            &mut state,
            tenant(),
            course(),
            assignment(),
            origin(12),
            JobId::from_uuid(Uuid::from_u128(12)),
        )
        .expect("new origin supersedes retryable predecessor");
        assert_eq!(
            state.automated_grading_operations[&(tenant(), invalidation.operation)].state,
            GradingOperationState::Superseded
        );
    }

    #[test]
    fn supersession_validates_every_revision_before_mutating_any_thread() {
        let mut state = State::default();
        state.assignment_scoring.insert(
            (tenant(), assignment()),
            (ScoringGeneration::INITIAL, ScoringStatus::Current),
        );
        let first = request_scoring_invalidation(
            &mut state,
            tenant(),
            course(),
            assignment(),
            origin(13),
            JobId::from_uuid(Uuid::from_u128(13)),
        )
        .expect("first origin");
        let second = request_scoring_invalidation(
            &mut state,
            tenant(),
            course(),
            assignment(),
            origin(14),
            JobId::from_uuid(Uuid::from_u128(14)),
        )
        .expect("second origin");
        let first_operation = state
            .automated_grading_operations
            .get_mut(&(tenant(), first.operation))
            .expect("first operation");
        first_operation.state = GradingOperationState::Actionable;
        first_operation.revision = GradingOperationRevision::from_u64(u64::MAX).expect("max");
        assert!(matches!(
            request_scoring_invalidation(
                &mut state,
                tenant(),
                course(),
                assignment(),
                origin(15),
                JobId::from_uuid(Uuid::from_u128(15)),
            ),
            Err(StoreError::Conflict)
        ));
        assert_eq!(
            state.automated_grading_operations[&(tenant(), second.operation)].state,
            GradingOperationState::ActionInProgress
        );
    }
}
