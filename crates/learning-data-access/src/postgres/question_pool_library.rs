//! PostgreSQL reads for published Question Pools and Assessment-owned forks.

use std::num::NonZeroU32;

use async_trait::async_trait;
use question_model::{
    AssessmentEntryId, AssessmentId, BloomClassificationEditNumber, BloomClassificationView,
    BloomCognitiveProcess, BloomKnowledgeDimension, CourseInstanceId, QuestionId,
    QuestionPoolEditNumber, QuestionPoolLibrarySummary, QuestionPoolMetadata,
    QuestionRevisionNumber, QuestionRevisionTuple, QuestionSearchBloomCognitiveProcessFacet,
    QuestionSearchBloomKnowledgeDimensionFacet, QuestionUsageTotals,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AssessmentQuestionPoolForkRecord, Cursor, PageRequest, PublishedQuestionPool,
    QuestionPoolDiscoveryFilter, QuestionPoolDiscoveryPage, QuestionPoolLibraryStore,
    QuestionPoolTextFilter, SessionTokenHash, StoreError,
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
    async fn list_published_question_pools(
        &self,
        session_token_hash: SessionTokenHash,
        page: PageRequest,
        filter: QuestionPoolDiscoveryFilter,
        text: QuestionPoolTextFilter,
    ) -> Result<QuestionPoolDiscoveryPage, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        if !filter.has_valid_structure() {
            return Err(StoreError::InvalidRecord(
                "Pool discovery hierarchy is invalid".into(),
            ));
        }
        // ASVS 1.2.4: identity and closed Bloom predicates remain typed SQL bind parameters.
        let rows = sqlx::query(
            "SELECT * FROM ple_api.list_published_question_pools($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(page.after.as_ref().map(Cursor::as_str))
        .bind(i32::from(page.size.get()))
        .bind(filter.discipline_uuid)
        .bind(filter.subject_uuid)
        .bind(filter.topic_uuid)
        .bind(filter.subtopic_uuid)
        .bind(filter.cross_discipline)
        .bind(serde_json::to_value(&text.terms).map_err(|_| invalid("Pool text terms"))?)
        .bind(text.tags)
        .bind(filter.bloom_cognitive_process.map(BloomCognitiveProcess::as_str))
        .bind(
            filter
                .bloom_knowledge_dimension
                .map(BloomKnowledgeDimension::as_str),
        )
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let first = rows
            .first()
            .ok_or_else(|| invalid("Question Pool discovery aggregates"))?;
        let (cognitive_processes, knowledge_dimensions) = decode_bloom_facets(first)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            if row
                .try_get::<Option<String>, _>("question_pool_id")
                .map_err(map_sqlx_error)?
                .is_some()
            {
                items.push(decode_summary(row)?);
            }
        }
        let has_next = items.len() > usize::from(page.size.get());
        items.truncate(usize::from(page.size.get()));
        let next_cursor = has_next
            .then(|| {
                items
                    .last()
                    .map(|item| item.question_pool_id.to_string())
                    .ok_or_else(|| invalid("Question Pool page"))
            })
            .transpose()?
            .map(Cursor::parse)
            .transpose()
            .map_err(|_| invalid("Question Pool cursor"))?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(QuestionPoolDiscoveryPage {
            items,
            next_cursor,
            cognitive_processes,
            knowledge_dimensions,
        })
    }

    async fn load_current_published_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionId,
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
        question_pool_id: &QuestionId,
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
        course: CourseInstanceId,
        assessment: AssessmentId,
        assessment_entry: AssessmentEntryId,
    ) -> Result<AssessmentQuestionPoolForkRecord, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows =
            sqlx::query("SELECT * FROM ple_api.read_assessment_question_pool_fork($1, $2, $3)")
                .bind(course.as_string())
                .bind(assessment.as_string())
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
        let pool_id = decode_question_id(first, "question_pool_id")?;
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
        question_pool_id: &QuestionId,
    ) -> Result<(u64, QuestionUsageTotals), StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let row = sqlx::query(
            "SELECT pool_issued_count, issued_count, blank_count, answered_count, \
                    correct_count, partial_count, incorrect_count, credit_sum, credit_sum_sq \
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
        let totals = super::question_library::decode_usage_totals(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok((pool_issued_count, totals))
    }
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<QuestionPoolLibrarySummary, StoreError> {
    let question_pool_id = decode_question_id(row, "question_pool_id")?;
    let edit_number = decode_pool_edit_number(row)?;
    let member_count = u32::try_from(
        row.try_get::<i32, _>("member_count")
            .map_err(map_sqlx_error)?,
    )
    .ok()
    .and_then(NonZeroU32::new)
    .ok_or_else(|| invalid("Question Pool member count"))?;
    Ok(QuestionPoolLibrarySummary {
        metadata: decode_metadata(row)?,
        question_pool_id,
        question_pool_edit_number: edit_number,
        member_count,
        bloom: decode_bloom(row)?,
    })
}

fn decode_bloom_facets(
    row: &sqlx::postgres::PgRow,
) -> Result<
    (
        Vec<QuestionSearchBloomCognitiveProcessFacet>,
        Vec<QuestionSearchBloomKnowledgeDimensionFacet>,
    ),
    StoreError,
> {
    let cognitive_counts = row
        .try_get::<Vec<i64>, _>("bloom_cognitive_process_counts")
        .map_err(map_sqlx_error)?;
    let knowledge_counts = row
        .try_get::<Vec<i64>, _>("bloom_knowledge_dimension_counts")
        .map_err(map_sqlx_error)?;
    if cognitive_counts.len() != BloomCognitiveProcess::ALL.len()
        || knowledge_counts.len() != BloomKnowledgeDimension::ALL.len()
    {
        return Err(invalid("Question Pool Bloom counts"));
    }
    let cognitive_processes = BloomCognitiveProcess::ALL
        .into_iter()
        .zip(cognitive_counts)
        .map(|(cognitive_process, count)| {
            Ok(QuestionSearchBloomCognitiveProcessFacet {
                cognitive_process,
                count: u64::try_from(count).map_err(|_| invalid("Question Pool Bloom count"))?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let knowledge_dimensions = BloomKnowledgeDimension::ALL
        .into_iter()
        .zip(knowledge_counts)
        .map(|(knowledge_dimension, count)| {
            Ok(QuestionSearchBloomKnowledgeDimensionFacet {
                knowledge_dimension,
                count: u64::try_from(count).map_err(|_| invalid("Question Pool Bloom count"))?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok((cognitive_processes, knowledge_dimensions))
}

fn decode_pool_rows(
    rows: &[sqlx::postgres::PgRow],
    expected_id: &QuestionId,
) -> Result<Option<PublishedQuestionPool>, StoreError> {
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let pool_id = decode_question_id(first, "question_pool_id")?;
    let edit_number = decode_pool_edit_number(first)?;
    if expected_id != &pool_id {
        return Err(invalid("Question Pool identity"));
    }
    Ok(Some(PublishedQuestionPool {
        metadata: decode_metadata(first)?,
        bloom: decode_bloom(first)?,
        members: decode_member_rows(rows, &pool_id, edit_number)?,
        question_pool_id: pool_id,
        question_pool_edit_number: edit_number,
    }))
}

fn decode_member_rows(
    rows: &[sqlx::postgres::PgRow],
    pool_id: &QuestionId,
    edit_number: QuestionPoolEditNumber,
) -> Result<Vec<QuestionRevisionTuple>, StoreError> {
    let metadata = rows.first().map(decode_metadata).transpose()?;
    let bloom = rows.first().map(decode_bloom).transpose()?.flatten();
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            if metadata.as_ref() != Some(&decode_metadata(row)?) || bloom != decode_bloom(row)? {
                return Err(invalid("Question Pool lineage metadata"));
            }
            if decode_question_id(row, "question_pool_id")? != *pool_id
                || decode_pool_edit_number(row)? != edit_number
                || row
                    .try_get::<i32, _>("member_position")
                    .map_err(map_sqlx_error)?
                    != i32::try_from(index + 1).map_err(|_| invalid("Pool member position"))?
            {
                return Err(invalid("Question Pool member order"));
            }
            let question_id = decode_question_id(row, "published_question_id")?;
            let question_revision = u32::try_from(
                row.try_get::<i32, _>("question_revision_number")
                    .map_err(map_sqlx_error)?,
            )
            .ok()
            .and_then(|value| QuestionRevisionNumber::new(value).ok())
            .ok_or_else(|| invalid("Question Revision"))?;
            Ok(QuestionRevisionTuple {
                question_id,
                revision_number: question_revision,
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

fn decode_question_id(row: &sqlx::postgres::PgRow, column: &str) -> Result<QuestionId, StoreError> {
    row.try_get::<String, _>(column)
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| invalid("public Question ID"))
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
