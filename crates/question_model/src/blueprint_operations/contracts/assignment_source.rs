//! Exact answer-free source bindings for one Blueprint Assignment Content record.

use serde::{Deserialize, Serialize};

use super::BlueprintRevisionReference;
use crate::{BlueprintAssignmentId, BlueprintCourseReference, BlueprintRevision};

/// One exact Blueprint Assignment selected from a Blueprint Revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", from = "BlueprintAssignmentReferenceParts")]
pub struct BlueprintAssignmentRevisionReference {
    reference: BlueprintCourseReference,
    revision: BlueprintRevision,
    assignment_id: BlueprintAssignmentId,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct BlueprintAssignmentReferenceParts {
    reference: BlueprintCourseReference,
    revision: BlueprintRevision,
    assignment_id: BlueprintAssignmentId,
}

impl From<BlueprintAssignmentReferenceParts> for BlueprintAssignmentRevisionReference {
    fn from(value: BlueprintAssignmentReferenceParts) -> Self {
        Self::new(
            BlueprintRevisionReference {
                reference: value.reference,
                revision: value.revision,
            },
            value.assignment_id,
        )
    }
}

impl BlueprintAssignmentRevisionReference {
    /// Binds a Blueprint Revision to one stable Blueprint Assignment lineage.
    pub fn new(source: BlueprintRevisionReference, assignment_id: BlueprintAssignmentId) -> Self {
        Self {
            reference: source.reference,
            revision: source.revision,
            assignment_id,
        }
    }

    /// Returns the Blueprint Revision that contains this assignment.
    pub fn source(self) -> BlueprintRevisionReference {
        BlueprintRevisionReference {
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
