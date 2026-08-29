//! Exact answer-free source bindings for one reusable assignment definition.

use serde::{Deserialize, Serialize};

use super::ObservedBlueprintSource;
use crate::{BlueprintReference, BlueprintRevision, MAX_ASSIGNMENT_ORDERED_ENTRIES};

/// One exact assignment selected from a revision-bound BlueprintCourse location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    try_from = "ObservedBlueprintAssignmentSourceParts"
)]
pub struct AssignmentDefinitionSourceView {
    reference: BlueprintReference,
    revision: BlueprintRevision,
    module_index: u16,
    assignment_index: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct ObservedBlueprintAssignmentSourceParts {
    reference: BlueprintReference,
    revision: BlueprintRevision,
    module_index: u16,
    assignment_index: u16,
}

impl TryFrom<ObservedBlueprintAssignmentSourceParts> for AssignmentDefinitionSourceView {
    type Error = AssignmentDefinitionSourceViewError;

    fn try_from(value: ObservedBlueprintAssignmentSourceParts) -> Result<Self, Self::Error> {
        Self::new(
            ObservedBlueprintSource {
                reference: value.reference,
                revision: value.revision,
            },
            value.module_index,
            value.assignment_index,
        )
    }
}

impl AssignmentDefinitionSourceView {
    /// Binds an observed BlueprintCourse revision to one bounded assignment location.
    pub fn new(
        source: ObservedBlueprintSource,
        module_index: u16,
        assignment_index: u16,
    ) -> Result<Self, AssignmentDefinitionSourceViewError> {
        // ASVS 1.5.2, 2.2.1: deserialize through this allowlisted bounded constructor.
        let bound = u16::try_from(MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .expect("assignment source position bound fits u16");
        if module_index >= bound || assignment_index >= bound {
            return Err(AssignmentDefinitionSourceViewError);
        }
        Ok(Self {
            reference: source.reference,
            revision: source.revision,
            module_index,
            assignment_index,
        })
    }

    /// Returns the revision-bound BlueprintCourse that contains this assignment.
    pub fn source(self) -> ObservedBlueprintSource {
        ObservedBlueprintSource {
            reference: self.reference,
            revision: self.revision,
        }
    }

    /// Returns the zero-based authored module position in the observed BlueprintCourse.
    pub fn module_index(self) -> u16 {
        self.module_index
    }

    /// Returns the zero-based authored assignment position inside the selected module.
    pub fn assignment_index(self) -> u16 {
        self.assignment_index
    }
}

/// A BlueprintCourse assignment source position exceeded the reusable ordering bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentDefinitionSourceViewError;

impl std::fmt::Display for AssignmentDefinitionSourceViewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(
            "BlueprintCourse assignment source position is outside the reusable ordering bound",
        )
    }
}

impl std::error::Error for AssignmentDefinitionSourceViewError {}
