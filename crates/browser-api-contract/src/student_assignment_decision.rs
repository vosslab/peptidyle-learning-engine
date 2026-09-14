//! Browser-safe server decision for one released Student Assignment.

use question_model::{AccountTimeZone, LateWorkRule, Timestamp};
use serde::{Deserialize, Serialize};

/// The current server-calculated ability to start one released Assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStartDecision {
    /// The exact active Student Record may start now.
    MayStart,
    /// The released Assignment has not reached its availability time.
    NotYetAvailable,
    /// The Assignment is not available for a new Attempt.
    Closed,
    /// The Student has already used every allowed completed Attempt.
    AttemptLimitReached,
    /// The released late-work policy refuses a new Attempt.
    LateWorkRefused,
}

/// Complete Student-visible policy and server decision for one Assignment.
///
/// Instants are Unix milliseconds from PostgreSQL's authoritative clock. The
/// browser formats these values in `display_time_zone`; it never recomputes
/// `start_decision` from its own clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentAssignmentDecisionSummary {
    pub available_at: Option<Timestamp>,
    pub due_at: Option<Timestamp>,
    pub closes_at: Option<Timestamp>,
    pub time_limit_seconds: Option<u32>,
    pub attempt_limit: Option<u32>,
    pub late_work_rule: LateWorkRule,
    pub display_time_zone: AccountTimeZone,
    pub evaluated_at: Timestamp,
    pub start_decision: AssignmentStartDecision,
    /// `None` only when `start_decision` is `may_start`.
    pub public_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_utc_millis_and_closed_decision_fields() {
        let summary = StudentAssignmentDecisionSummary {
            available_at: Some(Timestamp::from_unix_millis(1_000)),
            due_at: Some(Timestamp::from_unix_millis(2_000)),
            closes_at: Some(Timestamp::from_unix_millis(3_000)),
            time_limit_seconds: Some(900),
            attempt_limit: Some(2),
            late_work_rule: LateWorkRule::Reject,
            display_time_zone: AccountTimeZone::parse("America/Chicago").expect("time zone"),
            evaluated_at: Timestamp::from_unix_millis(1_500),
            start_decision: AssignmentStartDecision::MayStart,
            public_reason: None,
        };

        let wire = serde_json::to_value(summary).expect("decision summary serializes");
        assert_eq!(wire["availableAt"], 1_000);
        assert_eq!(wire["displayTimeZone"], "America/Chicago");
        assert_eq!(wire["startDecision"], "may_start");
        assert!(wire["publicReason"].is_null());
    }
}
