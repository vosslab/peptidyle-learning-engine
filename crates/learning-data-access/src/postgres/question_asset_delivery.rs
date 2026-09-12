//! PostgreSQL adapter for the public Question Asset redirect boundary.

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::{ObjectId, QuestionAssetId, QuestionRevisionReference};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{QuestionAssetDeliveryStore, ReadyQuestionAssetDelivery, SessionTokenHash, StoreError};

/// Binds the attested API pool to the one opaque published-asset resolver.
#[derive(Clone)]
pub struct PostgresQuestionAssetDeliveryStore {
    pool: Pool,
}

impl PostgresQuestionAssetDeliveryStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl QuestionAssetDeliveryStore for PostgresQuestionAssetDeliveryStore {
    async fn resolve_ready_question_asset_delivery(
        &self,
        token: SessionTokenHash,
        question_revision: QuestionRevisionReference,
        asset_id: QuestionAssetId,
    ) -> Result<ReadyQuestionAssetDelivery, StoreError> {
        let mut transaction = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT public_object_id, rendition_checksum \
             FROM ple_api.resolve_ready_question_asset($1, $2, $3)",
        )
        .bind(question_revision.question_id.as_compact_str())
        .bind(
            i32::try_from(question_revision.revision_number.get()).map_err(|_| {
                StoreError::InvalidRecord("Question Revision number is invalid".to_string())
            })?,
        )
        .bind(asset_id.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let checksum: [u8; 32] = row
            .try_get::<Vec<u8>, _>("rendition_checksum")
            .map_err(map_sqlx_error)?
            .try_into()
            .map_err(|_| {
                StoreError::InvalidRecord("Question Asset checksum is invalid".to_string())
            })?;
        let value = ReadyQuestionAssetDelivery {
            question_revision,
            asset_id,
            public_object_id: ObjectId::from_uuid(
                row.try_get("public_object_id").map_err(map_sqlx_error)?,
            ),
            rendition_checksum: Sha256Checksum::from_bytes(checksum),
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(value)
    }
}
