//! Bounded ordinary-visibility reads of existing Blueprint history facts.

use async_trait::async_trait;
use question_model::BlueprintCourseId;

use crate::{Page, PageRequest, SessionTokenHash, StoreError};
use browser_api_contract::blueprint_course::BlueprintHistoryEntryView;

/// The two independent immutable fact sequences; no merged chronology is implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintHistoryKind {
    Revisions,
    Metadata,
}

impl BlueprintHistoryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Revisions => "revisions",
            Self::Metadata => "metadata",
        }
    }
}

/// Read-only history under the same visibility as exact Revision reads.
#[async_trait]
pub trait BlueprintHistoryStore: Send + Sync {
    async fn list_blueprint_history(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        kind: BlueprintHistoryKind,
        page: PageRequest,
    ) -> Result<Page<BlueprintHistoryEntryView>, StoreError>;
}
