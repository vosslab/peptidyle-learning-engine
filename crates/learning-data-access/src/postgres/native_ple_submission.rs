//! PostgreSQL adapter for the native PLE Question Submission transaction.

use async_trait::async_trait;
use sqlx::Row;
use sqlx::{Postgres, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::random_uuid::random_uuid_v4;
use crate::{
    AcceptNativePleSubmission, NativePleSubmissionStatus, NativePleSubmissionStore,
    ReadyQuestionAssetRendition, ResolvedNativePleSubmission, SessionTokenHash, StoreError,
    StudentQuestionSubmissionGradingState,
};

/// Binds the attested API pool to native PLE submission acceptance only.
#[derive(Clone)]
pub struct PostgresNativePleSubmissionStore {
    pool: Pool,
}

impl PostgresNativePleSubmissionStore {
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
impl NativePleSubmissionStore for PostgresNativePleSubmissionStore {
    async fn resolve_native_ple_submission(
        &self,
        token: SessionTokenHash,
        course_reference_number: u64,
        assignment_reference_number: u64,
        presentation_nonce: &str,
    ) -> Result<ResolvedNativePleSubmission, StoreError> {
        let course_reference_number = i64::try_from(course_reference_number).map_err(|_| {
            StoreError::InvalidRecord("Course Instance reference is invalid".to_string())
        })?;
        let assignment_reference_number =
            i64::try_from(assignment_reference_number).map_err(|_| {
                StoreError::InvalidRecord("Assignment reference is invalid".to_string())
            })?;
        let mut transaction = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT question_attempt_id, question_id, revision_number, source_object_id::text, \
             source_object_checksum, question_seed::text, presentation_nonce, presentation_checksum \
             FROM ple_api.resolve_live_demo_native_ple_submission($1, $2, $3)",
        )
        .bind(course_reference_number)
        .bind(assignment_reference_number)
        .bind(presentation_nonce)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Forbidden)?;
        let question_id: question_model::QuestionId = row
            .try_get::<String, _>("question_id")
            .map_err(map_sqlx_error)?
            .parse()
            .map_err(|_| StoreError::InvalidRecord("Question ID is invalid".to_string()))?;
        let revision_number = u32::try_from(
            row.try_get::<i32, _>("revision_number")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::InvalidRecord("Question Revision is invalid".to_string()))?;
        let rendition_rows = sqlx::query(
            "SELECT asset_id::text, question_asset_checksum, rendition_checksum, intrinsic_width, intrinsic_height \
             FROM ple_api.select_live_demo_ready_question_asset_renditions($1, $2)",
        )
        .bind(question_id.to_string())
        .bind(i32::try_from(revision_number).map_err(|_| {
            StoreError::InvalidRecord("Question Revision is invalid".to_string())
        })?)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let question_asset_renditions = rendition_rows
            .into_iter()
            .map(|row| {
                let asset_id = row
                    .try_get::<String, _>("asset_id")
                    .map_err(map_sqlx_error)?;
                Ok(ReadyQuestionAssetRendition {
                    question_asset: uuid::Uuid::parse_str(&asset_id)
                        .map(question_model::QuestionAssetId::from_uuid)
                        .map_err(|_| {
                            StoreError::InvalidRecord("Question Asset ID is invalid".to_string())
                        })?,
                    question_asset_checksum: row
                        .try_get("question_asset_checksum")
                        .map_err(map_sqlx_error)?,
                    rendition_checksum: row
                        .try_get("rendition_checksum")
                        .map_err(map_sqlx_error)?,
                    intrinsic_width: u32::try_from(
                        row.try_get::<i32, _>("intrinsic_width")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question Asset width is invalid".to_string())
                    })?,
                    intrinsic_height: u32::try_from(
                        row.try_get::<i32, _>("intrinsic_height")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question Asset height is invalid".to_string())
                    })?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let resolved = ResolvedNativePleSubmission {
            question_attempt: question_model::QuestionAttemptId::from_uuid(
                row.try_get("question_attempt_id").map_err(map_sqlx_error)?,
            ),
            question_id,
            revision_number,
            source_object_id: row.try_get("source_object_id").map_err(map_sqlx_error)?,
            source_object_checksum: row
                .try_get("source_object_checksum")
                .map_err(map_sqlx_error)?,
            question_seed: row
                .try_get::<String, _>("question_seed")
                .map_err(map_sqlx_error)?
                .parse()
                .map_err(|_| StoreError::InvalidRecord("Question Seed is invalid".to_string()))?,
            presentation_nonce: row.try_get("presentation_nonce").map_err(map_sqlx_error)?,
            presentation_checksum: row
                .try_get("presentation_checksum")
                .map_err(map_sqlx_error)?,
            question_asset_renditions,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(resolved)
    }

    async fn accept_native_ple_submission(
        &self,
        token: SessionTokenHash,
        submission: AcceptNativePleSubmission,
    ) -> Result<StudentQuestionSubmissionGradingState, StoreError> {
        let submission_id = random_uuid_v4(|_| {
            StoreError::Unavailable("Question Submission ID randomness unavailable".into())
        })?;
        let grading_id = random_uuid_v4(|_| {
            StoreError::Unavailable("Question Submission Grading ID randomness unavailable".into())
        })?;
        let job_id = random_uuid_v4(|_| {
            StoreError::Unavailable(
                "Question Submission grading Job ID randomness unavailable".into(),
            )
        })?;
        let mut transaction = self.begin(token).await?;
        let grading_state: String = sqlx::query_scalar(
            "SELECT ple_api.accept_live_demo_native_ple_submission($1, $2, $3, $4, $5)",
        )
        .bind(submission.question_attempt.as_uuid())
        .bind(submission.student_response)
        .bind(submission_id)
        .bind(grading_id)
        .bind(job_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = match grading_state.as_str() {
            "pending" => StudentQuestionSubmissionGradingState::Pending,
            _ => {
                return Err(StoreError::InvalidRecord(
                    "Question Submission grading state is invalid".to_string(),
                ));
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn native_ple_submission_status(
        &self,
        token: SessionTokenHash,
        course_reference_number: u64,
        assignment_reference_number: u64,
        presentation_nonce: &str,
    ) -> Result<NativePleSubmissionStatus, StoreError> {
        let course = i64::try_from(course_reference_number).map_err(|_| {
            StoreError::InvalidRecord("Course Instance reference is invalid".to_string())
        })?;
        let assignment = i64::try_from(assignment_reference_number).map_err(|_| {
            StoreError::InvalidRecord("Assignment reference is invalid".to_string())
        })?;
        let mut transaction = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT presentation_nonce, grading_state \
             FROM ple_api.read_live_demo_native_ple_submission_status($1, $2, $3)",
        )
        .bind(course)
        .bind(assignment)
        .bind(presentation_nonce)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Forbidden)?;
        let grading_state = match row
            .try_get::<String, _>("grading_state")
            .map_err(map_sqlx_error)?
            .as_str()
        {
            "pending" => StudentQuestionSubmissionGradingState::Pending,
            "instructor_attention" => StudentQuestionSubmissionGradingState::InstructorAttention,
            "graded" => StudentQuestionSubmissionGradingState::Graded,
            _ => {
                return Err(StoreError::InvalidRecord(
                    "Question Submission grading state is invalid".to_string(),
                ));
            }
        };
        let result = NativePleSubmissionStatus {
            presentation_nonce: row.try_get("presentation_nonce").map_err(map_sqlx_error)?,
            grading_state,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}
