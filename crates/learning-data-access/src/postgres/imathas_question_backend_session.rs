//! PostgreSQL persistence for server-only iMathAS Question Backend Sessions.

use async_trait::async_trait;

use super::Pool;
use crate::{
    ImathasQuestionBackendSessionCreate, ImathasQuestionBackendSessionId,
    ImathasQuestionBackendSessionRestoreExpectation, ImathasQuestionBackendSessionStore,
    ImathasQuestionBackendStateKeyRing, LoadedImathasQuestionBackendSession, SessionTokenHash,
    StoreError,
};

/// PostgreSQL implementation of the durable iMathAS Question Backend Session boundary.
#[derive(Clone)]
pub struct PostgresImathasQuestionBackendSessionStore {
    _pool: Pool,
    _key_ring: std::sync::Arc<ImathasQuestionBackendStateKeyRing>,
}

impl PostgresImathasQuestionBackendSessionStore {
    /// Binds an already-attested application pool and server-owned key ring.
    pub fn new(pool: Pool, key_ring: std::sync::Arc<ImathasQuestionBackendStateKeyRing>) -> Self {
        Self {
            _pool: pool,
            _key_ring: key_ring,
        }
    }
}

fn unavailable() -> StoreError {
    StoreError::Unavailable("iMathAS Question Backend Session store is unavailable".into())
}

#[async_trait]
impl ImathasQuestionBackendSessionStore for PostgresImathasQuestionBackendSessionStore {
    async fn create_imathas_question_backend_session(
        &self,
        _session_token_hash: SessionTokenHash,
        _create: ImathasQuestionBackendSessionCreate,
    ) -> Result<ImathasQuestionBackendSessionId, StoreError> {
        Err(unavailable())
    }

    async fn load_imathas_question_backend_session(
        &self,
        _session_token_hash: SessionTokenHash,
        _session_id: ImathasQuestionBackendSessionId,
        _expectation: ImathasQuestionBackendSessionRestoreExpectation,
    ) -> Result<LoadedImathasQuestionBackendSession, StoreError> {
        Err(unavailable())
    }
}
