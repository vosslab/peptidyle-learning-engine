//! PostgreSQL immutable registry for private flat-question image descriptors.

use async_trait::async_trait;
use question_model::{AssetId, WorkspaceId};

use super::{PostgresStore, decode_payload_row, map_sqlx_error, retry_transaction};
use crate::{FlatQuestionAssetStore, StoreError, TenantContext, WorkspaceFlatQuestionAsset};

#[async_trait]
impl FlatQuestionAssetStore for PostgresStore {
    async fn register_workspace_flat_question_asset(
        &self,
        context: TenantContext,
        descriptor: WorkspaceFlatQuestionAsset,
    ) -> Result<WorkspaceFlatQuestionAsset, StoreError> {
        descriptor.validate()?;
        retry_transaction(|| {
            let descriptor = descriptor.clone();
            async move {
                self.register_flat_question_asset_in_transaction(context, descriptor)
                    .await
            }
        })
        .await
    }

    async fn list_workspace_flat_question_assets(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
    ) -> Result<Vec<WorkspaceFlatQuestionAsset>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT payload, payload_sha256 FROM workspace_flat_question_asset \
             WHERE tenant_id = $1 AND workspace_id = $2 ORDER BY asset_id ASC",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let descriptors = rows
            .iter()
            .map(decode_workspace_flat_question_asset_row)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(descriptors)
    }

    async fn resolve_workspace_flat_question_asset(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        asset: AssetId,
        checksum: objects::Sha256Digest,
    ) -> Result<Option<WorkspaceFlatQuestionAsset>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM workspace_flat_question_asset \
             WHERE tenant_id = $1 AND workspace_id = $2 AND asset_id = $3 \
               AND payload #>> '{object,sha256}' = $4",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(asset.as_uuid())
        .bind(checksum.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let descriptor = row
            .as_ref()
            .map(decode_workspace_flat_question_asset_row)
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(descriptor)
    }
}

impl PostgresStore {
    async fn register_flat_question_asset_in_transaction(
        &self,
        context: TenantContext,
        descriptor: WorkspaceFlatQuestionAsset,
    ) -> Result<WorkspaceFlatQuestionAsset, StoreError> {
        let (payload, payload_sha256) = super::encode_payload(&descriptor)?;
        let mut transaction = self.begin_tenant(context).await?;
        let inserted = sqlx::query(
            "INSERT INTO workspace_flat_question_asset \
             (tenant_id, workspace_id, asset_id, object_id, payload, payload_sha256, \
              intrinsic_width, intrinsic_height, media_type) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT DO NOTHING",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(descriptor.workspace.as_uuid())
        .bind(descriptor.asset.as_uuid())
        .bind(descriptor.object.id.as_uuid())
        .bind(payload)
        .bind(&payload_sha256)
        .bind(i32::try_from(descriptor.intrinsic_width).map_err(|_| {
            StoreError::InvalidRecord(
                "workspace flat-question image width exceeds PostgreSQL range".to_string(),
            )
        })?)
        .bind(i32::try_from(descriptor.intrinsic_height).map_err(|_| {
            StoreError::InvalidRecord(
                "workspace flat-question image height exceeds PostgreSQL range".to_string(),
            )
        })?)
        .bind(&descriptor.object.media_type)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if inserted.rows_affected() == 1 {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(descriptor);
        }

        // The conflict may be either this logical identity or the global
        // object-id uniqueness guard.  RLS makes foreign rows invisible, and
        // both divergent cases intentionally collapse to Conflict.
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM workspace_flat_question_asset \
             WHERE tenant_id = $1 AND workspace_id = $2 AND asset_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(descriptor.workspace.as_uuid())
        .bind(descriptor.asset.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let existing = row
            .as_ref()
            .map(decode_workspace_flat_question_asset_row)
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        match existing {
            Some(existing) if existing == descriptor => Ok(existing),
            Some(_) | None => Err(StoreError::Conflict),
        }
    }
}

fn decode_workspace_flat_question_asset_row(
    row: &sqlx::postgres::PgRow,
) -> Result<WorkspaceFlatQuestionAsset, StoreError> {
    let descriptor: WorkspaceFlatQuestionAsset = decode_payload_row(row)?;
    descriptor.validate()?;
    Ok(descriptor)
}
