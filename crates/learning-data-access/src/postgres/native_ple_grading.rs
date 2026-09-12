//! PostgreSQL adapter for the native-PLE-only lease and commit procedures.

use async_trait::async_trait;
use sqlx::Row;

use super::{Pool, connection::map_sqlx_error};
use crate::{NativePleGradingJobLease, NativePleGradingStore, StoreError};

#[derive(Clone)]
pub struct PostgresNativePleGradingStore {
    pool: Pool,
}

impl PostgresNativePleGradingStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NativePleGradingStore for PostgresNativePleGradingStore {
    async fn claim_native_ple_grading_job(
        &self,
        lease_expires_at_unix_millis: i64,
    ) -> Result<Option<NativePleGradingJobLease>, StoreError> {
        let lease_token = crate::random_uuid::random_uuid_v4(|_| {
            StoreError::Unavailable("native PLE grading lease randomness unavailable".into())
        })?;
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_native_ple_grading_worker")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let row = sqlx::query(
            "SELECT job_id, question_attempt_id, question_id, revision_number, source_object_id, \
                    source_object_checksum, question_seed::text AS question_seed, student_response \
             FROM ple_api.claim_native_ple_grading_job(\
                 $1, to_timestamp($2::double precision / 1000.0)\
             )",
        )
        .bind(lease_token)
        .bind(lease_expires_at_unix_millis)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.map(|row| {
            let question_id: question_model::QuestionId = row
                .try_get::<String, _>("question_id")
                .map_err(map_sqlx_error)?
                .parse()
                .map_err(|_| {
                    StoreError::InvalidRecord("native PLE Question ID is invalid".into())
                })?;
            Ok(NativePleGradingJobLease {
                job_id: row.try_get("job_id").map_err(map_sqlx_error)?,
                lease_token,
                question_attempt: question_model::QuestionAttemptId::from_uuid(
                    row.try_get("question_attempt_id").map_err(map_sqlx_error)?,
                ),
                question_id,
                revision_number: u32::try_from(
                    row.try_get::<i32, _>("revision_number")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|_| StoreError::InvalidRecord("native PLE revision is invalid".into()))?,
                source_object_id: row
                    .try_get::<uuid::Uuid, _>("source_object_id")
                    .map_err(map_sqlx_error)?
                    .to_string(),
                source_object_checksum: row
                    .try_get("source_object_checksum")
                    .map_err(map_sqlx_error)?,
                question_seed: row
                    .try_get::<String, _>("question_seed")
                    .map_err(map_sqlx_error)?
                    .parse()
                    .map_err(|_| StoreError::InvalidRecord("native PLE seed is invalid".into()))?,
                student_response: row.try_get("student_response").map_err(map_sqlx_error)?,
            })
        })
        .transpose()
    }

    async fn commit_native_ple_grading(
        &self,
        lease: &NativePleGradingJobLease,
        correct: bool,
        normalized_credit: f64,
        committed_at_unix_millis: i64,
    ) -> Result<(), StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_native_ple_grading_worker")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query("SELECT ple_api.commit_native_ple_grading($1, $2, $3, $4, to_timestamp($5::double precision / 1000.0))")
            .bind(lease.job_id).bind(lease.lease_token).bind(correct).bind(normalized_credit).bind(committed_at_unix_millis).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }
}
