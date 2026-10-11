//! Instructor-facing Question Library usage statistics projection.
//!
//! Counts are identity-free global aggregates. Rates are omitted when their
//! denominator is zero. Existing Library DTOs stay camelCase; this payload is
//! snake_case, matching the one allowed Library JSON change.

use serde::{Deserialize, Serialize};

use crate::QuestionRevisionNumber;

/// Identity-free issued, blank, answered, outcome, and credit-sum totals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuestionUsageTotals {
    /// Issued Question observations.
    pub issued_count: u64,
    /// Submitted with no saved response.
    pub blank_count: u64,
    /// Submitted with a graded saved response.
    pub answered_count: u64,
    /// Full-credit answered observations.
    pub correct_count: u64,
    /// Partial-credit answered observations.
    pub partial_count: u64,
    /// Zero-credit answered observations.
    pub incorrect_count: u64,
    pub issued_contributor_floor: u64,
    pub blank_contributor_floor: u64,
    pub answered_contributor_floor: u64,
    pub correct_contributor_floor: u64,
    pub partial_contributor_floor: u64,
    pub incorrect_contributor_floor: u64,
    /// Sum of normalized credit fractions.
    pub credit_sum: f64,
    /// Sum of squared normalized credit fractions.
    pub credit_sum_sq: f64,
}

impl QuestionUsageTotals {
    /// Totals with no observations.
    pub const fn empty() -> Self {
        Self {
            issued_count: 0,
            blank_count: 0,
            answered_count: 0,
            correct_count: 0,
            partial_count: 0,
            incorrect_count: 0,
            issued_contributor_floor: 0,
            blank_contributor_floor: 0,
            answered_contributor_floor: 0,
            correct_contributor_floor: 0,
            partial_contributor_floor: 0,
            incorrect_contributor_floor: 0,
            credit_sum: 0.0,
            credit_sum_sq: 0.0,
        }
    }

    fn rate(numerator: u64, denominator: u64) -> Option<f64> {
        if denominator == 0 {
            None
        } else {
            Some(numerator as f64 / denominator as f64)
        }
    }

    fn mean_credit(self) -> Option<f64> {
        if self.answered_count == 0 {
            None
        } else {
            Some(self.credit_sum / self.answered_count as f64)
        }
    }

    /// Per-Revision Instructor projection.
    pub fn into_revision_statistics(
        self,
        revision_number: QuestionRevisionNumber,
    ) -> QuestionRevisionUsageStatistics {
        QuestionRevisionUsageStatistics {
            revision_number,
            issued_count: self.issued_count,
            blank_count: self.blank_count,
            answered_count: self.answered_count,
            correct_count: self.correct_count,
            partial_count: self.partial_count,
            incorrect_count: self.incorrect_count,
            issued_contributor_floor: self.issued_contributor_floor,
            blank_contributor_floor: self.blank_contributor_floor,
            answered_contributor_floor: self.answered_contributor_floor,
            correct_contributor_floor: self.correct_contributor_floor,
            partial_contributor_floor: self.partial_contributor_floor,
            incorrect_contributor_floor: self.incorrect_contributor_floor,
            credit_sum: self.credit_sum,
            credit_sum_sq: self.credit_sum_sq,
            blank_rate: Self::rate(self.blank_count, self.issued_count),
            answered_rate: Self::rate(self.answered_count, self.issued_count),
            correct_rate: Self::rate(self.correct_count, self.answered_count),
            partial_rate: Self::rate(self.partial_count, self.answered_count),
            incorrect_rate: Self::rate(self.incorrect_count, self.answered_count),
            mean_credit: self.mean_credit(),
        }
    }

    /// Builds the Available wire shape after the caller applies its access policy.
    ///
    /// `revisions` is Question detail only. `pool_issued_count` is Pool detail only
    /// (`question_pool_statistics`).
    fn into_available(
        self,
        revisions: Option<Vec<QuestionRevisionUsageStatistics>>,
        pool_issued_count: Option<u64>,
    ) -> QuestionStatistics {
        QuestionStatistics::Available {
            issued_count: self.issued_count,
            blank_count: self.blank_count,
            answered_count: self.answered_count,
            correct_count: self.correct_count,
            partial_count: self.partial_count,
            incorrect_count: self.incorrect_count,
            credit_sum: self.credit_sum,
            credit_sum_sq: self.credit_sum_sq,
            blank_rate: Self::rate(self.blank_count, self.issued_count),
            answered_rate: Self::rate(self.answered_count, self.issued_count),
            correct_rate: Self::rate(self.correct_count, self.answered_count),
            partial_rate: Self::rate(self.partial_count, self.answered_count),
            incorrect_rate: Self::rate(self.incorrect_count, self.answered_count),
            mean_credit: self.mean_credit(),
            revisions,
            pool_issued_count,
        }
    }

    /// Shared Library statistics an Instructor may see.
    ///
    /// A zero observation count stays visible. A positive metric needs a distinct
    /// contributor lower bound at or above the minimum; an unknown or smaller
    /// cohort stays hidden. A zero floor with positive observations is unknown.
    /// The per-Revision list is omitted when any Revision has an unsafe cell. A
    /// Pool issue count uses its own contributor floor while a disclosable
    /// Question rollup remains.
    ///
    /// ASVS 14.2.6: the response carries no small-cell aggregate.
    pub fn into_shared_statistics(
        self,
        revisions: Option<Vec<QuestionRevisionUsageStatistics>>,
        pool_issued_count: Option<(u64, u64)>,
    ) -> QuestionStatistics {
        if !self.cohorts_are_disclosable() {
            return QuestionStatistics::Unavailable;
        }
        let pool_issued_count = pool_issued_count
            .filter(|(count, floor)| cohort_is_disclosable(*count, *floor))
            .map(|(count, _)| count);
        let revisions = revisions.filter(|rows| {
            rows.iter()
                .all(QuestionRevisionUsageStatistics::cohorts_are_disclosable)
        });
        self.into_available(revisions, pool_issued_count)
    }

    fn cohorts_are_disclosable(self) -> bool {
        cohort_is_disclosable(self.issued_count, self.issued_contributor_floor)
            && cohort_is_disclosable(self.blank_count, self.blank_contributor_floor)
            && cohort_is_disclosable(self.answered_count, self.answered_contributor_floor)
            && cohort_is_disclosable(self.correct_count, self.correct_contributor_floor)
            && cohort_is_disclosable(self.partial_count, self.partial_contributor_floor)
            && cohort_is_disclosable(self.incorrect_count, self.incorrect_contributor_floor)
    }
}

/// A lower bound of zero means the cohort is unknown unless its metric is also zero.
fn cohort_is_disclosable(actual_count: u64, contributor_floor: u64) -> bool {
    actual_count == 0
        || contributor_floor >= u64::from(crate::DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE)
}

/// Per-Revision breakdown shown only on the Question detail page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QuestionRevisionUsageStatistics {
    /// Exact Published Question Revision.
    pub revision_number: QuestionRevisionNumber,
    /// Issued Question observations for this Revision.
    pub issued_count: u64,
    /// Blank observations for this Revision.
    pub blank_count: u64,
    /// Answered observations for this Revision.
    pub answered_count: u64,
    /// Full-credit answered observations.
    pub correct_count: u64,
    /// Partial-credit answered observations.
    pub partial_count: u64,
    /// Zero-credit answered observations.
    pub incorrect_count: u64,
    #[serde(skip)]
    pub issued_contributor_floor: u64,
    #[serde(skip)]
    pub blank_contributor_floor: u64,
    #[serde(skip)]
    pub answered_contributor_floor: u64,
    #[serde(skip)]
    pub correct_contributor_floor: u64,
    #[serde(skip)]
    pub partial_contributor_floor: u64,
    #[serde(skip)]
    pub incorrect_contributor_floor: u64,
    /// Sum of normalized credit fractions.
    pub credit_sum: f64,
    /// Sum of squared normalized credit fractions.
    pub credit_sum_sq: f64,
    /// `blank_count / issued_count` when `issued_count > 0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blank_rate: Option<f64>,
    /// `answered_count / issued_count` when `issued_count > 0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answered_rate: Option<f64>,
    /// `correct_count / answered_count` when `answered_count > 0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct_rate: Option<f64>,
    /// `partial_count / answered_count` when `answered_count > 0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_rate: Option<f64>,
    /// `incorrect_count / answered_count` when `answered_count > 0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incorrect_rate: Option<f64>,
    /// `credit_sum / answered_count` when `answered_count > 0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean_credit: Option<f64>,
}

impl QuestionRevisionUsageStatistics {
    fn cohorts_are_disclosable(&self) -> bool {
        cohort_is_disclosable(self.issued_count, self.issued_contributor_floor)
            && cohort_is_disclosable(self.blank_count, self.blank_contributor_floor)
            && cohort_is_disclosable(self.answered_count, self.answered_contributor_floor)
            && cohort_is_disclosable(self.correct_count, self.correct_contributor_floor)
            && cohort_is_disclosable(self.partial_count, self.partial_contributor_floor)
            && cohort_is_disclosable(self.incorrect_count, self.incorrect_contributor_floor)
    }
}

/// Current privacy-governed Question Statistics availability.
///
/// Instructors receive Available with zeros when no observations exist.
/// Positive metrics stay hidden until their retained distinct-contributor floor
/// reaches the cohort minimum; unknown floors stay hidden too.
/// Students never receive this aggregate; Sysadmin Library reads stay
/// Unavailable. The `state` tag remains camelCase so Unavailable is
/// `{ "state": "unavailable" }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "snake_case",
    deny_unknown_fields
)]
pub enum QuestionStatistics {
    /// No Instructor-visible usage statistics for this reader.
    Unavailable,
    /// Current-Revision Question rollup or originating-Pool outcome rollup,
    /// with optional per-Revision rows on Question detail.
    Available {
        /// Observation count used as the blank/answered rate denominator.
        issued_count: u64,
        /// Blank Issued Questions.
        blank_count: u64,
        /// Answered Issued Questions.
        answered_count: u64,
        /// Full-credit answered observations.
        correct_count: u64,
        /// Partial-credit answered observations.
        partial_count: u64,
        /// Zero-credit answered observations.
        incorrect_count: u64,
        /// Sum of normalized credit fractions.
        credit_sum: f64,
        /// Sum of squared normalized credit fractions.
        credit_sum_sq: f64,
        /// `blank_count / issued_count` when `issued_count > 0`.
        #[serde(skip_serializing_if = "Option::is_none")]
        blank_rate: Option<f64>,
        /// `answered_count / issued_count` when `issued_count > 0`.
        #[serde(skip_serializing_if = "Option::is_none")]
        answered_rate: Option<f64>,
        /// `correct_count / answered_count` when `answered_count > 0`.
        #[serde(skip_serializing_if = "Option::is_none")]
        correct_rate: Option<f64>,
        /// `partial_count / answered_count` when `answered_count > 0`.
        #[serde(skip_serializing_if = "Option::is_none")]
        partial_rate: Option<f64>,
        /// `incorrect_count / answered_count` when `answered_count > 0`.
        #[serde(skip_serializing_if = "Option::is_none")]
        incorrect_rate: Option<f64>,
        /// `credit_sum / answered_count` when `answered_count > 0`.
        #[serde(skip_serializing_if = "Option::is_none")]
        mean_credit: Option<f64>,
        /// Per-Revision rows; omitted from bulk search and Pool member rollups.
        #[serde(skip_serializing_if = "Option::is_none")]
        revisions: Option<Vec<QuestionRevisionUsageStatistics>>,
        /// Pool `question_pool_statistics.issued_count` when this payload is a
        /// Pool. Outcome counts use receipts attributed to the originating Pool.
        #[serde(skip_serializing_if = "Option::is_none")]
        pool_issued_count: Option<u64>,
    },
}
