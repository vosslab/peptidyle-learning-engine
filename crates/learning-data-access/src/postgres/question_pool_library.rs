//! PostgreSQL reads for published Question Pools and Assessment-owned forks.

use std::num::NonZeroU32;

use async_trait::async_trait;
use question_model::{
    AccountId, AssessmentEntryId, AssessmentId, BloomClassificationEditNumber,
    BloomClassificationView, BloomCognitiveProcess, BloomKnowledgeDimension, CourseInstanceId,
    PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionBackend, QuestionLicense,
    QuestionPoolEditNumber, QuestionPoolId, QuestionPoolMetadata, QuestionRevisionNumber,
    QuestionType, QuestionUsageTotals,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AssessmentQuestionPoolForkRecord, PublishedQuestionPool, QuestionPoolLibraryStore,
    SessionTokenHash, StoreError,
};

#[derive(Clone)]
pub struct PostgresQuestionPoolLibraryStore {
    pool: Pool,
}

impl PostgresQuestionPoolLibraryStore {
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
impl QuestionPoolLibraryStore for PostgresQuestionPoolLibraryStore {
    async fn load_current_published_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionPoolId,
    ) -> Result<PublishedQuestionPool, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows = sqlx::query("SELECT * FROM ple_api.read_current_published_question_pool($1)")
            .bind(question_pool_id.as_str())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let result = decode_pool_rows(&rows, question_pool_id)?.ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn correct_question_pool_bloom(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionPoolId,
        expected_edit_number: BloomClassificationEditNumber,
        cognitive_process: BloomCognitiveProcess,
        knowledge_dimension: BloomKnowledgeDimension,
    ) -> Result<BloomClassificationView, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        // ASVS 1.2.4/15.4.2: the existing PostgreSQL function owns exact-target
        // authorization, row locking, stale rejection, and no-op token behavior.
        let row = sqlx::query(
            "SELECT cognitive_process AS bloom_cognitive_process, \
                    knowledge_dimension AS bloom_knowledge_dimension, \
                    classification_edit_number AS bloom_classification_edit_number \
             FROM ple_api.correct_question_pool_bloom($1, $2, $3, $4)",
        )
        .bind(question_pool_id.as_str())
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

    async fn load_assessment_question_pool_fork(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
        assessment_entry: AssessmentEntryId,
    ) -> Result<AssessmentQuestionPoolForkRecord, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows =
            sqlx::query("SELECT * FROM ple_api.read_assessment_question_pool_fork($1, $2, $3)")
                .bind(course_instance_id.as_string())
                .bind(assessment_id.as_string())
                .bind(assessment_entry.as_uuid())
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let first = rows.first().ok_or(StoreError::NotFound)?;
        let stored_entry = AssessmentEntryId::from_uuid(
            first
                .try_get("assessment_entry_id")
                .map_err(map_sqlx_error)?,
        );
        if stored_entry != assessment_entry {
            return Err(invalid("Assessment Pool fork Entry"));
        }
        let pool_id = decode_pool_id(first, "question_pool_id")?;
        let edit_number = decode_pool_edit_number(first)?;
        let selection_count = u32::try_from(
            first
                .try_get::<i32, _>("selection_count")
                .map_err(map_sqlx_error)?,
        )
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| invalid("Assessment Pool selection count"))?;
        let members = decode_member_rows(&rows, &pool_id, edit_number)?;
        let result = AssessmentQuestionPoolForkRecord {
            metadata: decode_metadata(first)?,
            assessment_entry_id: stored_entry,
            question_pool_id: pool_id,
            owner_account_id: decode_owner_account_id(first)?,
            question_type: decode_pool_question_type(first)?,
            backend: decode_pool_backend(first)?,
            license: decode_pool_license(first)?,
            question_pool_edit_number: edit_number,
            selection_count,
            bloom: decode_bloom(first)?,
            members,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

impl PostgresQuestionPoolLibraryStore {
    /// Pool issued_count plus current-member all-Revision outcome rollup.
    pub async fn load_question_pool_usage_statistics(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionPoolId,
    ) -> Result<(u64, u64, QuestionUsageTotals), StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let row = sqlx::query(
            "SELECT pool_issued_count, pool_issued_contributor_floor, issued_count, blank_count, answered_count, \
                    correct_count, partial_count, incorrect_count, issued_contributor_floor, \
                    blank_contributor_floor, answered_contributor_floor, correct_contributor_floor, \
                    partial_contributor_floor, incorrect_contributor_floor, credit_sum, credit_sum_sq \
             FROM ple_api.read_question_pool_library_usage_statistics($1)",
        )
        .bind(question_pool_id.as_str())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let pool_issued_count = u64::try_from(
            row.try_get::<i64, _>("pool_issued_count")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Question Pool issued count"))?;
        let pool_issued_contributor_floor = u64::try_from(
            row.try_get::<i64, _>("pool_issued_contributor_floor")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Question Pool contributor floor"))?;
        let totals = super::question_library::decode_usage_totals(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok((pool_issued_count, pool_issued_contributor_floor, totals))
    }
}

fn decode_pool_rows(
    rows: &[sqlx::postgres::PgRow],
    expected_id: &QuestionPoolId,
) -> Result<Option<PublishedQuestionPool>, StoreError> {
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let pool_id = decode_pool_id(first, "question_pool_id")?;
    let edit_number = decode_pool_edit_number(first)?;
    if expected_id != &pool_id {
        return Err(invalid("Question Pool identity"));
    }
    Ok(Some(PublishedQuestionPool {
        metadata: decode_metadata(first)?,
        owner_account_id: decode_owner_account_id(first)?,
        question_type: decode_pool_question_type(first)?,
        backend: decode_pool_backend(first)?,
        license: decode_pool_license(first)?,
        bloom: decode_bloom(first)?,
        members: decode_member_rows(rows, &pool_id, edit_number)?,
        question_pool_id: pool_id,
        question_pool_edit_number: edit_number,
    }))
}

fn decode_member_rows(
    rows: &[sqlx::postgres::PgRow],
    pool_id: &QuestionPoolId,
    edit_number: QuestionPoolEditNumber,
) -> Result<Vec<PublishedQuestionRevisionTuple>, StoreError> {
    let metadata = rows.first().map(decode_metadata).transpose()?;
    let bloom = rows.first().map(decode_bloom).transpose()?.flatten();
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            if metadata.as_ref() != Some(&decode_metadata(row)?) || bloom != decode_bloom(row)? {
                return Err(invalid("Question Pool lineage metadata"));
            }
            if decode_pool_id(row, "question_pool_id")? != *pool_id
                || decode_pool_edit_number(row)? != edit_number
                || row
                    .try_get::<i32, _>("member_position")
                    .map_err(map_sqlx_error)?
                    != i32::try_from(index + 1).map_err(|_| invalid("Pool member position"))?
            {
                return Err(invalid("Question Pool member order"));
            }
            let question_id = decode_published_question_id(row, "published_question_id")?;
            let published_question_revision_tuple = u32::try_from(
                row.try_get::<i32, _>("question_revision_number")
                    .map_err(map_sqlx_error)?,
            )
            .ok()
            .and_then(|value| QuestionRevisionNumber::new(value).ok())
            .ok_or_else(|| invalid("Question Revision"))?;
            Ok(PublishedQuestionRevisionTuple {
                published_question_id: question_id,
                revision_number: published_question_revision_tuple,
            })
        })
        .collect()
}

pub(super) fn decode_bloom(
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

fn decode_metadata(row: &sqlx::postgres::PgRow) -> Result<QuestionPoolMetadata, StoreError> {
    let metadata = QuestionPoolMetadata {
        title: row.try_get("title").map_err(map_sqlx_error)?,
        description: row.try_get("description").map_err(map_sqlx_error)?,
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
    };
    if !(1..=question_model::MAX_QUESTION_TITLE_UNICODE_SCALARS)
        .contains(&metadata.title.chars().count())
        || !(1..=question_model::MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS)
            .contains(&metadata.description.chars().count())
        || [&metadata.title, &metadata.description]
            .iter()
            // Match PostgreSQL btrim's ASCII-space canonicalization.
            .any(|text| {
                text.as_str() != text.trim_matches(' ') || text.chars().any(char::is_control)
            })
        || (metadata.subtopic_uuid.is_some() && metadata.topic_uuid.is_none())
        || metadata.tags.iter().any(|tag| {
            tag.is_empty()
                || tag.chars().count() > 120
                || tag != tag.trim_matches(' ')
                || tag.chars().any(char::is_control)
        })
        || metadata
            .tags
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != metadata.tags.len()
    {
        return Err(invalid("Question Pool lineage metadata"));
    }
    Ok(metadata)
}

fn decode_pool_id(row: &sqlx::postgres::PgRow, column: &str) -> Result<QuestionPoolId, StoreError> {
    row.try_get::<String, _>(column)
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| invalid("public Question Pool ID"))
}

fn decode_pool_question_type(row: &sqlx::postgres::PgRow) -> Result<QuestionType, StoreError> {
    match row
        .try_get::<String, _>("question_type")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "multipleChoice" => Ok(QuestionType::MultipleChoice),
        "multipleAnswer" => Ok(QuestionType::MultipleAnswer),
        "fillInBlank" => Ok(QuestionType::FillInBlank),
        "multipleFillInBlank" => Ok(QuestionType::MultipleFillInBlank),
        "numeric" => Ok(QuestionType::Numeric),
        "matching" => Ok(QuestionType::Matching),
        "ordering" => Ok(QuestionType::Ordering),
        "hotspot" => Ok(QuestionType::Hotspot),
        _ => Err(invalid("Question Pool Type")),
    }
}

fn decode_pool_backend(row: &sqlx::postgres::PgRow) -> Result<QuestionBackend, StoreError> {
    match row
        .try_get::<String, _>("backend")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "ple" => Ok(QuestionBackend::Ple),
        "webwork" => Ok(QuestionBackend::Webwork),
        "imathas" => Ok(QuestionBackend::Imathas),
        _ => Err(invalid("Question Pool Backend")),
    }
}

fn decode_pool_license(row: &sqlx::postgres::PgRow) -> Result<QuestionLicense, StoreError> {
    serde_json::from_value(serde_json::Value::String(
        row.try_get("license").map_err(map_sqlx_error)?,
    ))
    .map_err(|_| invalid("Question Pool license"))
}

fn decode_owner_account_id(row: &sqlx::postgres::PgRow) -> Result<AccountId, StoreError> {
    row.try_get::<String, _>("owner_account_id")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| invalid("Question Pool owner Account ID"))
}

fn decode_published_question_id(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<PublishedQuestionId, StoreError> {
    row.try_get::<String, _>(column)
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| invalid("public Published Question ID"))
}

fn decode_pool_edit_number(
    row: &sqlx::postgres::PgRow,
) -> Result<QuestionPoolEditNumber, StoreError> {
    QuestionPoolEditNumber::new(
        u64::try_from(
            row.try_get::<i64, _>("question_pool_edit_number")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Question Pool Edit Number"))?,
    )
    .map_err(|_| invalid("Question Pool Edit Number"))
}

fn invalid(field: &str) -> StoreError {
    StoreError::InvalidRecord(format!("stored {field} is invalid"))
}
