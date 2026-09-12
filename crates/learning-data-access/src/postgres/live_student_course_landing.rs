//! PostgreSQL adapter for the Student Course landing projections.

use async_trait::async_trait;
use question_model::{AssignmentAttemptCompletion, AssignmentReference, CourseInstanceReference};
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    LiveAssignmentAttemptScore, LiveStudentAssignmentLandingSummary,
    LiveStudentCourseInvitationSummary, LiveStudentCourseLandingStore,
    LiveStudentCourseLandingSummary, SessionTokenHash, StoreError,
};

/// PostgreSQL Store for the active Student Course landing.
#[derive(Clone)]
pub struct PostgresLiveStudentCourseLandingStore {
    pool: Pool,
}

impl PostgresLiveStudentCourseLandingStore {
    /// Binds the attested API pool to the Student landing procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token_hash.to_string())
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
impl LiveStudentCourseLandingStore for PostgresLiveStudentCourseLandingStore {
    async fn list_live_student_courses(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<LiveStudentCourseLandingSummary>, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows = sqlx::query(
            "SELECT course_reference_number, course_short_name, course_long_name \
             FROM ple_api.list_live_student_course_landing()",
        )
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let courses = rows
            .iter()
            .map(decode_course)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(courses)
    }

    async fn list_pending_live_student_course_invitations(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<LiveStudentCourseInvitationSummary>, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows = sqlx::query(
            "SELECT course_reference_number, course_short_name, course_long_name \
             FROM ple_api.list_pending_student_course_invitations()",
        )
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let invitations = rows
            .iter()
            .map(decode_invitation)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(invitations)
    }

    async fn list_released_live_student_assignments(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<LiveStudentAssignmentLandingSummary>, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows = sqlx::query(
            "SELECT assignment_reference_number, assignment_title, assignment_attempt_number, \
             assignment_attempt_completion, graded_question_count, question_count, \
             points_earned, points_possible \
             FROM ple_api.list_released_live_student_assignments($1)",
        )
        .bind(i64::from(course.number()))
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let assignments = rows
            .iter()
            .map(decode_assignment)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(assignments)
    }
}

fn decode_course(
    row: &sqlx::postgres::PgRow,
) -> Result<LiveStudentCourseLandingSummary, StoreError> {
    Ok(LiveStudentCourseLandingSummary {
        course: reference(
            row.try_get::<i64, _>("course_reference_number")
                .map_err(map_sqlx_error)?,
            CourseInstanceReference::new,
            "Course Instance reference",
        )?,
        short_name: name(
            row.try_get("course_short_name").map_err(map_sqlx_error)?,
            "Course short name",
        )?,
        long_name: name(
            row.try_get("course_long_name").map_err(map_sqlx_error)?,
            "Course long name",
        )?,
    })
}

fn decode_assignment(
    row: &sqlx::postgres::PgRow,
) -> Result<LiveStudentAssignmentLandingSummary, StoreError> {
    let assignment_attempt_number = row
        .try_get::<Option<i32>, _>("assignment_attempt_number")
        .map_err(map_sqlx_error)?
        .map(u32::try_from)
        .transpose()
        .map_err(|_| invalid("Assignment Attempt number"))?;
    let assignment_attempt_completion = match row
        .try_get::<Option<String>, _>("assignment_attempt_completion")
        .map_err(map_sqlx_error)?
        .as_deref()
    {
        None => None,
        Some("in_progress") => Some(AssignmentAttemptCompletion::InProgress),
        Some("completed") => Some(AssignmentAttemptCompletion::Completed),
        Some(_) => return Err(invalid("Assignment Attempt completion")),
    };
    let graded_question_count = count(row, "graded_question_count")?;
    let question_count = count(row, "question_count")?;
    let points_earned = optional_finite_nonnegative(row, "points_earned")?;
    let points_possible = optional_finite_nonnegative(row, "points_possible")?;
    let score = match (points_earned, points_possible) {
        (Some(points_earned), Some(points_possible)) if points_earned <= points_possible => {
            Some(LiveAssignmentAttemptScore {
                points_earned,
                points_possible,
            })
        }
        (None, None) => None,
        _ => return Err(invalid("Assignment score")),
    };
    if question_count == 0
        || graded_question_count > question_count
        || (score.is_some() && graded_question_count != question_count)
        || (assignment_attempt_completion.is_none()
            && (assignment_attempt_number.is_some()
                || graded_question_count != 0
                || score.is_some()))
        || (assignment_attempt_completion.is_some() && assignment_attempt_number.is_none())
    {
        return Err(invalid("Assignment progress"));
    }
    Ok(LiveStudentAssignmentLandingSummary {
        assignment: reference(
            row.try_get::<i64, _>("assignment_reference_number")
                .map_err(map_sqlx_error)?,
            AssignmentReference::new,
            "Assignment reference",
        )?,
        title: row.try_get("assignment_title").map_err(map_sqlx_error)?,
        assignment_attempt_number,
        assignment_attempt_completion,
        graded_question_count,
        question_count,
        score,
    })
}

fn decode_invitation(
    row: &sqlx::postgres::PgRow,
) -> Result<LiveStudentCourseInvitationSummary, StoreError> {
    Ok(LiveStudentCourseInvitationSummary {
        course: reference(
            row.try_get::<i64, _>("course_reference_number")
                .map_err(map_sqlx_error)?,
            CourseInstanceReference::new,
            "Course Instance reference",
        )?,
        short_name: name(
            row.try_get("course_short_name").map_err(map_sqlx_error)?,
            "Course short name",
        )?,
        long_name: name(
            row.try_get("course_long_name").map_err(map_sqlx_error)?,
            "Course long name",
        )?,
    })
}

fn count(row: &sqlx::postgres::PgRow, column: &str) -> Result<u32, StoreError> {
    u32::try_from(row.try_get::<i64, _>(column).map_err(map_sqlx_error)?)
        .map_err(|_| invalid(column))
}

fn optional_finite_nonnegative(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<f64>, StoreError> {
    let value = row
        .try_get::<Option<f64>, _>(column)
        .map_err(map_sqlx_error)?;
    match value {
        Some(value) if value.is_finite() && value >= 0.0 => Ok(Some(value)),
        None => Ok(None),
        Some(_) => Err(invalid(column)),
    }
}

fn invalid(name: &str) -> StoreError {
    StoreError::InvalidRecord(format!("{name} is invalid"))
}

fn name(value: String, label: &str) -> Result<String, StoreError> {
    (value == value.trim() && !value.is_empty() && value.chars().count() <= 200)
        .then_some(value)
        .ok_or_else(|| invalid(label))
}

fn reference<T>(
    value: i64,
    build: impl FnOnce(u64) -> Option<T>,
    name: &str,
) -> Result<T, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(build)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{name} is invalid")))
}
