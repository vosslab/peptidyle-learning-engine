//! Private receipt identity, integrity, and repair helpers for Memory adoption state.

use question_model::{
    CourseInstanceReceiptTarget, CurriculumAdoptionIdempotencyKey, CurriculumReplayStatus, UserId,
};

use super::{
    CurriculumAdoptionOperation, CurriculumAdoptionRequestDigest, CurriculumAdoptionState,
    MemoryCurriculumAdoptionEvidence, MemoryCurriculumAdoptionOutcome,
    MemoryCurriculumAdoptionReceipt, StoredAssignmentImport,
};
use crate::StoreError;

/// Returns an exact replay, refusing a reused global actor/key with another meaning.
pub(crate) fn lookup_replay_or_conflict(
    state: &CurriculumAdoptionState,
    actor: UserId,
    key: &CurriculumAdoptionIdempotencyKey,
    operation: CurriculumAdoptionOperation,
    digest: CurriculumAdoptionRequestDigest,
) -> Result<Option<MemoryCurriculumAdoptionOutcome>, StoreError> {
    let Some(receipt) = state.receipts.get(&(actor, key.clone())) else {
        return Ok(None);
    };
    validate_receipt_evidence(receipt)?;
    if receipt.operation != operation || receipt.request_digest != digest {
        return Err(StoreError::Conflict);
    }
    Ok(Some(receipt.outcome.clone()))
}

/// Stores one validated immutable receipt; occupied receipt identities remain conflicts.
pub(crate) fn store_completed_receipt(
    state: &mut CurriculumAdoptionState,
    receipt: MemoryCurriculumAdoptionReceipt,
) -> Result<(), StoreError> {
    validate_receipt_evidence(&receipt)?;
    let identity = (receipt.actor, receipt.idempotency_key.clone());
    if state.receipts.contains_key(&identity) || state.receipt_targets.contains_key(&identity) {
        return Err(StoreError::Conflict);
    }
    if let MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(target) = &receipt.evidence {
        state
            .receipt_targets
            .insert(identity.clone(), target.clone());
    }
    state.receipts.insert(identity, receipt);
    Ok(())
}

/// Validates only immutable receipt-to-evidence binding before replay or repair.
pub(crate) fn validate_receipt_evidence(
    receipt: &MemoryCurriculumAdoptionReceipt,
) -> Result<(), StoreError> {
    if receipt.operation != receipt.outcome.operation() {
        return Err(StoreError::Conflict);
    }
    match (&receipt.outcome, &receipt.evidence) {
        (
            MemoryCurriculumAdoptionOutcome::ForkBlueprintCourse { source, created },
            MemoryCurriculumAdoptionEvidence::ForkBlueprintCourse {
                source: evidence_source,
                created: evidence_created,
            },
        ) if source == evidence_source && created == evidence_created => Ok(()),
        (
            MemoryCurriculumAdoptionOutcome::AdoptBlueprintAssignment { course, .. },
            MemoryCurriculumAdoptionEvidence::AdoptBlueprintAssignment { destination, .. },
        ) if *course == destination.course => Ok(()),
        (
            MemoryCurriculumAdoptionOutcome::InstantiateBlueprintCourse { course },
            MemoryCurriculumAdoptionEvidence::InstantiateBlueprintCourse { destination, .. },
        ) if *course == destination.course => Ok(()),
        (_, MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(target))
            if course_instance_outcome_course(&receipt.outcome)
                == Some(target.destination().course) =>
        {
            validate_receipt_target(receipt, target)
        }
        _ => Err(StoreError::Conflict),
    }
}

fn course_instance_outcome_course(
    outcome: &MemoryCurriculumAdoptionOutcome,
) -> Option<question_model::CourseReference> {
    match outcome {
        MemoryCurriculumAdoptionOutcome::RolloverCourseInstance { course }
        | MemoryCurriculumAdoptionOutcome::ShiftCourseInstanceTerm { course }
        | MemoryCurriculumAdoptionOutcome::ControlledUpdateBlueprintAssignment { course, .. }
        | MemoryCurriculumAdoptionOutcome::CreateSelectedBlueprintAssignment { course, .. }
        | MemoryCurriculumAdoptionOutcome::ReconcileCourseInstanceAdoption { course } => {
            Some(*course)
        }
        MemoryCurriculumAdoptionOutcome::ForkBlueprintCourse { .. }
        | MemoryCurriculumAdoptionOutcome::AdoptBlueprintAssignment { .. }
        | MemoryCurriculumAdoptionOutcome::InstantiateBlueprintCourse { .. } => None,
    }
}

/// Resolves an exact retained reconciliation target without accepting a forged substitute.
pub(crate) fn resolve_reconciliation_target(
    state: &CurriculumAdoptionState,
    target: &CourseInstanceReceiptTarget,
) -> Result<CourseInstanceReceiptTarget, StoreError> {
    let identity = (target.authorized_actor(), target.idempotency_key().clone());
    let stored = state
        .receipt_targets
        .get(&identity)
        .ok_or(StoreError::Conflict)?;
    if stored != target {
        return Err(StoreError::Conflict);
    }
    let receipt = state.receipts.get(&identity).ok_or(StoreError::Conflict)?;
    validate_receipt_evidence(receipt)?;
    Ok(stored.clone())
}

/// Rebuilds only an existing derived assignment-import projection from immutable evidence.
pub(crate) fn rebuild_current_projection(
    state: &mut CurriculumAdoptionState,
    assignment: question_model::AssignmentId,
    revision: question_model::CurriculumImportRevision,
) -> Result<(), StoreError> {
    let evidence = state
        .assignment_evidence
        .get(&(assignment, revision))
        .ok_or(StoreError::Conflict)?;
    state.import_records.insert(
        assignment,
        StoredAssignmentImport {
            receipt_actor: evidence.receipt_actor,
            receipt_key: evidence.receipt_key.clone(),
            import_revision: evidence.import_revision,
        },
    );
    Ok(())
}

pub(crate) fn completed_response(
    outcome: &MemoryCurriculumAdoptionOutcome,
    replay: bool,
) -> Result<question_model::CurriculumAdoptionCompleted, StoreError> {
    outcome
        .completed(if replay {
            CurriculumReplayStatus::Replayed
        } else {
            CurriculumReplayStatus::Applied
        })
        .ok_or(StoreError::Conflict)
}

fn validate_receipt_target(
    receipt: &MemoryCurriculumAdoptionReceipt,
    target: &CourseInstanceReceiptTarget,
) -> Result<(), StoreError> {
    let target_operation = match target.operation() {
        question_model::CourseInstanceOperationKind::Rollover => {
            CurriculumAdoptionOperation::RolloverCourseInstance
        }
        question_model::CourseInstanceOperationKind::ShiftTerm => {
            CurriculumAdoptionOperation::ShiftCourseInstanceTerm
        }
        question_model::CourseInstanceOperationKind::ControlledUpdate => {
            CurriculumAdoptionOperation::ControlledUpdateBlueprintAssignment
        }
        question_model::CourseInstanceOperationKind::SelectedCopy => {
            CurriculumAdoptionOperation::CreateSelectedBlueprintAssignment
        }
        question_model::CourseInstanceOperationKind::Reconcile => {
            CurriculumAdoptionOperation::ReconcileCourseInstanceAdoption
        }
    };
    if receipt.operation != target_operation
        || receipt.actor != target.authorized_actor()
        || receipt.idempotency_key != *target.idempotency_key()
        || receipt.request_digest.as_bytes() != &target.request_digest()
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}
