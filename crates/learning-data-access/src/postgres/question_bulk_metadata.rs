//! PostgreSQL implementation of the closed Published Question metadata command.

use async_trait::async_trait;
use question_model::QuestionId;
use serde_json::{Map, Value, json};
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    BulkPublishedQuestionMetadataError, BulkPublishedQuestionMetadataInput,
    BulkPublishedQuestionMetadataResult, BulkPublishedQuestionMetadataStore, SessionTokenHash,
    StoreError,
};

/// PostgreSQL Store for one all-or-none shared metadata command.
#[derive(Clone)]
pub struct PostgresBulkPublishedQuestionMetadataStore {
    pool: Pool,
}

impl PostgresBulkPublishedQuestionMetadataStore {
    /// Binds the attested API pool to the closed metadata procedure.
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
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
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
impl BulkPublishedQuestionMetadataStore for PostgresBulkPublishedQuestionMetadataStore {
    async fn bulk_replace_published_question_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        input: BulkPublishedQuestionMetadataInput,
    ) -> Result<Vec<BulkPublishedQuestionMetadataResult>, BulkPublishedQuestionMetadataError> {
        input
            .validate()
            .map_err(BulkPublishedQuestionMetadataError::Store)?;
        let selection = Value::Array(
            input
                .selection
                .iter()
                .map(|selected| {
                    json!({
                        "questionId": selected.question_id.as_compact_str(),
                        "metadataEditNumber": selected.metadata_edit_number,
                    })
                })
                .collect(),
        );
        let mut patch = Map::new();
        if let Some(tags) = input.patch.tags {
            patch.insert("tags".to_owned(), json!(tags));
        }
        if let Some(subject) = input.patch.subject {
            patch.insert("subject".to_owned(), json!(subject));
        }
        if let Some(topic) = input.patch.topic {
            patch.insert("topic".to_owned(), json!(topic));
        }
        let mut tx = self
            .begin(session_token_hash)
            .await
            .map_err(BulkPublishedQuestionMetadataError::Store)?;
        let rows = sqlx::query(
            "SELECT question_id, metadata_edit_number \
             FROM ple_api.bulk_replace_published_question_metadata($1, $2, $3)",
        )
        .bind(Json(selection))
        .bind(Json(Value::Object(patch)))
        .bind(input.idempotency_key)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_bulk_metadata_error)?;
        let result = rows
            .iter()
            .map(|row| {
                let question_id = row
                    .try_get::<String, _>("question_id")
                    .map_err(map_sqlx_error)?
                    .parse::<QuestionId>()
                    .map_err(|_| {
                        invalid("Bulk Published Question metadata returned an invalid Question ID")
                    })?;
                let metadata_edit_number = row
                    .try_get::<i64, _>("metadata_edit_number")
                    .map_err(map_sqlx_error)
                    .and_then(|value| {
                        u64::try_from(value).map_err(|_| {
                            invalid(
                                "Bulk Published Question metadata returned an invalid Edit Number",
                            )
                        })
                    })?;
                Ok(BulkPublishedQuestionMetadataResult {
                    question_id,
                    metadata_edit_number,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()
            .map_err(BulkPublishedQuestionMetadataError::Store)?;
        tx.commit()
            .await
            .map_err(map_sqlx_error)
            .map_err(BulkPublishedQuestionMetadataError::Store)?;
        Ok(result)
    }
}

fn map_bulk_metadata_error(error: sqlx::Error) -> BulkPublishedQuestionMetadataError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("23514")
    {
        return BulkPublishedQuestionMetadataError::IdempotencyConflict;
    }
    BulkPublishedQuestionMetadataError::Store(map_sqlx_error(error))
}

fn invalid(message: &str) -> StoreError {
    StoreError::InvalidRecord(message.to_owned())
}
