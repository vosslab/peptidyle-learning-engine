//! In-memory flat-question store implementations.

use async_trait::async_trait;
use question_model::DraftQuestionSource::Native;
use question_model::{UserId, WorkspaceId};

use super::{MemoryFlatQuestionGraderStore, MemoryStore};
use crate::{
    ActorContext, FlatQuestionGradingPayload, FlatQuestionGradingStore, FlatQuestionStore,
    StoreError, UpsertFlatQuestionCommand, WorkspaceDraftRevision, WorkspaceDraftRole,
    WorkspaceFlatQuestionSource,
};

#[async_trait]
impl FlatQuestionStore for MemoryStore {
    async fn upsert_flat_question(
        &self,
        actor: ActorContext,
        command: UpsertFlatQuestionCommand,
    ) -> Result<WorkspaceFlatQuestionSource, StoreError> {
        crate::flat_question::validate_upsert_flat_question_command(&command)?;
        let actor = actor.user_id();
        let key = command.draft.question.workspace;
        let mut state = self.write_state()?;
        let source_family = match &command.draft.question.source {
            Native { family } => family.clone(),
            _ => {
                return Err(StoreError::InvalidRecord(
                    "flat-question sources require native draft source".to_string(),
                ));
            }
        };
        let is_new = !state.drafts.contains_key(&key);
        let revision = if !is_new {
            let role = state
                .draft_access
                .get(&(command.draft.question.workspace, actor));
            if !matches!(
                role,
                Some(WorkspaceDraftRole::Owner | WorkspaceDraftRole::Collaborator)
            ) {
                return Err(StoreError::Forbidden);
            }
            let current = state
                .draft_revisions
                .get(&key)
                .copied()
                .ok_or(StoreError::Forbidden)?;
            if command.expected_revision != Some(current) {
                return Err(StoreError::Conflict);
            }
            current.next()?
        } else {
            if command.expected_revision.is_some() {
                return Err(StoreError::Conflict);
            }
            if state.workspace_references.contains_key(&key)
                && state.draft_access.get(&(key, actor)) != Some(&WorkspaceDraftRole::Owner)
            {
                return Err(StoreError::Forbidden);
            }
            WorkspaceDraftRevision::INITIAL
        };
        let source = WorkspaceFlatQuestionSource::new(
            command.draft.question.workspace,
            revision,
            source_family,
            command.source,
            command.canonical_source_sha256,
            command.public_binding_sha256,
        )?;
        // Draft, source, and grading maps change only after every authorization,
        // revision, and binding check succeeds while this write lock is held.
        state.drafts.insert(key, command.draft.clone());
        state.draft_revisions.insert(key, revision);
        if is_new {
            super::navigation_references::ensure_workspace_reference(&mut state, key)?;
            state
                .draft_access
                .entry((key, actor))
                .or_insert(WorkspaceDraftRole::Owner);
        }
        state.flat_question_sources.insert(key, source.clone());
        state
            .workspace_flat_question_grading
            .insert(key, command.grading);
        Ok(source)
    }

    async fn flat_question_source(
        &self,
        actor: ActorContext,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceFlatQuestionSource>, StoreError> {
        let actor = actor.user_id();
        let state = self.read_state()?;
        let key = workspace;
        if !state.draft_access.contains_key(&(workspace, actor)) {
            return Ok(None);
        }
        Ok(state.flat_question_sources.get(&key).cloned())
    }
}

#[async_trait]
impl FlatQuestionGradingStore for MemoryFlatQuestionGraderStore {
    async fn flat_question_published_grading(
        &self,
        reference: question_model::ProblemVersionRef,
    ) -> Result<Option<FlatQuestionGradingPayload>, StoreError> {
        let state = self
            .state
            .read()
            .map_err(|error| StoreError::Unavailable(error.to_string()))?;
        let Some(published) = state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|record| record.scope == question_model::PublicationScope::Public)
        else {
            return Ok(None);
        };
        Ok(state
            .published_flat_question_grading
            .get(&(published.problem, published.version))
            .cloned())
    }
}
