//! PLE server core for the clean single-installation baseline.
//!
//! The current executable surface is deliberately small: one global Account
//! session boundary plus the deployment-gated seeded Live Demo entry. Course,
//! Question Library, and delivery routes return only after their fresh Store contracts
//! and PostgreSQL capabilities are reconstructed on this foundation.

/// Authentication, sessions, and the first-party browser boundary.
pub mod auth;
/// Production database/session composition.
pub mod composition;
/// Readiness probe support for the executable process.
pub mod health;
/// Uniform dynamic-response security headers.
pub(crate) mod http_security;
/// Process-wide safe request lifecycle handling.
pub mod request_lifecycle;
