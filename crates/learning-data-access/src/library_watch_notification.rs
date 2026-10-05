//! Private in-app Watch inbox and bounded materialization.
//!
//! This is the sole delivery boundary for Question and Question Pool Watch
//! activity. It intentionally has no email provider, recipient directory, or
//! public aggregate surface.

use async_trait::async_trait;
use question_model::{PublishedQuestionId, Timestamp};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// The two stable Library Object kinds a Watch can follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryWatchTargetKind {
    Question,
    QuestionPool,
}

/// Complete evidence for one Watch event that reaches the inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryWatchActivity {
    Revision {
        revision_number: u64,
    },
    /// A Question Pool member-list save. `edit_number` is the Pool Edit Number.
    MembersChanged {
        edit_number: u64,
    },
    Fork {
        source_revision_number: u64,
        forked_public_id: PublishedQuestionId,
    },
    ImpactNotice {
        affected_revision_number: Option<u64>,
        impact_notice_id: Uuid,
    },
}

/// One self-only, immutable in-app notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryWatchNotification {
    pub target_kind: LibraryWatchTargetKind,
    pub target_public_id: PublishedQuestionId,
    pub activity: LibraryWatchActivity,
    pub occurred_at: Timestamp,
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
