//! PostgreSQL resolution for one authorized private Question Thumbnail.

use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{ObjectId, PublishedQuestionRevisionTuple, Timestamp};
use sqlx::Row;

use super::{PostgresQuestionLibraryStore, invalid, map_sqlx_error};
use crate::{PublishedQuestionThumbnail, SessionTokenHash, StoreError};

impl PostgresQuestionLibraryStore {
    /// Resolves one exact Revision thumbnail through the existing Library
    /// visibility function without granting the API direct private-table read.
    pub(super) async fn resolve_question_thumbnail(
        &self,
        session_token_hash: SessionTokenHash,
        published_question_revision_tuple: &PublishedQuestionRevisionTuple,
    ) -> Result<PublishedQuestionThumbnail, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query("SELECT * FROM ple_api.resolve_question_thumbnail($1, $2)")
            .bind(
                published_question_revision_tuple
                    .published_question_id
                    .as_str(),
            )
            .bind(
                i32::try_from(published_question_revision_tuple.revision_number.get())
                    .map_err(|_| invalid("Question Revision Number"))?,
            )
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;

        let object_id =
            ObjectId::from_uuid(row.try_get("object_record_id").map_err(map_sqlx_error)?);
        let address: ObjectAddress =
            serde_json::from_value(row.try_get("object_address").map_err(map_sqlx_error)?)
                .map_err(|_| invalid("Question Thumbnail Object Address"))?;
        let expected_address = ObjectAddress::QuestionThumbnail {
            published_question_revision_tuple: published_question_revision_tuple.clone(),
            object_id,
        };
        let checksum: [u8; 32] = row
            .try_get::<Vec<u8>, _>("sha256")
            .map_err(map_sqlx_error)?
            .try_into()
            .map_err(|_| invalid("Question Thumbnail checksum"))?;
        let size_bytes = u64::try_from(
            row.try_get::<i64, _>("size_bytes")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Question Thumbnail byte length"))?;
        let intrinsic_width = u32::try_from(
            row.try_get::<i32, _>("intrinsic_width")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Question Thumbnail width"))?;
        let intrinsic_height = u32::try_from(
            row.try_get::<i32, _>("intrinsic_height")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Question Thumbnail height"))?;
        let storage_area: String = row.try_get("object_storage_area").map_err(map_sqlx_error)?;
        let data_class: String = row.try_get("object_data_class").map_err(map_sqlx_error)?;
        let media_type: String = row.try_get("media_type").map_err(map_sqlx_error)?;
        if address != expected_address
            || storage_area != "private-content"
            || data_class != "question-thumbnail"
            || media_type != "image/webp"
            || intrinsic_width != 256
            || intrinsic_height != 256
        {
            return Err(invalid("Question Thumbnail metadata"));
        }
        let record = ObjectRecord {
            id: object_id,
            storage_area: ObjectStorageArea::PrivateContent,
            data_class: ObjectDataClass::QuestionThumbnail,
            address,
            sha256: Sha256Checksum::from_bytes(checksum),
            size_bytes,
            media_type,
            published_question_revision_tuple: Some(published_question_revision_tuple.clone()),
            created_at: Timestamp::from_unix_millis(
                row.try_get("created_at_millis").map_err(map_sqlx_error)?,
            ),
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(PublishedQuestionThumbnail {
            object_record: record,
            intrinsic_width,
            intrinsic_height,
        })
    }
}
