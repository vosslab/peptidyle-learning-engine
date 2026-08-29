//! Exact-pin BlueprintCourse source snapshots built from private Store rows.

use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticCourse,
    CurriculumSemanticPayload,
};
use question_model::{
    AssignmentDefinitionSourceView, CurriculumSourceView, ObservedBlueprintSource,
};

use super::{
    BlueprintCourseId, StoredBlueprintCourse, StoredDefinition, StoredEntry,
    allocate_blueprint_course_reference, random_uuid, reconciliation_error,
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
    actor: UserId,
    source: CurriculumSourceView,
) -> Result<ReusableSourceSnapshot, StoreError> {
    let observed = source.source();
    let id = *state
        .blueprint_courses_by_reference
        .get(&(tenant, observed.reference))
        .ok_or(StoreError::NotFound)?;
    let row = state
        .blueprint_courses
        .get(&(tenant, id))
        .ok_or_else(|| reconciliation_error("BlueprintCourse"))?;
    if row.creator != actor {
        return Err(StoreError::NotFound);
    }
    if row.revision != observed.revision {
        return Err(StoreError::Conflict);
    }
    Ok(ReusableSourceSnapshot {
        payload: CurriculumSemanticPayload::course(semantic_course(row)?),
    })
}

pub(crate) fn curriculum_assignment_source_snapshot(
    state: &State,
    tenant: question_model::TenantId,
    actor: UserId,
    source: AssignmentDefinitionSourceView,
) -> Result<ReusableSourceSnapshot, StoreError> {
    let observed = source.source();
    let whole =
        curriculum_source_snapshot(state, tenant, actor, CurriculumSourceView::new(observed))?;
    let CurriculumSemanticPayload::Course(course) = whole.payload else {
        unreachable!("BlueprintCourse snapshots are course-sized")
    };
    let assignment = course
        .modules()
        .get(usize::from(source.module_index()))
        .and_then(|module| {
            module
                .assignments()
                .get(usize::from(source.assignment_index()))
        })
        .cloned()
        .ok_or(StoreError::NotFound)?;
    Ok(ReusableSourceSnapshot {
        payload: CurriculumSemanticPayload::assignment(assignment),
    })
}

pub(crate) fn current_assignment_source(
    state: &State,
    tenant: question_model::TenantId,
    actor: UserId,
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
        .filter(|row| row.creator == actor)
        .ok_or(StoreError::NotFound)?;
    let current = AssignmentDefinitionSourceView::new(
        ObservedBlueprintSource {
            reference: observed.reference,
            revision: row.revision,
        },
        source.module_index(),
        source.assignment_index(),
    )
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    curriculum_assignment_source_snapshot(state, tenant, actor, current)?;
    Ok(current)
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
            Ok((
                module.label().to_owned(),
                module
                    .assignments()
                    .iter()
                    .map(stored_definition)
                    .collect::<Result<Vec<_>, StoreError>>()?,
            ))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let id = BlueprintCourseId(random_uuid("BlueprintCourse fork")?);
    let reference = allocate_blueprint_course_reference(state, tenant, id)?;
    state.blueprint_courses.insert(
        (tenant, id),
        StoredBlueprintCourse {
            creator: actor,
            revision: question_model::BlueprintRevision::INITIAL,
            title: semantic.title().to_owned(),
            modules,
        },
    );
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

fn semantic_course(row: &StoredBlueprintCourse) -> Result<CurriculumSemanticCourse, StoreError> {
    let modules = row
        .modules
        .iter()
        .map(|(label, definitions)| SemanticModuleInputV1 {
            label: label.clone(),
            assignments: definitions.iter().map(semantic_assignment_input).collect(),
        })
        .collect();
    let CurriculumSemanticPayload::Course(course) =
        normalize_payload(SemanticPayloadInputV1::Course {
            title: row.title.clone(),
            modules,
        })
        .map_err(semantic_error)?
    else {
        unreachable!("course input normalizes to course meaning")
    };
    Ok(course)
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
