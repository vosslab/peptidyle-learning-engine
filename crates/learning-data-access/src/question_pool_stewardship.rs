//! Stable published Pool Stars and actor-private Watches.
use crate::{SessionTokenHash, StoreError};
use async_trait::async_trait;
use question_model::QuestionId;

/// Approved public endorsement facts, with no Account/Profile or Watch fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionPoolStarProjection {
    pub viewer_has_starred: bool,
    pub star_count: u64,
    pub starred_instructors: Vec<QuestionPoolStarredInstructor>,
}

/// The immutable vetted public name, not an Account identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionPoolStarredInstructor {
    pub display_name: String,
}

/// Subscription state for the authenticated actor only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionPoolWatchProjection {
    pub watching: bool,
}

/// Database-authorized lineage relationships; retries set explicit state.
#[async_trait]
pub trait QuestionPoolStewardshipStore: Send + Sync {
    async fn question_pool_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionId,
    ) -> Result<QuestionPoolStarProjection, StoreError>;
    async fn set_current_question_pool_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionId,
        starred: bool,
    ) -> Result<QuestionPoolStarProjection, StoreError>;
    async fn question_pool_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionId,
    ) -> Result<QuestionPoolWatchProjection, StoreError>;
    async fn set_current_question_pool_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionId,
        watching: bool,
    ) -> Result<QuestionPoolWatchProjection, StoreError>;
}
