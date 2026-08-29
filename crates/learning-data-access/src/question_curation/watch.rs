//! Private in-app watch intent for published question activity.
//!
//! A Watch records one global account's interest in one published lineage or
//! immutable version. It carries no delivery mechanism or access authority;
//! future Stores and protected services provide owner-private, idempotent
//! subscription behavior.

use question_model::{ProblemVersionRef, QuestionId, UserId};

/// One published question target that an account can watch.
///
/// A lineage target follows activity for every version of one published
/// question. An exact-version target follows activity tied to that immutable
/// publication evidence. The closed enum keeps draft, course, and source
/// material outside the watch relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionWatchTarget {
    /// All watchable activity for one published question lineage.
    Lineage(QuestionId),
    /// Watchable activity tied to one immutable published question version.
    Version(ProblemVersionRef),
}

/// Private in-app activity kinds a Watch can surface.
///
/// Delivery and notification content are later service concerns. This closed
/// vocabulary establishes the durable meaning of a Watch without representing
/// an outbox, email setting, or browser notification preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuestionWatchNoticeKind {
    /// A new immutable version became available in the watched lineage.
    Version,
    /// A published fork linked to the watched lineage became available.
    Fork,
    /// An improvement-thread action affected the watched target.
    ImprovementThread,
    /// An impact notice describes a controlled update or correction effect.
    Impact,
}

/// One server-only Watch presence relation for a published question target.
///
/// The global owner and closed target are intentionally private. Protected
/// services resolve the actor from a server session before future Stores add
/// or remove this relation, so a Watch never grants catalog, course, Student,
/// workspace, publication, or grading authority. This value is deliberately
/// not serializable; browser-safe projections are separate contracts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionWatch {
    owner: UserId,
    target: QuestionWatchTarget,
}

impl QuestionWatch {
    /// Creates non-authorizing private Watch presence intent for one account.
    pub fn new(owner: UserId, target: QuestionWatchTarget) -> Self {
        Self { owner, target }
    }

    /// Returns the global account that owns this private Watch intent.
    pub fn owner(&self) -> UserId {
        self.owner
    }

    /// Returns the closed published-question target for this Watch intent.
    pub fn target(&self) -> &QuestionWatchTarget {
        &self.target
    }
}
