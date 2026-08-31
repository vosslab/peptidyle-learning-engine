//! Stable child identity and complete-tree edit contracts for BlueprintCourses.

use std::collections::BTreeSet;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    BlueprintCourseValidationError, ReusableAssignmentDefinitionInput,
    ReusableAssignmentDefinitionView, validate_blueprint_course_title,
};
use crate::MAX_ASSIGNMENT_ORDERED_ENTRIES;

/// Opaque stable identity for one retained module in a BlueprintCourse lineage.
///
/// It is an answer-free edit token, not a route locator or human-facing label.
/// The server allocates it when a module first enters a BlueprintCourse and
/// validates retained ownership when the complete tree is replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintModuleId(Uuid);

/// Opaque stable identity for one retained assignment in a BlueprintCourse lineage.
///
/// Vector position remains authored order; this identifier is the immutable
/// lineage key used by snapshots, controlled updates, and audit evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintAssignmentId(Uuid);

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

impl_blueprint_child_id!(BlueprintModuleId);
impl_blueprint_child_id!(BlueprintAssignmentId);

/// A browser-supplied Blueprint child handle was not a canonical UUID string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintChildIdError;

impl std::fmt::Display for BlueprintChildIdError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Blueprint child ID must be a canonical UUID string")
    }
}

impl std::error::Error for BlueprintChildIdError {}

/// One labelled module in a new BlueprintCourse submitted in authored order.
///
/// Creation deliberately carries no child handles. The server allocates them
/// only after it accepts the complete tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateBlueprintCourseModuleInput {
    /// Week or module label visible to approved Instructor readers.
    pub label: String,
    /// Reusable definitions in authored order.
    pub definitions: Vec<ReusableAssignmentDefinitionInput>,
}

/// Complete submitted meaning for a newly created BlueprintCourse.
///
/// Creation deliberately has no identity fields, so the browser cannot choose
/// stable module or assignment lineage identifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateBlueprintCourseDefinitionInput {
    /// Instructor-visible course title.
    pub title: String,
    /// Ordered labelled curriculum modules.
    pub modules: Vec<CreateBlueprintCourseModuleInput>,
}

impl CreateBlueprintCourseDefinitionInput {
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
            if module.definitions.is_empty()
                || module.definitions.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES
            {
                return Err(BlueprintCourseValidationError::InvalidModuleDefinitionCount);
            }
            for definition in &module.definitions {
                definition.validate()?;
            }
        }
        Ok(())
    }
}

/// Explicit identity choice for one module in a complete BlueprintCourse edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintModuleEditHandle {
    /// Keep this exact module lineage from the expected head revision.
    Retained { module_id: BlueprintModuleId },
    /// Add a module and let the server allocate its stable identity.
    New,
}

impl BlueprintModuleEditHandle {
    /// Returns the retained identity, if this edit preserves an existing node.
    pub fn retained_id(self) -> Option<BlueprintModuleId> {
        match self {
            Self::Retained { module_id } => Some(module_id),
            Self::New => None,
        }
    }
}

/// Explicit identity choice for one assignment in a complete BlueprintCourse edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintAssignmentEditHandle {
    /// Keep this exact assignment lineage from the expected head revision.
    Retained {
        assignment_id: BlueprintAssignmentId,
    },
    /// Add an assignment and let the server allocate its stable identity.
    New,
}

impl BlueprintAssignmentEditHandle {
    /// Returns the retained identity, if this edit preserves an existing node.
    pub fn retained_id(self) -> Option<BlueprintAssignmentId> {
        match self {
            Self::Retained { assignment_id } => Some(assignment_id),
            Self::New => None,
        }
    }
}

/// One reusable assignment in a complete BlueprintCourse edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseAssignmentReplacementInput {
    /// Explicit retained/new identity choice for this ordered assignment node.
    pub handle: BlueprintAssignmentEditHandle,
    /// Complete assignment meaning for this revision snapshot.
    pub definition: ReusableAssignmentDefinitionInput,
}

/// One module in a complete BlueprintCourse edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseModuleReplacementInput {
    /// Explicit retained/new identity choice for this ordered module node.
    pub handle: BlueprintModuleEditHandle,
    /// Week or module label visible to approved Instructor readers.
    pub label: String,
    /// Complete reusable definitions in authored order.
    pub definitions: Vec<BlueprintCourseAssignmentReplacementInput>,
}

/// Complete submitted meaning for a replacement of one BlueprintCourse head.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReplaceBlueprintCourseDefinitionInput {
    /// Instructor-visible course title.
    pub title: String,
    /// Ordered labelled curriculum modules for the next complete snapshot.
    pub modules: Vec<BlueprintCourseModuleReplacementInput>,
}

impl ReplaceBlueprintCourseDefinitionInput {
    /// Validates complete tree meaning and rejects duplicate retained handles.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        validate_blueprint_course_title(&self.title)
            .map_err(|_| BlueprintCourseValidationError::InvalidBlueprintTitle)?;
        if self.modules.is_empty() || self.modules.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleCount);
        }
        let mut retained_modules = BTreeSet::new();
        let mut retained_assignments = BTreeSet::new();
        for module in &self.modules {
            if let Some(module_id) = module.handle.retained_id()
                && !retained_modules.insert(module_id)
            {
                return Err(BlueprintCourseValidationError::DuplicateRetainedModuleHandle);
            }
            validate_blueprint_course_title(&module.label)
                .map_err(|_| BlueprintCourseValidationError::InvalidModuleLabel)?;
            if module.definitions.is_empty()
                || module.definitions.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES
            {
                return Err(BlueprintCourseValidationError::InvalidModuleDefinitionCount);
            }
            for assignment in &module.definitions {
                if let Some(assignment_id) = assignment.handle.retained_id()
                    && !retained_assignments.insert(assignment_id)
                {
                    return Err(BlueprintCourseValidationError::DuplicateRetainedAssignmentHandle);
                }
                assignment.definition.validate()?;
            }
        }
        Ok(())
    }
}

/// One answer-free reusable assignment with its stable BlueprintCourse handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseAssignmentDefinitionView {
    /// Stable opaque handle retained by an edit of this assignment.
    pub assignment_id: BlueprintAssignmentId,
    /// Current answer-free assignment meaning.
    pub definition: ReusableAssignmentDefinitionView,
}

/// One answer-free BlueprintCourse module in retained aggregate-owned order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseModuleView {
    /// Stable opaque handle retained by an edit of this module.
    pub module_id: BlueprintModuleId,
    /// Week or module label visible to approved Instructor readers.
    pub label: String,
    /// Reusable definitions in retained aggregate-owned order.
    pub definitions: Vec<BlueprintCourseAssignmentDefinitionView>,
}
