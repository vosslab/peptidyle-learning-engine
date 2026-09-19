//! Stable child identity and complete-tree edit contracts for BlueprintCourses.

use std::collections::BTreeSet;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    BlueprintAssessmentContentInput, BlueprintAssessmentContentView,
    BlueprintCourseValidationError, validate_blueprint_course_title,
};
use crate::MAX_ASSESSMENT_ORDERED_ENTRIES;

/// Opaque stable reference for one retained Blueprint Module in a Blueprint Course lineage.
///
/// It is an answer-free edit token, not a route Reference or human-facing label.
/// The server allocates it when a module first enters a BlueprintCourse and
/// validates retained ownership when the complete tree is replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintModuleReference(Uuid);

/// Opaque stable identity for one retained assessment in a BlueprintCourse lineage.
///
/// Vector position remains authored order; this identifier is the immutable
/// lineage key used by snapshots, Blueprint updates, and audit evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintAssessmentId(Uuid);

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
impl_blueprint_child_id!(BlueprintAssessmentId);

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
    /// Blueprint Assessments in authored order.
    pub assessments: Vec<BlueprintAssessmentContentInput>,
}

/// Complete submitted meaning for a newly created Blueprint Course.
///
/// Creation carries lineage names beside the first reusable structure. It has
/// no child identity fields, so the browser cannot choose stable Module or
/// Assessment lineage identifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateBlueprintCourseInput {
    /// Explicit current Course metadata, not inferred from reusable content.
    pub classification: crate::CourseClassification,
    /// Compact Blueprint Course name used in constrained navigation.
    pub short_name: String,
    /// Descriptive Blueprint Course name used in headings and listings.
    pub long_name: String,
    /// Ordered labelled curriculum modules.
    pub modules: Vec<CreateBlueprintModuleInput>,
}

/// Instructor-chosen metadata for creating a Blueprint from one Course Instance.
///
/// Reusable content is deliberately absent: the trusted Store derives it from
/// the authorized Course snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateBlueprintFromCourseInstanceInput {
    /// Independently chosen classification for the new Blueprint Course.
    pub classification: crate::CourseClassification,
    /// Compact Blueprint Course name used in constrained navigation.
    pub short_name: String,
    /// Descriptive Blueprint Course name used in headings and listings.
    pub long_name: String,
}

impl CreateBlueprintFromCourseInstanceInput {
    /// Validates only caller-owned metadata; source content is Store-owned.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        self.classification
            .validate()
            .map_err(|_| BlueprintCourseValidationError::InvalidClassification)?;
        validate_blueprint_course_title(&self.short_name)
            .map_err(|_| BlueprintCourseValidationError::InvalidBlueprintName)?;
        validate_blueprint_course_title(&self.long_name)
            .map_err(|_| BlueprintCourseValidationError::InvalidBlueprintName)?;
        Ok(())
    }
}

impl CreateBlueprintCourseInput {
    /// Validates lineage names and the complete ordered reusable structure.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        self.classification
            .validate()
            .map_err(|_| BlueprintCourseValidationError::InvalidClassification)?;
        validate_blueprint_course_title(&self.short_name)
            .map_err(|_| BlueprintCourseValidationError::InvalidBlueprintName)?;
        validate_blueprint_course_title(&self.long_name)
            .map_err(|_| BlueprintCourseValidationError::InvalidBlueprintName)?;
        if self.modules.is_empty() || self.modules.len() > MAX_ASSESSMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleCount);
        }
        for module in &self.modules {
            validate_blueprint_course_title(&module.label)
                .map_err(|_| BlueprintCourseValidationError::InvalidModuleLabel)?;
            if module.assessments.is_empty()
                || module.assessments.len() > MAX_ASSESSMENT_ORDERED_ENTRIES
            {
                return Err(BlueprintCourseValidationError::InvalidModuleAssessmentCount);
            }
            for content in &module.assessments {
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

/// Explicit Blueprint Assessment Edit Choice in a complete Blueprint Course edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintAssessmentEditChoice {
    /// Keep this exact assessment lineage from the expected head revision.
    Retained {
        blueprint_assessment_id: BlueprintAssessmentId,
    },
    /// Add an assessment and let the server allocate its stable identity.
    New,
}

impl BlueprintAssessmentEditChoice {
    /// Returns the retained Blueprint Assessment Reference, if this edit preserves the lineage.
    pub fn retained_reference(self) -> Option<BlueprintAssessmentId> {
        match self {
            Self::Retained {
                blueprint_assessment_id,
            } => Some(blueprint_assessment_id),
            Self::New => None,
        }
    }
}

/// One Blueprint Assessment in a complete BlueprintCourse edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssessmentReplacementInput {
    /// Explicit retained/new Blueprint Assessment Edit Choice for this ordered node.
    pub choice: BlueprintAssessmentEditChoice,
    /// Complete assessment meaning for this revision snapshot.
    pub content: BlueprintAssessmentContentInput,
}

/// One module in a complete BlueprintCourse edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintModuleReplacementInput {
    /// Explicit retained/new Blueprint Module Edit Choice for this ordered node.
    pub choice: BlueprintModuleEditChoice,
    /// Week or module label visible to active Instructor readers.
    pub label: String,
    /// Complete Blueprint Assessments in authored order.
    pub assessments: Vec<BlueprintAssessmentReplacementInput>,
}

/// Complete submitted meaning for a replacement of one BlueprintCourse head.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReplaceBlueprintCourseContentInput {
    /// Ordered labelled curriculum modules for the next complete snapshot.
    pub modules: Vec<BlueprintModuleReplacementInput>,
}

impl ReplaceBlueprintCourseContentInput {
    /// Validates complete tree meaning and rejects duplicate retained References.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        if self.modules.is_empty() || self.modules.len() > MAX_ASSESSMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleCount);
        }
        let mut retained_modules = BTreeSet::new();
        let mut retained_assessments = BTreeSet::new();
        for module in &self.modules {
            if let Some(module_reference) = module.choice.retained_reference()
                && !retained_modules.insert(module_reference)
            {
                return Err(BlueprintCourseValidationError::DuplicateRetainedBlueprintModuleChoice);
            }
            validate_blueprint_course_title(&module.label)
                .map_err(|_| BlueprintCourseValidationError::InvalidModuleLabel)?;
            if module.assessments.is_empty()
                || module.assessments.len() > MAX_ASSESSMENT_ORDERED_ENTRIES
            {
                return Err(BlueprintCourseValidationError::InvalidModuleAssessmentCount);
            }
            for assessment in &module.assessments {
                if let Some(assessment_reference) = assessment.choice.retained_reference()
                    && !retained_assessments.insert(assessment_reference)
                {
                    return Err(
                        BlueprintCourseValidationError::DuplicateRetainedBlueprintAssessmentChoice,
                    );
                }
                assessment.content.validate()?;
            }
        }
        Ok(())
    }
}

/// One answer-free Blueprint Assessment with its stable Blueprint Assessment Reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseAssessmentContentView {
    /// Stable opaque Blueprint Assessment Reference retained by an edit of this Assessment.
    pub blueprint_assessment_id: BlueprintAssessmentId,
    /// Current answer-free assessment meaning.
    pub content: BlueprintAssessmentContentView,
}

/// One answer-free Blueprint Module in retained aggregate-owned order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintModuleView {
    /// Stable opaque Blueprint Module Reference retained by an edit of this module.
    pub blueprint_module_reference: BlueprintModuleReference,
    /// Week or module label visible to active Instructor readers.
    pub label: String,
    /// Blueprint Assessments in retained aggregate-owned order.
    pub assessments: Vec<BlueprintCourseAssessmentContentView>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssessmentActivityRules, AssessmentInstructions, AssessmentType,
        BlueprintAssessmentDefaults, LateWorkRule, StudentFeedbackReleaseRule,
    };

    #[test]
    fn ordinary_blueprint_creation_still_requires_authored_modules() {
        let input = CreateBlueprintCourseInput {
            classification: crate::CourseClassification {
                discipline_uuid: Uuid::from_u128(1),
                subject_uuid: None,
                topic_uuid: None,
                subtopic_uuid: None,
                tags: Vec::new(),
            },
            short_name: "Short".to_string(),
            long_name: "Long Blueprint Name".to_string(),
            modules: Vec::new(),
        };

        assert_eq!(
            input.validate(),
            Err(BlueprintCourseValidationError::InvalidModuleCount)
        );
    }

    #[test]
    fn ordinary_blueprint_creation_still_requires_assessment_entries() {
        let content = BlueprintAssessmentContentInput {
            assessment_type: AssessmentType::RegularAssignment,
            title: "Empty Assessment".to_string(),
            instructions: AssessmentInstructions::default(),
            entries: Vec::new(),
            defaults: BlueprintAssessmentDefaults {
                assessment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::Accept,
                activity_rules: AssessmentActivityRules::default(),
                student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            },
        };

        assert_eq!(
            content.validate(),
            Err(BlueprintCourseValidationError::InvalidEntryCount)
        );
    }
}
