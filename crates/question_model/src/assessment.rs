//! Browser-safe current Assessment model with stable authored-entry identities.

use std::num::{NonZeroU32, NonZeroU64};

use serde::{Deserialize, Serialize};

mod edit_number;
#[cfg(test)]
#[path = "assessment/local_scheduling_tests.rs"]
mod local_scheduling_tests;
mod point_value;
mod teaching_settings_local;
pub use edit_number::{AssessmentEditNumber, AssessmentEditNumberError};
pub use point_value::AssessmentPointValue;
pub use teaching_settings_local::{
    AssessmentAuthoredContentFailureCode, AssessmentAuthoredContentFailureReason,
    AssessmentAuthoredContentField, AssessmentAuthoredContentLocalError,
    AssessmentAuthoredContentValidationFailure, InstructorAssessmentAuthoredContentLocal,
    InstructorAssessmentAvailabilityView, LocalDateAndTime, LocalDateAndTimeError,
    derive_instructor_assessment_availability,
};

use crate::{
    AssessmentActivityRules, AssessmentEntryId, BlueprintAssessmentSource, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionId, QuestionRevisionReference, Timestamp,
};

/// Maximum Unicode scalar values in one human-facing Assessment Title.
pub const MAX_ASSESSMENT_TITLE_UNICODE_SCALARS: usize = 200;

/// Largest accepted assessment-instructions length, measured in Unicode scalars.
pub const MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS: usize = 50_000;

pub const MAX_ASSESSMENT_ORDERED_ENTRIES: usize = 1_024;

pub const MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY: usize = 1_024;

pub const MAX_ASSESSMENT_QUESTION_POOL_ITEMS: usize = 8_192;

/// Instructor-controlled stable status for one Assessment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentStatus {
    /// The Assessment remains private to Instructors.
    #[default]
    Unreleased,
    /// The current Assessment is available for future Student access.
    Released,
    /// The Assessment no longer accepts new Student work.
    Closed,
    /// The Assessment is retired from current teaching surfaces.
    Archived,
}

/// Fixed pedagogical purpose selected for one Assessment.
///
/// Assessment settings remain independently editable; changing those settings
/// does not infer or rewrite this identity.
/// @tsgen-runtime-values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentType {
    RegularAssignment,
    PracticeQuestionAssignment,
    BonusAssignment,
    Quiz,
    Exam,
}

/// Immutable creation origin for one Course Instance Assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AssessmentOrigin {
    /// Created directly inside a Course Instance.
    Direct,
    /// Copied from one exact Assessment in an immutable Blueprint Revision.
    Adopted {
        /// Existing exact Blueprint Assessment provenance wire shape.
        source: BlueprintAssessmentSource,
    },
}

impl<'de> Deserialize<'de> for AssessmentOrigin {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // ASVS 1.5.2 and 2.2.1: a unit enum variant accepts ignored object
        // fields in Serde, so use an empty struct variant for strict input.
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
        enum StrictAssessmentOrigin {
            Direct {},
            Adopted { source: BlueprintAssessmentSource },
        }

        match StrictAssessmentOrigin::deserialize(deserializer)? {
            StrictAssessmentOrigin::Direct {} => Ok(Self::Direct),
            StrictAssessmentOrigin::Adopted { source } => Ok(Self::Adopted { source }),
        }
    }
}

impl AssessmentType {
    /// Every persisted and browser-visible Assessment Type, in authoring order.
    pub const ALL: [Self; 5] = [
        Self::RegularAssignment,
        Self::PracticeQuestionAssignment,
        Self::BonusAssignment,
        Self::Quiz,
        Self::Exam,
    ];

    /// Returns the exact persisted and JSON wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RegularAssignment => "regular_assignment",
            Self::PracticeQuestionAssignment => "practice_question_assignment",
            Self::BonusAssignment => "bonus_assignment",
            Self::Quiz => "quiz",
            Self::Exam => "exam",
        }
    }

    /// Parses one exact persisted or JSON wire value.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == value)
    }
}

/// Validation failure for browser-safe Assessment Titles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentTitleError {
    /// Titles must contain visible text after trimming.
    Blank,
    /// Titles exceed the bounded shared-contract payload.
    TooLong,
}

impl std::fmt::Display for AssessmentTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blank => formatter.write_str("assessment title must contain non-whitespace text"),
            Self::TooLong => formatter.write_str("assessment title exceeds the maximum length"),
        }
    }
}

impl std::error::Error for AssessmentTitleError {}

/// A validated human-facing Assessment Title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct AssessmentTitle(String);

impl AssessmentTitle {
    /// Validates a title while retaining its authored spacing for display.
    pub fn try_new(value: String) -> Result<Self, AssessmentTitleError> {
        if value.trim().is_empty() {
            return Err(AssessmentTitleError::Blank);
        }
        if value.chars().count() > MAX_ASSESSMENT_TITLE_UNICODE_SCALARS {
            return Err(AssessmentTitleError::TooLong);
        }
        Ok(Self(value))
    }

    /// Returns the exact authored title.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for AssessmentTitle {
    type Error = AssessmentTitleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<AssessmentTitle> for String {
    fn from(value: AssessmentTitle) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for AssessmentTitle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::try_new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Validation failure for browser-safe assessment instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentInstructionsError {
    /// Instructions contain a NUL scalar, which is not accepted as plain text.
    ContainsNul,
    /// Instructions exceed the bounded shared-contract payload.
    TooLong,
}

impl std::fmt::Display for AssessmentInstructionsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContainsNul => {
                formatter.write_str("assessment instructions contain a NUL character")
            }
            Self::TooLong => {
                formatter.write_str("assessment instructions exceed the maximum length")
            }
        }
    }
}

/// Validated plain-text instructions shown with an assessment.
///
/// The transparent serialization keeps the browser contract as one JSON string.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct AssessmentInstructions(String);

impl AssessmentInstructions {
    /// Validates instructions while allowing the empty text.
    pub fn try_new(value: String) -> Result<Self, AssessmentInstructionsError> {
        if value.contains('\0') {
            return Err(AssessmentInstructionsError::ContainsNul);
        }
        if value.chars().count() > MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS {
            return Err(AssessmentInstructionsError::TooLong);
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

impl TryFrom<String> for AssessmentInstructions {
    type Error = AssessmentInstructionsError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl<'de> Deserialize<'de> for AssessmentInstructions {
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
    /// Initial generation for a newly created assessment.
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
pub enum AssessmentScoringState {
    /// Current score rows match the assessment generation.
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

/// Serializable assessment-owned inputs to effective-policy resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BaseAssessmentPolicy {
    /// First instant when the assessment may be opened.
    #[serde(rename = "available_at")]
    pub available_at: Option<Timestamp>,
    /// Ordinary due instant.
    #[serde(rename = "due_at")]
    pub due_at: Option<Timestamp>,
    /// Hard instant after which new work is closed.
    #[serde(rename = "closes_at")]
    pub closes_at: Option<Timestamp>,
    /// Whole Assessment Attempt time limit when one applies.
    #[serde(rename = "assessment_attempt_time_limit_seconds")]
    pub assessment_attempt_time_limit_seconds: Option<NonZeroU32>,
    /// Maximum number of Assessment Attempts when one applies.
    #[serde(rename = "attempt_limit")]
    pub attempt_limit: Option<NonZeroU32>,
    /// Treatment of work after the ordinary due instant.
    #[serde(rename = "late_work_rule")]
    pub late_work_rule: LateWorkRule,
}

impl Default for BaseAssessmentPolicy {
    fn default() -> Self {
        Self {
            available_at: None,
            due_at: None,
            closes_at: None,
            assessment_attempt_time_limit_seconds: None,
            attempt_limit: None,
            late_work_rule: LateWorkRule::Reject,
        }
    }
}

/// Replaceable Instructor-authored Assessment Content.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentAuthoredContent {
    /// Validated student-facing plain-text instructions.
    pub instructions: AssessmentInstructions,
    /// Base policy supplied to the effective-policy resolver.
    pub base_policy: BaseAssessmentPolicy,
    /// The complete independent activity policy for future Assessment Attempts.
    pub activity_rules: AssessmentActivityRules,
}

/// Largest whole Assessment Attempt limit representable by PostgreSQL `INTEGER`.
pub const MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS: u32 = 2_147_483_647;

/// Largest attempt limit representable by PostgreSQL `INTEGER`.
pub const MAX_ASSESSMENT_ATTEMPT_LIMIT: u32 = 2_147_483_647;

/// Whether a top-level Assessment Entry remains available for future Assessment Attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentEntryAvailability {
    /// Make this Assessment Entry available for future Assessment Attempts.
    Available,
    /// Referential tombstone retained for protected existing responses.
    Retired,
}

/// Current scoring treatment for one stable Assessment Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentEntryScoringRule {
    /// Multiply normalized credit by current points and include the denominator.
    Normal,
    /// Award full current points regardless of normalized credit.
    FullCredit,
    /// Add earned points without increasing the normal denominator.
    ExtraCredit,
    /// Contribute to neither numerator nor denominator.
    Excluded,
}

/// One fixed Question Assessment Entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixedQuestionAssessmentEntry {
    /// Stable identity preserved across point and order changes.
    pub id: AssessmentEntryId,
    /// Exact immutable Question Library content pinned by this Fixed Question Assessment Entry.
    pub reference: QuestionRevisionReference,
    /// Current assessment-authored points.
    pub points_possible: AssessmentPointValue,
    /// Whether future Assessment Attempts may receive this Assessment Entry.
    pub availability: AssessmentEntryAvailability,
    /// Current-only scoring treatment.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Question Attempt retry bound frozen with this Assessment Entry.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Question Attempt timing frozen with this Assessment Entry.
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

/// Positive, monotonic Revision number within one Question Pool lineage.
///
/// Pool Revisions use PostgreSQL `BIGINT` and are deliberately independent of
/// the `u32` Question Revision number domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct QuestionPoolRevisionNumber(NonZeroU64);

impl QuestionPoolRevisionNumber {
    /// Creates a positive Pool Revision number.
    pub fn new(value: u64) -> Result<Self, &'static str> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or("Question Pool Revision number must be positive")
    }

    /// Returns the stored positive integer.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl TryFrom<u64> for QuestionPoolRevisionNumber {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<QuestionPoolRevisionNumber> for u64 {
    fn from(value: QuestionPoolRevisionNumber) -> Self {
        value.get()
    }
}

impl std::fmt::Display for QuestionPoolRevisionNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.get())
    }
}

/// One immutable published Question Pool Revision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolRevisionReference {
    /// Public identity of the Assessment-owned fork Pool lineage.
    pub question_pool_id: QuestionId,
    /// Exact immutable Revision of that fork Pool.
    pub revision_number: QuestionPoolRevisionNumber,
}

/// One exact member of an immutable Question Pool Revision.
///
/// The member position is zero-based in the Pool Revision's only member list.
/// It is not a separately minted, mutable item identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolRevisionMemberReference {
    /// Exact immutable Pool Revision that owns the member.
    pub question_pool_revision: QuestionPoolRevisionReference,
    /// Zero-based member position within that immutable Pool Revision.
    pub member_position: u32,
}

/// A Question Pool Assessment Entry; issued Questions snapshot the selected result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolAssessmentEntry {
    /// Stable Assessment Entry identity.
    pub id: AssessmentEntryId,
    /// Exact current immutable Revision of this Assessment-owned fork Pool.
    pub question_pool_revision: QuestionPoolRevisionReference,
    /// Whether future Assessment Attempts may receive this Assessment Entry.
    pub availability: AssessmentEntryAvailability,
    /// Current-only scoring rule applied to every selected Question Pool Item.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Number of available Question Pool Items selected for each future Assessment Attempt.
    pub selection_count: NonZeroU32,
    /// Uniform current points for each selected Question Pool Item.
    pub points_per_item: AssessmentPointValue,
    /// Instructor-owned ordering behavior for the selected Questions.
    pub selection_rule: QuestionPoolSelectionRule,
    /// Uniform Question Attempt retry bound for every Question selected from this pool.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Uniform Question Attempt timing for every Question selected from this pool.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// One ordered Assessment Entry in the complete Assessment Content record.
///
/// Assessment Entry order is the authored delivery order. Fixed Questions and
/// Question Pools deliberately share one identity and one top-level sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssessmentEntry {
    /// One exact published Question delivered as written.
    FixedQuestion(FixedQuestionAssessmentEntry),
    /// A deterministic selection from explicit Question Pool Items.
    QuestionPool(QuestionPoolAssessmentEntry),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CourseTerm;
    use chrono::{TimeZone, Utc};

    fn course_term(_time_zone: &str) -> CourseTerm {
        CourseTerm::from_parts("2026-01-01", "2026-12-31").expect("valid course term")
    }

    fn local(value: &str) -> LocalDateAndTime {
        LocalDateAndTime::parse(value).expect("valid local wall-clock value")
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
    fn assessment_time_limit_domain_matches_postgres_integer() {
        assert_eq!(MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS, 2_147_483_647);
        assert_eq!(MAX_ASSESSMENT_ATTEMPT_LIMIT, 2_147_483_647);
        assert_eq!(MAX_ASSESSMENT_ORDERED_ENTRIES, 1_024);
        assert_eq!(MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY, 1_024);
        assert_eq!(MAX_ASSESSMENT_QUESTION_POOL_ITEMS, 8_192);
    }

    #[test]
    fn assessment_types_have_one_closed_wire_registry() {
        let wire_values = AssessmentType::ALL.map(AssessmentType::as_str);
        assert_eq!(
            wire_values,
            [
                "regular_assignment",
                "practice_question_assignment",
                "bonus_assignment",
                "quiz",
                "exam",
            ]
        );
        for (assessment_type, wire_value) in AssessmentType::ALL.into_iter().zip(wire_values) {
            assert_eq!(AssessmentType::parse(wire_value), Some(assessment_type));
            assert_eq!(
                serde_json::to_string(&assessment_type).expect("Assessment Type serializes"),
                format!("\"{wire_value}\"")
            );
        }
        assert!(AssessmentType::parse("assignment").is_none());
        assert!(serde_json::from_str::<AssessmentType>("\"assignment\"").is_err());
    }

    #[test]
    fn assessment_edit_numbers_are_distinct_canonical_assessment_preconditions() {
        let edit: AssessmentEditNumber = "43".parse().expect("canonical edit number");
        assert_eq!(serde_json::json!(edit), serde_json::json!("43"));
        assert_eq!(
            edit.checked_next().map(|value| value.to_string()),
            Some("44".into())
        );
        for invalid in ["", "0", "01", "+2", "-2", "9223372036854775808"] {
            assert!(
                invalid.parse::<AssessmentEditNumber>().is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn assessment_instructions_are_transparent_and_validated() {
        let empty = AssessmentInstructions::try_new(String::new()).expect("empty instructions");
        assert_eq!(serde_json::to_string(&empty).expect("serialize"), "\"\"");

        let instructions =
            AssessmentInstructions::try_new("Read each prompt carefully.".to_string())
                .expect("valid instructions");
        let json = serde_json::to_string(&instructions).expect("serialize");
        let decoded: AssessmentInstructions = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, instructions);
        assert_eq!(
            AssessmentInstructions::try_new("invalid\0text".to_string()),
            Err(AssessmentInstructionsError::ContainsNul)
        );
    }

    #[test]
    fn assessment_title_is_transparent_and_validated() {
        let title = AssessmentTitle::try_new("Protein folding".to_string()).expect("valid title");
        assert_eq!(
            serde_json::to_string(&title).expect("serialize"),
            "\"Protein folding\""
        );
        assert!(serde_json::from_str::<AssessmentTitle>("\"   \"").is_err());
        assert_eq!(
            AssessmentTitle::try_new("   ".to_string()),
            Err(AssessmentTitleError::Blank)
        );
        assert_eq!(
            AssessmentTitle::try_new("a".repeat(MAX_ASSESSMENT_TITLE_UNICODE_SCALARS + 1)),
            Err(AssessmentTitleError::TooLong)
        );
    }

    #[test]
    fn assessment_instructions_reject_excessive_unicode_scalars() {
        let too_long = "a".repeat(MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS + 1);
        assert_eq!(
            AssessmentInstructions::try_new(too_long),
            Err(AssessmentInstructionsError::TooLong)
        );
        assert!(
            serde_json::from_value::<AssessmentInstructions>(serde_json::json!("\u{0000}"))
                .is_err()
        );
    }

    #[test]
    fn assessment_authored_content_is_strict_and_uses_direct_cutover_defaults() {
        assert_eq!(
            AssessmentAuthoredContent::default(),
            AssessmentAuthoredContent {
                instructions: AssessmentInstructions::default(),
                base_policy: BaseAssessmentPolicy {
                    late_work_rule: LateWorkRule::Reject,
                    ..BaseAssessmentPolicy::default()
                },
                activity_rules: AssessmentActivityRules::default(),
            }
        );
        assert_eq!(
            serde_json::to_value(LateWorkRule::MarkLate).expect("late-work rule serializes"),
            serde_json::json!("mark_late")
        );
        assert!(serde_json::from_value::<LateWorkRule>(serde_json::json!("markLate")).is_err());
        assert!(
            serde_json::from_value::<AssessmentAuthoredContent>(serde_json::json!({
                "instructions": "",
                "basePolicy": {
                    "available_at": null,
                    "due_at": null,
                    "closes_at": null,
                    "assessment_attempt_time_limit_seconds": null,
                    "attempt_limit": null,
                    "late_work_rule": "accept"
                },
                "activityRules": {
                    "questionVariationRule": "newVariation",
                    "assessmentQuestionOrderRule": "authoredOrder",
                    "unexpected": true
                }
            }))
            .is_err()
        );
    }

    #[test]
    fn local_assessment_authored_content_is_strict_and_validates_bounds() {
        assert!(LocalDateAndTime::parse("2026-09-01T10:04").is_err());
        assert!(LocalDateAndTime::parse("2026-09-01T10:04:05.12").is_err());
        assert_eq!(
            InstructorAssessmentAuthoredContentLocal::new(
                AssessmentInstructions::default(),
                Some(local("2026-09-01T10:05:00.000")),
                Some(local("2026-09-01T10:04:00.000")),
                None,
                None,
                None,
                LateWorkRule::Accept,
            ),
            Err(AssessmentAuthoredContentLocalError::ScheduleOutOfOrder)
        );
        assert_eq!(
            InstructorAssessmentAuthoredContentLocal::new(
                AssessmentInstructions::default(),
                None,
                None,
                None,
                None,
                NonZeroU32::new(MAX_ASSESSMENT_ATTEMPT_LIMIT + 1),
                LateWorkRule::Accept,
            ),
            Err(AssessmentAuthoredContentLocalError::AttemptLimitOutOfRange)
        );
        assert!(
            serde_json::from_value::<InstructorAssessmentAuthoredContentLocal>(serde_json::json!({
                "instructions": "",
                "available_at": null,
                "due_at": null,
                "closes_at": null,
                "assessment_attempt_time_limit_seconds": 0,
                "attempt_limit": null,
                "late_work_rule": "accept"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<InstructorAssessmentAuthoredContentLocal>(serde_json::json!({
                "timeZone": "UTC",
                "instructions": "",
                "available_at": null,
                "due_at": null,
                "closes_at": null,
                "assessment_attempt_time_limit_seconds": null,
                "attempt_limit": null,
                "late_work_rule": "accept",
                "unexpected": true
            }))
            .is_err()
        );
    }

    #[test]
    fn instructor_assessment_availability_uses_authoritative_time_at_exact_boundaries() {
        let term = course_term("UTC");
        let account_time_zone = crate::AccountTimeZone::parse("UTC").expect("Account zone");
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
        let settings = AssessmentAuthoredContent {
            instructions: AssessmentInstructions::default(),
            base_policy: BaseAssessmentPolicy {
                available_at: Some(available),
                closes_at: Some(closes),
                ..BaseAssessmentPolicy::default()
            },
            activity_rules: AssessmentActivityRules::default(),
        };

        assert_eq!(
            derive_instructor_assessment_availability(
                &term,
                &account_time_zone,
                AssessmentStatus::Released,
                &settings,
                Timestamp::from_unix_millis(available.as_unix_millis() - 1),
            )
            .expect("scheduled state"),
            InstructorAssessmentAvailabilityView::Scheduled {
                available_at: local("2026-09-01T10:00:00.000"),
            }
        );
        assert_eq!(
            derive_instructor_assessment_availability(
                &term,
                &account_time_zone,
                AssessmentStatus::Released,
                &settings,
                available,
            )
            .expect("open state"),
            InstructorAssessmentAvailabilityView::Available
        );
        assert_eq!(
            derive_instructor_assessment_availability(
                &term,
                &account_time_zone,
                AssessmentStatus::Released,
                &settings,
                closes,
            )
            .expect("closed state"),
            InstructorAssessmentAvailabilityView::Closed {
                closed_at: Some(local("2026-09-01T12:00:00.000")),
            }
        );
    }

    #[test]
    fn instructor_assessment_availability_honors_due_rejection_and_stored_intent() {
        let term = course_term("UTC");
        let account_time_zone = crate::AccountTimeZone::parse("UTC").expect("Account zone");
        let due = Timestamp::from_unix_millis(
            Utc.with_ymd_and_hms(2026, 9, 1, 11, 0, 0)
                .single()
                .expect("valid time")
                .timestamp_millis(),
        );
        let settings = AssessmentAuthoredContent {
            instructions: AssessmentInstructions::default(),
            base_policy: BaseAssessmentPolicy {
                due_at: Some(due),
                late_work_rule: LateWorkRule::Reject,
                ..BaseAssessmentPolicy::default()
            },
            activity_rules: AssessmentActivityRules::default(),
        };
        assert_eq!(
            derive_instructor_assessment_availability(
                &term,
                &account_time_zone,
                AssessmentStatus::Released,
                &settings,
                due,
            )
            .expect("due-date closure"),
            InstructorAssessmentAvailabilityView::Closed {
                closed_at: Some(local("2026-09-01T11:00:00.000")),
            }
        );

        for (status, expected) in [
            (
                AssessmentStatus::Unreleased,
                InstructorAssessmentAvailabilityView::Unreleased,
            ),
            (
                AssessmentStatus::Closed,
                InstructorAssessmentAvailabilityView::Closed { closed_at: None },
            ),
            (
                AssessmentStatus::Archived,
                InstructorAssessmentAvailabilityView::Archived,
            ),
        ] {
            assert_eq!(
                derive_instructor_assessment_availability(
                    &term,
                    &account_time_zone,
                    status,
                    &settings,
                    due
                )
                .expect("stored assessment status"),
                expected
            );
        }
    }
}
