//! Deterministic Memory implementation of reusable BlueprintCourse aggregates.

use async_trait::async_trait;
use question_model::{
    AssignmentInstructions, AssignmentScoringMode, BlueprintCourseAccess,
    BlueprintCourseDefinitionInput, BlueprintCourseModuleView, BlueprintCourseSummaryView,
    BlueprintCourseView, BlueprintReference, BlueprintRevision, CatalogDiscoveryItem, PointValue,
    PoolDrawAlgorithm, ProblemVersionRef, PublicationScope, RelativeAssignmentSchedule,
    ReusableAssignmentDefaults, ReusableAssignmentDefinitionInput,
    ReusableAssignmentDefinitionView, ReusableAssignmentEntryInput, ReusableAssignmentEntryView,
    ReusablePoolCandidateView, ReusablePoolView, ReusableQuestionView,
    ReusableSelectionAvailability, SelectionOrdering,
};
use uuid::Uuid;

use super::{MemoryStore, State, catalog_record_visible};
use crate::{
    Cursor, Page, PageRequest, ReplaceBlueprintCourseCommand, ReusableCurriculumCapability,
    ReusableCurriculumStore, SessionTokenHash, StoreError, TenantContext, UserId,
};

mod source_snapshot;
pub(super) use source_snapshot::{
    ReusableSourceSnapshot, create_blueprint_course_from_semantic_locked,
    current_assignment_source, curriculum_assignment_source_snapshot, curriculum_source_snapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct BlueprintCourseId(Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoredReusableCurriculumCursor {
    tenant: question_model::TenantId,
    actor: UserId,
    after_key: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct StoredDefinition {
    title: String,
    instructions: AssignmentInstructions,
    defaults: ReusableAssignmentDefaults,
    schedule: RelativeAssignmentSchedule,
    entries: Vec<StoredEntry>,
}

#[derive(Debug, Clone, PartialEq)]
enum StoredEntry {
    Fixed {
        pin: ProblemVersionRef,
        points_possible: PointValue,
        scoring_mode: AssignmentScoringMode,
    },
    Pool {
        pins: Vec<ProblemVersionRef>,
        draw_count: u32,
        points_per_item: PointValue,
        ordering: SelectionOrdering,
        algorithm: PoolDrawAlgorithm,
    },
}

#[derive(Debug, Clone)]
pub(super) struct StoredBlueprintCourse {
    pub(super) creator: UserId,
    pub(super) revision: BlueprintRevision,
    pub(super) title: String,
    pub(super) modules: Vec<(String, Vec<StoredDefinition>)>,
}

#[async_trait]
impl ReusableCurriculumStore for MemoryStore {
    async fn preflight_reusable_curriculum(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        _capability: ReusableCurriculumCapability,
    ) -> Result<(), StoreError> {
        let state = self.read_state()?;
        require_approved_instructor(&state, context, session).map(|_| ())
    }

    async fn list_blueprint_courses(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<BlueprintCourseSummaryView>, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let mut rows = state
            .blueprint_courses
            .iter()
            .filter(|((tenant, _), _)| *tenant == context.tenant_id())
            .map(|((_, id), row)| {
                blueprint_course_summary(&state, context.tenant_id(), *id, row, actor)
            })
            .collect::<Result<Vec<_>, _>>()?;
        rows.sort_by_key(|row| row.reference.number());
        page_rows(
            &mut state,
            rows.into_iter()
                .map(|row| (row.reference.number(), row))
                .collect(),
            page,
            context.tenant_id(),
            actor,
        )
    }

    async fn get_blueprint_course(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: BlueprintReference,
    ) -> Result<Option<BlueprintCourseView>, StoreError> {
        let state = self.read_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let Some(id) = state
            .blueprint_courses_by_reference
            .get(&(context.tenant_id(), reference))
        else {
            return Ok(None);
        };
        let row = state
            .blueprint_courses
            .get(&(context.tenant_id(), *id))
            .ok_or_else(|| reconciliation_error("BlueprintCourse"))?;
        blueprint_course_view(&state, context.tenant_id(), *id, row, actor).map(Some)
    }

    async fn replace_blueprint_course(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceBlueprintCourseCommand,
    ) -> Result<BlueprintCourseView, StoreError> {
        command.definition.validate().map_err(validation_error)?;
        let mut state = self.write_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let tenant = context.tenant_id();
        let (title, modules) = resolve_modules(&state, self, tenant, &command.definition)?;
        let id = match command.reference {
            None => {
                if command.expected_revision.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "new BlueprintCourse cannot carry a revision".into(),
                    ));
                }
                let id = BlueprintCourseId(random_uuid("BlueprintCourse")?);
                allocate_blueprint_course_reference(&mut state, tenant, id)?;
                state.blueprint_courses.insert(
                    (tenant, id),
                    StoredBlueprintCourse {
                        creator: actor,
                        revision: BlueprintRevision::INITIAL,
                        title,
                        modules,
                    },
                );
                id
            }
            Some(reference) => {
                let expected = command.expected_revision.ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "BlueprintCourse replacement requires its observed revision".into(),
                    )
                })?;
                let id = *state
                    .blueprint_courses_by_reference
                    .get(&(tenant, reference))
                    .ok_or(StoreError::NotFound)?;
                let row = state
                    .blueprint_courses
                    .get_mut(&(tenant, id))
                    .ok_or_else(|| reconciliation_error("BlueprintCourse"))?;
                if row.creator != actor {
                    return Err(StoreError::Forbidden);
                }
                if row.revision != expected {
                    return Err(StoreError::Conflict);
                }
                if row.title != title || row.modules != modules {
                    row.title = title;
                    row.modules = modules;
                    row.revision = row.revision.checked_next().ok_or_else(|| {
                        StoreError::Unavailable("BlueprintCourse revision exhausted".into())
                    })?;
                }
                id
            }
        };
        let row = state
            .blueprint_courses
            .get(&(tenant, id))
            .ok_or(StoreError::NotFound)?;
        blueprint_course_view(&state, tenant, id, row, actor)
    }
}

pub(super) fn require_approved_instructor(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
) -> Result<UserId, StoreError> {
    let subject =
        super::sessions::active_subject(state, context, session).ok_or(StoreError::NotFound)?;
    if !subject
        .roles()
        .contains(&question_model::UserRole::Instructor)
    {
        return Err(StoreError::Forbidden);
    }
    let actor = subject.user();
    let approval = state
        .instructor_approvals
        .get(&actor)
        .ok_or(StoreError::Forbidden)?;
    domain::teaching_authority::validate_instructor_approval(
        &approval.approval,
        state.authoritative_time,
    )
    .map_err(|error| {
        StoreError::InvalidRecord(format!("invalid instructor approval: {error:?}"))
    })?;
    (approval.approval.user == actor && approval.approval.revoked_at.is_none())
        .then_some(actor)
        .ok_or(StoreError::Forbidden)
}

fn resolve_modules(
    state: &State,
    store: &MemoryStore,
    tenant: question_model::TenantId,
    input: &BlueprintCourseDefinitionInput,
) -> Result<(String, Vec<(String, Vec<StoredDefinition>)>), StoreError> {
    let modules = input
        .modules
        .iter()
        .map(|module| {
            Ok((
                module.label.clone(),
                module
                    .definitions
                    .iter()
                    .map(|definition| resolve_definition(state, store, tenant, definition))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok((input.title.clone(), modules))
}

fn resolve_definition(
    state: &State,
    store: &MemoryStore,
    tenant: question_model::TenantId,
    input: &ReusableAssignmentDefinitionInput,
) -> Result<StoredDefinition, StoreError> {
    let entries = input
        .entries
        .iter()
        .map(|entry| match entry {
            ReusableAssignmentEntryInput::Fixed(source) => Ok(StoredEntry::Fixed {
                pin: resolve_question(state, store, tenant, &source.question_id)?,
                points_possible: source.points_possible,
                scoring_mode: source.scoring_mode,
            }),
            ReusableAssignmentEntryInput::Pool(source) => Ok(StoredEntry::Pool {
                pins: source
                    .candidates
                    .iter()
                    .map(|id| resolve_question(state, store, tenant, id))
                    .collect::<Result<Vec<_>, _>>()?,
                draw_count: source.draw_count,
                points_per_item: source.points_per_item,
                ordering: source.ordering,
                algorithm: source.algorithm,
            }),
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(StoredDefinition {
        title: input.title.clone(),
        instructions: input.instructions.clone(),
        defaults: input.defaults.clone(),
        schedule: input.schedule.clone(),
        entries,
    })
}

fn resolve_question(
    state: &State,
    store: &MemoryStore,
    tenant: question_model::TenantId,
    question_id: &question_model::QuestionId,
) -> Result<ProblemVersionRef, StoreError> {
    if !store.question_ids.validates(question_id) {
        return Err(StoreError::NotFound);
    }
    let record = state
        .published
        .values()
        .find(|record| &record.question_id == question_id)
        .ok_or(StoreError::NotFound)?;
    reusable_question_selectable(state, tenant, record)
        .then_some(ProblemVersionRef {
            problem: record.problem,
            version: record.version,
        })
        .ok_or(StoreError::NotFound)
}

pub(super) fn resolve_public_replacement(
    state: &State,
    store: &MemoryStore,
    tenant: question_model::TenantId,
    question: &question_model::QuestionId,
) -> Result<ProblemVersionRef, StoreError> {
    if !store.question_ids.validates(question) {
        return Err(StoreError::NotFound);
    }
    let record = state
        .published
        .values()
        .find(|record| &record.question_id == question)
        .ok_or(StoreError::NotFound)?;
    (record.scope == PublicationScope::Public
        && reusable_question_selectable(state, tenant, record))
    .then_some(ProblemVersionRef {
        problem: record.problem,
        version: record.version,
    })
    .ok_or(StoreError::NotFound)
}

fn reusable_question_selectable(
    state: &State,
    tenant: question_model::TenantId,
    record: &crate::PublishedProblemRecord,
) -> bool {
    catalog_record_visible(state, tenant, record)
        && record.lifecycle.is_eligible_for_ordinary_new_selection()
}

fn definition_view(
    state: &State,
    tenant: question_model::TenantId,
    stored: &StoredDefinition,
) -> Result<ReusableAssignmentDefinitionView, StoreError> {
    let entries = stored
        .entries
        .iter()
        .map(|entry| match entry {
            StoredEntry::Fixed {
                pin,
                points_possible,
                scoring_mode,
            } => Ok(ReusableAssignmentEntryView::Fixed {
                question: Box::new(question_view(state, tenant, pin)?),
                points_possible: *points_possible,
                scoring_mode: *scoring_mode,
            }),
            StoredEntry::Pool {
                pins,
                draw_count,
                points_per_item,
                ordering,
                algorithm,
            } => Ok(ReusableAssignmentEntryView::Pool(ReusablePoolView {
                candidates: pins
                    .iter()
                    .map(|pin| {
                        question_view(state, tenant, pin).map(|question| {
                            ReusablePoolCandidateView {
                                catalog: question.catalog,
                                selection_availability: question.selection_availability,
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                draw_count: *draw_count,
                points_per_item: *points_per_item,
                ordering: *ordering,
                algorithm: *algorithm,
            })),
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(ReusableAssignmentDefinitionView {
        title: stored.title.clone(),
        instructions: stored.instructions.clone(),
        entries,
        defaults: stored.defaults.clone(),
        schedule: stored.schedule.clone(),
    })
}

fn question_view(
    state: &State,
    tenant: question_model::TenantId,
    pin: &ProblemVersionRef,
) -> Result<ReusableQuestionView, StoreError> {
    let record = state
        .published
        .get(&(pin.problem, pin.version))
        .ok_or_else(|| reconciliation_error("reusable question pin"))?;
    let (evidence, _) = super::catalog::catalog_discovery_evidence(
        state,
        (pin.problem, pin.version),
        super::catalog::state_catalog_snapshot_boundary(state),
    );
    Ok(ReusableQuestionView {
        catalog: CatalogDiscoveryItem {
            summary: record.summary(),
            evidence,
        },
        selection_availability: if reusable_question_selectable(state, tenant, record) {
            ReusableSelectionAvailability::Available
        } else {
            ReusableSelectionAvailability::Retained
        },
    })
}

fn blueprint_course_summary(
    state: &State,
    tenant: question_model::TenantId,
    id: BlueprintCourseId,
    row: &StoredBlueprintCourse,
    actor: UserId,
) -> Result<BlueprintCourseSummaryView, StoreError> {
    Ok(BlueprintCourseSummaryView {
        reference: *state
            .blueprint_course_references
            .get(&(tenant, id))
            .ok_or_else(|| reconciliation_error("BlueprintCourse"))?,
        title: row.title.clone(),
        revision: row.revision,
        access: access(row, actor),
    })
}

fn blueprint_course_view(
    state: &State,
    tenant: question_model::TenantId,
    id: BlueprintCourseId,
    row: &StoredBlueprintCourse,
    actor: UserId,
) -> Result<BlueprintCourseView, StoreError> {
    Ok(BlueprintCourseView {
        reference: *state
            .blueprint_course_references
            .get(&(tenant, id))
            .ok_or_else(|| reconciliation_error("BlueprintCourse"))?,
        title: row.title.clone(),
        revision: row.revision,
        access: access(row, actor),
        modules: row
            .modules
            .iter()
            .map(|(label, definitions)| {
                Ok(BlueprintCourseModuleView {
                    label: label.clone(),
                    definitions: definitions
                        .iter()
                        .map(|definition| definition_view(state, tenant, definition))
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?,
    })
}

fn access(row: &StoredBlueprintCourse, actor: UserId) -> BlueprintCourseAccess {
    if row.creator == actor {
        BlueprintCourseAccess::Owner
    } else {
        BlueprintCourseAccess::ApprovedInstructor
    }
}

fn page_rows<T: Clone>(
    state: &mut State,
    rows: Vec<(u32, T)>,
    page: PageRequest,
    tenant: question_model::TenantId,
    actor: UserId,
) -> Result<Page<T>, StoreError> {
    let after_key = match page.after {
        Some(cursor) => {
            let stored = state
                .reusable_curriculum_cursors
                .get(cursor.as_str())
                .cloned()
                .ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "reusable curriculum cursor is malformed or expired".into(),
                    )
                })?;
            if stored.tenant != tenant || stored.actor != actor {
                return Err(StoreError::InvalidRecord(
                    "reusable curriculum cursor is not authorized for this view".into(),
                ));
            }
            state.reusable_curriculum_cursors.remove(cursor.as_str());
            stored.after_key
        }
        None => 0,
    };
    let rows = rows
        .into_iter()
        .filter(|(key, _)| *key > after_key)
        .collect::<Vec<_>>();
    let end = usize::from(page.size.get()).min(rows.len());
    let next_cursor = if end < rows.len() {
        while state.reusable_curriculum_cursors.len() >= 128 {
            let Some(token) = state.reusable_curriculum_cursors.keys().next().cloned() else {
                break;
            };
            state.reusable_curriculum_cursors.remove(&token);
        }
        let token = random_uuid("reusable curriculum cursor")?.to_string();
        state.reusable_curriculum_cursors.insert(
            token.clone(),
            StoredReusableCurriculumCursor {
                tenant,
                actor,
                after_key: rows[end - 1].0,
            },
        );
        Some(Cursor::from_stable_key(token))
    } else {
        None
    };
    Ok(Page {
        items: rows[..end].iter().map(|(_, row)| row.clone()).collect(),
        next_cursor,
    })
}

fn allocate_blueprint_course_reference(
    state: &mut State,
    tenant: question_model::TenantId,
    id: BlueprintCourseId,
) -> Result<BlueprintReference, StoreError> {
    state.next_blueprint_course_reference = state
        .next_blueprint_course_reference
        .checked_add(1)
        .ok_or_else(|| StoreError::Unavailable("BlueprintCourse reference exhausted".into()))?;
    let reference = BlueprintReference::new(u64::from(state.next_blueprint_course_reference))
        .ok_or_else(|| StoreError::Unavailable("BlueprintCourse reference exhausted".into()))?;
    state
        .blueprint_course_references
        .insert((tenant, id), reference);
    state
        .blueprint_courses_by_reference
        .insert((tenant, reference), id);
    Ok(reference)
}

fn random_uuid(label: &str) -> Result<Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("{label} randomness is unavailable: {error}"))
    })
}
fn validation_error(error: question_model::ReusableCurriculumValidationError) -> StoreError {
    StoreError::InvalidRecord(error.to_string())
}
fn reconciliation_error(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("{label} reference is not reconciled"))
}
