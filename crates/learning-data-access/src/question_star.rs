//! Authenticated Instructor Star state for one Published Question lineage.
//!
//! This boundary intentionally names neither an Account nor a User Role.
//! PostgreSQL derives both from the installed session and returns only the
//! current caller's state, public aggregate, and public display
//! identities. It never returns an Account identifier, email, credential,
//! Watch state, or Student fact.

use async_trait::async_trait;
use question_model::{AccountId, PublishedQuestionId};

use crate::{DiscoveryPageRequest, Page, SessionTokenHash, StoreError};

/// Browser-safe Star facts for one Published Question lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionStarProjection {
    /// Whether the authenticated Instructor currently Stars this lineage.
    pub viewer_has_starred: bool,
    /// Current number of active Instructor endorsements for this lineage.
    pub star_count: u64,
    /// Active Instructor display identities that visibly endorsed this lineage.
    pub starred_instructors: Vec<QuestionStarredInstructor>,
}

/// One public identity in a Question Star endorsement projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionStarredInstructor {
    /// Canonical public display name with its canonical Profile route identity.
    pub display_name: String,
    pub account_id: AccountId,
}

/// One Published Question in the authenticated Instructor's personal collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarredQuestionSummary {
    /// Canonical Published Question identity.
    pub question_id: PublishedQuestionId,
    /// Current Published Question title.
    pub question_title: String,
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
        question_id: &PublishedQuestionId,
    ) -> Result<QuestionStarProjection, StoreError>;

    /// Sets only the authenticated Instructor's Star state for one lineage.
    /// Repeating `starred` is intentionally a successful no-op.
    async fn set_current_question_star(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
        starred: bool,
    ) -> Result<QuestionStarProjection, StoreError>;

    /// Lists only the authenticated Instructor's Starred Questions, newest first.
    ///
    /// An empty collection is a successful page. The database rejects Students,
    /// anonymous callers, and inactive Accounts.
    async fn list_current_starred_questions(
        &self,
        session_token_hash: SessionTokenHash,
        page: DiscoveryPageRequest,
    ) -> Result<Page<StarredQuestionSummary>, StoreError>;
}
