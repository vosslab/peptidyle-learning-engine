//! Exact-pin B2 source snapshots built from private B1 rows.

use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticCourse,
    CurriculumSemanticModule, CurriculumSemanticPayload, CurriculumSemanticPool,
};
use question_model::{
    AssignmentDefinitionSourceView, CurriculumSourceView, ObservedAlphaAssignmentSource,
    ObservedAlphaSource, ObservedBlueprintSource,
};

use super::*;

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
        .map(|(label, definitions)| {
            CurriculumSemanticModule::new(
                label.clone(),
                definitions
                    .iter()
                    .map(semantic_assignment)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(validation_error)
        })
        .collect::<Result<Vec<_>, _>>()?;
    CurriculumSemanticCourse::new(row.title.clone(), modules).map_err(validation_error)
}

fn semantic_assignment(
    definition: &StoredDefinition,
) -> Result<CurriculumSemanticAssignment, StoreError> {
    let entries = definition
        .entries
        .iter()
        .map(|entry| match entry {
            StoredEntry::Fixed {
                pin,
                points_possible,
                scoring_mode,
            } => Ok(CurriculumSemanticAssignmentEntry::Fixed {
                reference: *pin,
                points_possible: *points_possible,
                scoring_mode: *scoring_mode,
            }),
            StoredEntry::Pool {
                pins,
                draw_count,
                points_per_item,
                ordering,
                algorithm,
            } => CurriculumSemanticPool::new(
                pins.clone(),
                *draw_count,
                *points_per_item,
                *ordering,
                *algorithm,
            )
            .map(CurriculumSemanticAssignmentEntry::Pool)
            .map_err(validation_error),
        })
        .collect::<Result<Vec<_>, _>>()?;
    CurriculumSemanticAssignment::new(
        definition.title.clone(),
        definition.instructions.clone(),
        entries,
        definition.defaults.clone(),
        definition.schedule.clone(),
    )
    .map_err(validation_error)
}
