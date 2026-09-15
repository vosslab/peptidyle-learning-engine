//! Assessment Attempt Continuation eligibility and shared Assessment Activity errors.
//!
//! Every function is pure. A caller supplies question state, policy, and
//! summary state; this module reads no clock and performs no storage. The
//! completion and scoring modules own those transitions; this module exposes
//! their Assessment Activity composition surface.

use question_model::{AssessmentAttemptContinuationRule, AssessmentProgressRecord};

pub use crate::completion::{RequiredQuestionState, derive_within_assessment_attempt_completion};
pub use question_model::AssessmentAttemptCompletion;

pub use crate::scoring::{AssessmentActivityTransition, project_assessment_activity};

/// A rejected Assessment Attempt model input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssessmentActivityError {
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

impl std::fmt::Display for AssessmentActivityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCompletionThreshold { fraction } => {
                write!(formatter, "invalid completion threshold: {fraction}")
            }
            Self::InvalidScore { score } => {
                write!(formatter, "invalid Assessment Attempt score: {score}")
            }
            Self::InvalidQuestionPoints => {
                write!(formatter, "question points cannot form a score fraction")
            }
            Self::SummaryCounterOverflow => write!(formatter, "summary counter overflow"),
        }
    }
}

impl std::error::Error for AssessmentActivityError {}

/// Whether the Assessment Attempt Continuation Rule permits another Assessment Attempt.
///
/// Before the first completion, the continuation rule does not apply.
/// A cap counts only Assessment Attempts after the first completed Assessment Attempt.
pub fn assessment_attempt_continuation_allows_assessment_attempt(
    summary: &AssessmentProgressRecord,
    rule: AssessmentAttemptContinuationRule,
) -> bool {
    if summary.completed_assessment_attempt_count == 0 {
        return true;
    }

    match rule {
        AssessmentAttemptContinuationRule::Unlimited => true,
        AssessmentAttemptContinuationRule::Capped {
            max_additional_assessment_attempts,
        } => {
            summary.completed_assessment_attempt_count.saturating_sub(1)
                < max_additional_assessment_attempts
        }
        AssessmentAttemptContinuationRule::Closed => false,
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
    use question_model::{AssessmentId, AssessmentProgressRecord, StudentRecordId};
    use uuid::Uuid;

    fn empty_summary() -> AssessmentProgressRecord {
        AssessmentProgressRecord::empty(
            StudentRecordId::from_uuid(Uuid::from_u128(2)),
            AssessmentId::from_uuid(Uuid::from_u128(3)),
        )
    }

    #[test]
    fn a_practice_cap_counts_assessment_attempts_after_first_completion() {
        let mut summary = empty_summary();
        summary.completed_assessment_attempt_count = 3;

        assert!(!assessment_attempt_continuation_allows_assessment_attempt(
            &summary,
            AssessmentAttemptContinuationRule::Capped {
                max_additional_assessment_attempts: 2,
            }
        ));
    }
}
