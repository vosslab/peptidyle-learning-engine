//! Browser-safe reusable BlueprintCourse definitions and answer-free views.
//!
//! A reusable definition has no course, student, version, or server-private
//! identity. The Store resolves its public Question IDs to exact publication
//! pins before persistence. Browser views deliberately keep the same ordered
//! shape while substituting current answer-free Question Library discovery rows.

use std::collections::BTreeSet;
use std::num::NonZeroU64;
use std::str::FromStr;

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

use crate::{
    AssignmentActivityRules, AssignmentDeadlineRule, AssignmentEntryScoringRule,
    AssignmentInstructions, AssignmentPointValue, BlueprintCourseReference, LateWorkRule,
    MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL, MAX_ASSIGNMENT_ORDERED_ENTRIES,
    MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES, MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS,
    QuestionId, QuestionPoolSelectionRule, QuestionSearchResult, StudentFeedbackReleaseRule,
};

/// Shared instructor-content bound for reusable titles and module labels.
pub const MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS: usize =
    MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS;

mod blueprint_children;
pub use blueprint_children::*;

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

/// Validates a durable reusable definition title or module label.
pub fn validate_blueprint_course_title(value: &str) -> Result<(), BlueprintCourseTitleError> {
    (value == value.trim()
        && !value.is_empty()
        && value.chars().count() <= MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS)
        .then_some(())
        .ok_or(BlueprintCourseTitleError::Invalid)
}

/// Exact local wall-clock time used with a signed curriculum-day offset.
///
/// This is intentionally time-only: B2 resolves it against an instructor's
/// selected target term and reports any daylight-saving correction required.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LocalTimeOfDay(String);

impl LocalTimeOfDay {
    /// Parses the canonical `HH:MM:SS.sss` browser wire value.
    pub fn parse(value: &str) -> Result<Self, LocalTimeOfDayError> {
        let bytes = value.as_bytes();
        let exact_shape = bytes.len() == 12
            && bytes[2] == b':'
            && bytes[5] == b':'
            && bytes[8] == b'.'
            && bytes
                .iter()
                .enumerate()
                .all(|(index, byte)| matches!(index, 2 | 5 | 8) || byte.is_ascii_digit());
        if !exact_shape || NaiveTime::parse_from_str(value, "%H:%M:%S%.3f").is_err() {
            return Err(LocalTimeOfDayError);
        }
        Ok(Self(value.to_owned()))
    }

    /// Returns the canonical browser wire value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for LocalTimeOfDay {
    type Error = LocalTimeOfDayError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<LocalTimeOfDay> for String {
    fn from(value: LocalTimeOfDay) -> Self {
        value.0
    }
}

/// A local time was not the exact time-only browser wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTimeOfDayError;

impl std::fmt::Display for LocalTimeOfDayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("local time must be exact HH:MM:SS.sss")
    }
}

impl std::error::Error for LocalTimeOfDayError {}

/// One curriculum-calendar moment relative to a target term's first day.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RelativeAssignmentScheduleMoment {
    /// Signed calendar-day offset from the target term's first local day.
    pub day_offset: i32,
    /// Exact local wall-clock time for that calendar day.
    pub local_time: LocalTimeOfDay,
}

/// Optional curriculum-relative availability, due, and close defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RelativeAssignmentSchedule {
    /// First local moment when students may open a future copied assignment.
    pub available_at: Option<RelativeAssignmentScheduleMoment>,
    /// Ordinary local due moment for a future copied assignment.
    pub due_at: Option<RelativeAssignmentScheduleMoment>,
    /// Local moment after which a future copied assignment is closed.
    pub closes_at: Option<RelativeAssignmentScheduleMoment>,
}

impl RelativeAssignmentSchedule {
    /// Validates the partial schedule's meaningful chronological order.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        if ordered_after(self.available_at.as_ref(), self.due_at.as_ref())
            || ordered_after(self.available_at.as_ref(), self.closes_at.as_ref())
            || ordered_after(self.due_at.as_ref(), self.closes_at.as_ref())
        {
            return Err(BlueprintCourseValidationError::InvalidScheduleOrder);
        }
        Ok(())
    }
}

fn ordered_after(
    earlier: Option<&RelativeAssignmentScheduleMoment>,
    later: Option<&RelativeAssignmentScheduleMoment>,
) -> bool {
    earlier
        .zip(later)
        .is_some_and(|(earlier, later)| earlier > later)
}

/// Reusable assignment policy defaults copied into a future teaching course.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableAssignmentDefaults {
    /// Whole Assignment Attempt time limit, if the reusable definition establishes one.
    pub assignment_attempt_time_limit_seconds: Option<std::num::NonZeroU32>,
    /// Number of Student runs, if the reusable definition establishes one.
    pub attempt_limit: Option<std::num::NonZeroU32>,
    /// Late-work treatment copied into the future assignment policy.
    pub late_work_rule: LateWorkRule,
    /// Server deadline action copied into the future assignment policy.
    pub assignment_deadline_rule: AssignmentDeadlineRule,
    /// Independent run behavior copied into the future assignment policy.
    pub activity_rules: AssignmentActivityRules,
    /// Student-release policy copied into the future assignment policy.
    #[serde(rename = "student_feedback_release_rule")]
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
}

impl ReusableAssignmentDefaults {
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

/// One public Question ID submitted as a fixed ordered item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableFixedQuestionInput {
    /// Public published-question locator resolved under destination authority.
    pub question_id: QuestionId,
    /// Points copied into the future fixed item.
    pub points_possible: AssignmentPointValue,
    /// Score treatment copied into the future fixed item.
    pub scoring_rule: AssignmentEntryScoringRule,
}

/// One submitted pool, including its ordered public Question ID candidates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolInput {
    /// Public candidates resolved under destination authority in this order.
    pub candidates: Vec<QuestionId>,
    /// Number of candidates drawn into each future run.
    pub draw_count: u32,
    /// Points copied for every selected candidate.
    pub points_per_item: AssignmentPointValue,
    /// Scoring rule copied for every selected candidate.
    pub scoring_rule: AssignmentEntryScoringRule,
    /// Complete reviewed selection behavior.
    pub selection_rule: QuestionPoolSelectionRule,
}

impl ReusablePoolInput {
    fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        if self.candidates.is_empty()
            || self.candidates.len() > MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL
        {
            return Err(BlueprintCourseValidationError::InvalidPoolCandidates);
        }
        if self.draw_count == 0
            || usize::try_from(self.draw_count).ok() > Some(self.candidates.len())
        {
            return Err(BlueprintCourseValidationError::InvalidPoolDrawCount);
        }
        if self.candidates.iter().collect::<BTreeSet<_>>().len() != self.candidates.len() {
            return Err(BlueprintCourseValidationError::DuplicatePoolCandidate);
        }
        Ok(())
    }
}

/// One ordered reusable definition entry. Vector order is the only position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReusableAssignmentEntryInput {
    /// One fixed question in definition order.
    Fixed(ReusableFixedQuestionInput),
    /// One pool in definition order.
    Pool(ReusablePoolInput),
}

/// Complete submitted reusable-assignment meaning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableAssignmentDefinitionInput {
    /// Instructor-facing title copied into future assignment definitions.
    pub title: String,
    /// Student-facing instructions copied into future assignment definitions.
    pub instructions: AssignmentInstructions,
    /// Fixed items and pools in authored order.
    pub entries: Vec<ReusableAssignmentEntryInput>,
    /// Reusable delivery and run defaults.
    pub defaults: ReusableAssignmentDefaults,
    /// Optional local calendar-relative timing defaults.
    pub schedule: RelativeAssignmentSchedule,
}

impl ReusableAssignmentDefinitionInput {
    /// Validates bounded ordered entries and their reusable schedule meaning.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        validate_blueprint_course_title(&self.title)
            .map_err(|_| BlueprintCourseValidationError::InvalidDefinitionTitle)?;
        if self.entries.is_empty() || self.entries.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidEntryCount);
        }
        self.defaults.validate()?;
        self.schedule.validate()?;
        let mut total_pool_candidates = 0_usize;
        for entry in &self.entries {
            if let ReusableAssignmentEntryInput::Pool(pool) = entry {
                pool.validate()?;
                total_pool_candidates = total_pool_candidates
                    .checked_add(pool.candidates.len())
                    .ok_or(BlueprintCourseValidationError::TooManyPoolCandidates)?;
            }
        }
        (total_pool_candidates <= MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES)
            .then_some(())
            .ok_or(BlueprintCourseValidationError::TooManyPoolCandidates)
    }
}

/// Current selection status for an exact retained reusable question member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReusableSelectionAvailability {
    /// The current publication remains selectable for a new definition.
    Available,
    /// The pinned member remains inspectable but cannot be selected anew.
    Retained,
}

/// Current answer-free discovery projection of one reusable question member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableQuestionView {
    /// Current public Question Library metadata and disclosed evidence.
    pub question_library: QuestionSearchResult,
    /// Whether the stored exact member remains selectable for a new copy.
    pub selection_availability: ReusableSelectionAvailability,
}

/// Current answer-free pool candidate projection in stored candidate order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolCandidateView {
    /// Current public Question Library metadata and disclosed evidence.
    pub question_library: QuestionSearchResult,
    /// Whether the stored exact member remains selectable for a new copy.
    pub selection_availability: ReusableSelectionAvailability,
}

/// Current answer-free reusable pool projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolView {
    /// Current candidates in their retained definition order.
    pub candidates: Vec<ReusablePoolCandidateView>,
    /// Number of candidates drawn into each future run.
    pub draw_count: u32,
    /// Points copied for every selected candidate.
    pub points_per_item: AssignmentPointValue,
    /// Scoring rule copied for every selected candidate.
    pub scoring_rule: AssignmentEntryScoringRule,
    /// Complete reviewed selection behavior.
    pub selection_rule: QuestionPoolSelectionRule,
}

/// Current answer-free reusable-definition entry. Vector order is its position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReusableAssignmentEntryView {
    /// One fixed question in definition order.
    Fixed {
        /// Current answer-free question projection.
        question: Box<ReusableQuestionView>,
        /// Points copied into the future fixed item.
        points_possible: AssignmentPointValue,
        /// Score treatment copied into the future fixed item.
        scoring_rule: AssignmentEntryScoringRule,
    },
    /// One pool in definition order.
    Pool(ReusablePoolView),
}

/// Current answer-free reusable-assignment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableAssignmentDefinitionView {
    /// Instructor-facing title copied into future assignment definitions.
    pub title: String,
    /// Student-facing instructions copied into future assignment definitions.
    pub instructions: AssignmentInstructions,
    /// Fixed items and pools in retained authored order.
    pub entries: Vec<ReusableAssignmentEntryView>,
    /// Reusable delivery and run defaults.
    pub defaults: ReusableAssignmentDefaults,
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

/// Closed lifecycle/read access for one BlueprintCourse view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintCourseAccess {
    /// The active approved Instructor owns this draft.
    Owner,
    /// The active approved Instructor may read this published course.
    ApprovedInstructor,
}

/// Safe compact current projection of one BlueprintCourse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseSummaryView {
    /// Typed route locator resolved under current read authority.
    pub reference: BlueprintCourseReference,
    /// Display title from the aggregate.
    pub title: String,
    /// Strong complete-aggregate revision.
    pub revision: BlueprintRevision,
    /// Current owner-scoped read authority.
    pub access: BlueprintCourseAccess,
}

/// Safe current projection of one complete BlueprintCourse tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseView {
    /// Typed route locator resolved under current owner or published-read authority.
    pub reference: BlueprintCourseReference,
    /// Instructor-visible course title.
    pub title: String,
    /// Strong complete-aggregate revision.
    pub revision: BlueprintRevision,
    /// Current closed read authority.
    pub access: BlueprintCourseAccess,
    /// Labelled modules in retained aggregate-owned order.
    pub modules: Vec<BlueprintCourseModuleView>,
}

/// Meaning-level validation failure for a Blueprint Course command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintCourseValidationError {
    /// A reusable definition title is not durable instructor content.
    InvalidDefinitionTitle,
    /// A BlueprintCourse title is not durable instructor content.
    InvalidBlueprintTitle,
    /// A module label is not durable instructor content.
    InvalidModuleLabel,
    /// A reusable definition has no usable entries or exceeds its shared bound.
    InvalidEntryCount,
    /// A BlueprintCourse has no usable modules or exceeds its shared bound.
    InvalidModuleCount,
    /// A module has no usable definitions or exceeds its shared bound.
    InvalidModuleDefinitionCount,
    /// A pool candidate list has no members or exceeds its shared bound.
    InvalidPoolCandidates,
    /// A pool draw count cannot select a meaningful subset of its candidates.
    InvalidPoolDrawCount,
    /// A pool repeats a candidate and therefore changes no selectable meaning.
    DuplicatePoolCandidate,
    /// All pool candidates exceed the assignment-level shared bound.
    TooManyPoolCandidates,
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
            Self::InvalidDefinitionTitle => "reusable definition title is invalid",
            Self::InvalidBlueprintTitle => "BlueprintCourse title is invalid",
            Self::InvalidModuleLabel => "BlueprintCourse module label is invalid",
            Self::InvalidEntryCount => "reusable definition must contain bounded ordered entries",
            Self::InvalidModuleCount => "BlueprintCourse must contain bounded modules",
            Self::InvalidModuleDefinitionCount => {
                "BlueprintCourse module must contain bounded reusable definitions"
            }
            Self::InvalidPoolCandidates => "pool candidates must be present and within their bound",
            Self::InvalidPoolDrawCount => "pool draw count must be between one and candidate count",
            Self::DuplicatePoolCandidate => "pool candidates must be distinct",
            Self::TooManyPoolCandidates => "pool candidates exceed the assignment-level bound",
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
    use crate::taxonomy::License;
    use crate::{
        ActivityTimestamp, PublicAuthorName, PublicByline, QuestionBackend,
        QuestionBackendCapabilities, QuestionMetadata, QuestionStatistics, QuestionSummary,
        QuestionType, QuestionVersionAvailability,
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

    fn defaults() -> ReusableAssignmentDefaults {
        ReusableAssignmentDefaults {
            assignment_attempt_time_limit_seconds: None,
            attempt_limit: None,
            late_work_rule: LateWorkRule::Accept,
            assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
            activity_rules: AssignmentActivityRules {
                assignment_completion_rule: crate::AssignmentCompletionRule::AnswerAll,
                assignment_attempt_grade_rule: crate::AssignmentAttemptGradeRule::Highest,
                assignment_attempt_continuation_rule:
                    crate::AssignmentAttemptContinuationRule::Unlimited,
                question_variation_rule: crate::QuestionVariationRule::ReuseQuestionsWithNewSeeds,
                ..AssignmentActivityRules::default()
            },
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
        }
    }

    fn input(schedule: RelativeAssignmentSchedule) -> ReusableAssignmentDefinitionInput {
        ReusableAssignmentDefinitionInput {
            title: "Protein structure practice".to_string(),
            instructions: AssignmentInstructions::try_new("Explain each choice.".to_string())
                .expect("valid instructions"),
            entries: vec![
                ReusableAssignmentEntryInput::Fixed(ReusableFixedQuestionInput {
                    question_id: question_id(),
                    points_possible: AssignmentPointValue::from_whole(3),
                    scoring_rule: AssignmentEntryScoringRule::Normal,
                }),
                ReusableAssignmentEntryInput::Pool(ReusablePoolInput {
                    candidates: vec![
                        question_id(),
                        "12A-4BCZ".parse().expect("valid question ID"),
                    ],
                    draw_count: 1,
                    points_per_item: AssignmentPointValue::from_whole(2),
                    scoring_rule: AssignmentEntryScoringRule::Normal,
                    selection_rule: QuestionPoolSelectionRule {
                        ordering: crate::SelectionOrdering::Randomized,
                        algorithm: crate::PoolDrawAlgorithm::V1,
                    },
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
                backend: QuestionBackend::Native,
                question_type: QuestionType::MultipleChoice,
                capabilities: QuestionBackendCapabilities::none(),
                metadata: QuestionMetadata {
                    title: "Safe Question Library row".to_string(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::Cc0,
                    language: "en".to_string(),
                },
                byline: PublicByline::new(vec![
                    PublicAuthorName::new("Ada Lovelace".to_string()).expect("valid byline"),
                ])
                .expect("valid byline"),
                availability: QuestionVersionAvailability::Available,
                published_at: ActivityTimestamp::from_unix_millis(0),
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
    fn ordered_definition_validation_uses_vector_order_and_pool_meaning() {
        let definition = input(RelativeAssignmentSchedule::default());
        assert!(definition.validate().is_ok());
        let wire = serde_json::to_value(&definition).expect("definition serializes");
        assert_eq!(wire["entries"][0]["kind"], "fixed");
        assert_eq!(wire["entries"][0]["question_id"], "7K3-M9QX");
        assert_eq!(wire["entries"][0]["points_possible"], "3");
        assert_eq!(wire["entries"][1]["kind"], "pool");
        assert_eq!(wire["entries"][1]["points_per_item"], "2");
        assert!(wire["defaults"].is_object());
        assert!(wire["schedule"].is_object());
        assert_eq!(
            serde_json::from_value::<ReusableAssignmentDefinitionInput>(wire)
                .expect("definition round trips"),
            definition
        );
        let duplicate_pool = ReusableAssignmentDefinitionInput {
            entries: vec![ReusableAssignmentEntryInput::Pool(ReusablePoolInput {
                candidates: vec![question_id(), question_id()],
                draw_count: 1,
                points_per_item: AssignmentPointValue::from_whole(1),
                scoring_rule: AssignmentEntryScoringRule::Normal,
                selection_rule: QuestionPoolSelectionRule {
                    ordering: crate::SelectionOrdering::CandidateOrder,
                    algorithm: crate::PoolDrawAlgorithm::V1,
                },
            })],
            ..definition
        };
        assert_eq!(
            duplicate_pool.validate(),
            Err(BlueprintCourseValidationError::DuplicatePoolCandidate)
        );
        let blueprint = CreateBlueprintCourseDefinitionInput {
            title: "Biochemistry Blueprint".to_string(),
            modules: vec![CreateBlueprintCourseModuleInput {
                label: "Week 1".to_string(),
                definitions: vec![input(RelativeAssignmentSchedule::default())],
            }],
        };
        assert!(blueprint.validate().is_ok());
    }

    #[test]
    fn blueprint_projection_serializes_answer_free_question_library_rows_and_edit_handles() {
        let view = BlueprintCourseView {
            reference: "BP-12".parse().expect("valid reference"),
            title: "Biochemistry Blueprint".to_string(),
            revision: BlueprintRevision::new(4).expect("valid revision"),
            access: BlueprintCourseAccess::ApprovedInstructor,
            modules: vec![BlueprintCourseModuleView {
                module_id: module_id(),
                label: "Week 1".to_string(),
                definitions: vec![BlueprintCourseAssignmentDefinitionView {
                    assignment_id: assignment_id(),
                    definition: ReusableAssignmentDefinitionView {
                        title: "Protein structure practice".to_string(),
                        instructions: AssignmentInstructions::default(),
                        entries: vec![ReusableAssignmentEntryView::Fixed {
                            question: ReusableQuestionView {
                                question_library: discovery(),
                                selection_availability: ReusableSelectionAvailability::Available,
                            }
                            .into(),
                            points_possible: AssignmentPointValue::from_whole(3),
                            scoring_rule: AssignmentEntryScoringRule::Normal,
                        }],
                        defaults: defaults(),
                        schedule: RelativeAssignmentSchedule::default(),
                    },
                }],
            }],
        };
        let wire = serde_json::to_value(view).expect("safe projection serializes");
        assert_eq!(wire["reference"], "BP-12");
        assert_eq!(
            wire["modules"][0]["definitions"][0]["definition"]["entries"][0]["kind"],
            "fixed"
        );
        assert!(
            wire.pointer("/modules/0/definitions/0/definition/entries/0/question/question_library")
                .is_some()
        );
        assert_eq!(
            wire.pointer("/modules/0/definitions/0/assignment_id"),
            Some(&serde_json::Value::String(assignment_id().to_string()))
        );
        assert!(
            wire.pointer("/modules/0/definitions/0/definition/entries/0/revision")
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
        let input = CreateBlueprintCourseDefinitionInput {
            title: "Biochemistry".to_owned(),
            modules: vec![CreateBlueprintCourseModuleInput {
                label: "Week 1".to_owned(),
                definitions: vec![ReusableAssignmentDefinitionInput {
                    title: "Protein folding".to_owned(),
                    instructions: AssignmentInstructions::default(),
                    entries: vec![ReusableAssignmentEntryInput::Fixed(
                        ReusableFixedQuestionInput {
                            question_id: "7K3-M9QX".parse().expect("QuestionId"),
                            points_possible: AssignmentPointValue::from_whole(1),
                            scoring_rule: AssignmentEntryScoringRule::Normal,
                        },
                    )],
                    defaults: ReusableAssignmentDefaults {
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
                            question_variation_rule:
                                crate::QuestionVariationRule::ReuseQuestionsWithNewSeeds,
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
        assert!(!wire.to_string().contains("QuestionVersionReference"));
        let mut forged = wire;
        forged["owner"] = serde_json::json!("U-1");
        assert!(serde_json::from_value::<CreateBlueprintCourseDefinitionInput>(forged).is_err());
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
        let definition = ReusableAssignmentDefinitionInput {
            title: "Protein folding".to_owned(),
            instructions: AssignmentInstructions::default(),
            entries: vec![ReusableAssignmentEntryInput::Fixed(
                ReusableFixedQuestionInput {
                    question_id: "7K3-M9QX".parse().expect("QuestionId"),
                    points_possible: AssignmentPointValue::from_whole(1),
                    scoring_rule: AssignmentEntryScoringRule::Normal,
                },
            )],
            defaults: ReusableAssignmentDefaults {
                assignment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::Accept,
                assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
                activity_rules: AssignmentActivityRules {
                    assignment_completion_rule: crate::AssignmentCompletionRule::AnswerAll,
                    assignment_attempt_grade_rule: crate::AssignmentAttemptGradeRule::Highest,
                    assignment_attempt_continuation_rule:
                        crate::AssignmentAttemptContinuationRule::Unlimited,
                    question_variation_rule:
                        crate::QuestionVariationRule::ReuseQuestionsWithNewSeeds,
                    ..AssignmentActivityRules::default()
                },
                student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            },
            schedule: RelativeAssignmentSchedule::default(),
        };
        let replacement = ReplaceBlueprintCourseDefinitionInput {
            title: "Biochemistry".to_owned(),
            modules: vec![BlueprintCourseModuleReplacementInput {
                handle: BlueprintModuleEditHandle::Retained { module_id },
                label: "Week 1".to_owned(),
                definitions: vec![
                    BlueprintCourseAssignmentReplacementInput {
                        handle: BlueprintAssignmentEditHandle::Retained { assignment_id },
                        definition: definition.clone(),
                    },
                    BlueprintCourseAssignmentReplacementInput {
                        handle: BlueprintAssignmentEditHandle::New,
                        definition: definition.clone(),
                    },
                ],
            }],
        };
        replacement.validate().expect("valid retained/new tree");
        let wire = serde_json::to_value(&replacement).expect("serializes");
        assert_eq!(wire["modules"][0]["handle"]["kind"], "retained");
        assert_eq!(
            wire["modules"][0]["definitions"][1]["handle"]["kind"],
            "new"
        );
        let mut forged = wire;
        forged["modules"][0]["handle"]["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ReplaceBlueprintCourseDefinitionInput>(forged).is_err());

        let duplicated = ReplaceBlueprintCourseDefinitionInput {
            modules: vec![
                BlueprintCourseModuleReplacementInput {
                    handle: BlueprintModuleEditHandle::Retained { module_id },
                    label: "Week 1".to_owned(),
                    definitions: vec![BlueprintCourseAssignmentReplacementInput {
                        handle: BlueprintAssignmentEditHandle::Retained { assignment_id },
                        definition: definition.clone(),
                    }],
                },
                BlueprintCourseModuleReplacementInput {
                    handle: BlueprintModuleEditHandle::Retained { module_id },
                    label: "Week 2".to_owned(),
                    definitions: vec![BlueprintCourseAssignmentReplacementInput {
                        handle: BlueprintAssignmentEditHandle::New,
                        definition,
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
