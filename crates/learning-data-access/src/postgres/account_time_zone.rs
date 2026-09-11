//! PostgreSQL persistence for the authenticated Account time-zone preference.

use async_trait::async_trait;
use question_model::AccountTimeZone;
use sqlx::{Postgres, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{AccountTimeZoneStore, SessionTokenHash, StoreError};

/// PostgreSQL Store for the account derived from the authenticated session.
#[derive(Clone)]
pub struct PostgresAccountTimeZoneStore {
    pool: Pool,
}

impl PostgresAccountTimeZoneStore {
    /// Binds the attested API pool to the self-only preference procedure.
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
}

#[async_trait]
impl AccountTimeZoneStore for PostgresAccountTimeZoneStore {
    async fn authenticated_account_time_zone(
        &self,
        token: SessionTokenHash,
    ) -> Result<AccountTimeZone, StoreError> {
        let mut tx = self.begin(token).await?;
        // ASVS 8.2.2 and 8.3.1: SQL derives the subject from the installed
        // authenticated session, so callers cannot select another Account.
        let value =
            sqlx::query_scalar::<_, Option<String>>("SELECT ple_api.current_account_time_zone()")
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)?;
        let zone = AccountTimeZone::parse(&value).map_err(|_| {
            StoreError::InvalidRecord("database returned an invalid Account time zone".to_string())
        })?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(zone)
    }
}
