//! Problem lifecycle and its transitions (WP-C2, MOD-ID).
//!
//! A draft lives in a private workspace and has no [`ProblemId`]. Validation is
//! explicit, publication is the only transition that assigns a catalog ID,
//! and deprecated or archived versions remain exactly resolvable. Deprecation
//! hides discovery; deprecation and archival retain exact history while blocking
//! ordinary new selection.

use serde::{Deserialize, Serialize};

use crate::catalog::ProblemVersionRef;
use crate::identity::{ProblemId, VersionId, WorkspaceId};

/// Where a question sits in the required one-way content lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Lifecycle {
    /// Freely editable, private workspace content with no catalog identifier.
    Draft {
        /// Private workspace authoring the question.
        workspace: WorkspaceId,
    },
    /// Validation passed, but publication has not minted a catalog identifier.
    Validated {
        /// Private workspace authoring the question.
        workspace: WorkspaceId,
    },
    /// Immutable version available for discovery and ordinary new selection.
    Published {
        /// Stable catalog problem assigned only by publication.
        problem: ProblemId,
        /// Immutable published version.
        version: VersionId,
    },
    /// Hidden from discovery but retained for authorized exact-pin resolution.
    Deprecated {
        /// Stable catalog problem.
        problem: ProblemId,
        /// Immutable deprecated version.
        version: VersionId,
        /// Author-supplied explanation, including corrections when applicable.
        reason: String,
    },
    /// Long-term historical record, still resolvable through existing exact pins.
    Archived {
        /// Stable catalog problem.
        problem: ProblemId,
        /// Immutable archived version.
        version: VersionId,
        /// Deprecation explanation retained with the historical version.
        reason: String,
    },
}

/// One requested lifecycle transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "event",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LifecycleEvent {
    /// Record that all publication validation passed.
    Validate,
    /// Publish a validated candidate with a server-minted catalog identifier.
    Publish {
        /// Complete immutable reference minted by publication.
        publication: ProblemVersionRef,
    },
    /// Hide a published version from discovery while keeping exact references.
    Deprecate {
        /// Required author explanation.
        reason: String,
    },
    /// Move a deprecated version into long-term historical status.
    Archive,
}

/// Why a lifecycle transition was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "camelCase")]
pub enum LifecycleError {
    /// The requested event does not follow the required one-way sequence.
    IllegalTransition,
    /// Deprecation must explain why the version should not be newly assigned.
    EmptyDeprecationReason,
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IllegalTransition => write!(
                formatter,
                "that change does not follow draft, validated, published, deprecated, archived"
            ),
            Self::EmptyDeprecationReason => {
                formatter.write_str("deprecation requires a nonempty reason")
            }
        }
    }
}

impl std::error::Error for LifecycleError {}

/// Applies one event to one state.
///
/// # Errors
///
/// Returns [`LifecycleError`] when the event skips or reverses the required
/// sequence, or when deprecation has no explanation.
///
/// # Examples
///
/// ```
/// use question_model::{Lifecycle, LifecycleEvent, ProblemId, ProblemVersionRef, VersionId, WorkspaceId};
/// use uuid::Uuid;
///
/// let draft = Lifecycle::Draft {
///     workspace: WorkspaceId::from_uuid(Uuid::from_u128(1)),
/// };
/// let validated = question_model::lifecycle::apply(draft, LifecycleEvent::Validate)
///     .expect("validation transition");
/// let published = question_model::lifecycle::apply(
///     validated,
///     LifecycleEvent::Publish { publication: ProblemVersionRef {
///         problem: ProblemId::from_uuid(Uuid::from_u128(3)),
///         version: VersionId::from_uuid(Uuid::from_u128(2)),
///     } },
/// )
/// .expect("publish transition");
/// assert!(published.is_eligible_for_ordinary_new_selection());
/// ```
pub fn apply(state: Lifecycle, event: LifecycleEvent) -> Result<Lifecycle, LifecycleError> {
    match (state, event) {
        (Lifecycle::Draft { workspace }, LifecycleEvent::Validate) => {
            Ok(Lifecycle::Validated { workspace })
        }
        (Lifecycle::Validated { .. }, LifecycleEvent::Publish { publication }) => {
            Ok(Lifecycle::Published {
                problem: publication.problem,
                version: publication.version,
            })
        }
        (Lifecycle::Published { problem, version }, LifecycleEvent::Deprecate { reason }) => {
            let reason = reason.trim();
            if reason.is_empty() {
                return Err(LifecycleError::EmptyDeprecationReason);
            }
            Ok(Lifecycle::Deprecated {
                problem,
                version,
                reason: reason.to_string(),
            })
        }
        (
            Lifecycle::Deprecated {
                problem,
                version,
                reason,
            },
            LifecycleEvent::Archive,
        ) => Ok(Lifecycle::Archived {
            problem,
            version,
            reason,
        }),
        _ => Err(LifecycleError::IllegalTransition),
    }
}

impl Lifecycle {
    /// Catalog identifier present only after successful publication.
    pub fn problem(&self) -> Option<ProblemId> {
        match self {
            Self::Draft { .. } | Self::Validated { .. } => None,
            Self::Published { problem, .. }
            | Self::Deprecated { problem, .. }
            | Self::Archived { problem, .. } => Some(*problem),
        }
    }

    /// Version present only after successful publication.
    pub fn version(&self) -> Option<VersionId> {
        match self {
            Self::Draft { .. } | Self::Validated { .. } => None,
            Self::Published { version, .. }
            | Self::Deprecated { version, .. }
            | Self::Archived { version, .. } => Some(*version),
        }
    }

    /// Whether catalog browsing should discover this version.
    pub fn is_discoverable(&self) -> bool {
        matches!(self, Self::Published { .. })
    }

    /// Whether this version can create a new reference through ordinary selection.
    ///
    /// This does not govern resolution of an existing exact immutable pin.
    pub fn is_eligible_for_ordinary_new_selection(&self) -> bool {
        matches!(self, Self::Published { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn draft() -> Lifecycle {
        Lifecycle::Draft {
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(1)),
        }
    }

    fn problem() -> ProblemId {
        ProblemId::from_uuid(Uuid::from_u128(3))
    }

    fn published() -> Lifecycle {
        let validated = apply(draft(), LifecycleEvent::Validate).expect("validation works");
        apply(
            validated,
            LifecycleEvent::Publish {
                publication: ProblemVersionRef {
                    problem: problem(),
                    version: VersionId::from_uuid(Uuid::from_u128(2)),
                },
            },
        )
        .expect("publish works")
    }

    #[test]
    fn publication_requires_validation_and_is_the_only_id_minting_transition() {
        assert_eq!(draft().problem(), None);
        assert_eq!(
            apply(
                draft(),
                LifecycleEvent::Publish {
                    publication: ProblemVersionRef {
                        problem: problem(),
                        version: VersionId::from_uuid(Uuid::from_u128(2))
                    }
                }
            ),
            Err(LifecycleError::IllegalTransition)
        );
        let published = published();
        assert_eq!(published.problem(), Some(problem()));
        assert!(published.is_discoverable());
        assert!(published.is_eligible_for_ordinary_new_selection());
    }

    #[test]
    fn deprecation_preserves_exact_resolution_identity_but_blocks_ordinary_selection() {
        assert_eq!(
            apply(
                published(),
                LifecycleEvent::Deprecate {
                    reason: "  ".to_string(),
                },
            ),
            Err(LifecycleError::EmptyDeprecationReason)
        );
        let deprecated = apply(
            published(),
            LifecycleEvent::Deprecate {
                reason: " Corrected molecular mass ".to_string(),
            },
        )
        .expect("deprecation works");
        assert_eq!(deprecated.problem(), Some(problem()));
        assert!(!deprecated.is_discoverable());
        assert!(!deprecated.is_eligible_for_ordinary_new_selection());
        assert!(matches!(
            deprecated,
            Lifecycle::Deprecated { ref reason, .. } if reason == "Corrected molecular mass"
        ));
    }

    #[test]
    fn archived_versions_are_terminal_but_keep_their_identity_and_reason() {
        let deprecated = apply(
            published(),
            LifecycleEvent::Deprecate {
                reason: "Superseded".to_string(),
            },
        )
        .expect("deprecation works");
        let archived = apply(deprecated, LifecycleEvent::Archive).expect("archive works");

        assert_eq!(archived.problem(), Some(problem()));
        assert_eq!(
            archived.version(),
            Some(VersionId::from_uuid(Uuid::from_u128(2)))
        );
        assert!(!archived.is_discoverable());
        assert!(!archived.is_eligible_for_ordinary_new_selection());
        assert_eq!(
            apply(archived, LifecycleEvent::Validate),
            Err(LifecycleError::IllegalTransition)
        );
    }
}
