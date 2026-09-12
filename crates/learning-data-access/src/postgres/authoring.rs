//! PostgreSQL persistence for the private Authoring Workspace boundary.

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{DraftQuestionReference, ObjectId, Timestamp, WorkspaceId};
use sqlx::{Postgres, Row, Transaction, types::Json};
use uuid::Uuid;

use super::Pool;
use super::connection::map_sqlx_error;
use crate::authoring::validate_initial_draft_source_binding;
use crate::{
    AuthoringDraft, AuthoringDraftStore, AuthoringDraftSummary, CreateAuthoringDraftInput,
    DraftQuestionEditNumber, DraftQuestionUuid, SaveAuthoringDraftInput, SessionTokenHash,
    StoreError, validate_workspace_question_source_object_record,
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
            input.webwork_pg_path.as_deref(),
        )?;
        let address = encode_address(&input.source_record)?;
        let size = postgres_size(input.source_record.size_bytes)?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let reference_number: i64 = sqlx::query_scalar(
            "SELECT ple_api.create_authoring_draft($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(workspace.as_uuid())
        .bind(input.draft_question_uuid.as_uuid())
        .bind(input.source_record.id.as_uuid())
        .bind(address)
        .bind(input.source_record.sha256.as_bytes().to_vec())
        .bind(size)
        .bind(&input.source_record.media_type)
        .bind(input.source_record.created_at.as_unix_millis())
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.language)
        .bind(&input.webwork_pg_path)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        self.load_authoring_draft(session_token_hash, reference(reference_number)?)
            .await
    }

    async fn load_authoring_draft(
        &self,
        session_token_hash: SessionTokenHash,
        reference: DraftQuestionReference,
    ) -> Result<AuthoringDraft, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query("SELECT * FROM ple_api.load_authoring_draft($1)")
            .bind(i64::from(reference.number()))
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
            .load_authoring_draft(session_token_hash, input.reference)
            .await?;
        validate_workspace_question_source_object_record(current.workspace, &input.source_record)?;
        let address = encode_address(&input.source_record)?;
        let size = postgres_size(input.source_record.size_bytes)?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        sqlx::query(
            "SELECT ple_api.save_authoring_draft($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(i64::from(input.reference.number()))
        .bind(input.expected_edit_number.as_postgres_bigint())
        .bind(input.source_record.id.as_uuid())
        .bind(address)
        .bind(input.source_record.sha256.as_bytes().to_vec())
        .bind(size)
        .bind(&input.source_record.media_type)
        .bind(input.source_record.created_at.as_unix_millis())
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.language)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        self.load_authoring_draft(session_token_hash, input.reference)
            .await
    }
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<AuthoringDraftSummary, StoreError> {
    Ok(AuthoringDraftSummary {
        reference: reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?,
        edit_number: edit_number(
            row.try_get("draft_question_edit_number")
                .map_err(map_sqlx_error)?,
        )?,
        title: row.try_get("question_title").map_err(map_sqlx_error)?,
        description: row
            .try_get("question_description")
            .map_err(map_sqlx_error)?,
    })
}

fn decode_draft(row: &sqlx::postgres::PgRow) -> Result<AuthoringDraft, StoreError> {
    let source_record = decode_source_record(row)?;
    let workspace = WorkspaceId::from_uuid(row.try_get("workspace_id").map_err(map_sqlx_error)?);
    validate_workspace_question_source_object_record(workspace, &source_record)?;
    Ok(AuthoringDraft {
        draft_question_uuid: DraftQuestionUuid::from_uuid(
            row.try_get("draft_question_uuid").map_err(map_sqlx_error)?,
        ),
        workspace,
        reference: reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?,
        edit_number: edit_number(
            row.try_get("draft_question_edit_number")
                .map_err(map_sqlx_error)?,
        )?,
        title: row.try_get("question_title").map_err(map_sqlx_error)?,
        description: row
            .try_get("question_description")
            .map_err(map_sqlx_error)?,
        source_record,
    })
}

fn decode_source_record(row: &sqlx::postgres::PgRow) -> Result<ObjectRecord, StoreError> {
    let Json(address): Json<ObjectAddress> =
        row.try_get("object_address").map_err(map_sqlx_error)?;
    let checksum: Vec<u8> = row.try_get("sha256").map_err(map_sqlx_error)?;
    let checksum: [u8; 32] = checksum
        .try_into()
        .map_err(|_| invalid("source checksum"))?;
    Ok(ObjectRecord {
        id: ObjectId::from_uuid(row.try_get("object_id").map_err(map_sqlx_error)?),
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
        question_revision: None,
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

fn reference(value: i64) -> Result<DraftQuestionReference, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(DraftQuestionReference::new)
        .ok_or_else(|| invalid("Draft Question Reference"))
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
