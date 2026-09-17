//! PostgreSQL reads for published Question Pools and Assessment-owned forks.

use std::num::NonZeroU32;

use async_trait::async_trait;
use question_model::{
    AssessmentEntryId, AssessmentReference, CourseInstanceReference, QuestionId,
    QuestionPoolLibrarySummary, QuestionPoolMetadata, QuestionPoolRevisionNumber,
    QuestionPoolRevisionReference, QuestionRevisionNumber, QuestionRevisionReference,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AssessmentQuestionPoolForkRecord, Cursor, Page, PageRequest, PublishedQuestionPoolRevision,
    QuestionPoolDiscoveryFilter, QuestionPoolLibraryStore, QuestionPoolTextFilter,
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
    async fn list_published_question_pools(
        &self,
        session_token_hash: SessionTokenHash,
        page: PageRequest,
        filter: QuestionPoolDiscoveryFilter,
        text: QuestionPoolTextFilter,
    ) -> Result<Page<QuestionPoolLibrarySummary>, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        if !filter.has_valid_structure() {
            return Err(StoreError::InvalidRecord(
                "Pool discovery hierarchy is invalid".into(),
            ));
        }
        // ASVS 1.2.4: identity predicates remain typed SQL bind parameters.
        let rows = sqlx::query(
            "SELECT * FROM ple_api.list_published_question_pools($1, $2, $3, $4, $5, $6, $7, $8, $9)",
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
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut items = rows
            .iter()
            .map(decode_summary)
            .collect::<Result<Vec<_>, _>>()?;
        let has_next = items.len() > usize::from(page.size.get());
        items.truncate(usize::from(page.size.get()));
        let next_cursor = has_next
            .then(|| {
                items
                    .last()
                    .map(|item| item.question_pool_revision.question_pool_id.to_string())
                    .ok_or_else(|| invalid("Question Pool page"))
            })
            .transpose()?
            .map(Cursor::parse)
            .transpose()
            .map_err(|_| invalid("Question Pool cursor"))?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Page { items, next_cursor })
    }

    async fn load_current_published_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        public_question_pool_id: &QuestionId,
    ) -> Result<PublishedQuestionPoolRevision, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows = sqlx::query("SELECT * FROM ple_api.read_current_published_question_pool($1)")
            .bind(public_question_pool_id.as_str())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let result = decode_revision_rows(&rows, public_question_pool_id, None)?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn load_published_question_pool_revision(
        &self,
        session_token_hash: SessionTokenHash,
        reference: &QuestionPoolRevisionReference,
    ) -> Result<PublishedQuestionPoolRevision, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows =
            sqlx::query("SELECT * FROM ple_api.read_published_question_pool_revision($1, $2)")
                .bind(reference.question_pool_id.as_str())
                .bind(
                    i64::try_from(reference.revision_number.get())
                        .map_err(|_| invalid("Question Pool Revision"))?,
                )
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let result = decode_revision_rows(
            &rows,
            &reference.question_pool_id,
            Some(reference.revision_number),
        )?
        .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn load_assessment_question_pool_fork(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assessment: AssessmentReference,
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
        let pool_id = decode_question_id(first, "public_question_pool_id")?;
        let revision_number = decode_pool_revision(first, "revision_number")?;
        let selection_count = u32::try_from(
            first
                .try_get::<i32, _>("selection_count")
                .map_err(map_sqlx_error)?,
        )
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| invalid("Assessment Pool selection count"))?;
        let members = decode_member_rows(&rows, &pool_id, revision_number)?;
        let result = AssessmentQuestionPoolForkRecord {
            metadata: decode_metadata(first)?,
            assessment_entry_id: stored_entry,
            question_pool_revision: QuestionPoolRevisionReference {
                question_pool_id: pool_id,
                revision_number,
            },
            pool_metadata_etag: first
                .try_get("pool_metadata_etag")
                .map_err(map_sqlx_error)?,
            selection_count,
            members,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<QuestionPoolLibrarySummary, StoreError> {
    let question_pool_id = decode_question_id(row, "public_question_pool_id")?;
    let revision_number = decode_pool_revision(row, "revision_number")?;
    let member_count = u32::try_from(
        row.try_get::<i32, _>("member_count")
            .map_err(map_sqlx_error)?,
    )
    .ok()
    .and_then(NonZeroU32::new)
    .ok_or_else(|| invalid("Question Pool member count"))?;
    Ok(QuestionPoolLibrarySummary {
        metadata: decode_metadata(row)?,
        question_pool_revision: QuestionPoolRevisionReference {
            question_pool_id,
            revision_number,
        },
        member_count,
    })
}

fn decode_revision_rows(
    rows: &[sqlx::postgres::PgRow],
    expected_id: &QuestionId,
    expected_revision: Option<QuestionPoolRevisionNumber>,
) -> Result<Option<PublishedQuestionPoolRevision>, StoreError> {
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let pool_id = decode_question_id(first, "public_question_pool_id")?;
    let revision_number = decode_pool_revision(first, "revision_number")?;
    if expected_id != &pool_id
        || expected_revision.is_some_and(|expected| expected != revision_number)
    {
        return Err(invalid("Question Pool identity"));
    }
    Ok(Some(PublishedQuestionPoolRevision {
        metadata: decode_metadata(first)?,
        members: decode_member_rows(rows, &pool_id, revision_number)?,
        question_pool_revision: QuestionPoolRevisionReference {
            question_pool_id: pool_id,
            revision_number,
        },
    }))
}

fn decode_member_rows(
    rows: &[sqlx::postgres::PgRow],
    pool_id: &QuestionId,
    revision_number: QuestionPoolRevisionNumber,
) -> Result<Vec<QuestionRevisionReference>, StoreError> {
    let metadata = rows.first().map(decode_metadata).transpose()?;
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            if metadata.as_ref() != Some(&decode_metadata(row)?) {
                return Err(invalid("Question Pool lineage metadata"));
            }
            if decode_question_id(row, "public_question_pool_id")? != *pool_id
                || decode_pool_revision(row, "revision_number")? != revision_number
                || row
                    .try_get::<i32, _>("member_position")
                    .map_err(map_sqlx_error)?
                    != i32::try_from(index + 1).map_err(|_| invalid("Pool member position"))?
            {
                return Err(invalid("Question Pool member order"));
            }
            let question_id = decode_question_id(row, "question_id")?;
            let question_revision = u32::try_from(
                row.try_get::<i32, _>("question_revision_number")
                    .map_err(map_sqlx_error)?,
            )
            .ok()
            .and_then(|value| QuestionRevisionNumber::new(value).ok())
            .ok_or_else(|| invalid("Question Revision"))?;
            Ok(QuestionRevisionReference {
                question_id,
                revision_number: question_revision,
            })
        })
        .collect()
}

fn decode_metadata(row: &sqlx::postgres::PgRow) -> Result<QuestionPoolMetadata, StoreError> {
    let metadata = QuestionPoolMetadata {
        title: row.try_get("title").map_err(map_sqlx_error)?,
        description: row.try_get("description").map_err(map_sqlx_error)?,
        discipline_uuid: row.try_get("discipline_uuid").map_err(map_sqlx_error)?,
        discipline_name: row.try_get("discipline_name").map_err(map_sqlx_error)?,
        discipline_is_retired: row
            .try_get("discipline_is_retired")
            .map_err(map_sqlx_error)?,
        subject_uuid: row.try_get("subject_uuid").map_err(map_sqlx_error)?,
        topic_uuid: row.try_get("topic_uuid").map_err(map_sqlx_error)?,
        subtopic_uuid: row.try_get("subtopic_uuid").map_err(map_sqlx_error)?,
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

fn decode_pool_revision(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<QuestionPoolRevisionNumber, StoreError> {
    QuestionPoolRevisionNumber::new(
        u64::try_from(row.try_get::<i64, _>(column).map_err(map_sqlx_error)?)
            .map_err(|_| invalid("Question Pool Revision"))?,
    )
    .map_err(|_| invalid("Question Pool Revision"))
}

fn invalid(field: &str) -> StoreError {
    StoreError::InvalidRecord(format!("stored {field} is invalid"))
}
