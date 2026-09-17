//! Retained, text-only improvement threads and impact notices for Library Objects.

use async_trait::async_trait;
use question_model::{LibraryObjectKind, QuestionId, Timestamp};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Stable Question or Question Pool lineage selected by a verified server route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryDiscussionTarget {
    pub kind: LibraryObjectKind,
    pub public_id: QuestionId,
}

/// Retained lifecycle of one improvement thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryImprovementThreadLifecycle {
    Open,
    Resolved { resolved_at: Timestamp },
}

/// One retained post with only its approved visible author identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryImprovementPost {
    pub post_id: Uuid,
    pub author_display_name: String,
    pub body: String,
    pub created_at: Timestamp,
    pub updated_at: Option<Timestamp>,
    pub viewer_may_edit: bool,
}

/// A stable-lineage thread and its ordered retained posts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryImprovementThread {
    pub thread_id: Uuid,
    pub creation_revision_number: u64,
    pub lifecycle: LibraryImprovementThreadLifecycle,
    pub created_at: Timestamp,
    pub viewer_may_resolve: bool,
    pub posts: Vec<LibraryImprovementPost>,
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

/// The ordinary vetted-Instructor discussion view for one Library Object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryDiscussionView {
    pub viewer_may_manage: bool,
    pub threads: Vec<LibraryImprovementThread>,
    pub impact_notices: Vec<LibraryImpactNotice>,
}

/// Database-authorized retained discussion and impact-notice boundary.
#[async_trait]
pub trait LibraryDiscussionStore: Send + Sync {
    async fn library_discussion_view(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryDiscussionTarget,
    ) -> Result<LibraryDiscussionView, StoreError>;

    async fn create_improvement_thread(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        body: &str,
    ) -> Result<Uuid, StoreError>;

    async fn reply_to_improvement_thread(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        thread_id: Uuid,
        body: &str,
    ) -> Result<Uuid, StoreError>;

    async fn edit_own_improvement_post(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        post_id: Uuid,
        body: &str,
    ) -> Result<(), StoreError>;

    async fn set_improvement_thread_resolved(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        thread_id: Uuid,
        resolved: bool,
    ) -> Result<(), StoreError>;

    async fn create_impact_notice(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        affected_revision_number: Option<u64>,
        body: &str,
    ) -> Result<Uuid, StoreError>;

    async fn update_impact_notice(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        impact_notice_id: Uuid,
        affected_revision_number: Option<u64>,
        body: &str,
    ) -> Result<(), StoreError>;

    async fn cancel_impact_notice(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        impact_notice_id: Uuid,
    ) -> Result<(), StoreError>;
}
