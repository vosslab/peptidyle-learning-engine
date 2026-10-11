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
