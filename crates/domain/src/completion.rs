//! Derived within-run completion (MOD-STATE).
//!
//! Completion is a projection over current required-question states, never a
//! stored boolean. A stored flag could disagree with the attempts that
//! produced it; deriving the value keeps those states inseparable.

use question_model::CompletionRequirement;

use crate::run::{RunModelError, validate_fraction};

/// Current state of one required question within a run.
///
/// This is a projection of its attempts, not another persisted completion
/// flag. The current response may change while retries remain available.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RequiredQuestionState {
    /// Whether the student has submitted a response for this question.
    pub answered: bool,
    /// Whether the current response is correct.
    pub correct: bool,
    /// Points earned by the current response.
    pub points_earned: f64,
    /// Points available for this question.
    pub points_possible: f64,
}

/// Derived within-run completion state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WithinRunCompletion {
    /// At least one completion condition remains unmet.
    InProgress,
    /// The current question states satisfy the completion policy.
    Complete,
}

/// Derives within-run completion from current required-question states.
///
/// An empty run is always in progress. Threshold completion requires every
/// question to have an answer before its score can complete the run.
///
/// # Errors
///
/// Returns [`RunModelError`] when a threshold or point value cannot represent
/// a finite score fraction.
pub fn derive_within_run_completion(
    questions: &[RequiredQuestionState],
    requirement: CompletionRequirement,
) -> Result<WithinRunCompletion, RunModelError> {
    if questions.is_empty() {
        return Ok(WithinRunCompletion::InProgress);
    }

    let all_answered = questions.iter().all(|question| question.answered);
    let complete = match requirement {
        CompletionRequirement::AnswerAll => all_answered,
        CompletionRequirement::AllCorrect => {
            all_answered && questions.iter().all(|question| question.correct)
        }
        CompletionRequirement::ScoreAtLeast { fraction } => {
            validate_fraction(fraction)
                .map_err(|_| RunModelError::InvalidCompletionThreshold { fraction })?;
            all_answered && score_fraction(questions)? >= fraction
        }
    };

    Ok(if complete {
        WithinRunCompletion::Complete
    } else {
        WithinRunCompletion::InProgress
    })
}

/// Computes the current score fraction across required questions.
fn score_fraction(questions: &[RequiredQuestionState]) -> Result<f64, RunModelError> {
    let mut earned = 0.0;
    let mut possible = 0.0;

    for question in questions {
        if !question.points_earned.is_finite()
            || !question.points_possible.is_finite()
            || question.points_earned < 0.0
            || question.points_possible <= 0.0
            || question.points_earned > question.points_possible
        {
            return Err(RunModelError::InvalidQuestionPoints);
        }
        earned += question.points_earned;
        possible += question.points_possible;
    }

    let fraction = earned / possible;
    validate_fraction(fraction).map_err(|_| RunModelError::InvalidQuestionPoints)?;
    Ok(fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(answered: bool, correct: bool, earned: f64) -> RequiredQuestionState {
        RequiredQuestionState {
            answered,
            correct,
            points_earned: earned,
            points_possible: 1.0,
        }
    }

    #[test]
    fn empty_run_is_not_complete() {
        assert_eq!(
            derive_within_run_completion(&[], CompletionRequirement::AnswerAll),
            Ok(WithinRunCompletion::InProgress)
        );
    }

    #[test]
    fn answer_all_requires_each_required_question() {
        let states = [question(true, false, 0.0), question(false, false, 0.0)];
        assert_eq!(
            derive_within_run_completion(&states, CompletionRequirement::AnswerAll),
            Ok(WithinRunCompletion::InProgress)
        );
    }

    #[test]
    fn all_correct_is_derived_from_every_required_question() {
        let states = [question(true, true, 1.0), question(true, false, 0.0)];
        assert_eq!(
            derive_within_run_completion(&states, CompletionRequirement::AllCorrect),
            Ok(WithinRunCompletion::InProgress)
        );
    }

    #[test]
    fn score_threshold_requires_answers_before_points_can_complete() {
        let states = [question(true, true, 1.0), question(false, false, 1.0)];
        assert_eq!(
            derive_within_run_completion(
                &states,
                CompletionRequirement::ScoreAtLeast { fraction: 0.5 }
            ),
            Ok(WithinRunCompletion::InProgress)
        );
    }

    #[test]
    fn score_threshold_completes_at_its_inclusive_boundary() {
        let states = [question(true, true, 1.0), question(true, false, 0.0)];
        assert_eq!(
            derive_within_run_completion(
                &states,
                CompletionRequirement::ScoreAtLeast { fraction: 0.5 }
            ),
            Ok(WithinRunCompletion::Complete)
        );
    }

    #[test]
    fn invalid_threshold_is_an_explicit_error() {
        let states = [question(true, true, 1.0)];
        assert_eq!(
            derive_within_run_completion(
                &states,
                CompletionRequirement::ScoreAtLeast { fraction: 1.1 }
            ),
            Err(RunModelError::InvalidCompletionThreshold { fraction: 1.1 })
        );
    }
}
