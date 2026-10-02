//! PostgreSQL implementation of the Question Pool search-metadata command.

use async_trait::async_trait;
use question_model::{QuestionPoolId, QuestionPoolMetadataEditNumber};
use serde_json::{Map, Value, json};
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    BulkQuestionPoolSearchMetadataInput, BulkQuestionPoolSearchMetadataResult,
    BulkQuestionPoolSearchMetadataStore, SessionTokenHash, StoreError,
};

/// PostgreSQL store for one all-or-none Question Pool search-metadata command.
#[derive(Clone)]
pub struct PostgresBulkQuestionPoolSearchMetadataStore {
    pool: Pool,
}

impl PostgresBulkQuestionPoolSearchMetadataStore {
    /// Binds the attested API pool to the closed Pool search-metadata procedure.
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
impl BulkQuestionPoolSearchMetadataStore for PostgresBulkQuestionPoolSearchMetadataStore {
    async fn bulk_replace_question_pool_search_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        input: BulkQuestionPoolSearchMetadataInput,
    ) -> Result<Vec<BulkQuestionPoolSearchMetadataResult>, StoreError> {
        input.validate()?;
        let selection = Value::Array(
            input
                .selection
                .iter()
                .map(|selected| {
                    json!({
                        "questionPoolId": selected.question_pool_id.as_str(),
                        "questionPoolMetadataEditNumber": selected.question_pool_metadata_edit_number.get(),
                    })
                })
                .collect(),
        );
        let mut patch = Map::new();
        if let Some(tags) = input.patch.tags {
            patch.insert("tags".to_owned(), json!(tags));
        }
        if let Some(value) = input.patch.topic_uuid {
            patch.insert("topicUuid".to_owned(), json!(value));
        }
        if let Some(value) = input.patch.subtopic_uuid {
            patch.insert("subtopicUuid".to_owned(), json!(value));
        }
        let mut tx = self.begin(session_token_hash).await?;
        let rows = sqlx::query(
            "SELECT question_pool_id, question_pool_metadata_edit_number \
             FROM ple_api.bulk_replace_question_pool_search_metadata($1, $2)",
        )
        .bind(Json(selection))
        .bind(Json(Value::Object(patch)))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let result = rows
            .iter()
            .map(|row| {
                let question_pool_id = row
                    .try_get::<String, _>("question_pool_id")
                    .map_err(map_sqlx_error)?
                    .parse::<QuestionPoolId>()
                    .map_err(|_| {
                        invalid("Question Pool search metadata returned an invalid Pool ID")
                    })?;
                let question_pool_metadata_edit_number = row
                    .try_get::<i64, _>("question_pool_metadata_edit_number")
                    .map_err(map_sqlx_error)
                    .and_then(|value| {
                        u64::try_from(value).map_err(|_| {
                            invalid("Question Pool search metadata returned an invalid Edit Number")
                        })
                    })
                    .and_then(|value| {
                        QuestionPoolMetadataEditNumber::new(value).map_err(|_| {
                            invalid("Question Pool search metadata returned an invalid metadata Edit Number")
                        })
                    })?;
                Ok(BulkQuestionPoolSearchMetadataResult {
                    question_pool_id,
                    question_pool_metadata_edit_number,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

fn invalid(message: &str) -> StoreError {
    StoreError::InvalidRecord(message.to_owned())
}
