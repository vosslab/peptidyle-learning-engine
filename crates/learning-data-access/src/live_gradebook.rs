//! Answer-free Instructor Gradebook projection of immutable Grading Results.

use async_trait::async_trait;
use question_model::CourseInstanceReference;
use serde::Serialize;

use crate::{LiveAssessmentAttemptScore, SessionTokenHash, StoreError};

/// One Course-local answer-free progress row for an active Student.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseGradebookStudentWork {
    pub roster_id: String,
    pub assessment_reference: question_model::AssessmentReference,
    pub assessment_attempt_completion: Option<question_model::AssessmentAttemptCompletion>,
    /// Derived only from server time and durable submission evidence.
    pub expired_submitting: bool,
    /// Missing only until background submission records immutable outcomes.
    pub score: Option<LiveAssessmentAttemptScore>,
}

/// The current Course Instructor's answer-free Gradebook evidence projection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseGradebook {
    pub course_reference: CourseInstanceReference,
    pub student_work: Vec<CourseGradebookStudentWork>,
}

#[async_trait]
pub trait CourseGradebookStore: Send + Sync {
    /// Reads the exact Course Instructor's answer-free Student Work progress.
    /// Foreign and unavailable Courses remain concealed.
    async fn course_gradebook(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<CourseGradebook, StoreError>;
}
