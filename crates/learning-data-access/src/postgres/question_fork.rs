//! PostgreSQL adapter for the server-derived Published Question fork command.

use async_trait::async_trait;
use question_model::{DraftQuestionReference, QuestionId, QuestionRevisionReference, WorkspaceId};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    ForkPublishedQuestionError, ForkPublishedQuestionInput, ForkedPublishedQuestionDraft,
    QuestionForkStore, SessionTokenHash, StoreError,
};

const FORKED_QUESTION_ID_UNIQUE: &str = "draft_question_fork_source_forked_question_id_key";

/// PostgreSQL Store for source resolution and one atomic Draft fork.
#[derive(Clone)]
pub struct PostgresQuestionForkStore {
    pool: Pool,
}

impl PostgresQuestionForkStore {
    /// Binds the attested application pool to the narrow fork procedures.
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
impl QuestionForkStore for PostgresQuestionForkStore {
    async fn load_available_question_fork_source(
        &self,
        session_token_hash: SessionTokenHash,
        question_revision: &QuestionRevisionReference,
    ) -> Result<QuestionRevisionReference, StoreError> {
        let revision_number =
            i32::try_from(question_revision.revision_number.get()).map_err(|_| {
                StoreError::InvalidRecord(
                    "Question Fork Revision Number exceeds PostgreSQL integer".to_owned(),
                )
            })?;
        let mut transaction = self.begin(session_token_hash).await?;
        let row = sqlx::query("SELECT * FROM ple_api.load_available_question_fork_source($1, $2)")
            .bind(question_revision.question_id.as_compact_str())
            .bind(revision_number)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let question_id: String = row.try_get("question_id").map_err(map_sqlx_error)?;
        let question_id = question_id.parse::<QuestionId>().map_err(|_| {
            StoreError::InvalidRecord(
                "Question Fork source returned an invalid Question ID".to_owned(),
            )
        })?;
        let revision_number: i32 = row.try_get("revision_number").map_err(map_sqlx_error)?;
        let revision_number = u32::try_from(revision_number)
            .ok()
            .and_then(|value| question_model::QuestionRevisionNumber::new(value).ok())
            .ok_or_else(|| {
                StoreError::InvalidRecord(
                    "Question Fork source returned an invalid Revision Number".to_owned(),
                )
            })?;
        let resolved = QuestionRevisionReference {
            question_id,
            revision_number,
        };
        if resolved != *question_revision {
            return Err(StoreError::InvalidRecord(
                "Question Fork source did not return the requested exact Revision".to_owned(),
            ));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(resolved)
    }

    async fn fork_published_question_to_draft(
        &self,
        session_token_hash: SessionTokenHash,
        input: ForkPublishedQuestionInput,
    ) -> Result<ForkedPublishedQuestionDraft, ForkPublishedQuestionError> {
        let source_revision_number =
            i32::try_from(input.source_question_revision.revision_number.get()).map_err(|_| {
                ForkPublishedQuestionError::Store(StoreError::InvalidRecord(
                    "Question Fork Revision Number exceeds PostgreSQL integer".to_owned(),
                ))
            })?;
        let mut transaction = self
            .begin(session_token_hash)
            .await
            .map_err(ForkPublishedQuestionError::Store)?;
        let row = sqlx::query(
            "SELECT * FROM ple_api.fork_published_question_to_draft($1, $2, $3, $4, $5, $6)",
        )
        .bind(input.proposed_workspace_id)
        .bind(input.proposed_draft_question_id)
        .bind(input.forked_question_id.as_compact_str())
        .bind(input.source_question_revision.question_id.as_compact_str())
        .bind(source_revision_number)
        .bind(input.idempotency_key)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_fork_error)?;
        let reference_number: i64 = row
            .try_get("reference_number")
            .map_err(map_sqlx_error)
            .map_err(ForkPublishedQuestionError::Store)?;
        let draft_question = u64::try_from(reference_number)
            .ok()
            .and_then(DraftQuestionReference::new)
            .ok_or_else(|| {
                ForkPublishedQuestionError::Store(StoreError::InvalidRecord(
                    "Question Fork returned an invalid Draft Question Reference".to_owned(),
                ))
            })?;
        let workspace = WorkspaceId::from_uuid(
            row.try_get("workspace_id")
                .map_err(map_sqlx_error)
                .map_err(ForkPublishedQuestionError::Store)?,
        );
        let forked_question_id: String = row
            .try_get("forked_question_id")
            .map_err(map_sqlx_error)
            .map_err(ForkPublishedQuestionError::Store)?;
        let forked_question_id = forked_question_id.parse::<QuestionId>().map_err(|_| {
            ForkPublishedQuestionError::Store(StoreError::InvalidRecord(
                "Question Fork returned an invalid Question ID".to_owned(),
            ))
        })?;
        transaction
            .commit()
            .await
            .map_err(map_sqlx_error)
            .map_err(ForkPublishedQuestionError::Store)?;
        Ok(ForkedPublishedQuestionDraft {
            draft_question,
            workspace,
            forked_question_id,
        })
    }
}

fn map_fork_error(error: sqlx::Error) -> ForkPublishedQuestionError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("23505")
        && database_error.constraint() == Some(FORKED_QUESTION_ID_UNIQUE)
    {
        return ForkPublishedQuestionError::IdentityCollision;
    }
    ForkPublishedQuestionError::Store(map_sqlx_error(error))
}
