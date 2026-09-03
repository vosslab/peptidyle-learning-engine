//! PostgreSQL persistence for Draft Question Source Registrations.

use async_trait::async_trait;
use serde::Serialize;
use sqlx::{Postgres, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    DraftQuestionSourceRegistrationInput, DraftQuestionSourceRegistrationStore, SessionTokenHash,
    StoreError,
};

/// PostgreSQL implementation of the session-authorized Draft Question Source Registration Store.
#[derive(Clone)]
pub struct PostgresDraftQuestionSourceRegistrationStore {
    pool: Pool,
}

impl PostgresDraftQuestionSourceRegistrationStore {
    /// Binds the already-attested API pool to private Question Source Registration persistence.
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
impl DraftQuestionSourceRegistrationStore for PostgresDraftQuestionSourceRegistrationStore {
    async fn register_draft_question_source_registration(
        &self,
        session_token_hash: SessionTokenHash,
        input: DraftQuestionSourceRegistrationInput,
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
            "SELECT ple_api.register_draft_question_source_registration(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13\
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
        .bind(input.qti_package_item_identifier)
        .bind(input.workspace_import_id.map(|id| id.as_uuid()))
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

fn wire_string(value: &impl Serialize, label: &str) -> Result<String, StoreError> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(value)) => Ok(value),
        _ => Err(StoreError::InvalidRecord(format!(
            "{label} must have one scalar canonical wire value"
        ))),
    }
}
