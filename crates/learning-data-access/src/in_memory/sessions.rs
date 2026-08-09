//! In-memory authentication-session persistence.

use async_trait::async_trait;
use question_model::ActivityTimestamp;

use super::MemoryStore;
use crate::{
    SessionLifetime, SessionRecord, SessionStore, SessionSubject, SessionTokenHash, StoreError,
};

#[derive(Debug, Clone)]
pub(super) struct StoredSession {
    pub(super) record: SessionRecord,
    pub(super) revoked: bool,
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn create_session(
        &self,
        token_hash: SessionTokenHash,
        subject: SessionSubject,
        lifetime: SessionLifetime,
    ) -> Result<SessionRecord, StoreError> {
        let mut state = self.write_state()?;
        if state.sessions.contains_key(&token_hash) {
            return Err(StoreError::AlreadyExists);
        }
        let created_at = state.authoritative_time;
        let lifetime_millis = i64::from(lifetime.as_seconds()) * 1_000;
        let expires_at = ActivityTimestamp::from_unix_millis(
            created_at
                .as_unix_millis()
                .checked_add(lifetime_millis)
                .ok_or_else(|| StoreError::InvalidRecord("session expiry overflow".to_string()))?,
        );
        let record = SessionRecord {
            token_hash,
            subject,
            created_at,
            expires_at,
        };
        state.sessions.insert(
            token_hash,
            StoredSession {
                record: record.clone(),
                revoked: false,
            },
        );
        Ok(record)
    }

    async fn resolve_session(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Option<SessionRecord>, StoreError> {
        let state = self.read_state()?;
        let now = state.authoritative_time;
        Ok(state.sessions.get(&token_hash).and_then(|stored| {
            (!stored.revoked && stored.record.expires_at > now).then(|| stored.record.clone())
        }))
    }

    async fn revoke_session(&self, token_hash: SessionTokenHash) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        if let Some(stored) = state.sessions.get_mut(&token_hash) {
            stored.revoked = true;
        }
        Ok(())
    }
}
