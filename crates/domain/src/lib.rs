//! MOD-DOMAIN: Assignment Attempt state, timing, generation, and validation.
//!
//! This crate reaches `question_model` and nothing else, so it has no clock
//! and no database. Time and storage arrive as parameters, which is what lets
//! the same code run on the server and in the browser through `wasm_bridge`
//! and makes the seed-parity test meaningful.

/// Pure current Student Assignment Access evaluation. Persistence supplies normalized facts;
/// this module never reads a roster, database, clock, or browser token.
pub mod active_student_course_membership;
/// Continued-practice eligibility and shared Assignment Activity errors.
pub mod assignment_activity;
/// Current course-owned Assignment Question Analysis projections (MOD-STATS).
pub mod assignment_question_analysis;
/// Attempt state machine (MOD-STATE).
/// Completion derivation within an Assignment Attempt (MOD-STATE).
pub mod completion;
/// Pure course-grade aggregation from selected assignment scores.
pub mod course_grade;
/// Key-free deterministic workspace-draft prompt preview (MOD-WASM).
pub mod draft_preview;
/// Pure current assignment-policy resolution after Student Assignment Access.
pub mod effective_assignment_policy;
/// Seeded question generation (MOD-GEN).
pub mod generator;
/// Assignment configuration validation (MOD-CAP).
pub mod policy;
/// Pure non-mutating S5 -> S3 -> S4 preview composition (WP-INST-T3).
pub mod preview_plane;
/// Server-owned exact Question Pool Item selection.
pub mod question_pool_selection;
/// Completed Assignment Attempt score selection and summary projection (MOD-SCORE).
pub mod scoring;
/// Retention-safe anonymous question-statistics aggregation (MOD-STATS).
pub mod statistics;
/// Pure evaluation of the Assignment-owned Student Feedback Release Rule (WP-INST-S4).
pub mod student_feedback_release;
/// Pure Course Membership and Course Invitation authority validation (WP-INST-T2).
pub mod teaching_authority;
/// Timer verdict for time-limited attempts (MOD-TIME).
pub mod timing;
/// Browser-safe student-response format validation (MOD-GRD boundary).
pub mod validation;

pub use crate::course_grade::{
    CourseGradeAssignment, CourseGradeError, CourseGradeOutcome, CourseGradeUnavailableReason,
    calculate_course_grade,
};
pub use crate::question_pool_selection::{
    QuestionPoolSelectionEntropy, QuestionPoolSelectionError, select_question_pool_items,
};
pub use crate::teaching_authority::{
    CourseInvitationAcceptance, CourseInvitationError, DirectInstructorMembership,
    InstructorAuthority, InstructorMembershipRemovalError, StudentCourseMembership,
    accept_course_invitation, current_course_instructor, evaluate_course_instructor_authority,
    invitation_state, refuse_final_instructor_removal, student_owns_course_record,
};
