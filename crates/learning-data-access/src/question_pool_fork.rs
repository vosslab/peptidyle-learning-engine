//! Explicitly forks one current reusable Question Pool for an active Instructor.

use async_trait::async_trait;
use question_model::QuestionPoolId;

use crate::{CreateQuestionPoolError, CreatedQuestionPool, SessionTokenHash};

/// Persists one ordinary current-state Pool copied from a source Pool.
#[async_trait]
pub trait QuestionPoolForkStore: Send + Sync {
    /// Creates a new independently owned Pool with the source's current content.
    async fn fork_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: QuestionPoolId,
        source_question_pool_id: QuestionPoolId,
    ) -> Result<CreatedQuestionPool, CreateQuestionPoolError>;
}
