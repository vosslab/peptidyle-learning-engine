//! In-memory atomic QTI queueing and status projection.

use async_trait::async_trait;
use question_model::{WorkspaceId, WorkspaceImportId};

use super::{MemoryStore, State, StoredJob};
use crate::{
    JobPayload, JobState, QtiImportApiState, QtiImportApiStore, QtiImportApiView, QtiImportRef,
    ActorContext, QueueQtiImportCommand, StoreError, WorkspaceDraftRole,
    qti_import_job_id, validate_queue_qti_import,
};

fn actor_can_access_workspace(
    state: &State,
    actor: ActorContext,
    workspace: WorkspaceId,
) -> bool {
    state.drafts.contains_key(&workspace)
        && matches!(
            state.draft_access.get(&(workspace, actor.user_id())),
            Some(WorkspaceDraftRole::Owner | WorkspaceDraftRole::Collaborator)
        )
}

fn import_view(
    state: &State,
    reference: QtiImportRef,
) -> Result<Option<QtiImportApiView>, StoreError> {
    let job_id = qti_import_job_id(reference);
    let Some(job) = state.jobs.get(&job_id) else {
        return Ok(None);
    };
    let expected_payload = JobPayload::QtiImport {
        workspace: reference.workspace,
        import: reference.import,
        source_object: objects::workspace_qti_archive_object_id(
            reference.workspace,
            reference.import,
        ),
    };
    if job.payload != expected_payload {
        return Err(StoreError::Unavailable(
            "deterministic QTI job identity is bound to different work".to_string(),
        ));
    }
    let key = (reference.workspace, reference.import);
    let registry = state.qti_imports.get(&key).cloned();
    let api_state = derive_api_state(job.state, registry.is_some())?;
    Ok(Some(QtiImportApiView {
        reference,
        state: api_state,
        registry,
    }))
}

fn derive_api_state(
    job_state: JobState,
    has_committed_registry: bool,
) -> Result<QtiImportApiState, StoreError> {
    match (job_state, has_committed_registry) {
        (JobState::Ready, false) => Ok(QtiImportApiState::Queued),
        (JobState::Leased, false) => Ok(QtiImportApiState::Processing),
        (JobState::Dead, false) => Ok(QtiImportApiState::Failed),
        (JobState::Completed, true) => Ok(QtiImportApiState::Ready),
        _ => Err(StoreError::Unavailable(
            "QTI import job and committed registry disagree".to_string(),
        )),
    }
}

#[async_trait]
impl QtiImportApiStore for MemoryStore {
    async fn queue_qti_import(
        &self,
        actor: ActorContext,
        command: QueueQtiImportCommand,
    ) -> Result<QtiImportApiView, StoreError> {
        validate_queue_qti_import(&command)?;
        let mut state = self.write_state()?;
        if !actor_can_access_workspace(&state, actor, command.reference.workspace) {
            return Err(StoreError::NotFound);
        }
        let job_id = qti_import_job_id(command.reference);
        let payload = JobPayload::QtiImport {
            workspace: command.reference.workspace,
            import: command.reference.import,
            source_object: command.source.id,
        };
        if let Some(existing) = state.jobs.get(&job_id) {
            if existing.payload != payload
                || existing.max_attempts != command.max_attempts
            {
                return Err(StoreError::Conflict);
            }
            return import_view(&state, command.reference)?.ok_or_else(|| {
                StoreError::Unavailable("QTI import replay lost its durable job".to_string())
            });
        }

        let available_at = state.authoritative_time;
        state.jobs.insert(
            job_id,
            StoredJob {
                payload,
                state: JobState::Ready,
                available_at,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: command.max_attempts,
                failure: None,
            },
        );
        import_view(&state, command.reference)?.ok_or_else(|| {
            StoreError::Unavailable("QTI import queueing lost its durable job".to_string())
        })
    }

    async fn qti_import_view(
        &self,
        actor: ActorContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportApiView>, StoreError> {
        let state = self.read_state()?;
        if !actor_can_access_workspace(&state, actor, workspace) {
            return Ok(None);
        }
        import_view(
            &state,
            QtiImportRef {
                workspace,
                import,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_requires_completed_job_and_committed_registry() {
        assert_eq!(
            derive_api_state(JobState::Completed, true),
            Ok(QtiImportApiState::Ready)
        );
        assert!(matches!(
            derive_api_state(JobState::Completed, false),
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            derive_api_state(JobState::Ready, true),
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            derive_api_state(JobState::Leased, true),
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            derive_api_state(JobState::Dead, true),
            Err(StoreError::Unavailable(_))
        ));
    }
}
