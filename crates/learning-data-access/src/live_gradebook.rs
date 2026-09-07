//! Answer-free Instructor Gradebook projection of immutable Grading Results.

use async_trait::async_trait;
use question_model::CourseInstanceReference;
use serde::Serialize;

use crate::{SessionTokenHash, StoreError};

/// One Course-local row derived only from completed immutable Grading Results.
/// It deliberately carries no Student Response, Answer Key, source, or grader
/// internals.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveDemoGradedStudentWork {
    pub roster_id: String,
    pub assignment_reference: question_model::AssignmentReference,
    pub graded_question_count: u32,
    pub points_earned: f64,
    pub points_possible: f64,
}

/// The current Course Instructor's answer-free Gradebook evidence projection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveDemoGradebook {
    pub course_reference: CourseInstanceReference,
    pub graded_student_work: Vec<LiveDemoGradedStudentWork>,
}

#[async_trait]
pub trait LiveDemoGradebookStore: Send + Sync {
    /// Reads the exact Course Instructor's immutable graded Student Work
    /// projection. Foreign and unavailable Courses remain concealed.
    async fn live_demo_gradebook(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<LiveDemoGradebook, StoreError>;
}
