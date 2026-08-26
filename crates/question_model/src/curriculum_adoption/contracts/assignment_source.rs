//! Exact answer-free source bindings for one reusable assignment definition.

use serde::{Deserialize, Serialize};

use super::{ObservedAlphaSource, ObservedBlueprintSource};
use crate::{AlphaCourseReference, AlphaCourseRevision, MAX_ASSIGNMENT_ORDERED_ENTRIES};

/// One observed assignment definition selected from a Blueprint or exact Alpha position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AssignmentDefinitionSourceView {
    /// The complete definition stored by one observed Blueprint revision.
    Blueprint(ObservedBlueprintSource),
    /// One exact module assignment stored by one observed Alpha revision.
    Alpha(ObservedAlphaAssignmentSource),
}

/// One revision-bound Alpha assignment located by its exact authored positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    try_from = "ObservedAlphaAssignmentSourceParts"
)]
pub struct ObservedAlphaAssignmentSource {
    reference: AlphaCourseReference,
    revision: AlphaCourseRevision,
    module_index: u16,
    assignment_index: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ObservedAlphaAssignmentSourceParts {
    reference: AlphaCourseReference,
    revision: AlphaCourseRevision,
    module_index: u16,
    assignment_index: u16,
}

impl TryFrom<ObservedAlphaAssignmentSourceParts> for ObservedAlphaAssignmentSource {
    type Error = ObservedAlphaAssignmentSourceError;

    fn try_from(value: ObservedAlphaAssignmentSourceParts) -> Result<Self, Self::Error> {
        Self::new(
            ObservedAlphaSource {
                reference: value.reference,
                revision: value.revision,
            },
            value.module_index,
            value.assignment_index,
        )
    }
}

impl ObservedAlphaAssignmentSource {
    /// Binds an observed Alpha revision to one bounded zero-based assignment location.
    pub fn new(
        source: ObservedAlphaSource,
        module_index: u16,
        assignment_index: u16,
    ) -> Result<Self, ObservedAlphaAssignmentSourceError> {
        // ASVS 1.5.2, 2.2.1: deserialize through this allowlisted bounded constructor.
        let bound = u16::try_from(MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .expect("assignment source position bound fits u16");
        if module_index >= bound || assignment_index >= bound {
            return Err(ObservedAlphaAssignmentSourceError);
        }
        Ok(Self {
            reference: source.reference,
            revision: source.revision,
            module_index,
            assignment_index,
        })
    }

    /// Returns the revision-bound whole Alpha that contains this assignment.
    pub fn source(self) -> ObservedAlphaSource {
        ObservedAlphaSource {
            reference: self.reference,
            revision: self.revision,
        }
    }

    /// Returns the zero-based authored module position in the observed Alpha.
    pub fn module_index(self) -> u16 {
        self.module_index
    }

    /// Returns the zero-based authored assignment position inside the selected module.
    pub fn assignment_index(self) -> u16 {
        self.assignment_index
    }
}

/// An Alpha assignment source position exceeded the reusable ordering bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedAlphaAssignmentSourceError;

impl std::fmt::Display for ObservedAlphaAssignmentSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("Alpha assignment source position is outside the reusable ordering bound")
    }
}

impl std::error::Error for ObservedAlphaAssignmentSourceError {}
