use async_trait::async_trait;

use super::{Pool, connection::map_sqlx_error};
use crate::{QuestionWatchNotificationStore, StoreError};

#[derive(Clone)]
pub struct PostgresQuestionWatchNotificationStore {
    pool: Pool,
}

impl PostgresQuestionWatchNotificationStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl QuestionWatchNotificationStore for PostgresQuestionWatchNotificationStore {
    async fn materialize_question_watch_notifications(
        &self,
        limit: u16,
    ) -> Result<u32, StoreError> {
        if limit == 0 || limit > 500 {
            return Err(StoreError::InvalidRecord(
                "Question Watch notification limit is invalid".to_owned(),
            ));
        }
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_assessment_attempt_expiry_worker")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let count: i32 =
            sqlx::query_scalar("SELECT ple_api.materialize_question_watch_notifications($1)")
                .bind(i32::from(limit))
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        u32::try_from(count).map_err(|_| {
            StoreError::InvalidRecord("Question Watch notification count is invalid".to_owned())
        })
    }
}
