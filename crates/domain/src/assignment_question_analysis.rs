//! Current, course-local Assignment Question Analysis projections.
//!
//! These records deliberately contain aggregate Question outcomes only. They
//! never retain Student identity, raw responses, answer choices, or Object Addresses.

use question_model::{
    AssignmentEntryId, AssignmentId, CourseId, QuestionRevisionReference, ScoringGeneration,
    Timestamp,
};
use serde::{Deserialize, Serialize};

/// Aggregate Question outcome category safe to persist in a course-local report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionOutcomeCategory {
    Correct,
    PartialCredit,
    Incorrect,
    Unanswered,
}

/// Counts for the fixed, non-identifying Question outcome categories.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QuestionOutcomeDistribution {
    pub correct: u32,
    pub partial_credit: u32,
    pub incorrect: u32,
    pub unanswered: u32,
}

impl QuestionOutcomeDistribution {
    /// Returns the count in the requested safe aggregate Question outcome category.
    pub fn count(self, category: QuestionOutcomeCategory) -> u32 {
        match category {
            QuestionOutcomeCategory::Correct => self.correct,
            QuestionOutcomeCategory::PartialCredit => self.partial_credit,
            QuestionOutcomeCategory::Incorrect => self.incorrect,
            QuestionOutcomeCategory::Unanswered => self.unanswered,
        }
    }
}

/// One current aggregate Question row delivered by an Assignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssignmentQuestionAnalysis {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub assignment_entry: AssignmentEntryId,
    pub question_revision: QuestionRevisionReference,
    pub scoring_generation: ScoringGeneration,
    pub analyzed_at: Timestamp,
    pub graded_attempt_count: u32,
    pub unanswered_attempt_count: u32,
    /// Submitted attempts whose automated evaluation has not produced a
    /// coherent score. These attempts are intentionally outside the Question
    /// outcome distribution: no score-derived category is truthful yet.
    pub unscored_attempt_count: u32,
    /// Fraction of graded attempts with full current credit.
    pub question_difficulty: Option<f64>,
    /// Mean current credit fraction among graded attempts.
    pub average_credit: Option<f64>,
    /// Sample standard deviation of current credit fraction among graded attempts.
    pub credit_standard_deviation: Option<f64>,
    /// Pearson correlation between Question credit and other Question credit in the Assignment Attempt.
    pub question_discrimination: Option<f64>,
    pub question_outcome_distribution: QuestionOutcomeDistribution,
    /// Mean elapsed milliseconds from the terminal Student submission for Assignment Attempts
    /// that delivered this Question.
    pub average_completion_time_millis: Option<u64>,
}

/// Current report header and all Question rows for one Course Instance Assignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssignmentQuestionAnalysisReport {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub scoring_generation: ScoringGeneration,
    pub analyzed_at: Timestamp,
    pub completed_assignment_attempt_count: u32,
    pub in_progress_assignment_attempt_count: u32,
    /// A terminal submitted attempt lacks a coherent automated score.
    /// Score-derived Assignment metrics remain suppressed while this is true.
    pub incomplete_scoring: bool,
    /// Derived at read time from the stored source generation and current scoring state.
    pub recent_rescoring: bool,
    pub assignment_average_score: Option<f64>,
    pub average_completion_time_millis: Option<u64>,
    pub question_analyses: Vec<AssignmentQuestionAnalysis>,
}

/// Private aggregate inputs stripped of all Student and response identity.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssignmentQuestionAnalysisMetricInput {
    pub graded_credits: Vec<f64>,
    pub graded_correct: Vec<bool>,
    pub rest_of_assignment_credits: Vec<f64>,
    pub unanswered_attempt_count: u32,
    /// Internal aggregate input for report completeness. It deliberately does
    /// not enter a Question outcome category.
    pub unscored_attempt_count: u32,
    pub completion_times_millis: Vec<u64>,
}

/// Pure computed metrics used by storage-specific Assignment Question Analysis builders.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentQuestionAnalysisMetrics {
    pub graded_attempt_count: u32,
    /// Submitted attempts awaiting a coherent automated score. This remains
    /// outside the closed Question Outcome Distribution.
    pub unscored_attempt_count: u32,
    pub question_difficulty: Option<f64>,
    pub average_credit: Option<f64>,
    pub credit_standard_deviation: Option<f64>,
    pub question_discrimination: Option<f64>,
    pub average_completion_time_millis: Option<u64>,
    pub question_outcome_distribution: QuestionOutcomeDistribution,
}

/// Invalid aggregate input rejected before it can corrupt an Assignment Question Analysis report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentQuestionAnalysisMetricError {
    UnpairedCredits,
    UnpairedCorrectness,
    InvalidCredit,
    InvalidRestOfAssignmentCredit,
}

impl std::fmt::Display for AssignmentQuestionAnalysisMetricError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::UnpairedCredits => {
                "Assignment Question Analysis credits must remain paired with other Assignment Attempt Question credits"
            }
            Self::UnpairedCorrectness => {
                "Assignment Question Analysis credits must remain paired with correctness"
            }
            Self::InvalidCredit => {
                "Assignment Question Analysis credit must be finite and between -1000 and 1000"
            }
            Self::InvalidRestOfAssignmentCredit => {
                "Assignment Question Analysis other Assignment Attempt Question credit must be finite"
            }
        })
    }
}

impl std::error::Error for AssignmentQuestionAnalysisMetricError {}

/// Calculates current-only Assignment Question Analysis metrics from identity-free aggregate inputs.
///
/// Credits must be finite and within the automated scoring contract; invalid
/// values are rejected rather than silently affecting an Instructor report.
pub fn calculate_assignment_question_analysis_metrics(
    input: &AssignmentQuestionAnalysisMetricInput,
) -> Result<AssignmentQuestionAnalysisMetrics, AssignmentQuestionAnalysisMetricError> {
    if input.graded_credits.len() != input.rest_of_assignment_credits.len() {
        return Err(AssignmentQuestionAnalysisMetricError::UnpairedCredits);
    }
    if input.graded_credits.len() != input.graded_correct.len() {
        return Err(AssignmentQuestionAnalysisMetricError::UnpairedCorrectness);
    }
    if input
        .graded_credits
        .iter()
        .any(|credit| !credit.is_finite() || !(-1_000.0..=1_000.0).contains(credit))
    {
        return Err(AssignmentQuestionAnalysisMetricError::InvalidCredit);
    }
    if input
        .rest_of_assignment_credits
        .iter()
        .any(|credit| !credit.is_finite())
    {
        return Err(AssignmentQuestionAnalysisMetricError::InvalidRestOfAssignmentCredit);
    }
    let credits = &input.graded_credits;
    let graded_attempt_count = u32::try_from(credits.len()).unwrap_or(u32::MAX);
    let average_credit = mean(credits);
    let credit_standard_deviation = sample_standard_deviation(credits);
    let question_difficulty = (!credits.is_empty()).then(|| {
        input
            .graded_correct
            .iter()
            .filter(|correct| **correct)
            .count() as f64
            / credits.len() as f64
    });
    let question_discrimination = pearson_correlation(credits, &input.rest_of_assignment_credits);
    let correct = u32::try_from(
        input
            .graded_correct
            .iter()
            .filter(|correct| **correct)
            .count(),
    )
    .unwrap_or(u32::MAX);
    // Positive non-full credit, including extra credit, remains Partial Credit;
    // zero and negative credit are Incorrect unless the evaluator marked it correct.
    let partial_credit = u32::try_from(
        credits
            .iter()
            .zip(input.graded_correct.iter())
            .filter(|(credit, correct)| **credit > 0.0 && !**correct)
            .count(),
    )
    .unwrap_or(u32::MAX);
    let incorrect = u32::try_from(
        credits
            .iter()
            .zip(input.graded_correct.iter())
            .filter(|(credit, correct)| **credit <= 0.0 && !**correct)
            .count(),
    )
    .unwrap_or(u32::MAX);
    Ok(AssignmentQuestionAnalysisMetrics {
        graded_attempt_count,
        unscored_attempt_count: input.unscored_attempt_count,
        question_difficulty,
        average_credit,
        credit_standard_deviation,
        question_discrimination,
        average_completion_time_millis: mean_u64(&input.completion_times_millis),
        question_outcome_distribution: QuestionOutcomeDistribution {
            correct,
            partial_credit,
            incorrect,
            unanswered: input.unanswered_attempt_count,
        },
    })
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn mean_u64(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let total = values.iter().map(|value| u128::from(*value)).sum::<u128>();
    u64::try_from(total / values.len() as u128).ok()
}

fn sample_standard_deviation(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean = mean(values)?;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    (variance.is_finite() && variance >= 0.0).then(|| variance.sqrt())
}

fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 2
        || x.iter().any(|value| !value.is_finite())
        || y.iter().any(|value| !value.is_finite())
    {
        return None;
    }
    let x_mean = mean(x)?;
    let y_mean = mean(y)?;
    let (numerator, x_sum_squares, y_sum_squares) = x.iter().zip(y).fold(
        (0.0, 0.0, 0.0),
        |(numerator, x_sum_squares, y_sum_squares), (x_value, y_value)| {
            let x_delta = x_value - x_mean;
            let y_delta = y_value - y_mean;
            (
                numerator + x_delta * y_delta,
                x_sum_squares + x_delta * x_delta,
                y_sum_squares + y_delta * y_delta,
            )
        },
    );
    let denominator = (x_sum_squares * y_sum_squares).sqrt();
    (denominator.is_finite() && denominator > 0.0).then(|| numerator / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_assignment_question_metrics_and_safe_outcomes() {
        let metrics = calculate_assignment_question_analysis_metrics(
            &AssignmentQuestionAnalysisMetricInput {
                graded_credits: vec![1.0, 0.5, 0.0],
                graded_correct: vec![true, false, false],
                rest_of_assignment_credits: vec![1.0, 0.5, 0.0],
                unanswered_attempt_count: 2,
                unscored_attempt_count: 1,
                completion_times_millis: vec![1_000, 2_001],
            },
        )
        .expect("valid metrics");
        assert_eq!(metrics.graded_attempt_count, 3);
        assert_eq!(metrics.unscored_attempt_count, 1);
        assert_eq!(metrics.question_difficulty, Some(1.0 / 3.0));
        assert_eq!(metrics.average_credit, Some(0.5));
        assert_eq!(metrics.question_outcome_distribution.correct, 1);
        assert_eq!(metrics.question_outcome_distribution.partial_credit, 1);
        assert_eq!(metrics.question_outcome_distribution.incorrect, 1);
        assert_eq!(metrics.question_outcome_distribution.unanswered, 2);
        assert_eq!(
            metrics.question_outcome_distribution.correct
                + metrics.question_outcome_distribution.partial_credit
                + metrics.question_outcome_distribution.incorrect
                + metrics.question_outcome_distribution.unanswered,
            metrics.graded_attempt_count + metrics.question_outcome_distribution.unanswered
        );
        assert_eq!(metrics.average_completion_time_millis, Some(1_500));
        assert_eq!(metrics.question_discrimination, Some(1.0));
    }

    #[test]
    fn leaves_undefined_math_unavailable() {
        let metrics = calculate_assignment_question_analysis_metrics(
            &AssignmentQuestionAnalysisMetricInput {
                graded_credits: vec![1.0],
                graded_correct: vec![true],
                rest_of_assignment_credits: vec![0.5],
                ..AssignmentQuestionAnalysisMetricInput::default()
            },
        )
        .expect("valid metrics");
        assert_eq!(metrics.credit_standard_deviation, None);
        assert_eq!(metrics.question_discrimination, None);

        let metrics = calculate_assignment_question_analysis_metrics(
            &AssignmentQuestionAnalysisMetricInput {
                graded_credits: vec![1.0, 1.0],
                graded_correct: vec![true, true],
                rest_of_assignment_credits: vec![0.5, 0.5],
                ..AssignmentQuestionAnalysisMetricInput::default()
            },
        )
        .expect("valid metrics");
        assert_eq!(metrics.question_discrimination, None);
    }

    #[test]
    fn correctness_controls_difficulty_independently_of_credit() {
        let metrics = calculate_assignment_question_analysis_metrics(
            &AssignmentQuestionAnalysisMetricInput {
                graded_credits: vec![0.0, 2.0, -0.5],
                graded_correct: vec![true, false, false],
                rest_of_assignment_credits: vec![0.0, 2.0, -0.5],
                ..AssignmentQuestionAnalysisMetricInput::default()
            },
        )
        .expect("finite bounded credits are valid");
        assert_eq!(metrics.question_difficulty, Some(1.0 / 3.0));
        assert_eq!(metrics.question_outcome_distribution.correct, 1);
        assert_eq!(metrics.question_outcome_distribution.partial_credit, 1);
        assert_eq!(metrics.question_outcome_distribution.incorrect, 1);
    }
}
