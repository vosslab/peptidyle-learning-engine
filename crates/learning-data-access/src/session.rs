//! Canonical persisted session contract for one global account.

use std::num::NonZeroU32;

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{AccountId, AccountRole, ActivityTimestamp};
use uuid::Uuid;

use crate::StoreError;

/// Opaque durable identity for one server-tracked login session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Mints a fresh durable session identity before persistence.
    pub fn generate() -> Result<Self, StoreError> {
        crate::random_uuid::random_uuid_v4(|error| {
            StoreError::Unavailable(format!("session ID randomness unavailable: {error}"))
        })
        .map(Self)
    }

    /// Reconstitutes an ID read from trusted session storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the UUID used by server-side session storage.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// SHA-256 of an opaque browser session credential.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionTokenHash([u8; 32]);

impl SessionTokenHash {
    /// Hashes raw session-token bytes for persistence and lookup.
    pub fn compute(token: &[u8]) -> Self {
        Self(*Sha256Digest::compute(token).as_bytes())
    }

    /// Parses the lowercase hexadecimal database representation.
    pub fn from_hex(value: &str) -> Result<Self, SessionTokenHashParseError> {
        if value.len() != 64 {
            return Err(SessionTokenHashParseError);
        }
        let mut bytes = [0_u8; 32];
        for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let high = hex_nibble(pair[0]).ok_or(SessionTokenHashParseError)?;
            let low = hex_nibble(pair[1]).ok_or(SessionTokenHashParseError)?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

impl std::fmt::Debug for SessionTokenHash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SessionTokenHash([redacted])")
    }
}

impl std::fmt::Display for SessionTokenHash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// A malformed persisted session hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionTokenHashParseError;

impl std::fmt::Display for SessionTokenHashParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("session token hash must be 64 lowercase hexadecimal characters")
    }
}

impl std::error::Error for SessionTokenHashParseError {}

/// Positive database-authoritative session lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLifetime(NonZeroU32);

impl SessionLifetime {
    /// Validates a lifetime expressed in whole seconds.
    pub fn from_seconds(seconds: u32) -> Option<Self> {
        NonZeroU32::new(seconds).map(Self)
    }

    /// Returns the validated number of seconds.
    pub fn as_seconds(self) -> u32 {
        self.0.get()
    }
}

/// Active database session returned after expiry and revocation checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    /// Durable session identity, distinct from the opaque credential hash.
    pub id: SessionId,
    /// One-way lookup key, never the raw browser credential.
    pub token_hash: SessionTokenHash,
    /// Global login account authenticated by this session.
    pub account: AccountId,
    /// Immutable product role of the authenticated account.
    pub role: AccountRole,
    /// Database-authoritative creation time.
    pub created_at: ActivityTimestamp,
    /// Database-authoritative exclusive expiration time.
    pub expires_at: ActivityTimestamp,
}

impl SessionRecord {
    /// Returns the global account authenticated by this resolved session.
    pub fn account_id(&self) -> AccountId {
        self.account
    }
}

/// Persistence contract separate from educational-record storage.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Inserts a new session using the backend's authoritative clock.
    async fn create_session(
        &self,
        token_hash: SessionTokenHash,
        account: AccountId,
        role: AccountRole,
        lifetime: SessionLifetime,
    ) -> Result<SessionRecord, StoreError>;

    /// Resolves an active session after backend-side expiry and revocation checks.
    async fn resolve_session(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Option<SessionRecord>, StoreError>;

    /// Revokes a session immediately and idempotently.
    async fn revoke_session(&self, token_hash: SessionTokenHash) -> Result<(), StoreError>;
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_hash_round_trips() {
        let value = SessionTokenHash::compute(b"one opaque token");
        let parsed = SessionTokenHash::from_hex(&value.to_string()).expect("canonical hash");

        assert_eq!(parsed, value);
    }
}
