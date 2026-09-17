//! Private in-app Watch inbox and bounded materialization.
//!
//! This is the sole delivery boundary for Question and Question Pool Watch
//! activity. It intentionally has no email provider, recipient directory, or
//! public aggregate surface.

use async_trait::async_trait;
use question_model::{QuestionId, Timestamp};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// The two stable Library Object kinds a Watch can follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryWatchTargetKind {
    Question,
    QuestionPool,
}

/// The only event kinds that reach a Watch inbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryWatchEventKind {
    Revision,
    Fork,
    ImprovementThread,
    ImpactNotice,
}

/// One self-only, immutable in-app notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryWatchNotification {
    pub target_kind: LibraryWatchTargetKind,
    pub target_public_id: QuestionId,
    pub event_kind: LibraryWatchEventKind,
    /// The exact source Revision when that event class has one.
    pub revision_number: Option<u64>,
    /// The new public lineage for a fork event only.
    pub forked_public_id: Option<QuestionId>,
    /// Exact thread, post, or impact-notice activity for activity events only.
    pub activity_id: Option<Uuid>,
    pub occurred_at: Timestamp,
}

/// Least-privilege worker capability: materialize a bounded Watch outbox batch.
#[async_trait]
pub trait LibraryWatchNotificationStore: Send + Sync {
    async fn materialize_library_watch_notifications(&self, limit: u16) -> Result<u32, StoreError>;
}

/// Browser-facing private inbox for the authenticated active Instructor only.
#[async_trait]
pub trait LibraryWatchInboxStore: Send + Sync {
    async fn library_watch_notifications(
        &self,
        session_token_hash: SessionTokenHash,
        limit: u16,
    ) -> Result<Vec<LibraryWatchNotification>, StoreError>;
}
