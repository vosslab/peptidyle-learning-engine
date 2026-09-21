//! Authenticated creation of independently owned Blueprint Course forks.

use async_trait::async_trait;
use question_model::{
    AccountId, BlueprintAvailability, BlueprintCourseId, BlueprintEditNumber,
    BlueprintRevisionTuple, PublishedQuestionRevisionTuple, QuestionPoolEditNumber, QuestionPoolId,
    RequestChecksum, Timestamp,
};
use std::collections::BTreeMap;

use crate::{SessionTokenHash, StoreError, StoredBlueprintRevision};

/// One direct fork readable by the caller, with its verified owning Instructor name.
/// Private children of another Instructor never appear, including to the source owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredKnownBlueprintFork {
    pub id: BlueprintCourseId,
    pub short_name: String,
    pub long_name: String,
    pub availability: BlueprintAvailability,
    pub current_revision_tuple: BlueprintRevisionTuple,
    /// Immutable source Blueprint Revision Tuple used when this direct fork was created.
    pub source_revision_tuple: BlueprintRevisionTuple,
    pub owner_display_name: String,
}

/// Authorized exact content and current names for a Blueprint fork review.
/// Name Edit Numbers reuse the ordinary lineage metadata concurrency boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintComparisonSources {
    pub left: StoredBlueprintRevision,
    pub right: StoredBlueprintRevision,
    pub left_short_name: String,
    pub left_long_name: String,
    pub right_short_name: String,
    pub right_long_name: String,
    pub left_blueprint_edit_number: BlueprintEditNumber,
    pub right_blueprint_edit_number: BlueprintEditNumber,
    pub pool_memberships:
        BTreeMap<(QuestionPoolId, QuestionPoolEditNumber), Vec<PublishedQuestionRevisionTuple>>,
}

/// Immutable source fact retained by a forked Blueprint lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintForkSource {
    pub blueprint_revision_tuple: BlueprintRevisionTuple,
}

/// Receipt for an idempotent fork creating an actor-owned Private child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkBlueprintCourseReceipt {
    pub blueprint_revision_tuple: BlueprintRevisionTuple,
    pub source: BlueprintForkSource,
    pub blueprint_edit_number: BlueprintEditNumber,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub accepted_at: Timestamp,
}

/// Closed persistence boundary for a new independent Blueprint fork.
#[async_trait]
pub trait BlueprintLineageStore: Send + Sync {
    /// Lists only readable direct children of a currently readable source.
    /// Missing or unreadable sources are NotFound; no readable children is empty.
    async fn list_known_blueprint_forks(
        &self,
        session: SessionTokenHash,
        source_blueprint_course_id: BlueprintCourseId,
    ) -> Result<Vec<StoredKnownBlueprintFork>, StoreError>;

    /// Loads visible current heads of two related Courses and exact Pool membership.
    /// Missing, hidden or unrelated selections are uniformly NotFound.
    async fn load_blueprint_comparison_sources(
        &self,
        session: SessionTokenHash,
        left: BlueprintCourseId,
        right: BlueprintCourseId,
    ) -> Result<BlueprintComparisonSources, StoreError>;

    /// Forks one exact Public or Archived source Revision into a distinct,
    /// actor-owned Private Blueprint with Revision 1. A checksum retry returns
    /// the original child, and this operation never creates Stars or Watches.
    async fn fork_blueprint_course(
        &self,
        session: SessionTokenHash,
        source: BlueprintForkSource,
        request_checksum: RequestChecksum,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<ForkBlueprintCourseReceipt, StoreError>;
}
