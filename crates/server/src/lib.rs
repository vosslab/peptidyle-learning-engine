//! MOD-SRV: the API server core.
//!
//! Route groups (auth, catalog, course, run, asset, validation) land in M2.
//! The deployment composition root remains intentionally small until its later
//! work package.

/// Public-CDN and authorized short-lived asset delivery.
pub mod asset;
/// Authentication, sessions, and the middleware stack.
pub mod auth;
/// Catalog, publication, taxonomy, and content-lifecycle routes.
pub mod catalog;
/// Tenant courses, course-local membership, and assignment routes.
pub mod course;
/// Readiness reporting for the container health check.
pub mod health;
/// Student runs, question attempts, submissions, and grading summaries.
pub mod run;
/// Authenticated, key-free fallbacks for browser-safe pure validation.
pub mod validation;
