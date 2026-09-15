//! PostgreSQL adapter for the minimal Student Assessment Attempt route context.

use std::str::FromStr;

use question_model::{
    AccountTimeZone, AssessmentAttemptReference, AssessmentReference, CourseInstanceReference,
    CourseTheme, Timestamp,
};
use sqlx::Row;

use super::{PostgresLiveAssessmentDeliveryStore, connection::map_sqlx_error};
use crate::{SessionTokenHash, StoreError, StudentAssessmentAttemptContext};

impl PostgresLiveAssessmentDeliveryStore {
    pub(super) async fn read_student_assessment_attempt_context(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptReference,
    ) -> Result<StudentAssessmentAttemptContext, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT assessment_attempt_reference_number, attempt_number, \
                    course_reference_number, course_short_name, course_long_name, course_theme, \
                    assessment_reference_number, assessment_title, \
                    display_time_zone, expires_at_millis, timer_remaining_milliseconds \
               FROM ple_api.read_student_assessment_attempt_context($1)",
        )
        .bind(i64::from(assessment_attempt.number()))
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let returned_attempt = reference(
            row.try_get("assessment_attempt_reference_number")
                .map_err(map_sqlx_error)?,
            "Assessment Attempt reference",
            AssessmentAttemptReference::new,
        )?;
        if returned_attempt != assessment_attempt {
            return Err(StoreError::InvalidRecord(
                "Assessment Attempt context reference is invalid".to_string(),
            ));
        }
        let value = StudentAssessmentAttemptContext {
            assessment_attempt,
            attempt_number: positive(
                row.try_get("attempt_number").map_err(map_sqlx_error)?,
                "Assessment Attempt number",
            )?,
            course: course_reference(
                row.try_get("course_reference_number")
                    .map_err(map_sqlx_error)?,
            )?,
            course_short_name: name(
                row.try_get("course_short_name").map_err(map_sqlx_error)?,
                "Course short name",
            )?,
            course_long_name: name(
                row.try_get("course_long_name").map_err(map_sqlx_error)?,
                "Course long name",
            )?,
            course_theme: CourseTheme::from_str(
                &row.try_get::<String, _>("course_theme")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::InvalidRecord("Course theme is invalid".to_string()))?,
            assessment: assessment_reference(
                row.try_get("assessment_reference_number")
                    .map_err(map_sqlx_error)?,
            )?,
            assessment_title: nonempty(
                row.try_get("assessment_title").map_err(map_sqlx_error)?,
                "Assessment title",
            )?,
            display_time_zone: AccountTimeZone::parse(
                &row.try_get::<String, _>("display_time_zone")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| {
                StoreError::InvalidRecord("Student Account time zone is invalid".to_string())
            })?,
            expires_at: row
                .try_get::<Option<i64>, _>("expires_at_millis")
                .map_err(map_sqlx_error)?
                .map(Timestamp::from_unix_millis),
            timer_remaining_milliseconds: row
                .try_get::<Option<i64>, _>("timer_remaining_milliseconds")
                .map_err(map_sqlx_error)?
                .map(|value| {
                    u64::try_from(value).map_err(|_| {
                        StoreError::InvalidRecord("Assessment Attempt timer is invalid".to_string())
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

fn course_reference(value: String) -> Result<CourseInstanceReference, StoreError> {
    CourseInstanceReference::new(value)
        .map_err(|_| StoreError::InvalidRecord("Course reference is invalid".to_string()))
}

fn assessment_reference(value: String) -> Result<AssessmentReference, StoreError> {
    AssessmentReference::new(value)
        .map_err(|_| StoreError::InvalidRecord("Assessment reference is invalid".to_string()))
}

fn name(value: String, label: &str) -> Result<String, StoreError> {
    (value == value.trim() && !value.is_empty() && value.chars().count() <= 200)
        .then_some(value)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{label} is invalid")))
}

fn nonempty(value: String, label: &str) -> Result<String, StoreError> {
    (!value.is_empty())
        .then_some(value)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{label} is invalid")))
}
