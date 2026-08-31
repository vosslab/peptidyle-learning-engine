//! Assignment Attempt Continuation eligibility and shared Assignment Activity errors.
//!
//! Every function is pure. A caller supplies question state, policy, and
//! summary state; this module reads no clock and performs no storage. MOD-STATE
//! and MOD-SCORE own completion and scoring; this module exposes their
//! Assignment Activity composition surface.

use question_model::{AssignmentAttemptContinuationRule, AssignmentProgressRecord};

pub use crate::completion::{RequiredQuestionState, derive_within_assignment_attempt_completion};
pub use question_model::AssignmentAttemptCompletion;

pub use crate::scoring::{AssignmentActivityTransition, project_summary};

/// A rejected Assignment Attempt model input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssignmentActivityError {
    /// A completion threshold was non-finite or outside `0.0..=1.0`.
    InvalidCompletionThreshold {
        /// Rejected threshold.
        fraction: f64,
    },
    /// A score was non-finite or outside `-1000.0..=1000.0`.
    InvalidScore {
        /// Rejected score.
        score: f64,
    },
    /// A question's earned and possible points could not form a score.
    InvalidQuestionPoints,
    /// A summary counter reached its numeric limit.
    SummaryCounterOverflow,
}

impl std::fmt::Display for AssignmentActivityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCompletionThreshold { fraction } => {
                write!(formatter, "invalid completion threshold: {fraction}")
            }
            Self::InvalidScore { score } => {
                write!(formatter, "invalid Assignment Attempt score: {score}")
            }
            Self::InvalidQuestionPoints => {
                write!(formatter, "question points cannot form a score fraction")
            }
            Self::SummaryCounterOverflow => write!(formatter, "summary counter overflow"),
        }
    }
}

impl std::error::Error for AssignmentActivityError {}

/// Whether the Assignment Attempt Continuation Rule permits another Assignment Attempt.
///
/// Before the first completion, the continuation rule does not apply.
/// A cap counts only Assignment Attempts after the first completed Assignment Attempt.
pub fn assignment_attempt_continuation_allows_assignment_attempt(
    summary: &AssignmentProgressRecord,
    rule: AssignmentAttemptContinuationRule,
) -> bool {
    if summary.completed_assignment_attempt_count == 0 {
        return true;
    }

    match rule {
        AssignmentAttemptContinuationRule::Unlimited => true,
        AssignmentAttemptContinuationRule::Capped {
            max_additional_runs,
        } => summary.completed_assignment_attempt_count.saturating_sub(1) < max_additional_runs,
        AssignmentAttemptContinuationRule::Closed => false,
    }
}

/// Validates a value intended to be a fraction.
pub(crate) fn validate_fraction(value: f64) -> Result<(), ()> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{AssignmentId, AssignmentProgressRecord, StudentRecordId};
    use uuid::Uuid;

    fn empty_summary() -> AssignmentProgressRecord {
        AssignmentProgressRecord::empty(
            StudentRecordId::from_uuid(Uuid::from_u128(2)),
            AssignmentId::from_uuid(Uuid::from_u128(3)),
        )
    }

    #[test]
    fn a_practice_cap_counts_assignment_attempts_after_first_completion() {
        let mut summary = empty_summary();
        summary.completed_assignment_attempt_count = 3;

        assert!(!assignment_attempt_continuation_allows_assignment_attempt(
            &summary,
            AssignmentAttemptContinuationRule::Capped {
                max_additional_runs: 2,
            }
        ));
    }
}
