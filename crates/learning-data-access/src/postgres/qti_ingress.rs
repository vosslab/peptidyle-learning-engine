//! PostgreSQL atomic QTI queueing and status projection.

use async_trait::async_trait;
use question_model::{TenantId, UserId, WorkspaceId, WorkspaceImportId};
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::{PostgresStore, decode_payload_row_named, map_sqlx_error};
use crate::{
    JobPayload, QtiImportApiState, QtiImportApiStore, QtiImportApiView, QtiImportRef,
    QtiImportRegistry, QueueQtiImportCommand, StoreError, TenantContext, ensure_tenant,
    qti_import_job_id, validate_queue_qti_import,
};

const ACCESS_LOCK_SQL: &str = "SELECT access.role \
 FROM workspace_draft AS draft \
 JOIN workspace_draft_access AS access \
   ON access.tenant_id = draft.tenant_id AND access.workspace_id = draft.workspace_id \
 WHERE draft.tenant_id = $1 AND draft.workspace_id = $2 \
   AND access.user_id = $3 AND access.role IN ('owner', 'collaborator') \
 FOR KEY SHARE OF draft, access";

const IMPORT_VIEW_SQL: &str = "SELECT job.tenant_id, job.payload AS job_payload, \
        job.state AS job_state, job.max_attempts, \
        registry.payload AS registry_payload, \
        registry.payload_sha256 AS registry_payload_sha256, \
        (registry.payload IS NOT NULL) AS registry_present \
 FROM workspace_draft AS draft \
 JOIN workspace_draft_access AS access \
   ON access.tenant_id = draft.tenant_id AND access.workspace_id = draft.workspace_id \
 JOIN worker_job AS job ON job.job_id = $4 \
 LEFT JOIN LATERAL ple_read_committed_qti_import($1, $2, $3) AS registry ON true \
 WHERE draft.tenant_id = $1 AND draft.workspace_id = $2 \
   AND access.user_id = $5 AND access.role IN ('owner', 'collaborator')";

struct StoredQtiImportView {
    tenant: TenantId,
    payload: Value,
    state: String,
    max_attempts: u16,
    registry: Option<QtiImportRegistry>,
}

fn expected_payload(reference: QtiImportRef) -> Result<Value, StoreError> {
    serde_json::to_value(JobPayload::QtiImport {
        workspace: reference.workspace,
        import: reference.import,
        source_object: objects::workspace_qti_archive_object_id(
            reference.tenant,
            reference.workspace,
            reference.import,
        ),
    })
    .map_err(|error| {
        StoreError::InvalidRecord(format!("job payload serialization failed: {error}"))
    })
}

fn derive_api_state(
    job_state: &str,
    has_committed_registry: bool,
) -> Result<QtiImportApiState, StoreError> {
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
    let max_attempts = row
        .try_get::<i32, _>("max_attempts")
        .map_err(map_sqlx_error)?;
    let max_attempts = u16::try_from(max_attempts).map_err(|_| {
        StoreError::Unavailable("stored QTI job retry bound is invalid".to_string())
    })?;
    let registry = if row
        .try_get::<bool, _>("registry_present")
        .map_err(map_sqlx_error)?
    {
        Some(decode_payload_row_named(
            row,
            "registry_payload",
            "registry_payload_sha256",
        )?)
    } else {
        None
    };
    let Json(payload): Json<Value> = row.try_get("job_payload").map_err(map_sqlx_error)?;
    Ok(StoredQtiImportView {
        tenant: TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?),
        payload,
        state: row.try_get("job_state").map_err(map_sqlx_error)?,
        max_attempts,
        registry,
    })
}

fn project_view(
    stored: StoredQtiImportView,
    reference: QtiImportRef,
    payload: &Value,
) -> Result<QtiImportApiView, StoreError> {
    if stored.tenant != reference.tenant || &stored.payload != payload {
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
    let state = derive_api_state(&stored.state, stored.registry.is_some())?;
    Ok(QtiImportApiView {
        reference,
        state,
        registry: stored.registry,
    })
}

async fn fetch_stored_view(
    transaction: &mut Transaction<'_, Postgres>,
    actor: UserId,
    reference: QtiImportRef,
) -> Result<Option<StoredQtiImportView>, StoreError> {
    let row = sqlx::query(IMPORT_VIEW_SQL)
        .bind(reference.tenant.as_uuid())
        .bind(reference.workspace.as_uuid())
        .bind(reference.import.as_uuid())
        .bind(qti_import_job_id(reference).as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    row.as_ref().map(decode_stored_view).transpose()
}

#[async_trait]
impl QtiImportApiStore for PostgresStore {
    async fn queue_qti_import(
        &self,
        context: TenantContext,
        actor: UserId,
        command: QueueQtiImportCommand,
    ) -> Result<QtiImportApiView, StoreError> {
        ensure_tenant(context, command.reference.tenant)?;
        validate_queue_qti_import(&command)?;
        let reference = command.reference;
        let payload = expected_payload(reference)?;
        let mut transaction = self.begin_tenant(context).await?;
        let access = sqlx::query(ACCESS_LOCK_SQL)
            .bind(reference.tenant.as_uuid())
            .bind(reference.workspace.as_uuid())
            .bind(actor.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if access.is_none() {
            return Err(StoreError::NotFound);
        }

        sqlx::query(
            "INSERT INTO worker_job (job_id, tenant_id, payload, state, max_attempts) \
             VALUES ($1, $2, $3, 'ready', $4) \
             ON CONFLICT (job_id) DO NOTHING",
        )
        .bind(qti_import_job_id(reference).as_uuid())
        .bind(reference.tenant.as_uuid())
        .bind(&payload)
        .bind(i32::from(command.max_attempts))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        let stored = fetch_stored_view(&mut transaction, actor, reference)
            .await?
            .ok_or_else(|| {
                StoreError::Unavailable(
                    "deterministic QTI job collision is not visible to this tenant".to_string(),
                )
            })?;
        if stored.tenant != reference.tenant
            || stored.payload != payload
            || stored.max_attempts != command.max_attempts
        {
            return Err(StoreError::Conflict);
        }
        let view = project_view(stored, reference, &payload)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(view)
    }

    async fn qti_import_view(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportApiView>, StoreError> {
        let reference = QtiImportRef {
            tenant: context.tenant_id(),
            workspace,
            import,
        };
        let payload = expected_payload(reference)?;
        let mut transaction = self.begin_tenant(context).await?;
        let stored = fetch_stored_view(&mut transaction, actor, reference).await?;
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
        assert_eq!(
            derive_api_state("completed", true),
            Ok(QtiImportApiState::Ready)
        );
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

    #[test]
    fn nonterminal_and_failed_jobs_have_coarse_states() {
        assert_eq!(
            derive_api_state("ready", false),
            Ok(QtiImportApiState::Queued)
        );
        assert_eq!(
            derive_api_state("leased", false),
            Ok(QtiImportApiState::Processing)
        );
        assert_eq!(
            derive_api_state("dead", false),
            Ok(QtiImportApiState::Failed)
        );
    }
}
