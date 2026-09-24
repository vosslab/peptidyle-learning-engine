//! Browser-safe self-only Course Progress projection.

use question_model::{AssessmentAttemptCompletion, AssessmentId, AssessmentType, Timestamp};
use serde::{Deserialize, Serialize};

/// One point-based Assessment score already authorized for Student disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentPointScore {
    pub points_earned: f64,
    pub points_possible: f64,
}

/// One released Assessment's Attempt activity and currently disclosed score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentCourseProgressAssessment {
    pub id: AssessmentId,
    pub title: String,
    pub assessment_type: AssessmentType,
    pub assessment_attempt_count: u32,
    pub submitted_assessment_attempt_count: u32,
    pub latest_assessment_attempt_number: Option<u32>,
    pub latest_assessment_attempt_completion: Option<AssessmentAttemptCompletion>,
    /// Latest observed Attempt start, saved response, or submission in Unix milliseconds.
    pub latest_activity_at: Option<Timestamp>,
    /// Omitted when the selected highest submitted Attempt's score is not released.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment_score: Option<AssessmentPointScore>,
    /// Omitted whenever the score is withheld, preserving score freshness confidentiality.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment_score_is_latest_attempt: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn withheld_score_keeps_attempt_activity_without_freshness_or_score_fields() {
        let progress = StudentCourseProgressAssessment {
            id: AssessmentId::new("A5D9Q3XAH").expect("Assessment ID"),
            title: "Peptide practice".into(),
            assessment_type: serde_json::from_value(serde_json::json!("regular_assignment"))
                .expect("Assessment Type"),
            assessment_attempt_count: 2,
            submitted_assessment_attempt_count: 1,
            latest_assessment_attempt_number: Some(2),
            latest_assessment_attempt_completion: Some(AssessmentAttemptCompletion::InProgress),
            latest_activity_at: Some(Timestamp::from_unix_millis(1_786_000_000_000)),
            assessment_score: None,
            assessment_score_is_latest_attempt: None,
        };

        let wire = serde_json::to_value(progress).expect("Course Progress serializes");
        assert_eq!(wire["assessmentAttemptCount"], 2);
        assert_eq!(wire["submittedAssessmentAttemptCount"], 1);
        assert_eq!(wire["latestAssessmentAttemptNumber"], 2);
        assert_eq!(wire["latestActivityAt"], 1_786_000_000_000_i64);
        assert!(wire.get("assessmentScore").is_none());
        assert!(wire.get("assessmentScoreIsLatestAttempt").is_none());
    }
}
