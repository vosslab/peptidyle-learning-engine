//! Session-authorized reads for reusable published Question Pools.
//!
//! PostgreSQL returns only public Pool identity and current membership.
//! Question source bindings remain owned by `QuestionLibraryStore` and
//! browser-safe rendering remains a server responsibility.

use async_trait::async_trait;
use question_model::{
    AccountId, AssessmentEntryId, BloomClassificationEditNumber, BloomClassificationView,
    BloomCognitiveProcess, BloomKnowledgeDimension, PublishedQuestionRevisionTuple,
    QuestionBackend, QuestionLicense, QuestionPoolEditNumber, QuestionPoolId, QuestionPoolMetadata,
    QuestionType,
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

/// Assessment-owned exact fork facts before answer-free Question projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessmentQuestionPoolForkRecord {
    pub metadata: QuestionPoolMetadata,
    pub assessment_entry_id: AssessmentEntryId,
    pub question_pool_id: QuestionPoolId,
    pub owner_account_id: AccountId,
    pub question_type: QuestionType,
    pub backend: QuestionBackend,
    pub license: QuestionLicense,
    pub question_pool_edit_number: QuestionPoolEditNumber,
    pub selection_count: std::num::NonZeroU32,
    pub bloom: Option<BloomClassificationView>,
    pub members: Vec<PublishedQuestionRevisionTuple>,
}

/// Store boundary for current Pool and owned Assessment-fork reads.
#[async_trait]
pub trait QuestionPoolLibraryStore: Send + Sync {
    /// Resolves the current membership of one published Pool.
    async fn load_current_published_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &question_model::QuestionPoolId,
    ) -> Result<PublishedQuestionPool, StoreError>;

    /// Corrects both Bloom dimensions for one current Pool through the
    /// classification-owned compare-and-swap number.
    async fn correct_question_pool_bloom(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &question_model::QuestionPoolId,
        expected_edit_number: BloomClassificationEditNumber,
        cognitive_process: BloomCognitiveProcess,
        knowledge_dimension: BloomKnowledgeDimension,
    ) -> Result<BloomClassificationView, StoreError>;

    /// Derives one exact Assessment-owned fork through Course, Assessment,
    /// and Entry authorization; callers cannot select historical Pool membership.
    async fn load_assessment_question_pool_fork(
        &self,
        session_token_hash: SessionTokenHash,
        course: question_model::CourseInstanceId,
        assessment: question_model::AssessmentId,
        assessment_entry: AssessmentEntryId,
    ) -> Result<AssessmentQuestionPoolForkRecord, StoreError>;
}
