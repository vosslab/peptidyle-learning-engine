//! PLE server core for the clean single-installation baseline.
//!
//! The current executable surface is deliberately small: one global Account
//! session boundary plus the deployment-gated seeded Live Demo entry. Course,
//! Question Library, and delivery routes return only after their fresh Store contracts
//! and PostgreSQL capabilities are reconstructed on this foundation.

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
/// Instructor Question Library browse and answer-free detail routes.
mod question_library;
/// Server-only verified Question Publication coordination.
pub mod question_publication;
/// Process-wide safe request lifecycle handling.
pub mod request_lifecycle;
/// Least-privilege worker process lifecycle with no browser or HTTP listener.
pub mod worker;
