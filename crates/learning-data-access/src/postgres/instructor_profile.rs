//! PostgreSQL persistence for the authenticated Instructor's self profile.

use async_trait::async_trait;
use question_model::AccountTimeZone;
use sqlx::{Postgres, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    InstructorProfile, InstructorProfileStore, SessionTokenHash, StoreError,
    UpdateInstructorProfileInput,
};

#[derive(Clone)]
/// PostgreSQL store for the authenticated Instructor's self-profile preferences.
pub struct PostgresInstructorProfileStore {
    pool: Pool,
}

impl PostgresInstructorProfileStore {
    /// Creates the store bound to the supplied PostgreSQL connection pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
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

    async fn profile_from(value: Option<String>) -> Result<InstructorProfile, StoreError> {
        let value = value.ok_or(StoreError::NotFound)?;
        let time_zone = AccountTimeZone::parse(&value).map_err(|_| {
            StoreError::InvalidRecord("database returned an invalid Account time zone".to_string())
        })?;
        Ok(InstructorProfile { time_zone })
    }
}

#[async_trait]
impl InstructorProfileStore for PostgresInstructorProfileStore {
    async fn read_instructor_profile(
        &self,
        token: SessionTokenHash,
    ) -> Result<InstructorProfile, StoreError> {
        let mut tx = self.begin(token).await?;
        let profile = Self::profile_from(
            sqlx::query_scalar("SELECT ple_api.read_instructor_profile()")
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(profile)
    }

    async fn update_instructor_profile(
        &self,
        token: SessionTokenHash,
        input: UpdateInstructorProfileInput,
    ) -> Result<InstructorProfile, StoreError> {
        let mut tx = self.begin(token).await?;
        let profile = Self::profile_from(
            sqlx::query_scalar("SELECT ple_api.update_instructor_profile($1)")
                .bind(input.time_zone.as_str())
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(profile)
    }
}
