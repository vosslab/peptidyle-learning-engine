//! Exact-pin B2 source snapshots built from private B1 rows.

use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticCourse,
    CurriculumSemanticPayload,
};
use question_model::{
    AlphaCourseRevision, AssignmentDefinitionSourceView, CurriculumSourceView,
    ObservedAlphaAssignmentSource, ObservedAlphaSource, ObservedBlueprintSource,
};

use super::{
    AlphaCourseId, StoredAlphaCourse, StoredDefinition, StoredEntry, allocate_alpha_reference,
    creator_byline, random_uuid, reconciliation_error,
};
use crate::curriculum_adoption::{
    SemanticAssignmentEntryInputV1, SemanticAssignmentInputV1, SemanticModuleInputV1,
    SemanticPayloadInputV1, SemanticPlannerError, SemanticPoolInputV1, normalize_payload,
};
use crate::in_memory::State;
use crate::{StoreError, UserId};

/// Private exact-pin source snapshot used by B2 under the already-held Memory lock.
///
/// Source locators are re-resolved in the trusted tier and never grant tenant or
/// object authority (ASVS 8.2.2, 8.3.1, 8.4.1).
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
    let payload = match source {
        CurriculumSourceView::Blueprint(observed) => {
            let id = *state
                .blueprints_by_reference
                .get(&(tenant, observed.reference))
                .ok_or(StoreError::NotFound)?;
            let row = state
                .blueprints
                .get(&(tenant, id))
                .ok_or_else(|| reconciliation_error("blueprint"))?;
            if row.owner != actor {
                return Err(StoreError::NotFound);
            }
            if row.revision != observed.revision {
                return Err(StoreError::Conflict);
            }
            CurriculumSemanticPayload::assignment(semantic_assignment(&row.definition)?)
        }
        CurriculumSourceView::Alpha(observed) => {
            let id = *state
                .alpha_courses_by_reference
                .get(&observed.reference)
                .ok_or(StoreError::NotFound)?;
            let row = state
                .alpha_courses
                .get(&id)
                .ok_or_else(|| reconciliation_error("Alpha curriculum"))?;
            if row.revision != observed.revision {
                return Err(StoreError::Conflict);
            }
            CurriculumSemanticPayload::course(semantic_course(row)?)
        }
    };
    Ok(ReusableSourceSnapshot { payload })
}

pub(crate) fn curriculum_assignment_source_snapshot(
    state: &State,
    tenant: question_model::TenantId,
    actor: UserId,
    source: AssignmentDefinitionSourceView,
) -> Result<ReusableSourceSnapshot, StoreError> {
    let payload = match source {
        AssignmentDefinitionSourceView::Blueprint(observed) => {
            curriculum_source_snapshot(
                state,
                tenant,
                actor,
                CurriculumSourceView::Blueprint(observed),
            )?
            .payload
        }
        AssignmentDefinitionSourceView::Alpha(observed) => {
            let whole = curriculum_source_snapshot(
                state,
                tenant,
                actor,
                CurriculumSourceView::Alpha(observed.source()),
            )?;
            let CurriculumSemanticPayload::Course(course) = whole.payload else {
                unreachable!("Alpha source snapshots are course-sized")
            };
            let assignment = course
                .modules()
                .get(usize::from(observed.module_index()))
                .and_then(|module| {
                    module
                        .assignments()
                        .get(usize::from(observed.assignment_index()))
                })
                .cloned()
                .ok_or(StoreError::NotFound)?;
            CurriculumSemanticPayload::assignment(assignment)
        }
    };
    Ok(ReusableSourceSnapshot { payload })
}

pub(crate) fn current_curriculum_source(
    state: &State,
    tenant: question_model::TenantId,
    actor: UserId,
    source: CurriculumSourceView,
) -> Result<CurriculumSourceView, StoreError> {
    match source {
        CurriculumSourceView::Blueprint(observed) => {
            let id = *state
                .blueprints_by_reference
                .get(&(tenant, observed.reference))
                .ok_or(StoreError::NotFound)?;
            let row = state
                .blueprints
                .get(&(tenant, id))
                .filter(|row| row.owner == actor)
                .ok_or(StoreError::NotFound)?;
            Ok(CurriculumSourceView::Blueprint(ObservedBlueprintSource {
                reference: observed.reference,
                revision: row.revision,
            }))
        }
        CurriculumSourceView::Alpha(observed) => {
            let id = *state
                .alpha_courses_by_reference
                .get(&observed.reference)
                .ok_or(StoreError::NotFound)?;
            let row = state
                .alpha_courses
                .get(&id)
                .ok_or_else(|| reconciliation_error("Alpha curriculum"))?;
            Ok(CurriculumSourceView::Alpha(ObservedAlphaSource {
                reference: observed.reference,
                revision: row.revision,
            }))
        }
    }
}

pub(crate) fn current_assignment_source(
    state: &State,
    tenant: question_model::TenantId,
    actor: UserId,
    source: AssignmentDefinitionSourceView,
) -> Result<AssignmentDefinitionSourceView, StoreError> {
    let current = match source {
        AssignmentDefinitionSourceView::Blueprint(observed) => {
            let CurriculumSourceView::Blueprint(current) = current_curriculum_source(
                state,
                tenant,
                actor,
                CurriculumSourceView::Blueprint(observed),
            )?
            else {
                unreachable!()
            };
            AssignmentDefinitionSourceView::Blueprint(current)
        }
        AssignmentDefinitionSourceView::Alpha(observed) => {
            let CurriculumSourceView::Alpha(current) = current_curriculum_source(
                state,
                tenant,
                actor,
                CurriculumSourceView::Alpha(observed.source()),
            )?
            else {
                unreachable!()
            };
            AssignmentDefinitionSourceView::Alpha(
                ObservedAlphaAssignmentSource::new(
                    current,
                    observed.module_index(),
                    observed.assignment_index(),
                )
                .expect("stored bounded Alpha assignment position remains valid"),
            )
        }
    };
    curriculum_assignment_source_snapshot(state, tenant, actor, current)?;
    Ok(current)
}

pub(crate) fn create_alpha_from_semantic_locked(
    state: &mut State,
    actor: UserId,
    semantic: &CurriculumSemanticCourse,
) -> Result<question_model::AlphaCourseReference, StoreError> {
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
    let id = AlphaCourseId(random_uuid("Alpha curriculum fork")?);
    let reference = allocate_alpha_reference(state, id)?;
    let byline = creator_byline(state, actor)?;
    state.alpha_courses.insert(
        id,
        StoredAlphaCourse {
            creator: actor,
            creator_byline: byline,
            revision: AlphaCourseRevision::INITIAL,
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

fn semantic_course(row: &StoredAlphaCourse) -> Result<CurriculumSemanticCourse, StoreError> {
    let modules = row
        .modules
        .iter()
        .map(|(label, definitions)| SemanticModuleInputV1 {
            label: label.clone(),
            assignments: definitions.iter().map(semantic_assignment_input).collect(),
        })
        .collect();
    let payload = normalize_payload(SemanticPayloadInputV1::Course {
        title: row.title.clone(),
        modules,
    })
    .map_err(semantic_error)?;
    let CurriculumSemanticPayload::Course(course) = payload else {
        unreachable!("course input normalizes to course meaning")
    };
    Ok(course)
}

fn semantic_assignment(
    definition: &StoredDefinition,
) -> Result<CurriculumSemanticAssignment, StoreError> {
    let payload = normalize_payload(SemanticPayloadInputV1::Assignment {
        definition: semantic_assignment_input(definition),
    })
    .map_err(semantic_error)?;
    let CurriculumSemanticPayload::Assignment(assignment) = payload else {
        unreachable!("assignment input normalizes to assignment meaning")
    };
    Ok(assignment)
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
