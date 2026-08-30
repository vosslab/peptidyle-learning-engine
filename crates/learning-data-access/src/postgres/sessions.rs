//! PostgreSQL authentication-session persistence through the dedicated auth role.

use async_trait::async_trait;
use question_model::{ActivityTimestamp, UserId, UserRole};
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};

use super::{PostgresStore, map_sqlx_error};
use crate::{
    SessionId, SessionLifetime, SessionRecord, SessionStore, SessionSubject, SessionTokenHash,
    StoreError,
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
        let session_id = SessionId::generate()?;
        let row = sqlx::query(
            "SELECT session_id, encode(token_hash, 'hex') AS session_hash, user_id, role, \
                    floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM ple_private.create_primary_session(\
                $1, $2, $3, decode($4, 'hex'), $5\
             )",
        )
        .bind(session_id.as_uuid())
        .bind(subject.user().as_uuid())
        .bind(role_name(subject.role()))
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
            "SELECT session_id, encode(token_hash, 'hex') AS session_hash, user_id, role, \
                    floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM ple_api.resolve_and_install_actor(decode($1, 'hex'))",
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
        sqlx::query("SELECT ple_private.revoke_primary_session(decode($1, 'hex'))")
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
    let user = row.try_get("user_id").map_err(map_sqlx_error)?;
    let role: String = row.try_get("role").map_err(map_sqlx_error)?;
    let created_at_millis: i64 = row.try_get("created_at_millis").map_err(map_sqlx_error)?;
    let expires_at_millis: i64 = row.try_get("expires_at_millis").map_err(map_sqlx_error)?;
    let token_hash = SessionTokenHash::from_hex(token_hash.trim_end()).map_err(|error| {
        StoreError::Unavailable(format!("stored session hash is invalid: {error}"))
    })?;
    let subject = SessionSubject::new(
        UserId::from_uuid(user),
        "Authenticated account",
        decode_role(&role)?,
    )
    .map_err(|error| {
        StoreError::Unavailable(format!("stored session subject is invalid: {error}"))
    })?;
    Ok(SessionRecord {
        id: SessionId::from_uuid(session_id),
        token_hash,
        subject,
        created_at: ActivityTimestamp::from_unix_millis(created_at_millis),
        expires_at: ActivityTimestamp::from_unix_millis(expires_at_millis),
    })
}

fn role_name(role: UserRole) -> &'static str {
    match role {
        UserRole::Student => "student",
        UserRole::Instructor => "instructor",
        UserRole::Sysadmin => "sysadmin",
    }
}

fn decode_role(value: &str) -> Result<UserRole, StoreError> {
    match value {
        "student" => Ok(UserRole::Student),
        "instructor" => Ok(UserRole::Instructor),
        "sysadmin" => Ok(UserRole::Sysadmin),
        _ => Err(StoreError::Unavailable(
            "stored session role is invalid".to_string(),
        )),
    }
}
