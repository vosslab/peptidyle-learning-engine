//! Retained impact notices for Question Library Objects.

use async_trait::async_trait;
use question_model::{LibraryObjectKind, PublishedQuestionId, Timestamp};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Stable Question or Question Pool lineage selected by a verified server route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryObjectTarget {
    pub kind: LibraryObjectKind,
    pub public_id: PublishedQuestionId,
}

/// Retained lifecycle of an impact notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryImpactNoticeLifecycle {
    Active,
    Cancelled { cancelled_at: Timestamp },
}

/// A maintained impact notice. Cancellation retains this record and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryImpactNotice {
    pub impact_notice_id: Uuid,
    pub affected_revision_number: Option<u64>,
    pub author_display_name: String,
    pub body: String,
    pub lifecycle: LibraryImpactNoticeLifecycle,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub viewer_may_manage: bool,
}

/// Database-authorized impact-notice mutation boundary.
#[async_trait]
pub trait LibraryImpactNoticeStore: Send + Sync {
    async fn create_impact_notice(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryObjectTarget,
        affected_revision_number: Option<u64>,
        body: &str,
    ) -> Result<Uuid, StoreError>;

    async fn update_impact_notice(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryObjectTarget,
        impact_notice_id: Uuid,
        affected_revision_number: Option<u64>,
        body: &str,
    ) -> Result<(), StoreError>;

    async fn cancel_impact_notice(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryObjectTarget,
        impact_notice_id: Uuid,
    ) -> Result<(), StoreError>;
}
