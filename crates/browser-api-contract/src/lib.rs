#![forbid(unsafe_code)]

//! Pure route-only browser API contract values.
//!
//! The admission front door is intentionally empty until a Server Route owns
//! its first DTO. Values introduced here remain owned, runtime-
//! free, and fallible-behavior-free; Axum, persistence, application state, and
//! project tooling stay outside this product boundary.

pub mod assessment_delivery;
pub mod blueprint_change_proposal;
pub mod blueprint_course;
pub mod student_assessment_decision;
pub mod student_course_attempt_history;
pub mod student_course_practice_stats;
pub mod student_course_progress;
pub mod student_question_display_duration;
