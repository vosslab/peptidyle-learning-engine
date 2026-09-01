//! PostgreSQL persistence for immutable Draft Question Sources.

use async_trait::async_trait;
use serde::Serialize;
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    DraftQuestionSourceInput, DraftQuestionSourceStore, QuestionSourceUuid, SessionTokenHash,
    StoreError,
};

/// PostgreSQL implementation of the session-authorized Draft Question Source Store.
#[derive(Clone)]
pub struct PostgresDraftQuestionSourceStore {
    pool: Pool,
}

impl PostgresDraftQuestionSourceStore {
    /// Binds the already-attested API pool to private Question Source persistence.
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
impl DraftQuestionSourceStore for PostgresDraftQuestionSourceStore {
    async fn register_draft_question_source(
        &self,
        session_token_hash: SessionTokenHash,
        input: DraftQuestionSourceInput,
    ) -> Result<QuestionSourceUuid, StoreError> {
        input.validate()?;
        let question_source_uuid = crate::random_uuid::random_uuid_v4(|error| {
            StoreError::Unavailable(format!(
                "Question Source ID randomness unavailable: {error}"
            ))
        })?;
        let question_format = wire_string(&input.question_format, "Question Format")?;
        let question_type = wire_string(&input.question_type, "Question Type")?;
        let draft_question_revision_number =
            postgres_revision_number(input.draft_question_revision.revision_number)?;
        let backend_locator = serde_json::to_value(&input.backend_locator).map_err(|_| {
            StoreError::InvalidRecord("Question Backend locator cannot be encoded".to_string())
        })?;

        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.4, 2.2.2, 2.3.1, 8.2.1, and 8.3.1: every value is
        // parameterized; the database resolves the authenticated session and
        // authorizes the exact workspace/revision/object relationship in one
        // transaction before it creates or returns an immutable record.
        let row = sqlx::query(
            "SELECT ple_api.register_draft_question_source(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11\
             ) AS question_source_uuid",
        )
        .bind(question_source_uuid)
        .bind(input.draft_question_revision.draft_question_uuid.as_uuid())
        .bind(draft_question_revision_number)
        .bind(input.workspace.as_uuid())
        .bind(input.question_backend.as_str())
        .bind(question_format)
        .bind(question_type)
        .bind(backend_locator)
        .bind(input.source_object_reference.object.as_uuid())
        .bind(input.source_object_checksum.as_str())
        .bind(input.public_binding_checksum.as_str())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let source = QuestionSourceUuid::from_uuid(
            row.try_get("question_source_uuid")
                .map_err(map_sqlx_error)?,
        );
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(source)
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

fn postgres_revision_number(value: crate::DraftQuestionRevisionNumber) -> Result<i32, StoreError> {
    i32::try_from(value.get()).map_err(|_| {
        StoreError::InvalidRecord(
            "Draft Question Revision Number exceeds the PostgreSQL integer range".to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::postgres_revision_number;
    use crate::{DraftQuestionRevisionNumber, StoreError};

    #[test]
    fn draft_question_revision_number_must_fit_its_postgresql_storage_column() {
        let maximum =
            DraftQuestionRevisionNumber::new(i32::MAX as u32).expect("positive PostgreSQL maximum");
        assert_eq!(postgres_revision_number(maximum), Ok(i32::MAX));

        let too_large = DraftQuestionRevisionNumber::new(i32::MAX as u32 + 1)
            .expect("positive number beyond PostgreSQL range");
        assert!(matches!(
            postgres_revision_number(too_large),
            Err(StoreError::InvalidRecord(_))
        ));
    }
}
