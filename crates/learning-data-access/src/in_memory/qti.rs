//! In-memory QTI import and isolated grader persistence.

use async_trait::async_trait;
use question_model::{WorkspaceId, WorkspaceImportId};

use super::{MemoryQtiGraderStore, MemoryStore, catalog_record_visible};
use crate::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand, JobPayload,
    JobState, QtiGradingStore, QtiImportGradingPayload, QtiImportRegistry, QtiImportStore,
    StoreError, TenantContext, ensure_tenant, validate_qti_import,
};

#[async_trait]
impl QtiImportStore for MemoryStore {
    async fn prepare_qti_import(
        &self,
        context: TenantContext,
        command: CreateQtiImportCommand,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, command.registry.reference.tenant)?;
        validate_qti_import(&command)?;
        let reference = command.registry.reference;
        let key = (reference.tenant, reference.workspace, reference.import);
        let mut state = self.write_state()?;
        if !state
            .drafts
            .contains_key(&(reference.tenant, reference.workspace))
        {
            return Err(StoreError::NotFound);
        }
        if state.qti_imports.contains_key(&key) {
            return Err(StoreError::Conflict);
        }
        if let Some(existing) = state.prepared_qti_imports.get(&key) {
            let exact_grading = command.item_bindings.len() == existing.items.len()
                && command.item_bindings.iter().all(|binding| {
                    state.prepared_qti_grading.get(&(
                        key.0,
                        key.1,
                        key.2,
                        binding.item.item_id.clone(),
                    )) == Some(&binding.grading)
                });
            return if existing == &command.registry && exact_grading {
                Ok(())
            } else {
                Err(StoreError::Conflict)
            };
        }
        for binding in &command.item_bindings {
            state.prepared_qti_grading.insert(
                (key.0, key.1, key.2, binding.item.item_id.clone()),
                binding.grading.clone(),
            );
        }
        state.prepared_qti_imports.insert(key, command.registry);
        Ok(())
    }

    async fn commit_prepared_qti_import(
        &self,
        context: TenantContext,
        command: CommitPreparedQtiImport,
    ) -> Result<CommitPreparedQtiImportOutcome, StoreError> {
        ensure_tenant(context, command.reference.tenant)?;
        let key = (
            command.reference.tenant,
            command.reference.workspace,
            command.reference.import,
        );
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let active = state.jobs.get(&command.job).is_some_and(|job| {
            job.tenant == context.tenant_id()
                && job.state == JobState::Leased
                && job.lease_token == Some(command.lease)
                && job.lease_expires_at.is_some_and(|expiry| expiry > now)
                && job.payload
                    == JobPayload::QtiImport {
                        workspace: key.1,
                        import: key.2,
                        source_object: command.source_object,
                    }
        });
        if !active {
            return Ok(CommitPreparedQtiImportOutcome::ClaimNoLongerActive);
        }
        let registry = state
            .prepared_qti_imports
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if registry.source.id != command.source_object {
            return Err(StoreError::Conflict);
        }
        if !state
            .qti_profile_import_evidence
            .completes_recognized_registry(&registry)
        {
            // Keep the active lease intact, matching the PostgreSQL commit
            // capability's false outcome when private staging is incomplete.
            return Ok(CommitPreparedQtiImportOutcome::ClaimNoLongerActive);
        }
        for item in &registry.items {
            let grade_key = (key.0, key.1, key.2, item.item_id.clone());
            let material = state
                .prepared_qti_grading
                .remove(&grade_key)
                .ok_or(StoreError::Conflict)?;
            state.qti_grading.insert(grade_key, material);
        }
        state.prepared_qti_imports.remove(&key);
        state.qti_imports.insert(key, registry);
        let job = state
            .jobs
            .get_mut(&command.job)
            .ok_or(StoreError::NotFound)?;
        job.state = JobState::Completed;
        job.lease_token = None;
        job.lease_expires_at = None;
        Ok(CommitPreparedQtiImportOutcome::Committed)
    }

    async fn get_qti_import(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportRegistry>, StoreError> {
        Ok(self
            .read_state()?
            .qti_imports
            .get(&(context.tenant_id(), workspace, import))
            .cloned())
    }
}

#[async_trait]
impl QtiGradingStore for MemoryQtiGraderStore {
    async fn qti_publication_grading(
        &self,
        context: TenantContext,
        reference: question_model::ProblemVersionRef,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        let state = self
            .state
            .read()
            .map_err(|error| StoreError::Unavailable(error.to_string()))?;
        let Some(record) = state.published.get(&(reference.problem, reference.version)) else {
            return Ok(None);
        };
        if !catalog_record_visible(&state, context.tenant_id(), record) {
            return Ok(None);
        }
        Ok(state
            .published_qti_grading
            .get(&(reference.problem, reference.version, item_id.to_string()))
            .cloned())
    }

    async fn qti_import_grading(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        // Require the public registry under the same tenant/workspace scope so
        // a guessed item key cannot enumerate private grading records.
        let state = self
            .state
            .read()
            .map_err(|error| StoreError::Unavailable(error.to_string()))?;
        if !state
            .qti_imports
            .contains_key(&(context.tenant_id(), workspace, import))
        {
            return Ok(None);
        }
        Ok(state
            .qti_grading
            .get(&(context.tenant_id(), workspace, import, item_id.to_string()))
            .cloned())
    }
}
