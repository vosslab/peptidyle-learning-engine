//! PostgreSQL implementation of the authenticated Question Star boundary.

use async_trait::async_trait;
use question_model::QuestionId;
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    QuestionStarProjection, QuestionStarStore, QuestionStarredInstructor, SessionTokenHash,
    StoreError,
};

/// PostgreSQL Store for self-only Instructor Star state and public counts.
#[derive(Clone)]
pub struct PostgresQuestionStarStore {
    pool: Pool,
}

impl PostgresQuestionStarStore {
    /// Binds the attested API pool to the Star procedures.
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

    fn projection(row: &sqlx::postgres::PgRow) -> Result<QuestionStarProjection, StoreError> {
        let count: i64 = row.try_get("star_count").map_err(map_sqlx_error)?;
        Ok(QuestionStarProjection {
            viewer_has_starred: row.try_get("viewer_has_starred").map_err(map_sqlx_error)?,
            star_count: u64::try_from(count)
                .map_err(|_| StoreError::InvalidRecord("Question Star count".to_owned()))?,
            starred_instructors: row
                .try_get::<Vec<String>, _>("starred_instructor_display_names")
                .map_err(map_sqlx_error)?
                .into_iter()
                .map(validated_display_name)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    async fn read_in(
        tx: &mut Transaction<'_, Postgres>,
        question_id: &QuestionId,
    ) -> Result<QuestionStarProjection, StoreError> {
        // ASVS 8.2.1--8.3.1: this API procedure derives the subject from the
        // installed session; it accepts no Account ID or client role claim.
        let row = sqlx::query("SELECT * FROM ple_api.read_current_question_star($1)")
            .bind(question_id.as_str())
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        Self::projection(&row)
    }
}

fn validated_display_name(value: String) -> Result<QuestionStarredInstructor, StoreError> {
    if value != value.trim()
        || value.is_empty()
        // C852's immutable vetting boundary owns the 1..=200 contract.
        // Keep this defensive decoder exactly aligned so a valid vetted name
        // cannot turn an otherwise authorized Star read into a server error.
        || value.chars().count() > 200
        || value.chars().any(char::is_control)
    {
        return Err(StoreError::InvalidRecord(
            "Question Star Instructor display identity".to_owned(),
        ));
    }
    Ok(QuestionStarredInstructor {
        display_name: value,
    })
}

#[async_trait]
impl QuestionStarStore for PostgresQuestionStarStore {
    async fn question_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
    ) -> Result<QuestionStarProjection, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        let projection = Self::read_in(&mut tx, question_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }

    async fn set_current_question_star(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
        starred: bool,
    ) -> Result<QuestionStarProjection, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        // The SQL procedure is the idempotent, database-authorized mutation;
        // the subsequent projection is from the same transaction snapshot.
        sqlx::query("SELECT ple_api.set_current_question_star($1, $2)")
            .bind(question_id.as_str())
            .bind(starred)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let projection = Self::read_in(&mut tx, question_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }
}
