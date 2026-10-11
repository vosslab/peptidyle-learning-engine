//! Self-only Course-wide Student Attempt History wire contract.

use question_model::{AssessmentAttemptId, AssessmentId, Timestamp};
use serde::{Deserialize, Serialize};

use crate::student_course_progress::AssessmentPointScore;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentCourseAttemptHistoryEntry {
    pub assessment_attempt_id: AssessmentAttemptId,
    pub assessment_id: AssessmentId,
    pub assessment_title: String,
    pub assessment_attempt_number: u32,
    pub started_at: Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<Timestamp>,
    /// Present only when this Attempt's score is complete and disclosed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment_score: Option<AssessmentPointScore>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentCourseAttemptHistoryPage {
    pub items: Vec<StudentCourseAttemptHistoryEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}
