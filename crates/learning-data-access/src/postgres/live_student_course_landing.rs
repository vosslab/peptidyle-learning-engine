//! PostgreSQL adapter for the Student Course landing projections.

use async_trait::async_trait;
use question_model::{AssignmentReference, CourseInstanceReference};
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    LiveStudentAssignmentLandingSummary, LiveStudentCourseInvitationSummary,
    LiveStudentCourseLandingStore, LiveStudentCourseLandingSummary, SessionTokenHash, StoreError,
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
            "SELECT course_reference_number, course_title \
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
            "SELECT course_reference_number, course_title \
             FROM ple_api.list_pending_live_student_course_invitations()",
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
            "SELECT assignment_reference_number, assignment_title \
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
        title: row.try_get("course_title").map_err(map_sqlx_error)?,
    })
}

fn decode_assignment(
    row: &sqlx::postgres::PgRow,
) -> Result<LiveStudentAssignmentLandingSummary, StoreError> {
    Ok(LiveStudentAssignmentLandingSummary {
        assignment: reference(
            row.try_get::<i64, _>("assignment_reference_number")
                .map_err(map_sqlx_error)?,
            AssignmentReference::new,
            "Assignment reference",
        )?,
        title: row.try_get("assignment_title").map_err(map_sqlx_error)?,
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
        title: row.try_get("course_title").map_err(map_sqlx_error)?,
    })
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
