//! PostgreSQL adapter for the minimal Student Assignment Attempt route context.

use std::str::FromStr;

use question_model::{
    AssignmentAttemptReference, AssignmentReference, CourseInstanceReference, CourseTheme,
};
use sqlx::Row;

use super::{PostgresLiveAssignmentDeliveryStore, connection::map_sqlx_error};
use crate::{SessionTokenHash, StoreError, StudentAssignmentAttemptContext};

impl PostgresLiveAssignmentDeliveryStore {
    pub(super) async fn read_student_assignment_attempt_context(
        &self,
        token: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
    ) -> Result<StudentAssignmentAttemptContext, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT assignment_attempt_reference_number, attempt_number, \
                    course_reference_number, course_title, course_theme, \
                    assignment_reference_number, assignment_title, \
                    timer_remaining_milliseconds \
               FROM ple_api.read_student_assignment_attempt_context($1)",
        )
        .bind(i64::from(assignment_attempt.number()))
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let returned_attempt = reference(
            row.try_get("assignment_attempt_reference_number")
                .map_err(map_sqlx_error)?,
            "Assignment Attempt reference",
            AssignmentAttemptReference::new,
        )?;
        if returned_attempt != assignment_attempt {
            return Err(StoreError::InvalidRecord(
                "Assignment Attempt context reference is invalid".to_string(),
            ));
        }
        let value = StudentAssignmentAttemptContext {
            assignment_attempt,
            attempt_number: positive(
                row.try_get("attempt_number").map_err(map_sqlx_error)?,
                "Assignment Attempt number",
            )?,
            course: reference(
                row.try_get("course_reference_number")
                    .map_err(map_sqlx_error)?,
                "Course reference",
                CourseInstanceReference::new,
            )?,
            course_title: nonempty(
                row.try_get("course_title").map_err(map_sqlx_error)?,
                "Course title",
            )?,
            course_theme: CourseTheme::from_str(
                &row.try_get::<String, _>("course_theme")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::InvalidRecord("Course theme is invalid".to_string()))?,
            assignment: reference(
                row.try_get("assignment_reference_number")
                    .map_err(map_sqlx_error)?,
                "Assignment reference",
                AssignmentReference::new,
            )?,
            assignment_title: nonempty(
                row.try_get("assignment_title").map_err(map_sqlx_error)?,
                "Assignment title",
            )?,
            timer_remaining_milliseconds: row
                .try_get::<Option<i64>, _>("timer_remaining_milliseconds")
                .map_err(map_sqlx_error)?
                .map(|value| {
                    u64::try_from(value).map_err(|_| {
                        StoreError::InvalidRecord("Assignment Attempt timer is invalid".to_string())
                    })
                })
                .transpose()?,
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(value)
    }
}

fn positive(value: i32, label: &str) -> Result<u32, StoreError> {
    u32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{label} is invalid")))
}

fn reference<T>(
    value: i64,
    label: &str,
    build: impl FnOnce(u64) -> Option<T>,
) -> Result<T, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(build)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{label} is invalid")))
}

fn nonempty(value: String, label: &str) -> Result<String, StoreError> {
    (!value.is_empty())
        .then_some(value)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{label} is invalid")))
}
