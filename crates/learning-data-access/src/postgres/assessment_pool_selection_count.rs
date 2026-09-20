//! PostgreSQL adapter for count-only Assessment Pool entry editing.

use std::num::NonZeroU32;

use async_trait::async_trait;
use question_model::{
    AssessmentEditNumber, AssessmentEntryId, AssessmentQuestionPoolSelectionCountReceipt,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AssessmentPoolSelectionCountInput, AssessmentPoolSelectionCountStore, SessionTokenHash,
    StoreError,
};

#[derive(Clone)]
pub struct PostgresAssessmentPoolSelectionCountStore {
    pool: Pool,
}

impl PostgresAssessmentPoolSelectionCountStore {
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
impl AssessmentPoolSelectionCountStore for PostgresAssessmentPoolSelectionCountStore {
    async fn update_assessment_question_pool_selection_count(
        &self,
        session_token_hash: SessionTokenHash,
        input: AssessmentPoolSelectionCountInput,
    ) -> Result<AssessmentQuestionPoolSelectionCountReceipt, StoreError> {
        let expected_next_edit = input
            .expected_assessment_edit_number
            .checked_next()
            .ok_or_else(|| invalid("Assessment Edit Number successor"))?;
        let expected_edit = i64::try_from(input.expected_assessment_edit_number.value())
            .map_err(|_| invalid("Assessment Edit Number"))?;
        let selection_count = i32::try_from(input.selection_count.get())
            .map_err(|_| invalid("Question Pool selection count"))?;
        let mut transaction = self.begin(session_token_hash).await?;
        let row = sqlx::query(
            "SELECT * FROM ple_api.update_assessment_question_pool_selection_count(\
             $1, $2, $3, $4, $5)",
        )
        .bind(input.course_instance_id.as_string())
        .bind(input.assessment_id.as_string())
        .bind(input.assessment_entry_id.as_uuid())
        .bind(expected_edit)
        .bind(selection_count)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        let assessment_entry_id = AssessmentEntryId::from_uuid(
            row.try_get("assessment_entry_id").map_err(map_sqlx_error)?,
        );
        let stored_count = u32::try_from(
            row.try_get::<i32, _>("selection_count")
                .map_err(map_sqlx_error)?,
        )
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| invalid("Question Pool selection count"))?;
        let assessment_edit_number = AssessmentEditNumber::new(
            u64::try_from(
                row.try_get::<i64, _>("assessment_edit_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| invalid("Assessment Edit Number"))?,
        )
        .ok_or_else(|| invalid("Assessment Edit Number"))?;

        if assessment_entry_id != input.assessment_entry_id
            || stored_count != input.selection_count
            || assessment_edit_number != expected_next_edit
        {
            return Err(invalid("Assessment Pool selection count receipt"));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(AssessmentQuestionPoolSelectionCountReceipt {
            assessment_entry_id,
            selection_count: stored_count,
            assessment_edit_number,
        })
    }
}

fn invalid(field: &str) -> StoreError {
    StoreError::InvalidRecord(format!("stored {field} is invalid"))
}
