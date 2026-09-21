//! PostgreSQL implementation of the Instructor Question Library read boundary.

use async_trait::async_trait;
use question_model::{
    BloomClassificationEditNumber, BloomClassificationView, BloomCognitiveProcess,
    BloomKnowledgeDimension, MAX_BULK_QUESTION_METADATA_ITEMS, ObjectId, PublishedQuestionId,
    PublishedQuestionRevisionTuple, PublishedQuestionSharedMetadata, QuestionAuthor,
    QuestionAuthorDisplayName, QuestionAuthorship, QuestionAvailability,
    QuestionAvailabilityEditNumber, QuestionBackend, QuestionRevisionNumber,
    QuestionRevisionUsageStatistics, QuestionType, QuestionUsageTotals, SourceObjectChecksum, Tag,
    Timestamp,
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
            "SELECT q.*, s.name AS subject_name, t.name AS topic_name, d.name AS discipline_name, d.is_retired AS discipline_is_retired, st.name AS subtopic_name \
             FROM ple_api.list_question_library_entries() q \
             JOIN LATERAL ple_api.list_content_disciplines_including_retired() d ON d.content_discipline_id = q.content_discipline_id \
             LEFT JOIN LATERAL ple_api.list_content_subtopics(q.content_topic_id) st ON st.content_subtopic_id = q.content_subtopic_id \
             JOIN LATERAL ple_api.list_content_subjects(q.content_discipline_id) s ON s.content_subject_id = q.content_subject_id \
             LEFT JOIN LATERAL ple_api.list_content_topics(q.content_subject_id) t ON t.content_topic_id = q.content_topic_id \
             WHERE q.availability = 'available'",
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
        question_id: &PublishedQuestionId,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query(
            "SELECT q.*, s.name AS subject_name, t.name AS topic_name, d.name AS discipline_name, d.is_retired AS discipline_is_retired, st.name AS subtopic_name \
             FROM ple_api.list_question_library_entries() q \
             JOIN LATERAL ple_api.list_content_disciplines_including_retired() d ON d.content_discipline_id = q.content_discipline_id \
             LEFT JOIN LATERAL ple_api.list_content_subtopics(q.content_topic_id) st ON st.content_subtopic_id = q.content_subtopic_id \
             JOIN LATERAL ple_api.list_content_subjects(q.content_discipline_id) s ON s.content_subject_id = q.content_subject_id \
             LEFT JOIN LATERAL ple_api.list_content_topics(q.content_subject_id) t ON t.content_topic_id = q.content_topic_id \
             WHERE q.published_question_id = $1 AND q.availability = 'available'",
        )
        .bind(question_id.as_str())
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
        published_question_revision_tuple: &PublishedQuestionRevisionTuple,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // This projection resolves an existing immutable pin. Unlike ordinary
        // library discovery, its result intentionally remains available after
        // the Question lineage is archived.
        let row = sqlx::query("SELECT q.*, s.name AS subject_name, t.name AS topic_name, d.name AS discipline_name, d.is_retired AS discipline_is_retired, st.name AS subtopic_name \
             FROM ple_api.load_question_library_revision($1, $2) q \
             JOIN LATERAL ple_api.list_content_disciplines_including_retired() d ON d.content_discipline_id = q.content_discipline_id \
             LEFT JOIN LATERAL ple_api.list_content_subtopics(q.content_topic_id) st ON st.content_subtopic_id = q.content_subtopic_id \
             JOIN LATERAL ple_api.list_content_subjects(q.content_discipline_id) s ON s.content_subject_id = q.content_subject_id \
             LEFT JOIN LATERAL ple_api.list_content_topics(q.content_subject_id) t ON t.content_topic_id = q.content_topic_id")
            .bind(published_question_revision_tuple.published_question_id.as_str())
            .bind(
                i32::try_from(published_question_revision_tuple.revision_number.get())
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
        if entry.published_question_revision_tuple != *published_question_revision_tuple {
            return Err(invalid("Question Library exact revision"));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(entry)
    }

    async fn correct_question_revision_bloom(
        &self,
        session_token_hash: SessionTokenHash,
        published_question_revision_tuple: &PublishedQuestionRevisionTuple,
        expected_edit_number: BloomClassificationEditNumber,
        cognitive_process: BloomCognitiveProcess,
        knowledge_dimension: BloomKnowledgeDimension,
    ) -> Result<BloomClassificationView, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.4/15.4.2: bind the entire typed command to the database-owned
        // row-locking CAS function; stale commands are returned and never retried.
        let row = sqlx::query(
            "SELECT cognitive_process AS bloom_cognitive_process, \
                    knowledge_dimension AS bloom_knowledge_dimension, \
                    classification_edit_number AS bloom_classification_edit_number \
             FROM ple_api.correct_question_revision_bloom($1, $2, $3, $4, $5)",
        )
        .bind(
            published_question_revision_tuple
                .published_question_id
                .as_str(),
        )
        .bind(
            i32::try_from(published_question_revision_tuple.revision_number.get())
                .map_err(|_| invalid("Question Revision Number"))?,
        )
        .bind(expected_edit_number.value() as i64)
        .bind(cognitive_process.as_str())
        .bind(knowledge_dimension.as_str())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let bloom = decode_bloom(&row)?.ok_or_else(|| invalid("Bloom Classification"))?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(bloom)
    }

    async fn load_current_published_question_shared_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        question_ids: &[PublishedQuestionId],
    ) -> Result<Vec<PublishedQuestionSharedMetadata>, StoreError> {
        if question_ids.is_empty() || question_ids.len() > MAX_BULK_QUESTION_METADATA_ITEMS {
            return Err(invalid("Published Question shared metadata selection"));
        }
        let mut canonical_ids = question_ids
            .iter()
            .map(|question_id| question_id.as_str().to_owned())
            .collect::<Vec<_>>();
        canonical_ids.sort();
        if canonical_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid("Published Question shared metadata selection"));
        }

        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.4: bind the complete ID array; no identifier or value is
        // interpolated into the SQL statement.
        let rows = sqlx::query(
            "SELECT published_question_id, metadata_edit_number, tags, \
                    content_discipline_id, content_subject_id, \
                    content_topic_id, content_subtopic_id \
             FROM ple_api.load_current_published_question_shared_metadata($1)",
        )
        .bind(&canonical_ids)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let items = rows
            .iter()
            .map(decode_shared_metadata)
            .collect::<Result<Vec<_>, _>>()?;
        let returned_ids = items
            .iter()
            .map(|item| item.question_id.as_str())
            .collect::<Vec<_>>();
        if returned_ids != canonical_ids.iter().map(String::as_str).collect::<Vec<_>>() {
            return Err(invalid("Published Question shared metadata result"));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(items)
    }

    async fn archive_published_question(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
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
        question_id: &PublishedQuestionId,
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
    /// All-Revision usage rollup for one page of Question Library IDs.
    pub async fn load_question_usage_statistics(
        &self,
        session_token_hash: SessionTokenHash,
        question_ids: &[PublishedQuestionId],
    ) -> Result<Vec<(PublishedQuestionId, QuestionUsageTotals)>, StoreError> {
        if question_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let ids = question_ids
            .iter()
            .map(PublishedQuestionId::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let rows = sqlx::query(
            "SELECT published_question_id, issued_count, blank_count, answered_count, \
                    correct_count, partial_count, incorrect_count, credit_sum, credit_sum_sq \
             FROM ple_api.read_question_library_usage_statistics($1)",
        )
        .bind(&ids)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            let question_id = row
                .try_get::<String, _>("published_question_id")
                .map_err(map_sqlx_error)?
                .parse::<PublishedQuestionId>()
                .map_err(|_| invalid("Question ID"))?;
            items.push((question_id, decode_usage_totals(row)?));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(items)
    }

    /// Per-Revision usage rows for one Question detail page.
    pub async fn load_question_revision_usage_statistics(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
    ) -> Result<Vec<QuestionRevisionUsageStatistics>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let rows = sqlx::query(
            "SELECT revision_number, issued_count, blank_count, answered_count, \
                    correct_count, partial_count, incorrect_count, credit_sum, credit_sum_sq \
             FROM ple_api.read_question_library_revision_usage_statistics($1)",
        )
        .bind(question_id.as_str())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            let revision_number: i32 = row.try_get("revision_number").map_err(map_sqlx_error)?;
            let revision_number = u32::try_from(revision_number)
                .ok()
                .and_then(|value| QuestionRevisionNumber::new(value).ok())
                .ok_or_else(|| invalid("Question Revision Number"))?;
            items.push(decode_usage_totals(row)?.into_revision_statistics(revision_number));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(items)
    }

    async fn set_published_question_availability(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
        expected_edit_number: QuestionAvailabilityEditNumber,
        target_availability: &str,
        archive_confirmation_title: Option<&str>,
    ) -> Result<PublishedQuestionAvailability, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row =
            sqlx::query("SELECT * FROM ple_api.set_question_availability($1, $2, $3, $4, $5)")
                .bind(question_id.as_str())
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
    let question_id: String = row
        .try_get("published_question_id")
        .map_err(map_sqlx_error)?;
    let question_id = question_id
        .parse::<PublishedQuestionId>()
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
    let question_format = serde_json::from_value(serde_json::Value::String(
        row.try_get("question_format").map_err(map_sqlx_error)?,
    ))
    .map_err(|_| invalid("Question Format"))?;
    let question_type: QuestionType = serde_json::from_value(serde_json::Value::String(
        row.try_get("question_type").map_err(map_sqlx_error)?,
    ))
    .map_err(|_| invalid("Question Type"))?;
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
    let source_object_id = ObjectId::from_uuid(
        row.try_get("source_object_record_id")
            .map_err(map_sqlx_error)?,
    );
    let source_object_checksum: String = row
        .try_get("source_object_checksum")
        .map_err(map_sqlx_error)?;
    let source_object_checksum = SourceObjectChecksum::parse(source_object_checksum)
        .map_err(|_| invalid("Question Source Checksum"))?;
    let published_at_millis: i64 = row.try_get("published_at_millis").map_err(map_sqlx_error)?;
    let shared_metadata = decode_shared_metadata(row)?;
    Ok(PublishedQuestionLibraryEntry {
        published_question_revision_tuple: PublishedQuestionRevisionTuple {
            published_question_id: question_id,
            revision_number,
        },
        backend,
        question_format,
        question_type,
        published_at: Timestamp::from_unix_millis(published_at_millis),
        bloom: decode_bloom(row)?,
        question_title: row.try_get("question_title").map_err(map_sqlx_error)?,
        question_description: row
            .try_get("question_description")
            .map_err(map_sqlx_error)?,
        shared_metadata,
        discipline_name: row.try_get("discipline_name").map_err(map_sqlx_error)?,
        discipline_is_retired: row
            .try_get("discipline_is_retired")
            .map_err(map_sqlx_error)?,
        subtopic_name: row.try_get("subtopic_name").map_err(map_sqlx_error)?,
        subject_name: row.try_get("subject_name").map_err(map_sqlx_error)?,
        topic_name: row.try_get("topic_name").map_err(map_sqlx_error)?,
        used_in_current_account_courses: row
            .try_get("used_in_current_account_courses")
            .map_err(map_sqlx_error)?,
        authorship: QuestionAuthorship::new(authors).map_err(|_| invalid("Question Authorship"))?,
        authored_by_current_account: row
            .try_get("authored_by_current_account")
            .map_err(map_sqlx_error)?,
        viewer_may_archive: row.try_get("viewer_may_archive").map_err(map_sqlx_error)?,
        question_license,
        availability,
        availability_edit_number,
        source_object_id,
        source_object_checksum,
        source_media_type: row.try_get("source_media_type").map_err(map_sqlx_error)?,
        webwork_pg_path: row.try_get("webwork_pg_path").map_err(map_sqlx_error)?,
    })
}

fn decode_bloom(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<BloomClassificationView>, StoreError> {
    let cognitive_process = row
        .try_get::<Option<String>, _>("bloom_cognitive_process")
        .map_err(map_sqlx_error)?;
    let knowledge_dimension = row
        .try_get::<Option<String>, _>("bloom_knowledge_dimension")
        .map_err(map_sqlx_error)?;
    let classification_edit_number = row
        .try_get::<Option<i64>, _>("bloom_classification_edit_number")
        .map_err(map_sqlx_error)?;
    let (cognitive_process, knowledge_dimension, classification_edit_number) = match (
        cognitive_process,
        knowledge_dimension,
        classification_edit_number,
    ) {
        (None, None, None) => return Ok(None),
        (Some(cognitive_process), Some(knowledge_dimension), Some(classification_edit_number)) => (
            cognitive_process,
            knowledge_dimension,
            classification_edit_number,
        ),
        _ => return Err(invalid("Bloom Classification")),
    };
    Ok(Some(BloomClassificationView {
        cognitive_process: cognitive_process
            .parse::<BloomCognitiveProcess>()
            .map_err(|_| invalid("Bloom Cognitive Process"))?,
        knowledge_dimension: knowledge_dimension
            .parse::<BloomKnowledgeDimension>()
            .map_err(|_| invalid("Bloom Knowledge Dimension"))?,
        classification_edit_number: u64::try_from(classification_edit_number)
            .ok()
            .and_then(BloomClassificationEditNumber::new)
            .ok_or_else(|| invalid("Bloom Classification Edit Number"))?,
    }))
}

fn decode_shared_metadata(
    row: &sqlx::postgres::PgRow,
) -> Result<PublishedQuestionSharedMetadata, StoreError> {
    let question_id = row
        .try_get::<String, _>("published_question_id")
        .map_err(map_sqlx_error)?
        .parse::<PublishedQuestionId>()
        .map_err(|_| invalid("Question ID"))?;
    let metadata_edit_number = row
        .try_get::<i64, _>("metadata_edit_number")
        .map_err(map_sqlx_error)
        .and_then(|value| {
            u64::try_from(value)
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| invalid("Question Metadata Edit Number"))
        })?;
    let tags = row
        .try_get::<Vec<String>, _>("tags")
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Tag::new)
        .collect();
    Ok(PublishedQuestionSharedMetadata {
        question_id,
        metadata_edit_number,
        tags,
        discipline_uuid: row
            .try_get("content_discipline_id")
            .map_err(map_sqlx_error)?,
        subject_uuid: row.try_get("content_subject_id").map_err(map_sqlx_error)?,
        topic_uuid: row.try_get("content_topic_id").map_err(map_sqlx_error)?,
        subtopic_uuid: row.try_get("content_subtopic_id").map_err(map_sqlx_error)?,
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

pub(crate) fn decode_usage_totals(
    row: &sqlx::postgres::PgRow,
) -> Result<QuestionUsageTotals, StoreError> {
    Ok(QuestionUsageTotals {
        issued_count: count_u64(row, "issued_count")?,
        blank_count: count_u64(row, "blank_count")?,
        answered_count: count_u64(row, "answered_count")?,
        correct_count: count_u64(row, "correct_count")?,
        partial_count: count_u64(row, "partial_count")?,
        incorrect_count: count_u64(row, "incorrect_count")?,
        credit_sum: numeric_f64(row, "credit_sum")?,
        credit_sum_sq: numeric_f64(row, "credit_sum_sq")?,
    })
}

fn count_u64(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64, StoreError> {
    u64::try_from(row.try_get::<i64, _>(column).map_err(map_sqlx_error)?)
        .map_err(|_| invalid("Question usage count"))
}

fn numeric_f64(row: &sqlx::postgres::PgRow, column: &str) -> Result<f64, StoreError> {
    row.try_get::<bigdecimal::BigDecimal, _>(column)
        .map_err(map_sqlx_error)?
        .to_string()
        .parse::<f64>()
        .map_err(|_| invalid("Question usage credit sum"))
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
