//! Student-authorized Course and released Assessment landing projections.
//!
//! The landing boundary exposes only public Course and Assessment references
//! with learner-facing titles. PostgreSQL derives the current active Student
//! Account and exact active Student Course Membership before it projects them.

use async_trait::async_trait;
use browser_api_contract::student_assessment_decision::StudentAssessmentDecisionSummary;
use question_model::{
    AssessmentAttemptCompletion, AssessmentReference, AssessmentType, CourseInstanceReference,
};
use serde::Serialize;

use crate::{SessionTokenHash, StoreError};

/// One point-based Assessment grade contribution selected across submitted Attempts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveAssessmentGradeContribution {
    pub points_earned: f64,
    pub points_possible: f64,
}

/// One active Student Course Instance available from the landing page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStudentCourseLandingSummary {
    /// Public Course Instance reference, never an internal Course identity.
    pub course: CourseInstanceReference,
    /// Compact Course Instance name for constrained navigation.
    pub short_name: String,
    /// Descriptive Course Instance name for headings and lists.
    pub long_name: String,
}

/// One pending Student Course Invitation available from the landing page.
///
/// This remains distinct from [`LiveStudentCourseLandingSummary`]: a pending
/// Course Invitation is not an active Student Course Membership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStudentCourseInvitationSummary {
    /// Public Course Instance reference, never an invitation or Account identity.
    pub course: CourseInstanceReference,
    /// Compact Course Instance name for constrained navigation.
    pub short_name: String,
    /// Descriptive Course Instance name for headings and lists.
    pub long_name: String,
}

/// One released Assessment available from an authorized Student Course.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStudentAssessmentLandingSummary {
    /// Public Assessment reference, never an internal Assessment identity.
    pub assessment: AssessmentReference,
    /// Student-facing released Assessment title.
    pub title: String,
    /// Product-defined pedagogical Type for this Assessment.
    pub assessment_type: AssessmentType,
    /// Same server-owned policy and start decision returned by Assessment Access.
    pub decision: StudentAssessmentDecisionSummary,
    /// One-based current Assessment Attempt number, or none before work starts.
    pub assessment_attempt_number: Option<u32>,
    /// Current Assessment Attempt completion, or none when work has not started.
    pub assessment_attempt_completion: Option<AssessmentAttemptCompletion>,
    /// Whether the ordinary start action would resume an existing Assessment Attempt.
    pub can_resume_assessment_attempt: bool,
    /// Questions with an immutable Grading Result in the current Assessment Attempt.
    pub graded_question_count: u32,
    /// Total questions in the current Assessment; an existing Attempt retains its issued-question evidence.
    pub question_count: u32,
    /// Highest submitted Assessment score when that Attempt's disclosure permits it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment_score: Option<LiveAssessmentGradeContribution>,
}

/// Session-authorized persistence boundary for the Student Course landing.
#[async_trait]
pub trait LiveStudentCourseLandingStore: Send + Sync {
    /// Lists only Course Instances with an active Student Course Membership.
    async fn list_live_student_courses(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<LiveStudentCourseLandingSummary>, StoreError>;

    /// Lists only the authenticated Student's pending, unexpired Course Invitations
    /// for Courses without an active Student Course Membership.
    ///
    /// The projection intentionally omits invitation, Account, Student Record,
    /// and membership identities. Accepting an invitation remains the separate
    /// exact-transaction Course Roster operation.
    async fn list_pending_live_student_course_invitations(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<LiveStudentCourseInvitationSummary>, StoreError>;

    /// Lists released Assessments after exact Course-membership authorization.
    ///
    /// A foreign, malformed, inactive, or ended Course is concealed as
    /// [`StoreError::Forbidden`]; an authorized Course without releases returns
    /// an empty list.
    async fn list_released_live_student_assessments(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<LiveStudentAssessmentLandingSummary>, StoreError>;
}
