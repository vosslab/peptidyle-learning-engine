//! Presence intent for an Instructor's Star on a published question lineage.
//!
//! A Star records only the global owner and question lineage. It neither
//! grants catalog access nor represents a version pin, collection membership,
//! or publication decision. Store implementations later provide idempotent
//! add/remove behavior for this relation.

use question_model::{QuestionId, UserId};

/// One server-only Star presence relation for a published question lineage.
///
/// [`QuestionId`] identifies the global lineage, allowing this intent to
/// survive ordinary immutable versions without exposing private drafts. This
/// value is deliberately not serializable: HTTP request and response shapes
/// are separate, browser-safe contracts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionStar {
    owner: UserId,
    question_id: QuestionId,
}

impl QuestionStar {
    /// Creates non-authorizing Star presence intent for one global account.
    ///
    /// Protected services resolve the actor from a server session and verify
    /// approved-Instructor scope before asking durable storage to add or
    /// remove this relation.
    pub fn new(owner: UserId, question_id: QuestionId) -> Self {
        Self { owner, question_id }
    }

    /// Returns the global account that owns this presence intent.
    pub fn owner(&self) -> UserId {
        self.owner
    }

    /// Returns the published question lineage this presence intent targets.
    pub fn question_id(&self) -> &QuestionId {
        &self.question_id
    }
}
