//! MOD-SRV: the API server core.
//!
//! Route groups (auth, catalog, course, run, asset) land in M2. What exists
//! today is the health surface WP-F4 gates the containers on.

/// Authentication, sessions, and the middleware stack.
pub mod auth;
/// Readiness reporting for the container health check.
pub mod health;
