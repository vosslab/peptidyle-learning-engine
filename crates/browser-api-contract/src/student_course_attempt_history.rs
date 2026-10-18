//! Self-only global Student Attempt History wire contract.

use question_model::{AssessmentAttemptId, AssessmentId, CourseInstanceId, Timestamp};
use serde::{Deserialize, Serialize};

use crate::student_course_progress::AssessmentPointScore;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentAttemptHistoryEntry {
    pub course_instance_id: CourseInstanceId,
    pub course_short_name: String,
    pub course_long_name: String,
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
pub struct StudentAttemptHistoryPage {
    pub items: Vec<StudentAttemptHistoryEntry>,
    pub page_number: u32,
    pub page_size: u16,
    pub total_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentAttemptHistoryFacetChoice {
    pub key: String,
    pub label: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentAttemptHistoryFacetPage {
    pub choices: Vec<StudentAttemptHistoryFacetChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}
