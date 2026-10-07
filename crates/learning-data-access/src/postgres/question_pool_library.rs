//! PostgreSQL reads for reusable published Question Pools.

use std::collections::BTreeSet;

use async_trait::async_trait;
use question_model::{
    AccountId, BloomClassificationView, BloomCognitiveProcess, BloomKnowledgeDimension,
    CurrentQuestionPoolMetadata, MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY, PublishedQuestionId,
    PublishedQuestionRevisionTuple, QuestionBackend, QuestionLicense, QuestionPoolEditNumber,
    QuestionPoolId, QuestionPoolMetadata, QuestionPoolMetadataEditNumber,
    QuestionPoolMetadataReplacement, QuestionRevisionNumber, QuestionType, QuestionUsageTotals,
    SaveQuestionPoolMembersRequest, SaveQuestionPoolMetadataRequest, SavedQuestionPoolMembers,
    SavedQuestionPoolMetadata,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{PublishedQuestionPool, QuestionPoolLibraryStore, SessionTokenHash, StoreError};

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
    async fn save_question_pool_members(
        &self,
        session_token_hash: SessionTokenHash,
        request: SaveQuestionPoolMembersRequest,
    ) -> Result<SavedQuestionPoolMembers, StoreError> {
        validate_member_replacement(&request)?;
        let mut transaction = self.begin(session_token_hash).await?;
        let question_ids = request
            .members
            .iter()
            .map(|member| member.published_question_id.as_str())
            .collect::<Vec<_>>();
        let revisions = request
            .members
            .iter()
            .map(|member| {
                i32::try_from(member.revision_number.get()).map_err(|_| {
                    StoreError::InvalidRecord(
                        "Question Pool member Revision number is outside the database range"
                            .to_owned(),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        // ASVS 8.3/13.4: bind every value; the SQL function performs the tuple-set CAS atomically.
        let row = sqlx::query(
            "SELECT question_pool_edit_number FROM ple_api.save_question_pool_members($1, $2, $3, $4)",
        )
        .bind(request.question_pool_id.as_str())
        .bind(request.expected_question_pool_edit_number.get() as i64)
        .bind(question_ids)
        .bind(revisions)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let question_pool_edit_number = u64::try_from(
            row.try_get::<i64, _>("question_pool_edit_number")
                .map_err(map_sqlx_error)?,
        )
        .ok()
        .and_then(|value| QuestionPoolEditNumber::new(value).ok())
        .ok_or_else(|| invalid("Question Pool Edit Number"))?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(SavedQuestionPoolMembers {
            question_pool_id: request.question_pool_id,
            question_pool_edit_number,
        })
    }

    async fn save_question_pool_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        request: SaveQuestionPoolMetadataRequest,
    ) -> Result<SavedQuestionPoolMetadata, StoreError> {
        validate_metadata_replacement(&request.metadata)?;
        let metadata = &request.metadata;
        if request.expected_metadata_edit_number.get() > i64::MAX as u64 {
            return Err(invalid("Question Pool metadata Edit Number is invalid"));
        }
        let mut transaction = self.begin(session_token_hash).await?;
        let row = sqlx::query(
            "SELECT question_pool_id, question_pool_metadata_edit_number \
             FROM ple_api.replace_question_pool_metadata($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(request.question_pool_id.as_str())
        .bind(request.expected_metadata_edit_number.get() as i64)
        .bind(&metadata.title)
        .bind(&metadata.description)
        .bind(metadata.topic_uuid)
        .bind(metadata.subtopic_uuid)
        .bind(&metadata.tags)
        .bind(
            metadata
                .bloom_cognitive_process
                .map(BloomCognitiveProcess::as_str),
        )
        .bind(
            metadata
                .bloom_knowledge_dimension
                .map(BloomKnowledgeDimension::as_str),
        )
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let question_pool_id = decode_pool_id(&row, "question_pool_id")?;
        if question_pool_id != request.question_pool_id {
            return Err(invalid("Question Pool metadata receipt target"));
        }
        let question_pool_metadata_edit_number = u64::try_from(
            row.try_get::<i64, _>("question_pool_metadata_edit_number")
                .map_err(map_sqlx_error)?,
        )
        .ok()
        .and_then(|value| question_model::QuestionPoolMetadataEditNumber::new(value).ok())
        .ok_or_else(|| invalid("Question Pool metadata Edit Number"))?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(SavedQuestionPoolMetadata {
            question_pool_id,
            question_pool_metadata_edit_number,
        })
    }

    async fn load_current_question_pool_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionPoolId,
    ) -> Result<CurrentQuestionPoolMetadata, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let row = sqlx::query("SELECT * FROM ple_api.read_current_question_pool_metadata($1)")
            .bind(question_pool_id.as_str())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let current = decode_current_metadata(&row)?;
        if current.question_pool_id != *question_pool_id {
            return Err(invalid("Question Pool metadata read target"));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(current)
    }

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
}

impl PostgresQuestionPoolLibraryStore {
    /// Pool delivery and outcome totals attributed to its originating receipt Pool.
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
    let mut tuples = Vec::with_capacity(rows.len());
    let mut question_ids = std::collections::BTreeSet::new();
    for row in rows {
        if metadata.as_ref() != Some(&decode_metadata(row)?) || bloom != decode_bloom(row)? {
            return Err(invalid("Question Pool lineage metadata"));
        }
        if decode_pool_id(row, "question_pool_id")? != *pool_id
            || decode_pool_edit_number(row)? != edit_number
        {
            return Err(invalid("Question Pool membership identity"));
        }
        let question_id = decode_published_question_id(row, "published_question_id")?;
        if !question_ids.insert(question_id.clone()) {
            return Err(invalid("Question Pool member set"));
        }
        let published_question_revision_tuple = u32::try_from(
            row.try_get::<i32, _>("question_revision_number")
                .map_err(map_sqlx_error)?,
        )
        .ok()
        .and_then(|value| QuestionRevisionNumber::new(value).ok())
        .ok_or_else(|| invalid("Question Revision"))?;
        tuples.push(PublishedQuestionRevisionTuple {
            published_question_id: question_id,
            revision_number: published_question_revision_tuple,
        });
    }
    Ok(tuples)
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
    if cognitive_process.is_none() && knowledge_dimension.is_none() {
        return Ok(None);
    }
    Ok(Some(BloomClassificationView {
        cognitive_process: cognitive_process
            .map(|value| value.parse::<BloomCognitiveProcess>())
            .transpose()
            .map_err(|_| invalid("Bloom Cognitive Process"))?,
        knowledge_dimension: knowledge_dimension
            .map(|value| value.parse::<BloomKnowledgeDimension>())
            .transpose()
            .map_err(|_| invalid("Bloom Knowledge Dimension"))?,
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
        bloom_cognitive_process: row
            .try_get::<Option<String>, _>("bloom_cognitive_process")
            .map_err(map_sqlx_error)?
            .map(|value| value.parse::<BloomCognitiveProcess>())
            .transpose()
            .map_err(|_| invalid("Bloom Cognitive Process"))?,
        bloom_knowledge_dimension: row
            .try_get::<Option<String>, _>("bloom_knowledge_dimension")
            .map_err(map_sqlx_error)?
            .map(|value| value.parse::<BloomKnowledgeDimension>())
            .transpose()
            .map_err(|_| invalid("Bloom Knowledge Dimension"))?,
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

fn validate_member_replacement(request: &SaveQuestionPoolMembersRequest) -> Result<(), StoreError> {
    if request.expected_question_pool_edit_number.get() > i64::MAX as u64 {
        return Err(StoreError::InvalidRecord(
            "Question Pool Edit Number is outside the database range".to_owned(),
        ));
    }
    if request.members.is_empty()
        || request.members.len() > MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
    {
        return Err(StoreError::InvalidRecord(
            "Question Pool member save requires a bounded nonempty tuple set".to_owned(),
        ));
    }
    let mut question_ids = BTreeSet::new();
    if request
        .members
        .iter()
        .any(|member| !question_ids.insert(member.published_question_id.as_str()))
    {
        return Err(StoreError::InvalidRecord(
            "Question Pool members must contain one Revision per Published Question".to_owned(),
        ));
    }
    Ok(())
}

fn validate_metadata_replacement(
    metadata: &QuestionPoolMetadataReplacement,
) -> Result<(), StoreError> {
    if !(1..=question_model::MAX_QUESTION_TITLE_UNICODE_SCALARS)
        .contains(&metadata.title.chars().count())
        || !(1..=question_model::MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS)
            .contains(&metadata.description.chars().count())
        || [&metadata.title, &metadata.description].iter().any(|text| {
            text.as_str() != text.trim_matches(' ') || text.chars().any(char::is_control)
        })
    {
        return Err(invalid("Question Pool title or description"));
    }
    if metadata.subtopic_uuid.is_some() && metadata.topic_uuid.is_none() {
        return Err(invalid("Question Pool classification"));
    }
    let mut unique_tags = std::collections::BTreeSet::new();
    if metadata.tags.iter().any(|tag| {
        tag.is_empty()
            || tag.trim_matches(' ') != tag
            || tag.chars().count() > 120
            || tag.chars().any(char::is_control)
            || !unique_tags.insert(tag)
    }) {
        return Err(invalid("Question Pool tags"));
    }
    Ok(())
}

fn decode_current_metadata(
    row: &sqlx::postgres::PgRow,
) -> Result<CurrentQuestionPoolMetadata, StoreError> {
    let question_pool_id = decode_pool_id(row, "question_pool_id")?;
    let question_pool_metadata_edit_number = u64::try_from(
        row.try_get::<i64, _>("question_pool_metadata_edit_number")
            .map_err(map_sqlx_error)?,
    )
    .ok()
    .and_then(|value| QuestionPoolMetadataEditNumber::new(value).ok())
    .ok_or_else(|| invalid("Question Pool metadata Edit Number"))?;
    let metadata = QuestionPoolMetadataReplacement {
        title: row.try_get("title").map_err(map_sqlx_error)?,
        description: row.try_get("description").map_err(map_sqlx_error)?,
        topic_uuid: row.try_get("topic_uuid").map_err(map_sqlx_error)?,
        subtopic_uuid: row.try_get("subtopic_uuid").map_err(map_sqlx_error)?,
        tags: row.try_get("tags").map_err(map_sqlx_error)?,
        bloom_cognitive_process: row
            .try_get::<Option<String>, _>("bloom_cognitive_process")
            .map_err(map_sqlx_error)?
            .map(|value| value.parse::<BloomCognitiveProcess>())
            .transpose()
            .map_err(|_| invalid("Bloom Cognitive Process"))?,
        bloom_knowledge_dimension: row
            .try_get::<Option<String>, _>("bloom_knowledge_dimension")
            .map_err(map_sqlx_error)?
            .map(|value| value.parse::<BloomKnowledgeDimension>())
            .transpose()
            .map_err(|_| invalid("Bloom Knowledge Dimension"))?,
    };
    validate_metadata_replacement(&metadata)?;
    Ok(CurrentQuestionPoolMetadata {
        question_pool_id,
        question_pool_metadata_edit_number,
        title: metadata.title,
        description: metadata.description,
        topic_uuid: metadata.topic_uuid,
        subtopic_uuid: metadata.subtopic_uuid,
        tags: metadata.tags,
        bloom_cognitive_process: metadata.bloom_cognitive_process,
        bloom_knowledge_dimension: metadata.bloom_knowledge_dimension,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn request(members: Vec<PublishedQuestionRevisionTuple>) -> SaveQuestionPoolMembersRequest {
        SaveQuestionPoolMembersRequest {
            question_pool_id: "3S8B-24DZ".parse().expect("canonical Pool ID"),
            expected_question_pool_edit_number: QuestionPoolEditNumber::new(4)
                .expect("positive Edit Number"),
            members,
        }
    }

    fn tuple(question_id: &str, revision_number: u32) -> PublishedQuestionRevisionTuple {
        PublishedQuestionRevisionTuple {
            published_question_id: question_id.parse().expect("canonical Question ID"),
            revision_number: QuestionRevisionNumber::new(revision_number)
                .expect("positive Revision number"),
        }
    }

    #[test]
    fn member_replacement_requires_bounded_nonempty_one_revision_per_question() {
        assert!(validate_member_replacement(&request(Vec::new())).is_err());
        assert!(
            validate_member_replacement(&request(vec![
                tuple("7K3M-79QP", 2),
                tuple("7K3M-79QP", 3),
            ]))
            .is_err()
        );
        assert!(
            validate_member_replacement(&request(vec![
                tuple("7K3M-79QP", 2),
                tuple("2R5X-E7YA", 3),
            ]))
            .is_ok()
        );
    }
}
