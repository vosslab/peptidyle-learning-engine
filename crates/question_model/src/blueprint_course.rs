//! Browser-safe reusable BlueprintCourse assessments and answer-free views.
//!
//! A reusable content has no course, student, version, or server-private
//! identity. Fixed entries carry exact public Question Revision Tuples
//! checked under destination authority before persistence. Browser views keep the same ordered
//! shape while substituting current answer-free Question Library discovery rows.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::BlueprintCourseId;

/// Shared instructor-content bound for reusable titles and module labels.
pub const MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS: usize = 200;

mod assessment_content;
mod blueprint_children;
/// Canonical reusable Blueprint exchange projection.
pub mod canonical_exchange;
mod fork_apply;
mod fork_comparison;
pub use assessment_content::{
    BlueprintAssessmentContentInput, BlueprintAssessmentContentView, BlueprintAssessmentDefaults,
    BlueprintAssessmentEntryInput, BlueprintAssessmentEntryView, ReusableFixedQuestionInput,
    ReusablePoolInput, ReusablePoolView, ReusableQuestionView, ReusableSelectionAvailability,
};
pub use blueprint_children::{
    BlueprintAssessmentEditChoice, BlueprintAssessmentId, BlueprintAssessmentReplacementInput,
    BlueprintChildIdError, BlueprintCourseAssessmentContentView, BlueprintModuleEditChoice,
    BlueprintModuleId, BlueprintModuleReplacementInput, BlueprintModuleView,
    CreateBlueprintCourseInput, CreateBlueprintFromCourseInstanceInput, CreateBlueprintModuleInput,
    ReplaceBlueprintCourseContentInput,
};
pub use fork_apply::{
    BlueprintForkApplyAssessmentCopy, BlueprintForkApplyAssessmentDestination,
    BlueprintForkApplyError, BlueprintForkApplyModuleDestination,
    BlueprintForkApplyModuleLabelCopy, BlueprintForkApplyModuleLayout, BlueprintForkApplySelection,
    apply_blueprint_fork,
};
pub use fork_comparison::{
    BlueprintAssessmentRelationship, BlueprintComparison, BlueprintComparisonAssessment,
    BlueprintComparisonError, BlueprintComparisonInventory, BlueprintComparisonModule,
    compare_blueprint_courses,
};

/// Failure to validate a reusable title or module label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintCourseTitleError {
    /// The value is blank, leading/trailing whitespace-bearing, or too long.
    Invalid,
}

impl std::fmt::Display for BlueprintCourseTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum text must be trimmed, nonempty, and within its bound")
    }
}

impl std::error::Error for BlueprintCourseTitleError {}

/// Validates a durable reusable content title or module label.
pub fn validate_blueprint_course_title(value: &str) -> Result<(), BlueprintCourseTitleError> {
    (value == value.trim()
        && !value.is_empty()
        && value.chars().count() <= MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS)
        .then_some(())
        .ok_or(BlueprintCourseTitleError::Invalid)
}

/// Strong Revision Number for one complete Blueprint Course tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintRevisionNumber(NonZeroU64);

macro_rules! impl_revision {
    ($name:ident) => {
        impl $name {
            /// Initial revision for a newly stored aggregate.
            pub const INITIAL: Self = Self(NonZeroU64::MIN);

            /// Rebuilds a positive PostgreSQL-bigint revision.
            pub fn new(value: u64) -> Option<Self> {
                (value <= i64::MAX as u64)
                    .then(|| NonZeroU64::new(value))
                    .flatten()
                    .map(Self)
            }

            /// Returns the exact positive revision scalar.
            pub fn value(self) -> u64 {
                self.0.get()
            }

            /// Advances one revision without exceeding PostgreSQL bigint.
            pub fn checked_next(self) -> Option<Self> {
                self.value().checked_add(1).and_then(Self::new)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.value())
            }
        }

        impl FromStr for $name {
            type Err = &'static str;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if value.is_empty()
                    || (value.len() > 1 && value.starts_with('0'))
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err("revision must be a canonical positive decimal string");
                }
                value
                    .parse::<u64>()
                    .ok()
                    .and_then(Self::new)
                    .ok_or("revision must fit a positive PostgreSQL bigint")
            }
        }

        impl TryFrom<String> for $name {
            type Error = &'static str;

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

impl_revision!(BlueprintRevisionNumber);

/// Closed browser-safe classification for one returned Blueprint Course view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintCourseReadAccess {
    /// The current Active Instructor Account is the exact Blueprint Course Owner.
    BlueprintCourseOwner,
    /// The current Active Instructor Account reads reusable saved Blueprint content.
    ActiveInstructor,
}

/// Safe compact current Blueprint Course Summary View.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseSummaryView {
    /// Independently editable current classification.
    pub classification: crate::CourseClassification,
    /// Blueprint Course ID resolved under current read authority.
    pub id: BlueprintCourseId,
    /// Compact stable-lineage name for constrained navigation.
    pub short_name: String,
    /// Descriptive stable-lineage name for headings and listings.
    pub long_name: String,
    /// Current independently editable Blueprint Theme.
    pub theme: crate::Theme,
    /// Current stable-lineage availability for discovery and new selection.
    pub availability: crate::BlueprintAvailability,
    /// Opaque validator for rename and availability actions.
    pub blueprint_edit_number: crate::BlueprintEditNumber,
    /// Exact current immutable reusable state.
    pub current_revision_tuple: crate::BlueprintCourseRevisionTuple,
    /// Browser-safe classification for this returned Blueprint Course view.
    pub read_access: BlueprintCourseReadAccess,
    /// Lifetime Course Instances adopted from this Blueprint lineage, across Revisions.
    pub total_adoptions: u64,
    /// Students counted once per adopted Course Instance, including ended memberships.
    pub total_students_ever_enrolled: u64,
    /// Account ID of the Blueprint Course owner.
    pub owner_account_id: crate::AccountId,
    /// Public display name of the Blueprint Course owner. This is the public author.
    pub owner_display_name: String,
    /// Institution supplied for the Blueprint Course owner's public Profile.
    pub owner_affiliation: String,
    /// Visible Instructor Stars across this Blueprint Course lineage.
    pub star_count: u64,
    /// Watch subscriptions counted without exposing watcher identities.
    pub watcher_count: u64,
    /// The most recent content or metadata edit on this Blueprint Course lineage.
    pub last_edited_at_millis: i64,
}

/// Safe current Blueprint Course View of one complete BlueprintCourse tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseView {
    /// Independently editable current classification.
    pub classification: crate::CourseClassification,
    /// Blueprint Course ID resolved for this returned read view.
    pub id: BlueprintCourseId,
    /// Compact stable-lineage name for constrained navigation.
    pub short_name: String,
    /// Descriptive stable-lineage name for headings and listings.
    pub long_name: String,
    /// Current independently editable Blueprint Theme.
    pub theme: crate::Theme,
    /// Current stable-lineage availability for discovery and new selection.
    pub availability: crate::BlueprintAvailability,
    /// Opaque validator for rename and availability actions.
    pub blueprint_edit_number: crate::BlueprintEditNumber,
    /// Exact current immutable reusable state.
    pub current_revision_tuple: crate::BlueprintCourseRevisionTuple,
    /// Browser-safe classification for this returned Blueprint Course view.
    pub read_access: BlueprintCourseReadAccess,
    /// Exact fork origin; roots and hidden sources both return null.
    pub fork_source_tuple: Option<crate::BlueprintCourseRevisionTuple>,
    /// Answer-free current Revision content.
    pub modules: Vec<BlueprintModuleView>,
}

/// Meaning-level validation failure for a Blueprint Course command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintCourseValidationError {
    /// Course hierarchy or Tags violate the current metadata contract.
    InvalidClassification,
    /// A reusable content title is not durable instructor content.
    InvalidContentTitle,
    /// A Blueprint Course lineage name is not durable instructor content.
    InvalidBlueprintName,
    /// A module label is not durable instructor content.
    InvalidModuleLabel,
    /// A reusable content has no usable entries or exceeds its shared bound.
    InvalidEntryCount,
    /// A BlueprintCourse has no usable modules or exceeds its shared bound.
    InvalidModuleCount,
    /// A module has no usable assessments or exceeds its shared bound.
    InvalidModuleAssessmentCount,
    /// A Question Pool Item list has no members or exceeds its shared bound.
    InvalidQuestionPoolItems,
    /// A pool selection count cannot select a meaningful subset of its Question Pool Items.
    InvalidPoolSelectionCount,
    /// A pool repeats a Question Pool Item and therefore changes no selectable meaning.
    DuplicateQuestionPoolItem,
    /// All Question Pool Items exceed the assessment-level shared bound.
    TooManyQuestionPoolItems,
    /// A reusable whole Assessment Attempt time limit exceeds the ordinary assessment bound.
    AssessmentAttemptTimeLimitOutOfRange,
    /// A reusable attempt limit exceeds the ordinary assessment bound.
    AttemptLimitOutOfRange,
    /// A replacement submitted the same retained Blueprint Module ID more than once.
    DuplicateRetainedBlueprintModuleChoice,
    /// A replacement submitted the same retained Blueprint Assessment ID more than once.
    DuplicateRetainedBlueprintAssessmentChoice,
}

impl std::fmt::Display for BlueprintCourseValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidContentTitle => "reusable content title is invalid",
            Self::InvalidBlueprintName => "Blueprint Course name is invalid",
            Self::InvalidClassification => "Course classification is invalid",
            Self::InvalidModuleLabel => "BlueprintCourse module label is invalid",
            Self::InvalidEntryCount => "reusable content must contain bounded ordered entries",
            Self::InvalidModuleCount => "BlueprintCourse must contain bounded modules",
            Self::InvalidModuleAssessmentCount => {
                "BlueprintCourse module must contain bounded Blueprint Assessments"
            }
            Self::InvalidQuestionPoolItems => {
                "Question Pool Items must be present and within their bound"
            }
            Self::InvalidPoolSelectionCount => {
                "Question Pool selection count must be between one and Question Pool Item count"
            }
            Self::DuplicateQuestionPoolItem => "Question Pool Items must be distinct",
            Self::TooManyQuestionPoolItems => {
                "Question Pool Items exceed the assessment-level bound"
            }
            Self::AssessmentAttemptTimeLimitOutOfRange => {
                "reusable time limit exceeds the supported range"
            }
            Self::AttemptLimitOutOfRange => "reusable attempt limit exceeds the supported range",
            Self::DuplicateRetainedBlueprintModuleChoice => {
                "Blueprint Course replacement repeats a retained Blueprint Module ID"
            }
            Self::DuplicateRetainedBlueprintAssessmentChoice => {
                "Blueprint Course replacement repeats a retained Blueprint Assessment ID"
            }
        })
    }
}

impl std::error::Error for BlueprintCourseValidationError {}
