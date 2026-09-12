//! Student-authorized Course and released Assignment landing projections.
//!
//! The landing boundary exposes only public Course and Assignment references
//! with learner-facing titles. PostgreSQL derives the current active Student
//! Account and exact active Student Course Membership before it projects them.

use async_trait::async_trait;
use question_model::{AssignmentAttemptCompletion, AssignmentReference, CourseInstanceReference};
use serde::Serialize;

use crate::{LiveAssignmentAttemptScore, SessionTokenHash, StoreError};

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

/// One released Assignment available from an authorized Student Course.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStudentAssignmentLandingSummary {
    /// Public Assignment reference, never an internal Assignment identity.
    pub assignment: AssignmentReference,
    /// Student-facing released Assignment title.
    pub title: String,
    /// One-based current Assignment Attempt number, or none before work starts.
    pub assignment_attempt_number: Option<u32>,
    /// Current Assignment Attempt completion, or none when work has not started.
    pub assignment_attempt_completion: Option<AssignmentAttemptCompletion>,
    /// Questions with an immutable Grading Result in the current Assignment Attempt.
    pub graded_question_count: u32,
    /// Total questions in the current Assignment; an existing Attempt retains its issued-question evidence.
    pub question_count: u32,
    /// Current aggregate score when the pinned Assignment disclosure permits it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<LiveAssignmentAttemptScore>,
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

    /// Lists released Assignments after exact Course-membership authorization.
    ///
    /// A foreign, malformed, inactive, or ended Course is concealed as
    /// [`StoreError::Forbidden`]; an authorized Course without releases returns
    /// an empty list.
    async fn list_released_live_student_assignments(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<LiveStudentAssignmentLandingSummary>, StoreError>;
}
