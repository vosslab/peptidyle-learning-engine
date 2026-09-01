//! Private passwordless credential-ceremony contracts.
//!
//! These records prove possession of an existing Account's credential. They
//! never create an Account, assign a Product Role, or replace the canonical
//! Authenticated Session record.

use std::num::NonZeroU32;

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::{AccountId, AccountRole, Timestamp};
use uuid::Uuid;

use crate::StoreError;

/// Maximum server-authoritative lifetime for an email or WebAuthn ceremony.
pub const MAX_AUTHENTICATION_CEREMONY_SECONDS: u32 = 10 * 60;

/// Validated short lifetime for a one-use authentication ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticationCeremonyLifetime(NonZeroU32);

impl AuthenticationCeremonyLifetime {
    /// Accepts only a positive lifetime bounded by the database contract.
    pub fn from_seconds(seconds: u32) -> Option<Self> {
        NonZeroU32::new(seconds)
            .filter(|seconds| seconds.get() <= MAX_AUTHENTICATION_CEREMONY_SECONDS)
            .map(Self)
    }

    /// Returns the bounded lifetime in whole seconds.
    pub fn as_seconds(self) -> u32 {
        self.0.get()
    }
}

/// One-way hash of a browser-bound secret or credential proof.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuthenticationSecretHash(Sha256Checksum);

impl AuthenticationSecretHash {
    /// Computes the persisted hash without retaining the raw secret.
    pub fn compute(value: &[u8]) -> Self {
        Self(Sha256Checksum::compute(value))
    }

    /// Returns the verified fixed-width storage form.
    pub fn as_bytes(self) -> [u8; 32] {
        *self.0.as_bytes()
    }
}

impl std::fmt::Debug for AuthenticationSecretHash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AuthenticationSecretHash([redacted])")
    }
}

/// Durable identity for one email-code ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmailAuthenticationChallengeId(Uuid);

impl EmailAuthenticationChallengeId {
    /// Reconstitutes an ID read from trusted private storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the private-storage identifier.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Purpose limited to sign-in or a verified email replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailAuthenticationPurpose {
    /// Establish a new session for an existing Account.
    SignIn,
    /// Replace an existing Account's verified Authentication Email.
    ChangeEmail,
}

/// Private one-use email-code ceremony state.
#[derive(Clone, PartialEq, Eq)]
pub struct EmailAuthenticationChallenge {
    /// Database identity for this ceremony.
    pub id: EmailAuthenticationChallengeId,
    /// Existing Account authenticated when the challenge succeeds.
    pub account: AccountId,
    /// Closed credential-flow purpose.
    pub purpose: EmailAuthenticationPurpose,
    /// One-way hash of the emailed proof; raw codes never enter this type.
    pub proof_hash: AuthenticationSecretHash,
    /// One-way binding to the initiating browser.
    pub browser_binding_hash: AuthenticationSecretHash,
    /// Database-authoritative creation time.
    pub created_at: Timestamp,
    /// Database-authoritative exclusive expiration time.
    pub expires_at: Timestamp,
}

impl std::fmt::Debug for EmailAuthenticationChallenge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EmailAuthenticationChallenge")
            .field("id", &self.id)
            .field("account", &self.account)
            .field("purpose", &self.purpose)
            .field("proof_hash", &"[redacted]")
            .field("browser_binding_hash", &"[redacted]")
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Trusted Account facts returned only after a credential ceremony succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticatedAccount {
    /// Existing global Account verified by the ceremony.
    pub account: AccountId,
    /// Immutable Product Role stored with that Account.
    pub role: AccountRole,
}

/// Durable identity for one registered passkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PasskeyId(Uuid);

impl PasskeyId {
    /// Reconstitutes an ID read from trusted private storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the private-storage identifier.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Durable identity for one browser-bound WebAuthn ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WebauthnCeremonyId(Uuid);

impl WebauthnCeremonyId {
    /// Reconstitutes an ID read from trusted private storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the private-storage identifier.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Private registered WebAuthn credential for one existing Account.
#[derive(Clone, PartialEq, Eq)]
pub struct Passkey {
    /// Private passkey identifier.
    pub id: PasskeyId,
    /// Existing Account that owns this credential.
    pub account: AccountId,
    /// One-way lookup hash for the authenticator credential ID.
    pub credential_id_hash: AuthenticationSecretHash,
    /// Database-authoritative registration time.
    pub created_at: Timestamp,
    /// Revocation time, if the credential is no longer usable.
    pub revoked_at: Option<Timestamp>,
}

impl std::fmt::Debug for Passkey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Passkey")
            .field("id", &self.id)
            .field("account", &self.account)
            .field("credential_id_hash", &"[redacted]")
            .field("created_at", &self.created_at)
            .field("revoked_at", &self.revoked_at)
            .finish()
    }
}

/// Persistence boundary for passwordless credential ceremonies.
///
/// ASVS 3.3.1 and 3.3.3: raw authentication secrets are never retained;
/// completion is atomic, browser-bound, single-use, and returns only trusted
/// existing-Account facts to the route that will create a `SessionStore` record.
#[async_trait]
pub trait AuthenticationCeremonyStore: Send + Sync {
    /// Atomically consumes an eligible email proof and returns its Account.
    async fn consume_email_authentication_challenge(
        &self,
        challenge: EmailAuthenticationChallengeId,
        proof_hash: AuthenticationSecretHash,
        browser_binding_hash: AuthenticationSecretHash,
    ) -> Result<Option<AuthenticatedAccount>, StoreError>;

    /// Resolves an eligible passkey only after its WebAuthn adapter validates
    /// the browser ceremony and credential proof.
    async fn authenticate_passkey(
        &self,
        ceremony: WebauthnCeremonyId,
        credential_id_hash: AuthenticationSecretHash,
        browser_binding_hash: AuthenticationSecretHash,
    ) -> Result<Option<AuthenticatedAccount>, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceremony_lifetime_is_positive_and_bounded_to_ten_minutes() {
        assert!(AuthenticationCeremonyLifetime::from_seconds(0).is_none());
        assert_eq!(
            AuthenticationCeremonyLifetime::from_seconds(MAX_AUTHENTICATION_CEREMONY_SECONDS)
                .expect("maximum lifetime")
                .as_seconds(),
            MAX_AUTHENTICATION_CEREMONY_SECONDS,
        );
        assert!(
            AuthenticationCeremonyLifetime::from_seconds(MAX_AUTHENTICATION_CEREMONY_SECONDS + 1)
                .is_none()
        );
    }

    #[test]
    fn secret_hash_is_redacted_but_stable_for_storage() {
        let value = AuthenticationSecretHash::compute(b"one-time credential proof");
        assert_eq!(value.as_bytes().len(), 32);
        assert_eq!(format!("{value:?}"), "AuthenticationSecretHash([redacted])");
    }
}
