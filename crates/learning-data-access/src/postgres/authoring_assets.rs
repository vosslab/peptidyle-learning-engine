//! Parameterized narrow private Draft raster operations.

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{ObjectId, QuestionAssetId, Timestamp};
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AuthoringAssetsStore, OwnedDraftQuestionAsset, RegisterDraftQuestionAssetInput,
    SessionTokenHash, StoreError,
};

#[derive(Clone)]
pub struct PostgresAuthoringAssetsStore {
    pool: Pool,
}

impl PostgresAuthoringAssetsStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        if sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .is_none()
        {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        Ok(tx)
    }
}

#[async_trait]
impl AuthoringAssetsStore for PostgresAuthoringAssetsStore {
    async fn register_draft_question_asset(
        &self,
        session_hash: SessionTokenHash,
        input: RegisterDraftQuestionAssetInput,
    ) -> Result<OwnedDraftQuestionAsset, StoreError> {
        input.validate()?;
        let address = serde_json::to_value(&input.source_record.address).map_err(|_| {
            StoreError::InvalidRecord("Draft image address cannot be encoded".into())
        })?;
        let mut tx = self.begin(session_hash).await?;
        // ASVS 1.2.4, 8.2.2, 15.4.2: parameters and transactional owner/CAS recheck.
        sqlx::query(
            "SELECT ple_api.register_draft_question_asset($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(input.draft_question_uuid.as_uuid())
        .bind(input.expected_edit_number.as_postgres_bigint())
        .bind(input.asset_id.as_uuid())
        .bind(input.source_record.id.as_uuid())
        .bind(address)
        .bind(input.source_record.sha256.as_bytes().to_vec())
        .bind(input.source_record.size_bytes as i64)
        .bind(&input.source_record.media_type)
        .bind(input.source_record.created_at.as_unix_millis())
        .bind(input.intrinsic_width as i32)
        .bind(input.intrinsic_height as i32)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(OwnedDraftQuestionAsset {
            asset_id: input.asset_id,
            source_record: input.source_record,
            intrinsic_width: input.intrinsic_width,
            intrinsic_height: input.intrinsic_height,
        })
    }

    async fn load_draft_question_asset(
        &self,
        session_hash: SessionTokenHash,
        draft_question_uuid: crate::DraftQuestionUuid,
        asset_id: QuestionAssetId,
    ) -> Result<OwnedDraftQuestionAsset, StoreError> {
        let mut tx = self.begin(session_hash).await?;
        let row = sqlx::query("SELECT * FROM ple_api.load_draft_question_asset($1,$2)")
            .bind(draft_question_uuid.as_uuid())
            .bind(asset_id.as_uuid())
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let Json(address): Json<ObjectAddress> =
            row.try_get("object_address").map_err(map_sqlx_error)?;
        let sha: Vec<u8> = row.try_get("sha256").map_err(map_sqlx_error)?;
        let invalid =
            || StoreError::InvalidRecord("Database returned invalid Draft raster facts".into());
        let record = ObjectRecord {
            id: ObjectId::from_uuid(row.try_get("object_id").map_err(map_sqlx_error)?),
            address,
            storage_area: ObjectStorageArea::PrivateContent,
            data_class: ObjectDataClass::AuthoringContent,
            sha256: Sha256Checksum::from_bytes(sha.try_into().map_err(|_| invalid())?),
            size_bytes: u64::try_from(
                row.try_get::<i64, _>("size_bytes")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| invalid())?,
            media_type: row.try_get("media_type").map_err(map_sqlx_error)?,
            question_revision_tuple: None,
            created_at: Timestamp::from_unix_millis(
                row.try_get("created_at_millis").map_err(map_sqlx_error)?,
            ),
        };
        let width = u32::try_from(
            row.try_get::<i32, _>("intrinsic_width")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid())?;
        let height = u32::try_from(
            row.try_get::<i32, _>("intrinsic_height")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid())?;
        crate::authoring_assets::validate_raster_facts(&record, width, height)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(OwnedDraftQuestionAsset {
            asset_id,
            source_record: record,
            intrinsic_width: width,
            intrinsic_height: height,
        })
    }
}
