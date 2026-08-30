//! Replica-safe, provider-neutral session persistence (MOD-API-AUTH).

use std::num::NonZeroU32;

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{ActivityTimestamp, UserId, UserRole};
use uuid::Uuid;

use crate::StoreError;

/// Maximum displayed identity length accepted from an identity provider.
pub const MAX_DISPLAY_NAME_CHARS: usize = 200;

/// Opaque durable identity for one server-tracked login session.
///
/// This identity is distinct from [`SessionTokenHash`]: storage adapters
/// reconstitute it from trusted session records, and it is never a browser
/// credential or locator.
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

/// Server-derived identity of an authenticated request actor.
///
/// This context carries no authorization grant. Protected operations authorize
/// the actor against their exact durable scope. The coordinated session-record
/// cutover will construct this context from a resolved [`SessionRecord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorContext {
    user_id: UserId,
    session_id: SessionId,
    role: UserRole,
}

impl ActorContext {
    /// Derives request identity only from an active session record resolved by
    /// trusted session storage. ASVS 7.2.1 and 8.3.1: trusted backend storage
    /// establishes the identity used by every later authorization decision.
    pub fn from_session_record(record: &SessionRecord) -> Self {
        Self {
            user_id: record.subject.user(),
            session_id: record.id,
            role: record.subject.role(),
        }
    }

    /// Returns the authenticated global account identity.
    pub fn user_id(self) -> UserId {
        self.user_id
    }

    /// Returns the resolved durable session-record identity.
    pub fn session_id(self) -> SessionId {
        self.session_id
    }

    /// Returns the one immutable role resolved with this active session.
    pub fn role(self) -> UserRole {
        self.role
    }
}

/// SHA-256 of an opaque session credential.
///
/// Only this one-way value crosses the storage boundary. The raw cookie value
/// remains confined to the server authentication module.
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

/// Identity established by a trusted credential provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSubject {
    user: UserId,
    display_name: String,
    role: UserRole,
}

impl SessionSubject {
    /// Validates one provider-established account identity.
    pub fn new(
        user: UserId,
        display_name: impl Into<String>,
        role: UserRole,
    ) -> Result<Self, SessionSubjectError> {
        let display_name = display_name.into();
        let display_name = display_name.trim();
        if display_name.is_empty() {
            return Err(SessionSubjectError::EmptyDisplayName);
        }
        if display_name.chars().count() > MAX_DISPLAY_NAME_CHARS {
            return Err(SessionSubjectError::DisplayNameTooLong);
        }
        Ok(Self {
            user,
            display_name: display_name.to_string(),
            role,
        })
    }

    /// Authenticated person, distinct from an enrollment identifier.
    pub fn user(&self) -> UserId {
        self.user
    }

    /// Provider-established display label.
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Immutable product role for this account.
    pub fn role(&self) -> UserRole {
        self.role
    }
}

/// Invalid provider-established session identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionSubjectError {
    /// The provider supplied no visible identity label.
    EmptyDisplayName,
    /// The label exceeded the bounded session-field size.
    DisplayNameTooLong,
}

impl std::fmt::Display for SessionSubjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyDisplayName => formatter.write_str("display name must not be empty"),
            Self::DisplayNameTooLong => write!(
                formatter,
                "display name must contain at most {MAX_DISPLAY_NAME_CHARS} characters"
            ),
        }
    }
}

impl std::error::Error for SessionSubjectError {}

/// Active database session returned after expiry and revocation checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    /// Durable session identity, distinct from the opaque credential hash.
    pub id: SessionId,
    /// One-way lookup key, never the raw cookie credential.
    pub token_hash: SessionTokenHash,
    /// Identity established at credential verification time.
    pub subject: SessionSubject,
    /// Database-authoritative creation time.
    pub created_at: ActivityTimestamp,
    /// Database-authoritative exclusive expiration time.
    pub expires_at: ActivityTimestamp,
}

/// Persistence contract kept separate from educational-record storage.
///
/// Login can issue on one server replica and resolve on any other replica
/// because implementations persist the row in shared backend state.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Inserts a new session using the backend's authoritative clock.
    async fn create_session(
        &self,
        token_hash: SessionTokenHash,
        subject: SessionSubject,
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
    use uuid::Uuid;

    #[test]
    fn session_hash_round_trips_without_exposing_it_in_debug_output() {
        let hash = SessionTokenHash::compute(b"opaque credential");
        assert_eq!(SessionTokenHash::from_hex(&hash.to_string()), Ok(hash));
        assert_eq!(format!("{hash:?}"), "SessionTokenHash([redacted])");
        assert!(SessionTokenHash::from_hex(&hash.to_string().to_uppercase()).is_err());
    }

    #[test]
    fn session_subject_is_bounded_and_canonical() {
        let user = UserId::from_uuid(Uuid::from_u128(2));
        let subject = SessionSubject::new(user, " Fixture User ", UserRole::Student)
            .expect("valid session subject");

        assert_eq!(subject.display_name(), "Fixture User");
        assert_eq!(subject.role(), UserRole::Student);
        assert_eq!(
            SessionSubject::new(user, " ", UserRole::Student),
            Err(SessionSubjectError::EmptyDisplayName)
        );
    }

    #[test]
    fn actor_context_preserves_the_resolved_session_role() {
        let subject = SessionSubject::new(
            UserId::from_uuid(Uuid::from_u128(2)),
            "Fixture User",
            UserRole::Instructor,
        )
        .expect("valid session subject");
        let record = SessionRecord {
            id: SessionId::from_uuid(Uuid::from_u128(3)),
            token_hash: SessionTokenHash::compute(b"opaque credential"),
            subject,
            created_at: ActivityTimestamp::from_unix_millis(1),
            expires_at: ActivityTimestamp::from_unix_millis(2),
        };

        let actor = ActorContext::from_session_record(&record);

        assert_eq!(actor.user_id(), record.subject.user());
        assert_eq!(actor.session_id(), record.id);
        assert_eq!(actor.role(), UserRole::Instructor);
    }
}
