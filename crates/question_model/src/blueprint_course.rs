//! Browser-safe reusable BlueprintCourse assignments and answer-free views.
//!
//! A reusable content has no course, student, version, or server-private
//! identity. The Store resolves its public Question IDs to exact publication
//! pins before persistence. Browser views deliberately keep the same ordered
//! shape while substituting current answer-free Question Library discovery rows.

use std::collections::BTreeSet;
use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{
    AssignmentActivityRules, AssignmentDeadlineRule, AssignmentEntryScoringRule,
    AssignmentInstructions, AssignmentPointValue, BlueprintCourseReference, LateWorkRule,
    MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    MAX_ASSIGNMENT_ORDERED_ENTRIES, MAX_ASSIGNMENT_QUESTION_POOL_ITEMS,
    MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY, QuestionAttemptLimit, QuestionAttemptTimeLimit,
    QuestionId, QuestionPoolSelectionRule, QuestionSearchResult, StudentFeedbackReleaseRule,
};

/// Shared instructor-content bound for reusable titles and module labels.
pub const MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS: usize = 200;

mod blueprint_children;
pub use blueprint_children::{
    BlueprintAssignmentEditHandle, BlueprintAssignmentId, BlueprintChildIdError,
    BlueprintCourseAssignmentContentView, BlueprintCourseAssignmentReplacementInput,
    BlueprintCourseModuleReplacementInput, BlueprintCourseModuleView, BlueprintModuleEditHandle,
    BlueprintModuleId, CreateBlueprintCourseContentInput, CreateBlueprintCourseModuleInput,
    ReplaceBlueprintCourseContentInput,
};

mod relative_assignment_schedule;
pub use relative_assignment_schedule::{
    LocalTimeOfDay, LocalTimeOfDayError, RelativeAssignmentSchedule,
    RelativeAssignmentScheduleMoment,
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

/// Blueprint Assignment policy defaults copied into a future teaching course.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssignmentDefaults {
    /// Whole Assignment Attempt time limit, if the reusable content establishes one.
    pub assignment_attempt_time_limit_seconds: Option<std::num::NonZeroU32>,
    /// Number of Assignment Attempts, if the reusable content establishes one.
    pub attempt_limit: Option<std::num::NonZeroU32>,
    /// Late-work treatment copied into the future assignment policy.
    pub late_work_rule: LateWorkRule,
    /// Server deadline action copied into the future assignment policy.
    pub assignment_deadline_rule: AssignmentDeadlineRule,
    /// Independent Assignment Attempt behavior copied into the future assignment policy.
    pub activity_rules: AssignmentActivityRules,
    /// Student-release policy copied into the future assignment policy.
    #[serde(rename = "student_feedback_release_rule")]
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
}

impl BlueprintAssignmentDefaults {
    /// Validates reusable limits against the ordinary teaching-policy bounds.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        if self
            .assignment_attempt_time_limit_seconds
            .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS)
        {
            return Err(BlueprintCourseValidationError::AssignmentAttemptTimeLimitOutOfRange);
        }
        if self
            .attempt_limit
            .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_LIMIT)
        {
            return Err(BlueprintCourseValidationError::AttemptLimitOutOfRange);
        }
        Ok(())
    }
}

/// One Fixed Question Assignment Entry submitted in authored order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableFixedQuestionInput {
    /// Public Question ID resolved under destination authority.
    pub question_id: QuestionId,
    /// Points copied into the future Fixed Question Assignment Entry.
    pub points_possible: AssignmentPointValue,
    /// Score treatment copied into the future Fixed Question Assignment Entry.
    pub scoring_rule: AssignmentEntryScoringRule,
    /// Question Attempt retry bound copied into the future Fixed Question Assignment Entry.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Question Attempt timing copied into the future Fixed Question Assignment Entry.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// One Question Pool Assignment Entry, including its ordered public Question Pool Item IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolInput {
    /// Public Question IDs resolved into Question Pool Items under destination authority in this order.
    pub items: Vec<QuestionId>,
    /// Number of Question Pool Items selected for each future Assignment Attempt.
    pub selection_count: u32,
    /// Points copied for every selected Question Pool Item.
    pub points_per_item: AssignmentPointValue,
    /// Scoring rule copied for every selected Question Pool Item.
    pub scoring_rule: AssignmentEntryScoringRule,
    /// Complete reviewed selection behavior.
    pub selection_rule: QuestionPoolSelectionRule,
    /// Uniform Question Attempt retry bound copied for every selected Question Pool Item.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Uniform Question Attempt timing copied for every selected Question Pool Item.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

impl ReusablePoolInput {
    fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        if self.items.is_empty() || self.items.len() > MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY
        {
            return Err(BlueprintCourseValidationError::InvalidQuestionPoolItems);
        }
        if self.selection_count == 0
            || usize::try_from(self.selection_count).ok() > Some(self.items.len())
        {
            return Err(BlueprintCourseValidationError::InvalidPoolSelectionCount);
        }
        if self.items.iter().collect::<BTreeSet<_>>().len() != self.items.len() {
            return Err(BlueprintCourseValidationError::DuplicateQuestionPoolItem);
        }
        Ok(())
    }
}

/// One ordered reusable content entry. Vector order is the only position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintAssignmentEntryInput {
    /// One Fixed Question Assignment Entry in content order.
    Fixed(ReusableFixedQuestionInput),
    /// One Question Pool Assignment Entry in content order.
    Pool(ReusablePoolInput),
}

/// Complete submitted reusable-assignment meaning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssignmentContentInput {
    /// Instructor-facing title copied into future assignment assignments.
    pub title: String,
    /// Student-facing instructions copied into future assignment assignments.
    pub instructions: AssignmentInstructions,
    /// Fixed Question Assignment Entries and Question Pool Assignment Entries in authored order.
    pub entries: Vec<BlueprintAssignmentEntryInput>,
    /// Reusable delivery and Assignment Attempt defaults.
    pub defaults: BlueprintAssignmentDefaults,
    /// Optional local calendar-relative timing defaults.
    pub schedule: RelativeAssignmentSchedule,
}

impl BlueprintAssignmentContentInput {
    /// Validates bounded ordered entries and their reusable schedule meaning.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        validate_blueprint_course_title(&self.title)
            .map_err(|_| BlueprintCourseValidationError::InvalidContentTitle)?;
        if self.entries.is_empty() || self.entries.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidEntryCount);
        }
        self.defaults.validate()?;
        self.schedule.validate()?;
        let mut total_question_pool_items = 0_usize;
        for entry in &self.entries {
            if let BlueprintAssignmentEntryInput::Pool(pool) = entry {
                pool.validate()?;
                total_question_pool_items = total_question_pool_items
                    .checked_add(pool.items.len())
                    .ok_or(BlueprintCourseValidationError::TooManyQuestionPoolItems)?;
            }
        }
        (total_question_pool_items <= MAX_ASSIGNMENT_QUESTION_POOL_ITEMS)
            .then_some(())
            .ok_or(BlueprintCourseValidationError::TooManyQuestionPoolItems)
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

/// Current answer-free Reusable Question View.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableQuestionView {
    /// Current public Question Library metadata and disclosed evidence.
    pub question_library: QuestionSearchResult,
    /// Whether the stored exact member remains selectable for a new copy.
    pub selection_availability: ReusableSelectionAvailability,
}

/// Current answer-free reusable Question Pool Item in stored Question Pool Item order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableQuestionPoolItemView {
    /// Current public Question Library metadata and disclosed evidence.
    pub question_library: QuestionSearchResult,
    /// Whether the stored exact member remains selectable for a new copy.
    pub selection_availability: ReusableSelectionAvailability,
}

/// Current answer-free Reusable Pool View.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolView {
    /// Current Question Pool Items in their retained Question Pool Item order.
    pub items: Vec<ReusableQuestionPoolItemView>,
    /// Number of Question Pool Items selected for each future Assignment Attempt.
    pub selection_count: u32,
    /// Points copied for every selected Question Pool Item.
    pub points_per_item: AssignmentPointValue,
    /// Scoring rule copied for every selected Question Pool Item.
    pub scoring_rule: AssignmentEntryScoringRule,
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
pub enum BlueprintAssignmentEntryView {
    /// One Fixed Question Assignment Entry in content order.
    Fixed {
        /// Current answer-free Reusable Question View.
        question: Box<ReusableQuestionView>,
        /// Points copied into the future Fixed Question Assignment Entry.
        points_possible: AssignmentPointValue,
        /// Score treatment copied into the future Fixed Question Assignment Entry.
        scoring_rule: AssignmentEntryScoringRule,
        /// Question Attempt retry bound copied into the future Fixed Question Assignment Entry.
        question_attempt_limit: QuestionAttemptLimit,
        /// Question Attempt timing copied into the future Fixed Question Assignment Entry.
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// One Question Pool Assignment Entry in content order.
    Pool(ReusablePoolView),
}

/// Current answer-free reusable-assignment content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssignmentContentView {
    /// Instructor-facing title copied into future assignment assignments.
    pub title: String,
    /// Student-facing instructions copied into future assignment assignments.
    pub instructions: AssignmentInstructions,
    /// Fixed Question Assignment Entries and Question Pool Assignment Entries in retained authored order.
    pub entries: Vec<BlueprintAssignmentEntryView>,
    /// Reusable delivery and Assignment Attempt defaults.
    pub defaults: BlueprintAssignmentDefaults,
    /// Optional local calendar-relative timing defaults.
    pub schedule: RelativeAssignmentSchedule,
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
    /// The current Active Instructor Account reads reusable published Blueprint content.
    ActiveInstructor,
}

/// Safe compact current Blueprint Course Summary View.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseSummaryView {
    /// Blueprint Course Reference resolved under current read authority.
    pub reference: BlueprintCourseReference,
    /// Display title from the aggregate.
    pub title: String,
    /// Strong complete-aggregate revision.
    pub revision: BlueprintRevision,
    /// Browser-safe classification for this returned Blueprint Course view.
    pub read_access: BlueprintCourseReadAccess,
}

/// Safe current Blueprint Course View of one complete BlueprintCourse tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseView {
    /// Blueprint Course Reference resolved for this returned read view.
    pub reference: BlueprintCourseReference,
    /// Instructor-visible course title.
    pub title: String,
    /// Strong complete-aggregate revision.
    pub revision: BlueprintRevision,
    /// Browser-safe classification for this returned Blueprint Course view.
    pub read_access: BlueprintCourseReadAccess,
    /// Labelled modules in retained aggregate-owned order.
    pub modules: Vec<BlueprintCourseModuleView>,
}

/// Meaning-level validation failure for a Blueprint Course command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintCourseValidationError {
    /// A reusable content title is not durable instructor content.
    InvalidContentTitle,
    /// A BlueprintCourse title is not durable instructor content.
    InvalidBlueprintTitle,
    /// A module label is not durable instructor content.
    InvalidModuleLabel,
    /// A reusable content has no usable entries or exceeds its shared bound.
    InvalidEntryCount,
    /// A BlueprintCourse has no usable modules or exceeds its shared bound.
    InvalidModuleCount,
    /// A module has no usable assignments or exceeds its shared bound.
    InvalidModuleAssignmentCount,
    /// A Question Pool Item list has no members or exceeds its shared bound.
    InvalidQuestionPoolItems,
    /// A pool selection count cannot select a meaningful subset of its Question Pool Items.
    InvalidPoolSelectionCount,
    /// A pool repeats a Question Pool Item and therefore changes no selectable meaning.
    DuplicateQuestionPoolItem,
    /// All Question Pool Items exceed the assignment-level shared bound.
    TooManyQuestionPoolItems,
    /// Relative available, due, and close moments are not chronologically meaningful.
    InvalidScheduleOrder,
    /// A reusable whole Assignment Attempt time limit exceeds the ordinary assignment bound.
    AssignmentAttemptTimeLimitOutOfRange,
    /// A reusable attempt limit exceeds the ordinary assignment bound.
    AttemptLimitOutOfRange,
    /// A replacement submitted the same retained module handle more than once.
    DuplicateRetainedModuleHandle,
    /// A replacement submitted the same retained assignment handle more than once.
    DuplicateRetainedAssignmentHandle,
}

impl std::fmt::Display for BlueprintCourseValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidContentTitle => "reusable content title is invalid",
            Self::InvalidBlueprintTitle => "BlueprintCourse title is invalid",
            Self::InvalidModuleLabel => "BlueprintCourse module label is invalid",
            Self::InvalidEntryCount => "reusable content must contain bounded ordered entries",
            Self::InvalidModuleCount => "BlueprintCourse must contain bounded modules",
            Self::InvalidModuleAssignmentCount => {
                "BlueprintCourse module must contain bounded reusable assignments"
            }
            Self::InvalidQuestionPoolItems => {
                "Question Pool Items must be present and within their bound"
            }
            Self::InvalidPoolSelectionCount => {
                "Question Pool selection count must be between one and Question Pool Item count"
            }
            Self::DuplicateQuestionPoolItem => "Question Pool Items must be distinct",
            Self::TooManyQuestionPoolItems => {
                "Question Pool Items exceed the assignment-level bound"
            }
            Self::InvalidScheduleOrder => {
                "relative availability, due, and close moments must be ordered"
            }
            Self::AssignmentAttemptTimeLimitOutOfRange => {
                "reusable time limit exceeds the supported range"
            }
            Self::AttemptLimitOutOfRange => "reusable attempt limit exceeds the supported range",
            Self::DuplicateRetainedModuleHandle => {
                "BlueprintCourse replacement repeats a retained module handle"
            }
            Self::DuplicateRetainedAssignmentHandle => {
                "BlueprintCourse replacement repeats a retained assignment handle"
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
        QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionBackend,
        QuestionBackendCapabilities, QuestionMetadata, QuestionRevisionAvailability,
        QuestionRevisionNumber, QuestionRevisionReference, QuestionStatistics, QuestionSummary,
        QuestionType, Timestamp,
    };
    use uuid::Uuid;

    fn module_id() -> BlueprintModuleId {
        BlueprintModuleId::from_uuid(Uuid::from_u128(1))
    }

    fn assignment_id() -> BlueprintAssignmentId {
        BlueprintAssignmentId::from_uuid(Uuid::from_u128(2))
    }

    fn question_id() -> QuestionId {
        "7K3-M9QX".parse().expect("valid question ID")
    }

    fn defaults() -> BlueprintAssignmentDefaults {
        BlueprintAssignmentDefaults {
            assignment_attempt_time_limit_seconds: None,
            attempt_limit: None,
            late_work_rule: LateWorkRule::Accept,
            assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
            activity_rules: AssignmentActivityRules {
                assignment_completion_rule: crate::AssignmentCompletionRule::AnswerAll,
                assignment_attempt_grade_rule: crate::AssignmentAttemptGradeRule::Highest,
                assignment_attempt_continuation_rule:
                    crate::AssignmentAttemptContinuationRule::Unlimited,
                question_pool_reuse_rule: crate::QuestionPoolReuseRule::ReuseSelection,
                question_variation_rule: crate::AssignmentQuestionVariationRule::NewVariation,
                ..AssignmentActivityRules::default()
            },
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
        }
    }

    fn input(schedule: RelativeAssignmentSchedule) -> BlueprintAssignmentContentInput {
        BlueprintAssignmentContentInput {
            title: "Protein structure practice".to_string(),
            instructions: AssignmentInstructions::try_new("Explain each choice.".to_string())
                .expect("valid instructions"),
            entries: vec![
                BlueprintAssignmentEntryInput::Fixed(ReusableFixedQuestionInput {
                    question_id: question_id(),
                    points_possible: AssignmentPointValue::from_whole(3),
                    scoring_rule: AssignmentEntryScoringRule::Normal,
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                }),
                BlueprintAssignmentEntryInput::Pool(ReusablePoolInput {
                    items: vec![
                        question_id(),
                        "12A-4BCZ".parse().expect("valid question ID"),
                    ],
                    selection_count: 1,
                    points_per_item: AssignmentPointValue::from_whole(2),
                    scoring_rule: AssignmentEntryScoringRule::Normal,
                    selection_rule: QuestionPoolSelectionRule {
                        selected_question_order:
                            crate::QuestionPoolSelectedQuestionOrder::RandomOrder,
                    },
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                }),
            ],
            defaults: defaults(),
            schedule,
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
                question_type: QuestionType::MultipleChoice,
                capabilities: QuestionBackendCapabilities::none(),
                metadata: QuestionMetadata {
                    title: "Safe Question Library row".to_string(),
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
                availability: QuestionRevisionAvailability::Available,
                published_at: Timestamp::from_unix_millis(0),
            },
            evidence: QuestionStatistics::InsufficientEvidence,
        }
    }

    #[test]
    fn curriculum_references_round_trip_as_compact_wire_values() {
        let blueprint: BlueprintCourseReference = "BP-42".parse().expect("valid reference");
        assert_eq!(
            serde_json::to_value(blueprint).expect("serializes"),
            "BP-42"
        );
        assert!("BP-042".parse::<BlueprintCourseReference>().is_err());
        assert!("AC-43".parse::<BlueprintCourseReference>().is_err());
    }

    #[test]
    fn local_relative_schedule_keeps_partial_defaults_and_rejects_reversed_pairs() {
        let time = |value| LocalTimeOfDay::parse(value).expect("valid local time");
        let available = RelativeAssignmentScheduleMoment {
            day_offset: -1,
            local_time: time("08:30:00.000"),
        };
        let due = RelativeAssignmentScheduleMoment {
            day_offset: 0,
            local_time: time("17:00:00.000"),
        };
        let close = RelativeAssignmentScheduleMoment {
            day_offset: 1,
            local_time: time("08:00:00.000"),
        };
        for schedule in [
            RelativeAssignmentSchedule {
                available_at: Some(available.clone()),
                due_at: None,
                closes_at: None,
            },
            RelativeAssignmentSchedule {
                available_at: None,
                due_at: Some(due.clone()),
                closes_at: None,
            },
            RelativeAssignmentSchedule {
                available_at: None,
                due_at: None,
                closes_at: Some(close.clone()),
            },
        ] {
            assert!(schedule.validate().is_ok());
        }
        assert_eq!(
            RelativeAssignmentSchedule {
                available_at: None,
                due_at: Some(due),
                closes_at: Some(available),
            }
            .validate(),
            Err(BlueprintCourseValidationError::InvalidScheduleOrder)
        );
        assert!(LocalTimeOfDay::parse("08:30").is_err());
    }

    #[test]
    fn ordered_content_validation_uses_vector_order_and_pool_meaning() {
        let content = input(RelativeAssignmentSchedule::default());
        assert!(content.validate().is_ok());
        let wire = serde_json::to_value(&content).expect("content serializes");
        assert_eq!(wire["entries"][0]["kind"], "fixed");
        assert_eq!(wire["entries"][0]["question_id"], "7K3-M9QX");
        assert_eq!(wire["entries"][0]["points_possible"], "3");
        assert_eq!(wire["entries"][1]["kind"], "pool");
        assert!(wire["entries"][1]["items"].is_array());
        assert!(wire["entries"][1].get("entries").is_none());
        assert_eq!(wire["entries"][1]["points_per_item"], "2");
        assert!(wire["defaults"].is_object());
        assert!(wire["schedule"].is_object());
        assert_eq!(
            serde_json::from_value::<BlueprintAssignmentContentInput>(wire)
                .expect("content round trips"),
            content
        );
        let duplicate_pool = BlueprintAssignmentContentInput {
            entries: vec![BlueprintAssignmentEntryInput::Pool(ReusablePoolInput {
                items: vec![question_id(), question_id()],
                selection_count: 1,
                points_per_item: AssignmentPointValue::from_whole(1),
                scoring_rule: AssignmentEntryScoringRule::Normal,
                selection_rule: QuestionPoolSelectionRule {
                    selected_question_order:
                        crate::QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                },
                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            })],
            ..content
        };
        assert_eq!(
            duplicate_pool.validate(),
            Err(BlueprintCourseValidationError::DuplicateQuestionPoolItem)
        );
        let blueprint = CreateBlueprintCourseContentInput {
            title: "Biochemistry Blueprint".to_string(),
            modules: vec![CreateBlueprintCourseModuleInput {
                label: "Week 1".to_string(),
                assignments: vec![input(RelativeAssignmentSchedule::default())],
            }],
        };
        assert!(blueprint.validate().is_ok());
    }

    #[test]
    fn blueprint_course_view_serializes_answer_free_question_library_rows_and_edit_handles() {
        let view = BlueprintCourseView {
            reference: "BP-12".parse().expect("valid reference"),
            title: "Biochemistry Blueprint".to_string(),
            revision: BlueprintRevision::new(4).expect("valid revision"),
            read_access: BlueprintCourseReadAccess::ActiveInstructor,
            modules: vec![BlueprintCourseModuleView {
                module_id: module_id(),
                label: "Week 1".to_string(),
                assignments: vec![BlueprintCourseAssignmentContentView {
                    assignment_id: assignment_id(),
                    content: BlueprintAssignmentContentView {
                        title: "Protein structure practice".to_string(),
                        instructions: AssignmentInstructions::default(),
                        entries: vec![
                            BlueprintAssignmentEntryView::Fixed {
                                question: ReusableQuestionView {
                                    question_library: discovery(),
                                    selection_availability:
                                        ReusableSelectionAvailability::Available,
                                }
                                .into(),
                                points_possible: AssignmentPointValue::from_whole(3),
                                scoring_rule: AssignmentEntryScoringRule::Normal,
                                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                            },
                            BlueprintAssignmentEntryView::Pool(ReusablePoolView {
                                items: vec![ReusableQuestionPoolItemView {
                                    question_library: discovery(),
                                    selection_availability: ReusableSelectionAvailability::Retained,
                                }],
                                selection_count: 1,
                                points_per_item: AssignmentPointValue::from_whole(2),
                                scoring_rule: AssignmentEntryScoringRule::Normal,
                                selection_rule: QuestionPoolSelectionRule {
                                    selected_question_order:
                                        crate::QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                                },
                                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                            }),
                        ],
                        defaults: defaults(),
                        schedule: RelativeAssignmentSchedule::default(),
                    },
                }],
            }],
        };
        let wire = serde_json::to_value(view).expect("safe view serializes");
        assert_eq!(wire["reference"], "BP-12");
        assert_eq!(
            wire["modules"][0]["assignments"][0]["content"]["entries"][0]["kind"],
            "fixed"
        );
        assert!(
            wire.pointer("/modules/0/assignments/0/content/entries/0/question/question_library")
                .is_some()
        );
        assert_eq!(
            wire.pointer("/modules/0/assignments/0/assignment_id"),
            Some(&serde_json::Value::String(assignment_id().to_string()))
        );
        assert!(
            wire.pointer("/modules/0/assignments/0/content/entries/0/revision")
                .is_none()
        );
        assert_eq!(
            wire["modules"][0]["assignments"][0]["content"]["entries"][1]["kind"],
            "pool"
        );
        assert!(
            wire.pointer("/modules/0/assignments/0/content/entries/1/items/0/question_library")
                .is_some()
        );
        assert!(
            wire.pointer("/modules/0/assignments/0/content/entries/1/items/0/revision")
                .is_none()
        );
    }
}

#[cfg(test)]
mod blueprint_course_tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn blueprint_course_input_is_one_nested_question_id_tree() {
        let input = CreateBlueprintCourseContentInput {
            title: "Biochemistry".to_owned(),
            modules: vec![CreateBlueprintCourseModuleInput {
                label: "Week 1".to_owned(),
                assignments: vec![BlueprintAssignmentContentInput {
                    title: "Protein folding".to_owned(),
                    instructions: AssignmentInstructions::default(),
                    entries: vec![BlueprintAssignmentEntryInput::Fixed(
                        ReusableFixedQuestionInput {
                            question_id: "7K3-M9QX".parse().expect("QuestionId"),
                            points_possible: AssignmentPointValue::from_whole(1),
                            scoring_rule: AssignmentEntryScoringRule::Normal,
                            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                        },
                    )],
                    defaults: BlueprintAssignmentDefaults {
                        assignment_attempt_time_limit_seconds: None,
                        attempt_limit: None,
                        late_work_rule: LateWorkRule::Accept,
                        assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
                        activity_rules: AssignmentActivityRules {
                            assignment_completion_rule: crate::AssignmentCompletionRule::AnswerAll,
                            assignment_attempt_grade_rule:
                                crate::AssignmentAttemptGradeRule::Highest,
                            assignment_attempt_continuation_rule:
                                crate::AssignmentAttemptContinuationRule::Unlimited,
                            question_pool_reuse_rule: crate::QuestionPoolReuseRule::ReuseSelection,
                            question_variation_rule:
                                crate::AssignmentQuestionVariationRule::NewVariation,
                            ..AssignmentActivityRules::default()
                        },
                        student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
                    },
                    schedule: RelativeAssignmentSchedule::default(),
                }],
            }],
        };
        input.validate().expect("valid BlueprintCourse");
        let wire = serde_json::to_value(&input).expect("serializes");
        assert!(wire.get("modules").is_some());
        assert!(wire.to_string().contains("question_id"));
        assert!(!wire.to_string().contains("QuestionRevisionReference"));
        let mut forged = wire;
        forged["owner"] = serde_json::json!("U-1");
        assert!(serde_json::from_value::<CreateBlueprintCourseContentInput>(forged).is_err());
    }

    #[test]
    fn replacement_handles_are_explicit_strict_and_unique() {
        let module_id = BlueprintModuleId::from_uuid(Uuid::from_u128(1));
        let assignment_id = BlueprintAssignmentId::from_uuid(Uuid::from_u128(2));
        assert!(
            "00000000000000000000000000000001"
                .parse::<BlueprintModuleId>()
                .is_err()
        );
        assert!(
            "00000000-0000-0000-0000-00000000000A"
                .parse::<BlueprintAssignmentId>()
                .is_err()
        );
        let content = BlueprintAssignmentContentInput {
            title: "Protein folding".to_owned(),
            instructions: AssignmentInstructions::default(),
            entries: vec![BlueprintAssignmentEntryInput::Fixed(
                ReusableFixedQuestionInput {
                    question_id: "7K3-M9QX".parse().expect("QuestionId"),
                    points_possible: AssignmentPointValue::from_whole(1),
                    scoring_rule: AssignmentEntryScoringRule::Normal,
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                },
            )],
            defaults: BlueprintAssignmentDefaults {
                assignment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::Accept,
                assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
                activity_rules: AssignmentActivityRules {
                    assignment_completion_rule: crate::AssignmentCompletionRule::AnswerAll,
                    assignment_attempt_grade_rule: crate::AssignmentAttemptGradeRule::Highest,
                    assignment_attempt_continuation_rule:
                        crate::AssignmentAttemptContinuationRule::Unlimited,
                    question_pool_reuse_rule: crate::QuestionPoolReuseRule::ReuseSelection,
                    question_variation_rule: crate::AssignmentQuestionVariationRule::NewVariation,
                    ..AssignmentActivityRules::default()
                },
                student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            },
            schedule: RelativeAssignmentSchedule::default(),
        };
        let replacement = ReplaceBlueprintCourseContentInput {
            title: "Biochemistry".to_owned(),
            modules: vec![BlueprintCourseModuleReplacementInput {
                handle: BlueprintModuleEditHandle::Retained { module_id },
                label: "Week 1".to_owned(),
                assignments: vec![
                    BlueprintCourseAssignmentReplacementInput {
                        handle: BlueprintAssignmentEditHandle::Retained { assignment_id },
                        content: content.clone(),
                    },
                    BlueprintCourseAssignmentReplacementInput {
                        handle: BlueprintAssignmentEditHandle::New,
                        content: content.clone(),
                    },
                ],
            }],
        };
        replacement.validate().expect("valid retained/new tree");
        let wire = serde_json::to_value(&replacement).expect("serializes");
        assert_eq!(wire["modules"][0]["handle"]["kind"], "retained");
        assert_eq!(
            wire["modules"][0]["assignments"][1]["handle"]["kind"],
            "new"
        );
        let mut forged = wire;
        forged["modules"][0]["handle"]["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ReplaceBlueprintCourseContentInput>(forged).is_err());

        let duplicated = ReplaceBlueprintCourseContentInput {
            modules: vec![
                BlueprintCourseModuleReplacementInput {
                    handle: BlueprintModuleEditHandle::Retained { module_id },
                    label: "Week 1".to_owned(),
                    assignments: vec![BlueprintCourseAssignmentReplacementInput {
                        handle: BlueprintAssignmentEditHandle::Retained { assignment_id },
                        content: content.clone(),
                    }],
                },
                BlueprintCourseModuleReplacementInput {
                    handle: BlueprintModuleEditHandle::Retained { module_id },
                    label: "Week 2".to_owned(),
                    assignments: vec![BlueprintCourseAssignmentReplacementInput {
                        handle: BlueprintAssignmentEditHandle::New,
                        content,
                    }],
                },
            ],
            ..replacement
        };
        assert_eq!(
            duplicated.validate(),
            Err(BlueprintCourseValidationError::DuplicateRetainedModuleHandle)
        );
    }
}
