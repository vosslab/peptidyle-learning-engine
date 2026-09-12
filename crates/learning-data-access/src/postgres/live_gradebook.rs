//! PostgreSQL adapter for the focused Course Gradebook read boundary.

use async_trait::async_trait;
use question_model::{AssignmentAttemptCompletion, AssignmentReference, CourseInstanceReference};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    CourseGradebook, CourseGradebookStore, CourseGradebookStudentWork, SessionTokenHash, StoreError,
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
            "SELECT course_reference_number, roster_id, assignment_reference_number, \
             assignment_attempt_completion, graded_question_count, question_count, \
             points_earned, points_possible \
             FROM ple_api.read_course_gradebook($1)",
        )
        .bind(i64::from(course.number()))
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(first) = rows.first() else {
            return Err(StoreError::NotFound);
        };
        let returned_course = reference(
            first
                .try_get("course_reference_number")
                .map_err(map_sqlx_error)?,
            "Course Reference",
            CourseInstanceReference::new,
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
    let assignment_reference = reference(
        row.try_get("assignment_reference_number")
            .map_err(map_sqlx_error)?,
        "Assignment Reference",
        AssignmentReference::new,
    )?;
    let assignment_attempt_completion = match row
        .try_get::<Option<String>, _>("assignment_attempt_completion")
        .map_err(map_sqlx_error)?
        .as_deref()
    {
        None => None,
        Some("in_progress") => Some(AssignmentAttemptCompletion::InProgress),
        Some("completed") => Some(AssignmentAttemptCompletion::Completed),
        Some(_) => return Err(invalid("Assignment Attempt Completion")),
    };
    let graded_question_count = u32::try_from(
        row.try_get::<i64, _>("graded_question_count")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| invalid("graded Question count"))?;
    let question_count = u32::try_from(
        row.try_get::<i64, _>("question_count")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| invalid("Question count"))?;
    let points_earned = finite_nonnegative(
        row.try_get("points_earned").map_err(map_sqlx_error)?,
        "points earned",
    )?;
    let points_possible = finite_nonnegative(
        row.try_get("points_possible").map_err(map_sqlx_error)?,
        "points possible",
    )?;
    if question_count == 0
        || graded_question_count > question_count
        || points_earned > points_possible
        || (assignment_attempt_completion.is_none()
            && (graded_question_count != 0 || points_earned != 0.0 || points_possible != 0.0))
    {
        return Err(invalid("Gradebook point ordering"));
    }
    Ok(Some(CourseGradebookStudentWork {
        roster_id,
        assignment_reference,
        assignment_attempt_completion,
        graded_question_count,
        question_count,
        points_earned,
        points_possible,
    }))
}

fn reference<T>(
    value: i64,
    label: &str,
    build: impl FnOnce(u64) -> Option<T>,
) -> Result<T, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(build)
        .ok_or_else(|| invalid(label))
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
