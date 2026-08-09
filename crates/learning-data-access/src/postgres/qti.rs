//! PostgreSQL QTI import and grader-boundary persistence.

use super::*;
use crate::QtiImportItemStatus;

#[cfg(feature = "postgres")]
#[async_trait]
impl QtiImportStore for PostgresStore {
    async fn prepare_qti_import(
        &self,
        context: TenantContext,
        command: CreateQtiImportCommand,
    ) -> Result<(), StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
        // This is deliberately not implemented as `create` followed by an
        // UPDATE.  A committed row, however briefly present, is observable by
        // a concurrent request and would leak an incomplete import.
        ensure_tenant(context, command.registry.reference.tenant)?;
        validate_qti_import(&command)?;
        let reference = command.registry.reference;
        let (registry_payload, registry_checksum) = encode_payload(&command.registry)?;
        let grading_checksums = Json(Value::Object(
            command
                .item_bindings
                .iter()
                .map(|binding| {
                    (
                        binding.item.item_id.clone(),
                        Value::String(binding.grading.sha256().to_string()),
                    )
                })
                .collect(),
        ));
        let mut transaction = self.begin_tenant(context).await?;
        // Draft deletion takes this same row FOR UPDATE before removing
        // prepared QTI state. Taking KEY SHARE first makes preparation and
        // deletion a single total order: deletion either waits to remove this
        // transaction's staging graph, or wins and makes preparation refuse
        // before any import row is written.
        let draft_exists: Option<i64> = sqlx::query_scalar(
            "SELECT revision FROM workspace_draft WHERE tenant_id = $1 AND workspace_id = $2 FOR KEY SHARE",
        )
        .bind(reference.tenant.as_uuid())
        .bind(reference.workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if draft_exists.is_none() {
            return Err(StoreError::NotFound);
        }
        // Serialize same-import preparation. Hash collisions only add waiting;
        // equality is still checked against the complete typed reference.
        let preparation_lock = format!(
            "{}:{}:{}",
            reference.tenant, reference.workspace, reference.import
        );
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(preparation_lock)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let matches_prepared: bool =
            sqlx::query_scalar("SELECT ple_prepared_qti_import_matches($1, $2, $3, $4, $5, $6)")
                .bind(reference.tenant.as_uuid())
                .bind(reference.workspace.as_uuid())
                .bind(reference.import.as_uuid())
                .bind(registry_payload.clone())
                .bind(&registry_checksum)
                .bind(grading_checksums)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if matches_prepared {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO workspace_qti_import \
             (tenant_id, workspace_id, import_id, source_object_id, payload, payload_sha256, state) \
             VALUES ($1, $2, $3, $4, $5, $6, 'prepared')",
        )
        .bind(reference.tenant.as_uuid())
        .bind(reference.workspace.as_uuid())
        .bind(reference.import.as_uuid())
        .bind(command.registry.source.id.as_uuid())
        .bind(registry_payload)
        .bind(registry_checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        for binding in &command.item_bindings {
            let (item_payload, item_checksum) = encode_payload(&binding.item)?;
            sqlx::query(
                "INSERT INTO workspace_qti_import_item \
                 (tenant_id, workspace_id, import_id, item_id, payload, payload_sha256) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(reference.tenant.as_uuid())
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(&binding.item.item_id)
            .bind(item_payload)
            .bind(item_checksum)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "INSERT INTO workspace_qti_import_grading \
                 (tenant_id, workspace_id, import_id, item_id, payload, payload_sha256) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(reference.tenant.as_uuid())
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(&binding.item.item_id)
            .bind(binding.grading.bytes())
            .bind(Sha256Digest::compute(binding.grading.bytes()).to_string())
            .execute(&mut *transaction)
            .await
                .map_err(map_sqlx_error)?;
        }
        for (ordinal, result) in command.registry.item_results.iter().enumerate() {
            let (payload, checksum) = encode_payload(result)?;
            let status = match result.status {
                QtiImportItemStatus::Accepted => "accepted",
                QtiImportItemStatus::Rejected => "rejected",
            };
            sqlx::query(
                "INSERT INTO workspace_qti_import_result \
                 (tenant_id, workspace_id, import_id, ordinal, source_identifier, status, \
                  normalized_sha256, payload, payload_sha256) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(reference.tenant.as_uuid())
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(i32::try_from(ordinal).map_err(|_| {
                StoreError::InvalidRecord("too many QTI item results".to_string())
            })?)
            .bind(&result.source_identifier)
            .bind(status)
            .bind(result.normalized_sha256.map(|digest| digest.to_string()))
            .bind(payload)
            .bind(checksum)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        for asset in &command.registry.assets {
            let objects::ObjectKey::WorkspaceAsset {
                asset: logical_asset,
                ..
            } = &asset.key
            else {
                return Err(StoreError::InvalidRecord(
                    "validated QTI asset lost its logical identity".to_string(),
                ));
            };
            let (payload, checksum) = encode_payload(asset)?;
            sqlx::query(
                "INSERT INTO workspace_qti_import_asset \
                 (tenant_id, workspace_id, import_id, asset_id, object_id, payload, payload_sha256) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(reference.tenant.as_uuid())
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(logical_asset.as_uuid())
            .bind(asset.id.as_uuid())
            .bind(payload)
            .bind(checksum)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        for (ordinal, feature) in command.registry.unsupported_features.iter().enumerate() {
            let (payload, checksum) = encode_payload(feature)?;
            sqlx::query(
                "INSERT INTO workspace_qti_import_unsupported \
                 (tenant_id, workspace_id, import_id, ordinal, payload, payload_sha256) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(reference.tenant.as_uuid())
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(i32::try_from(ordinal).map_err(|_| {
                StoreError::InvalidRecord("too many QTI unsupported features".to_string())
            })?)
            .bind(payload)
            .bind(checksum)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)
            }
        })
        .await
    }

    async fn commit_prepared_qti_import(
        &self,
        context: TenantContext,
        command: CommitPreparedQtiImport,
    ) -> Result<CommitPreparedQtiImportOutcome, StoreError> {
        ensure_tenant(context, command.reference.tenant)?;
        let mut transaction = self.begin_tenant(context).await?;
        let committed: bool =
            sqlx::query_scalar("SELECT ple_commit_prepared_qti_import($1, $2, $3, $4, $5, $6)")
                .bind(context.tenant_id().as_uuid())
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
        context: TenantContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportRegistry>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM ple_read_committed_qti_import($1, $2, $3)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(import.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl QtiGradingStore for PostgresGraderStore {
    async fn qti_import_grading(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        let mut transaction = self.begin_grader_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM ple_read_committed_qti_grading($1, $2, $3, $4)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(import.as_uuid())
        .bind(item_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let material = row
            .as_ref()
            .map(|row| {
                let bytes: Vec<u8> = row.try_get("payload").map_err(map_sqlx_error)?;
                let expected: String = row.try_get("payload_sha256").map_err(map_sqlx_error)?;
                if Sha256Digest::compute(&bytes).to_string() != expected {
                    return Err(StoreError::Unavailable(
                        "stored QTI grading payload checksum mismatch".to_string(),
                    ));
                }
                QtiImportGradingPayload::new(bytes)
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(material)
    }

    async fn qti_published_grading(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        let mut transaction = self.begin_grader_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM ple_read_published_qti_grading($1, $2, $3, $4)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .bind(item_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let material = decode_qti_grading_row(row.as_ref())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(material)
    }
}
#[cfg(feature = "postgres")]
fn decode_qti_grading_row(
    row: Option<&PgRow>,
) -> Result<Option<QtiImportGradingPayload>, StoreError> {
    row.map(|row| {
        let bytes: Vec<u8> = row.try_get("payload").map_err(map_sqlx_error)?;
        let expected: String = row.try_get("payload_sha256").map_err(map_sqlx_error)?;
        if Sha256Digest::compute(&bytes).to_string() != expected {
            return Err(StoreError::Unavailable(
                "stored QTI grading payload checksum mismatch".to_string(),
            ));
        }
        QtiImportGradingPayload::new(bytes)
    })
    .transpose()
}
