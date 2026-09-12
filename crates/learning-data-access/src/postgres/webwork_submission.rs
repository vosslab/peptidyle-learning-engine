//! PostgreSQL adapter for the WeBWorK submission procedures.

use async_trait::async_trait;
use sqlx::Row;

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AcceptNativePleSubmission, ResolvedWebworkSubmission, SessionTokenHash, StoreError,
    WebworkSubmissionStore,
};

#[derive(Clone)]
pub struct PostgresWebworkSubmissionStore {
    pool: Pool,
}

impl PostgresWebworkSubmissionStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, StoreError> {
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
impl WebworkSubmissionStore for PostgresWebworkSubmissionStore {
    async fn resolve_webwork_submission(
        &self,
        token: SessionTokenHash,
        course: u64,
        assignment: u64,
        nonce: &str,
    ) -> Result<ResolvedWebworkSubmission, StoreError> {
        let course = i64::try_from(course).map_err(|_| {
            StoreError::InvalidRecord("Course Instance reference is invalid".into())
        })?;
        let assignment = i64::try_from(assignment)
            .map_err(|_| StoreError::InvalidRecord("Assignment reference is invalid".into()))?;
        let mut transaction = self.begin(token).await?;
        let row = sqlx::query("SELECT question_attempt_id, question_id, revision_number, source_object_id::text, source_object_checksum, webwork_pg_path, question_seed::text AS question_seed, presentation_nonce, presentation_checksum, replay_details FROM ple_api.resolve_webwork_submission($1, $2, $3)")
            .bind(course).bind(assignment).bind(nonce).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?.ok_or(StoreError::Forbidden)?;
        let question_id = row
            .try_get::<String, _>("question_id")
            .map_err(map_sqlx_error)?
            .parse()
            .map_err(|_| StoreError::InvalidRecord("WeBWorK Question ID is invalid".into()))?;
        let resolved = ResolvedWebworkSubmission {
            question_attempt: question_model::QuestionAttemptId::from_uuid(
                row.try_get("question_attempt_id").map_err(map_sqlx_error)?,
            ),
            question_id,
            revision_number: u32::try_from(
                row.try_get::<i32, _>("revision_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::InvalidRecord("WeBWorK revision is invalid".into()))?,
            source_object_id: row.try_get("source_object_id").map_err(map_sqlx_error)?,
            source_object_checksum: row
                .try_get("source_object_checksum")
                .map_err(map_sqlx_error)?,
            webwork_pg_path: row.try_get("webwork_pg_path").map_err(map_sqlx_error)?,
            question_seed: row
                .try_get::<String, _>("question_seed")
                .map_err(map_sqlx_error)?
                .parse()
                .map_err(|_| StoreError::InvalidRecord("WeBWorK seed is invalid".into()))?,
            presentation_nonce: row.try_get("presentation_nonce").map_err(map_sqlx_error)?,
            presentation_checksum: row
                .try_get("presentation_checksum")
                .map_err(map_sqlx_error)?,
            replay_details: row.try_get("replay_details").map_err(map_sqlx_error)?,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(resolved)
    }

    async fn accept_webwork_submission(
        &self,
        token: SessionTokenHash,
        submission: AcceptNativePleSubmission,
    ) -> Result<(), StoreError> {
        let submission_id = crate::random_uuid::random_uuid_v4(|_| {
            StoreError::Unavailable("Question Submission ID randomness unavailable".into())
        })?;
        let grading_id = crate::random_uuid::random_uuid_v4(|_| {
            StoreError::Unavailable("Question Submission grading ID randomness unavailable".into())
        })?;
        let job_id = crate::random_uuid::random_uuid_v4(|_| {
            StoreError::Unavailable(
                "Question Submission grading Job ID randomness unavailable".into(),
            )
        })?;
        let mut transaction = self.begin(token).await?;
        sqlx::query("SELECT ple_api.accept_webwork_submission($1, $2, $3, $4, $5)")
            .bind(submission.question_attempt.as_uuid())
            .bind(submission.student_response)
            .bind(submission_id)
            .bind(grading_id)
            .bind(job_id)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }
}
