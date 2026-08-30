//! Private receipt identity, integrity, and repair helpers for Memory adoption state.

use question_model::{
    CourseInstanceReceiptTarget, CurriculumAdoptionIdempotencyKey, CurriculumReplayStatus, UserId,
};

use super::{
    AssignmentAdoptionEvidenceDetail, CurriculumAdoptionOperation, CurriculumAdoptionRequestDigest,
    CurriculumAdoptionState, MemoryCurriculumAdoptionEvidence, MemoryCurriculumAdoptionOutcome,
    MemoryCurriculumAdoptionReceipt, StoredAssignmentAdoptionEvidence, StoredAssignmentImport,
};
use crate::StoreError;
use crate::in_memory::State;

/// Returns an exact replay, refusing a reused global actor/key with another meaning.
pub(crate) fn lookup_replay_or_conflict(
    state: &State,
    actor: UserId,
    key: &CurriculumAdoptionIdempotencyKey,
    operation: CurriculumAdoptionOperation,
    digest: CurriculumAdoptionRequestDigest,
) -> Result<Option<MemoryCurriculumAdoptionOutcome>, StoreError> {
    let Some(receipt) = state
        .curriculum_adoption
        .receipts
        .get(&(actor, key.clone()))
    else {
        return Ok(None);
    };
    validate_receipt_evidence(state, receipt)?;
    if receipt.operation != operation || receipt.request_digest != digest {
        return Err(StoreError::Conflict);
    }
    Ok(Some(receipt.outcome.clone()))
}

/// Stores one validated immutable receipt; occupied receipt identities remain conflicts.
pub(crate) fn store_completed_receipt(
    state: &mut State,
    receipt: MemoryCurriculumAdoptionReceipt,
) -> Result<(), StoreError> {
    validate_receipt_evidence(state, &receipt)?;
    let identity = (receipt.actor, receipt.idempotency_key.clone());
    if state.curriculum_adoption.receipts.contains_key(&identity)
        || state
            .curriculum_adoption
            .receipt_targets
            .contains_key(&identity)
    {
        return Err(StoreError::Conflict);
    }
    if let MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(target) = &receipt.evidence {
        state
            .curriculum_adoption
            .receipt_targets
            .insert(identity.clone(), target.as_ref().clone());
    }
    state.curriculum_adoption.receipts.insert(identity, receipt);
    Ok(())
}

/// Validates only immutable receipt-to-evidence binding before replay or repair.
pub(crate) fn validate_receipt_evidence(
    state: &State,
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
            MemoryCurriculumAdoptionOutcome::AdoptBlueprintAssignment { course, assignment },
            MemoryCurriculumAdoptionEvidence::AdoptBlueprintAssignment {
                source,
                precondition,
                outcome,
                applied_assignment,
                import_revision,
            },
        ) if precondition.course == outcome.course
            && *course == outcome.course
            && *assignment == applied_assignment.assignment
            && AdoptAssignmentReceiptEvidence {
                receipt,
                source,
                precondition,
                outcome,
                applied_assignment,
                import_revision,
            }
            .validate(state)? =>
        {
            Ok(())
        }
        (
            MemoryCurriculumAdoptionOutcome::InstantiateBlueprintCourse { course },
            MemoryCurriculumAdoptionEvidence::InstantiateBlueprintCourse {
                source,
                destination,
                blueprint_application,
            },
        ) if *course == destination.course => validate_whole_course_receipt(
            state,
            receipt,
            destination,
            *blueprint_application,
            Some(*source),
        ),
        (_, MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(target))
            if course_instance_outcome_course(&receipt.outcome)
                == Some(target.destination().course) =>
        {
            validate_receipt_target(receipt, target)?;
            validate_assignment_receipt_target(state, target)?;
            if let CourseInstanceReceiptTarget::Rollover(rollover) = target.as_ref() {
                validate_whole_course_receipt(
                    state,
                    receipt,
                    rollover.created_course_instance(),
                    rollover.created_blueprint_application(),
                    Some(rollover.created_blueprint_application().source),
                )?;
            }
            Ok(())
        }
        _ => Err(StoreError::Conflict),
    }
}

/// Proves that one repairable import projection resolves to its exact immutable receipt chain.
///
/// ASVS 2.2.3 and 2.3.1: the projection, evidence row, receipt identity, and operation-specific
/// receipt facts are validated as one contextual business-logic unit before inspection exposes
/// provenance. Any missing or contradictory server-owned fact is an integrity failure.
pub(crate) fn validate_assignment_import_projection<'a>(
    state: &'a State,
    course: question_model::CourseReference,
    assignment: question_model::AssignmentId,
    assignment_reference: question_model::AssignmentReference,
    projection: &StoredAssignmentImport,
) -> Result<&'a StoredAssignmentAdoptionEvidence, StoreError> {
    let evidence = state
        .curriculum_adoption
        .assignment_evidence
        .get(&(assignment, projection.import_revision))
        .ok_or_else(|| inspection_integrity("immutable assignment evidence"))?;
    if projection.receipt_actor != evidence.receipt_actor
        || projection.receipt_key != evidence.receipt_key
        || projection.import_revision != evidence.import_revision()
        || evidence.assignment() != assignment_reference
        || evidence.outcome().course != course
        || !evidence
            .outcome()
            .assignments()
            .iter()
            .any(|observed| observed.assignment == assignment_reference)
    {
        return Err(inspection_integrity("assignment import projection binding"));
    }

    let receipt_identity = (projection.receipt_actor, projection.receipt_key.clone());
    let receipt = state
        .curriculum_adoption
        .receipts
        .get(&receipt_identity)
        .ok_or_else(|| inspection_integrity("completed assignment import receipt"))?;
    if receipt.actor != projection.receipt_actor
        || receipt.idempotency_key != projection.receipt_key
    {
        return Err(inspection_integrity("assignment import receipt identity"));
    }
    validate_receipt_evidence(state, receipt)
        .map_err(|_| inspection_integrity("completed assignment import receipt evidence"))?;
    if !receipt_identifies_assignment_import(
        receipt,
        evidence,
        course,
        assignment_reference,
        projection.import_revision,
    ) {
        return Err(inspection_integrity("assignment import receipt binding"));
    }
    Ok(evidence)
}

fn receipt_identifies_assignment_import(
    receipt: &MemoryCurriculumAdoptionReceipt,
    evidence: &StoredAssignmentAdoptionEvidence,
    course: question_model::CourseReference,
    assignment: question_model::AssignmentReference,
    import_revision: question_model::CurriculumImportRevision,
) -> bool {
    match &evidence.detail {
        AssignmentAdoptionEvidenceDetail::AdoptBlueprintAssignment { .. } => {
            receipt_identifies_adopted_assignment(
                receipt,
                evidence,
                course,
                assignment,
                import_revision,
            )
        }
        AssignmentAdoptionEvidenceDetail::ControlledUpdate { .. } => matches!(
            (&receipt.outcome, &receipt.evidence),
            (
                MemoryCurriculumAdoptionOutcome::ControlledUpdateBlueprintAssignment {
                    course: receipt_course,
                    assignment: receipt_assignment,
                },
                MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(target),
            ) if *receipt_course == course
                && *receipt_assignment == assignment
                && matches!(target.as_ref(), CourseInstanceReceiptTarget::ControlledUpdate(_))
                && assignment_target_matches(
                    target.as_ref(),
                    evidence,
                    course,
                    assignment,
                    import_revision,
                )
        ),
        AssignmentAdoptionEvidenceDetail::SelectedCopy { .. } => matches!(
            (&receipt.outcome, &receipt.evidence),
            (
                MemoryCurriculumAdoptionOutcome::CreateSelectedBlueprintAssignment {
                    course: receipt_course,
                    assignment: receipt_assignment,
                },
                MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(target),
            ) if *receipt_course == course
                && *receipt_assignment == assignment
                && matches!(target.as_ref(), CourseInstanceReceiptTarget::SelectedCopy(_))
                && assignment_target_matches(
                    target.as_ref(),
                    evidence,
                    course,
                    assignment,
                    import_revision,
                )
        ),
    }
}

fn receipt_identifies_adopted_assignment(
    receipt: &MemoryCurriculumAdoptionReceipt,
    evidence: &StoredAssignmentAdoptionEvidence,
    course: question_model::CourseReference,
    assignment: question_model::AssignmentReference,
    import_revision: question_model::CurriculumImportRevision,
) -> bool {
    match (&receipt.outcome, &receipt.evidence) {
        (
            MemoryCurriculumAdoptionOutcome::AdoptBlueprintAssignment {
                course: receipt_course,
                assignment: receipt_assignment,
            },
            MemoryCurriculumAdoptionEvidence::AdoptBlueprintAssignment {
                source,
                outcome,
                applied_assignment,
                import_revision: receipt_revision,
                ..
            },
        ) => {
            *receipt_course == course
                && *receipt_assignment == assignment
                && *source == evidence.source
                && outcome.course == course
                && applied_assignment.assignment == assignment
                && *receipt_revision == import_revision
        }
        (
            MemoryCurriculumAdoptionOutcome::InstantiateBlueprintCourse {
                course: receipt_course,
            },
            MemoryCurriculumAdoptionEvidence::InstantiateBlueprintCourse {
                source,
                destination,
                ..
            },
        ) => {
            *receipt_course == course
                && destination.course == course
                && destination
                    .assignments()
                    .iter()
                    .any(|observed| observed.assignment == assignment)
                && evidence.source.source() == *source
                && import_revision.value() == 1
        }
        (
            MemoryCurriculumAdoptionOutcome::RolloverCourseInstance {
                course: receipt_course,
            },
            MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(target),
        ) => {
            let CourseInstanceReceiptTarget::Rollover(rollover) = target.as_ref() else {
                return false;
            };
            let copied_source = rollover
                .created_course_instance()
                .assignments()
                .iter()
                .position(|observed| observed.assignment == assignment)
                .and_then(|index| rollover.manifest().copied.assignments().get(index));
            *receipt_course == course
                && rollover.created_course_instance().course == course
                && copied_source == Some(&evidence.source)
                && import_revision.value() == 1
        }
        _ => false,
    }
}

fn assignment_target_matches(
    target: &CourseInstanceReceiptTarget,
    evidence: &StoredAssignmentAdoptionEvidence,
    course: question_model::CourseReference,
    assignment: question_model::AssignmentReference,
    import_revision: question_model::CurriculumImportRevision,
) -> bool {
    target.assignment_import_target().is_some_and(|locator| {
        locator.receipt_actor() == evidence.receipt_actor
            && locator.receipt_key() == &evidence.receipt_key
            && locator.course() == course
            && locator.assignment() == assignment
            && locator.import_revision() == import_revision
    })
}

fn inspection_integrity(missing: &str) -> StoreError {
    super::destination::integrity(&format!("CourseInstance inspection {missing}"))
}

/// Proves that a course-level materialization has its exact immutable Memory row.
fn validate_whole_course_receipt(
    state: &State,
    receipt: &MemoryCurriculumAdoptionReceipt,
    destination: &question_model::CourseInstanceWitness,
    application: question_model::CourseInstanceBlueprintApplication,
    source: Option<question_model::ObservedBlueprintSource>,
) -> Result<(), StoreError> {
    if source != Some(application.source) {
        return Err(StoreError::Conflict);
    }
    let course = super::resolve_course(state, destination.course)?;
    if super::course_instance_blueprint_application(state, course)? != application {
        return Err(StoreError::Conflict);
    }
    let stored = state
        .curriculum_adoption
        .whole_course_adoptions
        .get(&course)
        .ok_or(StoreError::Conflict)?;
    (stored.receipt_actor == receipt.actor
        && stored.receipt_key == receipt.idempotency_key
        && stored.destination == *destination
        && stored.blueprint_application == application)
        .then_some(())
        .ok_or(StoreError::Conflict)
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
    state: &State,
    target: &CourseInstanceReceiptTarget,
) -> Result<CourseInstanceReceiptTarget, StoreError> {
    let identity = (target.authorized_actor(), target.idempotency_key().clone());
    let stored = state
        .curriculum_adoption
        .receipt_targets
        .get(&identity)
        .ok_or(StoreError::Conflict)?;
    if stored != target {
        return Err(StoreError::Conflict);
    }
    let receipt = state
        .curriculum_adoption
        .receipts
        .get(&identity)
        .ok_or(StoreError::Conflict)?;
    validate_receipt_evidence(state, receipt)?;
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
            import_revision: evidence.import_revision(),
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
    match (&receipt.outcome, target) {
        (
            MemoryCurriculumAdoptionOutcome::ControlledUpdateBlueprintAssignment {
                course,
                assignment,
            },
            CourseInstanceReceiptTarget::ControlledUpdate(target),
        ) if *course == target.binding().outcome().course
            && *assignment == target.applied().assignment().assignment => {}
        (
            MemoryCurriculumAdoptionOutcome::CreateSelectedBlueprintAssignment {
                course,
                assignment,
            },
            CourseInstanceReceiptTarget::SelectedCopy(target),
        ) if *course == target.binding().outcome().course
            && *assignment == target.applied().assignment().assignment => {}
        (_, CourseInstanceReceiptTarget::ControlledUpdate(_))
        | (_, CourseInstanceReceiptTarget::SelectedCopy(_)) => return Err(StoreError::Conflict),
        _ => {}
    }
    Ok(())
}

/// Validates the M4 receipt facts against one immutable assignment-import row.
///
/// M5 resolves the locator to its map key under its one write lock.  This
/// validation is intentionally exhaustive over the closed stored-detail enum,
/// so a receipt cannot be accepted with partial or cross-operation evidence.
fn validate_assignment_receipt_target(
    state: &State,
    target: &CourseInstanceReceiptTarget,
) -> Result<(), StoreError> {
    let course = super::resolve_course(state, target.destination().course)?;
    if super::course_instance_blueprint_application(state, course)?
        != target.blueprint_application()
    {
        return Err(StoreError::Conflict);
    }
    if let CourseInstanceReceiptTarget::Reconcile(receipt) = target {
        return validate_reconciliation_original_import(state, receipt);
    }
    let Some(locator) = target.assignment_import_target() else {
        return Ok(());
    };
    if locator.course() != target.destination().course {
        return Err(StoreError::Conflict);
    }
    let assignment = assignment_id_for_reference(state, course, locator.assignment())?;
    let evidence = state
        .curriculum_adoption
        .assignment_evidence
        .get(&(assignment, locator.import_revision()))
        .ok_or(StoreError::Conflict)?;
    match (target, &evidence.detail) {
        (
            CourseInstanceReceiptTarget::ControlledUpdate(receipt),
            AssignmentAdoptionEvidenceDetail::ControlledUpdate {
                precondition,
                outcome,
                effect,
                replacements,
                semantic_digest,
                applied_assignment,
                import_revision,
            },
        ) if evidence.receipt_actor == locator.receipt_actor()
            && evidence.receipt_key == *locator.receipt_key()
            && evidence.source == receipt.applied().source()
            && *precondition == *receipt.binding().precondition()
            && *outcome == *receipt.binding().outcome()
            && *effect == receipt.effect()
            && *replacements == *receipt.applied().replacements()
            && *semantic_digest == receipt.applied().semantic_digest()
            && *applied_assignment == receipt.applied().assignment()
            && *import_revision == receipt.applied().import_revision()
            && outcome.assignments().contains(applied_assignment)
            && validate_controlled_import_progression(state, assignment, receipt)? =>
        {
            Ok(())
        }
        (
            CourseInstanceReceiptTarget::SelectedCopy(receipt),
            AssignmentAdoptionEvidenceDetail::SelectedCopy {
                precondition,
                outcome,
                replacements,
                semantic_digest,
                applied_assignment,
                import_revision,
            },
        ) if evidence.receipt_actor == locator.receipt_actor()
            && evidence.receipt_key == *locator.receipt_key()
            && evidence.source == receipt.applied().source()
            && *precondition == *receipt.binding().precondition()
            && *outcome == *receipt.binding().outcome()
            && *replacements == *receipt.applied().replacements()
            && *semantic_digest == receipt.applied().semantic_digest()
            && *applied_assignment == receipt.applied().assignment()
            && *import_revision == receipt.applied().import_revision()
            && outcome.assignments().contains(applied_assignment) =>
        {
            Ok(())
        }
        _ => Err(StoreError::Conflict),
    }
}

fn validate_reconciliation_original_import(
    state: &State,
    receipt: &question_model::ReconcileCourseInstanceAdoptionReceipt,
) -> Result<(), StoreError> {
    let locator = receipt.original_import_target();
    if locator.course() != receipt.binding().outcome().course {
        return Err(StoreError::Conflict);
    }
    let course = super::resolve_course(state, locator.course())?;
    let original_identity = (locator.receipt_actor(), locator.receipt_key().clone());
    let original_target = state
        .curriculum_adoption
        .receipt_targets
        .get(&original_identity)
        .ok_or(StoreError::Conflict)?;
    if original_target.assignment_import_target().as_ref() != Some(locator) {
        return Err(StoreError::Conflict);
    }
    let original_receipt = state
        .curriculum_adoption
        .receipts
        .get(&original_identity)
        .ok_or(StoreError::Conflict)?;
    validate_receipt_evidence(state, original_receipt)?;
    let assignment = assignment_id_for_reference(state, course, locator.assignment())?;
    let evidence = state
        .curriculum_adoption
        .assignment_evidence
        .get(&(assignment, locator.import_revision()))
        .ok_or(StoreError::Conflict)?;
    if evidence.receipt_actor != locator.receipt_actor()
        || evidence.receipt_key != *locator.receipt_key()
        || evidence.outcome().course != locator.course()
        || evidence.assignment() != locator.assignment()
        || evidence.import_revision() != locator.import_revision()
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

/// One cohesive, borrowed view of the immutable facts that authorize an assignment receipt.
///
/// ASVS 2.2.3 and 2.3.1: validating the related actor, key, source, state transition, applied
/// assignment, and import revision as one unit prevents partial or cross-operation evidence from
/// authorizing replay.
struct AdoptAssignmentReceiptEvidence<'a> {
    receipt: &'a MemoryCurriculumAdoptionReceipt,
    source: &'a question_model::AssignmentDefinitionSourceView,
    precondition: &'a question_model::CourseInstanceWitness,
    outcome: &'a question_model::CourseInstanceWitness,
    applied_assignment: &'a question_model::ObservedCourseInstanceAssignment,
    import_revision: &'a question_model::CurriculumImportRevision,
}

impl AdoptAssignmentReceiptEvidence<'_> {
    fn validate(&self, state: &State) -> Result<bool, StoreError> {
        let course = super::resolve_course(state, self.outcome.course)?;
        let _blueprint_application = super::course_instance_blueprint_application(state, course)?;
        let assignment =
            assignment_id_for_reference(state, course, self.applied_assignment.assignment)?;
        let Some(evidence) = state
            .curriculum_adoption
            .assignment_evidence
            .get(&(assignment, *self.import_revision))
        else {
            return Ok(false);
        };
        Ok(matches!(
            &evidence.detail,
            AssignmentAdoptionEvidenceDetail::AdoptBlueprintAssignment {
                precondition: evidence_precondition,
                outcome: evidence_outcome,
                applied_assignment,
                import_revision: evidence_revision,
            } if evidence.receipt_actor == self.receipt.actor
                && evidence.receipt_key == self.receipt.idempotency_key
                && evidence.source == *self.source
                && evidence_precondition == self.precondition
                && evidence_outcome == self.outcome
                && applied_assignment == self.applied_assignment
                && !self.precondition.assignments().contains(applied_assignment)
                && self.outcome.assignments().contains(applied_assignment)
                && evidence_revision == self.import_revision
        ))
    }
}

fn assignment_id_for_reference(
    state: &State,
    course: question_model::CourseId,
    reference: question_model::AssignmentReference,
) -> Result<question_model::AssignmentId, StoreError> {
    let assignment = *state
        .assignments_by_reference
        .get(&reference)
        .ok_or(StoreError::NotFound)?;
    state
        .assignments
        .get(&assignment)
        .filter(|record| record.course_id == course)
        .map(|_| assignment)
        .ok_or(StoreError::NotFound)
}

fn validate_controlled_import_progression(
    state: &State,
    assignment: question_model::AssignmentId,
    receipt: &question_model::ControlledUpdateBlueprintAssignmentReceipt,
) -> Result<bool, StoreError> {
    let consumed = receipt.consumed_import();
    let prior = state
        .curriculum_adoption
        .assignment_evidence
        .get(&(assignment, consumed.import_revision))
        .ok_or(StoreError::Conflict)?;
    let Some(next_revision) = consumed
        .import_revision
        .value()
        .checked_add(1)
        .and_then(question_model::CurriculumImportRevision::new)
    else {
        return Ok(false);
    };
    Ok(prior.source == consumed.source
        && prior.assignment() == consumed.destination.assignment
        && prior.outcome().course == receipt.binding().precondition().course
        && prior
            .outcome()
            .assignments()
            .contains(&consumed.destination)
        && prior.import_revision() == consumed.import_revision
        && receipt.applied().import_revision() == next_revision)
}
