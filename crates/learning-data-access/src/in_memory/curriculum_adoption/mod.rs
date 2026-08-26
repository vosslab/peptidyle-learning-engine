//! Deterministic one-lock Memory parity for B2 curriculum adoption.

mod destination;
mod operations;
mod state;
#[cfg(test)]
mod tests;

use state::{CurriculumAdoptionOperation, MemoryCurriculumAdoptionOutcome, StoredCurriculumSource};
pub(super) use state::{
    MemoryCurriculumAdoptionReceipt, StoredAlphaForkLineage, StoredAssignmentAdoptionEvidence,
    StoredCourseAdoptionRecord, StoredCourseImportEnvelope, StoredCurriculumBaseline,
    StoredCurriculumEnvelope,
};

use objects::Sha256Digest;
use question_model::{
    AssignmentDefinitionSourceView, AssignmentId, CourseId, CourseReference,
    CourseScheduleRevision, CourseScheduleWitness, CurriculumAdoptionIdempotencyKey,
    CurriculumPinPosition, CurriculumPinReplacements, CurriculumSourceView,
    ObservedAssignmentRevision, ReplacementQuestionChoices, UnavailablePinRecoveryAction,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use super::{
    ActivityTimestamp, AssignmentId, MemoryStore, SessionTokenHash, State, StoreError,
    TenantContext, TenantId, UserId, catalog_record_visible,
};

const REQUEST_DIGEST_DOMAIN: &[u8] = b"ple:curriculum-adoption-request:v1\0";

pub(super) fn advance_course_schedule_revision(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseScheduleRevision, StoreError> {
    let current = state
        .course_schedule_revisions
        .get_mut(&(tenant, course))
        .ok_or_else(|| destination::integrity("course schedule revision"))?;
    *current = current
        .checked_next()
        .ok_or_else(|| StoreError::Unavailable("course schedule revision exhausted".into()))?;
    Ok(*current)
}

fn authorized_actor(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
) -> Result<UserId, StoreError> {
    super::reusable_curriculum::require_approved_instructor(state, context, session)
}

fn resolve_course(
    state: &State,
    tenant: TenantId,
    reference: CourseReference,
) -> Result<CourseId, StoreError> {
    state
        .courses_by_reference
        .get(&(tenant, reference))
        .copied()
        .ok_or(StoreError::NotFound)
}

fn require_course_instructor(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> Result<(), StoreError> {
    super::teaching_authority::require_direct_instructor(state, tenant, course, actor).map(|_| ())
}

/// Re-reads a complete course witness under the same lock used by the later
/// mutation. Stable internal assignment identity establishes lock/write order
/// (ASVS 15.4.2, 15.4.3).
fn course_witness(
    state: &State,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseScheduleWitness, StoreError> {
    let course_reference = *state
        .course_references
        .get(&(tenant, course))
        .ok_or_else(|| destination::integrity("course reference"))?;
    let schedule_revision = *state
        .course_schedule_revisions
        .get(&(tenant, course))
        .ok_or_else(|| destination::integrity("course schedule revision"))?;
    let mut assignment_ids = state
        .assignments
        .iter()
        .filter_map(|((record_tenant, assignment), row)| {
            (*record_tenant == tenant && row.course_id == course).then_some(*assignment)
        })
        .collect::<Vec<_>>();
    assignment_ids.sort_unstable();
    let assignments = assignment_ids
        .into_iter()
        .map(|assignment| {
            Ok(ObservedAssignmentRevision {
                assignment: *state
                    .assignment_references
                    .get(&(tenant, assignment))
                    .ok_or_else(|| destination::integrity("assignment reference"))?,
                revision: *state
                    .assignment_revisions
                    .get(&(tenant, assignment))
                    .ok_or_else(|| destination::integrity("assignment revision"))?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    CourseScheduleWitness::new(course_reference, schedule_revision, assignments)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}

fn require_exact_witness(
    state: &State,
    tenant: TenantId,
    expected: &CourseScheduleWitness,
) -> Result<CourseId, StoreError> {
    let course = resolve_course(state, tenant, expected.course)?;
    let current = course_witness(state, tenant, course)?;
    if &current != expected {
        return Err(StoreError::Conflict);
    }
    Ok(course)
}

fn course_has_any_run(state: &State, tenant: TenantId, course: CourseId) -> bool {
    state.runs.values().any(|run| {
        run.tenant == tenant
            && state
                .enrollments
                .get(&(tenant, run.enrollment))
                .and_then(|enrollment| state.assignments.get(&(tenant, enrollment.assignment)))
                .is_some_and(|assignment| assignment.course_id == course)
    })
}

fn assignment_has_run(state: &State, tenant: TenantId, assignment: AssignmentId) -> bool {
    state
        .assignments
        .get(&(tenant, assignment))
        .is_some_and(|record| super::course_policy::memory_assignment_has_run(state, record))
}

fn validate_destination_pins(
    state: &State,
    tenant: TenantId,
    payload: &question_model::curriculum_adoption::CurriculumSemanticPayload,
) -> Result<(), CurriculumPinPosition> {
    let assignments = match payload {
        question_model::curriculum_adoption::CurriculumSemanticPayload::Assignment(value) => {
            vec![(None, 0_usize, value)]
        }
        question_model::curriculum_adoption::CurriculumSemanticPayload::Course(course) => course
            .modules()
            .iter()
            .enumerate()
            .flat_map(|(module_index, module)| {
                module.assignments().iter().enumerate().map(
                    move |(assignment_index, assignment)| {
                        (Some(module_index), assignment_index, assignment)
                    },
                )
            })
            .collect(),
    };
    for (module_index, assignment_index, assignment) in assignments {
        for (entry_index, entry) in assignment.entries().iter().enumerate() {
            let unavailable_candidate = match entry {
                question_model::curriculum_adoption::CurriculumSemanticAssignmentEntry::Fixed {
                    reference,
                    ..
                } => (!pin_authorized(state, tenant, *reference)).then_some(None),
                question_model::curriculum_adoption::CurriculumSemanticAssignmentEntry::Pool(
                    pool,
                ) => pool
                    .candidates()
                    .iter()
                    .position(|reference| !pin_authorized(state, tenant, *reference))
                    .map(Some),
            };
            if let Some(candidate_index) = unavailable_candidate {
                return Err(CurriculumPinPosition::new(
                    module_index.and_then(|index| u16::try_from(index).ok()),
                    u16::try_from(assignment_index).unwrap_or(u16::MAX),
                    u16::try_from(entry_index).unwrap_or(u16::MAX),
                    candidate_index.and_then(|index| u16::try_from(index).ok()),
                )
                .expect("validated semantic bounds fit source position"));
            }
        }
    }
    Ok(())
}

fn source_snapshot_with_replacements(
    state: &State,
    store: &MemoryStore,
    tenant: TenantId,
    actor: UserId,
    source: CurriculumSourceView,
    replacements: &CurriculumPinReplacements,
) -> Result<super::reusable_curriculum::ReusableSourceSnapshot, StoreError> {
    let snapshot =
        super::reusable_curriculum::curriculum_source_snapshot(state, tenant, actor, source)?;
    let payload = apply_pin_replacements(state, store, tenant, &snapshot.payload, replacements)?;
    Ok(super::reusable_curriculum::ReusableSourceSnapshot { payload })
}

fn assignment_source_snapshot_with_replacements(
    state: &State,
    store: &MemoryStore,
    tenant: TenantId,
    actor: UserId,
    source: AssignmentDefinitionSourceView,
    replacements: &CurriculumPinReplacements,
) -> Result<super::reusable_curriculum::ReusableSourceSnapshot, StoreError> {
    let snapshot = super::reusable_curriculum::curriculum_assignment_source_snapshot(
        state, tenant, actor, source,
    )?;
    let payload = apply_pin_replacements(state, store, tenant, &snapshot.payload, replacements)?;
    Ok(super::reusable_curriculum::ReusableSourceSnapshot { payload })
}

fn apply_pin_replacements(
    state: &State,
    store: &MemoryStore,
    tenant: TenantId,
    payload: &question_model::curriculum_adoption::CurriculumSemanticPayload,
    replacements: &CurriculumPinReplacements,
) -> Result<question_model::curriculum_adoption::CurriculumSemanticPayload, StoreError> {
    use question_model::curriculum_adoption::{
        CurriculumSemanticCourse, CurriculumSemanticModule, CurriculumSemanticPayload,
    };
    let mut applied = BTreeSet::new();
    let transformed = match payload {
        CurriculumSemanticPayload::Assignment(assignment) => {
            CurriculumSemanticPayload::assignment(replace_assignment_pins(
                state,
                store,
                tenant,
                assignment,
                None,
                0,
                replacements,
                &mut applied,
            )?)
        }
        CurriculumSemanticPayload::Course(course) => {
            let modules = course
                .modules()
                .iter()
                .enumerate()
                .map(|(module_index, module)| {
                    let assignments = module
                        .assignments()
                        .iter()
                        .enumerate()
                        .map(|(assignment_index, assignment)| {
                            replace_assignment_pins(
                                state,
                                store,
                                tenant,
                                assignment,
                                u16::try_from(module_index).ok(),
                                u16::try_from(assignment_index).unwrap_or(u16::MAX),
                                replacements,
                                &mut applied,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    CurriculumSemanticModule::new(module.label().to_owned(), assignments)
                        .map_err(|error| StoreError::InvalidRecord(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            CurriculumSemanticPayload::course(
                CurriculumSemanticCourse::new(course.title().to_owned(), modules)
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
            )
        }
    };
    let expected = replacements
        .as_slice()
        .iter()
        .map(|replacement| replacement.position)
        .collect::<BTreeSet<_>>();
    if applied != expected {
        return Err(StoreError::InvalidRecord(
            "pin replacement does not identify a matching source pin".into(),
        ));
    }
    Ok(transformed)
}

#[allow(clippy::too_many_arguments)]
fn replace_assignment_pins(
    state: &State,
    store: &MemoryStore,
    tenant: TenantId,
    assignment: &question_model::curriculum_adoption::CurriculumSemanticAssignment,
    module_index: Option<u16>,
    assignment_index: u16,
    replacements: &CurriculumPinReplacements,
    applied: &mut BTreeSet<CurriculumPinPosition>,
) -> Result<question_model::curriculum_adoption::CurriculumSemanticAssignment, StoreError> {
    use question_model::curriculum_adoption::{
        CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticPool,
    };
    let entries = assignment
        .entries()
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            let entry_index = u16::try_from(entry_index).unwrap_or(u16::MAX);
            match entry {
                CurriculumSemanticAssignmentEntry::Fixed {
                    reference,
                    points_possible,
                    scoring_mode,
                } => {
                    let position = CurriculumPinPosition::new(
                        module_index,
                        assignment_index,
                        entry_index,
                        None,
                    )
                    .expect("validated semantic bounds fit source position");
                    let reference = replacement_pin(
                        state,
                        store,
                        tenant,
                        position,
                        *reference,
                        replacements,
                        applied,
                    )?;
                    Ok(CurriculumSemanticAssignmentEntry::Fixed {
                        reference,
                        points_possible: *points_possible,
                        scoring_mode: *scoring_mode,
                    })
                }
                CurriculumSemanticAssignmentEntry::Pool(pool) => {
                    let candidates = pool
                        .candidates()
                        .iter()
                        .enumerate()
                        .map(|(candidate_index, reference)| {
                            let position = CurriculumPinPosition::new(
                                module_index,
                                assignment_index,
                                entry_index,
                                u16::try_from(candidate_index).ok(),
                            )
                            .expect("validated pool bounds fit source position");
                            replacement_pin(
                                state,
                                store,
                                tenant,
                                position,
                                *reference,
                                replacements,
                                applied,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    CurriculumSemanticPool::new(
                        candidates,
                        pool.draw_count(),
                        pool.points_per_item(),
                        pool.ordering(),
                        pool.algorithm(),
                    )
                    .map(CurriculumSemanticAssignmentEntry::Pool)
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))
                }
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    CurriculumSemanticAssignment::new(
        assignment.title().to_owned(),
        assignment.instructions().clone(),
        entries,
        assignment.defaults().clone(),
        assignment.schedule().clone(),
    )
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}

fn replacement_pin(
    state: &State,
    store: &MemoryStore,
    tenant: TenantId,
    position: CurriculumPinPosition,
    original: question_model::ProblemVersionRef,
    replacements: &CurriculumPinReplacements,
    applied: &mut BTreeSet<CurriculumPinPosition>,
) -> Result<question_model::ProblemVersionRef, StoreError> {
    let Some(replacement) = replacements
        .as_slice()
        .iter()
        .find(|replacement| replacement.position == position)
    else {
        return Ok(original);
    };
    let pin = super::reusable_curriculum::resolve_public_replacement(
        state,
        store,
        tenant,
        &replacement.question,
    )?;
    applied.insert(position);
    Ok(pin)
}

fn pin_authorized(
    state: &State,
    tenant: TenantId,
    reference: question_model::ProblemVersionRef,
) -> bool {
    state
        .published
        .get(&(reference.problem, reference.version))
        .is_some_and(|record| {
            record.lifecycle.is_assignable() && catalog_record_visible(state, tenant, record)
        })
}

fn pin_correction(
    state: &State,
    tenant: TenantId,
    payload: &question_model::curriculum_adoption::CurriculumSemanticPayload,
) -> Result<Option<UnavailablePinRecoveryAction>, StoreError> {
    let Err(position) = validate_destination_pins(state, tenant, payload) else {
        return Ok(None);
    };
    let candidates = state
        .published
        .values()
        .filter(|record| {
            super::reusable_curriculum::replacement_candidate_selectable(state, tenant, record)
        })
        .map(|record| record.question_id.clone())
        .take(question_model::MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP)
        .collect::<Vec<_>>();
    let candidates = ReplacementQuestionChoices::new(candidates)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    Ok(Some(
        UnavailablePinRecoveryAction::SelectReplacementQuestion {
            position,
            candidates,
        },
    ))
}

#[derive(Serialize)]
struct RequestDigest<'a, T> {
    version: u8,
    operation: &'a str,
    actor: UserId,
    request: T,
}

fn request_digest<T: Serialize>(
    operation: &'static str,
    actor: UserId,
    request: T,
) -> Result<Sha256Digest, StoreError> {
    // Versioned, domain-separated SHA-256 binds the closed command shape and
    // actor before any mutation or source re-resolution (ASVS 11.4.1, 11.4.3).
    let wire = serde_json::to_vec(&RequestDigest {
        version: 1,
        operation,
        actor,
        request,
    })
    .map_err(|error| {
        StoreError::InvalidRecord(format!("request digest encoding failed: {error}"))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(REQUEST_DIGEST_DOMAIN);
    hasher.update(wire);
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}

fn matching_receipt(
    state: &State,
    tenant: TenantId,
    key: &CurriculumAdoptionIdempotencyKey,
    operation: CurriculumAdoptionOperation,
    actor: UserId,
    digest: Sha256Digest,
) -> Result<Option<MemoryCurriculumAdoptionOutcome>, StoreError> {
    let Some(receipt) = state
        .curriculum_adoption_receipts
        .get(&(tenant, key.clone()))
    else {
        return Ok(None);
    };
    if receipt.operation != operation || receipt.actor != actor || receipt.request_sha256 != digest
    {
        return Err(StoreError::Conflict);
    }
    let _authoritative_completion_time = receipt.occurred_at;
    ensure_receipt_evidence(
        state,
        tenant,
        key,
        receipt.actor,
        receipt.occurred_at,
        &receipt.completed,
    )?;
    authorize_receipt_outcome(state, tenant, actor, &receipt.completed)?;
    Ok(Some(receipt.completed.clone()))
}

/// Completed receipts disclose protected object references, so a replay keeps
/// enforcing current object authority at this trusted Store boundary.
fn authorize_receipt_outcome(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    outcome: &MemoryCurriculumAdoptionOutcome,
) -> Result<(), StoreError> {
    let require_destination = |reference| {
        let course = resolve_course(state, tenant, reference)?;
        require_course_instructor(state, tenant, course, actor)
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

fn ensure_receipt_evidence(
    state: &State,
    tenant: TenantId,
    receipt: &CurriculumAdoptionIdempotencyKey,
    actor: UserId,
    occurred_at: ActivityTimestamp,
    outcome: &MemoryCurriculumAdoptionOutcome,
) -> Result<(), StoreError> {
    let assignment = match outcome {
        MemoryCurriculumAdoptionOutcome::InstantiateBlueprint { assignment, .. }
        | MemoryCurriculumAdoptionOutcome::FastForwardAssignment { assignment, .. }
        | MemoryCurriculumAdoptionOutcome::CreateSourceDerivedAssignment { assignment, .. } => {
            Some(*assignment)
        }
        _ => None,
    };
    if let Some(reference) = assignment {
        let id = *state
            .assignments_by_reference
            .get(&(tenant, reference))
            .ok_or_else(|| destination::integrity("completed assignment"))?;
        validate_assignment_evidence(state, tenant, receipt, actor, occurred_at, id)?;
    }
    match outcome {
        MemoryCurriculumAdoptionOutcome::ForkAlpha { source, alpha } => {
            let lineage = state
                .curriculum_alpha_fork_lineage
                .get(alpha)
                .ok_or_else(|| destination::integrity("Alpha fork lineage"))?;
            if lineage.receipt != *receipt
                || lineage.source != *source
                || lineage.actor != actor
                || lineage.occurred_at != occurred_at
                || question_model::curriculum_adoption::CurriculumSemanticPayload::course(
                    lineage.payload.clone(),
                )
                .digest()
                    != lineage.digest
            {
                return Err(destination::integrity("Alpha fork semantic evidence"));
            }
        }
        MemoryCurriculumAdoptionOutcome::InstantiateAlpha { source, course } => {
            let id = *state
                .courses_by_reference
                .get(&(tenant, *course))
                .ok_or_else(|| destination::integrity("completed course"))?;
            let import = state
                .curriculum_course_imports
                .get(&(tenant, id))
                .ok_or_else(|| destination::integrity("course import envelope"))?;
            let envelope = state
                .curriculum_course_envelopes
                .get(&(tenant, id))
                .ok_or_else(|| destination::integrity("course provenance envelope"))?;
            let StoredCurriculumSource::WholeReusable(CurriculumSourceView::Alpha(envelope_source)) =
                envelope.source
            else {
                return Err(destination::integrity("Alpha course source envelope"));
            };
            if import.source != *source
                || import.receipt != *receipt
                || import.actor != actor
                || import.occurred_at != occurred_at
                || envelope_source != *source
                || envelope.receipt != *receipt
                || envelope.actor != actor
                || envelope.occurred_at != occurred_at
            {
                return Err(destination::integrity("Alpha course provenance binding"));
            }
            validate_course_adoption_evidence(
                state,
                tenant,
                id,
                receipt,
                actor,
                occurred_at,
                Some(&import.assignments),
            )?;
        }
        MemoryCurriculumAdoptionOutcome::RolloverCourse {
            source_course,
            course,
        } => {
            let id = *state
                .courses_by_reference
                .get(&(tenant, *course))
                .ok_or_else(|| destination::integrity("completed course"))?;
            let envelope = state
                .curriculum_course_envelopes
                .get(&(tenant, id))
                .ok_or_else(|| destination::integrity("rollover envelope"))?;
            let StoredCurriculumSource::Rollover {
                source_course: envelope_source,
                source_schedule_revision: envelope_schedule_revision,
                ..
            } = envelope.source
            else {
                return Err(destination::integrity("rollover source envelope"));
            };
            if envelope_source != *source_course
                || envelope.receipt != *receipt
                || envelope.actor != actor
                || envelope.occurred_at != occurred_at
            {
                return Err(destination::integrity("rollover provenance binding"));
            }
            validate_course_adoption_evidence(
                state,
                tenant,
                id,
                receipt,
                actor,
                occurred_at,
                None,
            )?;
            let adopted = state
                .curriculum_course_adoptions
                .get(&(tenant, id))
                .ok_or_else(|| destination::integrity("rollover course adoption record"))?;
            for assignment in &adopted.assignments {
                let evidence = state
                    .curriculum_assignment_adoption_evidence
                    .get(&(tenant, receipt.clone(), *assignment))
                    .ok_or_else(|| destination::integrity("rollover assignment evidence"))?;
                let StoredCurriculumSource::RolloverAssignment {
                    source_course: assignment_source_course,
                    source_schedule_revision,
                    ..
                } = evidence.envelope.source
                else {
                    return Err(destination::integrity("rollover assignment source binding"));
                };
                if assignment_source_course != *source_course
                    || source_schedule_revision != envelope_schedule_revision
                {
                    return Err(destination::integrity("rollover assignment source witness"));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_course_adoption_evidence(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    receipt: &CurriculumAdoptionIdempotencyKey,
    actor: UserId,
    occurred_at: ActivityTimestamp,
    expected_assignments: Option<&[AssignmentId]>,
) -> Result<(), StoreError> {
    let record = state
        .curriculum_course_adoptions
        .get(&(tenant, course))
        .ok_or_else(|| destination::integrity("course adoption assignment record"))?;
    if &record.receipt != receipt {
        return Err(destination::integrity("course adoption receipt binding"));
    }
    if expected_assignments.is_some_and(|expected| expected != record.assignments) {
        return Err(destination::integrity("course adoption assignment binding"));
    }
    if question_model::curriculum_adoption::CurriculumSemanticPayload::course(
        record.payload.clone(),
    )
    .digest()
        != record.digest
    {
        return Err(destination::integrity("course adoption semantic evidence"));
    }
    for assignment in &record.assignments {
        let row = state
            .assignments
            .get(&(tenant, *assignment))
            .ok_or_else(|| destination::integrity("adopted assignment"))?;
        if row.course_id != course {
            return Err(destination::integrity("adopted assignment course binding"));
        }
        validate_assignment_evidence(state, tenant, receipt, actor, occurred_at, *assignment)?;
    }
    Ok(())
}

fn validate_assignment_evidence(
    state: &State,
    tenant: TenantId,
    receipt: &CurriculumAdoptionIdempotencyKey,
    actor: UserId,
    occurred_at: ActivityTimestamp,
    assignment: AssignmentId,
) -> Result<(), StoreError> {
    let evidence = state
        .curriculum_assignment_adoption_evidence
        .get(&(tenant, receipt.clone(), assignment))
        .ok_or_else(|| destination::integrity("immutable import baseline or envelope"))?;
    if evidence.envelope.receipt != *receipt
        || evidence.envelope.actor != actor
        || evidence.envelope.occurred_at != occurred_at
        || question_model::curriculum_adoption::CurriculumSemanticPayload::assignment(
            evidence.baseline.payload.clone(),
        )
        .digest()
            != evidence.baseline.digest
    {
        return Err(destination::integrity(
            "immutable assignment adoption evidence",
        ));
    }
    Ok(())
}

pub(super) fn validate_current_assignment_import_evidence(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
    baseline: &StoredCurriculumBaseline,
    envelope: &StoredCurriculumEnvelope,
) -> Result<(), StoreError> {
    let receipt = state
        .curriculum_adoption_receipts
        .get(&(tenant, envelope.receipt.clone()))
        .ok_or_else(|| destination::integrity("import receipt"))?;
    validate_assignment_evidence(
        state,
        tenant,
        &envelope.receipt,
        receipt.actor,
        receipt.occurred_at,
        assignment,
    )?;
    let evidence = state
        .curriculum_assignment_adoption_evidence
        .get(&(tenant, envelope.receipt.clone(), assignment))
        .ok_or_else(|| destination::integrity("immutable import baseline or envelope"))?;
    if evidence.baseline != *baseline || evidence.envelope != *envelope {
        return Err(destination::integrity(
            "current import baseline or provenance binding",
        ));
    }
    Ok(())
}

fn store_receipt(
    state: &mut State,
    tenant: TenantId,
    key: CurriculumAdoptionIdempotencyKey,
    operation: CurriculumAdoptionOperation,
    actor: UserId,
    request_sha256: Sha256Digest,
    completed: MemoryCurriculumAdoptionOutcome,
) {
    state.curriculum_adoption_receipts.insert(
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
