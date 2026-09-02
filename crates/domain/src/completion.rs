//! Derived within-Assignment-Attempt completion.
//!
//! Completion derives from current required-question states, never a
//! stored boolean. A stored flag could disagree with the attempts that
//! produced it; deriving the value keeps those states inseparable.

use question_model::{AssignmentAttemptCompletion, AssignmentCompletionRule};

use crate::assignment_activity::{AssignmentActivityError, validate_fraction};

/// Current state of one required question within an Assignment Attempt.
///
/// This derives from its attempts, not another persisted completion
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

/// Derives within-Assignment-Attempt completion from current required-question states.
///
/// An empty Assignment Attempt is always in progress. Threshold completion requires every
/// question to have an answer before its score can complete the Assignment Attempt.
///
/// # Errors
///
/// Returns [`AssignmentActivityError`] when a threshold or normalized result cannot
/// represent a finite bounded score.
pub fn derive_within_assignment_attempt_completion(
    questions: &[RequiredQuestionState],
    rule: AssignmentCompletionRule,
) -> Result<AssignmentAttemptCompletion, AssignmentActivityError> {
    if questions.is_empty() {
        return Ok(AssignmentAttemptCompletion::InProgress);
    }

    let all_answered = questions.iter().all(|question| question.answered);
    let complete = match rule {
        AssignmentCompletionRule::AnswerAll => all_answered,
        AssignmentCompletionRule::AllCorrect => {
            all_answered && questions.iter().all(|question| question.correct)
        }
        AssignmentCompletionRule::ScoreAtLeast { fraction } => {
            validate_fraction(fraction)
                .map_err(|_| AssignmentActivityError::InvalidCompletionThreshold { fraction })?;
            all_answered && score_fraction(questions)? >= fraction
        }
    };

    Ok(if complete {
        AssignmentAttemptCompletion::Completed
    } else {
        AssignmentAttemptCompletion::InProgress
    })
}

/// Computes the current score fraction across required questions.
fn score_fraction(questions: &[RequiredQuestionState]) -> Result<f64, AssignmentActivityError> {
    let mut earned = 0.0;
    let mut possible = 0.0;

    for question in questions {
        let credit = question.points_earned / question.points_possible;
        if !question.points_earned.is_finite()
            || !question.points_possible.is_finite()
            || question.points_possible <= 0.0
            || !credit.is_finite()
            || !(-1_000.0..=1_000.0).contains(&credit)
        {
            return Err(AssignmentActivityError::InvalidQuestionPoints);
        }
        earned += question.points_earned;
        possible += question.points_possible;
    }

    let fraction = earned / possible;
    if fraction.is_finite() && (-1_000.0..=1_000.0).contains(&fraction) {
        Ok(fraction)
    } else {
        Err(AssignmentActivityError::InvalidQuestionPoints)
    }
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
    fn empty_assignment_attempt_is_not_complete() {
        assert_eq!(
            derive_within_assignment_attempt_completion(&[], AssignmentCompletionRule::AnswerAll),
            Ok(AssignmentAttemptCompletion::InProgress)
        );
    }

    #[test]
    fn answer_all_requires_each_required_question() {
        let states = [question(true, false, 0.0), question(false, false, 0.0)];
        assert_eq!(
            derive_within_assignment_attempt_completion(
                &states,
                AssignmentCompletionRule::AnswerAll
            ),
            Ok(AssignmentAttemptCompletion::InProgress)
        );
    }

    #[test]
    fn all_correct_is_derived_from_every_required_question() {
        let states = [question(true, true, 1.0), question(true, false, 0.0)];
        assert_eq!(
            derive_within_assignment_attempt_completion(
                &states,
                AssignmentCompletionRule::AllCorrect
            ),
            Ok(AssignmentAttemptCompletion::InProgress)
        );
    }

    #[test]
    fn score_threshold_requires_answers_before_points_can_complete() {
        let states = [question(true, true, 1.0), question(false, false, 1.0)];
        assert_eq!(
            derive_within_assignment_attempt_completion(
                &states,
                AssignmentCompletionRule::ScoreAtLeast { fraction: 0.5 }
            ),
            Ok(AssignmentAttemptCompletion::InProgress)
        );
    }

    #[test]
    fn score_threshold_completes_at_its_inclusive_boundary() {
        let states = [question(true, true, 1.0), question(true, false, 0.0)];
        assert_eq!(
            derive_within_assignment_attempt_completion(
                &states,
                AssignmentCompletionRule::ScoreAtLeast { fraction: 0.5 }
            ),
            Ok(AssignmentAttemptCompletion::Completed)
        );
    }

    #[test]
    fn invalid_threshold_is_an_explicit_error() {
        let states = [question(true, true, 1.0)];
        assert_eq!(
            derive_within_assignment_attempt_completion(
                &states,
                AssignmentCompletionRule::ScoreAtLeast { fraction: 1.1 }
            ),
            Err(AssignmentActivityError::InvalidCompletionThreshold { fraction: 1.1 })
        );
    }
}
