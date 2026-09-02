//! Current, course-local item-analysis projections.
//!
//! These records deliberately contain aggregate buckets only. They never
//! retain Student identity, raw responses, answer choices, or Object Addresses.

use question_model::{
    AssignmentEntryId, AssignmentId, CourseId, QuestionRevisionReference, ScoringGeneration,
    Timestamp,
};
use serde::{Deserialize, Serialize};

/// Aggregate response category safe to persist in a course-local report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemAnalysisResponseBucket {
    Correct,
    Partial,
    Incorrect,
    Unanswered,
}

/// Counts for the fixed, non-identifying item-analysis response categories.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ItemAnalysisResponseDistribution {
    pub correct: u32,
    pub partial: u32,
    pub incorrect: u32,
    pub unanswered: u32,
}

impl ItemAnalysisResponseDistribution {
    /// Returns the count in the requested safe aggregate bucket.
    pub fn count(self, bucket: ItemAnalysisResponseBucket) -> u32 {
        match bucket {
            ItemAnalysisResponseBucket::Correct => self.correct,
            ItemAnalysisResponseBucket::Partial => self.partial,
            ItemAnalysisResponseBucket::Incorrect => self.incorrect,
            ItemAnalysisResponseBucket::Unanswered => self.unanswered,
        }
    }
}

/// One current aggregate row for an item delivered by an assignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssignmentItemAnalysis {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub assignment_entry: AssignmentEntryId,
    pub reference: QuestionRevisionReference,
    pub source_scoring_generation: ScoringGeneration,
    pub analyzed_at: Timestamp,
    pub graded_attempt_count: u32,
    pub unanswered_attempt_count: u32,
    /// Submitted attempts whose automated evaluation has not produced a
    /// coherent score. These attempts are intentionally outside the response
    /// distribution: no score-derived bucket is truthful yet.
    pub unscored_attempt_count: u32,
    /// Fraction of graded attempts with full current credit.
    pub difficulty: Option<f64>,
    /// Mean current credit fraction among graded attempts.
    pub average_credit: Option<f64>,
    /// Sample standard deviation of current credit fraction among graded attempts.
    pub credit_standard_deviation: Option<f64>,
    /// Pearson correlation between item credit and rest-of-Assignment-Attempt credit.
    pub discrimination: Option<f64>,
    pub response_distribution: ItemAnalysisResponseDistribution,
    /// Mean elapsed milliseconds from the terminal Student submission for Assignment Attempts
    /// that delivered this item.
    pub average_completion_time_millis: Option<u64>,
}

/// Current report header and all item rows for one course assignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CourseItemAnalysisReport {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub source_scoring_generation: ScoringGeneration,
    pub analyzed_at: Timestamp,
    pub completed_assignment_attempt_count: u32,
    pub in_progress_assignment_attempt_count: u32,
    /// A terminal submitted attempt lacks a coherent automated score.
    /// Score-derived assignment metrics remain suppressed while this is true.
    pub incomplete_scoring: bool,
    /// Derived at read time from the stored source generation and current scoring state.
    pub recent_rescoring: bool,
    pub assignment_average_score: Option<f64>,
    pub average_completion_time_millis: Option<u64>,
    pub items: Vec<AssignmentItemAnalysis>,
}

/// Private aggregate inputs stripped of all Student and response identity.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemAnalysisMetricInput {
    pub graded_credits: Vec<f64>,
    pub graded_correct: Vec<bool>,
    pub rest_of_run_credits: Vec<f64>,
    pub unanswered_attempt_count: u32,
    /// Internal aggregate input for report completeness. It deliberately does
    /// not enter a response bucket.
    pub unscored_attempt_count: u32,
    pub completion_times_millis: Vec<u64>,
}

/// Pure computed metrics used by storage-specific report builders.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemAnalysisMetrics {
    pub graded_attempt_count: u32,
    pub difficulty: Option<f64>,
    pub average_credit: Option<f64>,
    pub credit_standard_deviation: Option<f64>,
    pub discrimination: Option<f64>,
    pub average_completion_time_millis: Option<u64>,
    pub response_distribution: ItemAnalysisResponseDistribution,
}

/// Invalid aggregate input rejected before it can corrupt a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAnalysisMetricError {
    UnpairedCredits,
    UnpairedCorrectness,
    InvalidCredit,
    InvalidRestOfRunCredit,
}

impl std::fmt::Display for ItemAnalysisMetricError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::UnpairedCredits => {
                "item-analysis credits must remain paired with rest-of-Assignment-Attempt credits"
            }
            Self::UnpairedCorrectness => {
                "item-analysis credits must remain paired with correctness"
            }
            Self::InvalidCredit => "item-analysis credit must be finite and between -1000 and 1000",
            Self::InvalidRestOfRunCredit => {
                "item-analysis rest-of-Assignment-Attempt credit must be finite"
            }
        })
    }
}

impl std::error::Error for ItemAnalysisMetricError {}

/// Calculates current-only item metrics from identity-free aggregate inputs.
///
/// Credits must be finite and within the automated scoring contract; invalid
/// values are rejected rather than silently affecting an instructor report.
pub fn calculate_item_analysis_metrics(
    input: &ItemAnalysisMetricInput,
) -> Result<ItemAnalysisMetrics, ItemAnalysisMetricError> {
    if input.graded_credits.len() != input.rest_of_run_credits.len() {
        return Err(ItemAnalysisMetricError::UnpairedCredits);
    }
    if input.graded_credits.len() != input.graded_correct.len() {
        return Err(ItemAnalysisMetricError::UnpairedCorrectness);
    }
    if input
        .graded_credits
        .iter()
        .any(|credit| !credit.is_finite() || !(-1_000.0..=1_000.0).contains(credit))
    {
        return Err(ItemAnalysisMetricError::InvalidCredit);
    }
    if input
        .rest_of_run_credits
        .iter()
        .any(|credit| !credit.is_finite())
    {
        return Err(ItemAnalysisMetricError::InvalidRestOfRunCredit);
    }
    let credits = &input.graded_credits;
    let graded_attempt_count = u32::try_from(credits.len()).unwrap_or(u32::MAX);
    let average_credit = mean(credits);
    let credit_standard_deviation = sample_standard_deviation(credits);
    let difficulty = (!credits.is_empty()).then(|| {
        input
            .graded_correct
            .iter()
            .filter(|correct| **correct)
            .count() as f64
            / credits.len() as f64
    });
    let discrimination = pearson_correlation(credits, &input.rest_of_run_credits);
    let correct = u32::try_from(
        input
            .graded_correct
            .iter()
            .filter(|correct| **correct)
            .count(),
    )
    .unwrap_or(u32::MAX);
    // Positive non-full credit, including extra credit, remains Partial;
    // zero and negative credit are Incorrect unless the evaluator marked it correct.
    let partial = u32::try_from(
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
    Ok(ItemAnalysisMetrics {
        graded_attempt_count,
        difficulty,
        average_credit,
        credit_standard_deviation,
        discrimination,
        average_completion_time_millis: mean_u64(&input.completion_times_millis),
        response_distribution: ItemAnalysisResponseDistribution {
            correct,
            partial,
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
    fn computes_current_credit_metrics_and_safe_buckets() {
        let metrics = calculate_item_analysis_metrics(&ItemAnalysisMetricInput {
            graded_credits: vec![1.0, 0.5, 0.0],
            graded_correct: vec![true, false, false],
            rest_of_run_credits: vec![1.0, 0.5, 0.0],
            unanswered_attempt_count: 2,
            unscored_attempt_count: 1,
            completion_times_millis: vec![1_000, 2_001],
        })
        .expect("valid metrics");
        assert_eq!(metrics.graded_attempt_count, 3);
        assert_eq!(metrics.difficulty, Some(1.0 / 3.0));
        assert_eq!(metrics.average_credit, Some(0.5));
        assert_eq!(metrics.response_distribution.correct, 1);
        assert_eq!(metrics.response_distribution.partial, 1);
        assert_eq!(metrics.response_distribution.incorrect, 1);
        assert_eq!(metrics.response_distribution.unanswered, 2);
        assert_eq!(metrics.average_completion_time_millis, Some(1_500));
        assert_eq!(metrics.discrimination, Some(1.0));
    }

    #[test]
    fn leaves_undefined_math_unavailable() {
        let metrics = calculate_item_analysis_metrics(&ItemAnalysisMetricInput {
            graded_credits: vec![1.0],
            graded_correct: vec![true],
            rest_of_run_credits: vec![0.5],
            ..ItemAnalysisMetricInput::default()
        })
        .expect("valid metrics");
        assert_eq!(metrics.credit_standard_deviation, None);
        assert_eq!(metrics.discrimination, None);

        let metrics = calculate_item_analysis_metrics(&ItemAnalysisMetricInput {
            graded_credits: vec![1.0, 1.0],
            graded_correct: vec![true, true],
            rest_of_run_credits: vec![0.5, 0.5],
            ..ItemAnalysisMetricInput::default()
        })
        .expect("valid metrics");
        assert_eq!(metrics.discrimination, None);
    }

    #[test]
    fn correctness_controls_difficulty_independently_of_credit() {
        let metrics = calculate_item_analysis_metrics(&ItemAnalysisMetricInput {
            graded_credits: vec![0.0, 2.0, -0.5],
            graded_correct: vec![true, false, false],
            rest_of_run_credits: vec![0.0, 2.0, -0.5],
            ..ItemAnalysisMetricInput::default()
        })
        .expect("finite bounded credits are valid");
        assert_eq!(metrics.difficulty, Some(1.0 / 3.0));
        assert_eq!(metrics.response_distribution.correct, 1);
        assert_eq!(metrics.response_distribution.partial, 1);
        assert_eq!(metrics.response_distribution.incorrect, 1);
    }
}
