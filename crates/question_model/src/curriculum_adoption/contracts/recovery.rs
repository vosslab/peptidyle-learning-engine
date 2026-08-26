//! Closed recovery decisions for imports whose current state cannot be overwritten.

use serde::{Deserialize, Serialize};

use super::{
    AssignmentDefinitionSourceView, CourseScheduleWitness, CurriculumImportRevision,
    ReplacementQuestionChoices,
};
use crate::CourseReference;

/// Explicit recovery that preserves an assignment whose reusable meaning or evidence is fixed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PreservedAssignmentRecoveryAction {
    /// Preserve the divergent assignment and create a new source-derived draft.
    CreateSourceDerivedAssignment,
}

/// Explicit replacement action for one source pin unavailable to the destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum UnavailablePinRecoveryAction {
    /// Choose one public replacement question for a pin that cannot be reauthorized.
    SelectReplacementQuestion {
        /// Bounded reusable source position containing the unavailable pin.
        position: super::CurriculumPinPosition,
        /// Public catalog question IDs suitable for the explicit replacement flow.
        candidates: ReplacementQuestionChoices,
    },
}

/// Structured outcome of an assignment fast-forward preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AssignmentFastForwardDecision {
    /// All source, baseline, issued-work, and exact-pin checks permit a fast-forward.
    Eligible,
    /// Destination reusable meaning changed; preserve it and create a separate draft.
    Divergent {
        /// Preserve the current assignment and create an independent source-derived draft.
        recovery: PreservedAssignmentRecoveryAction,
    },
    /// A required source pin cannot be reauthorized for new destination use.
    UnavailablePin {
        /// Choose an authorized public replacement for the exact unavailable position.
        recovery: UnavailablePinRecoveryAction,
    },
    /// The source changed or the observed source revision no longer matches.
    SourceRevisionDrift {
        /// Current exact assignment-definition source returned for a corrected preview.
        source: AssignmentDefinitionSourceView,
    },
    /// Learner work was issued, so the existing assignment retains its immutable evidence context.
    IssuedWork {
        /// Preserve the issued assignment and create an independent source-derived draft.
        recovery: PreservedAssignmentRecoveryAction,
    },
}

/// Fast-forward preview with one structured, recoverable decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentFastForwardPreviewView {
    /// Destination course.
    pub course: CourseReference,
    /// Destination assignment and observed definition revision.
    pub assignment: super::ObservedAssignmentRevision,
    /// Import baseline revision.
    pub import_revision: CurriculumImportRevision,
    /// Source selected for re-read and comparison.
    pub source: AssignmentDefinitionSourceView,
    /// Schedule/assignment witness preserved through an eligible apply.
    pub witness: CourseScheduleWitness,
    /// Explicit result and available recovery action.
    pub decision: AssignmentFastForwardDecision,
}
