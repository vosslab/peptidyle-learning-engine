//! Deterministic Memory implementation of reusable curriculum aggregates.

use async_trait::async_trait;
use question_model::{
    AlphaCourseAccess, AlphaCourseDefinitionInput, AlphaCourseModuleView, AlphaCourseReference,
    AlphaCourseRevision, AlphaCourseSummaryView, AlphaCourseView, AssignmentInstructions,
    AssignmentScoringMode, BlueprintAccess, BlueprintReference, BlueprintRevision,
    BlueprintSummaryView, BlueprintView, CatalogDiscoveryItem, PointValue, PoolDrawAlgorithm,
    ProblemVersionRef, PublicAuthorName, PublicByline, PublicationScope,
    RelativeAssignmentSchedule, ReusableAssignmentDefaults, ReusableAssignmentDefinitionInput,
    ReusableAssignmentDefinitionView, ReusableAssignmentEntryInput, ReusableAssignmentEntryView,
    ReusablePoolCandidateView, ReusablePoolView, ReusableQuestionView,
    ReusableSelectionAvailability, SelectionOrdering,
};
use uuid::Uuid;

use super::{MemoryStore, State, catalog_record_visible};
use crate::{
    Cursor, Page, PageRequest, ReplaceAlphaCourseCommand, ReplaceBlueprintCommand,
    ReusableCurriculumCapability, ReusableCurriculumStore, SessionTokenHash, StoreError,
    TenantContext, UserId,
};

mod source_snapshot;
pub(super) use source_snapshot::{
    ReusableSourceSnapshot, create_alpha_from_semantic_locked, current_assignment_source,
    curriculum_assignment_source_snapshot, curriculum_source_snapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct BlueprintId(Uuid);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct AlphaCourseId(Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoredReusableCurriculumCursor {
    tenant: question_model::TenantId,
    actor: UserId,
    kind: ReusableCurriculumListKind,
    after_key: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReusableCurriculumListKind {
    Blueprints,
    AlphaCourses,
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
pub(super) struct StoredBlueprint {
    owner: UserId,
    revision: BlueprintRevision,
    definition: StoredDefinition,
}
#[derive(Debug, Clone)]
pub(super) struct StoredAlphaCourse {
    creator: UserId,
    creator_byline: PublicByline,
    revision: AlphaCourseRevision,
    title: String,
    modules: Vec<(String, Vec<StoredDefinition>)>,
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

    async fn list_blueprints(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<BlueprintSummaryView>, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let mut rows = state
            .blueprints
            .iter()
            .filter_map(|((tenant, id), row)| {
                (*tenant == context.tenant_id() && row.owner == actor).then_some((*id, row))
            })
            .map(|(id, row)| blueprint_summary(&state, context.tenant_id(), id, row))
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
            ReusableCurriculumListKind::Blueprints,
        )
    }

    async fn get_blueprint(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: BlueprintReference,
    ) -> Result<Option<BlueprintView>, StoreError> {
        let state = self.read_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let Some(id) = state
            .blueprints_by_reference
            .get(&(context.tenant_id(), reference))
        else {
            return Ok(None);
        };
        let Some(row) = state.blueprints.get(&(context.tenant_id(), *id)) else {
            return Err(reconciliation_error("blueprint"));
        };
        (row.owner == actor)
            .then(|| blueprint_view(&state, context.tenant_id(), *id, row))
            .transpose()
    }

    async fn replace_blueprint(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceBlueprintCommand,
    ) -> Result<BlueprintView, StoreError> {
        command.definition.validate().map_err(validation_error)?;
        let mut state = self.write_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let tenant = context.tenant_id();
        let definition =
            resolve_definition(&state, self, tenant, &command.definition.definition, false)?;
        match command.reference {
            None => {
                if command.expected_revision.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "new blueprint cannot carry a revision".into(),
                    ));
                }
                let id = BlueprintId(random_uuid("blueprint")?);
                allocate_blueprint_reference(&mut state, tenant, id)?;
                state.blueprints.insert(
                    (tenant, id),
                    StoredBlueprint {
                        owner: actor,
                        revision: BlueprintRevision::INITIAL,
                        definition,
                    },
                );
                let row = state
                    .blueprints
                    .get(&(tenant, id))
                    .ok_or(StoreError::NotFound)?;
                blueprint_view(&state, tenant, id, row)
            }
            Some(reference) => {
                let expected = command.expected_revision.ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "blueprint replacement requires its observed revision".into(),
                    )
                })?;
                let id = *state
                    .blueprints_by_reference
                    .get(&(tenant, reference))
                    .ok_or(StoreError::NotFound)?;
                let row = state
                    .blueprints
                    .get_mut(&(tenant, id))
                    .ok_or_else(|| reconciliation_error("blueprint"))?;
                if row.owner != actor {
                    return Err(StoreError::Forbidden);
                }
                if row.revision != expected {
                    return Err(StoreError::Conflict);
                }
                if row.definition != definition {
                    row.definition = definition;
                    row.revision = row.revision.checked_next().ok_or_else(|| {
                        StoreError::Unavailable("blueprint revision exhausted".into())
                    })?;
                }
                let row = state
                    .blueprints
                    .get(&(tenant, id))
                    .ok_or(StoreError::NotFound)?;
                blueprint_view(&state, tenant, id, row)
            }
        }
    }

    async fn delete_blueprint(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: BlueprintReference,
        expected_revision: BlueprintRevision,
    ) -> Result<bool, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let tenant = context.tenant_id();
        let Some(id) = state
            .blueprints_by_reference
            .get(&(tenant, reference))
            .copied()
        else {
            return Ok(false);
        };
        let row = state
            .blueprints
            .get(&(tenant, id))
            .ok_or_else(|| reconciliation_error("blueprint"))?;
        if row.owner != actor {
            return Ok(false);
        }
        if row.revision != expected_revision {
            return Err(StoreError::Conflict);
        }
        state.blueprints.remove(&(tenant, id));
        state.blueprints_by_reference.remove(&(tenant, reference));
        state.blueprint_references.remove(&(tenant, id));
        Ok(true)
    }

    async fn list_alpha_courses(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<AlphaCourseSummaryView>, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let mut rows = state
            .alpha_courses
            .iter()
            .map(|(id, row)| alpha_summary(&state, *id, row, actor))
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
            ReusableCurriculumListKind::AlphaCourses,
        )
    }
    async fn get_alpha_course(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: AlphaCourseReference,
    ) -> Result<Option<AlphaCourseView>, StoreError> {
        let state = self.read_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let Some(id) = state.alpha_courses_by_reference.get(&reference) else {
            return Ok(None);
        };
        state
            .alpha_courses
            .get(id)
            .map(|row| alpha_view(&state, context.tenant_id(), *id, row, actor))
            .transpose()
    }
    async fn replace_alpha_course(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceAlphaCourseCommand,
    ) -> Result<AlphaCourseView, StoreError> {
        command.definition.validate().map_err(validation_error)?;
        let mut state = self.write_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let modules = resolve_modules(&state, self, context.tenant_id(), &command.definition)?;
        match command.reference {
            None => {
                if command.expected_revision.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "new Alpha curriculum cannot carry a revision".into(),
                    ));
                }
                let id = AlphaCourseId(random_uuid("Alpha curriculum")?);
                allocate_alpha_reference(&mut state, id)?;
                let creator_byline = creator_byline(&state, actor)?;
                state.alpha_courses.insert(
                    id,
                    StoredAlphaCourse {
                        creator: actor,
                        creator_byline,
                        revision: AlphaCourseRevision::INITIAL,
                        title: command.definition.title,
                        modules,
                    },
                );
                alpha_view(
                    &state,
                    context.tenant_id(),
                    id,
                    state.alpha_courses.get(&id).ok_or(StoreError::NotFound)?,
                    actor,
                )
            }
            Some(reference) => {
                let expected = command.expected_revision.ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "Alpha replacement requires its observed revision".into(),
                    )
                })?;
                let id = *state
                    .alpha_courses_by_reference
                    .get(&reference)
                    .ok_or(StoreError::NotFound)?;
                let row = state
                    .alpha_courses
                    .get_mut(&id)
                    .ok_or_else(|| reconciliation_error("Alpha curriculum"))?;
                if row.creator != actor {
                    return Err(StoreError::Forbidden);
                }
                if row.revision != expected {
                    return Err(StoreError::Conflict);
                }
                if row.title != command.definition.title || row.modules != modules {
                    row.title = command.definition.title;
                    row.modules = modules;
                    row.revision = row.revision.checked_next().ok_or_else(|| {
                        StoreError::Unavailable("Alpha revision exhausted".into())
                    })?;
                }
                alpha_view(
                    &state,
                    context.tenant_id(),
                    id,
                    state.alpha_courses.get(&id).ok_or(StoreError::NotFound)?,
                    actor,
                )
            }
        }
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
    input: &AlphaCourseDefinitionInput,
) -> Result<Vec<(String, Vec<StoredDefinition>)>, StoreError> {
    input
        .modules
        .iter()
        .map(|module| {
            Ok((
                module.label.clone(),
                module
                    .definitions
                    .iter()
                    .map(|definition| resolve_definition(state, store, tenant, definition, true))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        })
        .collect()
}
fn resolve_definition(
    state: &State,
    store: &MemoryStore,
    tenant: question_model::TenantId,
    input: &ReusableAssignmentDefinitionInput,
    require_public: bool,
) -> Result<StoredDefinition, StoreError> {
    let entries = input
        .entries
        .iter()
        .map(|entry| match entry {
            ReusableAssignmentEntryInput::Fixed(source) => {
                Ok::<StoredEntry, StoreError>(StoredEntry::Fixed {
                    pin: resolve_question(
                        state,
                        store,
                        tenant,
                        &source.question_id,
                        require_public,
                    )?,
                    points_possible: source.points_possible,
                    scoring_mode: source.scoring_mode,
                })
            }
            ReusableAssignmentEntryInput::Pool(source) => {
                Ok::<StoredEntry, StoreError>(StoredEntry::Pool {
                    pins: source
                        .candidates
                        .iter()
                        .map(|id| resolve_question(state, store, tenant, id, require_public))
                        .collect::<Result<Vec<_>, StoreError>>()?,
                    draw_count: source.draw_count,
                    points_per_item: source.points_per_item,
                    ordering: source.ordering,
                    algorithm: source.algorithm,
                })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
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
    require_public: bool,
) -> Result<ProblemVersionRef, StoreError> {
    if !store.question_ids.validates(question_id) {
        return Err(StoreError::NotFound);
    }
    let record = state
        .published
        .values()
        .find(|record| &record.question_id == question_id)
        .ok_or(StoreError::NotFound)?;
    (reusable_question_selectable(state, tenant, record)
        && (!require_public || record.scope == PublicationScope::Public))
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
    replacement_candidate_selectable(state, tenant, record)
        .then_some(ProblemVersionRef {
            problem: record.problem,
            version: record.version,
        })
        .ok_or(StoreError::NotFound)
}

/// One policy for advertised and accepted recovery choices: public,
/// tenant-visible, and discoverable for new destination use.
pub(super) fn replacement_candidate_selectable(
    state: &State,
    tenant: question_model::TenantId,
    record: &crate::PublishedProblemRecord,
) -> bool {
    record.scope == PublicationScope::Public && reusable_question_selectable(state, tenant, record)
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
            } => {
                Ok::<ReusableAssignmentEntryView, StoreError>(ReusableAssignmentEntryView::Fixed {
                    question: Box::new(question_view(state, tenant, pin)?),
                    points_possible: *points_possible,
                    scoring_mode: *scoring_mode,
                })
            }
            StoredEntry::Pool {
                pins,
                draw_count,
                points_per_item,
                ordering,
                algorithm,
            } => Ok::<ReusableAssignmentEntryView, StoreError>(ReusableAssignmentEntryView::Pool(
                ReusablePoolView {
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
                        .collect::<Result<Vec<_>, StoreError>>()?,
                    draw_count: *draw_count,
                    points_per_item: *points_per_item,
                    ordering: *ordering,
                    algorithm: *algorithm,
                },
            )),
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

fn reusable_question_selectable(
    state: &State,
    tenant: question_model::TenantId,
    record: &crate::PublishedProblemRecord,
) -> bool {
    catalog_record_visible(state, tenant, record) && record.lifecycle.is_discoverable()
}
fn blueprint_summary(
    state: &State,
    tenant: question_model::TenantId,
    id: BlueprintId,
    row: &StoredBlueprint,
) -> Result<BlueprintSummaryView, StoreError> {
    Ok(BlueprintSummaryView {
        reference: *state
            .blueprint_references
            .get(&(tenant, id))
            .ok_or_else(|| reconciliation_error("blueprint"))?,
        title: row.definition.title.clone(),
        revision: row.revision,
        access: BlueprintAccess::Owner,
    })
}
fn blueprint_view(
    state: &State,
    tenant: question_model::TenantId,
    id: BlueprintId,
    row: &StoredBlueprint,
) -> Result<BlueprintView, StoreError> {
    Ok(BlueprintView {
        reference: *state
            .blueprint_references
            .get(&(tenant, id))
            .ok_or_else(|| reconciliation_error("blueprint"))?,
        revision: row.revision,
        access: BlueprintAccess::Owner,
        definition: definition_view(state, tenant, &row.definition)?,
    })
}
fn alpha_summary(
    state: &State,
    id: AlphaCourseId,
    row: &StoredAlphaCourse,
    actor: UserId,
) -> Result<AlphaCourseSummaryView, StoreError> {
    Ok(AlphaCourseSummaryView {
        reference: *state
            .alpha_course_references
            .get(&id)
            .ok_or_else(|| reconciliation_error("Alpha curriculum"))?,
        title: row.title.clone(),
        revision: row.revision,
        creator_byline: row.creator_byline.clone(),
        access: if row.creator == actor {
            AlphaCourseAccess::Creator
        } else {
            AlphaCourseAccess::ApprovedInstructor
        },
    })
}
fn alpha_view(
    state: &State,
    tenant: question_model::TenantId,
    id: AlphaCourseId,
    row: &StoredAlphaCourse,
    actor: UserId,
) -> Result<AlphaCourseView, StoreError> {
    Ok(AlphaCourseView {
        reference: *state
            .alpha_course_references
            .get(&id)
            .ok_or_else(|| reconciliation_error("Alpha curriculum"))?,
        title: row.title.clone(),
        revision: row.revision,
        creator_byline: row.creator_byline.clone(),
        access: if row.creator == actor {
            AlphaCourseAccess::Creator
        } else {
            AlphaCourseAccess::ApprovedInstructor
        },
        modules: row
            .modules
            .iter()
            .map(|(label, definitions)| {
                Ok::<AlphaCourseModuleView, StoreError>(AlphaCourseModuleView {
                    label: label.clone(),
                    definitions: definitions
                        .iter()
                        .map(|definition| definition_view(state, tenant, definition))
                        .collect::<Result<Vec<_>, StoreError>>()?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?,
    })
}
fn creator_byline(state: &State, actor: UserId) -> Result<PublicByline, StoreError> {
    let account = state.accounts.get(&actor).ok_or(StoreError::NotFound)?;
    PublicByline::new(vec![
        PublicAuthorName::new(account.display_name.clone())
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
    ])
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}
fn page_rows<T: Clone>(
    state: &mut State,
    rows: Vec<(u32, T)>,
    page: PageRequest,
    tenant: question_model::TenantId,
    actor: UserId,
    kind: ReusableCurriculumListKind,
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
            if stored.tenant != tenant || stored.actor != actor || stored.kind != kind {
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
                kind,
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
fn allocate_blueprint_reference(
    state: &mut State,
    tenant: question_model::TenantId,
    id: BlueprintId,
) -> Result<BlueprintReference, StoreError> {
    state.next_blueprint_reference = state
        .next_blueprint_reference
        .checked_add(1)
        .ok_or_else(|| StoreError::Unavailable("blueprint reference exhausted".into()))?;
    let reference = BlueprintReference::new(u64::from(state.next_blueprint_reference))
        .ok_or_else(|| StoreError::Unavailable("blueprint reference exhausted".into()))?;
    state.blueprint_references.insert((tenant, id), reference);
    state
        .blueprints_by_reference
        .insert((tenant, reference), id);
    Ok(reference)
}
fn allocate_alpha_reference(
    state: &mut State,
    id: AlphaCourseId,
) -> Result<AlphaCourseReference, StoreError> {
    state.next_alpha_course_reference = state
        .next_alpha_course_reference
        .checked_add(1)
        .ok_or_else(|| StoreError::Unavailable("Alpha course reference exhausted".into()))?;
    let reference = AlphaCourseReference::new(u64::from(state.next_alpha_course_reference))
        .ok_or_else(|| StoreError::Unavailable("Alpha course reference exhausted".into()))?;
    state.alpha_course_references.insert(id, reference);
    state.alpha_courses_by_reference.insert(reference, id);
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
