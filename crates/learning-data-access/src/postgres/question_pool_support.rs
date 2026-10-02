//! PostgreSQL read and replacement for one Question Pool's optional support texts.

use async_trait::async_trait;
use question_model::{QuestionPoolId, QuestionPoolMetadataEditNumber};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    QuestionPoolPleManagedSupport, QuestionPoolSupportStore, SessionTokenHash, StoreError,
};

/// PostgreSQL store for the Instructor Pool support command.
#[derive(Clone)]
pub struct PostgresQuestionPoolSupportStore {
    pool: Pool,
}

impl PostgresQuestionPoolSupportStore {
    /// Binds the attested API pool to the closed Pool support procedures.
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

fn support_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<QuestionPoolPleManagedSupport, StoreError> {
    let question_pool_id = row
        .try_get::<String, _>("question_pool_id")
        .map_err(map_sqlx_error)?
        .parse::<QuestionPoolId>()
        .map_err(|_| invalid("Question Pool support returned an invalid Pool ID"))?;
    let question_pool_metadata_edit_number = row
        .try_get::<i64, _>("question_pool_metadata_edit_number")
        .map_err(map_sqlx_error)
        .and_then(|value| {
            u64::try_from(value)
                .map_err(|_| invalid("Question Pool support returned an invalid Edit Number"))
        })
        .and_then(|value| {
            QuestionPoolMetadataEditNumber::new(value).map_err(|_| {
                invalid("Question Pool support returned an invalid metadata Edit Number")
            })
        })?;
    Ok(QuestionPoolPleManagedSupport {
        question_pool_id,
        question_pool_metadata_edit_number,
        hint: row.try_get("hint").map_err(map_sqlx_error)?,
        general_feedback: row.try_get("general_feedback").map_err(map_sqlx_error)?,
        worked_solution: row.try_get("worked_solution").map_err(map_sqlx_error)?,
    })
}

#[async_trait]
impl QuestionPoolSupportStore for PostgresQuestionPoolSupportStore {
    async fn read_question_pool_ple_managed_support(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: QuestionPoolId,
    ) -> Result<QuestionPoolPleManagedSupport, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        let row = sqlx::query(
            "SELECT question_pool_id, question_pool_metadata_edit_number, hint, general_feedback, \
             worked_solution FROM ple_api.read_question_pool_ple_managed_support($1)",
        )
        .bind(question_pool_id.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            return Err(StoreError::Forbidden);
        };
        let support = support_from_row(&row)?;
        support.validate()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(support)
    }

    async fn save_question_pool_ple_managed_support(
        &self,
        session_token_hash: SessionTokenHash,
        support: QuestionPoolPleManagedSupport,
    ) -> Result<QuestionPoolPleManagedSupport, StoreError> {
        support.validate()?;
        let mut tx = self.begin(session_token_hash).await?;
        // The procedure updates ple_data.question_pool only.
        let row = sqlx::query(
            "SELECT question_pool_id, question_pool_metadata_edit_number, hint, general_feedback, \
             worked_solution FROM ple_api.save_question_pool_ple_managed_support($1, $2, $3, $4, $5)",
        )
        .bind(support.question_pool_id.as_str())
        .bind(i64::try_from(support.question_pool_metadata_edit_number.get()).map_err(|_| {
            invalid("Question Pool support Edit Number is invalid")
        })?)
        .bind(support.hint)
        .bind(support.general_feedback)
        .bind(support.worked_solution)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            return Err(StoreError::Forbidden);
        };
        let saved = support_from_row(&row)?;
        saved.validate()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(saved)
    }
}

fn invalid(message: &str) -> StoreError {
    StoreError::InvalidRecord(message.to_owned())
}
