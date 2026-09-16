//! Shared Assessment Activity completion types and errors.

pub use question_model::AssessmentAttemptCompletion;

/// A rejected Assessment Attempt model input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssessmentActivityError {
    /// A score was non-finite or outside `-1000.0..=1000.0`.
    InvalidScore {
        /// Rejected score.
        score: f64,
    },
    /// A summary counter reached its numeric limit.
    SummaryCounterOverflow,
}

impl std::fmt::Display for AssessmentActivityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidScore { score } => {
                write!(formatter, "invalid Assessment Attempt score: {score}")
            }
            Self::SummaryCounterOverflow => write!(formatter, "summary counter overflow"),
        }
    }
}

impl std::error::Error for AssessmentActivityError {}
