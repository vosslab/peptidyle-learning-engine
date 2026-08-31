//! Browser-safe current assignment model.
//!
//! Stable item identities let instructors change points, scoring behavior,
//! and future ordering without rewriting immutable published content or
//! inventing assignment-history rows.

use std::num::NonZeroU32;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

mod revision;
mod teaching_settings_local;
pub use revision::{AssignmentRevision, AssignmentRevisionError};
pub use teaching_settings_local::{
    AssignmentTeachingSettingsFailureCode, AssignmentTeachingSettingsFailureReason,
    AssignmentTeachingSettingsField, AssignmentTeachingSettingsLocalError,
    AssignmentTeachingSettingsValidationFailure, CourseLocalDateTime, CourseLocalDateTimeError,
    InstructorAssignmentCurrentState, InstructorAssignmentTeachingSettingsLocal,
    derive_instructor_assignment_current_state,
};

use crate::{ActivityTimestamp, AssignmentItemId, AssignmentSelectionGroupId, QuestionVersionReference};

const POINT_SCALE: i64 = 10_000;
const MAX_WHOLE_POINTS: i64 = 1_000_000_000;
const MAX_SCALED_POINTS: i64 = MAX_WHOLE_POINTS * POINT_SCALE + (POINT_SCALE - 1);

/// Largest accepted assignment-instructions length, measured in Unicode scalars.
pub const MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS: usize = 50_000;

/// Maximum fixed-or-pool entries in one ordered assignment definition.
pub const MAX_ASSIGNMENT_ORDERED_ENTRIES: usize = 1_024;

/// Maximum candidate Question IDs in one assignment selection group.
pub const MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP: usize = 1_024;

/// Maximum candidate Question IDs across all selection groups in one assignment definition.
pub const MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES: usize = 8_192;

/// Instructor-controlled lifecycle intent for an assignment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentLifecycle {
    /// The assignment remains private to instructors.
    #[default]
    Draft,
    /// The assignment is eligible for student access, subject to all other gates.
    Published,
    /// The assignment is no longer open to new student work.
    Closed,
    /// The assignment is permanently retired from student access.
    Archived,
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
pub enum ScoringStatus {
    /// Current score rows match the assignment generation.
    Current,
    /// Stale rows are hidden while one idempotent recalculation runs.
    Recalculating,
    /// Recalculation failed and remains visibly retryable.
    Failed,
}

/// Whether work arriving after the ordinary due date remains acceptable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LateSubmissionPolicy {
    /// Accept work after the due date until another hard boundary closes it.
    Accept,
    /// Accept it but preserve the late condition for later reporting.
    MarkLate,
    /// Treat the due date as a hard submission boundary.
    Reject,
}

/// Closed behavior at an effective assignment deadline.
///
/// The accepted database plan deliberately chooses server auto-submit instead
/// of an unbounded overtime mode. Keeping that choice as an enum leaves a
/// deliberate extension point without accepting an unsupported boolean state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentDeadlineBehavior {
    /// The server closes active work independently of browser connectivity.
    AutoSubmit,
}

/// Serializable assignment-owned inputs to effective-policy resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BaseAssignmentPolicy {
    /// First instant when the assignment may be opened.
    pub available_at: Option<ActivityTimestamp>,
    /// Ordinary due instant.
    pub due_at: Option<ActivityTimestamp>,
    /// Hard instant after which new work is closed.
    pub closes_at: Option<ActivityTimestamp>,
    /// Whole-run time limit when one applies.
    pub time_limit_seconds: Option<NonZeroU32>,
    /// Maximum number of runs when one applies.
    pub attempt_limit: Option<NonZeroU32>,
    /// Treatment of work after the ordinary due instant.
    pub late_submission: LateSubmissionPolicy,
    /// Server behavior at an effective assignment deadline.
    pub deadline_behavior: AssignmentDeadlineBehavior,
}

impl Default for BaseAssignmentPolicy {
    fn default() -> Self {
        Self {
            available_at: None,
            due_at: None,
            closes_at: None,
            time_limit_seconds: None,
            attempt_limit: None,
            late_submission: LateSubmissionPolicy::Accept,
            deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
        }
    }
}

/// Shared instructor-facing assignment teaching settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentTeachingSettings {
    /// Instructor-controlled assignment lifecycle intent.
    pub lifecycle: AssignmentLifecycle,
    /// Validated student-facing plain-text instructions.
    pub instructions: AssignmentInstructions,
    /// Base policy supplied to the effective-policy resolver.
    pub base_policy: BaseAssignmentPolicy,
}

impl Default for AssignmentTeachingSettings {
    fn default() -> Self {
        Self {
            lifecycle: AssignmentLifecycle::Draft,
            instructions: AssignmentInstructions::default(),
            base_policy: BaseAssignmentPolicy::default(),
        }
    }
}

/// Largest whole-run limit representable by the current PostgreSQL `INTEGER`
/// columns. Keeping this public makes every browser and storage boundary share
/// the same lossless domain without a needless `BIGINT` migration.
pub const MAX_ASSIGNMENT_TIME_LIMIT_SECONDS: u32 = 2_147_483_647;

/// Largest attempt limit representable by the normalized PostgreSQL `INTEGER`
/// policy column. Keeping it separate from the time-limit name prevents a
/// browser-only value that the PostgreSQL implementation cannot persist.
pub const MAX_ASSIGNMENT_ATTEMPT_LIMIT: u32 = 2_147_483_647;

/// Exact nonnegative point value with four decimal places of precision.
///
/// JSON represents this as a decimal string so JavaScript cannot silently
/// round an instructor-authored value before it reaches PostgreSQL `NUMERIC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PointValue(i64);

impl PointValue {
    /// Zero points, used for remediation without retiring the item.
    pub const ZERO: Self = Self(0);

    /// Rebuilds an exact point value from its fixed four-decimal-place integer.
    pub fn from_scaled(value: i64) -> Option<Self> {
        (0..=MAX_SCALED_POINTS)
            .contains(&value)
            .then_some(Self(value))
    }

    /// Builds an exact whole-number point value.
    pub fn from_whole(value: u32) -> Self {
        Self(i64::from(value) * POINT_SCALE)
    }

    /// Returns the fixed four-decimal-place storage integer.
    pub fn scaled(self) -> i64 {
        self.0
    }

    /// Adds two exact point values when their sum remains representable.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).and_then(Self::from_scaled)
    }

    /// Multiplies an exact point value by a nonnegative item count.
    pub fn checked_mul_u32(self, multiplier: u32) -> Option<Self> {
        self.0
            .checked_mul(i64::from(multiplier))
            .and_then(Self::from_scaled)
    }
}

impl std::fmt::Display for PointValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let whole = self.0 / POINT_SCALE;
        let fractional = self.0 % POINT_SCALE;
        if fractional == 0 {
            write!(formatter, "{whole}")
        } else {
            let value = format!("{whole}.{fractional:04}");
            formatter.write_str(value.trim_end_matches('0'))
        }
    }
}

impl FromStr for PointValue {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() || value.starts_with('-') || value.starts_with('+') {
            return Err("points must be a nonnegative decimal value");
        }
        let (whole, fractional) = value.split_once('.').unwrap_or((value, ""));
        if whole.is_empty()
            || whole.len() > 10
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
            || fractional.len() > 4
            || !fractional.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("points must have at most four decimal places");
        }
        let whole = whole
            .parse::<i64>()
            .map_err(|_| "points are outside the supported range")?;
        if whole > MAX_WHOLE_POINTS {
            return Err("points are outside the supported range");
        }
        let mut fraction = fractional.to_string();
        while fraction.len() < 4 {
            fraction.push('0');
        }
        let fraction = if fraction.is_empty() {
            0
        } else {
            fraction
                .parse::<i64>()
                .map_err(|_| "points are outside the supported range")?
        };
        whole
            .checked_mul(POINT_SCALE)
            .and_then(|scaled| scaled.checked_add(fraction))
            .and_then(Self::from_scaled)
            .ok_or("points are outside the supported range")
    }
}

impl TryFrom<String> for PointValue {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<PointValue> for String {
    fn from(value: PointValue) -> Self {
        value.to_string()
    }
}

/// Whether an item remains deliverable in future runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentDeliveryState {
    /// Deliver and score this item normally according to its scoring mode.
    Active,
    /// Referential tombstone retained for protected existing responses.
    Retired,
}

/// Current scoring treatment for one stable assignment item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentScoringMode {
    /// Multiply normalized credit by current points and include the denominator.
    Normal,
    /// Award full current points regardless of normalized credit.
    FullCredit,
    /// Add earned points without increasing the normal denominator.
    ExtraCredit,
    /// Contribute to neither numerator nor denominator.
    Excluded,
}

/// One ordered fixed question in the mutable current assignment definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentItem {
    /// Stable identity preserved across point and order changes.
    pub id: AssignmentItemId,
    /// Exact immutable catalog content pinned by this item.
    pub reference: QuestionVersionReference,
    /// Zero-based position used for future runs.
    pub position: u32,
    /// Current assignment-authored points.
    pub points_possible: PointValue,
    /// Whether future runs may receive the item.
    pub delivery_state: AssignmentDeliveryState,
    /// Current-only scoring treatment.
    pub scoring_mode: AssignmentScoringMode,
}

/// Ordering policy for candidates selected into a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionOrdering {
    /// Preserve candidate order after deterministic selection.
    CandidateOrder,
    /// Deterministically shuffle selected candidates from the run seed.
    Randomized,
}

/// Reviewed deterministic pool-draw implementation stored with a selection
/// group.
///
/// This closed value makes the persisted draw contract explicit. A later
/// algorithm may be added without changing how existing definitions or issued
/// run evidence are interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PoolDrawAlgorithm {
    /// The first reviewed candidate-ranking implementation.
    V1,
}

impl PoolDrawAlgorithm {
    /// Integer representation retained by the normalized PostgreSQL schema.
    pub const fn storage_version(self) -> u16 {
        match self {
            Self::V1 => 1,
        }
    }

    /// Decodes only algorithms the current server can execute.
    pub const fn from_storage_version(version: u16) -> Option<Self> {
        match version {
            1 => Some(Self::V1),
            _ => None,
        }
    }
}

/// One pinned candidate eligible for a random-selection group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentSelectionCandidate {
    /// Stable candidate identity used for retirement and audit actions.
    pub id: AssignmentItemId,
    /// Zero-based authored order within this selection group.
    pub position: u32,
    /// Exact immutable catalog version eligible for selection.
    pub reference: QuestionVersionReference,
    /// Whether future runs may select this candidate.
    pub delivery_state: AssignmentDeliveryState,
}

/// Current random-selection definition; run rows snapshot the actual result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentSelectionGroup {
    /// Stable group identity.
    pub id: AssignmentSelectionGroupId,
    /// Position of this group among fixed items and other groups.
    pub position: u32,
    /// Number of active candidates selected for each future run.
    pub draw_count: u32,
    /// Uniform current points for each selected candidate.
    pub points_per_item: PointValue,
    /// Output ordering after selection.
    pub ordering: SelectionOrdering,
    /// Closed reviewed algorithm needed to reproduce selection.
    pub algorithm: PoolDrawAlgorithm,
    /// Pinned candidate set; search criteria are deliberately absent.
    pub candidates: Vec<AssignmentSelectionCandidate>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CourseTerm, IanaTimeZone};
    use chrono::{TimeZone, Utc};

    fn course_term(time_zone: &str) -> CourseTerm {
        CourseTerm::from_parts("2026-01-01", "2026-12-31", time_zone).expect("valid course term")
    }

    fn local(value: &str) -> CourseLocalDateTime {
        CourseLocalDateTime::parse(value).expect("valid local wall-clock value")
    }

    fn local_settings(
        time_zone: &str,
        available_at: Option<CourseLocalDateTime>,
        due_at: Option<CourseLocalDateTime>,
        closes_at: Option<CourseLocalDateTime>,
    ) -> InstructorAssignmentTeachingSettingsLocal {
        InstructorAssignmentTeachingSettingsLocal::new(
            IanaTimeZone::parse(time_zone).expect("known zone"),
            AssignmentLifecycle::Published,
            AssignmentInstructions::try_new("Read the diagram.".to_string())
                .expect("valid instructions"),
            available_at,
            due_at,
            closes_at,
            NonZeroU32::new(900),
            NonZeroU32::new(2),
            LateSubmissionPolicy::MarkLate,
            AssignmentDeadlineBehavior::AutoSubmit,
        )
        .expect("valid local settings")
    }

    #[test]
    fn point_values_round_trip_without_binary_floating_point() {
        for value in ["0", "1", "2.5", "100.125", "0.0001"] {
            let points: PointValue = value.parse().expect("valid exact points");
            let json = serde_json::to_string(&points).expect("points serialize");
            let decoded: PointValue = serde_json::from_str(&json).expect("points deserialize");
            assert_eq!(decoded, points);
            assert_eq!(decoded.to_string(), value);
        }
        for invalid in ["", "-1", "+1", "1.00001", "NaN", "1000000001"] {
            assert!(invalid.parse::<PointValue>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn pool_draw_algorithm_accepts_only_the_reviewed_storage_version() {
        assert_eq!(PoolDrawAlgorithm::V1.storage_version(), 1);
        assert_eq!(PoolDrawAlgorithm::from_storage_version(2), None);
    }

    #[test]
    fn point_values_add_and_multiply_exact_scaled_values() {
        let one_and_a_half = PointValue::from_scaled(15_000).expect("in range");
        let quarter = PointValue::from_scaled(2_500).expect("in range");

        assert_eq!(
            one_and_a_half.checked_add(quarter).map(PointValue::scaled),
            Some(17_500)
        );
        assert_eq!(
            quarter.checked_mul_u32(6).map(PointValue::scaled),
            Some(15_000)
        );
    }

    #[test]
    fn point_values_reject_negative_and_overflowing_scaled_arithmetic() {
        let maximum = PointValue::from_scaled(MAX_SCALED_POINTS).expect("maximum is valid");
        let smallest = PointValue::from_scaled(1).expect("smallest is valid");

        assert!(PointValue::from_scaled(-1).is_none());
        assert!(
            PointValue::from_scaled(MAX_SCALED_POINTS.checked_add(1).expect("i64 room")).is_none()
        );
        assert!(maximum.checked_add(smallest).is_none());
        assert!(maximum.checked_mul_u32(2).is_none());
    }

    #[test]
    fn assignment_time_limit_domain_matches_postgres_integer() {
        assert_eq!(MAX_ASSIGNMENT_TIME_LIMIT_SECONDS, 2_147_483_647);
        assert_eq!(MAX_ASSIGNMENT_ATTEMPT_LIMIT, 2_147_483_647);
        assert_eq!(MAX_ASSIGNMENT_ORDERED_ENTRIES, 1_024);
        assert_eq!(MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP, 1_024);
        assert_eq!(MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES, 8_192);
    }

    #[test]
    fn assignment_revisions_use_canonical_postgres_bigint_strings() {
        let revision: AssignmentRevision = "43".parse().expect("canonical revision");
        assert_eq!(serde_json::json!(revision), serde_json::json!("43"));
        assert_eq!(
            revision.checked_next().map(|value| value.to_string()),
            Some("44".into())
        );
        assert!(
            AssignmentRevision::new(i64::MAX as u64)
                .expect("maximum revision")
                .checked_next()
                .is_none()
        );
        for invalid in ["", "0", "01", "+2", "-2", "9223372036854775808"] {
            assert!(invalid.parse::<AssignmentRevision>().is_err(), "{invalid}");
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
    fn teaching_settings_are_strict_and_use_direct_cutover_defaults() {
        assert_eq!(
            AssignmentTeachingSettings::default(),
            AssignmentTeachingSettings {
                lifecycle: AssignmentLifecycle::Draft,
                instructions: AssignmentInstructions::default(),
                base_policy: BaseAssignmentPolicy {
                    late_submission: LateSubmissionPolicy::Accept,
                    deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
                    ..BaseAssignmentPolicy::default()
                },
            }
        );
        assert!(
            serde_json::from_value::<AssignmentTeachingSettings>(serde_json::json!({
                "lifecycle": "draft",
                "instructions": "",
                "basePolicy": {
                    "availableAt": null,
                    "dueAt": null,
                    "closesAt": null,
                    "timeLimitSeconds": null,
                    "attemptLimit": null,
                    "lateSubmission": "accept",
                    "deadlineBehavior": "autoSubmit",
                    "unexpected": true
                }
            }))
            .is_err()
        );
    }

    #[test]
    fn local_teaching_settings_round_trip_exact_milliseconds_in_utc_and_chicago() {
        let timestamp = ActivityTimestamp::from_unix_millis(
            Utc.with_ymd_and_hms(2026, 9, 1, 15, 4, 5)
                .single()
                .expect("valid UTC time")
                .timestamp_millis()
                + 123,
        );
        let settings = AssignmentTeachingSettings {
            lifecycle: AssignmentLifecycle::Published,
            instructions: AssignmentInstructions::try_new("Read the diagram.".to_string())
                .expect("valid instructions"),
            base_policy: BaseAssignmentPolicy {
                available_at: Some(timestamp),
                due_at: Some(timestamp),
                closes_at: Some(timestamp),
                time_limit_seconds: NonZeroU32::new(900),
                attempt_limit: NonZeroU32::new(2),
                late_submission: LateSubmissionPolicy::MarkLate,
                deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
            },
        };
        let utc = InstructorAssignmentTeachingSettingsLocal::from_absolute(
            &course_term("UTC"),
            &settings,
        )
        .expect("UTC projection");
        assert_eq!(
            utc.available_at.as_ref().map(CourseLocalDateTime::as_str),
            Some("2026-09-01T15:04:05.123")
        );
        assert_eq!(
            utc.into_absolute(&course_term("UTC"))
                .expect("UTC resolution"),
            settings
        );

        let chicago = InstructorAssignmentTeachingSettingsLocal::from_absolute(
            &course_term("America/Chicago"),
            &settings,
        )
        .expect("Chicago projection");
        assert_eq!(
            chicago
                .available_at
                .as_ref()
                .map(CourseLocalDateTime::as_str),
            Some("2026-09-01T10:04:05.123")
        );
        assert_eq!(
            chicago
                .into_absolute(&course_term("America/Chicago"))
                .expect("Chicago resolution"),
            settings
        );
    }

    #[test]
    fn local_teaching_settings_refuse_dst_gap_ambiguity_and_mismatch() {
        let term = course_term("America/Chicago");
        let gap = local_settings(
            "America/Chicago",
            Some(local("2026-03-08T02:30:00.000")),
            None,
            None,
        );
        assert_eq!(
            gap.into_absolute(&term),
            Err(AssignmentTeachingSettingsLocalError::NonexistentLocalTime(
                AssignmentTeachingSettingsField::AvailableAt
            ))
        );
        let ambiguity = local_settings(
            "America/Chicago",
            Some(local("2026-11-01T01:30:00.000")),
            None,
            None,
        );
        assert_eq!(
            ambiguity.into_absolute(&term),
            Err(AssignmentTeachingSettingsLocalError::AmbiguousLocalTime(
                AssignmentTeachingSettingsField::AvailableAt
            ))
        );
        let mismatch = local_settings("UTC", Some(local("2026-09-01T15:04:05.123")), None, None);
        assert_eq!(
            mismatch.into_absolute(&term),
            Err(AssignmentTeachingSettingsLocalError::CourseTimeZoneMismatch)
        );
    }

    #[test]
    fn local_teaching_settings_are_strict_and_validate_schedule_and_integer_bounds() {
        assert!(CourseLocalDateTime::parse("2026-09-01T10:04").is_err());
        assert!(CourseLocalDateTime::parse("2026-09-01T10:04:05.12").is_err());
        assert_eq!(
            InstructorAssignmentTeachingSettingsLocal::new(
                IanaTimeZone::parse("UTC").expect("known zone"),
                AssignmentLifecycle::Draft,
                AssignmentInstructions::default(),
                Some(local("2026-09-01T10:05:00.000")),
                Some(local("2026-09-01T10:04:00.000")),
                None,
                None,
                None,
                LateSubmissionPolicy::Accept,
                AssignmentDeadlineBehavior::AutoSubmit,
            ),
            Err(AssignmentTeachingSettingsLocalError::ScheduleOutOfOrder)
        );
        assert_eq!(
            InstructorAssignmentTeachingSettingsLocal::new(
                IanaTimeZone::parse("UTC").expect("known zone"),
                AssignmentLifecycle::Draft,
                AssignmentInstructions::default(),
                None,
                None,
                None,
                None,
                NonZeroU32::new(MAX_ASSIGNMENT_ATTEMPT_LIMIT + 1),
                LateSubmissionPolicy::Accept,
                AssignmentDeadlineBehavior::AutoSubmit,
            ),
            Err(AssignmentTeachingSettingsLocalError::AttemptLimitOutOfRange)
        );
        assert!(
            serde_json::from_value::<InstructorAssignmentTeachingSettingsLocal>(
                serde_json::json!({
                    "timeZone": "UTC",
                    "lifecycle": "draft",
                    "instructions": "",
                    "availableAt": null,
                    "dueAt": null,
                    "closesAt": null,
                    "timeLimitSeconds": 0,
                    "attemptLimit": null,
                    "lateSubmission": "accept",
                    "deadlineBehavior": "autoSubmit"
                })
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<InstructorAssignmentTeachingSettingsLocal>(
                serde_json::json!({
                    "timeZone": "UTC",
                    "lifecycle": "draft",
                    "instructions": "",
                    "availableAt": null,
                    "dueAt": null,
                    "closesAt": null,
                    "timeLimitSeconds": null,
                    "attemptLimit": null,
                    "lateSubmission": "accept",
                    "deadlineBehavior": "autoSubmit",
                    "unexpected": true
                })
            )
            .is_err()
        );
    }

    #[test]
    fn instructor_current_state_uses_authoritative_time_at_exact_boundaries() {
        let term = course_term("UTC");
        let available = ActivityTimestamp::from_unix_millis(
            Utc.with_ymd_and_hms(2026, 9, 1, 10, 0, 0)
                .single()
                .expect("valid time")
                .timestamp_millis(),
        );
        let closes = ActivityTimestamp::from_unix_millis(
            Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0)
                .single()
                .expect("valid time")
                .timestamp_millis(),
        );
        let settings = AssignmentTeachingSettings {
            lifecycle: AssignmentLifecycle::Published,
            instructions: AssignmentInstructions::default(),
            base_policy: BaseAssignmentPolicy {
                available_at: Some(available),
                closes_at: Some(closes),
                ..BaseAssignmentPolicy::default()
            },
        };

        assert_eq!(
            derive_instructor_assignment_current_state(
                &term,
                &settings,
                ActivityTimestamp::from_unix_millis(available.as_unix_millis() - 1),
            )
            .expect("scheduled state"),
            InstructorAssignmentCurrentState::Scheduled {
                available_at: local("2026-09-01T10:00:00.000"),
            }
        );
        assert_eq!(
            derive_instructor_assignment_current_state(&term, &settings, available)
                .expect("open state"),
            InstructorAssignmentCurrentState::Open
        );
        assert_eq!(
            derive_instructor_assignment_current_state(&term, &settings, closes)
                .expect("closed state"),
            InstructorAssignmentCurrentState::Closed {
                closed_at: Some(local("2026-09-01T12:00:00.000")),
            }
        );
    }

    #[test]
    fn instructor_current_state_honors_due_rejection_and_stored_intent() {
        let term = course_term("UTC");
        let due = ActivityTimestamp::from_unix_millis(
            Utc.with_ymd_and_hms(2026, 9, 1, 11, 0, 0)
                .single()
                .expect("valid time")
                .timestamp_millis(),
        );
        let mut settings = AssignmentTeachingSettings {
            lifecycle: AssignmentLifecycle::Published,
            instructions: AssignmentInstructions::default(),
            base_policy: BaseAssignmentPolicy {
                due_at: Some(due),
                late_submission: LateSubmissionPolicy::Reject,
                ..BaseAssignmentPolicy::default()
            },
        };
        assert_eq!(
            derive_instructor_assignment_current_state(&term, &settings, due)
                .expect("due-date closure"),
            InstructorAssignmentCurrentState::Closed {
                closed_at: Some(local("2026-09-01T11:00:00.000")),
            }
        );

        for (lifecycle, expected) in [
            (
                AssignmentLifecycle::Draft,
                InstructorAssignmentCurrentState::Draft,
            ),
            (
                AssignmentLifecycle::Closed,
                InstructorAssignmentCurrentState::Closed { closed_at: None },
            ),
            (
                AssignmentLifecycle::Archived,
                InstructorAssignmentCurrentState::Archived,
            ),
        ] {
            settings.lifecycle = lifecycle;
            assert_eq!(
                derive_instructor_assignment_current_state(&term, &settings, due)
                    .expect("stored lifecycle state"),
                expected
            );
        }
    }
}
