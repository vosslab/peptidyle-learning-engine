//! Browser-safe read models for reusable published Question Pools.
//!
//! These views expose the Pool owner, current Pool Edit, and exact immutable
//! Question Revision pins for Instructor library and Assessment editing.
//! They contain no Question source, Course, Student, or selection-result facts.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AccountId, AssessmentEditNumber, AssessmentEntryId, BloomClassificationView,
    PublishedQuestionRevisionTuple, QuestionBackend, QuestionLicense, QuestionPoolEditNumber,
    QuestionPoolId, QuestionPoolMetadataEditNumber, QuestionStatistics, QuestionType,
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
    pub question_pool_id: QuestionPoolId,
    /// Account that created or forked this Pool lineage.
    pub owner_account_id: AccountId,
    /// Immutable Type established by the first Pool member.
    pub question_type: QuestionType,
    /// Immutable Backend established by the first Pool member.
    pub backend: QuestionBackend,
    /// Calculated collection license; each member keeps its own exact Revision license.
    pub license: QuestionLicense,
    /// Membership version used by Assessment and historical membership references.
    pub question_pool_edit_number: QuestionPoolEditNumber,
    /// Current mutable Pool metadata concurrency token.
    pub question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber,
    /// Total members in the current Pool membership.
    pub member_count: NonZeroU32,
    /// Exact Pool-owned Bloom Classification when assigned; member classifications do not substitute.
    pub bloom: Option<BloomClassificationView>,
}

/// One ordered exact Question Revision in the current Pool membership.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolMemberView {
    /// Zero-based position in the Pool's current member list.
    pub member_position: u32,
    /// Exact immutable Question Revision at this position.
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// Answer-free reusable Question projection for the exact member.
    pub question: ReusableQuestionView,
}

/// Complete ordered contents of one current published Question Pool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolView {
    /// Current Pool metadata.
    pub metadata: QuestionPoolMetadata,
    pub question_pool_id: QuestionPoolId,
    /// Account that created or forked this Pool lineage.
    pub owner_account_id: AccountId,
    /// Immutable Type established by the first Pool member.
    pub question_type: QuestionType,
    /// Immutable Backend established by the first Pool member.
    pub backend: QuestionBackend,
    /// Calculated collection license; each member keeps its own exact Revision license.
    pub license: QuestionLicense,
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
    pub question_pool_id: QuestionPoolId,
    /// Account that created or forked this Pool lineage.
    pub owner_account_id: AccountId,
    /// Immutable Type established by the first Pool member.
    pub question_type: QuestionType,
    /// Immutable Backend established by the first Pool member.
    pub backend: QuestionBackend,
    /// Calculated collection license; each member keeps its own exact Revision license.
    pub license: QuestionLicense,
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
