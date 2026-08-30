//! Continued-practice eligibility and shared run-model errors (MOD-RUN).
//!
//! Every function is pure. A caller supplies question state, policy, and
//! summary state; this module reads no clock and performs no storage. MOD-STATE
//! and MOD-SCORE own completion and scoring, with compatibility re-exports here
//! for the frozen WP-C3 contract.

use question_model::{ContinuedPractice, StudentAssignmentSummary};

// Compatibility path for WP-C3 consumers. MOD-STATE owns the implementation.
pub use crate::completion::{RequiredQuestionState, derive_within_run_completion};
pub use question_model::RunCompletionStatus;

// Compatibility path for WP-C3 consumers. MOD-SCORE owns the implementation.
pub use crate::scoring::{RunTransition, project_summary};

/// A rejected run-model input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunModelError {
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

impl std::fmt::Display for RunModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCompletionThreshold { fraction } => {
                write!(formatter, "invalid completion threshold: {fraction}")
            }
            Self::InvalidScore { score } => write!(formatter, "invalid run score: {score}"),
            Self::InvalidQuestionPoints => {
                write!(formatter, "question points cannot form a score fraction")
            }
            Self::SummaryCounterOverflow => write!(formatter, "summary counter overflow"),
        }
    }
}

impl std::error::Error for RunModelError {}

/// Whether policy permits starting another run from the current summary.
///
/// Before the first completion, the continued-practice policy does not apply.
/// A cap counts only runs after the first completed run.
pub fn continued_practice_allows_run(
    summary: &StudentAssignmentSummary,
    policy: ContinuedPractice,
) -> bool {
    if summary.completed_run_count == 0 {
        return true;
    }

    match policy {
        ContinuedPractice::Unlimited => true,
        ContinuedPractice::Capped {
            max_additional_runs,
        } => summary.completed_run_count.saturating_sub(1) < max_additional_runs,
        ContinuedPractice::Closed => false,
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
    use question_model::{EnrollmentId, StudentAssignmentSummary};
    use uuid::Uuid;

    fn empty_summary() -> StudentAssignmentSummary {
        StudentAssignmentSummary::empty(EnrollmentId::from_uuid(Uuid::from_u128(2)))
    }

    #[test]
    fn a_practice_cap_counts_runs_after_first_completion() {
        let mut summary = empty_summary();
        summary.completed_run_count = 3;

        assert!(!continued_practice_allows_run(
            &summary,
            ContinuedPractice::Capped {
                max_additional_runs: 2,
            }
        ));
    }
}
