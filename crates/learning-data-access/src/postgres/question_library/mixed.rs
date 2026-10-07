//! Mixed Library row decoding and authorized object-kind resolution.

use std::num::NonZeroU32;

use question_model::{
    LibraryObjectId, LibraryObjectKind, QuestionPoolLibrarySummary, QuestionPoolMetadata, Timestamp,
};
use sqlx::{Row, postgres::PgRow};

use super::{PostgresQuestionLibraryStore, decode_entry, invalid, map_sqlx_error};
use crate::{LibrarySearchEntry, SessionTokenHash, StoreError};

impl PostgresQuestionLibraryStore {
    pub async fn load_library_object_kind(
        &self,
        session: SessionTokenHash,
        public_id: &LibraryObjectId,
    ) -> Result<LibraryObjectKind, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let row = sqlx::query("SELECT row_kind FROM ple_api.load_library_object_kind($1)")
            .bind(public_id.as_str())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let kind = match row
            .try_get::<String, _>("row_kind")
            .map_err(map_sqlx_error)?
            .as_str()
        {
            "question" => LibraryObjectKind::Question,
            "pool" => LibraryObjectKind::QuestionPool,
            _ => return Err(invalid("Library object kind")),
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(kind)
    }
}

pub(super) fn decode_search_entry(row: &PgRow) -> Result<Option<LibrarySearchEntry>, StoreError> {
    let Some(kind) = row
        .try_get::<Option<String>, _>("row_kind")
        .map_err(map_sqlx_error)?
    else {
        return Ok(None);
    };
    let owner_account_id = row
        .try_get::<String, _>("owner_account_id")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| invalid("Library owner"))?;
    match kind.as_str() {
        "question" => Ok(Some(LibrarySearchEntry::Question {
            entry: Box::new(decode_entry(row)?),
            owner_account_id,
        })),
        "pool" => {
            let metadata = QuestionPoolMetadata {
                title: row.try_get("question_title").map_err(map_sqlx_error)?,
                description: row
                    .try_get("question_description")
                    .map_err(map_sqlx_error)?,
                discipline_uuid: row
                    .try_get("content_discipline_id")
                    .map_err(map_sqlx_error)?,
                discipline_name: row.try_get("discipline_name").map_err(map_sqlx_error)?,
                discipline_is_retired: row
                    .try_get("discipline_is_retired")
                    .map_err(map_sqlx_error)?,
                subject_uuid: row.try_get("content_subject_id").map_err(map_sqlx_error)?,
                topic_uuid: row.try_get("content_topic_id").map_err(map_sqlx_error)?,
                subtopic_uuid: row.try_get("content_subtopic_id").map_err(map_sqlx_error)?,
                tags: row.try_get("tags").map_err(map_sqlx_error)?,
                bloom_cognitive_process: row
                    .try_get::<Option<String>, _>("bloom_cognitive_process")
                    .map_err(map_sqlx_error)?
                    .map(|value| value.parse::<question_model::BloomCognitiveProcess>())
                    .transpose()
                    .map_err(|_| invalid("Bloom Cognitive Process"))?,
                bloom_knowledge_dimension: row
                    .try_get::<Option<String>, _>("bloom_knowledge_dimension")
                    .map_err(map_sqlx_error)?
                    .map(|value| value.parse::<question_model::BloomKnowledgeDimension>())
                    .transpose()
                    .map_err(|_| invalid("Bloom Knowledge Dimension"))?,
            };
            let summary = QuestionPoolLibrarySummary {
                metadata,
                question_pool_id: row
                    .try_get::<String, _>("public_id")
                    .map_err(map_sqlx_error)?
                    .parse()
                    .map_err(|_| invalid("Library Pool ID"))?,
                owner_account_id,
                question_type: decode_enum(row, "question_type")?,
                backend: decode_enum(row, "backend")?,
                license: decode_enum(row, "question_license")?,
                question_pool_edit_number: u64::try_from(
                    row.try_get::<i64, _>("question_pool_edit_number")
                        .map_err(map_sqlx_error)?,
                )
                .ok()
                .and_then(|value| question_model::QuestionPoolEditNumber::new(value).ok())
                .ok_or_else(|| invalid("Library Pool edit number"))?,
                question_pool_metadata_edit_number: u64::try_from(
                    row.try_get::<i64, _>("question_pool_metadata_edit_number")
                        .map_err(map_sqlx_error)?,
                )
                .ok()
                .and_then(|value| question_model::QuestionPoolMetadataEditNumber::new(value).ok())
                .ok_or_else(|| invalid("Library Pool metadata edit number"))?,
                member_count: u32::try_from(
                    row.try_get::<i64, _>("question_pool_member_count")
                        .map_err(map_sqlx_error)?,
                )
                .ok()
                .and_then(NonZeroU32::new)
                .ok_or_else(|| invalid("Library Pool member count"))?,
                bloom: super::super::question_pool_library::decode_bloom(row)?,
            };
            Ok(Some(LibrarySearchEntry::Pool {
                summary: Box::new(summary),
                created_at: Timestamp::from_unix_millis(
                    row.try_get("published_at_millis").map_err(map_sqlx_error)?,
                ),
            }))
        }
        _ => Err(invalid("Library search row kind")),
    }
}

fn decode_enum<T: serde::de::DeserializeOwned>(row: &PgRow, column: &str) -> Result<T, StoreError> {
    serde_json::from_value(serde_json::Value::String(
        row.try_get(column).map_err(map_sqlx_error)?,
    ))
    .map_err(|_| invalid("Library search metadata"))
}
