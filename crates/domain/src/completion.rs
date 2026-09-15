//! Derived within-Assessment-Attempt completion.
//!
//! Completion derives from current required-question states, never a
//! stored boolean. A stored flag could disagree with the attempts that
//! produced it; deriving the value keeps those states inseparable.

use question_model::{AssessmentAttemptCompletion, AssessmentCompletionRule};

use crate::assessment_activity::{AssessmentActivityError, validate_fraction};

/// Current state of one required question within an Assessment Attempt.
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

/// Derives within-Assessment-Attempt completion from current required-question states.
///
/// An empty Assessment Attempt is always in progress. Threshold completion requires every
/// question to have an answer before its score can complete the Assessment Attempt.
///
/// # Errors
///
/// Returns [`AssessmentActivityError`] when a threshold or normalized result cannot
/// represent a finite bounded score.
pub fn derive_within_assessment_attempt_completion(
    questions: &[RequiredQuestionState],
    rule: AssessmentCompletionRule,
) -> Result<AssessmentAttemptCompletion, AssessmentActivityError> {
    if questions.is_empty() {
        return Ok(AssessmentAttemptCompletion::InProgress);
    }

    let all_answered = questions.iter().all(|question| question.answered);
    let complete = match rule {
        AssessmentCompletionRule::AnswerAll => all_answered,
        AssessmentCompletionRule::AllCorrect => {
            all_answered && questions.iter().all(|question| question.correct)
        }
        AssessmentCompletionRule::ScoreAtLeast { fraction } => {
            validate_fraction(fraction)
                .map_err(|_| AssessmentActivityError::InvalidCompletionThreshold { fraction })?;
            all_answered && score_fraction(questions)? >= fraction
        }
    };

    Ok(if complete {
        AssessmentAttemptCompletion::Completed
    } else {
        AssessmentAttemptCompletion::InProgress
    })
}

/// Computes the current score fraction across required questions.
fn score_fraction(questions: &[RequiredQuestionState]) -> Result<f64, AssessmentActivityError> {
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
            return Err(AssessmentActivityError::InvalidQuestionPoints);
        }
        earned += question.points_earned;
        possible += question.points_possible;
    }

    let fraction = earned / possible;
    if fraction.is_finite() && (-1_000.0..=1_000.0).contains(&fraction) {
        Ok(fraction)
    } else {
        Err(AssessmentActivityError::InvalidQuestionPoints)
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
    fn empty_assessment_attempt_is_not_complete() {
        assert_eq!(
            derive_within_assessment_attempt_completion(&[], AssessmentCompletionRule::AnswerAll),
            Ok(AssessmentAttemptCompletion::InProgress)
        );
    }

    #[test]
    fn answer_all_requires_each_required_question() {
        let states = [question(true, false, 0.0), question(false, false, 0.0)];
        assert_eq!(
            derive_within_assessment_attempt_completion(
                &states,
                AssessmentCompletionRule::AnswerAll
            ),
            Ok(AssessmentAttemptCompletion::InProgress)
        );
    }

    #[test]
    fn all_correct_is_derived_from_every_required_question() {
        let states = [question(true, true, 1.0), question(true, false, 0.0)];
        assert_eq!(
            derive_within_assessment_attempt_completion(
                &states,
                AssessmentCompletionRule::AllCorrect
            ),
            Ok(AssessmentAttemptCompletion::InProgress)
        );
    }

    #[test]
    fn score_threshold_requires_answers_before_points_can_complete() {
        let states = [question(true, true, 1.0), question(false, false, 1.0)];
        assert_eq!(
            derive_within_assessment_attempt_completion(
                &states,
                AssessmentCompletionRule::ScoreAtLeast { fraction: 0.5 }
            ),
            Ok(AssessmentAttemptCompletion::InProgress)
        );
    }

    #[test]
    fn score_threshold_completes_at_its_inclusive_boundary() {
        let states = [question(true, true, 1.0), question(true, false, 0.0)];
        assert_eq!(
            derive_within_assessment_attempt_completion(
                &states,
                AssessmentCompletionRule::ScoreAtLeast { fraction: 0.5 }
            ),
            Ok(AssessmentAttemptCompletion::Completed)
        );
    }

    #[test]
    fn invalid_threshold_is_an_explicit_error() {
        let states = [question(true, true, 1.0)];
        assert_eq!(
            derive_within_assessment_attempt_completion(
                &states,
                AssessmentCompletionRule::ScoreAtLeast { fraction: 1.1 }
            ),
            Err(AssessmentActivityError::InvalidCompletionThreshold { fraction: 1.1 })
        );
    }
}
