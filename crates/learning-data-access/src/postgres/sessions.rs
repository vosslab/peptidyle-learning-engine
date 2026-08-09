//! PostgreSQL authentication-session persistence through the dedicated auth role.

use async_trait::async_trait;
use question_model::{ActivityTimestamp, TenantId, UserId, UserRole};
use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{Postgres, Row, Transaction};

use super::{PostgresStore, map_sqlx_error};
use crate::{
    SessionLifetime, SessionRecord, SessionStore, SessionSubject, SessionTokenHash, StoreError,
};

impl PostgresStore {
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
impl SessionStore for PostgresStore {
    async fn create_session(
        &self,
        token_hash: SessionTokenHash,
        subject: SessionSubject,
        lifetime: SessionLifetime,
    ) -> Result<SessionRecord, StoreError> {
        let mut transaction = self.begin_session(token_hash).await?;
        let row = sqlx::query(
            "INSERT INTO auth_session \
             (session_hash, tenant_id, user_id, display_name, roles, expires_at) \
             VALUES ($1, $2, $3, $4, $5, \
                     transaction_timestamp() + ($6::bigint * interval '1 second')) \
             RETURNING session_hash, tenant_id, user_id, display_name, roles, \
                       floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                       floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis",
        )
        .bind(token_hash.to_string())
        .bind(subject.tenant().as_uuid())
        .bind(subject.user().as_uuid())
        .bind(subject.display_name())
        .bind(Json(subject.roles().to_vec()))
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
            "SELECT session_hash, tenant_id, user_id, display_name, roles, \
                    floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM auth_session \
             WHERE session_hash = $1 AND revoked_at IS NULL \
                   AND expires_at > transaction_timestamp()",
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
        sqlx::query(
            "UPDATE auth_session SET revoked_at = transaction_timestamp() \
             WHERE session_hash = $1 AND revoked_at IS NULL",
        )
        .bind(token_hash.to_string())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

fn decode_session_row(row: &PgRow) -> Result<SessionRecord, StoreError> {
    let token_hash: String = row.try_get("session_hash").map_err(map_sqlx_error)?;
    let tenant = row.try_get("tenant_id").map_err(map_sqlx_error)?;
    let user = row.try_get("user_id").map_err(map_sqlx_error)?;
    let display_name: String = row.try_get("display_name").map_err(map_sqlx_error)?;
    let Json(roles): Json<Vec<UserRole>> = row.try_get("roles").map_err(map_sqlx_error)?;
    let created_at_millis: i64 = row.try_get("created_at_millis").map_err(map_sqlx_error)?;
    let expires_at_millis: i64 = row.try_get("expires_at_millis").map_err(map_sqlx_error)?;
    let token_hash = SessionTokenHash::from_hex(token_hash.trim_end()).map_err(|error| {
        StoreError::Unavailable(format!("stored session hash is invalid: {error}"))
    })?;
    let subject = SessionSubject::new(
        TenantId::from_uuid(tenant),
        UserId::from_uuid(user),
        display_name,
        roles,
    )
    .map_err(|error| {
        StoreError::Unavailable(format!("stored session subject is invalid: {error}"))
    })?;
    Ok(SessionRecord {
        token_hash,
        subject,
        created_at: ActivityTimestamp::from_unix_millis(created_at_millis),
        expires_at: ActivityTimestamp::from_unix_millis(expires_at_millis),
    })
}
