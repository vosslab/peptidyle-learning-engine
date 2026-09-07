//! PostgreSQL adapter for the dedicated immutable public-asset publisher.

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::{
    ObjectId, QuestionAssetId, QuestionRevisionNumber, QuestionRevisionReference,
};
use sqlx::{Row, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error};
use crate::{ClaimedQuestionAssetPublication, PublicAssetPublicationStore, StoreError};

/// Binds the publisher-only attested pool to the registry's closed procedures.
#[derive(Clone)]
pub struct PostgresPublicAssetPublicationStore {
    pool: Pool,
}

impl PostgresPublicAssetPublicationStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(&self) -> Result<Transaction<'_, sqlx::Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_public_asset_publisher")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl PublicAssetPublicationStore for PostgresPublicAssetPublicationStore {
    async fn claim_question_asset_publication(
        &self,
        lease_token: Uuid,
    ) -> Result<Option<ClaimedQuestionAssetPublication>, StoreError> {
        let mut transaction = self.begin().await?;
        let row = sqlx::query(
            "SELECT * FROM ple_private.claim_question_asset_publication_job(\
             $1, pg_catalog.clock_timestamp() + interval '300 seconds')",
        )
        .bind(lease_token)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.map(decode_claim).transpose()
    }

    async fn activate_question_asset_publication(
        &self,
        job_id: Uuid,
        lease_token: Uuid,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin().await?;
        sqlx::query("SELECT ple_private.activate_question_asset_publication($1, $2)")
            .bind(job_id)
            .bind(lease_token)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

fn decode_claim(row: sqlx::postgres::PgRow) -> Result<ClaimedQuestionAssetPublication, StoreError> {
    let revision_number = row
        .try_get::<i32, _>("revision_number")
        .map_err(map_sqlx_error)?;
    let revision_number = u32::try_from(revision_number)
        .ok()
        .and_then(|value| QuestionRevisionNumber::new(value).ok())
        .ok_or_else(|| {
            StoreError::InvalidRecord("Question Revision number is invalid".to_string())
        })?;
    let checksum = |column| -> Result<Sha256Checksum, StoreError> {
        let bytes = row.try_get::<Vec<u8>, _>(column).map_err(map_sqlx_error)?;
        let bytes: [u8; 32] = bytes.try_into().map_err(|_| {
            StoreError::InvalidRecord("Question Asset Publication checksum is invalid".to_string())
        })?;
        Ok(Sha256Checksum::from_bytes(bytes))
    };
    let byte_length = row
        .try_get::<i64, _>("public_byte_length")
        .map_err(map_sqlx_error)?;
    Ok(ClaimedQuestionAssetPublication {
        job_id: row.try_get("job_id").map_err(map_sqlx_error)?,
        question_revision: QuestionRevisionReference {
            question_id: row
                .try_get::<String, _>("question_id")
                .map_err(map_sqlx_error)?
                .parse()
                .map_err(|_| StoreError::InvalidRecord("Question ID is invalid".to_string()))?,
            revision_number,
        },
        asset_id: QuestionAssetId::from_uuid(row.try_get("asset_id").map_err(map_sqlx_error)?),
        source_object_id: ObjectId::from_uuid(
            row.try_get("source_object_id").map_err(map_sqlx_error)?,
        ),
        source_checksum: checksum("source_object_checksum")?,
        public_object_id: ObjectId::from_uuid(
            row.try_get("public_object_id").map_err(map_sqlx_error)?,
        ),
        public_checksum: checksum("public_object_checksum")?,
        public_byte_length: u64::try_from(byte_length).map_err(|_| {
            StoreError::InvalidRecord(
                "Question Asset Publication byte length is invalid".to_string(),
            )
        })?,
        verified_media_type: row.try_get("verified_media_type").map_err(map_sqlx_error)?,
        intrinsic_width: u32::try_from(
            row.try_get::<i32, _>("intrinsic_width")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::InvalidRecord("Question Asset width is invalid".to_string()))?,
        intrinsic_height: u32::try_from(
            row.try_get::<i32, _>("intrinsic_height")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::InvalidRecord("Question Asset height is invalid".to_string()))?,
    })
}
