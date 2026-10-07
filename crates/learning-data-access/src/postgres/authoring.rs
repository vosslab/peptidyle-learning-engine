//! PostgreSQL persistence for the private Authoring Workspace boundary.

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{
    ObjectId, PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionAuthorDisplayName,
    QuestionBackend, QuestionFormat, QuestionLicense, QuestionMetadata, QuestionRevisionNumber,
    QuestionType, Tag, Timestamp, WorkspaceId,
};
use sqlx::{Postgres, Row, Transaction, types::Json};
use uuid::Uuid;

use super::Pool;
use super::connection::map_sqlx_error;
use crate::authoring::validate_initial_draft_source_binding;
use crate::{
    AuthoringDraft, AuthoringDraftStore, AuthoringDraftSummary, CreateAuthoringDraftInput,
    DeleteAuthoringDraftInput, DraftQuestionEditNumber, DraftQuestionUuid,
    SaveAuthoringDraftGeneralFeedbackInput, SaveAuthoringDraftInput, SessionTokenHash, StoreError,
    validate_workspace_question_source_object_record,
};

/// PostgreSQL Store for private Authoring Workspace and Draft Question operations.
#[derive(Clone)]
pub struct PostgresAuthoringDraftStore {
    pool: Pool,
}

impl PostgresAuthoringDraftStore {
    /// Binds the attested API pool to the private authoring procedures.
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
impl AuthoringDraftStore for PostgresAuthoringDraftStore {
    async fn ensure_own_authoring_workspace(
        &self,
        session_token_hash: SessionTokenHash,
        proposed_workspace_id: Uuid,
    ) -> Result<WorkspaceId, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let workspace_id: Uuid =
            sqlx::query_scalar("SELECT ple_api.ensure_own_authoring_workspace($1)")
                .bind(proposed_workspace_id)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(WorkspaceId::from_uuid(workspace_id))
    }

    async fn list_authoring_drafts(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<AuthoringDraftSummary>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let rows = sqlx::query("SELECT * FROM ple_api.list_authoring_drafts()")
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let summaries = rows
            .iter()
            .map(decode_summary)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(summaries)
    }

    async fn create_authoring_draft(
        &self,
        session_token_hash: SessionTokenHash,
        workspace: WorkspaceId,
        input: CreateAuthoringDraftInput,
    ) -> Result<AuthoringDraft, StoreError> {
        validate_workspace_question_source_object_record(workspace, &input.source_record)?;
        validate_initial_draft_source_binding(
            &input.source_record.media_type,
            input.question_format,
            input.webwork_pg_path.as_deref(),
        )?;
        let question_format = question_format_wire(input.question_format)?;
        let address = encode_address(&input.source_record)?;
        let size = postgres_size(input.source_record.size_bytes)?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let draft_question_uuid: Uuid = sqlx::query_scalar(
            "SELECT ple_api.create_authoring_draft($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
        )
        .bind(workspace.as_uuid())
        .bind(input.draft_question_uuid.as_uuid())
        .bind(input.source_record.id.as_uuid())
        .bind(address)
        .bind(input.source_record.sha256.as_bytes().to_vec())
        .bind(size)
        .bind(&input.source_record.media_type)
        .bind(input.source_record.created_at.as_unix_millis())
        .bind(&input.metadata.question_title)
        .bind(&input.metadata.question_description)
        .bind(input.metadata.tags.iter().map(|tag| tag.as_str().to_owned()).collect::<Vec<_>>())
        .bind(input.metadata.question_license.as_ref().map(question_license_wire).transpose()?)
        .bind(input.metadata.question_citation.as_deref())
        .bind(&input.metadata.language)
        .bind(&input.webwork_pg_path)
        .bind(input.question_type.map(question_type_wire).transpose()?)
        .bind(question_format)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        self.load_authoring_draft(
            session_token_hash,
            DraftQuestionUuid::from_uuid(draft_question_uuid),
        )
        .await
    }

    async fn load_authoring_draft(
        &self,
        session_token_hash: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
    ) -> Result<AuthoringDraft, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query("SELECT * FROM ple_api.load_authoring_draft($1)")
            .bind(draft_question_uuid.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let draft = row
            .as_ref()
            .map(decode_draft)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(draft)
    }

    async fn save_authoring_draft(
        &self,
        session_token_hash: SessionTokenHash,
        input: SaveAuthoringDraftInput,
    ) -> Result<AuthoringDraft, StoreError> {
        let current = self
            .load_authoring_draft(session_token_hash, input.draft_question_uuid)
            .await?;
        validate_workspace_question_source_object_record(current.workspace, &input.source_record)?;
        let address = encode_address(&input.source_record)?;
        let size = postgres_size(input.source_record.size_bytes)?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        sqlx::query("SELECT ple_api.save_authoring_draft($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(input.draft_question_uuid.as_uuid())
            .bind(input.expected_edit_number.as_postgres_bigint())
            .bind(input.source_record.id.as_uuid())
            .bind(address)
            .bind(input.source_record.sha256.as_bytes().to_vec())
            .bind(size)
            .bind(&input.source_record.media_type)
            .bind(input.source_record.created_at.as_unix_millis())
            .bind(input.question_type.map(question_type_wire).transpose()?)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        // Decode this projection before commit releases the CAS row lock so its
        // Edit Number and values remain tied to this save.
        let row = sqlx::query("SELECT * FROM ple_api.load_authoring_draft($1)")
            .bind(input.draft_question_uuid.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let draft = row
            .as_ref()
            .map(decode_draft)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(draft)
    }

    async fn save_authoring_draft_general_feedback(
        &self,
        session_token_hash: SessionTokenHash,
        input: SaveAuthoringDraftGeneralFeedbackInput,
    ) -> Result<AuthoringDraft, StoreError> {
        self.save_authoring_draft_general_feedback_with_question_type(
            session_token_hash,
            input,
            None,
        )
        .await
    }

    async fn save_authoring_draft_general_feedback_with_question_type(
        &self,
        session_token_hash: SessionTokenHash,
        input: SaveAuthoringDraftGeneralFeedbackInput,
        question_type_update: Option<Option<QuestionType>>,
    ) -> Result<AuthoringDraft, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let (replace_question_type, question_type) =
            question_type_update_args(question_type_update)?;
        sqlx::query(
            "SELECT * FROM ple_api.save_authoring_draft_general_feedback($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(input.draft_question_uuid.as_uuid())
        .bind(input.expected_edit_number.as_postgres_bigint())
        .bind(&input.general_feedback)
        .bind(&input.hint)
        .bind(&input.worked_solution)
        .bind(input.replace_support)
        .bind(&input.metadata.question_title)
        .bind(&input.metadata.question_description)
        .bind(input.metadata.tags.iter().map(|tag| tag.as_str().to_owned()).collect::<Vec<_>>())
        .bind(input.metadata.question_license.as_ref().map(question_license_wire).transpose()?)
        .bind(input.metadata.question_citation.as_deref())
        .bind(&input.metadata.language)
        .bind(replace_question_type)
        .bind(question_type)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        // Decode the shared projection before commit releases the CAS row lock
        // so its Edit Number and values remain tied to this metadata/support save.
        let row = sqlx::query("SELECT * FROM ple_api.load_authoring_draft($1)")
            .bind(input.draft_question_uuid.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let draft = row
            .as_ref()
            .map(decode_draft)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(draft)
    }

    async fn delete_authoring_draft(
        &self,
        session_token_hash: SessionTokenHash,
        input: DeleteAuthoringDraftInput,
    ) -> Result<(), StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        sqlx::query("SELECT ple_api.delete_draft_question($1, $2)")
            .bind(input.draft_question_uuid.as_uuid())
            .bind(input.expected_edit_number.as_postgres_bigint())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<AuthoringDraftSummary, StoreError> {
    Ok(AuthoringDraftSummary {
        draft_question_uuid: DraftQuestionUuid::from_uuid(
            row.try_get("draft_question_id").map_err(map_sqlx_error)?,
        ),
        parent_published_question_revision_tuple: decode_parent_revision_tuple(row)?,
        edit_number: edit_number(
            row.try_get("draft_question_edit_number")
                .map_err(map_sqlx_error)?,
        )?,
        metadata: decode_metadata(row)?,
    })
}

fn decode_draft(row: &sqlx::postgres::PgRow) -> Result<AuthoringDraft, StoreError> {
    let source_record = decode_source_record(row)?;
    let workspace = WorkspaceId::from_uuid(
        row.try_get("authoring_workspace_id")
            .map_err(map_sqlx_error)?,
    );
    validate_workspace_question_source_object_record(workspace, &source_record)?;
    let draft = AuthoringDraft {
        draft_question_uuid: DraftQuestionUuid::from_uuid(
            row.try_get("draft_question_id").map_err(map_sqlx_error)?,
        ),
        workspace,
        parent_published_question_revision_tuple: decode_parent_revision_tuple(row)?,
        edit_number: edit_number(
            row.try_get("draft_question_edit_number")
                .map_err(map_sqlx_error)?,
        )?,
        metadata: decode_metadata(row)?,
        classification: decode_classification(row)?,
        question_backend: question_backend_from_wire(
            &row.try_get::<String, _>("question_backend")
                .map_err(map_sqlx_error)?,
        )?,
        question_format: question_format_from_wire(
            &row.try_get::<String, _>("question_format")
                .map_err(map_sqlx_error)?,
        )?,
        webwork_pg_path: row.try_get("webwork_pg_path").map_err(map_sqlx_error)?,
        general_feedback: row.try_get("general_feedback").map_err(map_sqlx_error)?,
        hint: row.try_get("hint").map_err(map_sqlx_error)?,
        worked_solution: row.try_get("worked_solution").map_err(map_sqlx_error)?,
        authors: row
            .try_get::<Vec<String>, _>("author_names")
            .map_err(map_sqlx_error)?
            .into_iter()
            .map(|display_name| {
                QuestionAuthorDisplayName::new(display_name)
                    .map_err(|_| invalid("Draft Question authorship"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        question_type: row
            .try_get::<Option<String>, _>("question_type")
            .map_err(map_sqlx_error)?
            .as_deref()
            .map(question_type_from_wire)
            .transpose()?,
        source_record,
    };
    if !draft
        .source_media_type()
        .is_some_and(|media_type| media_type == draft.source_record.media_type)
    {
        return Err(invalid("Draft Question source binding"));
    }
    Ok(draft)
}

fn decode_parent_revision_tuple(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<PublishedQuestionRevisionTuple>, StoreError> {
    let parent_id: Option<String> = row
        .try_get("parent_published_question_id")
        .map_err(map_sqlx_error)?;
    let parent_revision: Option<i32> = row
        .try_get("parent_revision_number")
        .map_err(map_sqlx_error)?;
    match (parent_id, parent_revision) {
        (None, None) => Ok(None),
        (Some(parent_id), Some(parent_revision)) if parent_revision > 0 => {
            Ok(Some(PublishedQuestionRevisionTuple {
                published_question_id: parent_id
                    .parse::<PublishedQuestionId>()
                    .map_err(|_| invalid("Draft Question Parent Revision Tuple"))?,
                revision_number: QuestionRevisionNumber::new(parent_revision as u32)
                    .map_err(|_| invalid("Draft Question Parent Revision Tuple"))?,
            }))
        }
        _ => Err(invalid("Draft Question Parent Revision Tuple")),
    }
}

fn decode_classification(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<question_model::DraftQuestionClassification>, StoreError> {
    let discipline_uuid: Option<Uuid> = row
        .try_get("content_discipline_id")
        .map_err(map_sqlx_error)?;
    let subject_uuid: Option<Uuid> = row.try_get("content_subject_id").map_err(map_sqlx_error)?;
    let topic_uuid: Option<Uuid> = row.try_get("content_topic_id").map_err(map_sqlx_error)?;
    let subtopic_uuid: Option<Uuid> = row.try_get("content_subtopic_id").map_err(map_sqlx_error)?;
    let cognitive_process: Option<String> = row
        .try_get("bloom_cognitive_process")
        .map_err(map_sqlx_error)?;
    let knowledge_dimension: Option<String> = row
        .try_get("bloom_knowledge_dimension")
        .map_err(map_sqlx_error)?;
    let bloom = if cognitive_process.is_none() && knowledge_dimension.is_none() {
        None
    } else {
        Some(question_model::BloomClassificationView {
            cognitive_process: cognitive_process
                .map(|value| serde_json::from_value(serde_json::Value::String(value)))
                .transpose()
                .map_err(|_| invalid("Draft Question Bloom Classification"))?,
            knowledge_dimension: knowledge_dimension
                .map(|value| serde_json::from_value(serde_json::Value::String(value)))
                .transpose()
                .map_err(|_| invalid("Draft Question Bloom Classification"))?,
        })
    };
    if discipline_uuid.is_none()
        && subject_uuid.is_none()
        && topic_uuid.is_none()
        && subtopic_uuid.is_none()
        && bloom.is_none()
    {
        return Ok(None);
    }
    Ok(Some(question_model::DraftQuestionClassification {
        discipline_uuid,
        subject_uuid,
        topic_uuid,
        subtopic_uuid,
        bloom,
    }))
}

fn question_backend_from_wire(value: &str) -> Result<QuestionBackend, StoreError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| invalid("Question Backend"))
}

fn question_type_wire(value: QuestionType) -> Result<String, StoreError> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| invalid("Question Type"))
}

fn question_type_update_args(
    update: Option<Option<QuestionType>>,
) -> Result<(bool, Option<String>), StoreError> {
    match update {
        None => Ok((false, None)),
        Some(None) => Ok((true, None)),
        Some(Some(question_type)) => Ok((true, Some(question_type_wire(question_type)?))),
    }
}

fn question_license_wire(value: &QuestionLicense) -> Result<String, StoreError> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| invalid("Question License"))
}

fn decode_metadata(row: &sqlx::postgres::PgRow) -> Result<QuestionMetadata, StoreError> {
    let tags = row
        .try_get::<Vec<String>, _>("tags")
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Tag::new)
        .collect();
    let question_license = row
        .try_get::<Option<String>, _>("question_license")
        .map_err(map_sqlx_error)?
        .map(|value| serde_json::from_value(serde_json::Value::String(value)))
        .transpose()
        .map_err(|_| invalid("Question License"))?;
    let question_citation = row
        .try_get::<Option<String>, _>("citation_text")
        .map_err(map_sqlx_error)?;
    Ok(QuestionMetadata {
        question_title: row.try_get("question_title").map_err(map_sqlx_error)?,
        question_description: row
            .try_get("question_description")
            .map_err(map_sqlx_error)?,
        tags,
        question_license,
        question_citation,
        language: row.try_get("language").map_err(map_sqlx_error)?,
    })
}

fn question_format_wire(value: QuestionFormat) -> Result<String, StoreError> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| invalid("Question Format"))
}

fn question_format_from_wire(value: &str) -> Result<QuestionFormat, StoreError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| invalid("Question Format"))
}

fn question_type_from_wire(value: &str) -> Result<QuestionType, StoreError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| invalid("Question Type"))
}

fn decode_source_record(row: &sqlx::postgres::PgRow) -> Result<ObjectRecord, StoreError> {
    let Json(address): Json<ObjectAddress> =
        row.try_get("object_address").map_err(map_sqlx_error)?;
    let checksum: Vec<u8> = row.try_get("sha256").map_err(map_sqlx_error)?;
    let checksum: [u8; 32] = checksum
        .try_into()
        .map_err(|_| invalid("source checksum"))?;
    Ok(ObjectRecord {
        id: ObjectId::from_uuid(row.try_get("object_record_id").map_err(map_sqlx_error)?),
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::AuthoringContent,
        address,
        sha256: Sha256Checksum::from_bytes(checksum),
        size_bytes: u64::try_from(
            row.try_get::<i64, _>("size_bytes")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("source size"))?,
        media_type: row.try_get("media_type").map_err(map_sqlx_error)?,
        published_question_revision_tuple: None,
        created_at: Timestamp::from_unix_millis(
            row.try_get("created_at_millis").map_err(map_sqlx_error)?,
        ),
    })
}

fn encode_address(record: &ObjectRecord) -> Result<serde_json::Value, StoreError> {
    serde_json::to_value(&record.address).map_err(|_| invalid("source object address"))
}

fn postgres_size(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| invalid("source size"))
}

fn edit_number(value: i64) -> Result<DraftQuestionEditNumber, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(|value| DraftQuestionEditNumber::new(value).ok())
        .ok_or_else(|| invalid("Draft Question Edit Number"))
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_question_type_maps_omission_clear_and_value_to_explicit_arguments() {
        assert_eq!(
            question_type_update_args(None).expect("omitted type"),
            (false, None)
        );
        assert_eq!(
            question_type_update_args(Some(None)).expect("clear type"),
            (true, None)
        );
        assert_eq!(
            question_type_update_args(Some(Some(QuestionType::Hotspot))).expect("manual type"),
            (true, Some("hotspot".to_owned()))
        );
    }
}
