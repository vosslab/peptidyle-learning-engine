//! Exact-pin BlueprintCourse source snapshots built from private Store rows.

use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticCourse,
    CurriculumSemanticPayload,
};
use question_model::{AssignmentDefinitionSourceView, BlueprintModuleId, ObservedBlueprintSource};

use super::{
    StoredBlueprintAssignment, StoredBlueprintCourse, StoredBlueprintCourseRevision,
    StoredBlueprintModule, StoredDefinition, StoredEntry, allocate_blueprint_course_reference,
    assert_fresh_snapshot_handles, fresh_blueprint_course_id, new_assignment_id, new_module_id,
    reconciliation_error,
};
use crate::curriculum_adoption::{
    SemanticAssignmentEntryInputV1, SemanticAssignmentInputV1, SemanticModuleInputV1,
    SemanticPayloadInputV1, SemanticPlannerError, SemanticPoolInputV1, normalize_payload,
};
use crate::in_memory::State;
use crate::{StoreError, UserId};

/// Private exact-pin source snapshot used under the already-held Memory lock.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReusableSourceSnapshot {
    pub(crate) payload: CurriculumSemanticPayload,
}

pub(crate) fn curriculum_source_snapshot(
    state: &State,
    tenant: question_model::TenantId,
    _actor: UserId,
    source: ObservedBlueprintSource,
) -> Result<ReusableSourceSnapshot, StoreError> {
    let observed = source;
    let id = *state
        .blueprint_courses_by_reference
        .get(&(tenant, observed.reference))
        .ok_or(StoreError::NotFound)?;
    let _row = state
        .blueprint_courses
        .get(&(tenant, id))
        .ok_or_else(|| reconciliation_error("BlueprintCourse"))?;
    let revision = state
        .blueprint_course_revisions
        .get(&(tenant, id, observed.revision))
        .ok_or(StoreError::NotFound)?;
    Ok(ReusableSourceSnapshot {
        payload: CurriculumSemanticPayload::course(semantic_course(revision)?),
    })
}

pub(crate) fn curriculum_assignment_source_snapshot(
    state: &State,
    tenant: question_model::TenantId,
    actor: UserId,
    source: AssignmentDefinitionSourceView,
) -> Result<ReusableSourceSnapshot, StoreError> {
    let _ = actor;
    let assignment = assignment_in_snapshot(state, tenant, source)?
        .map(|(_, assignment)| assignment)
        .ok_or(StoreError::NotFound)?;
    Ok(ReusableSourceSnapshot {
        payload: CurriculumSemanticPayload::assignment(assignment),
    })
}

pub(crate) fn current_assignment_source(
    state: &State,
    tenant: question_model::TenantId,
    _actor: UserId,
    source: AssignmentDefinitionSourceView,
) -> Result<AssignmentDefinitionSourceView, StoreError> {
    let observed = source.source();
    let id = *state
        .blueprint_courses_by_reference
        .get(&(tenant, observed.reference))
        .ok_or(StoreError::NotFound)?;
    let row = state
        .blueprint_courses
        .get(&(tenant, id))
        .ok_or_else(|| reconciliation_error("BlueprintCourse"))?;
    let revision = state
        .blueprint_course_revisions
        .get(&(tenant, id, row.head_revision))
        .ok_or_else(|| reconciliation_error("BlueprintCourse head revision"))?;
    if !revision
        .modules
        .iter()
        .flat_map(|module| module.definitions.iter())
        .any(|assignment| assignment.id == source.assignment_id())
    {
        return Err(StoreError::NotFound);
    }
    let current = AssignmentDefinitionSourceView::new(
        ObservedBlueprintSource {
            reference: observed.reference,
            revision: row.head_revision,
        },
        source.assignment_id(),
    );
    curriculum_assignment_source_snapshot(state, tenant, _actor, current)?;
    Ok(current)
}

/// Returns every stable assignment locator in the exact immutable snapshot's authored order.
/// Order drives whole-course materialization; identity remains the assigned opaque handle.
pub(crate) fn course_assignment_sources(
    state: &State,
    tenant: question_model::TenantId,
    source: ObservedBlueprintSource,
) -> Result<Vec<AssignmentDefinitionSourceView>, StoreError> {
    let id = *state
        .blueprint_courses_by_reference
        .get(&(tenant, source.reference))
        .ok_or(StoreError::NotFound)?;
    let revision = state
        .blueprint_course_revisions
        .get(&(tenant, id, source.revision))
        .ok_or(StoreError::NotFound)?;
    Ok(revision
        .modules
        .iter()
        .flat_map(|module| module.definitions.iter())
        .map(|assignment| AssignmentDefinitionSourceView::new(source, assignment.id))
        .collect())
}

/// Rebuilds a stable assignment locator from an exact snapshot position only for
/// answer-free pin-recovery presentation. Positions never authorize updates.
pub(crate) fn course_assignment_source_at_position(
    state: &State,
    tenant: question_model::TenantId,
    source: ObservedBlueprintSource,
    module_index: u16,
    assignment_index: u16,
) -> Result<AssignmentDefinitionSourceView, StoreError> {
    let id = *state
        .blueprint_courses_by_reference
        .get(&(tenant, source.reference))
        .ok_or(StoreError::NotFound)?;
    let revision = state
        .blueprint_course_revisions
        .get(&(tenant, id, source.revision))
        .ok_or(StoreError::NotFound)?;
    let assignment = revision
        .modules
        .get(usize::from(module_index))
        .and_then(|module| module.definitions.get(usize::from(assignment_index)))
        .ok_or(StoreError::NotFound)?;
    Ok(AssignmentDefinitionSourceView::new(source, assignment.id))
}

pub(crate) fn create_blueprint_course_from_semantic_locked(
    state: &mut State,
    tenant: question_model::TenantId,
    actor: UserId,
    semantic: &CurriculumSemanticCourse,
) -> Result<question_model::BlueprintReference, StoreError> {
    let modules = semantic
        .modules()
        .iter()
        .map(|module| {
            Ok(StoredBlueprintModule {
                id: new_module_id(state)?,
                label: module.label().to_owned(),
                definitions: module
                    .assignments()
                    .iter()
                    .map(|assignment| {
                        Ok(StoredBlueprintAssignment {
                            id: new_assignment_id(state)?,
                            definition: stored_definition(assignment)?,
                        })
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let snapshot = StoredBlueprintCourseRevision {
        title: semantic.title().to_owned(),
        modules,
    };
    assert_fresh_snapshot_handles(state, &snapshot)?;
    let id = fresh_blueprint_course_id(state)?;
    let reference = allocate_blueprint_course_reference(state, tenant, id)?;
    if state
        .blueprint_courses
        .insert(
            (tenant, id),
            StoredBlueprintCourse {
                creator: actor,
                head_revision: question_model::BlueprintRevision::INITIAL,
            },
        )
        .is_some()
    {
        return Err(StoreError::Unavailable(
            "BlueprintCourse identity collision".into(),
        ));
    }
    if state
        .blueprint_course_revisions
        .insert(
            (tenant, id, question_model::BlueprintRevision::INITIAL),
            snapshot,
        )
        .is_some()
    {
        return Err(StoreError::Unavailable(
            "BlueprintCourse initial revision collision".into(),
        ));
    }
    Ok(reference)
}

fn stored_definition(
    semantic: &CurriculumSemanticAssignment,
) -> Result<StoredDefinition, StoreError> {
    let entries = semantic
        .entries()
        .iter()
        .map(|entry| match entry {
            CurriculumSemanticAssignmentEntry::Fixed {
                reference,
                points_possible,
                scoring_mode,
            } => Ok(StoredEntry::Fixed {
                pin: *reference,
                points_possible: *points_possible,
                scoring_mode: *scoring_mode,
            }),
            CurriculumSemanticAssignmentEntry::Pool(pool) => Ok(StoredEntry::Pool {
                pins: pool.candidates().to_vec(),
                draw_count: pool.draw_count(),
                points_per_item: pool.points_per_item(),
                ordering: pool.ordering(),
                algorithm: pool.algorithm(),
            }),
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(StoredDefinition {
        title: semantic.title().to_owned(),
        instructions: semantic.instructions().clone(),
        defaults: semantic.defaults().clone(),
        schedule: semantic.schedule().clone(),
        entries,
    })
}

fn semantic_course(
    revision: &StoredBlueprintCourseRevision,
) -> Result<CurriculumSemanticCourse, StoreError> {
    let modules = revision
        .modules
        .iter()
        .map(|module| SemanticModuleInputV1 {
            label: module.label.clone(),
            assignments: module
                .definitions
                .iter()
                .map(|assignment| semantic_assignment_input(&assignment.definition))
                .collect(),
        })
        .collect();
    let CurriculumSemanticPayload::Course(course) =
        normalize_payload(SemanticPayloadInputV1::Course {
            title: revision.title.clone(),
            modules,
        })
        .map_err(semantic_error)?
    else {
        unreachable!("course input normalizes to course meaning")
    };
    Ok(course)
}

fn assignment_in_snapshot(
    state: &State,
    tenant: question_model::TenantId,
    source: AssignmentDefinitionSourceView,
) -> Result<Option<(BlueprintModuleId, CurriculumSemanticAssignment)>, StoreError> {
    let observed = source.source();
    let id = *state
        .blueprint_courses_by_reference
        .get(&(tenant, observed.reference))
        .ok_or(StoreError::NotFound)?;
    let revision = state
        .blueprint_course_revisions
        .get(&(tenant, id, observed.revision))
        .ok_or(StoreError::NotFound)?;
    revision
        .modules
        .iter()
        .find_map(|module| {
            module
                .definitions
                .iter()
                .find(|assignment| assignment.id == source.assignment_id())
                .map(|assignment| {
                    let CurriculumSemanticPayload::Assignment(assignment) =
                        normalize_payload(SemanticPayloadInputV1::Assignment {
                            definition: semantic_assignment_input(&assignment.definition),
                        })
                        .map_err(semantic_error)?
                    else {
                        unreachable!("assignment input normalizes to assignment meaning")
                    };
                    Ok((module.id, assignment))
                })
        })
        .transpose()
}

fn semantic_assignment_input(definition: &StoredDefinition) -> SemanticAssignmentInputV1 {
    let entries = definition
        .entries
        .iter()
        .map(|entry| match entry {
            StoredEntry::Fixed {
                pin,
                points_possible,
                scoring_mode,
            } => SemanticAssignmentEntryInputV1::Fixed {
                reference: *pin,
                points_possible: *points_possible,
                scoring_mode: *scoring_mode,
            },
            StoredEntry::Pool {
                pins,
                draw_count,
                points_per_item,
                ordering,
                algorithm,
            } => SemanticPoolInputV1 {
                candidates: pins.clone(),
                draw_count: *draw_count,
                points_per_item: *points_per_item,
                ordering: *ordering,
                algorithm: *algorithm,
            }
            .into(),
        })
        .collect();
    SemanticAssignmentInputV1 {
        title: definition.title.clone(),
        instructions: definition.instructions.clone(),
        entries,
        defaults: definition.defaults.clone(),
        schedule: definition.schedule.clone(),
    }
}

fn semantic_error(error: SemanticPlannerError) -> StoreError {
    StoreError::InvalidRecord(error.to_string())
}
