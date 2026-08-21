//! PostgreSQL passwordless-account persistence through `ple_auth`.

use async_trait::async_trait;
use question_model::{
    ActivityTimestamp, CourseId, CourseMembershipRole, TenantId, UserId, UserRole,
};
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::types::{Json, Uuid};
use sqlx::{Postgres, Row, Transaction};

use super::{PostgresStore, map_sqlx_error, page_from_keyed_records};
use crate::{
    AccountCourseContext, AccountIdentityStore, AccountRecord, AccountSessionLifetime,
    AccountSessionRecord, AccountSessionStore, AccountSessionTokenHash, AuthenticationEmail,
    AuthenticationRateLimitDecision, AuthenticationRateLimitKey, AuthenticationRateLimitScope,
    BeginEmailAuthentication, BeginWebauthnCeremony, BrowserBindingHash,
    CompleteEmailAuthentication, CompleteEmailAuthenticationAndCreateSession,
    CompleteEmailChangeAndRevokeUserSessions, CompletePasskeyAuthenticationAndCreateSession,
    CompleteSeededSysadminOwnership, CompletedAccountSession, CompletedEmailAuthentication,
    CompletedPasskeySession, ConsumeAuthenticationRateLimit, CredentialIdHash,
    EmailAuthenticationChallenge, EmailAuthenticationPurpose, EmailChallengeId,
    EmailChallengeSecretHash, Page, PageRequest, PasskeyId, PasskeyRecord, RegisterPasskey,
    StoreError, WebauthnCeremony, WebauthnCeremonyId, WebauthnCeremonyKind, WebauthnState,
    validated_account_display_name,
};

const AUTH_EXPIRY_CLEANUP_BATCH: i64 = 128;

impl PostgresStore {
    pub(super) async fn begin_auth(&self) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl AccountIdentityStore for PostgresStore {
    async fn consume_authentication_rate_limit(
        &self,
        command: ConsumeAuthenticationRateLimit,
    ) -> Result<AuthenticationRateLimitDecision, StoreError> {
        let mut transaction = self.begin_auth().await?;
        delete_expired_rate_limits(&mut transaction).await?;
        let row = sqlx::query(
            "INSERT INTO authentication_rate_limit \
             (limit_scope, key_hash, window_started_at, attempt_count, updated_at) \
             VALUES ($1, $2, transaction_timestamp(), 1, transaction_timestamp()) \
             ON CONFLICT (limit_scope, key_hash) DO UPDATE SET \
               window_started_at = CASE \
                 WHEN authentication_rate_limit.window_started_at \
                      + ($3::bigint * interval '1 second') <= transaction_timestamp() \
                 THEN transaction_timestamp() \
                 ELSE authentication_rate_limit.window_started_at END, \
               attempt_count = CASE \
                 WHEN authentication_rate_limit.window_started_at \
                      + ($3::bigint * interval '1 second') <= transaction_timestamp() THEN 1 \
                 ELSE least(authentication_rate_limit.attempt_count + 1, $4::integer + 1) END, \
               updated_at = transaction_timestamp() \
             RETURNING attempt_count, \
               greatest(1, ceil(extract(epoch FROM \
                 window_started_at + ($3::bigint * interval '1 second') \
                 - transaction_timestamp())))::bigint AS retry_after_seconds",
        )
        .bind(rate_limit_scope(command.scope))
        .bind(command.key.as_bytes().to_vec())
        .bind(i64::from(command.policy.window_seconds()))
        .bind(
            i32::try_from(command.policy.maximum_attempts()).map_err(|_| {
                StoreError::InvalidRecord("rate-limit attempt bound is invalid".to_string())
            })?,
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let attempts: i32 = row.try_get("attempt_count").map_err(map_sqlx_error)?;
        let attempts = u32::try_from(attempts).map_err(|_| {
            StoreError::Unavailable("stored rate-limit count is invalid".to_string())
        })?;
        let decision = if attempts <= command.policy.maximum_attempts() {
            AuthenticationRateLimitDecision::Allowed {
                remaining_attempts: command.policy.maximum_attempts() - attempts,
            }
        } else {
            let retry_after: i64 = row.try_get("retry_after_seconds").map_err(map_sqlx_error)?;
            AuthenticationRateLimitDecision::Denied {
                retry_after_seconds: u32::try_from(retry_after).map_err(|_| {
                    StoreError::Unavailable("stored rate-limit window is invalid".to_string())
                })?,
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(decision)
    }

    async fn begin_email_authentication(
        &self,
        command: BeginEmailAuthentication,
    ) -> Result<EmailAuthenticationChallenge, StoreError> {
        let (purpose, purpose_user) = purpose_columns(command.purpose);
        let mut transaction = self.begin_auth().await?;
        delete_expired_email_challenges(&mut transaction).await?;
        let row = sqlx::query(
            "INSERT INTO email_authentication_challenge \
             (challenge_id, token_hash, browser_binding_hash, rate_limit_key_hash, normalized_email, \
              delivery_email, purpose, purpose_user_id, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, \
                     transaction_timestamp() + ($9::bigint * interval '1 second')) \
             ON CONFLICT ON CONSTRAINT email_authentication_challenge_subject_key \
             DO UPDATE SET challenge_id = EXCLUDED.challenge_id, \
                           token_hash = EXCLUDED.token_hash, \
                           browser_binding_hash = EXCLUDED.browser_binding_hash, \
                           rate_limit_key_hash = EXCLUDED.rate_limit_key_hash, \
                           delivery_email = EXCLUDED.delivery_email, \
                           created_at = transaction_timestamp(), \
                           expires_at = EXCLUDED.expires_at \
             RETURNING challenge_id, token_hash, browser_binding_hash, rate_limit_key_hash, normalized_email, \
                       delivery_email, purpose, purpose_user_id, \
                       floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                       floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis",
        )
        .bind(command.id.as_uuid())
        .bind(command.token_hash.as_bytes().to_vec())
        .bind(command.browser_binding.as_bytes().to_vec())
        .bind(command.email_rate_limit_key.as_bytes().to_vec())
        .bind(command.email.normalized())
        .bind(command.email.delivery())
        .bind(purpose)
        .bind(purpose_user.map(|user| user.as_uuid()))
        .bind(i64::from(command.lifetime.as_seconds()))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = decode_email_challenge(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn complete_email_authentication(
        &self,
        command: CompleteEmailAuthentication,
    ) -> Result<CompletedEmailAuthentication, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let result =
            complete_email_authentication_in_transaction(&mut transaction, command).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn complete_email_authentication_and_create_session(
        &self,
        command: CompleteEmailAuthenticationAndCreateSession,
    ) -> Result<CompletedAccountSession, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let authentication =
            complete_email_authentication_in_transaction(&mut transaction, command.authentication)
                .await?;
        let session = insert_account_session(
            &mut transaction,
            command.session_token_hash,
            authentication.account.user,
            command.session_lifetime,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CompletedAccountSession {
            authentication,
            session,
        })
    }

    async fn complete_email_change_and_revoke_user_sessions(
        &self,
        command: CompleteEmailChangeAndRevokeUserSessions,
    ) -> Result<CompletedAccountSession, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let row = sqlx::query(
            "SELECT user_id, normalized_email, delivery_email, display_name, platform_roles, \
                    created_at_millis, updated_at_millis, session_created_at_millis, \
                    session_expires_at_millis \
             FROM ple_complete_email_change_and_revoke_sessions($1, $2, $3, $4, $5)",
        )
        .bind(command.authentication.token_hash.as_bytes().to_vec())
        .bind(command.authentication.browser_binding.as_bytes().to_vec())
        .bind(command.authentication.proposed_user.as_uuid())
        .bind(command.session_token_hash.as_bytes().to_vec())
        .bind(i64::from(command.session_lifetime.as_seconds()))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let account = decode_account(&row)?;
        let session = AccountSessionRecord {
            token_hash: command.session_token_hash,
            user: account.user,
            created_at: timestamp(&row, "session_created_at_millis")?,
            expires_at: timestamp(&row, "session_expires_at_millis")?,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CompletedAccountSession {
            authentication: CompletedEmailAuthentication {
                account,
                created: false,
            },
            session,
        })
    }

    async fn get_account(&self, user: UserId) -> Result<Option<AccountRecord>, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let row = sqlx::query(ACCOUNT_SELECT)
            .bind(user.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let result = row.as_ref().map(decode_account).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn list_account_course_contexts(
        &self,
        user: UserId,
        page: PageRequest,
    ) -> Result<Page<AccountCourseContext>, StoreError> {
        let (after_tenant, after_course) = page
            .after
            .as_ref()
            .map(|cursor| decode_account_course_cursor(cursor.as_str()))
            .transpose()?
            .unzip();
        let mut transaction = self.begin_auth().await?;
        let rows = sqlx::query(
            "SELECT tenant_id, course_id, title, role \
             FROM public.ple_account_course_context_page($1, $2, $3, $4)",
        )
        .bind(user.as_uuid())
        .bind(after_tenant.map(|tenant| tenant.as_uuid()))
        .bind(after_course.map(|course| course.as_uuid()))
        .bind(i32::from(page.size.get()) + 1)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = rows
            .iter()
            .map(|row| {
                let context = decode_account_course_context(row)?;
                Ok((account_course_cursor(&context), context))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_account_course_context(
        &self,
        user: UserId,
        course: CourseId,
    ) -> Result<Option<AccountCourseContext>, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let rows = sqlx::query(
            "SELECT tenant_id, course_id, title, role \
             FROM public.ple_account_course_context($1, $2)",
        )
        .bind(user.as_uuid())
        .bind(course.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = match rows.as_slice() {
            [] => None,
            [row] => Some(decode_account_course_context(row)?),
            _ => {
                return Err(StoreError::Unavailable(
                    "course identity is ambiguous across account contexts".to_string(),
                ));
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn begin_webauthn_ceremony(
        &self,
        command: BeginWebauthnCeremony,
    ) -> Result<WebauthnCeremony, StoreError> {
        let (kind, user) = ceremony_kind_columns(command.kind);
        let mut transaction = self.begin_auth().await?;
        delete_expired_webauthn_ceremonies(&mut transaction).await?;
        let row = sqlx::query(
            "INSERT INTO webauthn_ceremony \
             (ceremony_id, ceremony_kind, user_id, browser_binding_hash, state, expires_at) \
             VALUES ($1, $2, $3, $4, $5, \
                     transaction_timestamp() + ($6::bigint * interval '1 second')) \
             RETURNING ceremony_id, ceremony_kind, user_id, browser_binding_hash, state, \
               floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis",
        )
        .bind(command.id.as_uuid())
        .bind(kind)
        .bind(user.map(|user| user.as_uuid()))
        .bind(command.browser_binding.as_bytes().to_vec())
        .bind(Json(json_value(&command.state)?))
        .bind(i64::from(command.lifetime.as_seconds()))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let ceremony = decode_ceremony(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(ceremony)
    }

    async fn take_webauthn_ceremony(
        &self,
        id: WebauthnCeremonyId,
        browser_binding: BrowserBindingHash,
    ) -> Result<Option<WebauthnCeremony>, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let row = sqlx::query(
            "DELETE FROM webauthn_ceremony \
             WHERE ceremony_id = $1 AND browser_binding_hash = $2 \
               AND expires_at > transaction_timestamp() \
             RETURNING ceremony_id, ceremony_kind, user_id, browser_binding_hash, state, \
               floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis",
        )
        .bind(id.as_uuid())
        .bind(browser_binding.as_bytes().to_vec())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let result = decode_ceremony(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(result))
    }

    async fn get_webauthn_ceremony(
        &self,
        id: WebauthnCeremonyId,
        browser_binding: BrowserBindingHash,
    ) -> Result<Option<WebauthnCeremony>, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let row = sqlx::query(
            "SELECT ceremony_id, ceremony_kind, user_id, browser_binding_hash, state, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM webauthn_ceremony \
             WHERE ceremony_id = $1 AND browser_binding_hash = $2 \
               AND expires_at > transaction_timestamp()",
        )
        .bind(id.as_uuid())
        .bind(browser_binding.as_bytes().to_vec())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let ceremony = row.as_ref().map(decode_ceremony).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(ceremony)
    }

    async fn seeded_sysadmin_ownership_available(&self, user: UserId) -> Result<bool, StoreError> {
        super::seeded_sysadmin_ownership::seeded_sysadmin_ownership_available(self, user).await
    }

    async fn complete_seeded_sysadmin_ownership(
        &self,
        command: CompleteSeededSysadminOwnership,
    ) -> Result<CompletedPasskeySession, StoreError> {
        super::seeded_sysadmin_ownership::complete_seeded_sysadmin_ownership(self, command).await
    }

    async fn insert_passkey(&self, command: RegisterPasskey) -> Result<PasskeyRecord, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let row = sqlx::query(
            "INSERT INTO account_passkey \
             (passkey_id, user_id, credential_id_hash, label, credential) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING passkey_id, user_id, credential_id_hash, label, credential, \
               floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
               floor(extract(epoch FROM last_used_at) * 1000)::bigint AS last_used_at_millis, \
               floor(extract(epoch FROM revoked_at) * 1000)::bigint AS revoked_at_millis",
        )
        .bind(command.id.as_uuid())
        .bind(command.user.as_uuid())
        .bind(command.credential_id_hash.as_bytes().to_vec())
        .bind(&command.label)
        .bind(Json(json_value(&command.credential)?))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let passkey = decode_passkey(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(passkey)
    }

    async fn list_active_passkeys(&self, user: UserId) -> Result<Vec<PasskeyRecord>, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let rows = sqlx::query(PASSKEY_SELECT)
            .bind(user.as_uuid())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let result = rows.iter().map(decode_passkey).collect::<Result<_, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn get_active_passkey_by_credential_id_hash(
        &self,
        credential_id_hash: CredentialIdHash,
    ) -> Result<Option<PasskeyRecord>, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let row = sqlx::query(PASSKEY_BY_CREDENTIAL_SELECT)
            .bind(credential_id_hash.as_bytes().to_vec())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let result = row.as_ref().map(decode_passkey).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn replace_passkey_after_authentication(
        &self,
        passkey: PasskeyRecord,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_auth().await?;
        let result = sqlx::query(
            "UPDATE account_passkey SET credential = $4, \
             last_used_at = CASE WHEN $5::bigint IS NULL THEN last_used_at \
             ELSE to_timestamp($5::double precision / 1000.0) END \
             WHERE passkey_id = $1 AND user_id = $2 AND credential_id_hash = $3 \
             AND revoked_at IS NULL",
        )
        .bind(passkey.id.as_uuid())
        .bind(passkey.user.as_uuid())
        .bind(passkey.credential_id_hash.as_bytes().to_vec())
        .bind(Json(json_value(&passkey.credential)?))
        .bind(passkey.last_used_at.map(|value| value.as_unix_millis()))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        one_row(result.rows_affected())?;
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn complete_passkey_authentication_and_create_session(
        &self,
        command: CompletePasskeyAuthenticationAndCreateSession,
    ) -> Result<CompletedPasskeySession, StoreError> {
        let mut transaction = self.begin_auth().await?;
        delete_expired_account_sessions(&mut transaction).await?;
        let row = sqlx::query(
            "UPDATE account_passkey SET credential = $4, last_used_at = transaction_timestamp() \
             WHERE passkey_id = $1 AND user_id = $2 AND credential_id_hash = $3 \
               AND revoked_at IS NULL \
             RETURNING passkey_id, user_id, credential_id_hash, label, credential, \
               floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
               floor(extract(epoch FROM last_used_at) * 1000)::bigint AS last_used_at_millis, \
               floor(extract(epoch FROM revoked_at) * 1000)::bigint AS revoked_at_millis",
        )
        .bind(command.passkey.id.as_uuid())
        .bind(command.passkey.user.as_uuid())
        .bind(command.passkey.credential_id_hash.as_bytes().to_vec())
        .bind(Json(json_value(&command.passkey.credential)?))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let passkey = decode_passkey(&row)?;
        let session = insert_account_session(
            &mut transaction,
            command.session_token_hash,
            passkey.user,
            command.session_lifetime,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CompletedPasskeySession { passkey, session })
    }

    async fn revoke_passkey(&self, user: UserId, passkey: PasskeyId) -> Result<(), StoreError> {
        let mut transaction = self.begin_auth().await?;
        let result = sqlx::query(
            "UPDATE account_passkey SET revoked_at = transaction_timestamp() \
             WHERE passkey_id = $1 AND user_id = $2 AND revoked_at IS NULL",
        )
        .bind(passkey.as_uuid())
        .bind(user.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        one_row(result.rows_affected())?;
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

#[async_trait]
impl AccountSessionStore for PostgresStore {
    async fn create_account_session(
        &self,
        token_hash: AccountSessionTokenHash,
        user: UserId,
        lifetime: AccountSessionLifetime,
    ) -> Result<AccountSessionRecord, StoreError> {
        let mut transaction = self.begin_auth().await?;
        delete_expired_account_sessions(&mut transaction).await?;
        let record = insert_account_session(&mut transaction, token_hash, user, lifetime).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn resolve_account_session(
        &self,
        token_hash: AccountSessionTokenHash,
    ) -> Result<Option<AccountSessionRecord>, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let row = sqlx::query(
            "SELECT token_hash, user_id, \
               floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
               floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM account_authentication_session \
             WHERE token_hash = $1 AND expires_at > transaction_timestamp()",
        )
        .bind(token_hash.as_bytes().to_vec())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_account_session).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn revoke_account_session(
        &self,
        token_hash: AccountSessionTokenHash,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_auth().await?;
        sqlx::query("DELETE FROM account_authentication_session WHERE token_hash = $1")
            .bind(token_hash.as_bytes().to_vec())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

async fn delete_expired_rate_limits(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), StoreError> {
    sqlx::query(
        "DELETE FROM authentication_rate_limit WHERE (limit_scope, key_hash) IN (\
         SELECT limit_scope, key_hash FROM authentication_rate_limit \
         WHERE updated_at <= transaction_timestamp() - interval '24 hours' \
         ORDER BY updated_at LIMIT $1)",
    )
    .bind(AUTH_EXPIRY_CLEANUP_BATCH)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

async fn delete_expired_email_challenges(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), StoreError> {
    sqlx::query(
        "DELETE FROM email_authentication_challenge WHERE challenge_id IN (\
         SELECT challenge_id FROM email_authentication_challenge \
         WHERE expires_at <= transaction_timestamp() \
         ORDER BY expires_at LIMIT $1)",
    )
    .bind(AUTH_EXPIRY_CLEANUP_BATCH)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

async fn delete_expired_webauthn_ceremonies(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), StoreError> {
    sqlx::query(
        "DELETE FROM webauthn_ceremony WHERE ceremony_id IN (\
         SELECT ceremony_id FROM webauthn_ceremony \
         WHERE expires_at <= transaction_timestamp() \
         ORDER BY expires_at LIMIT $1)",
    )
    .bind(AUTH_EXPIRY_CLEANUP_BATCH)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

pub(super) async fn delete_expired_account_sessions(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), StoreError> {
    sqlx::query(
        "DELETE FROM account_authentication_session WHERE token_hash IN (\
         SELECT token_hash FROM account_authentication_session \
         WHERE expires_at <= transaction_timestamp() \
         ORDER BY expires_at LIMIT $1)",
    )
    .bind(AUTH_EXPIRY_CLEANUP_BATCH)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

async fn complete_email_authentication_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    command: CompleteEmailAuthentication,
) -> Result<CompletedEmailAuthentication, StoreError> {
    let row = sqlx::query(
        "DELETE FROM email_authentication_challenge \
         WHERE token_hash = $1 AND browser_binding_hash = $2 \
           AND expires_at > transaction_timestamp() \
         RETURNING normalized_email, delivery_email, browser_binding_hash, rate_limit_key_hash, \
                   purpose, purpose_user_id",
    )
    .bind(command.token_hash.as_bytes().to_vec())
    .bind(command.browser_binding.as_bytes().to_vec())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let normalized: String = row.try_get("normalized_email").map_err(map_sqlx_error)?;
    let delivery: String = row.try_get("delivery_email").map_err(map_sqlx_error)?;
    let email = decode_email(&normalized, &delivery)?;
    let email_rate_limit_key =
        AuthenticationRateLimitKey::from_bytes(fixed_hash(&row, "rate_limit_key_hash")?);
    let purpose: String = row.try_get("purpose").map_err(map_sqlx_error)?;
    let purpose_user: Option<Uuid> = row.try_get("purpose_user_id").map_err(map_sqlx_error)?;
    let display_name = validated_account_display_name(&command.proposed_display_name)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let (account, created) = match purpose.as_str() {
        "sign_in_or_register" => match load_account_by_email(transaction, &normalized).await? {
            Some(account) => (account, false),
            None => (
                insert_account(transaction, command.proposed_user, &email, &display_name).await?,
                true,
            ),
        },
        "change_email" => {
            let user = purpose_user
                .map(UserId::from_uuid)
                .filter(|user| *user == command.proposed_user)
                .ok_or(StoreError::NotFound)?;
            let row = sqlx::query(
                "UPDATE ple_account SET normalized_email = $2, delivery_email = $3, \
                 updated_at = transaction_timestamp() WHERE user_id = $1 \
                 RETURNING user_id, normalized_email, delivery_email, display_name, platform_roles, \
                 floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                 floor(extract(epoch FROM updated_at) * 1000)::bigint AS updated_at_millis",
            )
            .bind(user.as_uuid())
            .bind(email.normalized())
            .bind(email.delivery())
            .fetch_optional(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
            (decode_account(&row)?, false)
        }
        _ => {
            return Err(StoreError::Unavailable(
                "stored email purpose is invalid".to_string(),
            ));
        }
    };
    // Challenge consumption and mailbox quota recovery are one transaction:
    // an attacker cannot release a target mailbox budget without its secret.
    sqlx::query(
        "DELETE FROM authentication_rate_limit WHERE limit_scope = 'email' AND key_hash = $1",
    )
    .bind(email_rate_limit_key.as_bytes().to_vec())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(CompletedEmailAuthentication { account, created })
}

pub(super) async fn insert_account_session(
    transaction: &mut Transaction<'_, Postgres>,
    token_hash: AccountSessionTokenHash,
    user: UserId,
    lifetime: AccountSessionLifetime,
) -> Result<AccountSessionRecord, StoreError> {
    let row = sqlx::query(
        "INSERT INTO account_authentication_session (\
             token_hash, user_id, expires_at\
         ) VALUES ($1, $2, transaction_timestamp() + ($3::bigint * interval '1 second')) \
         RETURNING token_hash, user_id, \
           floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
           floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis",
    )
    .bind(token_hash.as_bytes().to_vec())
    .bind(user.as_uuid())
    .bind(i64::from(lifetime.as_seconds()))
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    decode_account_session(&row)
}

const ACCOUNT_SELECT: &str = "SELECT user_id, normalized_email, delivery_email, display_name, platform_roles, \
 floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
 floor(extract(epoch FROM updated_at) * 1000)::bigint AS updated_at_millis \
 FROM ple_account WHERE user_id = $1";

const ACCOUNT_BY_EMAIL_SELECT: &str = "SELECT user_id, normalized_email, delivery_email, \
 display_name, platform_roles, floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
 floor(extract(epoch FROM updated_at) * 1000)::bigint AS updated_at_millis \
 FROM ple_account WHERE normalized_email = $1 FOR UPDATE";

const PASSKEY_SELECT: &str = "SELECT passkey_id, user_id, credential_id_hash, label, credential, \
 floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
 floor(extract(epoch FROM last_used_at) * 1000)::bigint AS last_used_at_millis, \
 floor(extract(epoch FROM revoked_at) * 1000)::bigint AS revoked_at_millis \
 FROM account_passkey WHERE user_id = $1 AND revoked_at IS NULL ORDER BY passkey_id";

const PASSKEY_BY_CREDENTIAL_SELECT: &str = "SELECT passkey_id, user_id, credential_id_hash, \
 label, credential, floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
 floor(extract(epoch FROM last_used_at) * 1000)::bigint AS last_used_at_millis, \
 floor(extract(epoch FROM revoked_at) * 1000)::bigint AS revoked_at_millis \
 FROM account_passkey WHERE credential_id_hash = $1 AND revoked_at IS NULL";

async fn load_account_by_email(
    transaction: &mut Transaction<'_, Postgres>,
    email: &str,
) -> Result<Option<AccountRecord>, StoreError> {
    let row = sqlx::query(ACCOUNT_BY_EMAIL_SELECT)
        .bind(email)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    row.as_ref().map(decode_account).transpose()
}

async fn insert_account(
    transaction: &mut Transaction<'_, Postgres>,
    user: UserId,
    email: &AuthenticationEmail,
    display_name: &str,
) -> Result<AccountRecord, StoreError> {
    let row = sqlx::query(
        "INSERT INTO ple_account (user_id, normalized_email, delivery_email, display_name) \
         VALUES ($1, $2, $3, $4) RETURNING user_id, normalized_email, delivery_email, \
         display_name, platform_roles, floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
         floor(extract(epoch FROM updated_at) * 1000)::bigint AS updated_at_millis",
    )
    .bind(user.as_uuid())
    .bind(email.normalized())
    .bind(email.delivery())
    .bind(display_name)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    decode_account(&row)
}

pub(super) fn decode_account(row: &PgRow) -> Result<AccountRecord, StoreError> {
    let normalized: String = row.try_get("normalized_email").map_err(map_sqlx_error)?;
    let delivery: String = row.try_get("delivery_email").map_err(map_sqlx_error)?;
    let Json(platform_roles): Json<Vec<UserRole>> =
        row.try_get("platform_roles").map_err(map_sqlx_error)?;
    if platform_roles
        .iter()
        .any(|role| *role != UserRole::Sysadmin)
    {
        return Err(StoreError::Unavailable(
            "stored account platform role is invalid".to_string(),
        ));
    }
    Ok(AccountRecord {
        user: UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_error)?),
        email: decode_email(&normalized, &delivery)?,
        display_name: row.try_get("display_name").map_err(map_sqlx_error)?,
        platform_roles,
        created_at: timestamp(row, "created_at_millis")?,
        updated_at: timestamp(row, "updated_at_millis")?,
    })
}

fn decode_account_course_context(row: &PgRow) -> Result<AccountCourseContext, StoreError> {
    let role: String = row.try_get("role").map_err(map_sqlx_error)?;
    let role = match role.as_str() {
        "student" => CourseMembershipRole::Student,
        "instructor" => CourseMembershipRole::Instructor,
        _ => {
            return Err(StoreError::Unavailable(
                "stored account course role is invalid".to_string(),
            ));
        }
    };
    Ok(AccountCourseContext {
        tenant: TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?),
        course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        title: row.try_get("title").map_err(map_sqlx_error)?,
        role,
    })
}

fn account_course_cursor(context: &AccountCourseContext) -> String {
    format!("{}/{}", context.tenant, context.course)
}

fn decode_account_course_cursor(value: &str) -> Result<(TenantId, CourseId), StoreError> {
    let (tenant, course) = value
        .split_once('/')
        .ok_or_else(|| StoreError::InvalidRecord("account course cursor is invalid".to_string()))?;
    if course.contains('/') {
        return Err(StoreError::InvalidRecord(
            "account course cursor is invalid".to_string(),
        ));
    }
    Ok((
        TenantId::from_uuid(Uuid::parse_str(tenant).map_err(|_| {
            StoreError::InvalidRecord("account course cursor is invalid".to_string())
        })?),
        CourseId::from_uuid(Uuid::parse_str(course).map_err(|_| {
            StoreError::InvalidRecord("account course cursor is invalid".to_string())
        })?),
    ))
}

fn decode_account_session(row: &PgRow) -> Result<AccountSessionRecord, StoreError> {
    Ok(AccountSessionRecord {
        token_hash: AccountSessionTokenHash::from_bytes(fixed_hash(row, "token_hash")?),
        user: UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_error)?),
        created_at: timestamp(row, "created_at_millis")?,
        expires_at: timestamp(row, "expires_at_millis")?,
    })
}

fn decode_email(normalized: &str, delivery: &str) -> Result<AuthenticationEmail, StoreError> {
    let email = AuthenticationEmail::parse(delivery)
        .map_err(|_| StoreError::Unavailable("stored account email is invalid".to_string()))?;
    (email.normalized() == normalized)
        .then_some(email)
        .ok_or_else(|| StoreError::Unavailable("stored email normalization mismatch".to_string()))
}

fn decode_email_challenge(row: &PgRow) -> Result<EmailAuthenticationChallenge, StoreError> {
    let normalized: String = row.try_get("normalized_email").map_err(map_sqlx_error)?;
    let delivery: String = row.try_get("delivery_email").map_err(map_sqlx_error)?;
    let purpose: String = row.try_get("purpose").map_err(map_sqlx_error)?;
    let purpose_user: Option<Uuid> = row.try_get("purpose_user_id").map_err(map_sqlx_error)?;
    Ok(EmailAuthenticationChallenge {
        id: EmailChallengeId::from_uuid(row.try_get("challenge_id").map_err(map_sqlx_error)?),
        token_hash: EmailChallengeSecretHash::from_bytes(fixed_hash(row, "token_hash")?),
        browser_binding: BrowserBindingHash::from_bytes(fixed_hash(row, "browser_binding_hash")?),
        email_rate_limit_key: AuthenticationRateLimitKey::from_bytes(fixed_hash(
            row,
            "rate_limit_key_hash",
        )?),
        email: decode_email(&normalized, &delivery)?,
        purpose: decode_purpose(&purpose, purpose_user)?,
        created_at: timestamp(row, "created_at_millis")?,
        expires_at: timestamp(row, "expires_at_millis")?,
    })
}

fn decode_ceremony(row: &PgRow) -> Result<WebauthnCeremony, StoreError> {
    let kind: String = row.try_get("ceremony_kind").map_err(map_sqlx_error)?;
    let user: Option<Uuid> = row.try_get("user_id").map_err(map_sqlx_error)?;
    let Json(state): Json<Value> = row.try_get("state").map_err(map_sqlx_error)?;
    Ok(WebauthnCeremony {
        id: WebauthnCeremonyId::from_uuid(row.try_get("ceremony_id").map_err(map_sqlx_error)?),
        kind: decode_ceremony_kind(&kind, user)?,
        browser_binding: BrowserBindingHash::from_bytes(fixed_hash(row, "browser_binding_hash")?),
        state: state_from_value(state)?,
        expires_at: timestamp(row, "expires_at_millis")?,
    })
}

pub(super) fn decode_passkey(row: &PgRow) -> Result<PasskeyRecord, StoreError> {
    let Json(credential): Json<Value> = row.try_get("credential").map_err(map_sqlx_error)?;
    Ok(PasskeyRecord {
        id: PasskeyId::from_uuid(row.try_get("passkey_id").map_err(map_sqlx_error)?),
        user: UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_error)?),
        credential_id_hash: CredentialIdHash::from_bytes(fixed_hash(row, "credential_id_hash")?),
        label: row.try_get("label").map_err(map_sqlx_error)?,
        credential: state_from_value(credential)?,
        created_at: timestamp(row, "created_at_millis")?,
        last_used_at: optional_timestamp(row, "last_used_at_millis")?,
        revoked_at: optional_timestamp(row, "revoked_at_millis")?,
    })
}

fn purpose_columns(value: EmailAuthenticationPurpose) -> (&'static str, Option<UserId>) {
    match value {
        EmailAuthenticationPurpose::SignInOrRegister => ("sign_in_or_register", None),
        EmailAuthenticationPurpose::ChangeEmail { user } => ("change_email", Some(user)),
    }
}

fn rate_limit_scope(value: AuthenticationRateLimitScope) -> &'static str {
    match value {
        AuthenticationRateLimitScope::Email => "email",
        AuthenticationRateLimitScope::Network => "network",
        AuthenticationRateLimitScope::Principal => "principal",
        AuthenticationRateLimitScope::Service => "service",
    }
}

fn decode_purpose(
    value: &str,
    user: Option<Uuid>,
) -> Result<EmailAuthenticationPurpose, StoreError> {
    match (value, user) {
        ("sign_in_or_register", None) => Ok(EmailAuthenticationPurpose::SignInOrRegister),
        ("change_email", Some(user)) => Ok(EmailAuthenticationPurpose::ChangeEmail {
            user: UserId::from_uuid(user),
        }),
        _ => Err(StoreError::Unavailable(
            "stored email purpose is invalid".to_string(),
        )),
    }
}

fn ceremony_kind_columns(value: WebauthnCeremonyKind) -> (&'static str, Option<UserId>) {
    match value {
        WebauthnCeremonyKind::Registration { user } => ("registration", Some(user)),
        WebauthnCeremonyKind::Authentication { user } => ("authentication", user),
    }
}

fn decode_ceremony_kind(
    value: &str,
    user: Option<Uuid>,
) -> Result<WebauthnCeremonyKind, StoreError> {
    match (value, user) {
        ("registration", Some(user)) => Ok(WebauthnCeremonyKind::Registration {
            user: UserId::from_uuid(user),
        }),
        ("authentication", user) => Ok(WebauthnCeremonyKind::Authentication {
            user: user.map(UserId::from_uuid),
        }),
        _ => Err(StoreError::Unavailable(
            "stored ceremony kind is invalid".to_string(),
        )),
    }
}

pub(super) fn json_value(state: &WebauthnState) -> Result<Value, StoreError> {
    serde_json::from_slice(state.as_bytes())
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}

fn state_from_value(value: Value) -> Result<WebauthnState, StoreError> {
    WebauthnState::new(
        serde_json::to_vec(&value).map_err(|error| StoreError::Unavailable(error.to_string()))?,
    )
    .map_err(|error| StoreError::Unavailable(error.to_string()))
}

fn fixed_hash(row: &PgRow, column: &str) -> Result<[u8; 32], StoreError> {
    let value: Vec<u8> = row.try_get(column).map_err(map_sqlx_error)?;
    value
        .try_into()
        .map_err(|_| StoreError::Unavailable(format!("stored {column} is invalid")))
}

fn timestamp(row: &PgRow, column: &str) -> Result<ActivityTimestamp, StoreError> {
    Ok(ActivityTimestamp::from_unix_millis(
        row.try_get(column).map_err(map_sqlx_error)?,
    ))
}

fn optional_timestamp(row: &PgRow, column: &str) -> Result<Option<ActivityTimestamp>, StoreError> {
    Ok(row
        .try_get::<Option<i64>, _>(column)
        .map_err(map_sqlx_error)?
        .map(ActivityTimestamp::from_unix_millis))
}

fn one_row(rows: u64) -> Result<(), StoreError> {
    (rows == 1).then_some(()).ok_or(StoreError::NotFound)
}
