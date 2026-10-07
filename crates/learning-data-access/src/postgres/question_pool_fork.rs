//! PostgreSQL implementation of explicit reusable Question Pool forks.

use async_trait::async_trait;
use question_model::QuestionPoolId;
use sqlx::{Postgres, Row, Transaction};

use super::{
    Pool, connection::map_sqlx_error, question_pool_creation::map_create_question_pool_error,
};
use crate::{
    CreateQuestionPoolError, CreatedQuestionPool, QuestionPoolForkStore, SessionTokenHash,
    StoreError,
};

/// PostgreSQL Store for the session-bound Pool-fork capability.
#[derive(Clone)]
pub struct PostgresQuestionPoolForkStore {
    pool: Pool,
}

impl PostgresQuestionPoolForkStore {
    /// Binds the authenticated API pool to the narrow Pool-fork procedure.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        Ok(tx)
    }
}

#[async_trait]
impl QuestionPoolForkStore for PostgresQuestionPoolForkStore {
    async fn fork_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: QuestionPoolId,
        source_question_pool_id: QuestionPoolId,
    ) -> Result<CreatedQuestionPool, CreateQuestionPoolError> {
        let mut tx = self
            .begin(session_token_hash)
            .await
            .map_err(CreateQuestionPoolError::Store)?;
        let row = sqlx::query(
            "SELECT question_pool_id, question_pool_edit_number \
             FROM ple_api.fork_question_pool($1, $2)",
        )
        .bind(question_pool_id.as_str())
        .bind(source_question_pool_id.as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_create_question_pool_error)?;
        let returned_id = row
            .try_get::<String, _>("question_pool_id")
            .map_err(map_sqlx_error)
            .and_then(|value| {
                value.parse::<QuestionPoolId>().map_err(|_| {
                    StoreError::InvalidRecord(
                        "Question Pool fork returned an invalid public identity".to_owned(),
                    )
                })
            })
            .map_err(CreateQuestionPoolError::Store)?;
        if returned_id != question_pool_id {
            return Err(CreateQuestionPoolError::Store(StoreError::InvalidRecord(
                "Question Pool fork returned an unexpected public identity".to_owned(),
            )));
        }
        let edit_number = row
            .try_get::<i64, _>("question_pool_edit_number")
            .map_err(map_sqlx_error)
            .and_then(|value| {
                u64::try_from(value).map_err(|_| {
                    StoreError::InvalidRecord(
                        "Question Pool fork returned an invalid Edit Number".to_owned(),
                    )
                })
            })
            .map_err(CreateQuestionPoolError::Store)?;
        if edit_number != 1 {
            return Err(CreateQuestionPoolError::Store(StoreError::InvalidRecord(
                "Question Pool fork must return Edit Number 1".to_owned(),
            )));
        }
        tx.commit()
            .await
            .map_err(map_sqlx_error)
            .map_err(CreateQuestionPoolError::Store)?;
        Ok(CreatedQuestionPool {
            question_pool_id: returned_id,
            edit_number,
        })
    }
}
