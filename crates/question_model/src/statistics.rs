//! Browser-safe anonymous question-statistics projections.
//!
//! The raw aggregate and its write path remain server-side. This module holds
//! only the redacted, k-anonymity-gated view a visible catalog version may
//! disclose, plus the small policy value used to make that decision.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// Default minimum number of independent cohort contributions before metrics
/// may be disclosed.
pub const DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE: u32 = 5;

/// Learner-safe result of considering the current course-local assignment
/// analysis for class-statistics disclosure.
///
/// This deliberately contains neither an analysis identity nor partial
/// evidence: a learner either receives the two safe aggregate values or no
/// cohort count/metric at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LearnerClassStatistics {
    /// No current, complete, k-anonymous analysis can safely be disclosed.
    InsufficientEvidence,
    /// Anonymous aggregate metrics for a completed-learner cohort.
    Available {
        /// Number of distinct learners represented by their latest completed run.
        completed_learner_cohort_size: u32,
        /// Mean normalized assignment score, in the inclusive range `0.0..=1.0`.
        assignment_average_score: f64,
    },
}

impl LearnerClassStatistics {
    /// Gates a current course-local analysis before it reaches a learner.
    ///
    /// The caller obtains the values only from the current report. Missing or
    /// stale reports, incomplete manual grading, insufficient cohort evidence,
    /// and malformed scores all collapse to the identity-free suppressed state.
    pub fn from_current_analysis(
        completed_learner_cohort_size: u32,
        incomplete_manual_grading: bool,
        recent_rescoring: bool,
        assignment_average_score: Option<f64>,
    ) -> Self {
        let valid_average = assignment_average_score
            .filter(|score| score.is_finite() && (0.0..=1.0).contains(score));
        if completed_learner_cohort_size < DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE
            || incomplete_manual_grading
            || recent_rescoring
        {
            return Self::InsufficientEvidence;
        }
        match valid_average {
            Some(assignment_average_score) => Self::Available {
                completed_learner_cohort_size,
                assignment_average_score,
            },
            None => Self::InsufficientEvidence,
        }
    }
}

/// A rejected statistics-disclosure configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticsDisclosurePolicyError {
    /// A configured threshold weakened the documented privacy floor.
    BelowPrivacyFloor,
}

impl std::fmt::Display for StatisticsDisclosurePolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("statistics disclosure threshold must be at least five")
    }
}

impl std::error::Error for StatisticsDisclosurePolicyError {}

/// A positive k-anonymity threshold for anonymous question statistics.
///
/// The threshold controls disclosure only. It must not cause the server to
/// discard aggregate evidence or vary a shared result by requesting tenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatisticsDisclosurePolicy {
    minimum_cohort_size: NonZeroU32,
}

impl StatisticsDisclosurePolicy {
    /// Creates a disclosure threshold without permitting a privacy regression.
    ///
    /// This is a deployment-wide shared-content policy, never a tenant or
    /// request input. Deployment configuration may raise the threshold, but a
    /// value below the documented privacy floor is an explicit configuration
    /// error.
    pub fn new(minimum_cohort_size: NonZeroU32) -> Result<Self, StatisticsDisclosurePolicyError> {
        if minimum_cohort_size.get() < DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE {
            return Err(StatisticsDisclosurePolicyError::BelowPrivacyFloor);
        }
        Ok(Self {
            minimum_cohort_size,
        })
    }

    /// Returns the minimum releasable cohort size.
    pub fn minimum_cohort_size(self) -> u32 {
        self.minimum_cohort_size.get()
    }
}

impl Default for StatisticsDisclosurePolicy {
    fn default() -> Self {
        Self::new(
            NonZeroU32::new(DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE)
                .expect("default statistics cohort size is positive"),
        )
        .expect("default statistics cohort size meets the privacy floor")
    }
}

/// Browser-safe anonymous metrics for one immutable published question version.
///
/// This projection intentionally carries no tenant, student, enrollment,
/// course, assignment, run, attempt, response, source, or feedback identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionStatisticsView {
    /// Number of first-completed-run cohort contributions represented.
    pub cohort_size: u64,
    /// Mean normalized question score in the inclusive range `0.0..=1.0`.
    pub difficulty_index: f64,
    /// Mean number of submitted attempts represented by one cohort observation.
    pub attempts_mean: f64,
    /// Fixed-histogram estimate of the median response duration in seconds.
    pub time_median_seconds_estimate: u64,
    /// Pearson correlation of question score with rest-of-run score, when the
    /// cohort has enough variance to calculate it honestly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discrimination_index: Option<f64>,
}

/// Safe result of applying the k-anonymity gate to one shared aggregate.
///
/// `Suppressed` intentionally contains no count or partial metrics. Catalog
/// composition maps it to its established `Unavailable` wire status, keeping
/// existing unavailable responses stable while later adding safe metrics.
#[derive(Debug, Clone, PartialEq)]
pub enum QuestionStatisticsDisclosure {
    /// The aggregate is absent or below the disclosure threshold.
    Suppressed,
    /// The aggregate passed k-anonymity and can be shown as safe metrics.
    Available(QuestionStatisticsView),
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;

    #[test]
    fn policy_refuses_each_threshold_below_the_privacy_floor() {
        for threshold in 1..DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE {
            assert_eq!(
                StatisticsDisclosurePolicy::new(NonZeroU32::new(threshold).unwrap()),
                Err(StatisticsDisclosurePolicyError::BelowPrivacyFloor)
            );
        }
    }

    #[test]
    fn view_omits_unavailable_discrimination_instead_of_sending_null() {
        let view = QuestionStatisticsView {
            cohort_size: 5,
            difficulty_index: 0.7,
            attempts_mean: 1.2,
            time_median_seconds_estimate: 30,
            discrimination_index: None,
        };
        let value = serde_json::to_value(view).expect("safe view serializes");
        assert!(value.get("discriminationIndex").is_none());
    }

    #[test]
    fn learner_class_statistics_releases_only_current_complete_valid_cohorts() {
        assert_eq!(
            LearnerClassStatistics::from_current_analysis(5, false, false, Some(0.8)),
            LearnerClassStatistics::Available {
                completed_learner_cohort_size: 5,
                assignment_average_score: 0.8,
            }
        );
        for value in [
            LearnerClassStatistics::from_current_analysis(4, false, false, Some(0.8)),
            LearnerClassStatistics::from_current_analysis(5, true, false, Some(0.8)),
            LearnerClassStatistics::from_current_analysis(5, false, true, Some(0.8)),
            LearnerClassStatistics::from_current_analysis(5, false, false, None),
            LearnerClassStatistics::from_current_analysis(5, false, false, Some(f64::NAN)),
            LearnerClassStatistics::from_current_analysis(5, false, false, Some(1.1)),
        ] {
            assert_eq!(value, LearnerClassStatistics::InsufficientEvidence);
        }
    }

    #[test]
    fn learner_class_statistics_suppression_serializes_without_metrics() {
        let value = serde_json::to_value(LearnerClassStatistics::InsufficientEvidence)
            .expect("suppression serializes");
        assert_eq!(
            value,
            serde_json::json!({ "state": "insufficientEvidence" })
        );
    }
}
