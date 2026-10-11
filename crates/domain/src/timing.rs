//! Server-authoritative Question Attempt timer verdicts.
//!
//! The caller supplies timestamps recorded from the server clock. This module
//! never reads a clock, so browser clock skew cannot change the verdict.
//! Authorized pauses arrive as one cumulative extension reconstructed from
//! audit events; pause authorization and persistence belong to the server.

use question_model::assessment_activity_rules::QuestionAttemptTimeLimit;
use question_model::{QuestionAttemptTiming, Timestamp};
use serde::{Deserialize, Serialize};

/// Largest whole millisecond value JavaScript can represent exactly.
pub const MAX_BROWSER_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Clock-free browser countdown input.
///
/// The server supplies `initial_remaining_milliseconds` in a progress
/// projection. The browser supplies only elapsed time from a monotonic clock;
/// it never derives a deadline from its wall clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentAttemptRemainingDurationInput {
    pub initial_remaining_milliseconds: Option<u64>,
    pub elapsed_milliseconds: u64,
}

/// Invalid browser countdown input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentAttemptRemainingDurationError {
    /// A JavaScript number would lose integer precision at this magnitude.
    UnsafeInteger,
}

impl std::fmt::Display for AssessmentAttemptRemainingDurationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("assessment attempt duration must be a JavaScript safe integer")
    }
}

impl std::error::Error for AssessmentAttemptRemainingDurationError {}

/// Calculates the displayed remaining duration without reading any clock.
///
/// `None` remains untimed. Timed values are clamped at zero, so a delayed
/// browser frame cannot produce a negative timer display.
pub fn assessment_attempt_remaining_milliseconds(
    input: AssessmentAttemptRemainingDurationInput,
) -> Result<Option<u64>, AssessmentAttemptRemainingDurationError> {
    if input.elapsed_milliseconds > MAX_BROWSER_SAFE_INTEGER
        || input
            .initial_remaining_milliseconds
            .is_some_and(|milliseconds| milliseconds > MAX_BROWSER_SAFE_INTEGER)
    {
        return Err(AssessmentAttemptRemainingDurationError::UnsafeInteger);
    }

    Ok(input
        .initial_remaining_milliseconds
        .map(|milliseconds| milliseconds.saturating_sub(input.elapsed_milliseconds)))
}

/// Complete clock-free input to one timer evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttemptTimingEvaluation {
    /// Authored timing and grace policy for the Question or Assessment Attempt.
    pub policy: QuestionAttemptTimeLimit,
    /// Server-recorded issue, base-deadline, and finalization timestamps.
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
    FinalizedOnTime,
    /// The response arrived after the deadline but within the inclusive grace window.
    FinalizedWithinGrace,
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
    FinalizationBeforeIssue,
    /// The supplied evaluation predates the recorded submission.
    FinalizationAfterEvaluation,
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
            Self::FinalizationBeforeIssue => "the timer finalization predates issue",
            Self::FinalizationAfterEvaluation => "the timer finalization follows evaluation",
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
        .finalized_at
        .unwrap_or(evaluation.evaluated_at);

    if observed_at <= effective_deadline {
        return Ok(if evaluation.timer.finalized_at.is_some() {
            QuestionAttemptTimingDecision::FinalizedOnTime
        } else {
            QuestionAttemptTimingDecision::Open
        });
    }
    if observed_at <= grace_deadline {
        return Ok(if evaluation.timer.finalized_at.is_some() {
            QuestionAttemptTimingDecision::FinalizedWithinGrace
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
    if let Some(finalized_at) = evaluation.timer.finalized_at {
        if finalized_at < evaluation.timer.issued_at {
            return Err(QuestionAttemptTimingEvaluationError::FinalizationBeforeIssue);
        }
        if finalized_at > evaluation.evaluated_at {
            return Err(QuestionAttemptTimingEvaluationError::FinalizationAfterEvaluation);
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
