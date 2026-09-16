//! Browser-safe reusable BlueprintCourse assessments and answer-free views.
//!
//! A reusable content has no course, student, version, or server-private
//! identity. Fixed entries carry exact public Question Revision references
//! checked under destination authority before persistence. Browser views keep the same ordered
//! shape while substituting current answer-free Question Library discovery rows.

use std::collections::BTreeSet;
use std::num::{NonZeroU32, NonZeroU64};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{
    AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
    AssessmentPointValue, BlueprintCourseReference, LateWorkRule, MAX_ASSESSMENT_ATTEMPT_LIMIT,
    MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS, MAX_ASSESSMENT_ORDERED_ENTRIES,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionId, QuestionPoolRevisionReference,
    QuestionPoolSelectionRule, QuestionRevisionReference, QuestionSearchResult,
    StudentFeedbackReleaseRule,
};

/// Shared instructor-content bound for reusable titles and module labels.
pub const MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS: usize = 200;

mod blueprint_children;
/// Canonical reusable Blueprint exchange projection.
pub mod canonical_exchange;
mod fork_apply;
mod fork_comparison;
pub use blueprint_children::{
    BlueprintAssessmentEditChoice, BlueprintAssessmentReference,
    BlueprintAssessmentReplacementInput, BlueprintChildIdError,
    BlueprintCourseAssessmentContentView, BlueprintModuleEditChoice, BlueprintModuleReference,
    BlueprintModuleReplacementInput, BlueprintModuleView, CreateBlueprintCourseInput,
    CreateBlueprintModuleInput, ReplaceBlueprintCourseContentInput,
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

/// Blueprint Assessment policy defaults copied into a future teaching course.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssessmentDefaults {
    /// Whole Assessment Attempt time limit, if the reusable content establishes one.
    pub assessment_attempt_time_limit_seconds: Option<std::num::NonZeroU32>,
    /// Number of Assessment Attempts, if the reusable content establishes one.
    #[serde(rename = "assessment_attempt_limit")]
    pub attempt_limit: Option<std::num::NonZeroU32>,
    /// Late-work treatment copied into the future assessment policy.
    pub late_work_rule: LateWorkRule,
    /// Independent Assessment Attempt behavior copied into the future assessment policy.
    pub activity_rules: AssessmentActivityRules,
    /// Student-release policy copied into the future assessment policy.
    #[serde(rename = "student_feedback_release_rule")]
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
}

impl BlueprintAssessmentDefaults {
    /// Validates reusable limits against the ordinary teaching-policy bounds.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        if self
            .assessment_attempt_time_limit_seconds
            .is_some_and(|limit| limit.get() > MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS)
        {
            return Err(BlueprintCourseValidationError::AssessmentAttemptTimeLimitOutOfRange);
        }
        if self
            .attempt_limit
            .is_some_and(|limit| limit.get() > MAX_ASSESSMENT_ATTEMPT_LIMIT)
        {
            return Err(BlueprintCourseValidationError::AttemptLimitOutOfRange);
        }
        Ok(())
    }
}

/// One Fixed Question Assessment Entry submitted in authored order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableFixedQuestionInput {
    /// Exact published Question Revision checked under destination authority.
    pub published_question: QuestionRevisionReference,
    /// Points copied into the future Fixed Question Assessment Entry.
    pub points_possible: AssessmentPointValue,
    /// Score treatment copied into the future Fixed Question Assessment Entry.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Question Attempt retry bound copied into the future Fixed Question Assessment Entry.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Question Attempt timing copied into the future Fixed Question Assessment Entry.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// Explicit source import or retained Assessment-owned Pool edit.
/// ASVS 1.5.2: only the reviewed input alternatives and fields are accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintPoolInputChoice {
    /// Copy the exact source Revision into a fresh Assessment-owned Pool.
    Import {
        /// Exact immutable source Pool Revision; never resolved to latest.
        #[serde(rename = "questionPoolRevision")]
        question_pool_revision: QuestionPoolRevisionReference,
    },
    /// Retain a Pool already owned by the Assessment being replaced.
    Retained {
        /// Exact prior Pool Revision checked under destination ownership by the Store.
        #[serde(rename = "questionPoolRevision")]
        question_pool_revision: QuestionPoolRevisionReference,
        /// Null preserves members; an ordered list authors a new immutable Pool Revision.
        #[serde(deserialize_with = "deserialize_blueprint_pool_members")]
        members: Option<Vec<QuestionRevisionReference>>,
        /// Explicit interchangeability review for newly submitted member content.
        #[serde(rename = "interchangeabilityAttested")]
        interchangeability_attested: bool,
    },
}

fn deserialize_blueprint_pool_members<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<QuestionRevisionReference>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<Vec<QuestionRevisionReference>>::deserialize(deserializer)
}

/// One Question Pool Assessment Entry with an explicit ownership operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolInput {
    /// Exact source import or explicit retained Assessment-owned Pool edit.
    pub pool: BlueprintPoolInputChoice,
    /// Positive number of Pool members selected for each future Assessment Attempt.
    pub selection_count: NonZeroU32,
    /// Points copied for every selected Question Pool Item.
    pub points_per_item: AssessmentPointValue,
    /// Scoring rule copied for every selected Question Pool Item.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Complete reviewed selection behavior.
    pub selection_rule: QuestionPoolSelectionRule,
    /// Uniform Question Attempt retry bound copied for every selected Question Pool Item.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Uniform Question Attempt timing copied for every selected Question Pool Item.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

impl ReusablePoolInput {
    fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        if let BlueprintPoolInputChoice::Retained {
            members: Some(members),
            interchangeability_attested,
            ..
        } = &self.pool
        {
            // ASVS 2.2.1, 2.2.3: bound related authored members and selection together.
            if members.is_empty()
                || members.len() > crate::MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
            {
                return Err(BlueprintCourseValidationError::InvalidQuestionPoolItems);
            }
            if self.selection_count.get() as usize > members.len() {
                return Err(BlueprintCourseValidationError::InvalidPoolSelectionCount);
            }
            let mut questions = BTreeSet::new();
            if members
                .iter()
                .any(|member| !questions.insert(member.question_id.clone()))
            {
                return Err(BlueprintCourseValidationError::DuplicateQuestionPoolItem);
            }
            if !interchangeability_attested {
                return Err(
                    BlueprintCourseValidationError::QuestionPoolInterchangeabilityNotAttested,
                );
            }
        }
        Ok(())
    }
}

/// One ordered reusable content entry. Vector order is the only position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintAssessmentEntryInput {
    /// One Fixed Question Assessment Entry in content order.
    Fixed(ReusableFixedQuestionInput),
    /// One Question Pool Assessment Entry in content order.
    Pool(ReusablePoolInput),
}

/// Complete submitted Blueprint Assessment meaning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssessmentContentInput {
    /// Fixed pedagogical purpose shared with Course Instance Assessments.
    pub assessment_type: crate::AssessmentType,
    /// Instructor-facing title copied into future assessment assessments.
    pub title: String,
    /// Student-facing instructions copied into future assessment assessments.
    pub instructions: AssessmentInstructions,
    /// Fixed Question Assessment Entries and Question Pool Assessment Entries in authored order.
    pub entries: Vec<BlueprintAssessmentEntryInput>,
    /// Reusable delivery and Assessment Attempt defaults.
    pub defaults: BlueprintAssessmentDefaults,
}

impl BlueprintAssessmentContentInput {
    /// Validates bounded ordered entries and their reusable assessment meaning.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        validate_blueprint_course_title(&self.title)
            .map_err(|_| BlueprintCourseValidationError::InvalidContentTitle)?;
        if self.entries.is_empty() || self.entries.len() > MAX_ASSESSMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidEntryCount);
        }
        self.defaults.validate()?;
        for entry in &self.entries {
            if let BlueprintAssessmentEntryInput::Pool(pool) = entry {
                pool.validate()?;
            }
        }
        Ok(())
    }
}

/// Current selection status for an exact retained reusable question member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReusableSelectionAvailability {
    /// The current publication remains selectable for a new content.
    Available,
    /// The pinned member remains inspectable but cannot be selected anew.
    Retained,
}

/// Answer-free view of one exact published Question Revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableQuestionView {
    /// Exact stored published Question Revision; never inferred from a library head.
    pub reference: QuestionRevisionReference,
    /// Public Question Library metadata and disclosed evidence for the stored Revision.
    pub question_library: QuestionSearchResult,
    /// Whether the stored exact member remains selectable for a new copy.
    pub selection_availability: ReusableSelectionAvailability,
}

/// Current answer-free Reusable Pool View.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolView {
    /// Exact immutable Revision of the reusable Pool source.
    pub question_pool_revision: QuestionPoolRevisionReference,
    /// Positive number of Pool members selected for each future Assessment Attempt.
    pub selection_count: NonZeroU32,
    /// Points copied for every selected Question Pool Item.
    pub points_per_item: AssessmentPointValue,
    /// Scoring rule copied for every selected Question Pool Item.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Complete reviewed selection behavior.
    pub selection_rule: QuestionPoolSelectionRule,
    /// Uniform Question Attempt retry bound copied for every selected Question Pool Item.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Uniform Question Attempt timing copied for every selected Question Pool Item.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// Current answer-free reusable-content entry. Vector order is its position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintAssessmentEntryView {
    /// One Fixed Question Assessment Entry in content order.
    Fixed {
        /// Current answer-free Reusable Question View.
        question: Box<ReusableQuestionView>,
        /// Points copied into the future Fixed Question Assessment Entry.
        points_possible: AssessmentPointValue,
        /// Score treatment copied into the future Fixed Question Assessment Entry.
        scoring_rule: AssessmentEntryScoringRule,
        /// Question Attempt retry bound copied into the future Fixed Question Assessment Entry.
        question_attempt_limit: QuestionAttemptLimit,
        /// Question Attempt timing copied into the future Fixed Question Assessment Entry.
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// One Question Pool Assessment Entry in content order.
    Pool(ReusablePoolView),
}

/// Current answer-free Blueprint Assessment content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssessmentContentView {
    /// Fixed pedagogical purpose shared with Course Instance Assessments.
    pub assessment_type: crate::AssessmentType,
    /// Instructor-facing title copied into future assessment assessments.
    pub title: String,
    /// Student-facing instructions copied into future assessment assessments.
    pub instructions: AssessmentInstructions,
    /// Fixed Question Assessment Entries and Question Pool Assessment Entries in retained authored order.
    pub entries: Vec<BlueprintAssessmentEntryView>,
    /// Reusable delivery and Assessment Attempt defaults.
    pub defaults: BlueprintAssessmentDefaults,
}

/// Strong revision evidence for one complete BlueprintCourse tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintRevision(NonZeroU64);

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

impl_revision!(BlueprintRevision);

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
    /// Blueprint Course Reference resolved under current read authority.
    pub reference: BlueprintCourseReference,
    /// Compact stable-lineage name for constrained navigation.
    pub short_name: String,
    /// Descriptive stable-lineage name for headings and listings.
    pub long_name: String,
    /// Current stable-lineage availability for discovery and new selection.
    pub availability: crate::BlueprintAvailability,
    /// Opaque validator for rename and availability actions.
    pub metadata_etag: crate::BlueprintMetadataEtag,
    /// Exact current immutable reusable state.
    pub current_revision: crate::BlueprintRevisionReference,
    /// Browser-safe classification for this returned Blueprint Course view.
    pub read_access: BlueprintCourseReadAccess,
    /// Lifetime Course Instances adopted from this Blueprint lineage, across Revisions.
    pub total_adoptions: u64,
    /// Students counted once per adopted Course Instance, including ended memberships.
    pub total_students_ever_enrolled: u64,
}

/// Safe current Blueprint Course View of one complete BlueprintCourse tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseView {
    /// Blueprint Course Reference resolved for this returned read view.
    pub reference: BlueprintCourseReference,
    /// Compact stable-lineage name for constrained navigation.
    pub short_name: String,
    /// Descriptive stable-lineage name for headings and listings.
    pub long_name: String,
    /// Current stable-lineage availability for discovery and new selection.
    pub availability: crate::BlueprintAvailability,
    /// Opaque validator for rename and availability actions.
    pub metadata_etag: crate::BlueprintMetadataEtag,
    /// Exact current immutable reusable state.
    pub current_revision: crate::BlueprintRevisionReference,
    /// Browser-safe classification for this returned Blueprint Course view.
    pub read_access: BlueprintCourseReadAccess,
    /// Exact fork origin; roots and hidden sources both return null.
    pub fork_source: Option<crate::BlueprintRevisionReference>,
    /// Answer-free current Revision content.
    pub modules: Vec<BlueprintModuleView>,
}

/// Meaning-level validation failure for a Blueprint Course command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintCourseValidationError {
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
    /// Newly authored Pool members lack explicit interchangeability review.
    QuestionPoolInterchangeabilityNotAttested,
    /// All Question Pool Items exceed the assessment-level shared bound.
    TooManyQuestionPoolItems,
    /// A reusable whole Assessment Attempt time limit exceeds the ordinary assessment bound.
    AssessmentAttemptTimeLimitOutOfRange,
    /// A reusable attempt limit exceeds the ordinary assessment bound.
    AttemptLimitOutOfRange,
    /// A replacement submitted the same retained Blueprint Module Reference more than once.
    DuplicateRetainedBlueprintModuleChoice,
    /// A replacement submitted the same retained Blueprint Assessment Reference more than once.
    DuplicateRetainedBlueprintAssessmentChoice,
}

impl std::fmt::Display for BlueprintCourseValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidContentTitle => "reusable content title is invalid",
            Self::InvalidBlueprintName => "Blueprint Course name is invalid",
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
            Self::QuestionPoolInterchangeabilityNotAttested => {
                "new Question Pool members require interchangeability attestation"
            }
            Self::TooManyQuestionPoolItems => {
                "Question Pool Items exceed the assessment-level bound"
            }
            Self::AssessmentAttemptTimeLimitOutOfRange => {
                "reusable time limit exceeds the supported range"
            }
            Self::AttemptLimitOutOfRange => "reusable attempt limit exceeds the supported range",
            Self::DuplicateRetainedBlueprintModuleChoice => {
                "Blueprint Course replacement repeats a retained Blueprint Module Reference"
            }
            Self::DuplicateRetainedBlueprintAssessmentChoice => {
                "Blueprint Course replacement repeats a retained Blueprint Assessment Reference"
            }
        })
    }
}

impl std::error::Error for BlueprintCourseValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QuestionLicense;
    use crate::{
        QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionAvailability,
        QuestionBackend, QuestionBackendCapabilities, QuestionFormat, QuestionMetadata,
        QuestionPoolRevisionNumber, QuestionRevisionNumber, QuestionRevisionReference,
        QuestionStatistics, QuestionSummary, QuestionType, Timestamp,
    };
    use uuid::Uuid;

    fn blueprint_module_reference() -> BlueprintModuleReference {
        BlueprintModuleReference::from_uuid(Uuid::from_u128(1))
    }

    fn blueprint_assessment_reference() -> BlueprintAssessmentReference {
        BlueprintAssessmentReference::from_uuid(Uuid::from_u128(2))
    }

    fn question_id() -> QuestionId {
        "7K3M-X9QX".parse().expect("valid question ID")
    }

    fn defaults() -> BlueprintAssessmentDefaults {
        BlueprintAssessmentDefaults {
            assessment_attempt_time_limit_seconds: None,
            attempt_limit: None,
            late_work_rule: LateWorkRule::Accept,
            activity_rules: AssessmentActivityRules {
                assessment_attempt_grade_rule: crate::AssessmentAttemptGradeRule::Highest,
                question_variation_rule: crate::AssessmentQuestionVariationRule::NewVariation,
                ..AssessmentActivityRules::default()
            },
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
        }
    }

    fn input() -> BlueprintAssessmentContentInput {
        BlueprintAssessmentContentInput {
            assessment_type: crate::AssessmentType::PracticeQuestionAssignment,
            title: "Protein structure practice".to_string(),
            instructions: AssessmentInstructions::try_new("Explain each choice.".to_string())
                .expect("valid instructions"),
            entries: vec![
                BlueprintAssessmentEntryInput::Fixed(ReusableFixedQuestionInput {
                    published_question: QuestionRevisionReference {
                        question_id: question_id(),
                        revision_number: QuestionRevisionNumber::new(1).expect("positive Revision"),
                    },
                    points_possible: AssessmentPointValue::from_whole(3),
                    scoring_rule: AssessmentEntryScoringRule::Normal,
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                }),
                BlueprintAssessmentEntryInput::Pool(ReusablePoolInput {
                    pool: BlueprintPoolInputChoice::Import {
                        question_pool_revision: QuestionPoolRevisionReference {
                            question_pool_id: "12A4-XBCZ".parse().expect("valid Pool ID"),
                            revision_number: QuestionPoolRevisionNumber::new(1)
                                .expect("valid Pool Revision"),
                        },
                    },
                    selection_count: NonZeroU32::new(1).expect("positive count"),
                    points_per_item: AssessmentPointValue::from_whole(2),
                    scoring_rule: AssessmentEntryScoringRule::Normal,
                    selection_rule: QuestionPoolSelectionRule {
                        selected_question_order:
                            crate::QuestionPoolSelectedQuestionOrder::RandomOrder,
                    },
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                }),
            ],
            defaults: defaults(),
        }
    }

    fn discovery() -> QuestionSearchResult {
        QuestionSearchResult {
            summary: QuestionSummary {
                question_id: question_id(),
                latest_question_revision: QuestionRevisionReference {
                    question_id: question_id(),
                    revision_number: QuestionRevisionNumber::new(1).expect("positive version"),
                },
                backend: QuestionBackend::Ple,
                question_format: QuestionFormat::PleQuestionJson,
                question_type: QuestionType::MultipleChoice,
                capabilities: QuestionBackendCapabilities::none(),
                metadata: QuestionMetadata {
                    question_title: "Safe Question Library row".to_string(),
                    question_description: "Instructor-facing Question Library row fixture."
                        .to_string(),
                    tags: Vec::new(),
                    question_license: Some(QuestionLicense::Cc0_1_0),
                    question_citation: None,
                    language: "en".to_string(),
                },
                authorship: QuestionAuthorship::new(vec![QuestionAuthor {
                    display_name: QuestionAuthorDisplayName::new("Ada Lovelace".to_string())
                        .expect("valid Question Author"),
                }])
                .expect("valid Question Authorship"),
                availability: QuestionAvailability::Available,
                published_at: Timestamp::from_unix_millis(0),
            },
            evidence: QuestionStatistics::Unavailable,
        }
    }

    #[test]
    fn curriculum_references_round_trip_as_compact_wire_values() {
        let blueprint: BlueprintCourseReference = "BP7K3M2Q".parse().expect("valid reference");
        assert_eq!(
            serde_json::to_value(blueprint).expect("serializes"),
            "BP7K3M2Q"
        );
        assert!("BP7K3M2I".parse::<BlueprintCourseReference>().is_err());
        assert!("AC7K3M2Q".parse::<BlueprintCourseReference>().is_err());
    }

    #[test]
    fn ordered_content_validation_uses_vector_order_and_pool_meaning() {
        let mut content = input();
        content.defaults.attempt_limit = NonZeroU32::new(3);
        assert!(content.validate().is_ok());
        let wire = serde_json::to_value(&content).expect("content serializes");
        assert_eq!(wire["entries"][0]["kind"], "fixed");
        assert_eq!(
            wire["entries"][0]["published_question"]["questionId"],
            "7K3M-X9QX"
        );
        assert_eq!(
            wire["entries"][0]["published_question"]["revisionNumber"],
            1
        );
        assert_eq!(wire["entries"][0]["points_possible"], "3");
        assert_eq!(wire["entries"][1]["kind"], "pool");
        assert_eq!(wire["entries"][1]["pool"]["kind"], "import");
        assert_eq!(wire["entries"][1]["points_per_item"], "2");
        assert!(wire["defaults"].is_object());
        assert_eq!(wire["defaults"]["assessment_attempt_limit"], 3);
        assert!(wire["defaults"].get("attempt_limit").is_none());
        assert!(wire.get("schedule").is_none());
        assert_eq!(
            serde_json::from_value::<BlueprintAssessmentContentInput>(wire)
                .expect("content round trips"),
            content
        );
        let mut legacy_wire = serde_json::to_value(&content).expect("content serializes");
        let defaults = legacy_wire["defaults"]
            .as_object_mut()
            .expect("defaults is an object");
        let attempt_limit = defaults
            .remove("assessment_attempt_limit")
            .expect("canonical key is present");
        defaults.insert("attempt_limit".to_string(), attempt_limit);
        assert!(serde_json::from_value::<BlueprintAssessmentContentInput>(legacy_wire).is_err());
        assert!(content.validate().is_ok());
        let blueprint = CreateBlueprintCourseInput {
            short_name: "Biochemistry".to_string(),
            long_name: "Biochemistry Blueprint".to_string(),
            modules: vec![CreateBlueprintModuleInput {
                label: "Week 1".to_string(),
                assessments: vec![input()],
            }],
        };
        assert!(blueprint.validate().is_ok());
    }

    #[test]
    fn blueprint_course_view_serializes_answer_free_question_library_rows_and_edit_choices() {
        let view = BlueprintCourseView {
            reference: "BP7K3M2Q".parse().expect("valid reference"),
            short_name: "Biochemistry".to_string(),
            long_name: "Biochemistry Blueprint".to_string(),
            availability: crate::BlueprintAvailability::Public,
            metadata_etag: crate::BlueprintMetadataEtag::from_uuid(uuid::Uuid::from_u128(42)),
            current_revision: crate::BlueprintRevisionReference {
                reference: "BP7K3M2Q".parse().expect("valid reference"),
                revision: BlueprintRevision::INITIAL,
            },
            read_access: BlueprintCourseReadAccess::ActiveInstructor,
            fork_source: None,
            modules: vec![BlueprintModuleView {
                blueprint_module_reference: blueprint_module_reference(),
                label: "Week 1".to_string(),
                assessments: vec![BlueprintCourseAssessmentContentView {
                    blueprint_assessment_reference: blueprint_assessment_reference(),
                    content: BlueprintAssessmentContentView {
                        assessment_type: crate::AssessmentType::PracticeQuestionAssignment,
                        title: "Protein structure practice".to_string(),
                        instructions: AssessmentInstructions::default(),
                        entries: vec![
                            BlueprintAssessmentEntryView::Fixed {
                                question: ReusableQuestionView {
                                    reference: discovery().summary.latest_question_revision,
                                    question_library: discovery(),
                                    selection_availability:
                                        ReusableSelectionAvailability::Available,
                                }
                                .into(),
                                points_possible: AssessmentPointValue::from_whole(3),
                                scoring_rule: AssessmentEntryScoringRule::Normal,
                                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                            },
                            BlueprintAssessmentEntryView::Pool(ReusablePoolView {
                                question_pool_revision: QuestionPoolRevisionReference {
                                    question_pool_id: "12A4-XBCZ".parse().expect("Pool ID"),
                                    revision_number: QuestionPoolRevisionNumber::new(1)
                                        .expect("Pool Revision"),
                                },
                                selection_count: NonZeroU32::new(1).expect("positive count"),
                                points_per_item: AssessmentPointValue::from_whole(2),
                                scoring_rule: AssessmentEntryScoringRule::Normal,
                                selection_rule: QuestionPoolSelectionRule {
                                    selected_question_order:
                                        crate::QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                                },
                                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                            }),
                        ],
                        defaults: defaults(),
                    },
                }],
            }],
        };
        let wire = serde_json::to_value(view).expect("safe view serializes");
        assert_eq!(wire["reference"], "BP7K3M2Q");
        assert_eq!(wire["current_revision"]["revision"], "1");
        assert_eq!(
            wire["modules"][0]["assessments"][0]["content"]["entries"][0]["kind"],
            "fixed"
        );
        assert!(
            wire.pointer("/modules/0/assessments/0/content/entries/0/question/question_library")
                .is_some()
        );
        assert_eq!(
            wire.pointer("/modules/0/assessments/0/blueprint_assessment_reference"),
            Some(&serde_json::Value::String(
                blueprint_assessment_reference().to_string(),
            ))
        );
        assert!(
            wire.pointer("/modules/0/assessments/0/content/entries/0/revision")
                .is_none()
        );
        assert_eq!(
            wire["modules"][0]["assessments"][0]["content"]["entries"][1]["kind"],
            "pool"
        );
        assert_eq!(
            wire.pointer(
                "/modules/0/assessments/0/content/entries/1/question_pool_revision/questionPoolId"
            ),
            Some(&serde_json::Value::String("12A4-XBCZ".to_string()))
        );
        assert_eq!(
            wire.pointer(
                "/modules/0/assessments/0/content/entries/1/question_pool_revision/revisionNumber"
            ),
            Some(&serde_json::Value::Number(1.into()))
        );
        assert!(
            wire.pointer("/modules/0/assessments/0/content/entries/1/items")
                .is_none()
        );
    }
}

#[cfg(test)]
mod blueprint_course_tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn blueprint_course_input_is_one_nested_question_revision_tree() {
        let input = CreateBlueprintCourseInput {
            short_name: "Biochemistry".to_owned(),
            long_name: "Biochemistry Blueprint".to_owned(),
            modules: vec![CreateBlueprintModuleInput {
                label: "Week 1".to_owned(),
                assessments: vec![BlueprintAssessmentContentInput {
                    assessment_type: crate::AssessmentType::RegularAssignment,
                    title: "Protein folding".to_owned(),
                    instructions: AssessmentInstructions::default(),
                    entries: vec![BlueprintAssessmentEntryInput::Fixed(
                        ReusableFixedQuestionInput {
                            published_question: QuestionRevisionReference {
                                question_id: "7K3M-X9QX".parse().expect("QuestionId"),
                                revision_number: QuestionRevisionNumber::new(1)
                                    .expect("positive Revision"),
                            },
                            points_possible: AssessmentPointValue::from_whole(1),
                            scoring_rule: AssessmentEntryScoringRule::Normal,
                            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                        },
                    )],
                    defaults: BlueprintAssessmentDefaults {
                        assessment_attempt_time_limit_seconds: None,
                        attempt_limit: None,
                        late_work_rule: LateWorkRule::Accept,
                        activity_rules: AssessmentActivityRules {
                            assessment_attempt_grade_rule:
                                crate::AssessmentAttemptGradeRule::Highest,
                            question_variation_rule:
                                crate::AssessmentQuestionVariationRule::NewVariation,
                            ..AssessmentActivityRules::default()
                        },
                        student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
                    },
                }],
            }],
        };
        input.validate().expect("valid BlueprintCourse");
        let wire = serde_json::to_value(&input).expect("serializes");
        assert!(wire.get("modules").is_some());
        assert!(wire.to_string().contains("published_question"));
        assert!(!wire.to_string().contains("QuestionRevisionReference"));
        let mut forged = wire;
        forged["owner"] = serde_json::json!("U-1");
        assert!(serde_json::from_value::<CreateBlueprintCourseInput>(forged).is_err());
    }

    #[test]
    fn replacement_choices_are_explicit_strict_and_unique() {
        let blueprint_module_reference = BlueprintModuleReference::from_uuid(Uuid::from_u128(1));
        let blueprint_assessment_reference =
            BlueprintAssessmentReference::from_uuid(Uuid::from_u128(2));
        assert!(
            "00000000000000000000000000000001"
                .parse::<BlueprintModuleReference>()
                .is_err()
        );
        assert!(
            "00000000-0000-0000-0000-00000000000A"
                .parse::<BlueprintAssessmentReference>()
                .is_err()
        );
        let content = BlueprintAssessmentContentInput {
            assessment_type: crate::AssessmentType::RegularAssignment,
            title: "Protein folding".to_owned(),
            instructions: AssessmentInstructions::default(),
            entries: vec![BlueprintAssessmentEntryInput::Fixed(
                ReusableFixedQuestionInput {
                    published_question: QuestionRevisionReference {
                        question_id: "7K3M-X9QX".parse().expect("QuestionId"),
                        revision_number: QuestionRevisionNumber::new(1).expect("positive Revision"),
                    },
                    points_possible: AssessmentPointValue::from_whole(1),
                    scoring_rule: AssessmentEntryScoringRule::Normal,
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                },
            )],
            defaults: BlueprintAssessmentDefaults {
                assessment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::Accept,
                activity_rules: AssessmentActivityRules {
                    assessment_attempt_grade_rule: crate::AssessmentAttemptGradeRule::Highest,
                    question_variation_rule: crate::AssessmentQuestionVariationRule::NewVariation,
                    ..AssessmentActivityRules::default()
                },
                student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            },
        };
        let replacement = ReplaceBlueprintCourseContentInput {
            modules: vec![BlueprintModuleReplacementInput {
                choice: BlueprintModuleEditChoice::Retained {
                    blueprint_module_reference,
                },
                label: "Week 1".to_owned(),
                assessments: vec![
                    BlueprintAssessmentReplacementInput {
                        choice: BlueprintAssessmentEditChoice::Retained {
                            blueprint_assessment_reference,
                        },
                        content: content.clone(),
                    },
                    BlueprintAssessmentReplacementInput {
                        choice: BlueprintAssessmentEditChoice::New,
                        content: content.clone(),
                    },
                ],
            }],
        };
        replacement.validate().expect("valid retained/new tree");
        let wire = serde_json::to_value(&replacement).expect("serializes");
        assert_eq!(wire["modules"][0]["choice"]["kind"], "retained");
        assert_eq!(
            wire["modules"][0]["assessments"][1]["choice"]["kind"],
            "new"
        );
        let mut forged = wire;
        forged["modules"][0]["choice"]["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ReplaceBlueprintCourseContentInput>(forged).is_err());

        let duplicated = ReplaceBlueprintCourseContentInput {
            modules: vec![
                BlueprintModuleReplacementInput {
                    choice: BlueprintModuleEditChoice::Retained {
                        blueprint_module_reference,
                    },
                    label: "Week 1".to_owned(),
                    assessments: vec![BlueprintAssessmentReplacementInput {
                        choice: BlueprintAssessmentEditChoice::Retained {
                            blueprint_assessment_reference,
                        },
                        content: content.clone(),
                    }],
                },
                BlueprintModuleReplacementInput {
                    choice: BlueprintModuleEditChoice::Retained {
                        blueprint_module_reference,
                    },
                    label: "Week 2".to_owned(),
                    assessments: vec![BlueprintAssessmentReplacementInput {
                        choice: BlueprintAssessmentEditChoice::New,
                        content,
                    }],
                },
            ],
        };
        assert_eq!(
            duplicated.validate(),
            Err(BlueprintCourseValidationError::DuplicateRetainedBlueprintModuleChoice)
        );
    }
}
