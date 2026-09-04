//! Browser-safe current assignment model.
//!
//! Stable Assignment Entry and Question Pool Item identities let Instructors change points, scoring behavior,
//! and future ordering without rewriting immutable published content or
//! inventing assignment-history rows.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

mod point_value;
mod revision;
mod teaching_settings_local;
pub use point_value::AssignmentPointValue;
pub use revision::{AssignmentEditNumber, AssignmentRevisionNumber, AssignmentRevisionNumberError};
pub use teaching_settings_local::{
    AssignmentAuthoredContentFailureCode, AssignmentAuthoredContentFailureReason,
    AssignmentAuthoredContentField, AssignmentAuthoredContentLocalError,
    AssignmentAuthoredContentValidationFailure, CourseLocalDateAndTime,
    CourseLocalDateAndTimeError, InstructorAssignmentAuthoredContentLocal,
    InstructorAssignmentAvailabilityView, derive_instructor_assignment_availability,
};

use crate::{
    AssignmentActivityRules, AssignmentEntryId, QuestionAttemptLimit, QuestionAttemptTimeLimit,
    QuestionPoolItemId, QuestionRevisionReference, Timestamp,
};

/// Maximum Unicode scalar values in one human-facing Assignment Title.
pub const MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS: usize = 200;

/// Largest accepted assignment-instructions length, measured in Unicode scalars.
pub const MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS: usize = 50_000;

/// Maximum fixed-or-pool entries in one ordered Assignment Content record.
pub const MAX_ASSIGNMENT_ORDERED_ENTRIES: usize = 1_024;

/// Maximum Question Pool Items in one Question Pool Assignment Entry.
pub const MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY: usize = 1_024;

/// Maximum Question Pool Items across one complete Assignment Content record.
pub const MAX_ASSIGNMENT_QUESTION_POOL_ITEMS: usize = 8_192;

/// Instructor-controlled stable status for one Assignment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentStatus {
    /// The Assignment remains private to Instructors.
    #[default]
    Unreleased,
    /// The Assignment selects one released revision for future Student access.
    Released,
    /// The Assignment no longer accepts new Student work.
    Closed,
    /// The Assignment is retired from current teaching surfaces.
    Archived,
}

/// Validation failure for browser-safe Assignment Titles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentTitleError {
    /// Titles must contain visible text after trimming.
    Blank,
    /// Titles exceed the bounded shared-contract payload.
    TooLong,
}

impl std::fmt::Display for AssignmentTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blank => formatter.write_str("assignment title must contain non-whitespace text"),
            Self::TooLong => formatter.write_str("assignment title exceeds the maximum length"),
        }
    }
}

impl std::error::Error for AssignmentTitleError {}

/// A validated human-facing Assignment Title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct AssignmentTitle(String);

impl AssignmentTitle {
    /// Validates a title while retaining its authored spacing for display.
    pub fn try_new(value: String) -> Result<Self, AssignmentTitleError> {
        if value.trim().is_empty() {
            return Err(AssignmentTitleError::Blank);
        }
        if value.chars().count() > MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS {
            return Err(AssignmentTitleError::TooLong);
        }
        Ok(Self(value))
    }

    /// Returns the exact authored title.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for AssignmentTitle {
    type Error = AssignmentTitleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<AssignmentTitle> for String {
    fn from(value: AssignmentTitle) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for AssignmentTitle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::try_new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Validation failure for browser-safe assignment instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentInstructionsError {
    /// Instructions contain a NUL scalar, which is not accepted as plain text.
    ContainsNul,
    /// Instructions exceed the bounded shared-contract payload.
    TooLong,
}

impl std::fmt::Display for AssignmentInstructionsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContainsNul => {
                formatter.write_str("assignment instructions contain a NUL character")
            }
            Self::TooLong => {
                formatter.write_str("assignment instructions exceed the maximum length")
            }
        }
    }
}

/// Validated plain-text instructions shown with an assignment.
///
/// The transparent serialization keeps the browser contract as one JSON string.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct AssignmentInstructions(String);

impl AssignmentInstructions {
    /// Validates instructions while allowing the empty text.
    pub fn try_new(value: String) -> Result<Self, AssignmentInstructionsError> {
        if value.contains('\0') {
            return Err(AssignmentInstructionsError::ContainsNul);
        }
        if value.chars().count() > MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS {
            return Err(AssignmentInstructionsError::TooLong);
        }
        Ok(Self(value))
    }

    /// Returns the validated plain text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the value into its validated plain text.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for AssignmentInstructions {
    type Error = AssignmentInstructionsError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl<'de> Deserialize<'de> for AssignmentInstructions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)
            .and_then(|value| Self::try_new(value).map_err(serde::de::Error::custom))
    }
}

/// Monotonic current-scoring generation used to discard stale worker output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ScoringGeneration(u64);

impl ScoringGeneration {
    /// Initial generation for a newly created assignment.
    pub const INITIAL: Self = Self(1);

    /// Rebuilds a positive stored generation.
    pub fn new(value: u64) -> Option<Self> {
        (value > 0).then_some(Self(value))
    }

    /// Returns the stored generation number.
    pub fn value(self) -> u64 {
        self.0
    }

    /// Advances to the next scoring generation.
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Whether current scores may be presented to students and instructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentScoringState {
    /// Current score rows match the assignment generation.
    Current,
    /// Stale rows are hidden while one exact current-generation recalculation runs.
    Recalculating,
    /// Recalculation failed and remains visibly retryable.
    Failed,
}

/// Whether work arriving after the ordinary due date remains acceptable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LateWorkRule {
    /// Accept work after the due date until another hard boundary closes it.
    Accept,
    /// Accept it but preserve the late condition for later reporting.
    MarkLate,
    /// Treat the due date as a hard submission boundary.
    Reject,
}

/// Closed behavior at an effective assignment deadline.
///
/// The database design deliberately chooses server auto-submit instead
/// of an unbounded overtime mode. Keeping that choice as an enum leaves a
/// deliberate extension point without accepting an unsupported boolean state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentDeadlineRule {
    /// The server closes active work independently of browser connectivity.
    AutoSubmit,
}

/// Serializable assignment-owned inputs to effective-policy resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BaseAssignmentPolicy {
    /// First instant when the assignment may be opened.
    #[serde(rename = "available_at")]
    pub available_at: Option<Timestamp>,
    /// Ordinary due instant.
    #[serde(rename = "due_at")]
    pub due_at: Option<Timestamp>,
    /// Hard instant after which new work is closed.
    #[serde(rename = "closes_at")]
    pub closes_at: Option<Timestamp>,
    /// Whole Assignment Attempt time limit when one applies.
    #[serde(rename = "assignment_attempt_time_limit_seconds")]
    pub assignment_attempt_time_limit_seconds: Option<NonZeroU32>,
    /// Maximum number of Assignment Attempts when one applies.
    #[serde(rename = "attempt_limit")]
    pub attempt_limit: Option<NonZeroU32>,
    /// Treatment of work after the ordinary due instant.
    #[serde(rename = "late_work_rule")]
    pub late_work_rule: LateWorkRule,
    /// Server behavior at an effective assignment deadline.
    #[serde(rename = "assignment_deadline_rule")]
    pub assignment_deadline_rule: AssignmentDeadlineRule,
}

impl Default for BaseAssignmentPolicy {
    fn default() -> Self {
        Self {
            available_at: None,
            due_at: None,
            closes_at: None,
            assignment_attempt_time_limit_seconds: None,
            attempt_limit: None,
            late_work_rule: LateWorkRule::Accept,
            assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
        }
    }
}

/// Replaceable Instructor-authored Assignment Content.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentAuthoredContent {
    /// Validated student-facing plain-text instructions.
    pub instructions: AssignmentInstructions,
    /// Base policy supplied to the effective-policy resolver.
    pub base_policy: BaseAssignmentPolicy,
    /// The complete independent activity policy for future Assignment Attempts.
    pub activity_rules: AssignmentActivityRules,
}

/// Largest whole Assignment Attempt limit representable by the current PostgreSQL `INTEGER`
/// columns. Keeping this public makes every browser and storage boundary share
/// the same lossless domain without a needless `BIGINT` migration.
pub const MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS: u32 = 2_147_483_647;

/// Largest attempt limit representable by the normalized PostgreSQL `INTEGER`
/// policy column. Keeping it separate from the time-limit name prevents a
/// browser-only value that the PostgreSQL implementation cannot persist.
pub const MAX_ASSIGNMENT_ATTEMPT_LIMIT: u32 = 2_147_483_647;

/// Whether a top-level Assignment Entry remains available for future Assignment Attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentEntryAvailability {
    /// Make this Assignment Entry available for future Assignment Attempts.
    Available,
    /// Referential tombstone retained for protected existing responses.
    Retired,
}

/// Whether a Question Pool Item remains available for selection by its owning Question Pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionPoolItemAvailability {
    /// Make this Question Pool Item available for future Question Pool Selections.
    Available,
    /// Referential tombstone retained for protected existing responses.
    Retired,
}

/// Current scoring treatment for one stable Assignment Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentEntryScoringRule {
    /// Multiply normalized credit by current points and include the denominator.
    Normal,
    /// Award full current points regardless of normalized credit.
    FullCredit,
    /// Add earned points without increasing the normal denominator.
    ExtraCredit,
    /// Contribute to neither numerator nor denominator.
    Excluded,
}

/// One fixed Question Assignment Entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixedQuestionAssignmentEntry {
    /// Stable identity preserved across point and order changes.
    pub id: AssignmentEntryId,
    /// Exact immutable Question Library content pinned by this Fixed Question Assignment Entry.
    pub reference: QuestionRevisionReference,
    /// Current assignment-authored points.
    pub points_possible: AssignmentPointValue,
    /// Whether future Assignment Attempts may receive this Assignment Entry.
    pub availability: AssignmentEntryAvailability,
    /// Current-only scoring treatment.
    pub scoring_rule: AssignmentEntryScoringRule,
    /// Question Attempt retry bound frozen with this Assignment Entry.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Question Attempt timing frozen with this Assignment Entry.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// Order used for the selected Questions from one Question Pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionPoolSelectedQuestionOrder {
    /// Preserve Question Pool Item order after deterministic selection.
    QuestionPoolOrder,
    /// Deterministically shuffle selected Question Pool Items from the server selection entropy.
    RandomOrder,
}

/// Complete reviewed selection behavior for one Question Pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolSelectionRule {
    /// Order used when the selected Questions are issued.
    pub selected_question_order: QuestionPoolSelectedQuestionOrder,
}

/// One pinned Question Pool Item eligible for its owning Question Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolItem {
    /// Stable Question Pool Item identity used for retirement and audit actions.
    pub id: QuestionPoolItemId,
    /// Exact immutable Question Library version eligible for selection.
    pub reference: QuestionRevisionReference,
    /// Whether future Question Pool Selections may select this Question Pool Item.
    pub availability: QuestionPoolItemAvailability,
}

/// A Question Pool Assignment Entry; issued Questions snapshot the selected result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolAssignmentEntry {
    /// Stable Assignment Entry identity.
    pub id: AssignmentEntryId,
    /// Whether future Assignment Attempts may receive this Assignment Entry.
    pub availability: AssignmentEntryAvailability,
    /// Current-only scoring rule applied to every selected Question Pool Item.
    pub scoring_rule: AssignmentEntryScoringRule,
    /// Number of available Question Pool Items selected for each future Assignment Attempt.
    pub selection_count: u32,
    /// Uniform current points for each selected Question Pool Item.
    pub points_per_item: AssignmentPointValue,
    /// Instructor-owned ordering behavior for the selected Questions.
    pub selection_rule: QuestionPoolSelectionRule,
    /// Uniform Question Attempt retry bound for every Question selected from this pool.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Uniform Question Attempt timing for every Question selected from this pool.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
    /// Pinned Question Pool Items; search criteria are deliberately absent.
    pub items: Vec<QuestionPoolItem>,
}

/// One ordered Assignment Entry in the complete Assignment Content record.
///
/// Assignment Entry order is the authored delivery order. Fixed Questions and
/// Question Pools deliberately share one identity and one top-level sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssignmentEntry {
    /// One exact published Question delivered as written.
    FixedQuestion(FixedQuestionAssignmentEntry),
    /// A deterministic selection from explicit Question Pool Items.
    QuestionPool(QuestionPoolAssignmentEntry),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CourseTerm, CourseTimeZone};
    use chrono::{TimeZone, Utc};

    fn course_term(time_zone: &str) -> CourseTerm {
        CourseTerm::from_parts("2026-01-01", "2026-12-31", time_zone).expect("valid course term")
    }

    fn local(value: &str) -> CourseLocalDateAndTime {
        CourseLocalDateAndTime::parse(value).expect("valid local wall-clock value")
    }

    fn local_settings(
        time_zone: &str,
        available_at: Option<CourseLocalDateAndTime>,
        due_at: Option<CourseLocalDateAndTime>,
        closes_at: Option<CourseLocalDateAndTime>,
    ) -> InstructorAssignmentAuthoredContentLocal {
        InstructorAssignmentAuthoredContentLocal::new(
            CourseTimeZone::parse(time_zone).expect("known zone"),
            AssignmentInstructions::try_new("Read the diagram.".to_string())
                .expect("valid instructions"),
            available_at,
            due_at,
            closes_at,
            NonZeroU32::new(900),
            NonZeroU32::new(2),
            LateWorkRule::MarkLate,
            AssignmentDeadlineRule::AutoSubmit,
        )
        .expect("valid local settings")
    }

    #[test]
    fn question_pool_selection_rule_contains_only_instructor_owned_order() {
        let rule = QuestionPoolSelectionRule {
            selected_question_order: QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
        };
        assert_eq!(
            rule.selected_question_order,
            QuestionPoolSelectedQuestionOrder::QuestionPoolOrder
        );
    }

    #[test]
    fn assignment_time_limit_domain_matches_postgres_integer() {
        assert_eq!(MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS, 2_147_483_647);
        assert_eq!(MAX_ASSIGNMENT_ATTEMPT_LIMIT, 2_147_483_647);
        assert_eq!(MAX_ASSIGNMENT_ORDERED_ENTRIES, 1_024);
        assert_eq!(MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY, 1_024);
        assert_eq!(MAX_ASSIGNMENT_QUESTION_POOL_ITEMS, 8_192);
    }

    #[test]
    fn assignment_revisions_use_canonical_postgres_bigint_strings() {
        let revision: AssignmentRevisionNumber = "43".parse().expect("canonical revision number");
        assert_eq!(serde_json::json!(revision), serde_json::json!("43"));
        assert_eq!(
            revision.checked_next().map(|value| value.to_string()),
            Some("44".into())
        );
        assert!(
            AssignmentRevisionNumber::new(i64::MAX as u64)
                .expect("maximum revision")
                .checked_next()
                .is_none()
        );
        for invalid in ["", "0", "01", "+2", "-2", "9223372036854775808"] {
            assert!(
                invalid.parse::<AssignmentRevisionNumber>().is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn assignment_edit_numbers_are_distinct_canonical_assignment_preconditions() {
        let edit: AssignmentEditNumber = "43".parse().expect("canonical edit number");
        assert_eq!(serde_json::json!(edit), serde_json::json!("43"));
        assert_eq!(
            edit.checked_next().map(|value| value.to_string()),
            Some("44".into())
        );
        for invalid in ["", "0", "01", "+2", "-2", "9223372036854775808"] {
            assert!(
                invalid.parse::<AssignmentEditNumber>().is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn assignment_instructions_are_transparent_and_validated() {
        let empty = AssignmentInstructions::try_new(String::new()).expect("empty instructions");
        assert_eq!(serde_json::to_string(&empty).expect("serialize"), "\"\"");

        let instructions =
            AssignmentInstructions::try_new("Read each prompt carefully.".to_string())
                .expect("valid instructions");
        let json = serde_json::to_string(&instructions).expect("serialize");
        let decoded: AssignmentInstructions = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, instructions);
        assert_eq!(
            AssignmentInstructions::try_new("invalid\0text".to_string()),
            Err(AssignmentInstructionsError::ContainsNul)
        );
    }

    #[test]
    fn assignment_title_is_transparent_and_validated() {
        let title = AssignmentTitle::try_new("Protein folding".to_string()).expect("valid title");
        assert_eq!(
            serde_json::to_string(&title).expect("serialize"),
            "\"Protein folding\""
        );
        assert!(serde_json::from_str::<AssignmentTitle>("\"   \"").is_err());
        assert_eq!(
            AssignmentTitle::try_new("   ".to_string()),
            Err(AssignmentTitleError::Blank)
        );
        assert_eq!(
            AssignmentTitle::try_new("a".repeat(MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS + 1)),
            Err(AssignmentTitleError::TooLong)
        );
    }

    #[test]
    fn assignment_instructions_reject_excessive_unicode_scalars() {
        let too_long = "a".repeat(MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS + 1);
        assert_eq!(
            AssignmentInstructions::try_new(too_long),
            Err(AssignmentInstructionsError::TooLong)
        );
        assert!(
            serde_json::from_value::<AssignmentInstructions>(serde_json::json!("\u{0000}"))
                .is_err()
        );
    }

    #[test]
    fn assignment_authored_content_is_strict_and_uses_direct_cutover_defaults() {
        assert_eq!(
            AssignmentAuthoredContent::default(),
            AssignmentAuthoredContent {
                instructions: AssignmentInstructions::default(),
                base_policy: BaseAssignmentPolicy {
                    late_work_rule: LateWorkRule::Accept,
                    assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
                    ..BaseAssignmentPolicy::default()
                },
                activity_rules: AssignmentActivityRules::default(),
            }
        );
        assert_eq!(
            serde_json::to_value(LateWorkRule::MarkLate).expect("late-work rule serializes"),
            serde_json::json!("mark_late")
        );
        assert!(serde_json::from_value::<LateWorkRule>(serde_json::json!("markLate")).is_err());
        assert_eq!(
            serde_json::to_value(AssignmentDeadlineRule::AutoSubmit)
                .expect("assignment deadline rule serializes"),
            serde_json::json!("auto_submit")
        );
        assert!(
            serde_json::from_value::<AssignmentDeadlineRule>(serde_json::json!("autoSubmit"))
                .is_err()
        );
        assert!(
            serde_json::from_value::<AssignmentAuthoredContent>(serde_json::json!({
                "instructions": "",
                "basePolicy": {
                    "available_at": null,
                    "due_at": null,
                    "closes_at": null,
                    "assignment_attempt_time_limit_seconds": null,
                    "attempt_limit": null,
                    "late_work_rule": "accept",
                    "assignment_deadline_rule": "auto_submit",
                    "unexpected": true
                },
                "activityRules": {
                    "assignmentCompletionRule": { "kind": "answerAll" },
                    "assignmentAttemptGradeRule": "highest",
                    "assignmentAttemptContinuationRule": { "kind": "unlimited" },
                    "questionVariationRule": "invalidValue",
                    "assignmentAttemptResumeRule": "resumable",
                    "assignmentQuestionDisplayRule": "allQuestions",
                    "assignmentNavigationRule": "freeNavigation",
                    "assignmentQuestionOrderRule": "authoredOrder"
                }
            }))
            .is_err()
        );
    }

    #[test]
    fn local_assignment_authored_content_round_trips_exact_milliseconds() {
        let timestamp = Timestamp::from_unix_millis(
            Utc.with_ymd_and_hms(2026, 9, 1, 15, 4, 5)
                .single()
                .expect("valid UTC time")
                .timestamp_millis()
                + 123,
        );
        let settings = AssignmentAuthoredContent {
            instructions: AssignmentInstructions::try_new("Read the diagram.".to_string())
                .expect("valid instructions"),
            base_policy: BaseAssignmentPolicy {
                available_at: Some(timestamp),
                due_at: Some(timestamp),
                closes_at: Some(timestamp),
                assignment_attempt_time_limit_seconds: NonZeroU32::new(900),
                attempt_limit: NonZeroU32::new(2),
                late_work_rule: LateWorkRule::MarkLate,
                assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
            },
            activity_rules: AssignmentActivityRules {
                assignment_attempt_resume_rule: crate::AssignmentAttemptResumeRule::SingleSession,
                assignment_question_display_rule:
                    crate::AssignmentQuestionDisplayRule::OneQuestionAtATime,
                assignment_navigation_rule: crate::AssignmentNavigationRule::ForwardOnly,
                assignment_question_order_rule: crate::AssignmentQuestionOrderRule::Shuffled,
                ..AssignmentActivityRules::default()
            },
        };
        let utc =
            InstructorAssignmentAuthoredContentLocal::from_absolute(&course_term("UTC"), &settings)
                .expect("UTC projection");
        assert_eq!(
            utc.available_at
                .as_ref()
                .map(CourseLocalDateAndTime::as_str),
            Some("2026-09-01T15:04:05.123")
        );
        assert_eq!(
            utc.into_absolute(&course_term("UTC"), settings.activity_rules)
                .expect("UTC resolution"),
            settings
        );

        let chicago = InstructorAssignmentAuthoredContentLocal::from_absolute(
            &course_term("America/Chicago"),
            &settings,
        )
        .expect("Chicago projection");
        assert_eq!(
            chicago
                .available_at
                .as_ref()
                .map(CourseLocalDateAndTime::as_str),
            Some("2026-09-01T10:04:05.123")
        );
        assert_eq!(
            chicago
                .into_absolute(&course_term("America/Chicago"), settings.activity_rules)
                .expect("Chicago resolution"),
            settings
        );
    }

    #[test]
    fn local_assignment_authored_content_refuses_dst_gap_ambiguity_and_mismatch() {
        let term = course_term("America/Chicago");
        let gap = local_settings(
            "America/Chicago",
            Some(local("2026-03-08T02:30:00.000")),
            None,
            None,
        );
        assert_eq!(
            gap.into_absolute(&term, AssignmentActivityRules::default()),
            Err(AssignmentAuthoredContentLocalError::NonexistentLocalTime(
                AssignmentAuthoredContentField::AvailableAt
            ))
        );
        let ambiguity = local_settings(
            "America/Chicago",
            Some(local("2026-11-01T01:30:00.000")),
            None,
            None,
        );
        assert_eq!(
            ambiguity.into_absolute(&term, AssignmentActivityRules::default()),
            Err(AssignmentAuthoredContentLocalError::AmbiguousLocalTime(
                AssignmentAuthoredContentField::AvailableAt
            ))
        );
        let mismatch = local_settings("UTC", Some(local("2026-09-01T15:04:05.123")), None, None);
        assert_eq!(
            mismatch.into_absolute(&term, AssignmentActivityRules::default()),
            Err(AssignmentAuthoredContentLocalError::CourseTimeZoneMismatch)
        );
    }

    #[test]
    fn local_assignment_authored_content_is_strict_and_validates_bounds() {
        assert!(CourseLocalDateAndTime::parse("2026-09-01T10:04").is_err());
        assert!(CourseLocalDateAndTime::parse("2026-09-01T10:04:05.12").is_err());
        assert_eq!(
            InstructorAssignmentAuthoredContentLocal::new(
                CourseTimeZone::parse("UTC").expect("known zone"),
                AssignmentInstructions::default(),
                Some(local("2026-09-01T10:05:00.000")),
                Some(local("2026-09-01T10:04:00.000")),
                None,
                None,
                None,
                LateWorkRule::Accept,
                AssignmentDeadlineRule::AutoSubmit,
            ),
            Err(AssignmentAuthoredContentLocalError::ScheduleOutOfOrder)
        );
        assert_eq!(
            InstructorAssignmentAuthoredContentLocal::new(
                CourseTimeZone::parse("UTC").expect("known zone"),
                AssignmentInstructions::default(),
                None,
                None,
                None,
                None,
                NonZeroU32::new(MAX_ASSIGNMENT_ATTEMPT_LIMIT + 1),
                LateWorkRule::Accept,
                AssignmentDeadlineRule::AutoSubmit,
            ),
            Err(AssignmentAuthoredContentLocalError::AttemptLimitOutOfRange)
        );
        assert!(
            serde_json::from_value::<InstructorAssignmentAuthoredContentLocal>(serde_json::json!({
                "timeZone": "UTC",
                "instructions": "",
                "available_at": null,
                "due_at": null,
                "closes_at": null,
                "assignment_attempt_time_limit_seconds": 0,
                "attempt_limit": null,
                "late_work_rule": "accept",
                "assignment_deadline_rule": "auto_submit"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<InstructorAssignmentAuthoredContentLocal>(serde_json::json!({
                "timeZone": "UTC",
                "instructions": "",
                "available_at": null,
                "due_at": null,
                "closes_at": null,
                "assignment_attempt_time_limit_seconds": null,
                "attempt_limit": null,
                "late_work_rule": "accept",
                "assignment_deadline_rule": "auto_submit",
                "unexpected": true
            }))
            .is_err()
        );
    }

    #[test]
    fn instructor_assignment_availability_uses_authoritative_time_at_exact_boundaries() {
        let term = course_term("UTC");
        let available = Timestamp::from_unix_millis(
            Utc.with_ymd_and_hms(2026, 9, 1, 10, 0, 0)
                .single()
                .expect("valid time")
                .timestamp_millis(),
        );
        let closes = Timestamp::from_unix_millis(
            Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0)
                .single()
                .expect("valid time")
                .timestamp_millis(),
        );
        let settings = AssignmentAuthoredContent {
            instructions: AssignmentInstructions::default(),
            base_policy: BaseAssignmentPolicy {
                available_at: Some(available),
                closes_at: Some(closes),
                ..BaseAssignmentPolicy::default()
            },
            activity_rules: AssignmentActivityRules::default(),
        };

        assert_eq!(
            derive_instructor_assignment_availability(
                &term,
                AssignmentStatus::Released,
                &settings,
                Timestamp::from_unix_millis(available.as_unix_millis() - 1),
            )
            .expect("scheduled state"),
            InstructorAssignmentAvailabilityView::Scheduled {
                available_at: local("2026-09-01T10:00:00.000"),
            }
        );
        assert_eq!(
            derive_instructor_assignment_availability(
                &term,
                AssignmentStatus::Released,
                &settings,
                available,
            )
            .expect("open state"),
            InstructorAssignmentAvailabilityView::Available
        );
        assert_eq!(
            derive_instructor_assignment_availability(
                &term,
                AssignmentStatus::Released,
                &settings,
                closes,
            )
            .expect("closed state"),
            InstructorAssignmentAvailabilityView::Closed {
                closed_at: Some(local("2026-09-01T12:00:00.000")),
            }
        );
    }

    #[test]
    fn instructor_assignment_availability_honors_due_rejection_and_stored_intent() {
        let term = course_term("UTC");
        let due = Timestamp::from_unix_millis(
            Utc.with_ymd_and_hms(2026, 9, 1, 11, 0, 0)
                .single()
                .expect("valid time")
                .timestamp_millis(),
        );
        let settings = AssignmentAuthoredContent {
            instructions: AssignmentInstructions::default(),
            base_policy: BaseAssignmentPolicy {
                due_at: Some(due),
                late_work_rule: LateWorkRule::Reject,
                ..BaseAssignmentPolicy::default()
            },
            activity_rules: AssignmentActivityRules::default(),
        };
        assert_eq!(
            derive_instructor_assignment_availability(
                &term,
                AssignmentStatus::Released,
                &settings,
                due,
            )
            .expect("due-date closure"),
            InstructorAssignmentAvailabilityView::Closed {
                closed_at: Some(local("2026-09-01T11:00:00.000")),
            }
        );

        for (status, expected) in [
            (
                AssignmentStatus::Unreleased,
                InstructorAssignmentAvailabilityView::Unreleased,
            ),
            (
                AssignmentStatus::Closed,
                InstructorAssignmentAvailabilityView::Closed { closed_at: None },
            ),
            (
                AssignmentStatus::Archived,
                InstructorAssignmentAvailabilityView::Archived,
            ),
        ] {
            assert_eq!(
                derive_instructor_assignment_availability(&term, status, &settings, due)
                    .expect("stored assignment status"),
                expected
            );
        }
    }
}
