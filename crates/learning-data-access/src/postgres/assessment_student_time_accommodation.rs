//! Session-installed PostgreSQL Student time configuration adapter.
use super::{Pool, connection::map_sqlx_error};
use crate::{AssessmentStudentTimeAccommodationStore, SessionTokenHash, StoreError};
use async_trait::async_trait;
use question_model::{
    AssessmentId, AssessmentStudentTimeAccommodation, CourseInstanceId,
    SaveAssessmentStudentTimeAccommodationInput,
};
use sqlx::Row;

#[derive(Clone)]
pub struct PostgresAssessmentStudentTimeAccommodationStore {
    pool: Pool,
}

impl PostgresAssessmentStudentTimeAccommodationStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AssessmentStudentTimeAccommodationStore for PostgresAssessmentStudentTimeAccommodationStore {
    async fn student_time_configuration(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
        roster_id: String,
        save: Option<SaveAssessmentStudentTimeAccommodationInput>,
    ) -> Result<AssessmentStudentTimeAccommodation, StoreError> {
        if roster_id.is_empty()
            || roster_id.len() > 64
            || !roster_id
                .bytes()
                .all(|value| value.is_ascii_alphanumeric() || b"._-".contains(&value))
        {
            return Err(StoreError::NotFound);
        }
        let multiplier = save.as_ref().and_then(|value| value.time_multiplier);
        if multiplier.is_some_and(|value| !value.is_finite() || value < 1.0) {
            return Err(invalid());
        }
        let expected = match save
            .as_ref()
            .and_then(|value| value.expected_edit_number.as_deref())
        {
            None => 0,
            Some(value)
                if value.bytes().all(|byte| byte.is_ascii_digit()) && !value.starts_with('0') =>
            {
                value
                    .parse::<i64>()
                    .ok()
                    .filter(|number| *number > 0)
                    .ok_or_else(invalid)?
            }
            Some(_) => return Err(invalid()),
        };
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
        // ASVS 1.2.4, 8.3.1: bound values; the database resolves and authorizes
        // Course + Assessment + active roster Student, then serializes with start.
        let row = sqlx::query("SELECT * FROM ple_api.student_assessment_time_configuration($1, $2, $3, $4, $5, $6::double precision::text::numeric)")
            .bind(course.as_string()).bind(assessment.as_string()).bind(&roster_id)
            .bind(save.is_some()).bind(expected).bind(multiplier)
            .fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        let stored_multiplier: Option<f64> =
            row.try_get("time_multiplier").map_err(map_sqlx_error)?;
        let edit: Option<i64> = row
            .try_get("accommodation_edit_number")
            .map_err(map_sqlx_error)?;
        let base: Option<i32> = row
            .try_get("base_duration_seconds")
            .map_err(map_sqlx_error)?;
        let effective: Option<i32> = row
            .try_get("effective_duration_seconds")
            .map_err(map_sqlx_error)?;
        if stored_multiplier.is_some_and(|value| !value.is_finite() || value < 1.0)
            || edit.is_some_and(|value| value < 1)
            || base.is_some_and(|value| !(1..=43200).contains(&value))
            || effective.is_some_and(|value| !(1..=86400).contains(&value))
            || base.is_none() != effective.is_none()
        {
            return Err(invalid());
        }
        let projection = AssessmentStudentTimeAccommodation {
            roster_id: row.try_get("roster_id").map_err(map_sqlx_error)?,
            time_multiplier: stored_multiplier,
            edit_number: edit.map(|value| value.to_string()),
            base_duration_seconds: base.map(|value| value as u32),
            effective_duration_seconds: effective.map(|value| value as u32),
            capped_at_24_hours: row.try_get("capped_at_24_hours").map_err(map_sqlx_error)?,
        };
        if projection.roster_id != roster_id {
            return Err(invalid());
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }
}

fn invalid() -> StoreError {
    StoreError::InvalidRecord("Student time configuration is invalid".into())
}
