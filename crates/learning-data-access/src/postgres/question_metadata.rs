//! PostgreSQL persistence for one exact Published Question metadata replacement.

use async_trait::async_trait;
use question_model::{
    PublishedQuestionRevisionTuple, QuestionRevisionNumber, SaveQuestionMetadataRequest,
    SavedQuestionMetadata, validate_question_description, validate_question_title,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{QuestionMetadataStore, SessionTokenHash, StoreError};

/// PostgreSQL Store for ordinary owner/Sysadmin metadata corrections.
#[derive(Clone)]
pub struct PostgresQuestionMetadataStore {
    pool: Pool,
}

impl PostgresQuestionMetadataStore {
    /// Binds the attested API pool to the trusted Question metadata procedure.
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

fn validate_request(request: &SaveQuestionMetadataRequest) -> Result<(), StoreError> {
    if request.expected_metadata_edit_number == 0
        || request.expected_metadata_edit_number > i64::MAX as u64
    {
        return Err(invalid("Question metadata Edit Number is invalid"));
    }
    validate_question_title(&request.metadata.question_title)
        .map_err(|_| invalid("Question Title is invalid"))?;
    validate_question_description(&request.metadata.question_description)
        .map_err(|_| invalid("Question Description is invalid"))?;
    let mut unique_tags = std::collections::BTreeSet::new();
    if request.metadata.tags.iter().any(|tag| {
        let value = tag.as_str();
        value.is_empty()
            || value.trim() != value
            || value.chars().count() > 120
            || value.chars().any(char::is_control)
            || !unique_tags.insert(value)
    }) {
        return Err(invalid("Question Tags are invalid"));
    }
    Ok(())
}

#[async_trait]
impl QuestionMetadataStore for PostgresQuestionMetadataStore {
    async fn save_question_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        request: SaveQuestionMetadataRequest,
    ) -> Result<SavedQuestionMetadata, StoreError> {
        validate_request(&request)?;
        let mut tx = self.begin(session_token_hash).await?;
        let revision_number = i32::try_from(
            request
                .published_question_revision_tuple
                .revision_number
                .get(),
        )
        .map_err(|_| invalid("Question Revision number is invalid"))?;
        let expected_metadata_edit_number = i64::try_from(request.expected_metadata_edit_number)
            .map_err(|_| invalid("Question metadata Edit Number is invalid"))?;
        let tags = request
            .metadata
            .tags
            .iter()
            .map(|tag| tag.as_str().to_owned())
            .collect::<Vec<_>>();
        let question_type = request.metadata.question_type.as_str();
        let row = sqlx::query(
            "SELECT published_question_id, revision_number, metadata_edit_number \
             FROM ple_api.replace_published_question_metadata( \
               $1, $2, $3, $4, $5, NULL, $6, $7, $8, $9, $10, \
               $11, $12, $13)",
        )
        .bind(
            request
                .published_question_revision_tuple
                .published_question_id
                .as_str(),
        )
        .bind(revision_number)
        .bind(expected_metadata_edit_number)
        .bind(&request.metadata.question_title)
        .bind(&request.metadata.question_description)
        .bind(tags)
        .bind(request.metadata.discipline_uuid)
        .bind(request.metadata.subject_uuid)
        .bind(request.metadata.topic_uuid)
        .bind(request.metadata.subtopic_uuid)
        .bind(question_type)
        .bind(
            request
                .metadata
                .bloom_cognitive_process
                .map(|process| process.as_str()),
        )
        .bind(
            request
                .metadata
                .bloom_knowledge_dimension
                .map(|dimension| dimension.as_str()),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_metadata_sqlx_error)?;
        let Some(row) = row else {
            return Err(StoreError::Forbidden);
        };
        let question_id = row
            .try_get::<String, _>("published_question_id")
            .map_err(map_sqlx_error)?
            .parse()
            .map_err(|_| invalid("Metadata save returned an invalid Question ID"))?;
        let revision_number = u32::try_from(
            row.try_get::<i32, _>("revision_number")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Metadata save returned an invalid Revision number"))
        .and_then(|number| {
            QuestionRevisionNumber::new(number)
                .map_err(|_| invalid("Metadata save returned an invalid Revision number"))
        })?;
        let metadata_edit_number = u64::try_from(
            row.try_get::<i64, _>("metadata_edit_number")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Metadata save returned an invalid Edit Number"))?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(SavedQuestionMetadata {
            published_question_revision_tuple: PublishedQuestionRevisionTuple {
                published_question_id: question_id,
                revision_number,
            },
            metadata_edit_number,
        })
    }
}

fn invalid(message: &str) -> StoreError {
    StoreError::InvalidRecord(message.to_owned())
}

fn map_metadata_sqlx_error(error: sqlx::Error) -> StoreError {
    if error
        .as_database_error()
        .and_then(|database| database.code())
        .as_deref()
        == Some("40001")
    {
        // This procedure uses serialization SQLSTATE only for stale Revision or metadata CAS.
        StoreError::Conflict
    } else {
        map_sqlx_error(error)
    }
}
