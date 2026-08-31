//! MOD-DOMAIN: attempt state, runs, timing, generation, and validation.
//!
//! This crate reaches `question_model` and nothing else, so it has no clock
//! and no database. Time and storage arrive as parameters, which is what lets
//! the same code run on the server and in the browser through `wasm_bridge`
//! and makes the seed-parity test meaningful.

/// Attempt state machine (MOD-STATE).
/// Completion derivation within a run (MOD-STATE).
pub mod completion;
/// Pure course-grade aggregation from selected assignment scores.
pub mod course_grade;
/// Pure evaluation of assignment-owned student disclosure policy (WP-INST-S4).
pub mod disclosure_policy;
/// Key-free deterministic workspace-draft prompt preview (MOD-WASM).
pub mod draft_preview;
/// Pure current assignment-policy resolution after S5 entitlement.
pub mod effective_assignment_policy;
/// Pure current-entitlement evaluation. Persistence supplies normalized facts;
/// this module never reads a roster, database, clock, or browser token.
pub mod entitlement;
/// Seeded question generation (MOD-GEN).
pub mod generator;
/// Current course-owned item-analysis projections (MOD-STATS).
pub mod item_analysis;
/// Assignment configuration validation (MOD-CAP).
pub mod policy;
/// Pure non-mutating S5 -> S3 -> S4 preview composition (WP-INST-T3).
pub mod preview_plane;
/// Continued-practice eligibility and shared run-model errors (MOD-RUN).
pub mod run;
/// Completed-run score selection and summary projection (MOD-SCORE).
pub mod scoring;
/// Retention-safe anonymous question-statistics aggregation (MOD-STATS).
pub mod statistics;
/// Pure group-membership and co-instructor authority validation (WP-INST-T2).
pub mod teaching_authority;
/// Timer verdict for time-limited attempts (MOD-TIME).
pub mod timing;
/// Browser-safe student-response format validation (MOD-GRD boundary).
pub mod validation;

pub use crate::course_grade::{
    CourseGradeAssignment, CourseGradeError, CourseGradeOutcome, CourseGradeUnavailableReason,
    calculate_course_grade,
};
pub use crate::teaching_authority::{
    CourseInvitationAcceptance, CourseInvitationError, DirectInstructorMembership,
    InstructorAuthority, InstructorMembershipRemovalError, StudentCourseMembership,
    accept_course_invitation, approved_instructor, current_course_instructor,
    evaluate_course_instructor_authority, invitation_state,
    refuse_final_instructor_removal, student_owns_course_record, validate_instructor_approval,
};
