//! Browser-safe read models for reusable published Question Pools.
//!
//! These views expose exact immutable Pool and Question Revision pins for
//! Instructor library and Assessment editing. They contain no source,
//! ownership, Course, Student, or selection-result facts.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AssessmentEditNumber, AssessmentEntryId, BloomClassificationView, QuestionId,
    QuestionPoolEditNumber, QuestionRevisionTuple, QuestionSearchBloomCognitiveProcessFacet,
    QuestionSearchBloomKnowledgeDimensionFacet, QuestionStatistics, ReusableQuestionView,
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
    pub question_pool_id: QuestionId,
    /// Current-state concurrency marker; not a historical membership object.
    pub question_pool_edit_number: QuestionPoolEditNumber,
    /// Total members in the current Pool membership.
    pub member_count: NonZeroU32,
    /// Exact Pool-owned Bloom Classification when assigned; member classifications do not substitute.
    pub bloom: Option<BloomClassificationView>,
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
    /// At most the caller's validated page size of current Pools.
    pub items: Vec<QuestionPoolLibrarySummary>,
    /// Opaque continuation bound to the complete normalized Pool query.
    pub next_cursor: Option<String>,
    /// Server-computed counts from all matching Pools, never the loaded page sample.
    pub bloom_facets: QuestionPoolBloomFacets,
}

/// One ordered exact Question Revision in the current Pool membership.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolMemberView {
    /// Zero-based position in the Pool's current member list.
    pub member_position: u32,
    /// Exact immutable Question Revision at this position.
    pub question_revision: QuestionRevisionTuple,
    /// Answer-free reusable Question projection for the exact member.
    pub question: ReusableQuestionView,
}

/// Complete ordered contents of one current published Question Pool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolView {
    /// Current Pool metadata.
    pub metadata: QuestionPoolMetadata,
    pub question_pool_id: QuestionId,
    /// Current-state concurrency marker; not a historical membership object.
    pub question_pool_edit_number: QuestionPoolEditNumber,
    /// Exact Pool-owned Bloom Classification when assigned.
    pub bloom: Option<BloomClassificationView>,
    /// Members in current Pool order.
    pub members: Vec<QuestionPoolMemberView>,
    /// Instructor-visible Pool issued_count plus current-member outcome rollup.
    pub evidence: QuestionStatistics,
}

/// Assessment-owned Pool fork content for an Instructor Assessment editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentQuestionPoolForkView {
    pub metadata: QuestionPoolMetadata,
    /// Stable Assessment Entry that owns this fork.
    pub assessment_entry_id: AssessmentEntryId,
    pub question_pool_id: QuestionId,
    /// Current-state concurrency marker for the Assessment-owned fork Pool.
    pub question_pool_edit_number: QuestionPoolEditNumber,
    /// Positive number of members selected for each future Assessment Attempt.
    pub selection_count: NonZeroU32,
    /// Exact fork-owned Bloom Classification when assigned; source and member pairs do not substitute.
    pub bloom: Option<BloomClassificationView>,
    /// Ordered current members of the fork Pool.
    pub members: Vec<QuestionPoolMemberView>,
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
