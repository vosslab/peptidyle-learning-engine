use std::collections::BTreeSet;

mod authority;

use authority::{CurationPrincipal, can_read_collection, curation_principal, require_instructor};

use super::{MemoryStore, State, catalog_record_visible};
use crate::{
    ActorContext, Cursor, Page, PageRequest, ProblemCollectionMembersPage,
    ProblemCollectionReplacementTarget, ProblemCurationCapability, ProblemCurationStore,
    ReplaceProblemCollectionCommand, ReplaceSavedProblemSearchCommand, SessionTokenHash,
    StoreError, UserId,
};
use async_trait::async_trait;
use question_model::{
    MAX_NAMED_PROBLEM_COLLECTIONS, MAX_PROBLEM_COLLECTION_MEMBERS, MAX_SAVED_PROBLEM_SEARCHES,
    ProblemCollectionAccess, ProblemCollectionKind, ProblemCollectionMemberView,
    ProblemCollectionReference, ProblemCollectionRevision, ProblemCollectionSelectionAvailability,
    ProblemCollectionSummaryView, ProblemCollectionVisibility, ProblemCurationTitleError,
    ProblemVersionRef, QuestionId, SavedProblemSearchReference, SavedProblemSearchRevision,
    SavedProblemSearchView, validate_problem_curation_title,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ProblemCollectionId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SavedProblemSearchId(Uuid);

#[derive(Debug, Clone)]
pub(super) struct StoredProblemCollection {
    owner: UserId,
    kind: ProblemCollectionKind,
    title: String,
    visibility: ProblemCollectionVisibility,
    revision: ProblemCollectionRevision,
    members: Vec<ProblemVersionRef>,
}

#[derive(Debug, Clone)]
pub(super) struct StoredSavedProblemSearch {
    owner: UserId,
    title: String,
    filter: question_model::CatalogSearchFilter,
    revision: SavedProblemSearchRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoredProblemCurationCursor {
    principal: CurationPrincipal,
    scope: String,
    after_key: u32,
}

#[async_trait]
impl ProblemCurationStore for MemoryStore {
    async fn preflight_problem_curation(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        capability: ProblemCurationCapability,
    ) -> Result<(), StoreError> {
        let state = self.read_state()?;
        match capability {
            ProblemCurationCapability::CatalogInstitutionRead => {
                curation_principal(&state, context, session)?;
            }
            ProblemCurationCapability::PersonalMutation => {
                require_instructor(&state, context, session)?;
            }
        }
        Ok(())
    }

    async fn get_or_create_favorites(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
    ) -> Result<ProblemCollectionSummaryView, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_instructor(&state, context, session)?;
        let id = ensure_favorites(&mut state, actor)?;
        let collection = state
            .problem_collections
            .get(&id)
            .ok_or(StoreError::NotFound)?;
        collection_summary(&state, id, collection, CurationPrincipal::Instructor(actor))
    }

    async fn list_problem_collections(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<ProblemCollectionSummaryView>, StoreError> {
        let mut state = self.write_state()?;
        let principal = curation_principal(&state, context, session)?;
        let mut collections = state
            .problem_collections
            .iter()
            .filter(|(_, collection)| can_read_collection(principal, collection))
            .map(|(id, collection)| {
                collection_summary(&state, *id, collection, principal)
                    .map(|summary| (summary.reference.number(), summary))
            })
            .collect::<Result<Vec<_>, _>>()?;
        collections.sort_by_key(|collection| collection.0);
        page_rows(
            &mut state,
            collections,
            page,
            cursor_scope("collections", principal, None, None),
            principal,
        )
    }

    async fn get_problem_collection_summary(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: ProblemCollectionReference,
    ) -> Result<Option<ProblemCollectionSummaryView>, StoreError> {
        let state = self.read_state()?;
        let principal = curation_principal(&state, context, session)?;
        let Some(id) = state.problem_collections_by_reference.get(&reference) else {
            return Ok(None);
        };
        let Some(collection) = state.problem_collections.get(id) else {
            return Err(StoreError::InvalidRecord(
                "problem collection reference is not reconciled".to_string(),
            ));
        };
        can_read_collection(principal, collection)
            .then(|| collection_summary(&state, *id, collection, principal))
            .transpose()
    }

    async fn list_problem_collection_members(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: ProblemCollectionReference,
        page: PageRequest,
    ) -> Result<Option<ProblemCollectionMembersPage>, StoreError> {
        let mut state = self.write_state()?;
        let principal = curation_principal(&state, context, session)?;
        let Some(id) = state.problem_collections_by_reference.get(&reference) else {
            return Ok(None);
        };
        let Some(collection) = state.problem_collections.get(id) else {
            return Err(StoreError::InvalidRecord(
                "problem collection reference is not reconciled".to_string(),
            ));
        };
        if !can_read_collection(principal, collection) {
            return Ok(None);
        }
        let members = collection
            .members
            .iter()
            .enumerate()
            .map(|(position, member)| {
                u32::try_from(position)
                    .map_err(|_| {
                        StoreError::InvalidRecord(
                            "problem collection position overflow".to_string(),
                        )
                    })
                    .and_then(|position| {
                        member_view(&state, member).map(|member| (position + 1, member))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let collection = collection_summary(&state, *id, collection, principal)?;
        let members = page_rows(
            &mut state,
            members,
            page,
            cursor_scope(
                "members",
                principal,
                Some(reference),
                Some(collection.revision),
            ),
            principal,
        )?;
        Ok(Some(ProblemCollectionMembersPage {
            collection,
            members,
        }))
    }

    async fn replace_problem_collection(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        command: ReplaceProblemCollectionCommand,
    ) -> Result<ProblemCollectionSummaryView, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_instructor(&state, context, session)?;
        validate_collection_command(&command)?;
        let members = resolve_members(&state, self, &command.question_ids)?;

        let (id, created) = match command.target {
            ProblemCollectionReplacementTarget::Favorites => {
                let created = favorite_id(&state, actor).is_none();
                (ensure_favorites(&mut state, actor)?, created)
            }
            ProblemCollectionReplacementTarget::NewNamed => (
                ProblemCollectionId(random_uuid("problem collection")?),
                true,
            ),
            ProblemCollectionReplacementTarget::Existing(reference) => {
                let id = *state
                    .problem_collections_by_reference
                    .get(&reference)
                    .ok_or(StoreError::NotFound)?;
                (id, false)
            }
        };

        if created {
            if command.expected_revision.is_some() {
                return Err(StoreError::Conflict);
            }
            if matches!(
                command.target,
                ProblemCollectionReplacementTarget::Favorites
            ) {
                let collection = state
                    .problem_collections
                    .get_mut(&id)
                    .ok_or(StoreError::NotFound)?;
                collection.members = members;
                let collection = collection.clone();
                return collection_summary(
                    &state,
                    id,
                    &collection,
                    CurationPrincipal::Instructor(actor),
                );
            }
            let (kind, title, visibility) = match command.target {
                ProblemCollectionReplacementTarget::Favorites => {
                    unreachable!("Favorites returned from its materialization branch")
                }
                ProblemCollectionReplacementTarget::NewNamed => (
                    ProblemCollectionKind::Named,
                    command.title.clone().expect("validated named title"),
                    command.visibility.expect("validated named visibility"),
                ),
                ProblemCollectionReplacementTarget::Existing(_) => {
                    unreachable!("existing target has a row")
                }
            };
            if kind == ProblemCollectionKind::Named
                && named_collection_count(&state, actor) >= MAX_NAMED_PROBLEM_COLLECTIONS
            {
                return Err(StoreError::InvalidRecord(
                    "named problem collection limit exceeded".to_string(),
                ));
            }
            ensure_unique_named_title(&state, actor, &title, None)?;
            let reference = allocate_collection_reference(&mut state, id)?;
            let collection = StoredProblemCollection {
                owner: actor,
                kind,
                title,
                visibility,
                revision: ProblemCollectionRevision::INITIAL,
                members,
            };
            state.problem_collections.insert(id, collection.clone());
            debug_assert_eq!(
                state.problem_collection_references.get(&id),
                Some(&reference)
            );
            return collection_summary(
                &state,
                id,
                &collection,
                CurationPrincipal::Instructor(actor),
            );
        }

        let existing = state
            .problem_collections
            .get(&id)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if existing.owner != actor {
            return Err(StoreError::Forbidden);
        }
        if command.expected_revision != Some(existing.revision) {
            return Err(StoreError::Conflict);
        }
        let (title, visibility) = match existing.kind {
            ProblemCollectionKind::Favorites => (existing.title.clone(), existing.visibility),
            ProblemCollectionKind::Named => (
                command.title.clone().expect("validated named title"),
                command.visibility.expect("validated named visibility"),
            ),
        };
        ensure_unique_named_title(&state, actor, &title, Some(id))?;
        if existing.title == title
            && existing.visibility == visibility
            && existing.members == members
        {
            return collection_summary(&state, id, &existing, CurationPrincipal::Instructor(actor));
        }
        let replacement = StoredProblemCollection {
            title,
            visibility,
            members,
            revision: existing.revision.checked_next().ok_or_else(|| {
                StoreError::Unavailable("problem collection revision exhausted".to_string())
            })?,
            ..existing
        };
        state.problem_collections.insert(id, replacement.clone());
        collection_summary(
            &state,
            id,
            &replacement,
            CurationPrincipal::Instructor(actor),
        )
    }

    async fn delete_problem_collection(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: ProblemCollectionReference,
        expected_revision: ProblemCollectionRevision,
    ) -> Result<bool, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_instructor(&state, context, session)?;
        let Some(id) = state
            .problem_collections_by_reference
            .get(&reference)
            .copied()
        else {
            return Ok(false);
        };
        let collection = state
            .problem_collections
            .get(&id)
            .ok_or(StoreError::NotFound)?;
        if collection.owner != actor {
            return Ok(false);
        }
        if collection.kind == ProblemCollectionKind::Favorites {
            return Err(StoreError::InvalidRecord(
                "Favorites is retained for its owner".to_string(),
            ));
        }
        if collection.revision != expected_revision {
            return Err(StoreError::Conflict);
        }
        state.problem_collections.remove(&id);
        state.problem_collections_by_reference.remove(&reference);
        state.problem_collection_references.remove(&id);
        Ok(true)
    }

    async fn list_saved_problem_searches(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<SavedProblemSearchView>, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_instructor(&state, context, session)?;
        let mut searches = state
            .saved_problem_searches
            .iter()
            .filter(|(_, search)| search.owner == actor)
            .map(|(id, search)| {
                saved_search_view(&state, *id, search).map(|view| (view.reference.number(), view))
            })
            .collect::<Result<Vec<_>, _>>()?;
        searches.sort_by_key(|search| search.0);
        page_rows(
            &mut state,
            searches,
            page,
            cursor_scope("searches", CurationPrincipal::Instructor(actor), None, None),
            CurationPrincipal::Instructor(actor),
        )
    }

    async fn get_saved_problem_search(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: SavedProblemSearchReference,
    ) -> Result<Option<SavedProblemSearchView>, StoreError> {
        let state = self.read_state()?;
        let actor = require_instructor(&state, context, session)?;
        let Some(id) = state.saved_problem_searches_by_reference.get(&reference) else {
            return Ok(None);
        };
        let Some(search) = state.saved_problem_searches.get(id) else {
            return Err(StoreError::InvalidRecord(
                "saved problem search reference is not reconciled".to_string(),
            ));
        };
        (search.owner == actor)
            .then(|| saved_search_view(&state, *id, search))
            .transpose()
    }

    async fn replace_saved_problem_search(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        command: ReplaceSavedProblemSearchCommand,
    ) -> Result<SavedProblemSearchView, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_instructor(&state, context, session)?;
        validate_problem_curation_title(&command.title).map_err(title_error)?;
        let filter = command
            .filter
            .normalized()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let (id, created) = match command.reference {
            Some(reference) => (
                *state
                    .saved_problem_searches_by_reference
                    .get(&reference)
                    .ok_or(StoreError::NotFound)?,
                false,
            ),
            None => (
                SavedProblemSearchId(random_uuid("saved problem search")?),
                true,
            ),
        };
        if created {
            if command.expected_revision.is_some() {
                return Err(StoreError::Conflict);
            }
            if saved_search_count(&state, actor) >= MAX_SAVED_PROBLEM_SEARCHES {
                return Err(StoreError::InvalidRecord(
                    "saved problem search limit exceeded".to_string(),
                ));
            }
            ensure_unique_saved_search_title(&state, actor, &command.title, None)?;
            allocate_saved_search_reference(&mut state, id)?;
            let search = StoredSavedProblemSearch {
                owner: actor,
                title: command.title,
                filter,
                revision: SavedProblemSearchRevision::INITIAL,
            };
            state.saved_problem_searches.insert(id, search.clone());
            return saved_search_view(&state, id, &search);
        }
        let existing = state
            .saved_problem_searches
            .get(&id)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if existing.owner != actor {
            return Err(StoreError::Forbidden);
        }
        if command.expected_revision != Some(existing.revision) {
            return Err(StoreError::Conflict);
        }
        ensure_unique_saved_search_title(&state, actor, &command.title, Some(id))?;
        if existing.title == command.title && existing.filter == filter {
            return saved_search_view(&state, id, &existing);
        }
        let replacement = StoredSavedProblemSearch {
            title: command.title,
            filter,
            revision: existing.revision.checked_next().ok_or_else(|| {
                StoreError::Unavailable("saved problem search revision exhausted".to_string())
            })?,
            ..existing
        };
        state.saved_problem_searches.insert(id, replacement.clone());
        saved_search_view(&state, id, &replacement)
    }

    async fn delete_saved_problem_search(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: SavedProblemSearchReference,
        expected_revision: SavedProblemSearchRevision,
    ) -> Result<bool, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_instructor(&state, context, session)?;
        let Some(id) = state
            .saved_problem_searches_by_reference
            .get(&reference)
            .copied()
        else {
            return Ok(false);
        };
        let search = state
            .saved_problem_searches
            .get(&id)
            .ok_or(StoreError::NotFound)?;
        if search.owner != actor {
            return Ok(false);
        }
        if search.revision != expected_revision {
            return Err(StoreError::Conflict);
        }
        state.saved_problem_searches.remove(&id);
        state.saved_problem_searches_by_reference.remove(&reference);
        state.saved_problem_search_references.remove(&id);
        Ok(true)
    }
}

fn validate_collection_command(
    command: &ReplaceProblemCollectionCommand,
) -> Result<(), StoreError> {
    if command.question_ids.len() > MAX_PROBLEM_COLLECTION_MEMBERS {
        return Err(StoreError::InvalidRecord(
            "problem collection member limit exceeded".to_string(),
        ));
    }
    let unique = command.question_ids.iter().collect::<BTreeSet<_>>();
    if unique.len() != command.question_ids.len() {
        return Err(StoreError::InvalidRecord(
            "problem collection contains duplicate Question IDs".to_string(),
        ));
    }
    match command.target {
        ProblemCollectionReplacementTarget::Favorites => {
            if command.title.is_some() || command.visibility.is_some() {
                return Err(StoreError::InvalidRecord(
                    "Favorites metadata is fixed".to_string(),
                ));
            }
        }
        ProblemCollectionReplacementTarget::NewNamed
        | ProblemCollectionReplacementTarget::Existing(_) => {
            validate_problem_curation_title(command.title.as_deref().ok_or_else(|| {
                StoreError::InvalidRecord("named collection requires a title".to_string())
            })?)
            .map_err(title_error)?;
            if command.visibility.is_none() {
                return Err(StoreError::InvalidRecord(
                    "named collection requires a visibility".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn title_error(_: ProblemCurationTitleError) -> StoreError {
    StoreError::InvalidRecord("invalid problem curation title".to_string())
}

fn resolve_members(
    state: &State,
    store: &MemoryStore,
    question_ids: &[QuestionId],
) -> Result<Vec<ProblemVersionRef>, StoreError> {
    question_ids
        .iter()
        .map(|question_id| {
            if !store.question_ids.validates(question_id) {
                return Err(StoreError::NotFound);
            }
            let record = state
                .published
                .values()
                .find(|record| &record.question_id == question_id)
                .ok_or(StoreError::NotFound)?;
            (catalog_record_visible(record)
                && record.lifecycle.is_eligible_for_ordinary_new_selection())
            .then_some(ProblemVersionRef {
                problem: record.problem,
                version: record.version,
            })
            .ok_or(StoreError::NotFound)
        })
        .collect()
}

fn favorite_id(state: &State, actor: UserId) -> Option<ProblemCollectionId> {
    state
        .problem_collections
        .iter()
        .find_map(|(id, collection)| {
            (collection.owner == actor && collection.kind == ProblemCollectionKind::Favorites)
                .then_some(*id)
        })
}
fn ensure_favorites(state: &mut State, actor: UserId) -> Result<ProblemCollectionId, StoreError> {
    if let Some(id) = favorite_id(state, actor) {
        return Ok(id);
    }
    let id = ProblemCollectionId(random_uuid("problem collection")?);
    allocate_collection_reference(state, id)?;
    state.problem_collections.insert(
        id,
        StoredProblemCollection {
            owner: actor,
            kind: ProblemCollectionKind::Favorites,
            title: "Favorites".to_string(),
            visibility: ProblemCollectionVisibility::Private,
            revision: ProblemCollectionRevision::INITIAL,
            members: Vec::new(),
        },
    );
    Ok(id)
}
fn named_collection_count(state: &State, actor: UserId) -> usize {
    state
        .problem_collections
        .iter()
        .filter(|(_, collection)| {
            collection.owner == actor && collection.kind == ProblemCollectionKind::Named
        })
        .count()
}
fn random_uuid(label: &str) -> Result<Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("{label} randomness is unavailable: {error}"))
    })
}
fn saved_search_count(state: &State, actor: UserId) -> usize {
    state
        .saved_problem_searches
        .iter()
        .filter(|(_, search)| search.owner == actor)
        .count()
}
fn ensure_unique_saved_search_title(
    state: &State,
    actor: UserId,
    title: &str,
    except: Option<SavedProblemSearchId>,
) -> Result<(), StoreError> {
    let title = title.to_lowercase();
    state
        .saved_problem_searches
        .iter()
        .any(|(id, search)| {
            Some(*id) != except && search.owner == actor && search.title.to_lowercase() == title
        })
        .then_some(())
        .map_or(Ok(()), |_| Err(StoreError::AlreadyExists))
}
fn ensure_unique_named_title(
    state: &State,
    actor: UserId,
    title: &str,
    except: Option<ProblemCollectionId>,
) -> Result<(), StoreError> {
    let title = title.to_lowercase();
    state
        .problem_collections
        .iter()
        .any(|(id, collection)| {
            Some(*id) != except
                && collection.owner == actor
                && collection.kind == ProblemCollectionKind::Named
                && collection.title.to_lowercase() == title
        })
        .then_some(())
        .map_or(Ok(()), |_| Err(StoreError::AlreadyExists))
}
fn allocate_collection_reference(
    state: &mut State,
    id: ProblemCollectionId,
) -> Result<ProblemCollectionReference, StoreError> {
    state.next_problem_collection_reference = state
        .next_problem_collection_reference
        .checked_add(1)
        .ok_or_else(|| {
            StoreError::Unavailable("problem collection reference exhausted".to_string())
        })?;
    let reference =
        ProblemCollectionReference::new(u64::from(state.next_problem_collection_reference))
            .ok_or_else(|| {
                StoreError::Unavailable("problem collection reference exhausted".to_string())
            })?;
    state.problem_collection_references.insert(id, reference);
    state.problem_collections_by_reference.insert(reference, id);
    Ok(reference)
}
fn allocate_saved_search_reference(
    state: &mut State,
    id: SavedProblemSearchId,
) -> Result<SavedProblemSearchReference, StoreError> {
    state.next_saved_problem_search_reference = state
        .next_saved_problem_search_reference
        .checked_add(1)
        .ok_or_else(|| {
            StoreError::Unavailable("saved problem search reference exhausted".to_string())
        })?;
    let reference =
        SavedProblemSearchReference::new(u64::from(state.next_saved_problem_search_reference))
            .ok_or_else(|| {
                StoreError::Unavailable("saved problem search reference exhausted".to_string())
            })?;
    state.saved_problem_search_references.insert(id, reference);
    state
        .saved_problem_searches_by_reference
        .insert(reference, id);
    Ok(reference)
}
fn collection_summary(
    state: &State,
    id: ProblemCollectionId,
    collection: &StoredProblemCollection,
    principal: CurationPrincipal,
) -> Result<ProblemCollectionSummaryView, StoreError> {
    let reference = *state
        .problem_collection_references
        .get(&id)
        .ok_or_else(|| {
            StoreError::InvalidRecord(
                "problem collection is missing its public reference".to_string(),
            )
        })?;
    let access = match principal {
        CurationPrincipal::Instructor(actor) if collection.owner == actor => {
            ProblemCollectionAccess::Owner
        }
        CurationPrincipal::Instructor(_) | CurationPrincipal::Sysadmin(_) => {
            ProblemCollectionAccess::InstitutionReader
        }
    };
    Ok(ProblemCollectionSummaryView {
        reference,
        kind: collection.kind,
        title: collection.title.clone(),
        visibility: collection.visibility,
        revision: collection.revision,
        access,
    })
}
fn member_view(
    state: &State,
    member: &ProblemVersionRef,
) -> Result<ProblemCollectionMemberView, StoreError> {
    let record = state
        .published
        .get(&(member.problem, member.version))
        .ok_or_else(|| {
            StoreError::InvalidRecord(
                "problem collection member is missing immutable publication".to_string(),
            )
        })?;
    Ok(ProblemCollectionMemberView {
        question_id: record.question_id.clone(),
        summary: record.summary(),
        selection_availability: if catalog_record_visible(record)
            && record.lifecycle.is_eligible_for_ordinary_new_selection()
        {
            ProblemCollectionSelectionAvailability::Available
        } else {
            ProblemCollectionSelectionAvailability::Retained
        },
    })
}
fn cursor_scope(
    kind: &str,
    principal: CurationPrincipal,
    collection: Option<ProblemCollectionReference>,
    revision: Option<ProblemCollectionRevision>,
) -> String {
    let actor = match principal {
        CurationPrincipal::Instructor(actor) => actor.as_uuid().to_string(),
        CurationPrincipal::Sysadmin(actor) => actor.as_uuid().to_string(),
    };
    format!(
        "{kind}:{actor}:{}:{}",
        collection.map_or_else(|| "-".to_string(), |reference| reference.to_string()),
        revision.map_or_else(|| "-".to_string(), |revision| revision.to_string()),
    )
}
fn page_rows<T: Clone>(
    state: &mut State,
    rows: Vec<(u32, T)>,
    page: PageRequest,
    scope: String,
    principal: CurationPrincipal,
) -> Result<Page<T>, StoreError> {
    let after_key = match page.after {
        Some(cursor) => {
            let stored = state
                .problem_curation_cursors
                .get(cursor.as_str())
                .cloned()
                .ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "problem curation cursor is malformed or expired".to_string(),
                    )
                })?;
            if stored.principal != principal || stored.scope != scope {
                return Err(StoreError::InvalidRecord(
                    "problem curation cursor is not authorized for this view".to_string(),
                ));
            }
            state.problem_curation_cursors.remove(cursor.as_str());
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
        while state.problem_curation_cursors.len() >= 128 {
            let Some(token) = state.problem_curation_cursors.keys().next().cloned() else {
                break;
            };
            state.problem_curation_cursors.remove(&token);
        }
        let token = random_uuid("problem curation cursor")?.to_string();
        state.problem_curation_cursors.insert(
            token.clone(),
            StoredProblemCurationCursor {
                principal,
                scope,
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
fn saved_search_view(
    state: &State,
    id: SavedProblemSearchId,
    search: &StoredSavedProblemSearch,
) -> Result<SavedProblemSearchView, StoreError> {
    let reference = *state
        .saved_problem_search_references
        .get(&id)
        .ok_or_else(|| {
            StoreError::InvalidRecord(
                "saved problem search is missing its public reference".to_string(),
            )
        })?;
    Ok(SavedProblemSearchView {
        reference,
        title: search.title.clone(),
        filter: search.filter.clone(),
        revision: search.revision,
    })
}
