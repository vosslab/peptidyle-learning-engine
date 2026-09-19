//! Session-installed protected recovery with no external renderer or grader calls.

use async_trait::async_trait;
use question_model::{AssessmentAttemptId, CourseInstanceId};
use serde::{Serialize, de::DeserializeOwned};
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};

use super::{Pool, connection::map_sqlx_error};
use crate::archived_student_work_recovery::{AttemptFacts, RetainedQuestion, Submission};
use crate::{
    ArchivedStudentWorkRecoveryStore, RecoveredAttempt, RecoveredQuestion, RecoverySummary,
    SessionTokenHash, StoreError,
};

/// Uses only protected recovery wrappers under the originating session.
#[derive(Clone)]
pub struct PostgresArchivedStudentWorkRecoveryStore {
    pool: Pool,
}

impl PostgresArchivedStudentWorkRecoveryStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(&self, hash: SessionTokenHash) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(hash.to_string())
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
impl ArchivedStudentWorkRecoveryStore for PostgresArchivedStudentWorkRecoveryStore {
    async fn select_retained_work(
        &self,
        session: SessionTokenHash,
        course: CourseInstanceId,
        after: Option<AssessmentAttemptId>,
    ) -> Result<Vec<RecoverySummary>, StoreError> {
        let mut tx = self.begin(session).await?;
        // ASVS 1.2.4/8.3.1: parameterized protected wrapper repeats exact authority.
        let rows = sqlx::query(
            "SELECT course_instance_id, roster_id, assessment_id, assessment_title, \
             assessment_attempt_id, assessment_attempt_number, started_at::text, \
             submitted_at::text, student_data_archived_at::text, delete_due_at::text \
             FROM ple_api.select_archived_assessment_attempts_for_recovery($1,$2,101)",
        )
        .bind(course.as_string())
        .bind(after.map(|id| id.as_uuid()))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let result = rows.iter().map(summary).collect::<Result<Vec<_>, _>>()?;
        if result.iter().any(|r| r.course != course) {
            return Err(invalid());
        }
        // ASVS 2.3.3: release retention lock before HTTP output or browser wait.
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn recover_retained_work(
        &self,
        session: SessionTokenHash,
        course: CourseInstanceId,
        attempt: AssessmentAttemptId,
    ) -> Result<RecoveredAttempt, StoreError> {
        let mut tx = self.begin(session).await?;
        // Explicit columns exclude internal Student Record UUID and all Account fields.
        let row = sqlx::query(
            "SELECT course_instance_id, roster_id, assessment_id, \
             assessment_attempt_id, assessment_attempt_number, started_at::text, \
             expires_at::text, student_data_archived_at::text, delete_due_at::text, \
             attempt_facts, submission, questions \
             FROM ple_api.read_archived_assessment_attempt_for_recovery($1,$2)",
        )
        .bind(course.as_string())
        .bind(attempt.as_uuid())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let facts: AttemptFacts = document(&row, "attempt_facts")?;
        let submission: Option<Submission> = document(&row, "submission")?;
        let questions: Vec<RetainedQuestion> = document(&row, "questions")?;
        let mut result = RecoveredAttempt {
            course: get::<String>(&row, "course_instance_id")?
                .parse()
                .map_err(|_| invalid())?,
            roster_id: get(&row, "roster_id")?,
            assessment: get::<String>(&row, "assessment_id")?
                .parse()
                .map_err(|_| invalid())?,
            assessment_attempt: assessment_attempt_id(&row)?,
            assessment_attempt_number: positive(&row, "assessment_attempt_number")?,
            started_at: get(&row, "started_at")?,
            expires_at: get(&row, "expires_at")?,
            student_data_archived_at: get(&row, "student_data_archived_at")?,
            delete_due_at: get(&row, "delete_due_at")?,
            assessment_title: facts.assessment_title.clone(),
            attempt_facts_text: text(&facts)?,
            submission_text: submission.as_ref().map(text).transpose()?,
            questions: Vec::with_capacity(questions.len()),
        };
        if result.course != course || result.assessment_attempt != attempt {
            return Err(invalid());
        }
        for question in questions {
            if question.delivery.revision_number == 0
                || result.questions.last().is_some_and(|previous| {
                    previous.issued_position >= question.delivery.issued_position
                })
            {
                return Err(invalid());
            }
            let response_text =
                |response: &crate::archived_student_work_recovery::RetainedResponse| {
                    // Existing StudentResponse preserves opaque BackendOwned bytes as bounded
                    // canonical base64. Never interpret or forward these bytes upstream here.
                    text(&ResponseEvidence {
                        student_response: &response.student_response,
                        saved_at: &response.saved_at,
                        finalized_at: &response.finalized_at,
                    })
                };
            result.questions.push(RecoveredQuestion {
                issued_position: question.delivery.issued_position,
                question_id: question.delivery.question_id.clone(),
                revision_number: question.delivery.revision_number,
                delivery_text: text(&question.delivery)?,
                pool_text: question.pool.as_ref().map(text).transpose()?,
                attempt_text: question.attempt.as_ref().map(text).transpose()?,
                presentation_text: question.presentation.as_ref().map(text).transpose()?,
                reproduction_text: question.reproduction.as_ref().map(text).transpose()?,
                // Retained renderer HTML is evidence TEXT only. Never render or call upstream.
                backend_document_text: question
                    .presentation
                    .as_ref()
                    .and_then(|p| p.backend_document.clone()),
                saved_response_text: question
                    .saved_response
                    .as_ref()
                    .map(response_text)
                    .transpose()?,
                finalized_response_text: question
                    .finalized_response
                    .as_ref()
                    .map(response_text)
                    .transpose()?,
                grading_text: question.grading.as_ref().map(text).transpose()?,
                unavailable_evidence: vec![
                    "Raw author source is not included in this minimized recovery projection"
                        .into(),
                ],
            });
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

#[derive(Serialize)]
struct ResponseEvidence<'a> {
    student_response: &'a question_model::response::StudentResponse,
    saved_at: &'a Option<String>,
    finalized_at: &'a Option<String>,
}

fn summary(row: &PgRow) -> Result<RecoverySummary, StoreError> {
    Ok(RecoverySummary {
        course: get::<String>(row, "course_instance_id")?
            .parse()
            .map_err(|_| invalid())?,
        roster_id: get(row, "roster_id")?,
        assessment: get::<String>(row, "assessment_id")?
            .parse()
            .map_err(|_| invalid())?,
        assessment_title: get(row, "assessment_title")?,
        assessment_attempt: assessment_attempt_id(row)?,
        assessment_attempt_number: positive(row, "assessment_attempt_number")?,
        started_at: get(row, "started_at")?,
        submitted_at: get(row, "submitted_at")?,
        student_data_archived_at: get(row, "student_data_archived_at")?,
        delete_due_at: get(row, "delete_due_at")?,
    })
}

fn assessment_attempt_id(row: &PgRow) -> Result<AssessmentAttemptId, StoreError> {
    Ok(AssessmentAttemptId::from_uuid(get(
        row,
        "assessment_attempt_id",
    )?))
}
fn positive(row: &PgRow, name: &str) -> Result<u32, StoreError> {
    u32::try_from(get::<i32>(row, name)?)
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(invalid)
}
fn get<T>(row: &PgRow, name: &str) -> Result<T, StoreError>
where
    T: for<'r> sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres>,
{
    row.try_get(name).map_err(map_sqlx_error)
}
fn document<T: DeserializeOwned>(row: &PgRow, name: &str) -> Result<T, StoreError> {
    let value: Option<serde_json::Value> = get(row, name)?;
    serde_json::from_value(value.unwrap_or(serde_json::Value::Null)).map_err(|_| invalid())
}
fn text<T: Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string_pretty(value).map_err(|_| invalid())
}
fn invalid() -> StoreError {
    StoreError::InvalidRecord("Retained Work evidence is invalid".into())
}
