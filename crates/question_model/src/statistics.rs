//! Browser-safe Class Statistics.
//!
//! Question Library Question Statistics are owned by `question_library`; this
//! module holds only the separately course-local Class Statistics.

use serde::{Deserialize, Serialize};

/// Default minimum number of independent cohort contributions before metrics
/// may be disclosed.
pub const DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE: u32 = 5;

/// Student-safe result of considering the current course-local assessment
/// analysis for class-statistics disclosure.
///
/// This deliberately contains neither an analysis identity nor partial
/// evidence: a Student either receives the two safe aggregate values or no
/// cohort count/metric at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "snake_case"
)]
pub enum ClassStatistics {
    /// No current, complete, k-anonymous analysis can safely be disclosed.
    Unavailable,
    /// Anonymous aggregate metrics for a completed-Student cohort.
    Available {
        /// Number of distinct Students represented by their latest completed Assessment Attempt.
        completed_student_cohort_size: u32,
        /// Mean normalized assessment score, in the inclusive range `0.0..=1.0`.
        assessment_average_score: f64,
    },
}

impl ClassStatistics {
    /// Gates a current course-local analysis before it reaches a Student.
    ///
    /// The caller obtains the values only from the current report. Missing or
    /// stale reports, incomplete automated scoring, insufficient cohort evidence,
    /// and malformed scores all collapse to the identity-free suppressed state.
    pub fn from_current_analysis(
        completed_student_cohort_size: u32,
        incomplete_scoring: bool,
        recent_rescoring: bool,
        assessment_average_score: Option<f64>,
    ) -> Self {
        let valid_average = assessment_average_score
            .filter(|score| score.is_finite() && (0.0..=1.0).contains(score));
        if completed_student_cohort_size < DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE
            || incomplete_scoring
            || recent_rescoring
        {
            return Self::Unavailable;
        }
        match valid_average {
            Some(assessment_average_score) => Self::Available {
                completed_student_cohort_size,
                assessment_average_score,
            },
            None => Self::Unavailable,
        }
    }
}
