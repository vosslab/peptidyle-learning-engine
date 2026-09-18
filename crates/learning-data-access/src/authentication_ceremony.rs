//! Private passwordless credential-ceremony contracts.
//!
//! These records prove possession of an existing Account's credential. They
//! never create an Account, assign a Product Role, or replace the canonical
//! Authenticated Session record.

use std::num::NonZeroU32;

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::{AccountId, ProductRole, Timestamp};
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::StoreError;
use crate::{SessionLifetime, SessionRecord, SessionTokenHash};

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedAccount {
    /// Existing global Account verified by the ceremony.
    pub account: AccountId,
    /// Immutable Product Role stored with that Account.
    pub product_role: ProductRole,
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

/// Durable identity for one browser-bound Passkey Ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PasskeyCeremonyId(Uuid);

impl PasskeyCeremonyId {
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
        ceremony: PasskeyCeremonyId,
        credential_id_hash: AuthenticationSecretHash,
        browser_binding_hash: AuthenticationSecretHash,
    ) -> Result<Option<AuthenticatedAccount>, StoreError>;
}

/// A private, server-held TOTP seed for one Sysadmin Account.
///
/// The local controller supplies CSPRNG bytes; this type refuses the short
/// values that would make a TOTP factor weak.  It deliberately has no
/// serialization or display implementation.
#[derive(PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct SysadminTotpSeed(Vec<u8>);

impl SysadminTotpSeed {
    /// Accepts a bounded seed generated by the local controller's CSPRNG.
    pub fn from_csprng_bytes(bytes: Vec<u8>) -> Result<Self, StoreError> {
        if !(20..=64).contains(&bytes.len()) {
            return Err(StoreError::InvalidRecord(
                "Sysadmin TOTP seed must contain 20 through 64 CSPRNG bytes".into(),
            ));
        }
        Ok(Self(bytes))
    }

    /// Returns the seed only to the server-side verifier after the Store has
    /// authenticated the pending browser-bound ceremony. It has no display or
    /// serialization representation.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for SysadminTotpSeed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SysadminTotpSeed([redacted])")
    }
}

/// Durable identifier for one short-lived pending Sysadmin MFA ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SysadminTotpAttestationId(Uuid);

impl SysadminTotpAttestationId {
    /// Mints the opaque pending-attestation identifier with the OS CSPRNG.
    pub fn generate() -> Result<Self, StoreError> {
        crate::random_uuid::random_uuid_v4(|error| {
            StoreError::Unavailable(format!(
                "Sysadmin TOTP attestation randomness unavailable: {error}"
            ))
        })
        .map(Self)
    }

    /// Reconstitutes a private-storage identifier.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the private-storage identifier.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// TOTP's server-derived 30-second counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SysadminTotpCounter(i64);

impl SysadminTotpCounter {
    /// Rejects negative and therefore invalid TOTP counters at the trusted
    /// service boundary (ASVS 2.2.1).
    pub fn from_server_counter(value: i64) -> Option<Self> {
        (value >= 0).then_some(Self(value))
    }

    /// Returns the trusted server-derived counter for the TOTP verifier or
    /// atomic persistence transition.
    pub fn as_i64(self) -> i64 {
        self.0
    }
}

/// A pending TOTP ceremony after the primary factor succeeded.
#[derive(PartialEq, Eq)]
pub struct PendingSysadminTotpAttestation {
    /// Opaque pending ceremony identity.
    pub id: SysadminTotpAttestationId,
    /// The existing Sysadmin Account derived by PostgreSQL.
    pub account: AccountId,
    /// Private seed, returned only to the server-side TOTP verifier.
    pub seed: SysadminTotpSeed,
}

/// A server-authorized attempt to verify one bound pending TOTP ceremony.
///
/// The reservation is deliberately opaque to the browser. Its presence means
/// PostgreSQL accepted this attempt before TOTP-code verification and charged
/// it to the one Account/browser-bound pending ceremony.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysadminTotpVerificationReservation {
    /// Existing Sysadmin Account resolved only by PostgreSQL.
    pub account: AccountId,
}

impl std::fmt::Debug for PendingSysadminTotpAttestation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PendingSysadminTotpAttestation")
            .field("id", &self.id)
            .field("account", &self.account)
            .field("seed", &"[redacted]")
            .finish()
    }
}

/// Dedicated persistence boundary for the Sysadmin-only TOTP second factor.
///
/// ASVS 2.3.1, 2.3.3, and 6.5.1: the database derives the role and atomically
/// turns exactly one browser-bound pending attestation and one unused counter
/// into a session.  There is no role or demo-mode argument that can bypass it.
#[async_trait]
pub trait SysadminTotpStore: Send + Sync {
    /// Encrypts and stores the controller-provisioned Sysadmin seed.
    async fn provision_sysadmin_totp_seed(
        &self,
        account: AccountId,
        seed: SysadminTotpSeed,
    ) -> Result<(), StoreError>;

    /// Begins a short-lived, browser-bound second-factor ceremony only for an
    /// active Sysadmin Account.  Other roles deliberately receive `None`.
    async fn create_pending_sysadmin_totp_attestation(
        &self,
        account: AccountId,
        browser_binding_hash: AuthenticationSecretHash,
        lifetime: AuthenticationCeremonyLifetime,
    ) -> Result<Option<SysadminTotpAttestationId>, StoreError>;

    /// Loads an eligible pending ceremony and opens its seed only for the
    /// matching browser-bound server-side verifier.
    async fn load_pending_sysadmin_totp_attestation(
        &self,
        attestation: SysadminTotpAttestationId,
        browser_binding_hash: AuthenticationSecretHash,
    ) -> Result<Option<PendingSysadminTotpAttestation>, StoreError>;

    /// Atomically validates and charges one verification attempt before a
    /// server-side TOTP verifier examines the code. Missing, expired,
    /// consumed, browser-mismatched, non-Sysadmin, or rate-limited input all
    /// returns `None` to preserve the authentication concealment boundary.
    async fn reserve_sysadmin_totp_verification_attempt(
        &self,
        attestation: SysadminTotpAttestationId,
        browser_binding_hash: AuthenticationSecretHash,
    ) -> Result<Option<SysadminTotpVerificationReservation>, StoreError>;

    /// Atomically consumes the pending attestation and server-verified TOTP
    /// counter while creating the authenticated session.
    async fn consume_sysadmin_totp_attestation_into_session(
        &self,
        attestation: SysadminTotpAttestationId,
        browser_binding_hash: AuthenticationSecretHash,
        counter: SysadminTotpCounter,
        token_hash: SessionTokenHash,
        lifetime: SessionLifetime,
    ) -> Result<Option<SessionRecord>, StoreError>;
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
