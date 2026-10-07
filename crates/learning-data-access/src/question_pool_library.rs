//! Session-authorized reads for reusable published Question Pools.
//!
//! PostgreSQL returns only public Pool identity and current membership.
//! Question source bindings remain owned by `QuestionLibraryStore` and
//! browser-safe rendering remains a server responsibility.

use async_trait::async_trait;
use question_model::{
    AccountId, BloomClassificationView, CurrentQuestionPoolMetadata,
    PublishedQuestionRevisionTuple, QuestionBackend, QuestionLicense, QuestionPoolEditNumber,
    QuestionPoolId, QuestionPoolMetadata, QuestionType, SaveQuestionPoolMembersRequest,
    SaveQuestionPoolMetadataRequest, SavedQuestionPoolMembers, SavedQuestionPoolMetadata,
};

use crate::{SessionTokenHash, StoreError};

/// Exact immutable Pool content before answer-free Question projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQuestionPool {
    pub metadata: QuestionPoolMetadata,
    pub question_pool_id: QuestionPoolId,
    pub owner_account_id: AccountId,
    pub question_type: QuestionType,
    pub backend: QuestionBackend,
    pub license: QuestionLicense,
    pub question_pool_edit_number: QuestionPoolEditNumber,
    pub bloom: Option<BloomClassificationView>,
    pub members: Vec<PublishedQuestionRevisionTuple>,
}

/// Store boundary for current reusable Pool reads and metadata edits.
#[async_trait]
pub trait QuestionPoolLibraryStore: Send + Sync {
    /// Replaces the current exact member tuple set with its ordinary Pool Edit Number CAS.
    async fn save_question_pool_members(
        &self,
        session_token_hash: SessionTokenHash,
        request: SaveQuestionPoolMembersRequest,
    ) -> Result<SavedQuestionPoolMembers, StoreError>;

    /// Replaces current Pool metadata with its ordinary metadata CAS. The Pool
    /// membership Edit Number is independent and is never advanced here.
    async fn save_question_pool_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        request: SaveQuestionPoolMetadataRequest,
    ) -> Result<SavedQuestionPoolMetadata, StoreError>;

    /// Reads the exact current Pool metadata and its ordinary concurrency token.
    async fn load_current_question_pool_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &QuestionPoolId,
    ) -> Result<CurrentQuestionPoolMetadata, StoreError>;

    /// Resolves the current membership of one published Pool.
    async fn load_current_published_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &question_model::QuestionPoolId,
    ) -> Result<PublishedQuestionPool, StoreError>;
}
