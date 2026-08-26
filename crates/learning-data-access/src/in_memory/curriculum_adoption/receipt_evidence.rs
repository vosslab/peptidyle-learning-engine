//! Canonical receipt, immutable evidence, and outcome binding validation.

use question_model::curriculum_adoption::{CurriculumSemanticDigest, CurriculumSemanticPayload};
use question_model::{
    ActivityTimestamp, AssignmentId, CourseId, CurriculumAdoptionIdempotencyKey, TenantId, UserId,
};

use super::{
    CurriculumAdoptionOperation, MemoryCurriculumAdoptionOutcome, MemoryCurriculumAdoptionReceipt,
    RolloverAssignmentProvenance, StoredAlphaForkLineage, StoredAssignmentAdoptionEvidence,
    StoredAssignmentImport, StoredAssignmentImportSource, StoredWholeCourseAdoption,
    StoredWholeCourseOrigin, destination, require_course_instructor, resolve_course,
};
use crate::StoreError;
use crate::curriculum_adoption::CurriculumAdoptionRequestDigest;
use crate::curriculum_adoption::{ObservedSemanticEnvelope, validate_semantic_evidence};
use crate::in_memory::State;
use crate::in_memory::reusable_curriculum;

pub(crate) fn matching_receipt(
    state: &State,
    tenant: TenantId,
    key: &CurriculumAdoptionIdempotencyKey,
    operation: CurriculumAdoptionOperation,
    actor: UserId,
    digest: CurriculumAdoptionRequestDigest,
) -> Result<Option<MemoryCurriculumAdoptionOutcome>, StoreError> {
    let Some(receipt) = state
        .curriculum_adoption
        .receipts
        .get(&(tenant, key.clone()))
    else {
        return Ok(None);
    };
    if receipt.operation != operation || receipt.actor != actor || receipt.request_sha256 != digest
    {
        return Err(StoreError::Conflict);
    }
    ensure_completed_outcome_binding(state, tenant, key, receipt)?;
    authorize_receipt_outcome(state, tenant, actor, &receipt.completed)?;
    Ok(Some(receipt.completed.clone()))
}

fn authorize_receipt_outcome(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    outcome: &MemoryCurriculumAdoptionOutcome,
) -> Result<(), StoreError> {
    let require_destination = |reference| {
        require_course_instructor(
            state,
            tenant,
            resolve_course(state, tenant, reference)?,
            actor,
        )
    };
    match outcome {
        MemoryCurriculumAdoptionOutcome::ForkAlpha { .. } => Ok(()),
        MemoryCurriculumAdoptionOutcome::InstantiateBlueprint { course, .. }
        | MemoryCurriculumAdoptionOutcome::InstantiateAlpha { course, .. }
        | MemoryCurriculumAdoptionOutcome::ShiftCourseTerm { course, .. }
        | MemoryCurriculumAdoptionOutcome::FastForwardAssignment { course, .. }
        | MemoryCurriculumAdoptionOutcome::CreateSourceDerivedAssignment { course, .. } => {
            require_destination(*course)
        }
        MemoryCurriculumAdoptionOutcome::RolloverCourse {
            source_course,
            course,
        } => {
            require_destination(*source_course)?;
            require_destination(*course)
        }
    }
}

/// Binds every completed outcome through its forward/reverse locators and its
/// immutable receipt-led evidence before authorization can expose a replay.
pub(crate) fn ensure_completed_outcome_binding(
    state: &State,
    tenant: TenantId,
    key: &CurriculumAdoptionIdempotencyKey,
    receipt: &MemoryCurriculumAdoptionReceipt,
) -> Result<(), StoreError> {
    match &receipt.completed {
        MemoryCurriculumAdoptionOutcome::ForkAlpha { source, alpha } => {
            validate_alpha_fork(state, *alpha, source, key, receipt)
        }
        MemoryCurriculumAdoptionOutcome::InstantiateBlueprint { course, assignment }
        | MemoryCurriculumAdoptionOutcome::CreateSourceDerivedAssignment { course, assignment } => {
            let id = resolve_assignment_reference(state, tenant, *assignment)?;
            ensure_assignment_course(state, tenant, id, *course)?;
            validate_assignment_evidence(state, tenant, key, receipt.actor, receipt.occurred_at, id)
                .map(|_| ())
        }
        MemoryCurriculumAdoptionOutcome::FastForwardAssignment {
            course,
            assignment,
            import_revision,
        } => {
            let id = resolve_assignment_reference(state, tenant, *assignment)?;
            ensure_assignment_course(state, tenant, id, *course)?;
            let evidence = validate_assignment_evidence(
                state,
                tenant,
                key,
                receipt.actor,
                receipt.occurred_at,
                id,
            )?;
            if evidence.baseline.revision != *import_revision {
                return Err(destination::integrity(
                    "completed fast-forward import revision",
                ));
            }
            Ok(())
        }
        MemoryCurriculumAdoptionOutcome::InstantiateAlpha { course, .. }
        | MemoryCurriculumAdoptionOutcome::RolloverCourse { course, .. } => {
            let course_id = resolve_course(state, tenant, *course)?;
            validate_whole_course_receipt_binding(state, tenant, course_id, key).map(|_| ())
        }
        MemoryCurriculumAdoptionOutcome::ShiftCourseTerm { course, term } => {
            let course_id = resolve_course(state, tenant, *course)?;
            if state.courses.get(&(tenant, course_id)).map(|row| &row.term) != Some(term) {
                Err(destination::integrity("completed term-shift term"))
            } else {
                Ok(())
            }
        }
    }
}

/// Requires a receipt's completed outcome to name this exact destination
/// assignment through its current locator and course ownership bindings.
/// Inspection and reconciliation share this invariant so receipt-keyed rows
/// cannot be attached to another completed operation.
pub(crate) fn ensure_completed_outcome_contains_assignment(
    state: &State,
    tenant: TenantId,
    key: &CurriculumAdoptionIdempotencyKey,
    receipt: &MemoryCurriculumAdoptionReceipt,
    assignment: AssignmentId,
) -> Result<(), StoreError> {
    ensure_completed_outcome_binding(state, tenant, key, receipt)?;
    if completed_outcome_assignment_ids(state, tenant, &receipt.completed)?.contains(&assignment) {
        Ok(())
    } else {
        Err(destination::integrity("receipt outcome assignment binding"))
    }
}

pub(crate) fn completed_outcome_assignment_ids(
    state: &State,
    tenant: TenantId,
    outcome: &MemoryCurriculumAdoptionOutcome,
) -> Result<Vec<AssignmentId>, StoreError> {
    match outcome {
        MemoryCurriculumAdoptionOutcome::InstantiateBlueprint { course, assignment }
        | MemoryCurriculumAdoptionOutcome::FastForwardAssignment {
            course, assignment, ..
        }
        | MemoryCurriculumAdoptionOutcome::CreateSourceDerivedAssignment { course, assignment } => {
            let assignment = resolve_assignment_reference(state, tenant, *assignment)?;
            ensure_assignment_course(state, tenant, assignment, *course)?;
            Ok(vec![assignment])
        }
        MemoryCurriculumAdoptionOutcome::InstantiateAlpha { course, .. }
        | MemoryCurriculumAdoptionOutcome::RolloverCourse { course, .. } => {
            let course = resolve_course(state, tenant, *course)?;
            Ok(validate_whole_course_adoption(state, tenant, course)?
                .destination_assignments
                .clone())
        }
        MemoryCurriculumAdoptionOutcome::ForkAlpha { .. }
        | MemoryCurriculumAdoptionOutcome::ShiftCourseTerm { .. } => Ok(Vec::new()),
    }
}

fn resolve_assignment_reference(
    state: &State,
    tenant: TenantId,
    reference: question_model::AssignmentReference,
) -> Result<AssignmentId, StoreError> {
    let id = *state
        .assignments_by_reference
        .get(&(tenant, reference))
        .ok_or_else(|| destination::integrity("completed assignment reverse reference"))?;
    if state.assignment_references.get(&(tenant, id)) != Some(&reference) {
        return Err(destination::integrity(
            "completed assignment forward reference",
        ));
    }
    Ok(id)
}

fn ensure_assignment_course(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
    course: question_model::CourseReference,
) -> Result<(), StoreError> {
    let course_id = resolve_course(state, tenant, course)?;
    if state
        .assignments
        .get(&(tenant, assignment))
        .map(|row| row.course_id)
        != Some(course_id)
    {
        return Err(destination::integrity(
            "completed assignment course ownership",
        ));
    }
    Ok(())
}

fn validate_alpha_fork(
    state: &State,
    alpha: question_model::AlphaCourseReference,
    source: &question_model::ObservedAlphaSource,
    key: &CurriculumAdoptionIdempotencyKey,
    receipt: &MemoryCurriculumAdoptionReceipt,
) -> Result<(), StoreError> {
    let id = *state
        .alpha_courses_by_reference
        .get(&alpha)
        .ok_or_else(|| destination::integrity("completed Alpha reverse reference"))?;
    if state.alpha_course_references.get(&id) != Some(&alpha) {
        return Err(destination::integrity("completed Alpha forward reference"));
    }
    reusable_curriculum::require_alpha_creator(state, alpha, receipt.actor)?;
    let lineage: &StoredAlphaForkLineage = state
        .curriculum_adoption
        .alpha_fork_lineage
        .get(&alpha)
        .ok_or_else(|| destination::integrity("Alpha fork lineage"))?;
    if lineage.source != *source
        || lineage.receipt != *key
        || lineage.actor != receipt.actor
        || lineage.occurred_at != receipt.occurred_at
        || !semantic_evidence_is_valid(
            &CurriculumSemanticPayload::course(lineage.payload.clone()),
            lineage.digest,
        )
    {
        return Err(destination::integrity("Alpha fork semantic evidence"));
    }
    Ok(())
}

pub(crate) fn validate_whole_course_adoption(
    state: &State,
    tenant: TenantId,
    course: CourseId,
) -> Result<&StoredWholeCourseAdoption, StoreError> {
    let adoption = state
        .curriculum_adoption
        .whole_course_adoptions
        .get(&(tenant, course))
        .ok_or_else(|| destination::integrity("whole-course adoption"))?;
    let receipt = state
        .curriculum_adoption
        .receipts
        .get(&(tenant, adoption.receipt.clone()))
        .ok_or_else(|| destination::integrity("whole-course receipt"))?;
    let course_reference = *state
        .course_references
        .get(&(tenant, course))
        .ok_or_else(|| destination::integrity("whole-course forward reference"))?;
    if state.courses_by_reference.get(&(tenant, course_reference)) != Some(&course) {
        return Err(destination::integrity("whole-course reverse reference"));
    }
    match (&adoption.origin, &receipt.completed) {
        (
            StoredWholeCourseOrigin::Alpha { source },
            MemoryCurriculumAdoptionOutcome::InstantiateAlpha {
                source: outcome_source,
                course: outcome_course,
            },
        ) if source == outcome_source && *outcome_course == course_reference => {}
        (
            StoredWholeCourseOrigin::Rollover { source_course, .. },
            MemoryCurriculumAdoptionOutcome::RolloverCourse {
                source_course: outcome_source,
                course: outcome_course,
            },
        ) if source_course == outcome_source && *outcome_course == course_reference => {}
        _ => return Err(destination::integrity("whole-course receipt outcome")),
    }
    let expected_operation = match adoption.origin {
        StoredWholeCourseOrigin::Alpha { .. } => CurriculumAdoptionOperation::InstantiateAlpha,
        StoredWholeCourseOrigin::Rollover { .. } => CurriculumAdoptionOperation::RolloverCourse,
    };
    if receipt.operation != expected_operation {
        return Err(destination::integrity("whole-course receipt operation"));
    }
    if adoption.actor != receipt.actor
        || adoption.occurred_at != receipt.occurred_at
        || !semantic_evidence_is_valid(
            &CurriculumSemanticPayload::course(adoption.payload.clone()),
            adoption.digest,
        )
    {
        return Err(destination::integrity("whole-course immutable binding"));
    }
    let semantic = adoption
        .payload
        .modules()
        .iter()
        .enumerate()
        .flat_map(|(module_index, module)| {
            module
                .assignments()
                .iter()
                .enumerate()
                .map(move |(assignment_index, semantic)| (module_index, assignment_index, semantic))
        })
        .collect::<Vec<_>>();
    if semantic.len() != adoption.destination_assignments.len() {
        return Err(destination::integrity("whole-course assignment positions"));
    }
    let unique = adoption
        .destination_assignments
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if unique.len() != adoption.destination_assignments.len() {
        return Err(destination::integrity("whole-course assignment uniqueness"));
    }
    let rollover_source_course = match adoption.origin {
        StoredWholeCourseOrigin::Rollover { source_course, .. } => {
            Some(resolve_course(state, tenant, source_course)?)
        }
        StoredWholeCourseOrigin::Alpha { .. } => None,
    };
    let mut rollover_source_assignments = std::collections::BTreeSet::new();
    for (assignment, (module_index, assignment_index, semantic)) in
        adoption.destination_assignments.iter().zip(semantic)
    {
        if state
            .assignments
            .get(&(tenant, *assignment))
            .map(|row| row.course_id)
            != Some(course)
        {
            return Err(destination::integrity("whole-course assignment ownership"));
        }
        let reference = *state
            .assignment_references
            .get(&(tenant, *assignment))
            .ok_or_else(|| destination::integrity("whole-course assignment forward reference"))?;
        if state.assignments_by_reference.get(&(tenant, reference)) != Some(assignment) {
            return Err(destination::integrity(
                "whole-course assignment reverse reference",
            ));
        }
        let evidence = validate_assignment_evidence(
            state,
            tenant,
            &adoption.receipt,
            adoption.actor,
            adoption.occurred_at,
            *assignment,
        )?;
        if evidence.baseline.payload != *semantic {
            return Err(destination::integrity("whole-course assignment baseline"));
        }
        match (&adoption.origin, &evidence.provenance.source) {
            (
                StoredWholeCourseOrigin::Alpha { source },
                StoredAssignmentImportSource::Reusable(
                    question_model::AssignmentDefinitionSourceView::Alpha(item),
                ),
            ) if item.source() == *source
                && usize::from(item.module_index()) == module_index
                && usize::from(item.assignment_index()) == assignment_index => {}
            (
                StoredWholeCourseOrigin::Rollover {
                    source_course,
                    source_schedule_revision,
                },
                StoredAssignmentImportSource::Rollover(RolloverAssignmentProvenance {
                    source_course: item_course,
                    source_schedule_revision: item_revision,
                    source_assignment,
                }),
            ) if item_course == source_course && item_revision == source_schedule_revision => {
                let source_course_id = rollover_source_course.expect("rollover source course");
                let source_id =
                    resolve_assignment_reference(state, tenant, source_assignment.assignment)?;
                if state
                    .assignments
                    .get(&(tenant, source_id))
                    .map(|row| row.course_id)
                    != Some(source_course_id)
                    || !rollover_source_assignments.insert(source_assignment.assignment)
                {
                    return Err(destination::integrity("rollover source assignment binding"));
                }
            }
            _ => return Err(destination::integrity("whole-course assignment provenance")),
        }
    }
    Ok(adoption)
}

/// Binds an aggregate to the exact completed receipt selected for replay or
/// repair. Inspection starts from the aggregate and therefore intentionally
/// uses `validate_whole_course_adoption` instead.
fn validate_whole_course_receipt_binding(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    key: &CurriculumAdoptionIdempotencyKey,
) -> Result<(), StoreError> {
    let adoption = validate_whole_course_adoption(state, tenant, course)?;
    if &adoption.receipt != key {
        return Err(destination::integrity(
            "whole-course selected receipt binding",
        ));
    }
    Ok(())
}

fn validate_assignment_evidence<'a>(
    state: &'a State,
    tenant: TenantId,
    receipt: &CurriculumAdoptionIdempotencyKey,
    actor: UserId,
    occurred_at: ActivityTimestamp,
    assignment: AssignmentId,
) -> Result<&'a StoredAssignmentAdoptionEvidence, StoreError> {
    let evidence = state
        .curriculum_adoption
        .assignment_evidence
        .get(&(tenant, receipt.clone(), assignment))
        .ok_or_else(|| destination::integrity("immutable assignment evidence"))?;
    if evidence.provenance.receipt != *receipt
        || evidence.provenance.actor != actor
        || evidence.provenance.occurred_at != occurred_at
        || !semantic_evidence_is_valid(
            &CurriculumSemanticPayload::assignment(evidence.baseline.payload.clone()),
            evidence.baseline.digest,
        )
    {
        return Err(destination::integrity("immutable assignment binding"));
    }
    Ok(evidence)
}

fn semantic_evidence_is_valid(
    payload: &CurriculumSemanticPayload,
    observed_digest: CurriculumSemanticDigest,
) -> bool {
    let envelope = payload.canonical_envelope();
    validate_semantic_evidence(
        payload,
        ObservedSemanticEnvelope {
            canonical_version: envelope.version(),
            canonical_bytes: envelope.canonical_bytes(),
            digest: observed_digest.as_bytes(),
        },
    )
    .is_ok()
}

pub(crate) fn validate_current_assignment_import_evidence(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
    import: &StoredAssignmentImport,
) -> Result<(), StoreError> {
    let receipt = state
        .curriculum_adoption
        .receipts
        .get(&(tenant, import.provenance.receipt.clone()))
        .ok_or_else(|| destination::integrity("current import receipt"))?;
    let evidence = validate_assignment_evidence(
        state,
        tenant,
        &import.provenance.receipt,
        receipt.actor,
        receipt.occurred_at,
        assignment,
    )?;
    ensure_completed_outcome_contains_assignment(
        state,
        tenant,
        &import.provenance.receipt,
        receipt,
        assignment,
    )?;
    if evidence.baseline != import.baseline || evidence.provenance != import.provenance {
        return Err(destination::integrity("current import evidence binding"));
    }
    Ok(())
}

/// A completed whole-course receipt proves that a destination must retain its
/// immutable whole-course adoption aggregate. This closes the otherwise
/// ambiguous absence case without introducing another course-origin index.
pub(crate) fn refuse_detached_whole_course_receipt(
    state: &State,
    tenant: TenantId,
    course: question_model::CourseReference,
) -> Result<(), StoreError> {
    let matches = state
        .curriculum_adoption
        .receipts
        .iter()
        .filter(|((entry_tenant, _), receipt)| {
            *entry_tenant == tenant
                && matches!(
                    receipt.completed,
                    MemoryCurriculumAdoptionOutcome::InstantiateAlpha {
                        course: completed_course,
                        ..
                    } | MemoryCurriculumAdoptionOutcome::RolloverCourse {
                        course: completed_course,
                        ..
                    } if completed_course == course
                )
        })
        .count();
    match matches {
        0 => Ok(()),
        1 => Err(destination::integrity("detached whole-course adoption")),
        _ => Err(destination::integrity(
            "duplicate whole-course adoption receipts",
        )),
    }
}

pub(crate) fn store_receipt(
    state: &mut State,
    tenant: TenantId,
    key: CurriculumAdoptionIdempotencyKey,
    operation: CurriculumAdoptionOperation,
    actor: UserId,
    request_sha256: CurriculumAdoptionRequestDigest,
    completed: MemoryCurriculumAdoptionOutcome,
) {
    state.curriculum_adoption.receipts.insert(
        (tenant, key),
        MemoryCurriculumAdoptionReceipt {
            operation,
            actor,
            request_sha256,
            completed,
            occurred_at: state.authoritative_time,
        },
    );
}
