//! PLE server core for the single-installation application.
//!
//! The executable composes authentication, Live Demo, Question Library,
//! authoring, Blueprint Course, Course Instance, roster, Account management,
//! Gradebook, Assignment, asset-delivery, and support-capability routes over
//! their typed Store contracts and PostgreSQL capabilities.

/// Student Assignment Access and answer-free initial delivery routes.
pub(crate) mod assignment_delivery;
/// Direct-Instructor Assignment Workspace and immutable release routes.
pub(crate) mod assignment_release;
/// Authentication, sessions, and the first-party browser boundary.
pub mod auth;
/// Private Authoring Workspace and Draft Question routes.
pub mod authoring;
/// Instructor-owned reusable Blueprint Course routes.
pub(crate) mod blueprint_course;
/// Production database/session composition.
pub mod composition;
/// Course Instance creation and initial teaching-team routes.
pub(crate) mod course_instance;
/// Course Roster Import, invitation claim, and access-revocation routes.
pub(crate) mod course_roster;
/// Readiness probe support for the executable process.
pub mod health;
/// Uniform dynamic-response security headers.
pub(crate) mod http_security;
/// Sysadmin-only Instructor Account management routes.
pub(crate) mod instructor_account;
/// Protected direct-Instructor invitation-mailer export route.
pub(crate) mod invitation_export;
pub(crate) mod live_gradebook;
/// Student-only Course and released Assignment landing routes.
pub(crate) mod live_student_course_landing;
/// One-shot immutable public Question Asset publisher with no HTTP surface.
pub mod public_asset_publisher;
/// Authorized immutable public Question Asset redirect route.
pub(crate) mod question_asset_delivery;
/// Instructor Question Library browse and answer-free detail routes.
mod question_library;
/// Server-only verified Question Publication coordination.
pub mod question_publication;
/// Process-wide safe request lifecycle handling.
pub mod request_lifecycle;
/// Exact-course, direct-Instructor support-capability routes.
pub(crate) mod support_capability;
/// Least-privilege worker process lifecycle with no browser or HTTP listener.
pub mod worker;
