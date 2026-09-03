//! PostgreSQL authentication-session persistence through the dedicated auth role.

use async_trait::async_trait;
use question_model::{AccountId, ProductRole, Timestamp};
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    SessionId, SessionLifetime, SessionRecord, SessionStore, SessionTokenHash, StoreError,
};

/// PostgreSQL implementation of the global Account session store.
///
/// The application pool stays private to this adapter so every session command
/// starts its own transaction, assumes only `ple_auth`, and supplies the
/// opaque token hash as transaction-local capability context.
#[derive(Clone)]
pub struct PostgresSessionStore {
    pool: Pool,
}

impl PostgresSessionStore {
    /// Binds the already-attested API pool to the authentication-session store.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin_session(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
            .bind(token_hash.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl SessionStore for PostgresSessionStore {
    async fn create_session(
        &self,
        token_hash: SessionTokenHash,
        account: AccountId,
        lifetime: SessionLifetime,
    ) -> Result<SessionRecord, StoreError> {
        let mut transaction = self.begin_session(token_hash).await?;
        let session_id = SessionId::generate()?;
        let row = sqlx::query(
            "SELECT session_id, encode(token_hash, 'hex') AS session_hash, account_id, product_role, \
                    floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM ple_api.create_authenticated_session(\
                $1, $2, decode($3, 'hex'), $4\
             )",
        )
        .bind(session_id.as_uuid())
        .bind(account.as_uuid())
        .bind(token_hash.to_string())
        .bind(i64::from(lifetime.as_seconds()))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = decode_session_row(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn resolve_session(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Option<SessionRecord>, StoreError> {
        let mut transaction = self.begin_session(token_hash).await?;
        let row = sqlx::query(
            "SELECT session_id, encode(token_hash, 'hex') AS session_hash, account_id, product_role, \
                    floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token_hash.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_session_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn revoke_session(&self, token_hash: SessionTokenHash) -> Result<(), StoreError> {
        let mut transaction = self.begin_session(token_hash).await?;
        sqlx::query("SELECT ple_api.revoke_authenticated_session(decode($1, 'hex'))")
            .bind(token_hash.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

fn decode_session_row(row: &PgRow) -> Result<SessionRecord, StoreError> {
    let token_hash: String = row.try_get("session_hash").map_err(map_sqlx_error)?;
    let session_id = row.try_get("session_id").map_err(map_sqlx_error)?;
    let account = row.try_get("account_id").map_err(map_sqlx_error)?;
    let product_role: String = row.try_get("product_role").map_err(map_sqlx_error)?;
    let created_at_millis: i64 = row.try_get("created_at_millis").map_err(map_sqlx_error)?;
    let expires_at_millis: i64 = row.try_get("expires_at_millis").map_err(map_sqlx_error)?;
    let token_hash = SessionTokenHash::from_hex(token_hash.trim_end()).map_err(|error| {
        StoreError::Unavailable(format!("stored session hash is invalid: {error}"))
    })?;
    Ok(SessionRecord {
        id: SessionId::from_uuid(session_id),
        token_hash,
        account: AccountId::from_uuid(account),
        product_role: decode_product_role(&product_role)?,
        created_at: Timestamp::from_unix_millis(created_at_millis),
        expires_at: Timestamp::from_unix_millis(expires_at_millis),
    })
}

fn decode_product_role(value: &str) -> Result<ProductRole, StoreError> {
    match value {
        "student" => Ok(ProductRole::Student),
        "instructor" => Ok(ProductRole::Instructor),
        "sysadmin" => Ok(ProductRole::Sysadmin),
        _ => Err(StoreError::Unavailable(
            "stored session role is invalid".to_string(),
        )),
    }
}
