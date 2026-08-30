//! PostgreSQL atomic QTI queueing and status projection.

use async_trait::async_trait;
use question_model::{WorkspaceId, WorkspaceImportId};
use serde_json::Value;
use sqlx::{Row, postgres::PgRow};

use super::{PostgresStore, map_sqlx_error};
use crate::{
    ActorContext, JobPayload, QtiImportApiState, QtiImportApiStore, QtiImportApiView,
    QtiImportRef, QtiImportRegistry, QueueQtiImportCommand, StoreError, qti_import_job_id,
    validate_queue_qti_import,
};

struct StoredQtiImportView {
    payload: Value,
    state: String,
    max_attempts: u16,
    registry: Option<QtiImportRegistry>,
}

fn expected_payload(reference: QtiImportRef) -> Result<Value, StoreError> {
    serde_json::to_value(JobPayload::QtiImport {
        workspace: reference.workspace,
        import: reference.import,
        source_object: objects::workspace_qti_archive_object_id(reference.workspace, reference.import),
    })
    .map_err(|error| StoreError::InvalidRecord(format!("job payload serialization failed: {error}")))
}

fn derive_api_state(job_state: &str, has_committed_registry: bool) -> Result<QtiImportApiState, StoreError> {
    match (job_state, has_committed_registry) {
        ("ready", false) => Ok(QtiImportApiState::Queued),
        ("leased", false) => Ok(QtiImportApiState::Processing),
        ("dead", false) => Ok(QtiImportApiState::Failed),
        ("completed", true) => Ok(QtiImportApiState::Ready),
        _ => Err(StoreError::Unavailable(
            "QTI import job and committed registry disagree".to_string(),
        )),
    }
}

fn decode_stored_view(row: &PgRow) -> Result<StoredQtiImportView, StoreError> {
    let max_attempts = u16::try_from(row.try_get::<i32, _>("max_attempts").map_err(map_sqlx_error)?)
        .map_err(|_| StoreError::Unavailable("stored QTI job retry bound is invalid".to_string()))?;
    let registry = row
        .try_get::<Option<Value>, _>("registry")
        .map_err(map_sqlx_error)?
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| StoreError::Unavailable(error.to_string()))?;
    Ok(StoredQtiImportView {
        payload: row.try_get("payload").map_err(map_sqlx_error)?,
        state: row.try_get("state").map_err(map_sqlx_error)?,
        max_attempts,
        registry,
    })
}

fn project_view(
    stored: StoredQtiImportView,
    reference: QtiImportRef,
    payload: &Value,
) -> Result<QtiImportApiView, StoreError> {
    if &stored.payload != payload {
        return Err(StoreError::Unavailable(
            "deterministic QTI job identity is bound to different work".to_string(),
        ));
    }
    if stored
        .registry
        .as_ref()
        .is_some_and(|registry| registry.reference != reference)
    {
        return Err(StoreError::Unavailable(
            "committed QTI registry identity is inconsistent".to_string(),
        ));
    }
    Ok(QtiImportApiView {
        reference,
        state: derive_api_state(&stored.state, stored.registry.is_some())?,
        registry: stored.registry,
    })
}

async fn fetch_stored_view(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    reference: QtiImportRef,
) -> Result<Option<StoredQtiImportView>, StoreError> {
    let row = sqlx::query(
        "SELECT job.payload, job.state, job.max_attempts, import_registry.registry \
         FROM ple_private.worker_job AS job \
         LEFT JOIN ple_private.workspace_qti_import AS import_registry \
           ON import_registry.workspace_id = job.workspace_id \
          AND import_registry.import_id = job.import_id \
          AND import_registry.state = 'committed' \
         WHERE job.job_id = $1 \
           AND job.target_kind = 'qti_import' \
           AND job.workspace_id = $2 AND job.import_id = $3",
    )
    .bind(qti_import_job_id(reference).as_uuid())
    .bind(reference.workspace.as_uuid())
    .bind(reference.import.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    row.as_ref().map(decode_stored_view).transpose()
}

#[async_trait]
impl QtiImportApiStore for PostgresStore {
    async fn queue_qti_import(
        &self,
        actor: ActorContext,
        command: QueueQtiImportCommand,
    ) -> Result<QtiImportApiView, StoreError> {
        validate_queue_qti_import(&command)?;
        let reference = command.reference;
        let payload = expected_payload(reference)?;
        let payload_digest = objects::Sha256Digest::compute(
            &serde_json::to_vec(&payload)
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
        );
        let mut transaction = self.begin_actor(actor).await?;
        let workspace_exists: Option<uuid::Uuid> = sqlx::query_scalar(
            "SELECT workspace_id FROM ple_private.workspace_draft_question \
             WHERE workspace_id = $1 FOR KEY SHARE",
        )
        .bind(reference.workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if workspace_exists.is_none() {
            return Err(StoreError::NotFound);
        }
        sqlx::query(
            "INSERT INTO ple_private.worker_job \
             (job_id, handler_kind, target_kind, workspace_id, import_id, source_object_id, \
              generation, target_digest, payload, state, available_at, max_attempts, created_at) \
             VALUES ($1, 'qti_import', 'qti_import', $2, $3, $4, 1, $5, $6, 'ready', \
                     transaction_timestamp(), $7, transaction_timestamp()) \
             ON CONFLICT (job_id) DO NOTHING",
        )
        .bind(qti_import_job_id(reference).as_uuid())
        .bind(reference.workspace.as_uuid())
        .bind(reference.import.as_uuid())
        .bind(command.source.id.as_uuid())
        .bind(payload_digest.as_bytes().to_vec())
        .bind(&payload)
        .bind(i32::from(command.max_attempts))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let stored = fetch_stored_view(&mut transaction, reference)
            .await?
            .ok_or_else(|| StoreError::Unavailable("QTI import queueing lost its durable job".to_string()))?;
        if stored.max_attempts != command.max_attempts {
            return Err(StoreError::Conflict);
        }
        let view = project_view(stored, reference, &payload)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(view)
    }

    async fn qti_import_view(
        &self,
        actor: ActorContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportApiView>, StoreError> {
        let reference = QtiImportRef { workspace, import };
        let payload = expected_payload(reference)?;
        let mut transaction = self.begin_actor(actor).await?;
        let stored = fetch_stored_view(&mut transaction, reference).await?;
        let view = stored
            .map(|stored| project_view(stored, reference, &payload))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_atomic_worker_completion_is_ready() {
        assert_eq!(derive_api_state("completed", true), Ok(QtiImportApiState::Ready));
        for (state, registry) in [
            ("completed", false),
            ("ready", true),
            ("leased", true),
            ("dead", true),
            ("unknown", false),
        ] {
            assert!(matches!(
                derive_api_state(state, registry),
                Err(StoreError::Unavailable(_))
            ));
        }
    }
}
