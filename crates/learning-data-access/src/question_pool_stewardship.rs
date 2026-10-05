//! Stable published Pool Stars and actor-private Watches.
use crate::{SessionTokenHash, StoreError};
use async_trait::async_trait;
use question_model::{AccountId, QuestionPoolId};

/// Public endorsement facts, with no Watch fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionPoolStarProjection {
    pub viewer_has_starred: bool,
    pub star_count: u64,
    pub starred_instructors: Vec<QuestionPoolStarredInstructor>,
}

/// An active Instructor's public Profile identity in a Star list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionPoolStarredInstructor {
    pub display_name: String,
    pub account_id: AccountId,
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
        question_pool_id: &QuestionPoolId,
    ) -> Result<QuestionPoolStarProjection, StoreError>;
    async fn set_current_question_pool_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionPoolId,
        starred: bool,
    ) -> Result<QuestionPoolStarProjection, StoreError>;
    async fn question_pool_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionPoolId,
    ) -> Result<QuestionPoolWatchProjection, StoreError>;
    async fn set_current_question_pool_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionPoolId,
        watching: bool,
    ) -> Result<QuestionPoolWatchProjection, StoreError>;
}
