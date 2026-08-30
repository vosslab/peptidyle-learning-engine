//! In-memory private workspace-image registry for native flat questions.

use async_trait::async_trait;
use question_model::{AssetId, WorkspaceId};

use super::MemoryStore;
use crate::{
    ActorContext, FlatQuestionAssetStore, StoreError, WorkspaceDraftRole,
    WorkspaceFlatQuestionAsset,
};

#[async_trait]
impl FlatQuestionAssetStore for MemoryStore {
    async fn register_workspace_flat_question_asset(
        &self,
        context: ActorContext,
        descriptor: WorkspaceFlatQuestionAsset,
    ) -> Result<WorkspaceFlatQuestionAsset, StoreError> {
        descriptor.validate()?;
        let key = (descriptor.workspace, descriptor.asset);
        let mut state = self.write_state()?;
        require_workspace_asset_author(&state, context, descriptor.workspace)?;
        match state.workspace_flat_question_assets.get(&key) {
            Some(existing) if existing == &descriptor => Ok(existing.clone()),
            Some(_) => Err(StoreError::Conflict),
            None => {
                state
                    .workspace_flat_question_assets
                    .insert(key, descriptor.clone());
                Ok(descriptor)
            }
        }
    }

    async fn list_workspace_flat_question_assets(
        &self,
        context: ActorContext,
        workspace: WorkspaceId,
    ) -> Result<Vec<WorkspaceFlatQuestionAsset>, StoreError> {
        let state = self.read_state()?;
        require_workspace_asset_reader(&state, context, workspace)?;
        Ok(state
            .workspace_flat_question_assets
            .iter()
            .filter(|((candidate_workspace, _), _)| *candidate_workspace == workspace)
            .map(|(_, descriptor)| descriptor.clone())
            .collect())
    }

    async fn resolve_workspace_flat_question_asset(
        &self,
        context: ActorContext,
        workspace: WorkspaceId,
        asset: AssetId,
        checksum: objects::Sha256Digest,
    ) -> Result<Option<WorkspaceFlatQuestionAsset>, StoreError> {
        let state = self.read_state()?;
        if require_workspace_asset_reader(&state, context, workspace).is_err() {
            return Ok(None);
        }
        Ok(state
            .workspace_flat_question_assets
            .get(&(workspace, asset))
            .filter(|descriptor| descriptor.checksum() == checksum)
            .cloned())
    }
}

fn require_workspace_asset_author(
    state: &super::State,
    context: ActorContext,
    workspace: WorkspaceId,
) -> Result<(), StoreError> {
    match state.draft_access.get(&(workspace, context.user_id())) {
        Some(WorkspaceDraftRole::Owner | WorkspaceDraftRole::Collaborator) => Ok(()),
        None => Err(StoreError::NotFound),
    }
}

fn require_workspace_asset_reader(
    state: &super::State,
    context: ActorContext,
    workspace: WorkspaceId,
) -> Result<(), StoreError> {
    state
        .draft_access
        .contains_key(&(workspace, context.user_id()))
        .then_some(())
        .ok_or(StoreError::NotFound)
}
