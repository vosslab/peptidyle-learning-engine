//! Server-authoritative timer verdicts (MOD-TIME).
//!
//! The caller supplies timestamps recorded from the server clock. This module
//! never reads a clock, so browser clock skew cannot change the verdict.
//! Authorized pauses arrive as one cumulative extension reconstructed from
//! audit events; pause authorization and persistence belong to the server.

use question_model::assignment_activity_rules::QuestionAttemptTimeLimit;
use question_model::{QuestionAttemptTiming, Timestamp};
use serde::{Deserialize, Serialize};

/// Complete clock-free input to one timer evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttemptTimingEvaluation {
    /// Authored timing and grace policy for the Question or Assignment Attempt.
    pub policy: QuestionAttemptTimeLimit,
    /// Server-recorded issue, base-deadline, and submission timestamps.
    pub timer: QuestionAttemptTiming,
    /// Server time at which an unsubmitted timer is being evaluated.
    pub evaluated_at: Timestamp,
    /// Total authorized pause duration added to the base deadline.
    pub pause_extension_millis: i64,
}

/// Authoritative result of evaluating one timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionAttemptTimingDecision {
    /// No deadline applies.
    Untimed,
    /// The unsubmitted timer has not reached its effective deadline.
    Open,
    /// The deadline passed, but the server still accepts an in-flight response.
    GracePeriod,
    /// The response arrived no later than the effective deadline.
    SubmittedOnTime,
    /// The response arrived after the deadline but within the inclusive grace window.
    SubmittedWithinGrace,
    /// No acceptable response arrived before the grace window closed.
    TimedOut,
}

/// Malformed or internally inconsistent timer input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionAttemptTimingEvaluationError {
    /// An untimed policy carried a deadline.
    UnexpectedDeadline,
    /// A timed policy omitted its deadline.
    MissingDeadline,
    /// An untimed policy carried a pause extension.
    PauseOnUntimed,
    /// A pause duration cannot be negative.
    NegativePauseExtension,
    /// The deadline predates the timer's issue timestamp.
    DeadlineBeforeIssue,
    /// The evaluation predates the timer's issue timestamp.
    EvaluationBeforeIssue,
    /// The submission predates the timer's issue timestamp.
    SubmissionBeforeIssue,
    /// The supplied evaluation predates the recorded submission.
    SubmissionAfterEvaluation,
    /// Applying pause or grace duration exceeded the timestamp range.
    TimestampOverflow,
}

impl std::fmt::Display for QuestionAttemptTimingEvaluationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::UnexpectedDeadline => "an untimed policy cannot carry a deadline",
            Self::MissingDeadline => "a timed policy requires a server deadline",
            Self::PauseOnUntimed => "an untimed policy cannot carry a pause extension",
            Self::NegativePauseExtension => "a pause extension cannot be negative",
            Self::DeadlineBeforeIssue => "the timer deadline predates issue",
            Self::EvaluationBeforeIssue => "the timer evaluation predates issue",
            Self::SubmissionBeforeIssue => "the timer submission predates issue",
            Self::SubmissionAfterEvaluation => "the timer submission follows evaluation",
            Self::TimestampOverflow => "the effective timer deadline overflowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for QuestionAttemptTimingEvaluationError {}

/// Evaluates a timer using only server-owned input.
///
/// Deadline and grace boundaries are inclusive. `GracePeriod` exists so the
/// server can wait for an already-sent response after the browser display has
/// expired; it is not extra student working time. The API should submit at the
/// base deadline and treat this function's result as authoritative.
///
/// # Errors
///
/// Returns [`QuestionAttemptTimingEvaluationError`] for inconsistent timestamps, policy and
/// deadline mismatches, negative pause duration, or arithmetic overflow.
pub fn question_attempt_timing_decision(
    evaluation: &QuestionAttemptTimingEvaluation,
) -> Result<QuestionAttemptTimingDecision, QuestionAttemptTimingEvaluationError> {
    validate_record_order(evaluation)?;

    let grace_seconds = match evaluation.policy {
        QuestionAttemptTimeLimit::Unlimited => {
            if evaluation.timer.deadline.is_some() {
                return Err(QuestionAttemptTimingEvaluationError::UnexpectedDeadline);
            }
            if evaluation.pause_extension_millis != 0 {
                return Err(QuestionAttemptTimingEvaluationError::PauseOnUntimed);
            }
            return Ok(QuestionAttemptTimingDecision::Untimed);
        }
        QuestionAttemptTimeLimit::Limited { grace_seconds, .. } => grace_seconds,
    };

    let deadline = evaluation
        .timer
        .deadline
        .ok_or(QuestionAttemptTimingEvaluationError::MissingDeadline)?;
    if deadline < evaluation.timer.issued_at {
        return Err(QuestionAttemptTimingEvaluationError::DeadlineBeforeIssue);
    }

    let effective_deadline = checked_add_millis(deadline, evaluation.pause_extension_millis)?;
    let grace_millis = i64::from(grace_seconds) * 1_000;
    let grace_deadline = checked_add_millis(effective_deadline, grace_millis)?;
    let observed_at = evaluation
        .timer
        .submitted_at
        .unwrap_or(evaluation.evaluated_at);

    if observed_at <= effective_deadline {
        return Ok(if evaluation.timer.submitted_at.is_some() {
            QuestionAttemptTimingDecision::SubmittedOnTime
        } else {
            QuestionAttemptTimingDecision::Open
        });
    }
    if observed_at <= grace_deadline {
        return Ok(if evaluation.timer.submitted_at.is_some() {
            QuestionAttemptTimingDecision::SubmittedWithinGrace
        } else {
            QuestionAttemptTimingDecision::GracePeriod
        });
    }
    Ok(QuestionAttemptTimingDecision::TimedOut)
}

fn validate_record_order(
    evaluation: &QuestionAttemptTimingEvaluation,
) -> Result<(), QuestionAttemptTimingEvaluationError> {
    if evaluation.pause_extension_millis < 0 {
        return Err(QuestionAttemptTimingEvaluationError::NegativePauseExtension);
    }
    if evaluation.evaluated_at < evaluation.timer.issued_at {
        return Err(QuestionAttemptTimingEvaluationError::EvaluationBeforeIssue);
    }
    if let Some(submitted_at) = evaluation.timer.submitted_at {
        if submitted_at < evaluation.timer.issued_at {
            return Err(QuestionAttemptTimingEvaluationError::SubmissionBeforeIssue);
        }
        if submitted_at > evaluation.evaluated_at {
            return Err(QuestionAttemptTimingEvaluationError::SubmissionAfterEvaluation);
        }
    }
    Ok(())
}

fn checked_add_millis(
    timestamp: Timestamp,
    milliseconds: i64,
) -> Result<Timestamp, QuestionAttemptTimingEvaluationError> {
    timestamp
        .as_unix_millis()
        .checked_add(milliseconds)
        .map(Timestamp::from_unix_millis)
        .ok_or(QuestionAttemptTimingEvaluationError::TimestampOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(milliseconds: i64) -> Timestamp {
        Timestamp::from_unix_millis(milliseconds)
    }

    fn evaluation(
        policy: QuestionAttemptTimeLimit,
        deadline: Option<i64>,
        submitted_at: Option<i64>,
        evaluated_at: i64,
        pause_extension_millis: i64,
    ) -> QuestionAttemptTimingEvaluation {
        QuestionAttemptTimingEvaluation {
            policy,
            timer: QuestionAttemptTiming {
                issued_at: timestamp(1_000),
                deadline: deadline.map(timestamp),
                submitted_at: submitted_at.map(timestamp),
            },
            evaluated_at: timestamp(evaluated_at),
            pause_extension_millis,
        }
    }

    #[test]
    fn grace_and_pause_boundaries_are_table_driven() {
        let timed = QuestionAttemptTimeLimit::Limited {
            seconds: 9,
            grace_seconds: 2,
        };
        let cases = [
            (
                "untimed",
                evaluation(QuestionAttemptTimeLimit::Unlimited, None, None, 50_000, 0),
                QuestionAttemptTimingDecision::Untimed,
            ),
            (
                "open before deadline",
                evaluation(timed, Some(10_000), None, 9_999, 0),
                QuestionAttemptTimingDecision::Open,
            ),
            (
                "open at inclusive deadline",
                evaluation(timed, Some(10_000), None, 10_000, 0),
                QuestionAttemptTimingDecision::Open,
            ),
            (
                "waiting inside grace",
                evaluation(timed, Some(10_000), None, 10_001, 0),
                QuestionAttemptTimingDecision::GracePeriod,
            ),
            (
                "on-time submission at deadline",
                evaluation(timed, Some(10_000), Some(10_000), 10_000, 0),
                QuestionAttemptTimingDecision::SubmittedOnTime,
            ),
            (
                "submission inside grace",
                evaluation(timed, Some(10_000), Some(10_001), 10_001, 0),
                QuestionAttemptTimingDecision::SubmittedWithinGrace,
            ),
            (
                "submission at inclusive grace boundary",
                evaluation(timed, Some(10_000), Some(12_000), 12_000, 0),
                QuestionAttemptTimingDecision::SubmittedWithinGrace,
            ),
            (
                "submission after grace",
                evaluation(timed, Some(10_000), Some(12_001), 12_001, 0),
                QuestionAttemptTimingDecision::TimedOut,
            ),
            (
                "authorized pause extends on-time deadline",
                evaluation(timed, Some(10_000), Some(11_500), 11_500, 2_000),
                QuestionAttemptTimingDecision::SubmittedOnTime,
            ),
            (
                "authorized pause extends grace deadline",
                evaluation(timed, Some(10_000), Some(13_500), 13_500, 2_000),
                QuestionAttemptTimingDecision::SubmittedWithinGrace,
            ),
            (
                "per-attempt uses the same verdict rules",
                evaluation(
                    QuestionAttemptTimeLimit::Limited {
                        seconds: 60,
                        grace_seconds: 0,
                    },
                    Some(10_000),
                    None,
                    10_001,
                    0,
                ),
                QuestionAttemptTimingDecision::TimedOut,
            ),
        ];

        for (name, input, expected) in cases {
            assert_eq!(
                question_attempt_timing_decision(&input),
                Ok(expected),
                "{name}"
            );
        }
    }

    #[test]
    fn malformed_inputs_return_specific_errors() {
        let timed = QuestionAttemptTimeLimit::Limited {
            seconds: 9,
            grace_seconds: 2,
        };
        let cases = [
            (
                "timed without deadline",
                evaluation(timed, None, None, 2_000, 0),
                QuestionAttemptTimingEvaluationError::MissingDeadline,
            ),
            (
                "untimed with deadline",
                evaluation(
                    QuestionAttemptTimeLimit::Unlimited,
                    Some(2_000),
                    None,
                    2_000,
                    0,
                ),
                QuestionAttemptTimingEvaluationError::UnexpectedDeadline,
            ),
            (
                "untimed with pause",
                evaluation(QuestionAttemptTimeLimit::Unlimited, None, None, 2_000, 1),
                QuestionAttemptTimingEvaluationError::PauseOnUntimed,
            ),
            (
                "negative pause",
                evaluation(timed, Some(2_000), None, 2_000, -1),
                QuestionAttemptTimingEvaluationError::NegativePauseExtension,
            ),
            (
                "deadline before issue",
                evaluation(timed, Some(999), None, 2_000, 0),
                QuestionAttemptTimingEvaluationError::DeadlineBeforeIssue,
            ),
            (
                "evaluation before issue",
                evaluation(timed, Some(2_000), None, 999, 0),
                QuestionAttemptTimingEvaluationError::EvaluationBeforeIssue,
            ),
            (
                "submission before issue",
                evaluation(timed, Some(2_000), Some(999), 2_000, 0),
                QuestionAttemptTimingEvaluationError::SubmissionBeforeIssue,
            ),
            (
                "submission after evaluation",
                evaluation(timed, Some(2_000), Some(1_500), 1_499, 0),
                QuestionAttemptTimingEvaluationError::SubmissionAfterEvaluation,
            ),
        ];

        for (name, input, expected) in cases {
            assert_eq!(
                question_attempt_timing_decision(&input),
                Err(expected),
                "{name}"
            );
        }
    }

    #[test]
    fn timestamp_overflow_is_an_error() {
        let input = QuestionAttemptTimingEvaluation {
            policy: QuestionAttemptTimeLimit::Limited {
                seconds: 1,
                grace_seconds: 0,
            },
            timer: QuestionAttemptTiming {
                issued_at: timestamp(i64::MAX - 1),
                deadline: Some(timestamp(i64::MAX)),
                submitted_at: None,
            },
            evaluated_at: timestamp(i64::MAX),
            pause_extension_millis: 1,
        };

        assert_eq!(
            question_attempt_timing_decision(&input),
            Err(QuestionAttemptTimingEvaluationError::TimestampOverflow)
        );
    }

    #[test]
    fn serde_uses_lower_camel_case_at_the_wire() {
        let input = evaluation(
            QuestionAttemptTimeLimit::Limited {
                seconds: 60,
                grace_seconds: 2,
            },
            Some(10_000),
            Some(10_001),
            10_001,
            500,
        );
        let json = serde_json::to_string(&input).expect("evaluation should serialize");

        assert!(json.contains(r#""evaluatedAt":10001"#));
        assert!(json.contains(r#""pauseExtensionMillis":500"#));
        assert_eq!(
            serde_json::to_string(&QuestionAttemptTimingDecision::SubmittedWithinGrace)
                .expect("verdict should serialize"),
            r#""submittedWithinGrace""#
        );
    }
}
