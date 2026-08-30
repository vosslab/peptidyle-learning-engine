//! In-memory passwordless account and credential persistence.

use async_trait::async_trait;

use super::MemoryStore;
use crate::{
    AccountCourseContext, AccountIdentityStore, AccountRecord, AccountSessionLifetime,
    AccountSessionRecord, AccountSessionStore, AccountSessionTokenHash,
    AuthenticationRateLimitDecision, AuthenticationRateLimitScope, BeginEmailAuthentication,
    BeginWebauthnCeremony, CompleteEmailAuthentication,
    CompleteEmailAuthenticationAndCreateSession, CompleteEmailChangeAndRevokeUserSessions,
    CompletePasskeyAuthenticationAndCreateSession, CompletedAccountSession,
    CompletedEmailAuthentication, CompletedPasskeySession, ConsumeAuthenticationRateLimit,
    EmailAuthenticationChallenge, EmailAuthenticationPurpose, LiveDemoInstallationStore, Page,
    PageRequest, PasskeyId, PasskeyRecord, RegisterPasskey, StoreError, WebauthnCeremony,
    WebauthnCeremonyId, validated_account_display_name,
};
use uuid::Uuid;

/// Private installation lifecycle retained with its account-facing reader.
#[cfg_attr(not(feature = "test-support"), allow(dead_code))]
#[derive(Debug, Default, Clone)]
pub(super) enum StoredLiveDemoInstallationState {
    #[default]
    Missing,
    Installing {
        generation: Uuid,
    },
    Complete {
        generation: Uuid,
    },
}

#[async_trait]
impl LiveDemoInstallationStore for MemoryStore {
    async fn completed_live_demo_installation_generation(
        &self,
    ) -> Result<Option<Uuid>, StoreError> {
        let state = self.read_state()?;
        Ok(match state.live_demo_installation_state {
            StoredLiveDemoInstallationState::Complete { generation } => Some(generation),
            StoredLiveDemoInstallationState::Missing => None,
            StoredLiveDemoInstallationState::Installing { generation } => {
                let _ = generation;
                None
            }
        })
    }
}
use question_model::{ActivityTimestamp, CourseId, CourseMembershipRole, UserId};

#[derive(Debug, Clone)]
pub(super) struct StoredAuthenticationRateLimit {
    pub(super) window_started_at: ActivityTimestamp,
    pub(super) attempts: u32,
    pub(super) window_seconds: u32,
}

#[derive(Debug, Clone)]
pub(super) struct StoredEmailChallenge {
    pub(super) record: EmailAuthenticationChallenge,
}

#[async_trait]
impl AccountSessionStore for MemoryStore {
    async fn create_account_session(
        &self,
        token_hash: AccountSessionTokenHash,
        user: question_model::UserId,
        lifetime: AccountSessionLifetime,
    ) -> Result<AccountSessionRecord, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        state
            .account_sessions
            .retain(|_, session| session.expires_at > now);
        if !state.accounts.contains_key(&user) {
            return Err(StoreError::NotFound);
        }
        if state.account_sessions.contains_key(&token_hash) {
            return Err(StoreError::AlreadyExists);
        }
        let record = AccountSessionRecord {
            token_hash,
            user,
            created_at: state.authoritative_time,
            expires_at: timestamp_after_seconds(state.authoritative_time, lifetime.as_seconds())?,
        };
        state.account_sessions.insert(token_hash, record.clone());
        Ok(record)
    }

    async fn resolve_account_session(
        &self,
        token_hash: AccountSessionTokenHash,
    ) -> Result<Option<AccountSessionRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .account_sessions
            .get(&token_hash)
            .filter(|session| session.expires_at > state.authoritative_time)
            .cloned())
    }

    async fn revoke_account_session(
        &self,
        token_hash: AccountSessionTokenHash,
    ) -> Result<(), StoreError> {
        self.write_state()?.account_sessions.remove(&token_hash);
        Ok(())
    }
}

#[async_trait]
impl AccountIdentityStore for MemoryStore {
    async fn consume_authentication_rate_limit(
        &self,
        command: ConsumeAuthenticationRateLimit,
    ) -> Result<AuthenticationRateLimitDecision, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        state.authentication_rate_limits.retain(|_, record| {
            timestamp_after_seconds(record.window_started_at, record.window_seconds)
                .is_ok_and(|expires_at| expires_at > now)
        });
        let key = (command.scope, command.key);
        let record =
            state
                .authentication_rate_limits
                .entry(key)
                .or_insert(StoredAuthenticationRateLimit {
                    window_started_at: now,
                    attempts: 0,
                    window_seconds: command.policy.window_seconds(),
                });
        let window_millis = i64::from(command.policy.window_seconds())
            .checked_mul(1_000)
            .ok_or_else(|| StoreError::InvalidRecord("rate-limit window overflow".to_string()))?;
        let window_ends_at = record
            .window_started_at
            .as_unix_millis()
            .checked_add(window_millis)
            .ok_or_else(|| StoreError::InvalidRecord("rate-limit window overflow".to_string()))?;
        if now.as_unix_millis() >= window_ends_at {
            record.window_started_at = now;
            record.attempts = 0;
            record.window_seconds = command.policy.window_seconds();
        }
        record.attempts = record
            .attempts
            .saturating_add(1)
            .min(command.policy.maximum_attempts().saturating_add(1));
        if record.attempts <= command.policy.maximum_attempts() {
            return Ok(AuthenticationRateLimitDecision::Allowed {
                remaining_attempts: command.policy.maximum_attempts() - record.attempts,
            });
        }
        let remaining_millis = window_ends_at.saturating_sub(now.as_unix_millis()).max(1);
        let retry_after_seconds = u32::try_from((remaining_millis + 999) / 1_000)
            .map_err(|_| StoreError::Unavailable("rate-limit window is invalid".to_string()))?;
        Ok(AuthenticationRateLimitDecision::Denied {
            retry_after_seconds,
        })
    }

    async fn begin_email_authentication(
        &self,
        command: BeginEmailAuthentication,
    ) -> Result<EmailAuthenticationChallenge, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        state
            .email_challenges
            .retain(|_, challenge| challenge.record.expires_at > now);
        state.email_challenges.retain(|_, challenge| {
            challenge.record.email != command.email || challenge.record.purpose != command.purpose
        });
        if state.email_challenges.contains_key(&command.token_hash) {
            return Err(StoreError::AlreadyExists);
        }
        let created_at = state.authoritative_time;
        let expires_at = timestamp_after_seconds(created_at, command.lifetime.as_seconds())?;
        let record = EmailAuthenticationChallenge {
            id: command.id,
            token_hash: command.token_hash,
            browser_binding: command.browser_binding,
            email_rate_limit_key: command.email_rate_limit_key,
            email: command.email,
            purpose: command.purpose,
            created_at,
            expires_at,
        };
        state.email_challenges.insert(
            command.token_hash,
            StoredEmailChallenge {
                record: record.clone(),
            },
        );
        Ok(record)
    }

    async fn complete_email_authentication(
        &self,
        command: CompleteEmailAuthentication,
    ) -> Result<CompletedEmailAuthentication, StoreError> {
        let mut state = self.write_state()?;
        complete_email_authentication_locked(&mut state, command)
    }

    async fn complete_email_authentication_and_create_session(
        &self,
        command: CompleteEmailAuthenticationAndCreateSession,
    ) -> Result<CompletedAccountSession, StoreError> {
        let mut state = self.write_state()?;
        let rollback = state.clone();
        let result = (|| {
            if state
                .account_sessions
                .contains_key(&command.session_token_hash)
            {
                return Err(StoreError::AlreadyExists);
            }
            let authentication =
                complete_email_authentication_locked(&mut state, command.authentication)?;
            let session = AccountSessionRecord {
                token_hash: command.session_token_hash,
                user: authentication.account.user,
                created_at: state.authoritative_time,
                expires_at: timestamp_after_seconds(
                    state.authoritative_time,
                    command.session_lifetime.as_seconds(),
                )?,
            };
            state
                .account_sessions
                .insert(command.session_token_hash, session.clone());
            Ok(CompletedAccountSession {
                authentication,
                session,
            })
        })();
        if result.is_err() {
            *state = rollback;
        }
        result
    }

    async fn complete_email_change_and_revoke_user_sessions(
        &self,
        command: CompleteEmailChangeAndRevokeUserSessions,
    ) -> Result<CompletedAccountSession, StoreError> {
        let mut state = self.write_state()?;
        let rollback = state.clone();
        let result = (|| {
            if !matches!(
                state
                    .email_challenges
                    .get(&command.authentication.token_hash)
                    .map(|challenge| &challenge.record.purpose),
                Some(EmailAuthenticationPurpose::ChangeEmail { .. })
            ) {
                return Err(StoreError::NotFound);
            }
            if state
                .account_sessions
                .contains_key(&command.session_token_hash)
            {
                return Err(StoreError::AlreadyExists);
            }
            let authentication =
                complete_email_authentication_locked(&mut state, command.authentication)?;
            if authentication.created {
                return Err(StoreError::InvalidRecord(
                    "email replacement must not create an account".to_string(),
                ));
            }
            let user = authentication.account.user;
            state
                .account_sessions
                .retain(|_, session| session.user != user);
            for stored in state.sessions.values_mut() {
                if stored.record.subject.user() == user {
                    stored.revoked = true;
                }
            }
            let session = AccountSessionRecord {
                token_hash: command.session_token_hash,
                user,
                created_at: state.authoritative_time,
                expires_at: timestamp_after_seconds(
                    state.authoritative_time,
                    command.session_lifetime.as_seconds(),
                )?,
            };
            state
                .account_sessions
                .insert(command.session_token_hash, session.clone());
            Ok(CompletedAccountSession {
                authentication,
                session,
            })
        })();
        if result.is_err() {
            *state = rollback;
        }
        result
    }

    async fn get_account(
        &self,
        user: question_model::UserId,
    ) -> Result<Option<AccountRecord>, StoreError> {
        Ok(self.read_state()?.accounts.get(&user).cloned())
    }

    async fn list_account_course_contexts(
        &self,
        user: UserId,
        page: PageRequest,
    ) -> Result<Page<AccountCourseContext>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .courses
            .iter()
            .filter_map(|(course, record)| {
                let role = super::entitlement::current_course_role(&state, *course, user)?;
                if role == CourseMembershipRole::Student
                    && !super::course_records_accessible(&state, *course)
                {
                    return None;
                }
                Some((
                    course.to_string(),
                    AccountCourseContext {
                        course: *course,
                        title: record.title.clone(),
                        role,
                    },
                ))
            })
            .collect();
        Ok(super::page_records(records, &page))
    }

    async fn resolve_account_course_context(
        &self,
        user: UserId,
        course: CourseId,
    ) -> Result<Option<AccountCourseContext>, StoreError> {
        let state = self.read_state()?;
        let mut matches = state.courses.iter().filter_map(|(stored_course, record)| {
            if *stored_course != course {
                return None;
            }
            let role = super::entitlement::current_course_role(&state, *stored_course, user)?;
            if role == CourseMembershipRole::Student
                && !super::course_records_accessible(&state, *stored_course)
            {
                return None;
            }
            Some(AccountCourseContext {
                course: *stored_course,
                title: record.title.clone(),
                role,
            })
        });
        let result = matches.next();
        if matches.next().is_some() {
            return Err(StoreError::Unavailable(
                "course identity is ambiguous across account contexts".to_string(),
            ));
        }
        Ok(result)
    }

    async fn begin_webauthn_ceremony(
        &self,
        command: BeginWebauthnCeremony,
    ) -> Result<WebauthnCeremony, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        state
            .webauthn_ceremonies
            .retain(|_, existing| existing.expires_at > now);
        let ceremony = WebauthnCeremony {
            id: command.id,
            kind: command.kind,
            browser_binding: command.browser_binding,
            state: command.state,
            expires_at: timestamp_after_seconds(now, command.lifetime.as_seconds())?,
        };
        if state
            .webauthn_ceremonies
            .insert(ceremony.id, ceremony.clone())
            .is_some()
        {
            return Err(StoreError::AlreadyExists);
        }
        Ok(ceremony)
    }

    async fn take_webauthn_ceremony(
        &self,
        id: WebauthnCeremonyId,
        browser_binding: crate::BrowserBindingHash,
    ) -> Result<Option<WebauthnCeremony>, StoreError> {
        let mut state = self.write_state()?;
        let Some(ceremony) = state.webauthn_ceremonies.get(&id) else {
            return Ok(None);
        };
        if !constant_time_eq(
            &ceremony.browser_binding.as_bytes(),
            &browser_binding.as_bytes(),
        ) || ceremony.expires_at <= state.authoritative_time
        {
            return Ok(None);
        }
        Ok(state.webauthn_ceremonies.remove(&id))
    }

    async fn get_webauthn_ceremony(
        &self,
        id: WebauthnCeremonyId,
        browser_binding: crate::BrowserBindingHash,
    ) -> Result<Option<WebauthnCeremony>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .webauthn_ceremonies
            .get(&id)
            .filter(|ceremony| {
                constant_time_eq(
                    &ceremony.browser_binding.as_bytes(),
                    &browser_binding.as_bytes(),
                ) && ceremony.expires_at > state.authoritative_time
            })
            .cloned())
    }

    async fn insert_passkey(&self, command: RegisterPasskey) -> Result<PasskeyRecord, StoreError> {
        let mut state = self.write_state()?;
        if !state.accounts.contains_key(&command.user) {
            return Err(StoreError::NotFound);
        }
        if state.passkeys.contains_key(&command.id)
            || state
                .passkey_by_credential
                .contains_key(&command.credential_id_hash)
        {
            return Err(StoreError::AlreadyExists);
        }
        let passkey = PasskeyRecord {
            id: command.id,
            user: command.user,
            credential_id_hash: command.credential_id_hash,
            label: command.label,
            credential: command.credential,
            created_at: state.authoritative_time,
            last_used_at: None,
            revoked_at: None,
        };
        state
            .passkey_by_credential
            .insert(passkey.credential_id_hash, passkey.id);
        state.passkeys.insert(passkey.id, passkey.clone());
        Ok(passkey)
    }

    async fn list_active_passkeys(
        &self,
        user: question_model::UserId,
    ) -> Result<Vec<PasskeyRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .passkeys
            .values()
            .filter(|passkey| passkey.user == user && passkey.revoked_at.is_none())
            .cloned()
            .collect())
    }

    async fn get_active_passkey_by_credential_id_hash(
        &self,
        credential_id_hash: crate::CredentialIdHash,
    ) -> Result<Option<PasskeyRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .passkey_by_credential
            .get(&credential_id_hash)
            .and_then(|passkey| state.passkeys.get(passkey))
            .filter(|passkey| passkey.revoked_at.is_none())
            .cloned())
    }

    async fn replace_passkey_after_authentication(
        &self,
        passkey: PasskeyRecord,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let existing = state
            .passkeys
            .get(&passkey.id)
            .ok_or(StoreError::NotFound)?;
        if existing.user != passkey.user
            || existing.credential_id_hash != passkey.credential_id_hash
            || existing.revoked_at.is_some()
            || passkey.revoked_at.is_some()
        {
            return Err(StoreError::Conflict);
        }
        state.passkeys.insert(passkey.id, passkey);
        Ok(())
    }

    async fn complete_passkey_authentication_and_create_session(
        &self,
        command: CompletePasskeyAuthenticationAndCreateSession,
    ) -> Result<CompletedPasskeySession, StoreError> {
        let mut state = self.write_state()?;
        let rollback = state.clone();
        let result = (|| {
            let existing = state
                .passkeys
                .get(&command.passkey.id)
                .ok_or(StoreError::NotFound)?;
            if existing.user != command.passkey.user
                || existing.credential_id_hash != command.passkey.credential_id_hash
                || existing.revoked_at.is_some()
                || command.passkey.revoked_at.is_some()
                || state
                    .account_sessions
                    .contains_key(&command.session_token_hash)
            {
                return Err(StoreError::Conflict);
            }
            let mut passkey = command.passkey;
            passkey.last_used_at = Some(state.authoritative_time);
            state.passkeys.insert(passkey.id, passkey.clone());
            let session = AccountSessionRecord {
                token_hash: command.session_token_hash,
                user: passkey.user,
                created_at: state.authoritative_time,
                expires_at: timestamp_after_seconds(
                    state.authoritative_time,
                    command.session_lifetime.as_seconds(),
                )?,
            };
            state
                .account_sessions
                .insert(command.session_token_hash, session.clone());
            Ok(CompletedPasskeySession { passkey, session })
        })();
        if result.is_err() {
            *state = rollback;
        }
        result
    }

    async fn revoke_passkey(
        &self,
        user: question_model::UserId,
        passkey: PasskeyId,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let record = state
            .passkeys
            .get_mut(&passkey)
            .ok_or(StoreError::NotFound)?;
        if record.user != user {
            return Err(StoreError::NotFound);
        }
        record.revoked_at.get_or_insert(now);
        Ok(())
    }
}

fn complete_email_authentication_locked(
    state: &mut super::State,
    command: CompleteEmailAuthentication,
) -> Result<CompletedEmailAuthentication, StoreError> {
    let challenge = state
        .email_challenges
        .get(&command.token_hash)
        .filter(|challenge| {
            constant_time_eq(
                &challenge.record.browser_binding.as_bytes(),
                &command.browser_binding.as_bytes(),
            ) && challenge.record.expires_at > state.authoritative_time
        })
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let display_name = validated_account_display_name(&command.proposed_display_name)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;

    let (account, created) = match challenge.record.purpose {
        EmailAuthenticationPurpose::ChangeEmail { user } => {
            if user != command.proposed_user {
                return Err(StoreError::Forbidden);
            }
            if state
                .account_by_email
                .get(challenge.record.email.normalized())
                .is_some_and(|existing| *existing != user)
            {
                return Err(StoreError::Conflict);
            }
            let mut account = state.accounts.remove(&user).ok_or(StoreError::NotFound)?;
            state.account_by_email.remove(account.email.normalized());
            account.email = challenge.record.email.clone();
            account.updated_at = state.authoritative_time;
            state
                .account_by_email
                .insert(account.email.normalized().to_string(), user);
            state.accounts.insert(user, account.clone());
            (account, false)
        }
        EmailAuthenticationPurpose::SignInOrRegister => {
            if let Some(user) = state
                .account_by_email
                .get(challenge.record.email.normalized())
                .copied()
            {
                let account = state.accounts.get(&user).cloned().ok_or_else(|| {
                    StoreError::Unavailable(
                        "account email index points to a missing account".to_string(),
                    )
                })?;
                (account, false)
            } else {
                if state.accounts.contains_key(&command.proposed_user) {
                    return Err(StoreError::AlreadyExists);
                }
                let account = AccountRecord {
                    user: command.proposed_user,
                    email: challenge.record.email.clone(),
                    display_name,
                    platform_roles: Vec::new(),
                    created_at: state.authoritative_time,
                    updated_at: state.authoritative_time,
                };
                state.account_by_email.insert(
                    account.email.normalized().to_string(),
                    command.proposed_user,
                );
                state
                    .accounts
                    .insert(command.proposed_user, account.clone());
                (account, true)
            }
        }
    };
    state.email_challenges.remove(&command.token_hash);
    // Mailbox proof is the only recovery authority for this quota. It never
    // clears a shared network, principal, or service budget.
    state.authentication_rate_limits.remove(&(
        AuthenticationRateLimitScope::Email,
        challenge.record.email_rate_limit_key,
    ));
    Ok(CompletedEmailAuthentication { account, created })
}

fn timestamp_after_seconds(
    timestamp: ActivityTimestamp,
    seconds: u32,
) -> Result<ActivityTimestamp, StoreError> {
    let millis = i64::from(seconds)
        .checked_mul(1_000)
        .and_then(|value| timestamp.as_unix_millis().checked_add(value))
        .ok_or_else(|| StoreError::InvalidRecord("authentication expiry overflow".to_string()))?;
    Ok(ActivityTimestamp::from_unix_millis(millis))
}

fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}
