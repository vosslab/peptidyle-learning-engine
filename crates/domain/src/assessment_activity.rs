//! Shared Assessment Activity projection types and errors.
//!
//! Scoring owns compact grade and progress transitions. Assessment completion
//! is projected separately from the authoritative submission timestamp.

pub use question_model::AssessmentAttemptCompletion;

pub use crate::scoring::{AssessmentActivityTransition, project_assessment_activity};

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
