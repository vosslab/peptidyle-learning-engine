//! Browser-safe read models for reusable published Question Pools.
//!
//! These views expose exact immutable Pool and Question Revision pins for
//! Instructor library and Assessment editing. They contain no source,
//! ownership, Course, Student, or selection-result facts.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AssessmentEditNumber, AssessmentEntryId, BloomClassificationView,
    QuestionPoolRevisionReference, QuestionRevisionReference,
    QuestionSearchBloomCognitiveProcessFacet, QuestionSearchBloomKnowledgeDimensionFacet,
    ReusableQuestionView,
};

/// Current Pool lineage metadata, independent of immutable membership Revisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolMetadata {
    pub title: String,
    pub description: String,
    pub discipline_uuid: Uuid,
    /// Current readable Discipline name for this Pool's existing classification reference.
    pub discipline_name: String,
    /// Whether `discipline_name` is retired. Retired classifications remain readable on existing
    /// references and are not new-selection choices.
    pub discipline_is_retired: bool,
    pub subject_uuid: Uuid,
    pub topic_uuid: Option<Uuid>,
    pub subtopic_uuid: Option<Uuid>,
    pub tags: Vec<String>,
}

/// One reusable published Question Pool available through the Question Library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolLibrarySummary {
    pub metadata: QuestionPoolMetadata,
    /// Current immutable published Pool Revision.
    pub question_pool_revision: QuestionPoolRevisionReference,
    /// Total members in that exact immutable Pool Revision.
    pub member_count: NonZeroU32,
    /// Exact Pool-owned Bloom Classification; member classifications do not substitute for it.
    pub bloom: BloomClassificationView,
}

/// Complete Bloom counts from the same filtered Pool discovery relation as one page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolBloomFacets {
    /// All six Cognitive Process values in teaching-guide order, including zero counts.
    pub cognitive_processes: Vec<QuestionSearchBloomCognitiveProcessFacet>,
    /// All four Knowledge Dimension values in teaching-guide order, including zero counts.
    pub knowledge_dimensions: Vec<QuestionSearchBloomKnowledgeDimensionFacet>,
}

/// Bounded Pool discovery page with whole-matching-set Bloom counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolLibraryPage {
    /// At most the caller's validated page size of current Pool Revisions.
    pub items: Vec<QuestionPoolLibrarySummary>,
    /// Opaque continuation bound to the complete normalized Pool query.
    pub next_cursor: Option<String>,
    /// Server-computed counts from all matching Pools, never the loaded page sample.
    pub bloom_facets: QuestionPoolBloomFacets,
}

/// One ordered exact Question Revision in an immutable Question Pool Revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolRevisionMemberView {
    /// Zero-based position matching `PoolRevisionMemberReference`.
    pub member_position: u32,
    /// Exact immutable Question Revision at this position.
    pub question_revision: QuestionRevisionReference,
    /// Answer-free reusable Question projection for the exact member.
    pub question: ReusableQuestionView,
}

/// Complete ordered contents of one immutable published Question Pool Revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolRevisionView {
    /// Current lineage metadata even when reading historical membership.
    pub metadata: QuestionPoolMetadata,
    /// Exact immutable Pool Revision being read.
    pub question_pool_revision: QuestionPoolRevisionReference,
    /// Exact Pool-owned Bloom Classification for `question_pool_revision`.
    pub bloom: BloomClassificationView,
    /// Members in their immutable Pool Revision order.
    pub members: Vec<QuestionPoolRevisionMemberView>,
}

/// Assessment-owned Pool fork content for an Instructor Assessment editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentQuestionPoolForkView {
    pub metadata: QuestionPoolMetadata,
    /// Stable Assessment Entry that owns this fork.
    pub assessment_entry_id: AssessmentEntryId,
    /// Exact immutable fork Pool Revision.
    pub question_pool_revision: QuestionPoolRevisionReference,
    /// Opaque current metadata validator for the Pool fork.
    pub pool_metadata_etag: Uuid,
    /// Positive number of members selected for each future Assessment Attempt.
    pub selection_count: NonZeroU32,
    /// Exact fork-owned Bloom Classification; source and member pairs do not substitute for it.
    pub bloom: BloomClassificationView,
    /// Ordered exact members in the fork Pool Revision.
    pub members: Vec<QuestionPoolRevisionMemberView>,
}

/// Accepted Assessment-owned Pool selection-count change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentQuestionPoolSelectionCountReceipt {
    /// Stable Assessment Entry whose selection count changed.
    pub assessment_entry_id: AssessmentEntryId,
    /// Accepted positive count for future Assessment Attempts.
    pub selection_count: NonZeroU32,
    /// Assessment Edit Number after the accepted change.
    pub assessment_edit_number: AssessmentEditNumber,
}
