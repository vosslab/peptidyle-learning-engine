//! Durable, server-owned state for shared-question change proposals.
//!
//! A proposal keeps its private source and rationale in the authoring
//! workspace.  This module owns only the exact-base lifecycle and the safe
//! validation, impact, contributor-credit, and decision facts that durable
//! catalog persistence must protect.  Browser-safe proposal views are a
//! separate B5 contract.

use question_model::{ProblemVersionRef, PublicByline};
use uuid::Uuid;

/// Opaque durable identity for one improvement proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QuestionChangeProposalId(Uuid);

impl QuestionChangeProposalId {
    /// Wraps an opaque identifier read from durable proposal storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the opaque value used by storage and audit relations.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Opaque durable identity for one question-improvement lifecycle fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QuestionImprovementEventId(Uuid);

impl QuestionImprovementEventId {
    /// Wraps an opaque identifier read from durable improvement-event storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the opaque value used by storage and audit relations.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Closed lifecycle fact for an improvement proposal thread.
///
/// The event records proposal linkage only. Contributor credit remains owned
/// by [`QuestionChangeProposal`], and persistence establishes event ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionImprovementEventKind {
    Submitted,
    Accepted {
        successor: ProblemVersionRef,
    },
    Rejected,
    Stale,
    Resubmitted {
        predecessor_proposal_id: QuestionChangeProposalId,
        predecessor_base: ProblemVersionRef,
    },
}

impl QuestionImprovementEventKind {
    /// Returns the accepted successor when this event records acceptance.
    pub fn successor(self) -> Option<ProblemVersionRef> {
        match self {
            Self::Accepted { successor } => Some(successor),
            Self::Submitted | Self::Rejected | Self::Stale | Self::Resubmitted { .. } => None,
        }
    }

    /// Returns the predecessor proposal when this event records resubmission.
    pub fn predecessor_proposal_id(self) -> Option<QuestionChangeProposalId> {
        match self {
            Self::Resubmitted {
                predecessor_proposal_id,
                ..
            } => Some(predecessor_proposal_id),
            Self::Submitted | Self::Accepted { .. } | Self::Rejected | Self::Stale => None,
        }
    }

    /// Returns the predecessor base when this event records resubmission.
    pub fn predecessor_base(self) -> Option<ProblemVersionRef> {
        match self {
            Self::Resubmitted {
                predecessor_base, ..
            } => Some(predecessor_base),
            Self::Submitted | Self::Accepted { .. } | Self::Rejected | Self::Stale => None,
        }
    }
}

/// Immutable proposal-thread fact retained by the stewardship persistence boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionImprovementEvent {
    id: QuestionImprovementEventId,
    proposal_id: QuestionChangeProposalId,
    base: ProblemVersionRef,
    kind: QuestionImprovementEventKind,
}

impl QuestionImprovementEvent {
    /// Creates one checked immutable improvement-event fact.
    pub fn new(
        id: QuestionImprovementEventId,
        proposal_id: QuestionChangeProposalId,
        base: ProblemVersionRef,
        kind: QuestionImprovementEventKind,
    ) -> Result<Self, QuestionImprovementEventError> {
        match kind {
            QuestionImprovementEventKind::Accepted { successor } => {
                if successor.problem != base.problem {
                    return Err(QuestionImprovementEventError::AcceptedSuccessorChangesLineage);
                }
                if successor.version == base.version {
                    return Err(QuestionImprovementEventError::AcceptedSuccessorDoesNotAdvance);
                }
            }
            QuestionImprovementEventKind::Resubmitted {
                predecessor_proposal_id,
                predecessor_base,
            } => {
                if predecessor_proposal_id == proposal_id {
                    return Err(QuestionImprovementEventError::ResubmissionSelfReference);
                }
                if predecessor_base.problem != base.problem {
                    return Err(QuestionImprovementEventError::ResubmissionChangesLineage);
                }
                if predecessor_base.version == base.version {
                    return Err(QuestionImprovementEventError::ResubmissionDoesNotAdvance);
                }
            }
            QuestionImprovementEventKind::Submitted
            | QuestionImprovementEventKind::Rejected
            | QuestionImprovementEventKind::Stale => {}
        }

        Ok(Self {
            id,
            proposal_id,
            base,
            kind,
        })
    }

    pub fn id(self) -> QuestionImprovementEventId {
        self.id
    }

    pub fn proposal_id(self) -> QuestionChangeProposalId {
        self.proposal_id
    }

    pub fn base(self) -> ProblemVersionRef {
        self.base
    }

    pub fn kind(self) -> QuestionImprovementEventKind {
        self.kind
    }
}

/// Why an improvement event cannot preserve a valid proposal-thread relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionImprovementEventError {
    AcceptedSuccessorChangesLineage,
    AcceptedSuccessorDoesNotAdvance,
    ResubmissionSelfReference,
    ResubmissionChangesLineage,
    ResubmissionDoesNotAdvance,
}

impl std::fmt::Display for QuestionImprovementEventError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::AcceptedSuccessorChangesLineage => {
                "an accepted improvement successor must remain in its question lineage"
            }
            Self::AcceptedSuccessorDoesNotAdvance => {
                "an accepted improvement successor must be a new immutable version"
            }
            Self::ResubmissionSelfReference => {
                "a resubmission must reference a distinct predecessor proposal"
            }
            Self::ResubmissionChangesLineage => {
                "a resubmission must remain in its question lineage"
            }
            Self::ResubmissionDoesNotAdvance => {
                "a resubmission must use a changed question base version"
            }
        })
    }
}

impl std::error::Error for QuestionImprovementEventError {}

/// Meaning-based classification produced by publication validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionSemanticChange {
    Presentation,
    Metadata,
    CompatibleImprovement,
    GradingSemanticCorrection,
    RequiresFullFork,
}

/// Safe classification of scoring work required by a validated proposal.
///
/// The later automated-grading operation resolves affected exact pins and
/// owns any generation-fenced recalculation.  This type stores no assignment,
/// Student, response, or score identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionGradingImpact {
    NoGradingImpact,
    FutureRunsOnly,
    ImpactAssessmentRequired,
    GenerationFencedRecalculationRequired,
}

/// Validation evidence retained with a submitted proposal.
///
/// It records a validated outcome.  The authoritative publication operation
/// proves validation before calling this crate-private constructor, rather
/// than treating this value as a self-authenticating authorization receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionChangeProposalValidation {
    semantic_change: QuestionSemanticChange,
    grading_impact: QuestionGradingImpact,
}

impl QuestionChangeProposalValidation {
    /// Checks the structural pairing of already-classified proposal evidence.
    ///
    /// This constructor does not prove publication validation.  The
    /// authoritative publication operation validates the private workspace
    /// source before recording this classification with a proposal.
    pub fn checked_classification(
        semantic_change: QuestionSemanticChange,
        grading_impact: QuestionGradingImpact,
    ) -> Result<Self, QuestionChangeProposalValidationError> {
        if semantic_change == QuestionSemanticChange::RequiresFullFork {
            return Err(QuestionChangeProposalValidationError::RequiresFullFork);
        }
        if semantic_change == QuestionSemanticChange::GradingSemanticCorrection
            && grading_impact == QuestionGradingImpact::NoGradingImpact
        {
            return Err(
                QuestionChangeProposalValidationError::GradingCorrectionRequiresImpactAssessment,
            );
        }
        Ok(Self {
            semantic_change,
            grading_impact,
        })
    }

    pub fn semantic_change(self) -> QuestionSemanticChange {
        self.semantic_change
    }

    pub fn grading_impact(self) -> QuestionGradingImpact {
        self.grading_impact
    }
}

/// Why an authoritative publication operation cannot record validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionChangeProposalValidationError {
    RequiresFullFork,
    GradingCorrectionRequiresImpactAssessment,
}

impl std::fmt::Display for QuestionChangeProposalValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::RequiresFullFork => {
                "an incompatible question change requires a private full-fork draft"
            }
            Self::GradingCorrectionRequiresImpactAssessment => {
                "a grading-semantic correction requires explicit impact assessment"
            }
        })
    }
}

impl std::error::Error for QuestionChangeProposalValidationError {}

/// A catalog head locked by the transaction that evaluates one proposal.
///
/// The C/D catalog-persistence owner will add its module-private constructor
/// from the locked current-head read.  Until then this type is constructed
/// only by this module's tests.  Server and browser callers cannot manufacture
/// one from an arbitrary version reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentQuestionHead(ProblemVersionRef);

impl CurrentQuestionHead {
    fn version(self) -> ProblemVersionRef {
        self.0
    }
}

/// A successor minted by the same transaction that locked the current head.
///
/// The C/D catalog-persistence owner will add its module-private constructor
/// after minting and recording a new immutable version from that exact head.
/// Until then this type is constructed only by this module's tests.  Its
/// private fields prevent an operation caller from attaching an arbitrary or
/// historical version to a proposal decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MintedQuestionSuccessor {
    base: ProblemVersionRef,
    successor: ProblemVersionRef,
}

/// Closed owner decision applied by the catalog operation.
///
/// Authorization belongs to that operation.  This durable transition carries
/// no actor identity, role, or authority receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionChangeProposalDecision {
    Accept(MintedQuestionSuccessor),
    Reject,
}

/// Closed lifecycle state for a validated improvement proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionChangeProposalState {
    Submitted,
    Accepted {
        published_version: ProblemVersionRef,
    },
    Rejected,
    Stale,
}

impl QuestionChangeProposalState {
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Submitted)
    }
}

/// Durable aggregate for one validated shared-question improvement proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionChangeProposal {
    id: QuestionChangeProposalId,
    base: ProblemVersionRef,
    contributor_credit: PublicByline,
    validation: QuestionChangeProposalValidation,
    state: QuestionChangeProposalState,
}

impl QuestionChangeProposal {
    /// Records a proposal after the authoritative publication operation has
    /// validated its private workspace source.
    pub(crate) fn submitted(
        id: QuestionChangeProposalId,
        base: ProblemVersionRef,
        contributor_credit: PublicByline,
        validation: QuestionChangeProposalValidation,
    ) -> Self {
        Self {
            id,
            base,
            contributor_credit,
            validation,
            state: QuestionChangeProposalState::Submitted,
        }
    }

    pub fn id(&self) -> QuestionChangeProposalId {
        self.id
    }

    pub fn base(&self) -> ProblemVersionRef {
        self.base
    }

    pub fn contributor_credit(&self) -> &PublicByline {
        &self.contributor_credit
    }

    pub fn validation(&self) -> QuestionChangeProposalValidation {
        self.validation
    }

    pub fn state(&self) -> QuestionChangeProposalState {
        self.state
    }

    /// Applies a decision using the transaction's locked current head.
    ///
    /// A head mismatch makes this proposal stale and requires a fresh,
    /// independently identified submission with freshly recorded validation.
    pub fn decide(
        &mut self,
        current_head: CurrentQuestionHead,
        decision: QuestionChangeProposalDecision,
    ) -> Result<(), QuestionChangeProposalTransitionError> {
        if self.state != QuestionChangeProposalState::Submitted {
            return Err(QuestionChangeProposalTransitionError::TerminalState { state: self.state });
        }

        let current_base = current_head.version();
        if current_base != self.base {
            self.state = QuestionChangeProposalState::Stale;
            return Err(QuestionChangeProposalTransitionError::StaleBase {
                base: self.base,
                current_base,
            });
        }

        self.state = match decision {
            QuestionChangeProposalDecision::Accept(successor) => {
                if successor.base != current_base {
                    return Err(QuestionChangeProposalTransitionError::SuccessorWitnessMismatch);
                }
                if successor.successor.problem != current_base.problem {
                    return Err(QuestionChangeProposalTransitionError::SuccessorChangesLineage);
                }
                if successor.successor.version == current_base.version {
                    return Err(QuestionChangeProposalTransitionError::SuccessorDoesNotAdvance);
                }
                QuestionChangeProposalState::Accepted {
                    published_version: successor.successor,
                }
            }
            QuestionChangeProposalDecision::Reject => QuestionChangeProposalState::Rejected,
        };
        Ok(())
    }

    /// Starts a new proposal from the transaction's witnessed current head.
    ///
    /// This preserves the original contributor credit but requires a distinct
    /// server-minted proposal identity and fresh validation because rebasing
    /// private source may change its meaning or grading impact.
    pub fn resubmit(
        &self,
        id: QuestionChangeProposalId,
        current_head: CurrentQuestionHead,
        validation: QuestionChangeProposalValidation,
    ) -> Result<Self, QuestionChangeProposalTransitionError> {
        if self.state != QuestionChangeProposalState::Stale {
            return Err(QuestionChangeProposalTransitionError::ResubmissionRequiresStale);
        }
        if id == self.id {
            return Err(QuestionChangeProposalTransitionError::ResubmissionRequiresDistinctId);
        }

        let base = current_head.version();
        if base.problem != self.base.problem {
            return Err(QuestionChangeProposalTransitionError::RebasedHeadChangesLineage);
        }
        if base.version == self.base.version {
            return Err(QuestionChangeProposalTransitionError::RebasedHeadDoesNotAdvance);
        }

        Ok(Self::submitted(
            id,
            base,
            self.contributor_credit.clone(),
            validation,
        ))
    }
}

/// A refused durable proposal transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionChangeProposalTransitionError {
    TerminalState {
        state: QuestionChangeProposalState,
    },
    StaleBase {
        base: ProblemVersionRef,
        current_base: ProblemVersionRef,
    },
    SuccessorChangesLineage,
    SuccessorDoesNotAdvance,
    SuccessorWitnessMismatch,
    ResubmissionRequiresStale,
    ResubmissionRequiresDistinctId,
    RebasedHeadChangesLineage,
    RebasedHeadDoesNotAdvance,
}

impl std::fmt::Display for QuestionChangeProposalTransitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::TerminalState { .. } => "a terminal proposal cannot receive another decision",
            Self::StaleBase { .. } => {
                "the question lineage advanced; rebase and resubmit the proposal"
            }
            Self::SuccessorChangesLineage => {
                "a minted proposal successor must remain in its question lineage"
            }
            Self::SuccessorDoesNotAdvance => {
                "a minted proposal successor must be a new immutable version"
            }
            Self::SuccessorWitnessMismatch => {
                "a proposal decision requires a successor minted from its exact current head"
            }
            Self::ResubmissionRequiresStale => "only a stale proposal can be resubmitted",
            Self::ResubmissionRequiresDistinctId => {
                "a rebased proposal requires a distinct server-minted identity"
            }
            Self::RebasedHeadChangesLineage => {
                "a rebased proposal must target the existing question lineage"
            }
            Self::RebasedHeadDoesNotAdvance => {
                "a rebased proposal must target a newer witnessed question head"
            }
        })
    }
}

impl std::error::Error for QuestionChangeProposalTransitionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{ProblemId, PublicAuthorName, VersionId};

    fn version(problem: u128, version: u128) -> ProblemVersionRef {
        ProblemVersionRef {
            problem: ProblemId::from_uuid(Uuid::from_u128(problem)),
            version: VersionId::from_uuid(Uuid::from_u128(version)),
        }
    }

    fn head(problem: u128, version_id: u128) -> CurrentQuestionHead {
        CurrentQuestionHead(version(problem, version_id))
    }

    fn validation() -> QuestionChangeProposalValidation {
        QuestionChangeProposalValidation::checked_classification(
            QuestionSemanticChange::CompatibleImprovement,
            QuestionGradingImpact::NoGradingImpact,
        )
        .expect("compatible improvement is valid")
    }

    fn submitted() -> QuestionChangeProposal {
        QuestionChangeProposal::submitted(
            QuestionChangeProposalId::from_uuid(Uuid::from_u128(11)),
            version(1, 2),
            PublicByline::new(vec![
                PublicAuthorName::new("Ada Lovelace".into()).expect("valid"),
            ])
            .expect("valid byline"),
            validation(),
        )
    }

    #[test]
    fn validation_rejects_incompatible_or_unassessed_grading_changes() {
        assert_eq!(
            QuestionChangeProposalValidation::checked_classification(
                QuestionSemanticChange::RequiresFullFork,
                QuestionGradingImpact::NoGradingImpact,
            ),
            Err(QuestionChangeProposalValidationError::RequiresFullFork)
        );
        assert_eq!(
            QuestionChangeProposalValidation::checked_classification(
                QuestionSemanticChange::GradingSemanticCorrection,
                QuestionGradingImpact::NoGradingImpact,
            ),
            Err(QuestionChangeProposalValidationError::GradingCorrectionRequiresImpactAssessment)
        );
    }

    #[test]
    fn stale_exact_head_requires_a_distinct_validated_resubmission() {
        let mut proposal = submitted();
        let error = proposal
            .decide(head(1, 3), QuestionChangeProposalDecision::Reject)
            .expect_err("an advanced head makes a proposal stale");
        assert!(matches!(
            error,
            QuestionChangeProposalTransitionError::StaleBase { .. }
        ));
        assert_eq!(proposal.state(), QuestionChangeProposalState::Stale);

        assert_eq!(
            proposal.resubmit(proposal.id(), head(1, 3), validation(),),
            Err(QuestionChangeProposalTransitionError::ResubmissionRequiresDistinctId)
        );
        let rebased = proposal
            .resubmit(
                QuestionChangeProposalId::from_uuid(Uuid::from_u128(12)),
                head(1, 3),
                validation(),
            )
            .expect("a fresh proposal can use the exact witnessed current head");
        assert_eq!(rebased.base(), version(1, 3));
        assert_eq!(rebased.state(), QuestionChangeProposalState::Submitted);
    }

    #[test]
    fn accepted_improvement_event_retains_exact_proposal_and_version_ancestry() {
        let event_id = QuestionImprovementEventId::from_uuid(Uuid::from_u128(21));
        let proposal_id = QuestionChangeProposalId::from_uuid(Uuid::from_u128(22));
        let base = version(1, 2);
        let successor = version(1, 3);
        let event = QuestionImprovementEvent::new(
            event_id,
            proposal_id,
            base,
            QuestionImprovementEventKind::Accepted { successor },
        )
        .expect("an advancing same-lineage successor is valid");

        assert_eq!(event.id(), event_id);
        assert_eq!(event.proposal_id(), proposal_id);
        assert_eq!(event.base(), base);
        assert_eq!(event.kind().successor(), Some(successor));
        assert_eq!(event.kind().predecessor_proposal_id(), None);
        assert_eq!(event.kind().predecessor_base(), None);
    }

    #[test]
    fn accepted_improvement_event_refuses_cross_lineage_or_non_advancing_successors() {
        let id = QuestionImprovementEventId::from_uuid(Uuid::from_u128(21));
        let proposal_id = QuestionChangeProposalId::from_uuid(Uuid::from_u128(22));
        let base = version(1, 2);

        assert_eq!(
            QuestionImprovementEvent::new(
                id,
                proposal_id,
                base,
                QuestionImprovementEventKind::Accepted {
                    successor: version(2, 3),
                },
            ),
            Err(QuestionImprovementEventError::AcceptedSuccessorChangesLineage)
        );
        assert_eq!(
            QuestionImprovementEvent::new(
                id,
                proposal_id,
                base,
                QuestionImprovementEventKind::Accepted { successor: base },
            ),
            Err(QuestionImprovementEventError::AcceptedSuccessorDoesNotAdvance)
        );
    }

    #[test]
    fn resubmitted_improvement_event_retains_new_and_predecessor_proposal_links() {
        let event_id = QuestionImprovementEventId::from_uuid(Uuid::from_u128(31));
        let predecessor_proposal_id = QuestionChangeProposalId::from_uuid(Uuid::from_u128(32));
        let proposal_id = QuestionChangeProposalId::from_uuid(Uuid::from_u128(33));
        let predecessor_base = version(1, 2);
        let base = version(1, 3);
        let event = QuestionImprovementEvent::new(
            event_id,
            proposal_id,
            base,
            QuestionImprovementEventKind::Resubmitted {
                predecessor_proposal_id,
                predecessor_base,
            },
        )
        .expect("a new proposal may link to an advanced same-lineage predecessor base");

        assert_eq!(event.id(), event_id);
        assert_eq!(event.proposal_id(), proposal_id);
        assert_eq!(event.base(), base);
        assert_eq!(event.kind().successor(), None);
        assert_eq!(
            event.kind().predecessor_proposal_id(),
            Some(predecessor_proposal_id)
        );
        assert_eq!(event.kind().predecessor_base(), Some(predecessor_base));
    }

    #[test]
    fn resubmitted_improvement_event_refuses_self_cross_lineage_or_unchanged_base() {
        let id = QuestionImprovementEventId::from_uuid(Uuid::from_u128(31));
        let predecessor_proposal_id = QuestionChangeProposalId::from_uuid(Uuid::from_u128(32));
        let proposal_id = QuestionChangeProposalId::from_uuid(Uuid::from_u128(33));
        let base = version(1, 3);

        assert_eq!(
            QuestionImprovementEvent::new(
                id,
                proposal_id,
                base,
                QuestionImprovementEventKind::Resubmitted {
                    predecessor_proposal_id: proposal_id,
                    predecessor_base: version(1, 2),
                },
            ),
            Err(QuestionImprovementEventError::ResubmissionSelfReference)
        );
        assert_eq!(
            QuestionImprovementEvent::new(
                id,
                proposal_id,
                base,
                QuestionImprovementEventKind::Resubmitted {
                    predecessor_proposal_id,
                    predecessor_base: version(2, 2),
                },
            ),
            Err(QuestionImprovementEventError::ResubmissionChangesLineage)
        );
        assert_eq!(
            QuestionImprovementEvent::new(
                id,
                proposal_id,
                base,
                QuestionImprovementEventKind::Resubmitted {
                    predecessor_proposal_id,
                    predecessor_base: base,
                },
            ),
            Err(QuestionImprovementEventError::ResubmissionDoesNotAdvance)
        );
    }

    #[test]
    fn only_a_successor_minted_from_the_exact_current_head_can_be_accepted() {
        let current_head = head(1, 2);
        let wrong_lineage = MintedQuestionSuccessor {
            base: version(1, 2),
            successor: version(2, 3),
        };
        let mut proposal = submitted();
        assert_eq!(
            proposal.decide(
                current_head,
                QuestionChangeProposalDecision::Accept(wrong_lineage),
            ),
            Err(QuestionChangeProposalTransitionError::SuccessorChangesLineage)
        );

        let historical_successor = MintedQuestionSuccessor {
            base: version(1, 1),
            successor: version(1, 2),
        };
        let mut proposal = submitted();
        assert_eq!(
            proposal.decide(
                current_head,
                QuestionChangeProposalDecision::Accept(historical_successor),
            ),
            Err(QuestionChangeProposalTransitionError::SuccessorWitnessMismatch)
        );

        let non_advancing_successor = MintedQuestionSuccessor {
            base: version(1, 2),
            successor: version(1, 2),
        };
        let mut proposal = submitted();
        assert_eq!(
            proposal.decide(
                current_head,
                QuestionChangeProposalDecision::Accept(non_advancing_successor),
            ),
            Err(QuestionChangeProposalTransitionError::SuccessorDoesNotAdvance)
        );

        let successor = MintedQuestionSuccessor {
            base: version(1, 2),
            successor: version(1, 3),
        };
        let mut proposal = submitted();
        proposal
            .decide(
                current_head,
                QuestionChangeProposalDecision::Accept(successor),
            )
            .expect("the exact-head minted successor is accepted");
        assert_eq!(
            proposal.state(),
            QuestionChangeProposalState::Accepted {
                published_version: version(1, 3),
            }
        );
    }

    #[test]
    fn accepted_and_rejected_proposals_are_terminal() {
        let current_head = head(1, 2);
        let successor = MintedQuestionSuccessor {
            base: version(1, 2),
            successor: version(1, 3),
        };
        let mut accepted = submitted();
        accepted
            .decide(
                current_head,
                QuestionChangeProposalDecision::Accept(successor),
            )
            .expect("exact-head acceptance");
        assert!(accepted.state().is_terminal());
        assert!(matches!(
            accepted.decide(current_head, QuestionChangeProposalDecision::Reject),
            Err(QuestionChangeProposalTransitionError::TerminalState { .. })
        ));

        let mut rejected = submitted();
        rejected
            .decide(current_head, QuestionChangeProposalDecision::Reject)
            .expect("exact-head rejection");
        assert!(rejected.state().is_terminal());
        assert!(matches!(
            rejected.decide(current_head, QuestionChangeProposalDecision::Reject),
            Err(QuestionChangeProposalTransitionError::TerminalState { .. })
        ));
    }
}
