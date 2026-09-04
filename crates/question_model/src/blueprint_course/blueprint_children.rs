//! Stable child identity and complete-tree edit contracts for BlueprintCourses.

use std::collections::BTreeSet;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    BlueprintAssignmentContentInput, BlueprintAssignmentContentView,
    BlueprintCourseValidationError, validate_blueprint_course_title,
};
use crate::MAX_ASSIGNMENT_ORDERED_ENTRIES;

/// Opaque stable reference for one retained Blueprint Module in a Blueprint Course lineage.
///
/// It is an answer-free edit token, not a route Reference or human-facing label.
/// The server allocates it when a module first enters a BlueprintCourse and
/// validates retained ownership when the complete tree is replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintModuleReference(Uuid);

/// Opaque stable identity for one retained assignment in a BlueprintCourse lineage.
///
/// Vector position remains authored order; this identifier is the immutable
/// lineage key used by snapshots, Blueprint updates, and audit evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintAssignmentReference(Uuid);

macro_rules! impl_blueprint_child_id {
    ($name:ident) => {
        impl $name {
            /// Rebuilds an identifier read from trusted storage.
            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            /// Returns the UUID used by trusted storage and server-side auditing.
            pub fn as_uuid(self) -> Uuid {
                self.0
            }

            /// Allocates a fresh child identity in server-owned code.
            #[cfg(feature = "generate")]
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = BlueprintChildIdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let parsed = Uuid::parse_str(value).map_err(|_| BlueprintChildIdError)?;
                (parsed.to_string() == value)
                    .then_some(Self(parsed))
                    .ok_or(BlueprintChildIdError)
            }
        }

        impl TryFrom<String> for $name {
            type Error = BlueprintChildIdError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.to_string()
            }
        }
    };
}

impl_blueprint_child_id!(BlueprintModuleReference);
impl_blueprint_child_id!(BlueprintAssignmentReference);

/// A browser-supplied Blueprint child Reference was not a canonical UUID string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintChildIdError;

impl std::fmt::Display for BlueprintChildIdError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Blueprint child Reference must be a canonical UUID string")
    }
}

impl std::error::Error for BlueprintChildIdError {}

/// One labelled module in a new BlueprintCourse submitted in authored order.
///
/// Creation deliberately carries no child References. The server allocates them
/// only after it accepts the complete tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateBlueprintModuleInput {
    /// Week or module label visible to active Instructor readers.
    pub label: String,
    /// Blueprint Assignments in authored order.
    pub assignments: Vec<BlueprintAssignmentContentInput>,
}

/// Complete submitted meaning for a newly created BlueprintCourse.
///
/// Creation deliberately has no identity fields, so the browser cannot choose
/// stable module or assignment lineage identifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateBlueprintCourseContentInput {
    /// Instructor-visible course title.
    pub title: String,
    /// Ordered labelled curriculum modules.
    pub modules: Vec<CreateBlueprintModuleInput>,
}

impl CreateBlueprintCourseContentInput {
    /// Validates the complete ordered BlueprintCourse tree.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        validate_blueprint_course_title(&self.title)
            .map_err(|_| BlueprintCourseValidationError::InvalidBlueprintTitle)?;
        if self.modules.is_empty() || self.modules.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleCount);
        }
        for module in &self.modules {
            validate_blueprint_course_title(&module.label)
                .map_err(|_| BlueprintCourseValidationError::InvalidModuleLabel)?;
            if module.assignments.is_empty()
                || module.assignments.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES
            {
                return Err(BlueprintCourseValidationError::InvalidModuleAssignmentCount);
            }
            for content in &module.assignments {
                content.validate()?;
            }
        }
        Ok(())
    }
}

/// Explicit Blueprint Module Edit Choice in a complete Blueprint Course edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintModuleEditChoice {
    /// Keep this exact module lineage from the expected head revision.
    Retained {
        blueprint_module_reference: BlueprintModuleReference,
    },
    /// Add a module and let the server allocate its stable identity.
    New,
}

impl BlueprintModuleEditChoice {
    /// Returns the retained identity, if this edit preserves an existing node.
    pub fn retained_reference(self) -> Option<BlueprintModuleReference> {
        match self {
            Self::Retained {
                blueprint_module_reference,
            } => Some(blueprint_module_reference),
            Self::New => None,
        }
    }
}

/// Explicit Blueprint Assignment Edit Choice in a complete Blueprint Course edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintAssignmentEditChoice {
    /// Keep this exact assignment lineage from the expected head revision.
    Retained {
        blueprint_assignment_reference: BlueprintAssignmentReference,
    },
    /// Add an assignment and let the server allocate its stable identity.
    New,
}

impl BlueprintAssignmentEditChoice {
    /// Returns the retained Blueprint Assignment Reference, if this edit preserves the lineage.
    pub fn retained_reference(self) -> Option<BlueprintAssignmentReference> {
        match self {
            Self::Retained {
                blueprint_assignment_reference,
            } => Some(blueprint_assignment_reference),
            Self::New => None,
        }
    }
}

/// One Blueprint Assignment in a complete BlueprintCourse edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssignmentReplacementInput {
    /// Explicit retained/new Blueprint Assignment Edit Choice for this ordered node.
    pub choice: BlueprintAssignmentEditChoice,
    /// Complete assignment meaning for this revision snapshot.
    pub content: BlueprintAssignmentContentInput,
}

/// One module in a complete BlueprintCourse edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintModuleReplacementInput {
    /// Explicit retained/new Blueprint Module Edit Choice for this ordered node.
    pub choice: BlueprintModuleEditChoice,
    /// Week or module label visible to active Instructor readers.
    pub label: String,
    /// Complete Blueprint Assignments in authored order.
    pub assignments: Vec<BlueprintAssignmentReplacementInput>,
}

/// Complete submitted meaning for a replacement of one BlueprintCourse head.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReplaceBlueprintCourseContentInput {
    /// Instructor-visible course title.
    pub title: String,
    /// Ordered labelled curriculum modules for the next complete snapshot.
    pub modules: Vec<BlueprintModuleReplacementInput>,
}

impl ReplaceBlueprintCourseContentInput {
    /// Validates complete tree meaning and rejects duplicate retained References.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        validate_blueprint_course_title(&self.title)
            .map_err(|_| BlueprintCourseValidationError::InvalidBlueprintTitle)?;
        if self.modules.is_empty() || self.modules.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleCount);
        }
        let mut retained_modules = BTreeSet::new();
        let mut retained_assignments = BTreeSet::new();
        for module in &self.modules {
            if let Some(module_reference) = module.choice.retained_reference()
                && !retained_modules.insert(module_reference)
            {
                return Err(BlueprintCourseValidationError::DuplicateRetainedBlueprintModuleChoice);
            }
            validate_blueprint_course_title(&module.label)
                .map_err(|_| BlueprintCourseValidationError::InvalidModuleLabel)?;
            if module.assignments.is_empty()
                || module.assignments.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES
            {
                return Err(BlueprintCourseValidationError::InvalidModuleAssignmentCount);
            }
            for assignment in &module.assignments {
                if let Some(assignment_reference) = assignment.choice.retained_reference()
                    && !retained_assignments.insert(assignment_reference)
                {
                    return Err(
                        BlueprintCourseValidationError::DuplicateRetainedBlueprintAssignmentChoice,
                    );
                }
                assignment.content.validate()?;
            }
        }
        Ok(())
    }
}

/// One answer-free Blueprint Assignment with its stable Blueprint Assignment Reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseAssignmentContentView {
    /// Stable opaque Blueprint Assignment Reference retained by an edit of this Assignment.
    pub blueprint_assignment_reference: BlueprintAssignmentReference,
    /// Current answer-free assignment meaning.
    pub content: BlueprintAssignmentContentView,
}

/// One answer-free Blueprint Module in retained aggregate-owned order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintModuleView {
    /// Stable opaque Blueprint Module Reference retained by an edit of this module.
    pub blueprint_module_reference: BlueprintModuleReference,
    /// Week or module label visible to active Instructor readers.
    pub label: String,
    /// Blueprint Assignments in retained aggregate-owned order.
    pub assignments: Vec<BlueprintCourseAssignmentContentView>,
}
