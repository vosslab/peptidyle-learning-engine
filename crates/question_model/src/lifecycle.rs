//! Problem lifecycle and its transitions (WP-C2, MOD-ID).
//!
//! The rule a maintainer can apply in one sentence: a draft lives in an
//! instructor workspace and has no [`ProblemId`]; publishing is the transition
//! that assigns one; a published version is immutable thereafter.
//!
//! Every state change goes through [`apply`], one fallible function. Keeping
//! the transitions in a single place means the legal moves are readable at a
//! glance, and an illegal move is a value returned rather than a state quietly
//! reached.
//!
//! Immutability of published content is what lets one published problem serve
//! thousands of courses: an assignment references `(ProblemId, VersionId)`, so
//! improving a problem creates a new version and leaves every course that
//! already assigned the old one delivering exactly what it delivered before.

use serde::{Deserialize, Serialize};

use crate::identity::{ProblemId, VersionId, WorkspaceId};

/// Where a question sits in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Lifecycle {
    /// Being authored, visible only inside its workspace.
    ///
    /// Carries no [`ProblemId`], which is what makes "drafts hold no catalog
    /// number" a property of the value rather than a rule to remember.
    Draft {
        /// The workspace authoring it.
        workspace: WorkspaceId,
        /// The version being edited.
        version: VersionId,
    },
    /// Published to the shared catalog and immutable.
    Published {
        /// The catalog identifier, assigned at this transition.
        problem: ProblemId,
        /// The immutable version published.
        version: VersionId,
    },
    /// Withdrawn from the catalog.
    ///
    /// Withdrawal hides a problem from new assignments and leaves existing
    /// ones working, because a course mid-term depends on the version it was
    /// assigned. Removal would break the record; withdrawal does not.
    Withdrawn {
        /// The catalog identifier it kept.
        problem: ProblemId,
        /// The version withdrawn.
        version: VersionId,
    },
}

/// A requested change of state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum LifecycleEvent {
    /// Move a draft into the shared catalog.
    Publish,
    /// Remove a published problem from new assignments.
    Withdraw,
    /// Return a withdrawn problem to the catalog.
    Restore,
}

/// Why a transition was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "camelCase")]
pub enum LifecycleError {
    /// The event does not apply to the current state.
    ///
    /// Republishing a published version is the common case: the version is
    /// immutable, so a change means publishing a new version instead.
    IllegalTransition,
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleError::IllegalTransition => write!(
                formatter,
                "that change does not apply to the current state; \
                 publish a new version to change published content"
            ),
        }
    }
}

impl std::error::Error for LifecycleError {}

/// Applies one event to one state.
///
/// The single door every lifecycle change passes through. Publishing mints the
/// [`ProblemId`], which is why the caller supplies one: minting happens
/// server-side, where the `generate` feature is enabled, and this function
/// stays callable from the browser for previewing what a transition would do.
///
/// # Errors
///
/// Returns [`LifecycleError::IllegalTransition`] when the event does not apply
/// to the current state.
///
/// # Examples
///
/// ```
/// use question_model::identity::{ProblemId, VersionId, WorkspaceId};
/// use question_model::lifecycle::{apply, Lifecycle, LifecycleEvent};
/// use uuid::Uuid;
///
/// let draft = Lifecycle::Draft {
///     workspace: WorkspaceId::from_uuid(Uuid::from_u128(1)),
///     version: VersionId::from_uuid(Uuid::from_u128(2)),
/// };
/// let minted = ProblemId::from_uuid(Uuid::from_u128(3));
///
/// let published = apply(draft, LifecycleEvent::Publish, minted).expect("publishing a draft");
/// assert!(matches!(published, Lifecycle::Published { .. }));
///
/// // A published version is immutable: changing it means publishing a new one.
/// assert!(apply(published, LifecycleEvent::Publish, minted).is_err());
/// ```
pub fn apply(
    state: Lifecycle,
    event: LifecycleEvent,
    minted: ProblemId,
) -> Result<Lifecycle, LifecycleError> {
    match (state, event) {
        (Lifecycle::Draft { version, .. }, LifecycleEvent::Publish) => Ok(Lifecycle::Published {
            problem: minted,
            version,
        }),
        (Lifecycle::Published { problem, version }, LifecycleEvent::Withdraw) => {
            Ok(Lifecycle::Withdrawn { problem, version })
        }
        (Lifecycle::Withdrawn { problem, version }, LifecycleEvent::Restore) => {
            Ok(Lifecycle::Published { problem, version })
        }
        _ => Err(LifecycleError::IllegalTransition),
    }
}

impl Lifecycle {
    /// The catalog identifier, once the question has one.
    ///
    /// `None` for a draft. Reading this is the same question as "is this
    /// published", with no separate flag to disagree with it.
    pub fn problem(&self) -> Option<ProblemId> {
        match self {
            Lifecycle::Draft { .. } => None,
            Lifecycle::Published { problem, .. } | Lifecycle::Withdrawn { problem, .. } => {
                Some(*problem)
            }
        }
    }

    /// Whether new assignments may reference this question.
    ///
    /// Withdrawn content stays readable for courses that already assigned it
    /// and stops appearing to new ones.
    pub fn is_assignable(&self) -> bool {
        matches!(self, Lifecycle::Published { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn draft() -> Lifecycle {
        Lifecycle::Draft {
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(1)),
            version: VersionId::from_uuid(Uuid::from_u128(2)),
        }
    }

    fn minted() -> ProblemId {
        ProblemId::from_uuid(Uuid::from_u128(3))
    }

    #[test]
    fn publishing_a_draft_assigns_the_catalog_identifier() {
        let published = apply(draft(), LifecycleEvent::Publish, minted()).expect("publish works");
        assert_eq!(published.problem(), Some(minted()));
    }

    #[test]
    fn a_draft_holds_no_catalog_identifier() {
        assert_eq!(draft().problem(), None);
        assert!(!draft().is_assignable());
    }

    #[test]
    fn republishing_a_published_version_is_refused() {
        let published = apply(draft(), LifecycleEvent::Publish, minted()).expect("publish works");
        assert_eq!(
            apply(published, LifecycleEvent::Publish, minted()),
            Err(LifecycleError::IllegalTransition)
        );
    }

    #[test]
    fn withdrawal_keeps_the_identifier_and_stops_new_assignments() {
        let published = apply(draft(), LifecycleEvent::Publish, minted()).expect("publish works");
        let withdrawn =
            apply(published, LifecycleEvent::Withdraw, minted()).expect("withdrawal works");
        assert_eq!(withdrawn.problem(), Some(minted()));
        assert!(!withdrawn.is_assignable());
    }

    #[test]
    fn a_withdrawn_problem_can_return_to_the_catalog() {
        let published = apply(draft(), LifecycleEvent::Publish, minted()).expect("publish works");
        let withdrawn =
            apply(published, LifecycleEvent::Withdraw, minted()).expect("withdrawal works");
        let restored = apply(withdrawn, LifecycleEvent::Restore, minted()).expect("restore works");
        assert!(restored.is_assignable());
    }

    #[test]
    fn restoring_a_draft_is_refused() {
        assert!(apply(draft(), LifecycleEvent::Restore, minted()).is_err());
    }
}
