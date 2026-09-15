//! Authenticated Instructor Star state for one Published Question lineage.
//!
//! This boundary intentionally names neither an Account nor a Product Role.
//! PostgreSQL derives both from the installed session and returns only the
//! current caller's state, public aggregate, and the approved public display
//! identities. It never returns an Account identifier, email, credential,
//! Watch state, or Student fact.

use async_trait::async_trait;
use question_model::QuestionId;

use crate::{SessionTokenHash, StoreError};

/// Browser-safe Star facts for one Published Question lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionStarProjection {
    /// Whether the authenticated Instructor currently Stars this lineage.
    pub viewer_has_starred: bool,
    /// Current number of active Instructor endorsements for this lineage.
    pub star_count: u64,
    /// Vetted active Instructor identities that visibly endorsed this lineage.
    pub starred_instructors: Vec<QuestionStarredInstructor>,
}

/// One approved public identity in a Question Star endorsement projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionStarredInstructor {
    /// Canonical public display name; no account identity or contact data.
    pub display_name: String,
}

/// Authenticated Instructor boundary for Star reads and idempotent mutations.
#[async_trait]
pub trait QuestionStarStore: Send + Sync {
    /// Reads the caller's own Star state and the lineage-wide Star count.
    ///
    /// The database rejects Students, anonymous callers, inactive Accounts,
    /// and anything other than an existing Published Question.
    async fn question_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
    ) -> Result<QuestionStarProjection, StoreError>;

    /// Sets only the authenticated Instructor's Star state for one lineage.
    /// Repeating `starred` is intentionally a successful no-op.
    async fn set_current_question_star(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
        starred: bool,
    ) -> Result<QuestionStarProjection, StoreError>;
}
