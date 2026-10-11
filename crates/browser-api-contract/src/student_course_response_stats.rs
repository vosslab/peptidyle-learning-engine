//! Self-only exact Published Question Revision Response Stats contract.

use question_model::{AssessmentAttemptId, PublishedQuestionRevisionTuple};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentCourseResponseQuestionStats {
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// Current lineage title. Empty when that title is absent.
    pub question_title: String,
    pub full_credit_attempt_count: u64,
    pub partial_credit_attempt_count: u64,
    pub incorrect_attempt_count: u64,
    pub unanswered_attempt_count: u64,
    pub disclosed_attempt_count: u64,
    pub not_full_credit_count: u64,
    pub average_display_duration_ms: Option<f64>,
    pub display_duration_sample_count: u64,
    pub relevant_assessment_attempt_id: AssessmentAttemptId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentCourseResponseStats {
    pub questions: Vec<StudentCourseResponseQuestionStats>,
}
