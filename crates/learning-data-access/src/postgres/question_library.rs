//! PostgreSQL implementation of the Instructor Question Library read boundary.

use async_trait::async_trait;
use question_model::{
    ObjectId, QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionBackend,
    QuestionId, QuestionRevisionAvailability, QuestionRevisionNumber, QuestionRevisionReference,
    SourceObjectChecksum, SourceObjectReference, Timestamp,
};
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{PublishedQuestionLibraryEntry, QuestionLibraryStore, SessionTokenHash, StoreError};

/// PostgreSQL Store for the session-authorized Instructor Question Library.
#[derive(Clone)]
pub struct PostgresQuestionLibraryStore {
    pool: Pool,
}

impl PostgresQuestionLibraryStore {
    /// Binds the attested API pool to Question Library reads.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin_authenticated_application_transaction(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token_hash.to_string())
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
impl QuestionLibraryStore for PostgresQuestionLibraryStore {
    async fn list_published_question_library_entries(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<PublishedQuestionLibraryEntry>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let rows = sqlx::query("SELECT * FROM ple_api.list_question_library_entries()")
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let entries = rows
            .iter()
            .map(decode_entry)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(entries)
    }

    async fn load_published_question_library_entry(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query(
            "SELECT * FROM ple_api.list_question_library_entries() WHERE question_id = $1",
        )
        .bind(question_id.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let entry = row
            .as_ref()
            .map(decode_entry)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(entry)
    }
}

fn decode_entry(row: &sqlx::postgres::PgRow) -> Result<PublishedQuestionLibraryEntry, StoreError> {
    let question_id: String = row.try_get("question_id").map_err(map_sqlx_error)?;
    let question_id = question_id
        .parse::<QuestionId>()
        .map_err(|_| invalid("Question ID"))?;
    let revision_number: i32 = row.try_get("revision_number").map_err(map_sqlx_error)?;
    let revision_number = u32::try_from(revision_number)
        .ok()
        .and_then(|value| QuestionRevisionNumber::new(value).ok())
        .ok_or_else(|| invalid("Question Revision Number"))?;
    let backend = match row
        .try_get::<String, _>("backend")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "ple" => QuestionBackend::Ple,
        "webwork" => QuestionBackend::Webwork,
        "imathas" => QuestionBackend::Imathas,
        _ => return Err(invalid("Question Backend")),
    };
    let author_names: Vec<String> = row.try_get("author_names").map_err(map_sqlx_error)?;
    let authors = author_names
        .into_iter()
        .map(|display_name| {
            QuestionAuthorDisplayName::new(display_name)
                .map(|display_name| QuestionAuthor { display_name })
                .map_err(|_| invalid("Question Authorship"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let question_license: String = row.try_get("question_license").map_err(map_sqlx_error)?;
    let question_license = serde_json::from_value(serde_json::Value::String(question_license))
        .map_err(|_| invalid("Question License"))?;
    let availability: String = row.try_get("availability").map_err(map_sqlx_error)?;
    let availability = match availability.as_str() {
        "available" => QuestionRevisionAvailability::Available,
        "archived" => QuestionRevisionAvailability::Archived {
            reason: row
                .try_get::<Option<String>, _>("availability_reason")
                .map_err(map_sqlx_error)?
                .ok_or_else(|| invalid("Question Revision Availability reason"))?,
        },
        _ => return Err(invalid("Question Revision Availability")),
    };
    let source_object_id =
        ObjectId::from_uuid(row.try_get("source_object_id").map_err(map_sqlx_error)?);
    let source_object_checksum: String = row
        .try_get("source_object_checksum")
        .map_err(map_sqlx_error)?;
    let source_object_checksum = SourceObjectChecksum::parse(source_object_checksum)
        .map_err(|_| invalid("Question Source Checksum"))?;
    let published_at_millis: i64 = row.try_get("published_at_millis").map_err(map_sqlx_error)?;
    Ok(PublishedQuestionLibraryEntry {
        question_revision: QuestionRevisionReference {
            question_id,
            revision_number,
        },
        backend,
        published_at: Timestamp::from_unix_millis(published_at_millis),
        question_title: row.try_get("question_title").map_err(map_sqlx_error)?,
        question_description: row
            .try_get("question_description")
            .map_err(map_sqlx_error)?,
        authorship: QuestionAuthorship::new(authors).map_err(|_| invalid("Question Authorship"))?,
        authored_by_current_account: row
            .try_get("authored_by_current_account")
            .map_err(map_sqlx_error)?,
        question_license,
        availability,
        source_object_reference: SourceObjectReference {
            object: source_object_id,
        },
        source_object_checksum,
        source_media_type: row.try_get("source_media_type").map_err(map_sqlx_error)?,
    })
}

fn invalid(field: &str) -> StoreError {
    StoreError::InvalidRecord(format!("stored {field} is invalid"))
}
