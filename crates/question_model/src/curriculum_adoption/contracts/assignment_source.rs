//! Exact answer-free source bindings for one reusable assignment definition.

use serde::{Deserialize, Serialize};

use super::ObservedBlueprintSource;
use crate::{BlueprintAssignmentId, BlueprintReference, BlueprintRevision};

/// One exact stable assignment selected from a revision-bound BlueprintCourse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    from = "ObservedBlueprintAssignmentSourceParts"
)]
pub struct AssignmentDefinitionSourceView {
    reference: BlueprintReference,
    revision: BlueprintRevision,
    assignment_id: BlueprintAssignmentId,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct ObservedBlueprintAssignmentSourceParts {
    reference: BlueprintReference,
    revision: BlueprintRevision,
    assignment_id: BlueprintAssignmentId,
}

impl From<ObservedBlueprintAssignmentSourceParts> for AssignmentDefinitionSourceView {
    fn from(value: ObservedBlueprintAssignmentSourceParts) -> Self {
        Self::new(
            ObservedBlueprintSource {
                reference: value.reference,
                revision: value.revision,
            },
            value.assignment_id,
        )
    }
}

impl AssignmentDefinitionSourceView {
    /// Binds an observed BlueprintCourse revision to one stable assignment lineage.
    pub fn new(source: ObservedBlueprintSource, assignment_id: BlueprintAssignmentId) -> Self {
        Self {
            reference: source.reference,
            revision: source.revision,
            assignment_id,
        }
    }

    /// Returns the revision-bound BlueprintCourse that contains this assignment.
    pub fn source(self) -> ObservedBlueprintSource {
        ObservedBlueprintSource {
            reference: self.reference,
            revision: self.revision,
        }
    }

    /// Returns the stable assignment identity inside this exact Blueprint revision.
    pub fn assignment_id(self) -> BlueprintAssignmentId {
        self.assignment_id
    }

    /// Returns whether both sources name the same retained assignment lineage.
    pub fn same_assignment_lineage(self, other: Self) -> bool {
        self.reference == other.reference && self.assignment_id == other.assignment_id
    }

    /// Returns whether this is a later revision of the exact same assignment lineage.
    pub fn is_strictly_newer_revision_of(self, earlier: Self) -> bool {
        self.same_assignment_lineage(earlier) && self.revision > earlier.revision
    }
}
