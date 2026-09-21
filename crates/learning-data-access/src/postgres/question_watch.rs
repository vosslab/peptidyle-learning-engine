//! PostgreSQL implementation of the private authenticated Question Watch boundary.

use async_trait::async_trait;
use question_model::PublishedQuestionId;
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{QuestionWatchProjection, QuestionWatchStore, SessionTokenHash, StoreError};

/// PostgreSQL Store for self-only Instructor Watch state.
#[derive(Clone)]
pub struct PostgresQuestionWatchStore {
    pool: Pool,
}

impl PostgresQuestionWatchStore {
    /// Binds the attested API pool to the private Watch procedures.
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

    fn projection(row: &sqlx::postgres::PgRow) -> Result<QuestionWatchProjection, StoreError> {
        Ok(QuestionWatchProjection {
            watching: row.try_get("watching").map_err(map_sqlx_error)?,
        })
    }

    async fn read_in(
        tx: &mut Transaction<'_, Postgres>,
        question_id: &PublishedQuestionId,
    ) -> Result<QuestionWatchProjection, StoreError> {
        // The SQL function derives and validates the actor from the installed
        // session. Its closed row has only the actor's boolean Watch state.
        let row = sqlx::query("SELECT * FROM ple_api.read_current_question_watch($1)")
            .bind(question_id.as_str())
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        Self::projection(&row)
    }
}

#[async_trait]
impl QuestionWatchStore for PostgresQuestionWatchStore {
    async fn question_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
    ) -> Result<QuestionWatchProjection, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        let projection = Self::read_in(&mut tx, question_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }

    async fn set_current_question_watch(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
        watching: bool,
    ) -> Result<QuestionWatchProjection, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        sqlx::query("SELECT ple_api.set_current_question_watch($1, $2)")
            .bind(question_id.as_str())
            .bind(watching)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let projection = Self::read_in(&mut tx, question_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }
}
