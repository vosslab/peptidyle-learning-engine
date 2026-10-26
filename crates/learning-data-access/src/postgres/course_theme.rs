//! PostgreSQL persistence for the scalar Course Theme setting.

use crate::postgres::connection::begin_authenticated_transaction;
use async_trait::async_trait;
use question_model::{CourseInstanceId, Theme};
use sqlx::Row;

use super::{Pool, connection::map_sqlx_error};
use crate::{CourseThemeStore, SessionTokenHash, StoreError};

/// PostgreSQL Store for session-authorized Course Theme reads and writes.
#[derive(Clone)]
pub struct PostgresCourseThemeStore {
    pool: Pool,
}

impl PostgresCourseThemeStore {
    /// Binds the attested API pool to Course Theme procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CourseThemeStore for PostgresCourseThemeStore {
    async fn read_course_theme(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
    ) -> Result<Theme, StoreError> {
        let mut transaction =
            begin_authenticated_transaction(&self.pool, session_token_hash).await?;
        // ASVS 1.2.3 and 8.2.2: the database function binds this opaque Course
        // identity to the installed session's active Course Membership.
        let row = sqlx::query("SELECT course_theme FROM ple_api.read_course_theme($1)")
            .bind(course_instance_id.as_str())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let theme = row
            .map(|row| theme(row.try_get("course_theme").map_err(map_sqlx_error)?))
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(theme)
    }

    async fn update_course_theme(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        selected_theme: Theme,
    ) -> Result<Theme, StoreError> {
        let mut transaction =
            begin_authenticated_transaction(&self.pool, session_token_hash).await?;
        // ASVS 1.2.3 and 8.2.2: Instructor authorization is enforced by the
        // same session-aware database predicate as the update itself.
        let row = sqlx::query("SELECT course_theme FROM ple_api.update_course_theme($1, $2)")
            .bind(course_instance_id.as_str())
            .bind(selected_theme.as_str())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let theme = row
            .map(|row| theme(row.try_get("course_theme").map_err(map_sqlx_error)?))
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(theme)
    }
}

fn theme(value: String) -> Result<Theme, StoreError> {
    value.parse().map_err(|_| {
        StoreError::InvalidRecord("database returned an invalid Course Theme".to_string())
    })
}
