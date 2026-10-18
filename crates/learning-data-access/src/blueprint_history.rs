//! Bounded ordinary-visibility reads of existing Blueprint history facts.

use async_trait::async_trait;
use question_model::BlueprintCourseId;

use crate::{NumberedPage, NumberedPageRequest, PageSize, SessionTokenHash, StoreError};
use browser_api_contract::blueprint_course::BlueprintHistoryEntryView;

/// The two independent immutable fact sequences; no merged chronology is implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintHistoryKind {
    Revisions,
    Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintHistorySort {
    Sequence,
    RecordedAt,
    Name,
}

impl BlueprintHistorySort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sequence => "sequence",
            Self::RecordedAt => "recorded_at",
            Self::Name => "name",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlueprintHistoryQuery {
    pub text: String,
    pub sort: BlueprintHistorySort,
    pub ascending: bool,
    pub availability: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BlueprintHistoryResult {
    pub page: NumberedPage<BlueprintHistoryEntryView, PageSize>,
    pub public_count: u64,
    pub archived_count: u64,
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
        query: BlueprintHistoryQuery,
        page: NumberedPageRequest,
    ) -> Result<BlueprintHistoryResult, StoreError>;
}
