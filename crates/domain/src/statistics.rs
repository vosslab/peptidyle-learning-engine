//! Retention-safe anonymous question-statistics facade.
//!
//! Aggregation keeps server-only sufficient statistics; disclosure owns the
//! k-anonymity-gated browser projection.  This facade preserves the stable
//! `domain::statistics` public API while keeping those capabilities separate.

mod aggregation;
mod disclosure;
mod version_counts;

pub use aggregation::{
    DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS, DURATION_HISTOGRAM_VERSION, DurationHistogram,
    DurationHistogramSnapshot, MAX_DURATION_SECONDS, PearsonMomentSnapshot, PearsonSufficientSums,
    QuestionCohortRollup, QuestionCohortRollupObservation, QuestionCohortRollupSnapshot,
    StatisticsError,
};
pub use version_counts::{
    QuestionStatisticsObservation, QuestionVersionStatistics, QuestionVersionStatisticsSnapshot,
};

#[cfg(test)]
#[path = "statistics/tests.rs"]
mod tests;
