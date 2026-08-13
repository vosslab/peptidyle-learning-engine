//! In-memory private workspace-image registry for native flat questions.

use async_trait::async_trait;
use question_model::{AssetId, WorkspaceId};

use super::MemoryStore;
use crate::{
    FlatQuestionAssetStore, StoreError, TenantContext, WorkspaceFlatQuestionAsset, ensure_tenant,
};

#[async_trait]
impl FlatQuestionAssetStore for MemoryStore {
    async fn register_workspace_flat_question_asset(
        &self,
        context: TenantContext,
        descriptor: WorkspaceFlatQuestionAsset,
    ) -> Result<WorkspaceFlatQuestionAsset, StoreError> {
        ensure_tenant(context, descriptor.tenant)?;
        descriptor.validate()?;
        let key = (descriptor.tenant, descriptor.workspace, descriptor.asset);
        let mut state = self.write_state()?;
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
        context: TenantContext,
        workspace: WorkspaceId,
    ) -> Result<Vec<WorkspaceFlatQuestionAsset>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .workspace_flat_question_assets
            .iter()
            .filter(|((tenant, candidate_workspace, _), _)| {
                *tenant == context.tenant_id() && *candidate_workspace == workspace
            })
            .map(|(_, descriptor)| descriptor.clone())
            .collect())
    }

    async fn resolve_workspace_flat_question_asset(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        asset: AssetId,
        checksum: objects::Sha256Digest,
    ) -> Result<Option<WorkspaceFlatQuestionAsset>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .workspace_flat_question_assets
            .get(&(context.tenant_id(), workspace, asset))
            .filter(|descriptor| descriptor.checksum() == checksum)
            .cloned())
    }
}
