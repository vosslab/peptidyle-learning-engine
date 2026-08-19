//! Browser-safe current assignment model.
//!
//! Stable item identities let instructors change points, scoring behavior,
//! and future ordering without rewriting immutable published content or
//! inventing assignment-history rows.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{AssignmentItemId, AssignmentSelectionGroupId, ProblemVersionRef};

const POINT_SCALE: i64 = 10_000;
const MAX_WHOLE_POINTS: i64 = 1_000_000_000;

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

/// Editor-owned whole-run timer choice.
///
/// This deliberately contains only the one value that an assignment editor
/// may change in this release. Schedule, late-work, and accommodation policy
/// are resolved by the server-owned effective-policy workflow and are not
/// accidentally overwritten by an editor save.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentRunTiming {
    /// Whole-run limit in seconds, or no whole-run limit.
    pub time_limit_seconds: Option<u32>,
}

/// Default timed-Mastery choice used by the instructor editor.
///
/// This is a `u32` because it crosses the TypeScript boundary and every
/// JavaScript number in the accepted range remains exactly representable.
pub const DEFAULT_MASTERY_TIME_LIMIT_SECONDS: u32 = 900;

/// Largest whole-run limit representable by the current PostgreSQL `INTEGER`
/// columns. Keeping this public makes every browser and storage boundary share
/// the same lossless domain without a needless `BIGINT` migration.
pub const MAX_ASSIGNMENT_TIME_LIMIT_SECONDS: u32 = 2_147_483_647;

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

    /// Builds an exact whole-number point value.
    pub fn from_whole(value: u32) -> Self {
        Self(i64::from(value) * POINT_SCALE)
    }

    /// Returns the fixed four-decimal-place storage integer.
    pub fn scaled(self) -> i64 {
        self.0
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
            .map(Self)
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
    pub reference: ProblemVersionRef,
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

/// One pinned candidate eligible for a random-selection group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentSelectionCandidate {
    /// Stable candidate identity used for retirement and audit actions.
    pub id: AssignmentItemId,
    /// Zero-based authored order within this selection group.
    pub position: u32,
    /// Exact immutable catalog version eligible for selection.
    pub reference: ProblemVersionRef,
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
    /// Stable algorithm version needed to reproduce selection.
    pub algorithm_version: u16,
    /// Pinned candidate set; search criteria are deliberately absent.
    pub candidates: Vec<AssignmentSelectionCandidate>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn assignment_time_limit_domain_matches_postgres_integer() {
        assert_eq!(MAX_ASSIGNMENT_TIME_LIMIT_SECONDS, 2_147_483_647);
    }
}
