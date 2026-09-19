//! Session-authorized reads for reusable published Question Pools.
//!
//! PostgreSQL returns only public Pool identity and current membership.
//! Question source bindings remain owned by `QuestionLibraryStore` and
//! browser-safe rendering remains a server responsibility.

use async_trait::async_trait;
use question_model::{
    AssessmentEntryId, BloomClassificationEditNumber, BloomClassificationView,
    BloomCognitiveProcess, BloomKnowledgeDimension, QuestionId, QuestionPoolEditNumber,
    QuestionPoolLibrarySummary, QuestionPoolMetadata, QuestionRevisionTuple,
    QuestionSearchBloomCognitiveProcessFacet, QuestionSearchBloomKnowledgeDimensionFacet,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{PageRequest, SessionTokenHash, StoreError};

/// Identity predicates against current Pool-owned lineage metadata.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct QuestionPoolDiscoveryFilter {
    pub discipline_uuid: Option<Uuid>,
    pub subject_uuid: Option<Uuid>,
    pub topic_uuid: Option<Uuid>,
    pub subtopic_uuid: Option<Uuid>,
    pub cross_discipline: bool,
    pub bloom_cognitive_process: Option<BloomCognitiveProcess>,
    pub bloom_knowledge_dimension: Option<BloomKnowledgeDimension>,
}

/// Pool-owned substring predicates, parsed once by the shared Library grammar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuestionPoolTextTerm {
    pub field: QuestionPoolTextField,
    pub value: String,
    pub excluded: bool,
}

/// Closed SQL field vocabulary; no member Question fields are representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionPoolTextField {
    Any,
    Discipline,
    Subject,
    Topic,
    Subtopic,
    Tags,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct QuestionPoolTextFilter {
    pub text: Option<String>,
    pub terms: Vec<QuestionPoolTextTerm>,
    /// Any exact normalized tag may match; combined with all text and identities.
    pub tags: Vec<String>,
}

/// One bounded Pool page plus Bloom counts from its complete filtered relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionPoolDiscoveryPage {
    pub items: Vec<QuestionPoolLibrarySummary>,
    pub next_cursor: Option<crate::Cursor>,
    pub cognitive_processes: Vec<QuestionSearchBloomCognitiveProcessFacet>,
    pub knowledge_dimensions: Vec<QuestionSearchBloomKnowledgeDimensionFacet>,
}

impl QuestionPoolDiscoveryFilter {
    /// Rejects skipped hierarchy levels and unanchored cross-Discipline searches.
    pub fn has_valid_structure(&self) -> bool {
        (self.subject_uuid.is_none() || self.discipline_uuid.is_some())
            && (self.topic_uuid.is_none() || self.subject_uuid.is_some())
            && (self.subtopic_uuid.is_none() || self.topic_uuid.is_some())
            && (!self.cross_discipline
                || (self.discipline_uuid.is_some() && self.subject_uuid.is_some()))
    }
}

/// Exact immutable Pool content before answer-free Question projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQuestionPool {
    pub metadata: QuestionPoolMetadata,
    pub question_pool_id: QuestionId,
    pub question_pool_edit_number: QuestionPoolEditNumber,
    pub bloom: Option<BloomClassificationView>,
    pub members: Vec<QuestionRevisionTuple>,
}

/// Assessment-owned exact fork facts before answer-free Question projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessmentQuestionPoolForkRecord {
    pub metadata: QuestionPoolMetadata,
    pub assessment_entry_id: AssessmentEntryId,
    pub question_pool_id: QuestionId,
    pub question_pool_edit_number: QuestionPoolEditNumber,
    pub selection_count: std::num::NonZeroU32,
    pub bloom: Option<BloomClassificationView>,
    pub members: Vec<QuestionRevisionTuple>,
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
        filter: QuestionPoolDiscoveryFilter,
        text: QuestionPoolTextFilter,
    ) -> Result<QuestionPoolDiscoveryPage, StoreError>;

    /// Resolves the current membership of one published Pool.
    async fn load_current_published_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &question_model::QuestionId,
    ) -> Result<PublishedQuestionPool, StoreError>;

    /// Corrects both Bloom dimensions for one current Pool through the
    /// classification-owned compare-and-swap number.
    async fn correct_question_pool_bloom(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: &question_model::QuestionId,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_requires_an_unbroken_hierarchy_and_anchored_cross_discipline() {
        let id = Some(Uuid::from_u128(1));
        for filter in [
            QuestionPoolDiscoveryFilter {
                subject_uuid: id,
                ..Default::default()
            },
            QuestionPoolDiscoveryFilter {
                discipline_uuid: id,
                topic_uuid: id,
                ..Default::default()
            },
            QuestionPoolDiscoveryFilter {
                discipline_uuid: id,
                subject_uuid: id,
                subtopic_uuid: id,
                ..Default::default()
            },
            QuestionPoolDiscoveryFilter {
                discipline_uuid: id,
                cross_discipline: true,
                ..Default::default()
            },
        ] {
            assert!(!filter.has_valid_structure());
        }
        assert!(QuestionPoolDiscoveryFilter::default().has_valid_structure());
        assert!(
            QuestionPoolDiscoveryFilter {
                discipline_uuid: id,
                subject_uuid: id,
                topic_uuid: id,
                subtopic_uuid: id,
                cross_discipline: true,
                bloom_cognitive_process: None,
                bloom_knowledge_dimension: None,
            }
            .has_valid_structure()
        );
    }
}
