//! Browser-safe activity vocabulary for retained Question Library discussions.
//!
//! This vocabulary deliberately describes a Library Object and its activity,
//! not a Watch recipient or a delivered notification. A later Watch owner can
//! use it as an atomic outbox boundary without widening this read model.

use serde::{Deserialize, Serialize};

use crate::QuestionId;

/// The two reusable Library Object lineages that can receive stewardship activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryObjectKind {
    /// A stable Published Question lineage.
    Question,
    /// A stable published Question Pool lineage.
    QuestionPool,
}

/// A stable Library Object lineage, distinct from a particular Revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryObjectReference {
    pub kind: LibraryObjectKind,
    pub public_id: QuestionId,
}

/// A later-deliverable activity concerning one stable Library Object lineage.
///
/// This is not a Watch delivery record and exposes no recipient, subscription,
/// or notification state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LibraryStewardshipEvent {
    /// An improvement thread was created or received a retained post.
    ImprovementThreadActivity { object: LibraryObjectReference },
    /// An owner-maintained impact notice was created, changed, or cancelled.
    ImpactNotice { object: LibraryObjectReference },
}

impl LibraryStewardshipEvent {
    /// The stable Library Object lineage affected by this activity.
    pub fn object(&self) -> &LibraryObjectReference {
        match self {
            Self::ImprovementThreadActivity { object } | Self::ImpactNotice { object } => object,
        }
    }
}
