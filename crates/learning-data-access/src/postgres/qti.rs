//! PostgreSQL QTI import and grader-boundary persistence.

use std::collections::BTreeMap;

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{ProblemVersionRef, WorkspaceId, WorkspaceImportId};
use serde_json::Value;
use sqlx::Row;

use super::{PostgresGraderStore, PostgresStore, map_sqlx_error};
use crate::{
    ActorContext, CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    PrepareClaimedQtiImport, QtiGradingStore, QtiImportGradingPayload, QtiImportRegistry,
    QtiImportStore, StoreError, validate_qti_import,
};

fn registry_payload(registry: &QtiImportRegistry) -> Result<(Value, String), StoreError> {
    let value = serde_json::to_value(registry)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let bytes =
        serde_json::to_vec(&value).map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    Ok((value, Sha256Digest::compute(&bytes).to_string()))
}

fn grading_bindings_sha256(command: &CreateQtiImportCommand) -> Result<String, StoreError> {
    let bindings = command
        .item_bindings
        .iter()
        .map(|binding| {
            (
                binding.item.item_id.clone(),
                binding.grading.sha256().to_string(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let bytes = serde_json::to_vec(&bindings)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    Ok(Sha256Digest::compute(&bytes).to_string())
}

fn decode_grading_row(row: &sqlx::postgres::PgRow) -> Result<QtiImportGradingPayload, StoreError> {
    let payload: Vec<u8> = row.try_get("payload").map_err(map_sqlx_error)?;
    let expected: String = row.try_get("payload_sha256").map_err(map_sqlx_error)?;
    if Sha256Digest::compute(&payload).to_string() != expected {
        return Err(StoreError::Unavailable(
            "stored QTI grading payload checksum mismatch".to_string(),
        ));
    }
    QtiImportGradingPayload::new(payload)
}

#[async_trait]
impl QtiImportStore for PostgresStore {
    async fn prepare_qti_import(
        &self,
        actor: ActorContext,
        command: CreateQtiImportCommand,
    ) -> Result<(), StoreError> {
        validate_qti_import(&command)?;
        let reference = command.registry.reference;
        let (registry, registry_sha256) = registry_payload(&command.registry)?;
        let source_record = serde_json::to_value(&command.registry.source)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let bindings_sha256 = grading_bindings_sha256(&command)?;
        let mut transaction = self.begin_actor(actor).await?;
        let draft_exists: Option<i32> = sqlx::query_scalar(
            "SELECT revision FROM ple_private.workspace_draft_question \
             WHERE workspace_id = $1 FOR KEY SHARE",
        )
        .bind(reference.workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if draft_exists.is_none() {
            return Err(StoreError::NotFound);
        }
        let existing: Option<(String, String, String)> = sqlx::query_as(
            "SELECT state, registry_sha256, grading_bindings_sha256 \
             FROM ple_private.workspace_qti_import \
             WHERE workspace_id = $1 AND import_id = $2 FOR UPDATE",
        )
        .bind(reference.workspace.as_uuid())
        .bind(reference.import.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if let Some((state, existing_registry_sha256, existing_bindings_sha256)) = existing {
            if state == "prepared"
                && existing_registry_sha256 == registry_sha256
                && existing_bindings_sha256 == bindings_sha256
            {
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(());
            }
            return Err(StoreError::Conflict);
        }
        sqlx::query(
            "INSERT INTO ple_private.workspace_qti_import \
             (workspace_id, import_id, source_record, registry, registry_sha256, \
              grading_bindings_sha256, state, prepared_at) \
             VALUES ($1, $2, $3, $4, $5, $6, 'prepared', transaction_timestamp())",
        )
        .bind(reference.workspace.as_uuid())
        .bind(reference.import.as_uuid())
        .bind(source_record)
        .bind(registry)
        .bind(registry_sha256)
        .bind(bindings_sha256)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        for binding in &command.item_bindings {
            sqlx::query(
                "INSERT INTO ple_private.workspace_qti_import_grading \
                 (workspace_id, import_id, item_id, payload, payload_sha256, created_at) \
                 VALUES ($1, $2, $3, $4, $5, transaction_timestamp())",
            )
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(&binding.item.item_id)
            .bind(binding.grading.bytes())
            .bind(binding.grading.sha256().to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn prepare_claimed_qti_import(
        &self,
        claimed: PrepareClaimedQtiImport,
    ) -> Result<(), StoreError> {
        validate_qti_import(&claimed.command)?;
        let reference = claimed.command.registry.reference;
        let source_object = claimed.command.registry.source.id;
        let (registry, registry_sha256) = registry_payload(&claimed.command.registry)?;
        let source_record = serde_json::to_value(&claimed.command.registry.source)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let bindings_sha256 = grading_bindings_sha256(&claimed.command)?;
        let mut transaction = self.begin_worker(claimed.job, claimed.lease).await?;
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM ple_private.worker_job \\
             WHERE job_id = $1 AND handler_kind = 'qti_import' \\
               AND target_kind = 'qti_import' AND workspace_id = $3 AND import_id = $4 \\
               AND source_object_id = $5 AND state = 'leased' AND lease_token = $2 \\
               AND lease_expires_at > transaction_timestamp())",
        )
        .bind(claimed.job.as_uuid())
        .bind(claimed.lease.as_uuid())
        .bind(reference.workspace.as_uuid())
        .bind(reference.import.as_uuid())
        .bind(source_object.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !active {
            return Err(StoreError::Conflict);
        }
        let existing: Option<(String, String, String)> = sqlx::query_as(
            "SELECT state, registry_sha256, grading_bindings_sha256 \\
             FROM ple_private.workspace_qti_import \\
             WHERE workspace_id = $1 AND import_id = $2 FOR UPDATE",
        )
        .bind(reference.workspace.as_uuid())
        .bind(reference.import.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if let Some((state, existing_registry_sha256, existing_bindings_sha256)) = existing {
            if state == "prepared"
                && existing_registry_sha256 == registry_sha256
                && existing_bindings_sha256 == bindings_sha256
            {
                return transaction.commit().await.map_err(map_sqlx_error);
            }
            return Err(StoreError::Conflict);
        }
        sqlx::query(
            "INSERT INTO ple_private.workspace_qti_import \\
             (workspace_id, import_id, source_record, registry, registry_sha256, \\
              grading_bindings_sha256, state, prepared_at) \\
             VALUES ($1, $2, $3, $4, $5, $6, 'prepared', transaction_timestamp())",
        )
        .bind(reference.workspace.as_uuid())
        .bind(reference.import.as_uuid())
        .bind(source_record)
        .bind(registry)
        .bind(registry_sha256)
        .bind(bindings_sha256)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        for binding in &claimed.command.item_bindings {
            sqlx::query(
                "INSERT INTO ple_private.workspace_qti_import_grading \\
                 (workspace_id, import_id, item_id, payload, payload_sha256, created_at) \\
                 VALUES ($1, $2, $3, $4, $5, transaction_timestamp())",
            )
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(&binding.item.item_id)
            .bind(binding.grading.bytes())
            .bind(binding.grading.sha256().to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn commit_prepared_qti_import(
        &self,
        command: CommitPreparedQtiImport,
    ) -> Result<CommitPreparedQtiImportOutcome, StoreError> {
        let mut transaction = self.begin_worker(command.job, command.lease).await?;
        let committed: bool = sqlx::query_scalar(
            "WITH active_job AS ( \
                 SELECT 1 FROM ple_private.worker_job \
                 WHERE job_id = $1 AND handler_kind = 'qti_import' \
                   AND target_kind = 'qti_import' AND workspace_id = $3 AND import_id = $4 \
                   AND source_object_id = $5 \
                   AND state = 'leased' AND lease_token = $2 \
                   AND lease_expires_at > transaction_timestamp() \
             ), committed_import AS ( \
                 UPDATE ple_private.workspace_qti_import \
                 SET state = 'committed', committed_at = transaction_timestamp() \
                 WHERE workspace_id = $3 AND import_id = $4 AND state = 'prepared' \
                   AND EXISTS (SELECT 1 FROM active_job) \
                 RETURNING 1 \
             ), completed_job AS ( \
                 UPDATE ple_private.worker_job \
                 SET state = 'completed', lease_token = NULL, lease_expires_at = NULL, \
                     completed_at = transaction_timestamp() \
                 WHERE job_id = $1 AND lease_token = $2 AND state = 'leased' \
                   AND EXISTS (SELECT 1 FROM committed_import) \
                 RETURNING 1 \
             ) SELECT EXISTS (SELECT 1 FROM completed_job)",
        )
        .bind(command.job.as_uuid())
        .bind(command.lease.as_uuid())
        .bind(command.reference.workspace.as_uuid())
        .bind(command.reference.import.as_uuid())
        .bind(command.source_object.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(if committed {
            CommitPreparedQtiImportOutcome::Committed
        } else {
            CommitPreparedQtiImportOutcome::ClaimNoLongerActive
        })
    }

    async fn get_qti_import(
        &self,
        actor: ActorContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportRegistry>, StoreError> {
        let mut transaction = self.begin_actor(actor).await?;
        let row = sqlx::query(
            "SELECT registry FROM ple_private.workspace_qti_import \
             WHERE workspace_id = $1 AND import_id = $2 AND state = 'committed'",
        )
        .bind(workspace.as_uuid())
        .bind(import.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let registry = row
            .map(|row| row.try_get::<Value, _>("registry").map_err(map_sqlx_error))
            .transpose()?
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| StoreError::Unavailable(error.to_string()))?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(registry)
    }
}

#[async_trait]
impl QtiGradingStore for PostgresGraderStore {
    async fn qti_publication_grading(
        &self,
        reference: ProblemVersionRef,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        let mut transaction = self.begin_grader().await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM ple_private.published_qti_question_grading \
             WHERE problem_id = $1 AND version_id = $2 AND item_id = $3",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .bind(item_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let material = row.as_ref().map(decode_grading_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(material)
    }

    async fn qti_import_grading(
        &self,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        let mut transaction = self.begin_grader().await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM ple_private.workspace_qti_import_grading \
             WHERE workspace_id = $1 AND import_id = $2 AND item_id = $3",
        )
        .bind(workspace.as_uuid())
        .bind(import.as_uuid())
        .bind(item_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let material = row.as_ref().map(decode_grading_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(material)
    }
}
