//! Authenticated creation of independently owned Blueprint Course forks.

use async_trait::async_trait;
use question_model::{
    AccountId, BlueprintMetadataEtag, BlueprintRevisionReference, RequestChecksum, Timestamp,
};

use crate::{SessionTokenHash, StoreError};

/// Immutable source fact retained by a forked Blueprint lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintForkSource {
    pub blueprint_revision: BlueprintRevisionReference,
}

/// Receipt for an idempotent fork creating an actor-owned Private child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkBlueprintCourseReceipt {
    pub blueprint_revision: BlueprintRevisionReference,
    pub source: BlueprintForkSource,
    pub metadata_etag: BlueprintMetadataEtag,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub accepted_at: Timestamp,
}

/// Closed persistence boundary for a new independent Blueprint fork.
#[async_trait]
pub trait BlueprintLineageStore: Send + Sync {
    /// Forks one exact Public or Archived source Revision into a distinct,
    /// actor-owned Private Blueprint with Revision 1. A checksum retry returns
    /// the original child, and this operation never creates Stars or Watches.
    async fn fork_blueprint_course(
        &self,
        session: SessionTokenHash,
        source: BlueprintForkSource,
        request_checksum: RequestChecksum,
    ) -> Result<ForkBlueprintCourseReceipt, StoreError>;
}
