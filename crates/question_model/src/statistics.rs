//! Browser-safe Class Statistics.
//!
//! Question Library Question Statistics are owned by `question_library`; this
//! module holds only the separately course-local Class Statistics.

use serde::{Deserialize, Serialize};

/// Default minimum number of independent cohort contributions before metrics
/// may be disclosed.
pub const DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE: u32 = 5;

/// Student-safe result of considering the current course-local assignment
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
        /// Number of distinct Students represented by their latest completed Assignment Attempt.
        completed_student_cohort_size: u32,
        /// Mean normalized assignment score, in the inclusive range `0.0..=1.0`.
        assignment_average_score: f64,
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
        assignment_average_score: Option<f64>,
    ) -> Self {
        let valid_average = assignment_average_score
            .filter(|score| score.is_finite() && (0.0..=1.0).contains(score));
        if completed_student_cohort_size < DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE
            || incomplete_scoring
            || recent_rescoring
        {
            return Self::Unavailable;
        }
        match valid_average {
            Some(assignment_average_score) => Self::Available {
                completed_student_cohort_size,
                assignment_average_score,
            },
            None => Self::Unavailable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_statistics_release_only_current_complete_valid_cohorts() {
        assert_eq!(
            ClassStatistics::from_current_analysis(5, false, false, Some(0.8)),
            ClassStatistics::Available {
                completed_student_cohort_size: 5,
                assignment_average_score: 0.8,
            }
        );
        for value in [
            ClassStatistics::from_current_analysis(4, false, false, Some(0.8)),
            ClassStatistics::from_current_analysis(5, true, false, Some(0.8)),
            ClassStatistics::from_current_analysis(5, false, true, Some(0.8)),
            ClassStatistics::from_current_analysis(5, false, false, None),
            ClassStatistics::from_current_analysis(5, false, false, Some(f64::NAN)),
            ClassStatistics::from_current_analysis(5, false, false, Some(1.1)),
        ] {
            assert_eq!(value, ClassStatistics::Unavailable);
        }
    }

    #[test]
    fn unavailable_class_statistics_serialize_without_metrics() {
        let value =
            serde_json::to_value(ClassStatistics::Unavailable).expect("suppression serializes");
        assert_eq!(value, serde_json::json!({ "state": "unavailable" }));
    }

    #[test]
    fn available_class_statistics_use_direct_snake_case() {
        let value = serde_json::to_value(ClassStatistics::Available {
            completed_student_cohort_size: 5,
            assignment_average_score: 0.8,
        })
        .expect("available statistics serialize");
        assert_eq!(
            value,
            serde_json::json!({
                "state": "available",
                "completed_student_cohort_size": 5,
                "assignment_average_score": 0.8
            })
        );
    }
}
