//! Browser-safe statistics disclosure.
//!
//! This module is the sole owner of the public projection.  Aggregation stays
//! server-only until the configured k-anonymity threshold is met.

use question_model::{
    QuestionStatisticsDisclosure, QuestionStatisticsView, StatisticsDisclosurePolicy,
};

use super::QuestionCohortRollup;

impl QuestionCohortRollup {
    /// Applies k-anonymity disclosure to construct the only browser-safe view.
    pub fn disclose(&self, policy: StatisticsDisclosurePolicy) -> QuestionStatisticsDisclosure {
        let minimum_cohort_size = u64::from(policy.minimum_cohort_size());
        if self.cohort_size() < minimum_cohort_size {
            return QuestionStatisticsDisclosure::Suppressed;
        }
        let Some(difficulty_index) = self.difficulty_index() else {
            return QuestionStatisticsDisclosure::Suppressed;
        };
        let Some(attempts_mean) = self.attempts_mean() else {
            return QuestionStatisticsDisclosure::Suppressed;
        };
        let Some(time_median_seconds_estimate) = self.time_median_seconds_estimate() else {
            return QuestionStatisticsDisclosure::Suppressed;
        };
        QuestionStatisticsDisclosure::Available(QuestionStatisticsView {
            cohort_size: self.cohort_size(),
            difficulty_index,
            attempts_mean,
            time_median_seconds_estimate,
            discrimination_index: (self.scored_cohort_size() >= minimum_cohort_size)
                .then(|| self.discrimination_index())
                .flatten(),
        })
    }
}
