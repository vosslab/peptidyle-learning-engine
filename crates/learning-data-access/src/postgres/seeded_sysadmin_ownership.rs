//! Atomic first-passkey ownership completion for the seeded Sysadmin.

use question_model::{UserId, UserRole};
use sqlx::types::Json;
use sqlx::{query, query_scalar};

use super::account_identity::{
    decode_account, decode_passkey, delete_expired_account_sessions, insert_account_session,
    json_value,
};
use super::{PostgresStore, map_sqlx_error};
use crate::{CompleteSeededSysadminOwnership, CompletedPasskeySession, StoreError};

/// Keeps the first-claim availability probe in the same capability module as
/// the atomic command, including the permanent historical-passkey rule.
pub(super) async fn seeded_sysadmin_ownership_available(
    store: &PostgresStore,
    user: UserId,
) -> Result<bool, StoreError> {
    let mut transaction = store.begin_auth().await?;
    let roles: Option<Json<Vec<UserRole>>> =
        query_scalar("SELECT platform_roles FROM ple_account WHERE user_id = $1")
            .bind(user.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
    let Json(roles) = roles.ok_or(StoreError::NotFound)?;
    if roles.as_slice() != [UserRole::Sysadmin] {
        return Err(StoreError::Forbidden);
    }
    let historical_passkey: bool =
        query_scalar("SELECT EXISTS(SELECT 1 FROM account_passkey WHERE user_id = $1)")
            .bind(user.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(!historical_passkey)
}

pub(super) async fn complete_seeded_sysadmin_ownership(
    store: &PostgresStore,
    command: CompleteSeededSysadminOwnership,
) -> Result<CompletedPasskeySession, StoreError> {
    if command.target != command.passkey.user {
        return Err(StoreError::Forbidden);
    }
    if command.presented_account_session == Some(command.session_token_hash) {
        return Err(StoreError::AlreadyExists);
    }
    let mut transaction = store.begin_auth().await?;
    delete_expired_account_sessions(&mut transaction).await?;
    let account = query(
        "SELECT user_id, normalized_email, delivery_email, display_name, platform_roles, \
                floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                floor(extract(epoch FROM updated_at) * 1000)::bigint AS updated_at_millis \
         FROM ple_account WHERE user_id = $1 FOR UPDATE",
    )
    .bind(command.target.as_uuid())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)
    .and_then(|row| decode_account(&row))?;
    if account.platform_roles.as_slice() != [UserRole::Sysadmin] {
        return Err(StoreError::Forbidden);
    }
    let historical_passkey: bool =
        query_scalar("SELECT EXISTS(SELECT 1 FROM account_passkey WHERE user_id = $1)")
            .bind(command.target.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
    if historical_passkey {
        return Err(StoreError::Conflict);
    }
    let ceremony = query(
        "DELETE FROM webauthn_ceremony \
         WHERE ceremony_id = $1 AND browser_binding_hash = $2 \
           AND ceremony_kind = 'registration' AND user_id = $3 \
           AND expires_at > transaction_timestamp() \
         RETURNING ceremony_id",
    )
    .bind(command.ceremony_id.as_uuid())
    .bind(command.browser_binding.as_bytes().to_vec())
    .bind(command.target.as_uuid())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;
    if ceremony.is_none() {
        return Err(StoreError::NotFound);
    }
    let row = query(
        "INSERT INTO account_passkey \
         (passkey_id, user_id, credential_id_hash, label, credential) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING passkey_id, user_id, credential_id_hash, label, credential, \
           floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
           floor(extract(epoch FROM last_used_at) * 1000)::bigint AS last_used_at_millis, \
           floor(extract(epoch FROM revoked_at) * 1000)::bigint AS revoked_at_millis",
    )
    .bind(command.passkey.id.as_uuid())
    .bind(command.target.as_uuid())
    .bind(command.passkey.credential_id_hash.as_bytes().to_vec())
    .bind(&command.passkey.label)
    .bind(Json(json_value(&command.passkey.credential)?))
    .fetch_one(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;
    let passkey = decode_passkey(&row)?;
    if let Some(token_hash) = command.presented_account_session {
        query("DELETE FROM account_authentication_session WHERE token_hash = $1")
            .bind(token_hash.as_bytes().to_vec())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
    }
    let session = insert_account_session(
        &mut transaction,
        command.session_token_hash,
        command.target,
        command.session_lifetime,
    )
    .await?;
    if let Some(token_hash) = command.presented_tenant_session {
        query("SELECT set_config('ple.session_hash', $1, true)")
            .bind(token_hash.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        query(
            "UPDATE auth_session SET revoked_at = transaction_timestamp() \
             WHERE session_hash = $1 AND revoked_at IS NULL",
        )
        .bind(token_hash.to_string())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(CompletedPasskeySession { passkey, session })
}
