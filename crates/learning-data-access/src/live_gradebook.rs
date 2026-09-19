//! Answer-free Instructor Gradebook projection of immutable Grading Results.

use async_trait::async_trait;
use question_model::CourseInstanceId;
use serde::Serialize;

use crate::{LiveAssessmentAttemptScore, SessionTokenHash, StoreError};

/// One Course-local answer-free progress row for an active Student.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseGradebookStudentWork {
    pub roster_id: String,
    /// Instructor-provided, exact-Course roster label.
    pub roster_name: String,
    pub assessment_id: question_model::AssessmentId,
    pub assessment_title: String,
    pub assessment_attempt_completion: Option<question_model::AssessmentAttemptCompletion>,
    /// Derived only from server time and durable submission evidence.
    pub expired_submitting: bool,
    /// Missing only until background submission records immutable outcomes.
    /// This is a Gradebook contribution pair: Bonus work can contribute earned
    /// points with zero possible, and extra credit can exceed possible points.
    pub score: Option<LiveAssessmentAttemptScore>,
}

/// The current Course Instructor's answer-free Gradebook evidence projection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseGradebook {
    pub course_id: CourseInstanceId,
    pub student_work: Vec<CourseGradebookStudentWork>,
}

#[async_trait]
pub trait CourseGradebookStore: Send + Sync {
    /// Reads the exact Course Instructor's answer-free Student Work progress.
    /// Foreign and unavailable Courses remain concealed.
    async fn course_gradebook(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceId,
    ) -> Result<CourseGradebook, StoreError>;
}
