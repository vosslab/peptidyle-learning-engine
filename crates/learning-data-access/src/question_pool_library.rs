//! Session-authorized reads for reusable published Question Pools.
//!
//! PostgreSQL returns only public Pool identity and exact immutable Revision
//! pins. Question source bindings remain owned by `QuestionLibraryStore` and
//! browser-safe rendering remains a server responsibility.

use async_trait::async_trait;
use question_model::{
    AssessmentEntryId, QuestionPoolLibrarySummary, QuestionPoolRevisionReference,
    QuestionRevisionReference,
};
use uuid::Uuid;

use crate::{Page, PageRequest, SessionTokenHash, StoreError};

/// Exact immutable Pool content before answer-free Question projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQuestionPoolRevision {
    pub question_pool_revision: QuestionPoolRevisionReference,
    pub members: Vec<QuestionRevisionReference>,
}

/// Assessment-owned exact fork facts before answer-free Question projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessmentQuestionPoolForkRecord {
    pub assessment_entry_id: AssessmentEntryId,
    pub question_pool_revision: QuestionPoolRevisionReference,
    pub pool_metadata_etag: Uuid,
    pub selection_count: std::num::NonZeroU32,
    pub members: Vec<QuestionRevisionReference>,
}

/// Store boundary for global Pool discovery and owned Assessment-fork reads.
#[async_trait]
pub trait QuestionPoolLibraryStore: Send + Sync {
    /// Lists every published Pool lineage, including child forks, in stable
    /// public-ID order under an active vetted Instructor session.
    async fn list_published_question_pools(
        &self,
        session_token_hash: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<QuestionPoolLibrarySummary>, StoreError>;

    /// Resolves the current Revision of one published Pool lineage.
    async fn load_current_published_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        public_question_pool_id: &question_model::QuestionId,
    ) -> Result<PublishedQuestionPoolRevision, StoreError>;

    /// Derives one exact Assessment-owned fork through Course, Assessment,
    /// and Entry authorization; callers cannot select a Pool Revision.
    async fn load_assessment_question_pool_fork(
        &self,
        session_token_hash: SessionTokenHash,
        course: question_model::CourseInstanceReference,
        assessment: question_model::AssessmentReference,
        assessment_entry: AssessmentEntryId,
    ) -> Result<AssessmentQuestionPoolForkRecord, StoreError>;
}
