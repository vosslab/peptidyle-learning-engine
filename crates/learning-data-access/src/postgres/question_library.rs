//! PostgreSQL implementation of the Instructor Question Library read boundary.

use async_trait::async_trait;
use question_model::{
    ObjectId, QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionAvailability,
    QuestionAvailabilityEditNumber, QuestionBackend, QuestionId, QuestionRevisionNumber,
    QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference, Timestamp,
};
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    PublishedQuestionAvailability, PublishedQuestionLibraryEntry, QuestionLibraryStore,
    SessionTokenHash, StoreError,
};

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
        let rows = sqlx::query(
            "SELECT * FROM ple_api.list_question_library_entries() WHERE availability = 'available'",
        )
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
            "SELECT * FROM ple_api.list_question_library_entries() \
             WHERE question_id = $1 AND availability = 'available'",
        )
        .bind(question_id.as_compact_str())
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

    async fn load_published_question_revision_library_entry(
        &self,
        session_token_hash: SessionTokenHash,
        question_revision: &QuestionRevisionReference,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // This projection resolves an existing immutable pin. Unlike ordinary
        // library discovery, its result intentionally remains available after
        // the Question lineage is archived.
        let row = sqlx::query("SELECT * FROM ple_api.load_question_library_revision($1, $2)")
            .bind(question_revision.question_id.as_compact_str())
            .bind(
                i32::try_from(question_revision.revision_number.get())
                    .map_err(|_| invalid("Question Revision Number"))?,
            )
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let entry = row
            .as_ref()
            .map(decode_entry)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        if entry.question_revision != *question_revision {
            return Err(invalid("Question Library exact revision"));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(entry)
    }

    async fn archive_published_question(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
        expected_edit_number: QuestionAvailabilityEditNumber,
        confirmation_title: &str,
    ) -> Result<PublishedQuestionAvailability, StoreError> {
        self.set_published_question_availability(
            session_token_hash,
            question_id,
            expected_edit_number,
            "archived",
            Some(confirmation_title),
        )
        .await
    }

    async fn restore_published_question(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
        expected_edit_number: QuestionAvailabilityEditNumber,
    ) -> Result<PublishedQuestionAvailability, StoreError> {
        self.set_published_question_availability(
            session_token_hash,
            question_id,
            expected_edit_number,
            "available",
            None,
        )
        .await
    }
}

impl PostgresQuestionLibraryStore {
    async fn set_published_question_availability(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
        expected_edit_number: QuestionAvailabilityEditNumber,
        target_availability: &str,
        archive_confirmation_title: Option<&str>,
    ) -> Result<PublishedQuestionAvailability, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row =
            sqlx::query("SELECT * FROM ple_api.set_question_availability($1, $2, $3, $4, $5)")
                .bind(question_id.as_compact_str())
                .bind(expected_edit_number.value() as i64)
                .bind(target_availability)
                .bind(archive_confirmation_title)
                .bind(crate::random_uuid::random_uuid_v4(|_| {
                    StoreError::Unavailable(
                        "Question availability event ID randomness unavailable".into(),
                    )
                })?)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)?;
        let result = decode_availability(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(PublishedQuestionAvailability {
            availability: result.0,
            edit_number: result.1,
        })
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
    let (availability, availability_edit_number) = decode_availability(row)?;
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
        availability_edit_number,
        source_object_reference: SourceObjectReference {
            object: source_object_id,
        },
        source_object_checksum,
        source_media_type: row.try_get("source_media_type").map_err(map_sqlx_error)?,
    })
}

fn decode_availability(
    row: &sqlx::postgres::PgRow,
) -> Result<(QuestionAvailability, QuestionAvailabilityEditNumber), StoreError> {
    let availability = availability_from_wire(
        &row.try_get::<String, _>("availability")
            .map_err(map_sqlx_error)?,
    )?;
    let edit_number = row
        .try_get::<i64, _>("availability_edit_number")
        .map_err(map_sqlx_error)
        .and_then(|value| {
            u64::try_from(value)
                .ok()
                .and_then(QuestionAvailabilityEditNumber::new)
                .ok_or_else(|| invalid("Question Availability Edit Number"))
        })?;
    Ok((availability, edit_number))
}

fn availability_from_wire(value: &str) -> Result<QuestionAvailability, StoreError> {
    match value {
        "available" => Ok(QuestionAvailability::Available),
        "archived" => Ok(QuestionAvailability::Archived),
        _ => Err(invalid("Question Availability")),
    }
}

fn invalid(field: &str) -> StoreError {
    StoreError::InvalidRecord(format!("stored {field} is invalid"))
}

#[cfg(test)]
mod tests {
    use question_model::QuestionAvailability;

    use super::availability_from_wire;

    #[test]
    fn stable_lineage_availability_uses_the_closed_wire_vocabulary() {
        assert_eq!(
            availability_from_wire("available"),
            Ok(QuestionAvailability::Available)
        );
        assert_eq!(
            availability_from_wire("archived"),
            Ok(QuestionAvailability::Archived)
        );
        assert!(availability_from_wire("retired").is_err());
    }
}
