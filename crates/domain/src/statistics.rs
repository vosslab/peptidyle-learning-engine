//! Exact Question Revision Statistics domain values.
//!
//! The persisted aggregate owns accepted-grade counts. A later Question
//! Statistics release service may apply its privacy rule, but this module does
//! not model an unimplemented global cohort calculation or release shape.

mod version_counts;

pub use version_counts::{QuestionRevisionStatistics, QuestionStatisticsObservation};

/// A rejected exact-count Question Statistics operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticsError {
    /// An eligible choice ID cannot identify a selection count.
    InvalidChoiceIdentifier,
    /// An integer aggregate counter could not represent another contribution.
    CounterOverflow,
}

impl std::fmt::Display for StatisticsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidChoiceIdentifier => {
                formatter.write_str("statistics choice identifier must be nonempty")
            }
            Self::CounterOverflow => formatter.write_str("statistics counter overflow"),
        }
    }
}

impl std::error::Error for StatisticsError {}

#[cfg(test)]
#[path = "statistics/tests.rs"]
mod tests;
