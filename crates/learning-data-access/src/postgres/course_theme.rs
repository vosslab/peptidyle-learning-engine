//! PostgreSQL persistence for the scalar Course Theme setting.

use async_trait::async_trait;
use question_model::{CourseId, CourseTheme};
use sqlx::{Postgres, Row, Transaction};

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
impl CourseThemeStore for PostgresCourseThemeStore {
    async fn read_course_theme(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseTheme, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        // ASVS 1.2.3 and 8.2.2: the database function binds this opaque Course
        // identity to the installed session's active Course Membership.
        let row = sqlx::query("SELECT course_theme FROM ple_api.read_course_theme($1)")
            .bind(course.as_uuid())
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
        course: CourseId,
        selected_theme: CourseTheme,
    ) -> Result<CourseTheme, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        // ASVS 1.2.3 and 8.2.2: Instructor authorization is enforced by the
        // same session-aware database predicate as the update itself.
        let row = sqlx::query("SELECT course_theme FROM ple_api.update_course_theme($1, $2)")
            .bind(course.as_uuid())
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

fn theme(value: String) -> Result<CourseTheme, StoreError> {
    value.parse().map_err(|_| {
        StoreError::InvalidRecord("database returned an invalid Course Theme".to_string())
    })
}
