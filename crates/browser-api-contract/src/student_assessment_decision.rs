//! Browser-safe server decision for one released Student Assessment.

use question_model::{AccountTimeZone, LateWorkRule, Timestamp};
use serde::{Deserialize, Serialize};

/// The current server-calculated ability to start one released Assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStartDecision {
    /// The exact active Student Record may start now.
    MayStart,
    /// The released Assessment has not reached its availability time.
    NotYetAvailable,
    /// The Assessment is not available for a new Attempt.
    Closed,
    /// The Student has already used every allowed completed Attempt.
    AttemptLimitReached,
    /// The released late-work policy refuses a new Attempt.
    LateWorkRefused,
}

/// Complete Student-visible policy and server decision for one Assessment.
///
/// Instants are Unix milliseconds from PostgreSQL's authoritative clock. The
/// browser formats these values in `display_time_zone`; it never recomputes
/// `start_decision` from its own clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentAssessmentDecisionSummary {
    pub available_at: Option<Timestamp>,
    pub due_at: Option<Timestamp>,
    pub closes_at: Option<Timestamp>,
    pub time_limit_seconds: Option<u32>,
    pub attempt_limit: Option<u32>,
    pub late_work_rule: LateWorkRule,
    pub display_time_zone: AccountTimeZone,
    pub evaluated_at: Timestamp,
    pub start_decision: AssessmentStartDecision,
    /// `None` only when `start_decision` is `may_start`.
    pub public_reason: Option<String>,
}
