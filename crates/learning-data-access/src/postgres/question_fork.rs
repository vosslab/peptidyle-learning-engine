//! PostgreSQL adapter for the server-derived Published Question fork command.

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{
    DraftQuestionReference, ObjectId, QuestionAssetId, QuestionRevisionReference, Timestamp,
    WorkspaceId,
};
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    ForkPublishedQuestionError, ForkPublishedQuestionInput, ForkedPublishedQuestionDraft,
    PublishedQuestionForkAsset, QuestionForkStore, SessionTokenHash, StoreError,
};

/// PostgreSQL Store for source resolution and one atomic Draft fork.
#[derive(Clone)]
pub struct PostgresQuestionForkStore {
    pool: Pool,
}

impl PostgresQuestionForkStore {
    /// Binds the attested application pool to the narrow fork procedures.
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
impl QuestionForkStore for PostgresQuestionForkStore {
    async fn load_published_question_fork_asset(
        &self,
        session_token_hash: SessionTokenHash,
        question_revision: &QuestionRevisionReference,
    ) -> Result<Option<PublishedQuestionForkAsset>, StoreError> {
        let revision_number =
            i32::try_from(question_revision.revision_number.get()).map_err(|_| {
                StoreError::InvalidRecord(
                    "Question Fork Revision Number exceeds PostgreSQL integer".to_owned(),
                )
            })?;
        let mut transaction = self.begin(session_token_hash).await?;
        let rows = sqlx::query("SELECT * FROM ple_api.load_question_fork_asset($1, $2)")
            .bind(question_revision.question_id.as_str())
            .bind(revision_number)
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if rows.len() > 1 {
            return Err(StoreError::InvalidRecord(
                "Native HOTSPOT source Revision has more than one raster".to_owned(),
            ));
        }
        let asset = rows
            .first()
            .map(|row| decode_fork_asset(row, question_revision))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(asset)
    }

    async fn fork_published_question_to_draft(
        &self,
        session_token_hash: SessionTokenHash,
        input: ForkPublishedQuestionInput,
    ) -> Result<ForkedPublishedQuestionDraft, ForkPublishedQuestionError> {
        input
            .validate()
            .map_err(ForkPublishedQuestionError::Store)?;
        let source_revision_number =
            i32::try_from(input.source_question_revision.revision_number.get()).map_err(|_| {
                ForkPublishedQuestionError::Store(StoreError::InvalidRecord(
                    "Question Fork Revision Number exceeds PostgreSQL integer".to_owned(),
                ))
            })?;
        let target = &input.target_source_record;
        let target_address = serde_json::to_value(&target.address).map_err(|_| {
            ForkPublishedQuestionError::Store(StoreError::InvalidRecord(
                "Question Fork target address cannot be encoded".to_owned(),
            ))
        })?;
        let target_size = i64::try_from(target.size_bytes).map_err(|_| {
            ForkPublishedQuestionError::Store(StoreError::InvalidRecord(
                "Question Fork target size exceeds PostgreSQL bigint".to_owned(),
            ))
        })?;
        let hotspot_asset = encode_hotspot_asset(input.hotspot_asset.as_ref())
            .map_err(ForkPublishedQuestionError::Store)?;
        let mut transaction = self
            .begin(session_token_hash)
            .await
            .map_err(ForkPublishedQuestionError::Store)?;
        let row = sqlx::query(
            "SELECT * FROM ple_api.fork_published_question_to_draft(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12\
             )",
        )
        .bind(input.workspace.as_uuid())
        .bind(input.proposed_draft_question_id)
        .bind(input.source_question_revision.question_id.as_str())
        .bind(source_revision_number)
        .bind(input.idempotency_key)
        .bind(target.id.as_uuid())
        .bind(target_address)
        .bind(target.sha256.as_bytes().to_vec())
        .bind(target_size)
        .bind(&target.media_type)
        .bind(target.created_at.as_unix_millis())
        .bind(hotspot_asset)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_fork_error)?;
        let reference_number: i64 = row
            .try_get("reference_number")
            .map_err(map_sqlx_error)
            .map_err(ForkPublishedQuestionError::Store)?;
        let draft_question = u64::try_from(reference_number)
            .ok()
            .and_then(DraftQuestionReference::new)
            .ok_or_else(|| {
                ForkPublishedQuestionError::Store(StoreError::InvalidRecord(
                    "Question Fork returned an invalid Draft Question Reference".to_owned(),
                ))
            })?;
        let workspace = WorkspaceId::from_uuid(
            row.try_get("workspace_id")
                .map_err(map_sqlx_error)
                .map_err(ForkPublishedQuestionError::Store)?,
        );
        let created_new: bool = row
            .try_get("created_new")
            .map_err(map_sqlx_error)
            .map_err(ForkPublishedQuestionError::Store)?;
        transaction
            .commit()
            .await
            .map_err(map_sqlx_error)
            .map_err(ForkPublishedQuestionError::Store)?;
        Ok(ForkedPublishedQuestionDraft {
            draft_question,
            workspace,
            created_new,
        })
    }
}

fn decode_fork_asset(
    row: &sqlx::postgres::PgRow,
    revision: &QuestionRevisionReference,
) -> Result<PublishedQuestionForkAsset, StoreError> {
    let object_id = ObjectId::from_uuid(row.try_get("object_id").map_err(map_sqlx_error)?);
    let asset_id = QuestionAssetId::from_uuid(row.try_get("asset_id").map_err(map_sqlx_error)?);
    let Json(address): Json<ObjectAddress> =
        row.try_get("object_address").map_err(map_sqlx_error)?;
    let checksum: Vec<u8> = row.try_get("sha256").map_err(map_sqlx_error)?;
    let checksum: [u8; 32] = checksum.try_into().map_err(|_| {
        StoreError::InvalidRecord("Question Fork asset checksum has invalid width".to_owned())
    })?;
    let size_bytes: i64 = row.try_get("size_bytes").map_err(map_sqlx_error)?;
    let size_bytes = u64::try_from(size_bytes).map_err(|_| {
        StoreError::InvalidRecord("Question Fork asset size is negative".to_owned())
    })?;
    let width: i32 = row.try_get("intrinsic_width").map_err(map_sqlx_error)?;
    let height: i32 = row.try_get("intrinsic_height").map_err(map_sqlx_error)?;
    let intrinsic_width = u32::try_from(width).map_err(|_| {
        StoreError::InvalidRecord("Question Fork asset width is invalid".to_owned())
    })?;
    let intrinsic_height = u32::try_from(height).map_err(|_| {
        StoreError::InvalidRecord("Question Fork asset height is invalid".to_owned())
    })?;
    let created_at_millis: i64 = row.try_get("created_at_millis").map_err(map_sqlx_error)?;
    let source_record = ObjectRecord {
        id: object_id,
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::QuestionAsset,
        address,
        sha256: Sha256Checksum::from_bytes(checksum),
        size_bytes,
        media_type: row.try_get("media_type").map_err(map_sqlx_error)?,
        question_revision: Some(revision.clone()),
        created_at: Timestamp::from_unix_millis(created_at_millis),
    };
    let expected_address = ObjectAddress::RestrictedQuestionAsset {
        question_revision: revision.clone(),
        asset: asset_id,
        object: object_id,
    };
    if source_record.address != expected_address {
        return Err(StoreError::InvalidRecord(
            "Question Fork asset is not owned by the exact source Revision".to_owned(),
        ));
    }
    Ok(PublishedQuestionForkAsset {
        asset_id,
        source_record,
        intrinsic_width,
        intrinsic_height,
    })
}

fn encode_hotspot_asset(
    asset: Option<&crate::ForkPublishedQuestionAssetInput>,
) -> Result<Option<serde_json::Value>, StoreError> {
    asset
        .map(|asset| {
            let record = &asset.target_record;
            let address = serde_json::to_value(&record.address).map_err(|_| {
                StoreError::InvalidRecord("Question Fork asset address cannot be encoded".into())
            })?;
            Ok(serde_json::json!({
                "assetId": asset.asset_id,
                "objectId": record.id,
                "objectAddress": address,
                "checksum": record.sha256.to_string(),
                "byteLength": record.size_bytes,
                "mediaType": record.media_type,
                "createdAtMillis": record.created_at.as_unix_millis(),
                "intrinsicWidth": asset.intrinsic_width,
                "intrinsicHeight": asset.intrinsic_height,
            }))
        })
        .transpose()
}

fn map_fork_error(error: sqlx::Error) -> ForkPublishedQuestionError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("QF002")
    {
        return ForkPublishedQuestionError::Store(StoreError::NotFound);
    }
    ForkPublishedQuestionError::Store(map_sqlx_error(error))
}
