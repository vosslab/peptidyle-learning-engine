//! PostgreSQL adapter for the focused Course Gradebook read boundary.

use async_trait::async_trait;
use question_model::{AssessmentAttemptCompletion, AssessmentReference, CourseInstanceReference};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    CourseGradebook, CourseGradebookStore, CourseGradebookStudentWork, LiveAssessmentAttemptScore,
    SessionTokenHash, StoreError,
};

#[derive(Clone)]
pub struct PostgresCourseGradebookStore {
    pool: Pool,
}

impl PostgresCourseGradebookStore {
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
        if sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .is_none()
        {
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
impl CourseGradebookStore for PostgresCourseGradebookStore {
    async fn course_gradebook(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<CourseGradebook, StoreError> {
        let mut transaction = self.begin(token).await?;
        let rows = sqlx::query(
            "SELECT course_public_reference, roster_id, assessment_reference_number, \
             assessment_attempt_completion, expired_submitting, \
             points_earned, points_possible \
             FROM ple_api.read_course_gradebook($1)",
        )
        .bind(course.as_string())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(first) = rows.first() else {
            return Err(StoreError::NotFound);
        };
        let returned_course = course_reference(
            first
                .try_get("course_public_reference")
                .map_err(map_sqlx_error)?,
        )?;
        if returned_course != course {
            return Err(StoreError::InvalidRecord(
                "database returned a different Course Reference".to_string(),
            ));
        }
        let student_work = rows
            .iter()
            .filter_map(|row| decode_row(row).transpose())
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseGradebook {
            course_reference: returned_course,
            student_work,
        })
    }
}

fn decode_row(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<CourseGradebookStudentWork>, StoreError> {
    let Some(roster_id) = row.try_get("roster_id").map_err(map_sqlx_error)? else {
        return Ok(None);
    };
    let assessment_reference = assessment_reference(
        row.try_get("assessment_reference_number")
            .map_err(map_sqlx_error)?,
    )?;
    let assessment_attempt_completion = match row
        .try_get::<Option<String>, _>("assessment_attempt_completion")
        .map_err(map_sqlx_error)?
        .as_deref()
    {
        None => None,
        Some("in_progress") => Some(AssessmentAttemptCompletion::InProgress),
        Some("completed") => Some(AssessmentAttemptCompletion::Completed),
        Some(_) => return Err(invalid("Assessment Attempt Completion")),
    };
    let expired_submitting: bool = row.try_get("expired_submitting").map_err(map_sqlx_error)?;
    let score = match (
        row.try_get::<Option<f64>, _>("points_earned")
            .map_err(map_sqlx_error)?,
        row.try_get::<Option<f64>, _>("points_possible")
            .map_err(map_sqlx_error)?,
    ) {
        (Some(points_earned), Some(points_possible)) => Some(LiveAssessmentAttemptScore {
            points_earned: finite_nonnegative(points_earned, "points earned")?,
            points_possible: finite_nonnegative(points_possible, "points possible")?,
        }),
        (None, None) => None,
        _ => return Err(invalid("partial Gradebook score")),
    };
    if score
        .as_ref()
        .is_some_and(|value| value.points_earned > value.points_possible)
        || (assessment_attempt_completion.is_none() && score.is_some())
        || (expired_submitting
            && (assessment_attempt_completion != Some(AssessmentAttemptCompletion::InProgress)
                || score.is_some()))
    {
        return Err(invalid("Gradebook point ordering"));
    }
    Ok(Some(CourseGradebookStudentWork {
        roster_id,
        assessment_reference,
        assessment_attempt_completion,
        expired_submitting,
        score,
    }))
}

fn course_reference(value: String) -> Result<CourseInstanceReference, StoreError> {
    CourseInstanceReference::new(value).map_err(|_| invalid("Course Reference"))
}

fn assessment_reference(value: String) -> Result<AssessmentReference, StoreError> {
    AssessmentReference::new(value).map_err(|_| invalid("Assessment Reference"))
}

fn finite_nonnegative(value: f64, label: &str) -> Result<f64, StoreError> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(invalid(label))
    }
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}
