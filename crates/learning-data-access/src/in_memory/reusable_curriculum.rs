//! Deterministic Memory implementation of reusable BlueprintCourse aggregates.

use async_trait::async_trait;
use question_model::{
    AssignmentInstructions, AssignmentScoringMode, BlueprintAssignmentEditHandle,
    BlueprintAssignmentId, BlueprintCourseAccess, BlueprintCourseAssignmentDefinitionView,
    BlueprintCourseModuleView, BlueprintCourseSummaryView, BlueprintCourseView,
    BlueprintModuleEditHandle, BlueprintModuleId, BlueprintReference, BlueprintRevision,
    CatalogDiscoveryItem, CreateBlueprintCourseDefinitionInput, PointValue, PoolDrawAlgorithm,
    ProblemVersionRef, PublicationScope, RelativeAssignmentSchedule,
    ReplaceBlueprintCourseDefinitionInput, ReusableAssignmentDefaults,
    ReusableAssignmentDefinitionInput, ReusableAssignmentDefinitionView,
    ReusableAssignmentEntryInput, ReusableAssignmentEntryView, ReusablePoolCandidateView,
    ReusablePoolView, ReusableQuestionView, ReusableSelectionAvailability, SelectionOrdering,
};
use uuid::Uuid;

use super::{MemoryStore, State, catalog_record_visible};
use crate::{
    ActorContext, CreateBlueprintCourseCommand, Cursor, Page, PageRequest,
    ReplaceBlueprintCourseCommand, ReusableCurriculumCapability, ReusableCurriculumStore,
    SessionTokenHash, StoreError, UserId,
};

mod source_snapshot;
pub(super) use source_snapshot::{
    ReusableSourceSnapshot, course_assignment_source_at_position, course_assignment_sources,
    create_blueprint_course_from_semantic_locked, current_assignment_source,
    curriculum_assignment_source_snapshot, curriculum_source_snapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct BlueprintCourseId(Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoredReusableCurriculumCursor {
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

/// Immutable module node in one complete BlueprintCourse revision snapshot.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct StoredBlueprintModule {
    pub(super) id: BlueprintModuleId,
    pub(super) label: String,
    pub(super) definitions: Vec<StoredBlueprintAssignment>,
}

/// Immutable assignment node in one complete BlueprintCourse revision snapshot.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct StoredBlueprintAssignment {
    pub(super) id: BlueprintAssignmentId,
    pub(super) definition: StoredDefinition,
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
    pub(super) head_revision: BlueprintRevision,
}

/// Append-only complete ordered BlueprintCourse revision snapshot.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct StoredBlueprintCourseRevision {
    pub(super) title: String,
    pub(super) modules: Vec<StoredBlueprintModule>,
}

#[async_trait]
impl ReusableCurriculumStore for MemoryStore {
    async fn preflight_reusable_curriculum(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        _capability: ReusableCurriculumCapability,
    ) -> Result<(), StoreError> {
        let state = self.read_state()?;
        require_approved_instructor(&state, context, session).map(|_| ())
    }

    async fn list_blueprint_courses(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<BlueprintCourseSummaryView>, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let mut rows = state
            .blueprint_courses
            .iter()
            .map(|(id, row)| blueprint_course_summary(&state, *id, row, actor))
            .collect::<Result<Vec<_>, _>>()?;
        rows.sort_by_key(|row| row.reference.number());
        page_rows(
            &mut state,
            rows.into_iter()
                .map(|row| (row.reference.number(), row))
                .collect(),
            page,
            actor,
        )
    }

    async fn get_blueprint_course(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: BlueprintReference,
    ) -> Result<Option<BlueprintCourseView>, StoreError> {
        let state = self.read_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let Some(id) = state.blueprint_courses_by_reference.get(&reference) else {
            return Ok(None);
        };
        let row = state
            .blueprint_courses
            .get(id)
            .ok_or_else(|| reconciliation_error("BlueprintCourse"))?;
        blueprint_course_view(&state, *id, row, actor).map(Some)
    }

    async fn create_blueprint_course(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        command: CreateBlueprintCourseCommand,
    ) -> Result<BlueprintCourseView, StoreError> {
        command.definition.validate().map_err(validation_error)?;
        let mut state = self.write_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let snapshot = resolve_create_snapshot(&state, self, &command.definition)?;
        assert_fresh_snapshot_handles(&state, &snapshot)?;
        let id = fresh_blueprint_course_id(&state)?;
        let _reference = allocate_blueprint_course_reference(&mut state, id)?;
        if state
            .blueprint_courses
            .insert(
                id,
                StoredBlueprintCourse {
                    creator: actor,
                    head_revision: BlueprintRevision::INITIAL,
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
            .insert((id, BlueprintRevision::INITIAL), snapshot)
            .is_some()
        {
            return Err(StoreError::Unavailable(
                "BlueprintCourse initial revision collision".into(),
            ));
        }
        let row = state
            .blueprint_courses
            .get(&id)
            .ok_or(StoreError::NotFound)?;
        blueprint_course_view(&state, id, row, actor)
    }

    async fn replace_blueprint_course(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        command: ReplaceBlueprintCourseCommand,
    ) -> Result<BlueprintCourseView, StoreError> {
        command.definition.validate().map_err(validation_error)?;
        let mut state = self.write_state()?;
        let actor = require_approved_instructor(&state, context, session)?;
        let id = *state
            .blueprint_courses_by_reference
            .get(&command.reference)
            .ok_or(StoreError::NotFound)?;
        let row = state
            .blueprint_courses
            .get(&id)
            .ok_or_else(|| reconciliation_error("BlueprintCourse"))?;
        // ASVS 8.2.1-8.3.1: only the authenticated aggregate owner may advance its head.
        if row.creator != actor {
            return Err(StoreError::Forbidden);
        }
        // ASVS 2.3.1/2.3.3: replacement consumes one observed head and creates one next snapshot.
        if row.head_revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let expected = state
            .blueprint_course_revisions
            .get(&(id, command.expected_revision))
            .ok_or_else(|| reconciliation_error("BlueprintCourse revision"))?;
        let replacement =
            resolve_replacement_snapshot(&state, self, expected, &command.definition)?;
        assert_replacement_handles(&replacement)?;
        if replacement != *expected {
            let next = command.expected_revision.checked_next().ok_or_else(|| {
                StoreError::Unavailable("BlueprintCourse revision exhausted".into())
            })?;
            // ASVS 2.3.3: append the immutable snapshot before moving the only mutable head.
            if state
                .blueprint_course_revisions
                .insert((id, next), replacement)
                .is_some()
            {
                return Err(StoreError::Unavailable(
                    "BlueprintCourse revision collision".into(),
                ));
            }
            state
                .blueprint_courses
                .get_mut(&id)
                .ok_or_else(|| reconciliation_error("BlueprintCourse"))?
                .head_revision = next;
        }
        let row = state
            .blueprint_courses
            .get(&id)
            .ok_or(StoreError::NotFound)?;
        blueprint_course_view(&state, id, row, actor)
    }
}

pub(super) fn require_approved_instructor(
    state: &State,
    context: ActorContext,
    session: SessionTokenHash,
) -> Result<UserId, StoreError> {
    let subject =
        super::sessions::active_subject(state, context, session).ok_or(StoreError::NotFound)?;
    if subject.role() != question_model::UserRole::Instructor {
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

fn resolve_create_snapshot(
    state: &State,
    store: &MemoryStore,
    input: &CreateBlueprintCourseDefinitionInput,
) -> Result<StoredBlueprintCourseRevision, StoreError> {
    let modules = input
        .modules
        .iter()
        .map(|module| {
            Ok::<_, StoreError>(StoredBlueprintModule {
                // ASVS 2.2.1-2.2.3: creation accepts only validated tree meaning;
                // stable identities are allocated after trusted validation.
                id: new_module_id(state)?,
                label: module.label.clone(),
                definitions: module
                    .definitions
                    .iter()
                    .map(|definition| {
                        Ok::<_, StoreError>(StoredBlueprintAssignment {
                            id: new_assignment_id(state)?,
                            definition: resolve_definition(state, store, definition)?,
                        })
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(StoredBlueprintCourseRevision {
        title: input.title.clone(),
        modules,
    })
}

fn resolve_replacement_snapshot(
    state: &State,
    store: &MemoryStore,
    expected: &StoredBlueprintCourseRevision,
    input: &ReplaceBlueprintCourseDefinitionInput,
) -> Result<StoredBlueprintCourseRevision, StoreError> {
    let expected_modules = expected
        .modules
        .iter()
        .map(|module| module.id)
        .collect::<std::collections::BTreeSet<_>>();
    let expected_assignments = expected
        .modules
        .iter()
        .flat_map(|module| module.definitions.iter().map(|definition| definition.id))
        .collect::<std::collections::BTreeSet<_>>();
    let modules = input
        .modules
        .iter()
        .map(|module| {
            let id = retained_or_new_module_id(state, module.handle, &expected_modules)?;
            let definitions = module
                .definitions
                .iter()
                .map(|assignment| {
                    Ok(StoredBlueprintAssignment {
                        id: retained_or_new_assignment_id(
                            state,
                            assignment.handle,
                            &expected_assignments,
                        )?,
                        definition: resolve_definition(state, store, &assignment.definition)?,
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?;
            Ok(StoredBlueprintModule {
                id,
                label: module.label.clone(),
                definitions,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(StoredBlueprintCourseRevision {
        title: input.title.clone(),
        modules,
    })
}

fn retained_or_new_module_id(
    state: &State,
    handle: BlueprintModuleEditHandle,
    expected: &std::collections::BTreeSet<BlueprintModuleId>,
) -> Result<BlueprintModuleId, StoreError> {
    match handle {
        BlueprintModuleEditHandle::Retained { module_id } if expected.contains(&module_id) => {
            Ok(module_id)
        }
        BlueprintModuleEditHandle::Retained { .. } => Err(StoreError::Conflict),
        BlueprintModuleEditHandle::New => new_module_id(state),
    }
}

fn retained_or_new_assignment_id(
    state: &State,
    handle: BlueprintAssignmentEditHandle,
    expected: &std::collections::BTreeSet<BlueprintAssignmentId>,
) -> Result<BlueprintAssignmentId, StoreError> {
    match handle {
        BlueprintAssignmentEditHandle::Retained { assignment_id }
            if expected.contains(&assignment_id) =>
        {
            Ok(assignment_id)
        }
        BlueprintAssignmentEditHandle::Retained { .. } => Err(StoreError::Conflict),
        BlueprintAssignmentEditHandle::New => new_assignment_id(state),
    }
}

pub(super) fn new_module_id(state: &State) -> Result<BlueprintModuleId, StoreError> {
    // ASVS 1.5.2: opaque handles are server allocated, never deserialized as authority.
    let id = BlueprintModuleId::from_uuid(random_uuid("BlueprintCourse module")?);
    (!state
        .blueprint_course_revisions
        .values()
        .flat_map(|revision| revision.modules.iter())
        .any(|module| module.id == id))
    .then_some(id)
    .ok_or_else(|| StoreError::Unavailable("BlueprintCourse module identity collision".into()))
}

pub(super) fn new_assignment_id(state: &State) -> Result<BlueprintAssignmentId, StoreError> {
    // ASVS 1.5.2: opaque handles are server allocated, never deserialized as authority.
    let id = BlueprintAssignmentId::from_uuid(random_uuid("BlueprintCourse assignment")?);
    (!state
        .blueprint_course_revisions
        .values()
        .flat_map(|revision| revision.modules.iter())
        .flat_map(|module| module.definitions.iter())
        .any(|assignment| assignment.id == id))
    .then_some(id)
    .ok_or_else(|| StoreError::Unavailable("BlueprintCourse assignment identity collision".into()))
}

pub(super) fn fresh_blueprint_course_id(state: &State) -> Result<BlueprintCourseId, StoreError> {
    let id = BlueprintCourseId(random_uuid("BlueprintCourse")?);
    (!state
        .blueprint_courses
        .keys()
        .any(|existing| *existing == id))
    .then_some(id)
    .ok_or_else(|| StoreError::Unavailable("BlueprintCourse identity collision".into()))
}

pub(super) fn assert_fresh_snapshot_handles(
    state: &State,
    snapshot: &StoredBlueprintCourseRevision,
) -> Result<(), StoreError> {
    let modules = snapshot
        .modules
        .iter()
        .map(|module| module.id)
        .collect::<std::collections::BTreeSet<_>>();
    let assignments = snapshot
        .modules
        .iter()
        .flat_map(|module| module.definitions.iter().map(|assignment| assignment.id))
        .collect::<std::collections::BTreeSet<_>>();
    (modules.len() == snapshot.modules.len()
        && assignments.len()
            == snapshot
                .modules
                .iter()
                .map(|module| module.definitions.len())
                .sum::<usize>())
    .then_some(())
    .ok_or_else(|| StoreError::Unavailable("BlueprintCourse child identity collision".into()))?;
    let modules_are_fresh = modules.iter().all(|id| {
        state
            .blueprint_course_revisions
            .values()
            .flat_map(|revision| revision.modules.iter())
            .all(|module| module.id != *id)
    });
    let assignments_are_fresh = assignments.iter().all(|id| {
        state
            .blueprint_course_revisions
            .values()
            .flat_map(|revision| revision.modules.iter())
            .flat_map(|module| module.definitions.iter())
            .all(|assignment| assignment.id != *id)
    });
    (modules_are_fresh && assignments_are_fresh)
        .then_some(())
        .ok_or_else(|| StoreError::Unavailable("BlueprintCourse child identity collision".into()))
}

fn assert_replacement_handles(
    replacement: &StoredBlueprintCourseRevision,
) -> Result<(), StoreError> {
    let replacement_modules = replacement
        .modules
        .iter()
        .map(|module| module.id)
        .collect::<std::collections::BTreeSet<_>>();
    let replacement_assignments = replacement
        .modules
        .iter()
        .flat_map(|module| module.definitions.iter().map(|assignment| assignment.id))
        .collect::<std::collections::BTreeSet<_>>();
    (replacement_modules.len() == replacement.modules.len()
        && replacement_assignments.len()
            == replacement
                .modules
                .iter()
                .map(|module| module.definitions.len())
                .sum::<usize>())
    .then_some(())
    .ok_or_else(|| StoreError::Unavailable("BlueprintCourse child identity collision".into()))
}

fn resolve_definition(
    state: &State,
    store: &MemoryStore,
    input: &ReusableAssignmentDefinitionInput,
) -> Result<StoredDefinition, StoreError> {
    let entries = input
        .entries
        .iter()
        .map(|entry| match entry {
            ReusableAssignmentEntryInput::Fixed(source) => Ok(StoredEntry::Fixed {
                pin: resolve_question(state, store, &source.question_id)?,
                points_possible: source.points_possible,
                scoring_mode: source.scoring_mode,
            }),
            ReusableAssignmentEntryInput::Pool(source) => Ok(StoredEntry::Pool {
                pins: source
                    .candidates
                    .iter()
                    .map(|id| resolve_question(state, store, id))
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
    reusable_question_selectable(record)
        .then_some(ProblemVersionRef {
            problem: record.problem,
            version: record.version,
        })
        .ok_or(StoreError::NotFound)
}

pub(super) fn resolve_public_replacement(
    state: &State,
    store: &MemoryStore,
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
    (record.scope == PublicationScope::Public && reusable_question_selectable(record))
        .then_some(ProblemVersionRef {
            problem: record.problem,
            version: record.version,
        })
        .ok_or(StoreError::NotFound)
}

fn reusable_question_selectable(record: &crate::PublishedProblemRecord) -> bool {
    catalog_record_visible(record) && record.lifecycle.is_eligible_for_ordinary_new_selection()
}

fn definition_view(
    state: &State,
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
                question: Box::new(question_view(state, pin)?),
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
                        question_view(state, pin).map(|question| ReusablePoolCandidateView {
                            catalog: question.catalog,
                            selection_availability: question.selection_availability,
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
        selection_availability: if reusable_question_selectable(record) {
            ReusableSelectionAvailability::Available
        } else {
            ReusableSelectionAvailability::Retained
        },
    })
}

fn blueprint_course_summary(
    state: &State,
    id: BlueprintCourseId,
    row: &StoredBlueprintCourse,
    actor: UserId,
) -> Result<BlueprintCourseSummaryView, StoreError> {
    let revision = blueprint_course_head_snapshot(state, id, row)?;
    Ok(BlueprintCourseSummaryView {
        reference: *state
            .blueprint_course_references
            .get(&id)
            .ok_or_else(|| reconciliation_error("BlueprintCourse"))?,
        title: revision.title.clone(),
        revision: row.head_revision,
        access: access(row, actor),
    })
}

fn blueprint_course_view(
    state: &State,
    id: BlueprintCourseId,
    row: &StoredBlueprintCourse,
    actor: UserId,
) -> Result<BlueprintCourseView, StoreError> {
    let revision = blueprint_course_head_snapshot(state, id, row)?;
    Ok(BlueprintCourseView {
        reference: *state
            .blueprint_course_references
            .get(&id)
            .ok_or_else(|| reconciliation_error("BlueprintCourse"))?,
        title: revision.title.clone(),
        revision: row.head_revision,
        access: access(row, actor),
        modules: revision
            .modules
            .iter()
            .map(|module| {
                Ok(BlueprintCourseModuleView {
                    module_id: module.id,
                    label: module.label.clone(),
                    definitions: module
                        .definitions
                        .iter()
                        .map(|assignment| {
                            Ok::<_, StoreError>(BlueprintCourseAssignmentDefinitionView {
                                assignment_id: assignment.id,
                                definition: definition_view(state, &assignment.definition)?,
                            })
                        })
                        .collect::<Result<Vec<_>, StoreError>>()?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?,
    })
}

fn blueprint_course_head_snapshot<'a>(
    state: &'a State,
    id: BlueprintCourseId,
    row: &StoredBlueprintCourse,
) -> Result<&'a StoredBlueprintCourseRevision, StoreError> {
    state
        .blueprint_course_revisions
        .get(&(id, row.head_revision))
        .ok_or_else(|| reconciliation_error("BlueprintCourse head revision"))
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
            if stored.actor != actor {
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
    id: BlueprintCourseId,
) -> Result<BlueprintReference, StoreError> {
    if state.blueprint_course_references.contains_key(&id) {
        return Err(StoreError::Unavailable(
            "BlueprintCourse reference identity collision".into(),
        ));
    }
    state.next_blueprint_course_reference = state
        .next_blueprint_course_reference
        .checked_add(1)
        .ok_or_else(|| StoreError::Unavailable("BlueprintCourse reference exhausted".into()))?;
    let reference = BlueprintReference::new(u64::from(state.next_blueprint_course_reference))
        .ok_or_else(|| StoreError::Unavailable("BlueprintCourse reference exhausted".into()))?;
    if state
        .blueprint_courses_by_reference
        .contains_key(&reference)
    {
        return Err(StoreError::Unavailable(
            "BlueprintCourse reference collision".into(),
        ));
    }
    if state
        .blueprint_course_references
        .insert(id, reference)
        .is_some()
    {
        return Err(StoreError::Unavailable(
            "BlueprintCourse reference identity collision".into(),
        ));
    }
    if state
        .blueprint_courses_by_reference
        .insert(reference, id)
        .is_some()
    {
        return Err(StoreError::Unavailable(
            "BlueprintCourse reference collision".into(),
        ));
    }
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
