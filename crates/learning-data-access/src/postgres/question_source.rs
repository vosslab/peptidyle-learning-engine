//! PostgreSQL persistence for Draft Question Source Bindings.

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{ObjectId, Timestamp};
use serde::Serialize;
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    DraftQuestionPublicationSourceStore, DraftQuestionSourceBindingInput,
    DraftQuestionSourceBindingStore, DraftQuestionUuid, NewQuestionLineagePublicationInput,
    NewQuestionLineagePublicationStore, SessionTokenHash, StoreError,
};
use question_model::WorkspaceId;

/// PostgreSQL implementation of the session-authorized Draft Question Source Binding Store.
#[derive(Clone)]
pub struct PostgresDraftQuestionSourceBindingStore {
    pool: Pool,
}

#[async_trait]
impl DraftQuestionPublicationSourceStore for PostgresDraftQuestionSourceBindingStore {
    async fn load_draft_question_publication_source(
        &self,
        session_token_hash: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
        expected_draft_question_edit_number: crate::DraftQuestionEditNumber,
        workspace: WorkspaceId,
    ) -> Result<ObjectRecord, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.4, 2.2.2, 2.3.1, 8.2.1-8.2.3, and 8.3.1: values
        // remain parameters, while PostgreSQL rechecks current Instructor and
        // workspace authority plus the exact Draft Question Edit Number.
        let row =
            sqlx::query("SELECT * FROM ple_api.load_draft_question_publication_source($1, $2, $3)")
                .bind(draft_question_uuid.as_uuid())
                .bind(expected_draft_question_edit_number.as_postgres_bigint())
                .bind(workspace.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;

        let object_id = ObjectId::from_uuid(row.try_get("object_id").map_err(map_sqlx_error)?);
        let Json(object_address): Json<ObjectAddress> =
            row.try_get("object_address").map_err(map_sqlx_error)?;
        let checksum: Vec<u8> = row.try_get("sha256").map_err(map_sqlx_error)?;
        let checksum: [u8; 32] = checksum.try_into().map_err(|_| {
            StoreError::InvalidRecord(
                "Draft Question Source Object Record checksum has invalid width".to_string(),
            )
        })?;
        let size_bytes: i64 = row.try_get("size_bytes").map_err(map_sqlx_error)?;
        let size_bytes = u64::try_from(size_bytes).map_err(|_| {
            StoreError::InvalidRecord(
                "Draft Question Source Object Record size is negative".to_string(),
            )
        })?;
        let created_at_millis: i64 = row.try_get("created_at_millis").map_err(map_sqlx_error)?;
        Ok(ObjectRecord {
            id: object_id,
            storage_area: ObjectStorageArea::PrivateContent,
            data_class: ObjectDataClass::AuthoringContent,
            address: object_address,
            sha256: Sha256Checksum::from_bytes(checksum),
            size_bytes,
            media_type: row.try_get("media_type").map_err(map_sqlx_error)?,
            question_revision: None,
            created_at: Timestamp::from_unix_millis(created_at_millis),
        })
    }
}

impl PostgresDraftQuestionSourceBindingStore {
    /// Binds the already-attested API pool to private Draft Question Source Binding persistence.
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
impl DraftQuestionSourceBindingStore for PostgresDraftQuestionSourceBindingStore {
    async fn bind_draft_question_source(
        &self,
        session_token_hash: SessionTokenHash,
        input: DraftQuestionSourceBindingInput,
    ) -> Result<(), StoreError> {
        input.validate()?;
        let question_format = wire_string(&input.question_format, "Question Format")?;
        let imathas_deployment_reference = input
            .draft_imathas_question_backend_binding
            .as_ref()
            .map(|binding| binding.deployment_reference().as_str().to_owned());
        let imathas_item_reference = input
            .draft_imathas_question_backend_binding
            .as_ref()
            .map(|binding| binding.item_reference().as_str().to_owned());

        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.4, 2.2.2, 2.3.1, 8.2.1, and 8.3.1: every value is
        // parameterized; the database resolves the authenticated session and
        // authorizes the exact workspace/Draft Question/Edit Number/object relationship in one
        // transaction before it creates or confirms an immutable record.
        sqlx::query(
            "SELECT ple_api.bind_draft_question_source(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11\
             )",
        )
        .bind(input.draft_question_uuid.as_uuid())
        .bind(
            input
                .expected_draft_question_edit_number
                .as_postgres_bigint(),
        )
        .bind(input.workspace.as_uuid())
        .bind(input.question_backend.as_str())
        .bind(question_format)
        .bind(input.webwork_pg_path)
        .bind(imathas_deployment_reference)
        .bind(imathas_item_reference)
        .bind(Option::<String>::None)
        .bind(input.source_object_reference.object.as_uuid())
        .bind(input.source_object_checksum.as_str())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }
}

#[async_trait]
impl NewQuestionLineagePublicationStore for PostgresDraftQuestionSourceBindingStore {
    async fn publish_new_question_lineage(
        &self,
        session_token_hash: SessionTokenHash,
        input: NewQuestionLineagePublicationInput,
    ) -> Result<question_model::QuestionRevisionReference, StoreError> {
        input.validate()?;
        let question_revision = input.question_revision();
        let object_record = &input.question_source_object_record;
        let object_address = serde_json::to_value(&object_record.address).map_err(|_| {
            StoreError::InvalidRecord(
                "Question Publication Object Address cannot be encoded".to_string(),
            )
        })?;
        let size_bytes = i64::try_from(object_record.size_bytes).map_err(|_| {
            StoreError::InvalidRecord(
                "Question Publication source size exceeds PostgreSQL bigint".to_string(),
            )
        })?;
        let question_authorship: Vec<&str> = input
            .question_authorship
            .authors
            .iter()
            .map(|author| author.display_name.as_str())
            .collect();
        let question_authorship = serde_json::to_value(question_authorship).map_err(|_| {
            StoreError::InvalidRecord("Question Authorship cannot be encoded".to_string())
        })?;
        let question_license = wire_string(&input.question_license, "Question License")?;

        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.4, 2.2.2, 2.3.1, 2.3.3, 5.3.2, 8.2.1-8.2.3,
        // and 8.3.1: all values are parameters. The database rechecks current
        // Instructor/workspace authority, locks the exact Draft Question Edit
        // Number, derives the target Object Address, and commits the complete
        // Published Question aggregate in one transaction.
        sqlx::query(
            "SELECT ple_api.publish_new_question_lineage(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16\
             )",
        )
        .bind(input.draft_question_uuid.as_uuid())
        .bind(
            input
                .expected_draft_question_edit_number
                .as_postgres_bigint(),
        )
        .bind(input.workspace.as_uuid())
        .bind(input.question_id.to_string())
        .bind(object_record.id.as_uuid())
        .bind(object_address)
        .bind(object_record.sha256.as_bytes().to_vec())
        .bind(size_bytes)
        .bind(&object_record.media_type)
        .bind(object_record.created_at.as_unix_millis())
        .bind(question_authorship)
        .bind(question_license)
        .bind(input.question_revision_reason.as_str())
        .bind(input.question_ownership_event_id)
        .bind(input.question_publication_event_id)
        .bind(input.question_availability_event_id)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(question_revision)
    }
}

fn wire_string(value: &impl Serialize, label: &str) -> Result<String, StoreError> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(value)) => Ok(value),
        _ => Err(StoreError::InvalidRecord(format!(
            "{label} must have one scalar canonical wire value"
        ))),
    }
}
