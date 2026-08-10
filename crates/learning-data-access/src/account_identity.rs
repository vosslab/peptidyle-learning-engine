//! PLE-owned passwordless account persistence.
//!
//! This capability is deliberately separate from [`crate::Store`]. Course and
//! run code must not gain access to authentication emails, passkey material,
//! or in-progress credential ceremonies merely because it can read tenant
//! educational records.

use std::num::NonZeroU32;

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{ActivityTimestamp, CourseId, CourseRole, TenantId, UserId};
use uuid::Uuid;

use crate::{Page, PageRequest, StoreError};

/// Maximum accepted authentication-email length after whitespace removal.
pub const MAX_AUTHENTICATION_EMAIL_BYTES: usize = 320;
/// Maximum user-controlled account label length.
pub const MAX_ACCOUNT_DISPLAY_NAME_CHARS: usize = 200;
/// Maximum user-controlled passkey label length.
pub const MAX_PASSKEY_LABEL_CHARS: usize = 80;
/// Maximum serialized WebAuthn credential or ceremony state.
pub const MAX_WEBAUTHN_STATE_BYTES: usize = 64 * 1_024;

/// Validated authentication email with a stable lookup form and delivery form.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthenticationEmail {
    normalized: String,
    delivery: String,
    domain: EmailDomain,
}

impl AuthenticationEmail {
    /// Validates a deliberately conservative mailbox form.
    ///
    /// PLE lowercases the ASCII local part for account lookup and preserves the
    /// original spelling for delivery. The domain is IDNA-normalized and
    /// lowercased. Quoted local parts and SMTP comments are intentionally
    /// unsupported at the HTTP boundary.
    pub fn parse(value: &str) -> Result<Self, AccountIdentityError> {
        let delivery = value.trim();
        if delivery.is_empty() || delivery.len() > MAX_AUTHENTICATION_EMAIL_BYTES {
            return Err(AccountIdentityError::InvalidEmail);
        }
        let (local, domain) = delivery
            .rsplit_once('@')
            .ok_or(AccountIdentityError::InvalidEmail)?;
        if local.is_empty()
            || local.len() > 64
            || local.starts_with('.')
            || local.ends_with('.')
            || local.contains("..")
            || local.contains('@')
            || !local.bytes().all(valid_local_part_byte)
        {
            return Err(AccountIdentityError::InvalidEmail);
        }
        let domain = EmailDomain::parse(domain).map_err(|_| AccountIdentityError::InvalidEmail)?;
        let normalized = format!("{}@{}", local.to_ascii_lowercase(), domain.as_str());
        Ok(Self {
            normalized,
            delivery: delivery.to_string(),
            domain,
        })
    }

    /// Canonical account lookup value.
    pub fn normalized(&self) -> &str {
        &self.normalized
    }

    /// Address spelling retained for delivery.
    pub fn delivery(&self) -> &str {
        &self.delivery
    }

    /// Complete normalized domain used by course policy.
    pub fn domain(&self) -> &EmailDomain {
        &self.domain
    }
}

impl std::fmt::Debug for AuthenticationEmail {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AuthenticationEmail([redacted])")
    }
}

fn valid_local_part_byte(value: u8) -> bool {
    value.is_ascii_alphanumeric()
        || matches!(
            value,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'/'
                | b'='
                | b'?'
                | b'^'
                | b'_'
                | b'`'
                | b'{'
                | b'|'
                | b'}'
                | b'~'
        )
}

/// Complete normalized email domain, never a substring or suffix pattern.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmailDomain(String);

impl EmailDomain {
    /// Applies strict IDNA conversion and validates DNS label boundaries.
    pub fn parse(value: &str) -> Result<Self, AccountIdentityError> {
        let value = value.trim().trim_end_matches('.');
        if value.is_empty() || value.len() > 253 || value.contains('@') {
            return Err(AccountIdentityError::InvalidEmailDomain);
        }
        let ascii = idna::domain_to_ascii_strict(value)
            .map_err(|_| AccountIdentityError::InvalidEmailDomain)?
            .to_ascii_lowercase();
        if ascii.is_empty()
            || ascii.len() > 253
            || !ascii.contains('.')
            || ascii.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
        {
            return Err(AccountIdentityError::InvalidEmailDomain);
        }
        Ok(Self(ascii))
    }

    /// Canonical full-domain comparison value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable PLE-owned account data; credential material is stored separately.
#[derive(Clone, PartialEq, Eq)]
pub struct AccountRecord {
    pub user: UserId,
    pub email: AuthenticationEmail,
    pub display_name: String,
    pub created_at: ActivityTimestamp,
    pub updated_at: ActivityTimestamp,
}

/// One course relationship proven from a PLE account, not browser authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountCourseContext {
    pub tenant: TenantId,
    pub course: CourseId,
    pub title: String,
    pub role: CourseRole,
}

impl std::fmt::Debug for AccountRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AccountRecord")
            .field("user", &self.user)
            .field("email", &"[redacted]")
            .field("display_name", &self.display_name)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Validates a user-controlled display label.
pub fn validated_account_display_name(value: &str) -> Result<String, AccountIdentityError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > MAX_ACCOUNT_DISPLAY_NAME_CHARS {
        return Err(AccountIdentityError::InvalidDisplayName);
    }
    Ok(value.to_string())
}

macro_rules! opaque_id {
    ($name:ident, $label:literal) => {
        #[doc = $label]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);

        impl $name {
            pub fn generate() -> Result<Self, StoreError> {
                let mut bytes = [0_u8; 16];
                getrandom::fill(&mut bytes).map_err(|error| {
                    StoreError::Unavailable(format!("{} randomness unavailable: {error}", $label))
                })?;
                Ok(Self(Uuid::from_bytes(bytes)))
            }

            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }
    };
}

opaque_id!(PasskeyId, "passkey ID");
opaque_id!(EmailChallengeId, "email challenge ID");
opaque_id!(WebauthnCeremonyId, "WebAuthn ceremony ID");

macro_rules! secret_hash {
    ($name:ident, $label:literal) => {
        #[doc = $label]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            pub fn compute(secret: &[u8]) -> Self {
                Self(*Sha256Digest::compute(secret).as_bytes())
            }

            pub fn from_bytes(value: [u8; 32]) -> Self {
                Self(value)
            }

            pub fn as_bytes(self) -> [u8; 32] {
                self.0
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(concat!(stringify!($name), "([redacted])"))
            }
        }
    };
}

secret_hash!(
    EmailChallengeSecretHash,
    "One-way email challenge token hash."
);
secret_hash!(
    BrowserBindingHash,
    "One-way browser-binding capability hash."
);
secret_hash!(CredentialIdHash, "One-way WebAuthn credential lookup hash.");
secret_hash!(
    AccountSessionTokenHash,
    "One-way short-lived account-session token hash."
);
secret_hash!(
    AuthenticationRateLimitKey,
    "Keyed, privacy-bounded authentication rate-limit identity."
);

/// Closed set of independently enforced passwordless-authentication limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthenticationRateLimitScope {
    Email,
    Network,
}

/// Bounded fixed-window policy owned by the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticationRateLimitPolicy {
    maximum_attempts: NonZeroU32,
    window_seconds: NonZeroU32,
}

impl AuthenticationRateLimitPolicy {
    pub const MAXIMUM_ATTEMPTS: u32 = 10_000;
    pub const MAXIMUM_WINDOW_SECONDS: u32 = 24 * 60 * 60;

    pub fn new(maximum_attempts: u32, window_seconds: u32) -> Option<Self> {
        Some(Self {
            maximum_attempts: NonZeroU32::new(maximum_attempts)
                .filter(|value| value.get() <= Self::MAXIMUM_ATTEMPTS)?,
            window_seconds: NonZeroU32::new(window_seconds)
                .filter(|value| value.get() <= Self::MAXIMUM_WINDOW_SECONDS)?,
        })
    }

    pub fn maximum_attempts(self) -> u32 {
        self.maximum_attempts.get()
    }

    pub fn window_seconds(self) -> u32 {
        self.window_seconds.get()
    }
}

/// Atomic request to consume one replica-shared authentication allowance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsumeAuthenticationRateLimit {
    pub scope: AuthenticationRateLimitScope,
    pub key: AuthenticationRateLimitKey,
    pub policy: AuthenticationRateLimitPolicy,
}

/// Server-only result; passwordless HTTP responses remain account-independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationRateLimitDecision {
    Allowed { remaining_attempts: u32 },
    Denied { retry_after_seconds: u32 },
}

/// Bounded lifetime for tenant-independent account authentication state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountSessionLifetime(NonZeroU32);

impl AccountSessionLifetime {
    pub const MAX_SECONDS: u32 = 15 * 60;

    pub fn from_seconds(seconds: u32) -> Option<Self> {
        NonZeroU32::new(seconds)
            .filter(|seconds| seconds.get() <= Self::MAX_SECONDS)
            .map(Self)
    }

    pub fn as_seconds(self) -> u32 {
        self.0.get()
    }
}

/// Short-lived account proof used before selecting an authorized tenant.
#[derive(Clone, PartialEq, Eq)]
pub struct AccountSessionRecord {
    pub token_hash: AccountSessionTokenHash,
    pub user: UserId,
    pub created_at: ActivityTimestamp,
    pub expires_at: ActivityTimestamp,
}

impl std::fmt::Debug for AccountSessionRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AccountSessionRecord")
            .field("token_hash", &"[redacted]")
            .field("user", &self.user)
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Closed purpose set for email authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailAuthenticationPurpose {
    SignInOrRegister,
    ChangeEmail { user: UserId },
}

/// Bounded lifetime for a single-use email challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmailChallengeLifetime(NonZeroU32);

impl EmailChallengeLifetime {
    pub const MAX_SECONDS: u32 = 10 * 60;

    pub fn from_seconds(seconds: u32) -> Option<Self> {
        NonZeroU32::new(seconds)
            .filter(|seconds| seconds.get() <= Self::MAX_SECONDS)
            .map(Self)
    }

    pub fn as_seconds(self) -> u32 {
        self.0.get()
    }
}

/// Server-only request to persist a freshly issued email challenge.
#[derive(Clone)]
pub struct BeginEmailAuthentication {
    pub id: EmailChallengeId,
    pub token_hash: EmailChallengeSecretHash,
    pub browser_binding: BrowserBindingHash,
    pub email: AuthenticationEmail,
    pub purpose: EmailAuthenticationPurpose,
    pub lifetime: EmailChallengeLifetime,
}

impl std::fmt::Debug for BeginEmailAuthentication {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BeginEmailAuthentication")
            .field("id", &self.id)
            .field("email", &"[redacted]")
            .field("purpose", &self.purpose)
            .field("lifetime", &self.lifetime)
            .finish_non_exhaustive()
    }
}

/// Persisted email challenge metadata. It never contains the raw token.
#[derive(Clone, PartialEq, Eq)]
pub struct EmailAuthenticationChallenge {
    pub id: EmailChallengeId,
    pub token_hash: EmailChallengeSecretHash,
    pub browser_binding: BrowserBindingHash,
    pub email: AuthenticationEmail,
    pub purpose: EmailAuthenticationPurpose,
    pub created_at: ActivityTimestamp,
    pub expires_at: ActivityTimestamp,
}

impl std::fmt::Debug for EmailAuthenticationChallenge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EmailAuthenticationChallenge")
            .field("id", &self.id)
            .field("email", &"[redacted]")
            .field("purpose", &self.purpose)
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// Atomic completion request after the browser returns the raw email secret.
#[derive(Debug, Clone)]
pub struct CompleteEmailAuthentication {
    pub token_hash: EmailChallengeSecretHash,
    pub browser_binding: BrowserBindingHash,
    pub proposed_user: UserId,
    pub proposed_display_name: String,
}

/// Account selected or created by one verified email challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedEmailAuthentication {
    pub account: AccountRecord,
    pub created: bool,
}

/// Atomic bootstrap request that consumes an email challenge and issues the
/// short-lived account proof needed by the passkey and invitation routes.
#[derive(Debug, Clone)]
pub struct CompleteEmailAuthenticationAndCreateSession {
    pub authentication: CompleteEmailAuthentication,
    pub session_token_hash: AccountSessionTokenHash,
    pub session_lifetime: AccountSessionLifetime,
}

/// Account and short-lived proof created in the same persistence transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedAccountSession {
    pub authentication: CompletedEmailAuthentication,
    pub session: AccountSessionRecord,
}

/// Opaque validated serialized state owned by the WebAuthn implementation.
#[derive(Clone, PartialEq, Eq)]
pub struct WebauthnState(Vec<u8>);

impl WebauthnState {
    pub fn new(bytes: Vec<u8>) -> Result<Self, AccountIdentityError> {
        if bytes.is_empty() || bytes.len() > MAX_WEBAUTHN_STATE_BYTES {
            return Err(AccountIdentityError::InvalidWebauthnState);
        }
        let value = serde_json::from_slice::<serde_json::Value>(&bytes)
            .map_err(|_| AccountIdentityError::InvalidWebauthnState)?;
        if !value.is_object() {
            return Err(AccountIdentityError::InvalidWebauthnState);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for WebauthnState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WebauthnState([redacted])")
    }
}

/// Type of a server-held WebAuthn ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebauthnCeremonyKind {
    Registration { user: UserId },
    Authentication { user: Option<UserId> },
}

/// Bounded lifetime for a browser-bound, single-use WebAuthn ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebauthnCeremonyLifetime(NonZeroU32);

impl WebauthnCeremonyLifetime {
    pub const MAX_SECONDS: u32 = 10 * 60;

    pub fn from_seconds(seconds: u32) -> Option<Self> {
        NonZeroU32::new(seconds)
            .filter(|seconds| seconds.get() <= Self::MAX_SECONDS)
            .map(Self)
    }

    pub fn as_seconds(self) -> u32 {
        self.0.get()
    }
}

/// Server request whose timestamps are assigned by the authoritative store.
#[derive(Clone, PartialEq, Eq)]
pub struct BeginWebauthnCeremony {
    pub id: WebauthnCeremonyId,
    pub kind: WebauthnCeremonyKind,
    pub browser_binding: BrowserBindingHash,
    pub state: WebauthnState,
    pub lifetime: WebauthnCeremonyLifetime,
}

impl std::fmt::Debug for BeginWebauthnCeremony {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BeginWebauthnCeremony")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("lifetime", &self.lifetime)
            .field("state", &"[redacted]")
            .finish_non_exhaustive()
    }
}

/// Server-held, single-use WebAuthn ceremony state.
#[derive(Clone, PartialEq, Eq)]
pub struct WebauthnCeremony {
    pub id: WebauthnCeremonyId,
    pub kind: WebauthnCeremonyKind,
    pub browser_binding: BrowserBindingHash,
    pub state: WebauthnState,
    pub expires_at: ActivityTimestamp,
}

impl std::fmt::Debug for WebauthnCeremony {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebauthnCeremony")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("expires_at", &self.expires_at)
            .field("state", &"[redacted]")
            .finish_non_exhaustive()
    }
}

/// Stored passkey record, including server-only serialized credential state.
#[derive(Clone, PartialEq, Eq)]
pub struct PasskeyRecord {
    pub id: PasskeyId,
    pub user: UserId,
    pub credential_id_hash: CredentialIdHash,
    pub label: String,
    pub credential: WebauthnState,
    pub created_at: ActivityTimestamp,
    pub last_used_at: Option<ActivityTimestamp>,
    pub revoked_at: Option<ActivityTimestamp>,
}

/// Trusted registration result whose timestamps are assigned by the store.
#[derive(Clone, PartialEq, Eq)]
pub struct RegisterPasskey {
    pub id: PasskeyId,
    pub user: UserId,
    pub credential_id_hash: CredentialIdHash,
    pub label: String,
    pub credential: WebauthnState,
}

impl std::fmt::Debug for RegisterPasskey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RegisterPasskey")
            .field("id", &self.id)
            .field("user", &self.user)
            .field("label", &self.label)
            .field("credential", &"[redacted]")
            .finish_non_exhaustive()
    }
}

/// Atomic credential update plus short-lived account proof after WebAuthn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletePasskeyAuthenticationAndCreateSession {
    pub passkey: PasskeyRecord,
    pub session_token_hash: AccountSessionTokenHash,
    pub session_lifetime: AccountSessionLifetime,
}

/// Persisted credential and account proof committed by one transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedPasskeySession {
    pub passkey: PasskeyRecord,
    pub session: AccountSessionRecord,
}

impl std::fmt::Debug for PasskeyRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PasskeyRecord")
            .field("id", &self.id)
            .field("user", &self.user)
            .field("label", &self.label)
            .field("created_at", &self.created_at)
            .field("last_used_at", &self.last_used_at)
            .field("revoked_at", &self.revoked_at)
            .field("credential", &"[redacted]")
            .finish_non_exhaustive()
    }
}

/// Validates a passkey's user-visible device label.
pub fn validated_passkey_label(value: &str) -> Result<String, AccountIdentityError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > MAX_PASSKEY_LABEL_CHARS {
        return Err(AccountIdentityError::InvalidPasskeyLabel);
    }
    Ok(value.to_string())
}

/// Validation failure that is safe to map to a generic authentication error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountIdentityError {
    InvalidEmail,
    InvalidEmailDomain,
    InvalidDisplayName,
    InvalidPasskeyLabel,
    InvalidWebauthnState,
}

impl std::fmt::Display for AccountIdentityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidEmail => "email address is invalid",
            Self::InvalidEmailDomain => "email domain is invalid",
            Self::InvalidDisplayName => "display name is invalid",
            Self::InvalidPasskeyLabel => "passkey label is invalid",
            Self::InvalidWebauthnState => "WebAuthn state is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for AccountIdentityError {}

/// Dedicated persistence boundary for PLE accounts and credentials.
#[async_trait]
pub trait AccountIdentityStore: Send + Sync {
    /// Atomically consumes one passwordless-authentication allowance.
    async fn consume_authentication_rate_limit(
        &self,
        command: ConsumeAuthenticationRateLimit,
    ) -> Result<AuthenticationRateLimitDecision, StoreError>;

    async fn begin_email_authentication(
        &self,
        command: BeginEmailAuthentication,
    ) -> Result<EmailAuthenticationChallenge, StoreError>;

    async fn complete_email_authentication(
        &self,
        command: CompleteEmailAuthentication,
    ) -> Result<CompletedEmailAuthentication, StoreError>;

    async fn complete_email_authentication_and_create_session(
        &self,
        command: CompleteEmailAuthenticationAndCreateSession,
    ) -> Result<CompletedAccountSession, StoreError>;

    async fn get_account(&self, user: UserId) -> Result<Option<AccountRecord>, StoreError>;

    /// Lists bounded course contexts currently reachable by one PLE account.
    ///
    /// Implementations must derive every tenant and role from a persisted
    /// course relationship. Student contexts whose learner records are no
    /// longer accessible are omitted.
    async fn list_account_course_contexts(
        &self,
        user: UserId,
        page: PageRequest,
    ) -> Result<Page<AccountCourseContext>, StoreError>;

    /// Resolves one course context without accepting a tenant or role from
    /// the browser. More than one matching tenant is invalid stored state.
    async fn resolve_account_course_context(
        &self,
        user: UserId,
        course: CourseId,
    ) -> Result<Option<AccountCourseContext>, StoreError>;

    async fn begin_webauthn_ceremony(
        &self,
        command: BeginWebauthnCeremony,
    ) -> Result<WebauthnCeremony, StoreError>;

    async fn take_webauthn_ceremony(
        &self,
        id: WebauthnCeremonyId,
        browser_binding: BrowserBindingHash,
    ) -> Result<Option<WebauthnCeremony>, StoreError>;

    async fn insert_passkey(&self, command: RegisterPasskey) -> Result<PasskeyRecord, StoreError>;

    async fn list_active_passkeys(&self, user: UserId) -> Result<Vec<PasskeyRecord>, StoreError>;

    /// Resolves the active credential selected by a discoverable WebAuthn assertion.
    ///
    /// The caller derives `credential_id_hash` from the credential identifier
    /// returned by the authenticator. An unknown or revoked credential returns
    /// `None`; the browser does not supply a `UserId` for this lookup.
    async fn get_active_passkey_by_credential_id_hash(
        &self,
        credential_id_hash: CredentialIdHash,
    ) -> Result<Option<PasskeyRecord>, StoreError>;

    async fn replace_passkey_after_authentication(
        &self,
        passkey: PasskeyRecord,
    ) -> Result<(), StoreError>;

    async fn complete_passkey_authentication_and_create_session(
        &self,
        command: CompletePasskeyAuthenticationAndCreateSession,
    ) -> Result<CompletedPasskeySession, StoreError>;

    async fn revoke_passkey(&self, user: UserId, passkey: PasskeyId) -> Result<(), StoreError>;
}

/// Tenant-independent account proof. Educational routes must still mint and
/// resolve the existing tenant-scoped session after an authorized context is
/// selected or an invitation is claimed.
#[async_trait]
pub trait AccountSessionStore: Send + Sync {
    async fn create_account_session(
        &self,
        token_hash: AccountSessionTokenHash,
        user: UserId,
        lifetime: AccountSessionLifetime,
    ) -> Result<AccountSessionRecord, StoreError>;

    async fn resolve_account_session(
        &self,
        token_hash: AccountSessionTokenHash,
    ) -> Result<Option<AccountSessionRecord>, StoreError>;

    async fn revoke_account_session(
        &self,
        token_hash: AccountSessionTokenHash,
    ) -> Result<(), StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_lookup_normalizes_case_and_idna_domain() {
        let email =
            AuthenticationEmail::parse("Student@B\u{fc}cher.example").expect("valid IDNA email");
        assert_eq!(email.normalized(), "student@xn--bcher-kva.example");
        assert_eq!(email.delivery(), "Student@B\u{fc}cher.example");
        assert_eq!(email.domain().as_str(), "xn--bcher-kva.example");
    }

    #[test]
    fn domain_comparison_does_not_accept_suffix_confusion() {
        let allowed = EmailDomain::parse("mail.roosevelt.edu").expect("allowed domain");
        let attacker = AuthenticationEmail::parse("student@mail.roosevelt.edu.attacker.example")
            .expect("syntactically valid attacker address");
        assert_ne!(attacker.domain(), &allowed);
    }

    #[test]
    fn malformed_mailboxes_are_rejected() {
        for value in [
            "",
            "student",
            "@example.edu",
            "student@localhost",
            ".student@example.edu",
            "student..name@example.edu",
        ] {
            assert_eq!(
                AuthenticationEmail::parse(value),
                Err(AccountIdentityError::InvalidEmail)
            );
        }
    }

    #[test]
    fn email_and_binding_hashes_are_domain_types() {
        let challenge = EmailChallengeSecretHash::compute(b"same bytes");
        let binding = BrowserBindingHash::compute(b"same bytes");
        assert_eq!(challenge.as_bytes(), binding.as_bytes());
        assert_eq!(
            format!("{challenge:?}"),
            "EmailChallengeSecretHash([redacted])"
        );
    }
}
