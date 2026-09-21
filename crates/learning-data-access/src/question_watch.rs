//! Authenticated Instructor Watch state for one Published Question lineage.
//!
//! A Watch is a private subscription. This boundary intentionally returns no
//! watcher count, identity, list, activity, or notification record.

use async_trait::async_trait;
use question_model::PublishedQuestionId;

use crate::{SessionTokenHash, StoreError};

/// Browser-safe private Watch state for the authenticated Instructor only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionWatchProjection {
    /// Whether this Instructor currently Watches the named Question lineage.
    pub watching: bool,
}

/// Authenticated self-only boundary for Question Watch reads and mutations.
#[async_trait]
pub trait QuestionWatchStore: Send + Sync {
    /// Reads only the caller's own Watch state for a Published Question.
    ///
    /// The database rejects Students, anonymous callers, inactive Accounts,
    /// and anything other than an existing Published Question.
    async fn question_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
    ) -> Result<QuestionWatchProjection, StoreError>;

    /// Sets only the authenticated Instructor's private Watch state.
    /// Repeating `watching` is intentionally a successful no-op.
    async fn set_current_question_watch(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
        watching: bool,
    ) -> Result<QuestionWatchProjection, StoreError>;
}
