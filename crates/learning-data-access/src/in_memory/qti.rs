//! In-memory QTI import and isolated grader persistence.

use async_trait::async_trait;
use question_model::{WorkspaceId, WorkspaceImportId};

use super::{MemoryQtiGraderStore, MemoryStore};
use crate::{
    ActorContext, CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    JobPayload, JobState, PrepareClaimedQtiImport, QtiGradingStore, QtiImportGradingPayload,
    QtiImportRegistry, QtiImportStore, StoreError, validate_qti_import,
};

fn stage_qti_import(
    state: &mut super::State,
    command: CreateQtiImportCommand,
) -> Result<(), StoreError> {
    validate_qti_import(&command)?;
    let reference = command.registry.reference;
    let key = (reference.workspace, reference.import);
    if state.qti_imports.contains_key(&key) {
        return Err(StoreError::Conflict);
    }
    if let Some(existing) = state.prepared_qti_imports.get(&key) {
        let exact_grading = command.item_bindings.len() == existing.items.len()
            && command.item_bindings.iter().all(|binding| {
                state
                    .prepared_qti_grading
                    .get(&(key.0, key.1, binding.item.item_id.clone()))
                    == Some(&binding.grading)
            });
        return if existing == &command.registry && exact_grading {
            Ok(())
        } else {
            Err(StoreError::Conflict)
        };
    }
    for binding in &command.item_bindings {
        state.prepared_qti_grading.insert(
            (key.0, key.1, binding.item.item_id.clone()),
            binding.grading.clone(),
        );
    }
    state.prepared_qti_imports.insert(key, command.registry);
    Ok(())
}

#[async_trait]
impl QtiImportStore for MemoryStore {
    async fn prepare_qti_import(
        &self,
        actor: ActorContext,
        command: CreateQtiImportCommand,
    ) -> Result<(), StoreError> {
        let reference = command.registry.reference;
        let mut state = self.write_state()?;
        if !state.drafts.contains_key(&reference.workspace) {
            return Err(StoreError::NotFound);
        }
        if !state
            .draft_access
            .contains_key(&(reference.workspace, actor.user_id()))
        {
            return Err(StoreError::NotFound);
        }
        stage_qti_import(&mut state, command)
    }

    async fn prepare_claimed_qti_import(
        &self,
        claimed: PrepareClaimedQtiImport,
    ) -> Result<(), StoreError> {
        let reference = claimed.command.registry.reference;
        let source_object = claimed.command.registry.source.id;
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let active = state.jobs.get(&claimed.job).is_some_and(|job| {
            job.state == JobState::Leased
                && job.lease_token == Some(claimed.lease)
                && job.lease_expires_at.is_some_and(|expiry| expiry > now)
                && job.payload
                    == JobPayload::QtiImport {
                        workspace: reference.workspace,
                        import: reference.import,
                        source_object,
                    }
        });
        if !active {
            return Err(StoreError::Conflict);
        }
        stage_qti_import(&mut state, claimed.command)
    }

    async fn commit_prepared_qti_import(
        &self,
        command: CommitPreparedQtiImport,
    ) -> Result<CommitPreparedQtiImportOutcome, StoreError> {
        let key = (command.reference.workspace, command.reference.import);
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let active = state.jobs.get(&command.job).is_some_and(|job| {
            job.state == JobState::Leased
                && job.lease_token == Some(command.lease)
                && job.lease_expires_at.is_some_and(|expiry| expiry > now)
                && job.payload
                    == JobPayload::QtiImport {
                        workspace: key.0,
                        import: key.1,
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
            let grade_key = (key.0, key.1, item.item_id.clone());
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
        actor: ActorContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportRegistry>, StoreError> {
        let state = self.read_state()?;
        if !state
            .draft_access
            .contains_key(&(workspace, actor.user_id()))
        {
            return Ok(None);
        }
        Ok(state.qti_imports.get(&(workspace, import)).cloned())
    }
}

#[async_trait]
impl QtiGradingStore for MemoryQtiGraderStore {
    async fn qti_publication_grading(
        &self,
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
        if record.scope != question_model::PublicationScope::Public {
            return Ok(None);
        }
        Ok(state
            .published_qti_grading
            .get(&(reference.problem, reference.version, item_id.to_string()))
            .cloned())
    }

    async fn qti_import_grading(
        &self,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        // Require the exact committed registry so
        // a guessed item key cannot enumerate private grading records.
        let state = self
            .state
            .read()
            .map_err(|error| StoreError::Unavailable(error.to_string()))?;
        if !state.qti_imports.contains_key(&(workspace, import)) {
            return Ok(None);
        }
        Ok(state
            .qti_grading
            .get(&(workspace, import, item_id.to_string()))
            .cloned())
    }
}
