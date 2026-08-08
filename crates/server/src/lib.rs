//! MOD-SRV: the API server core.
//!
//! Route groups (auth, catalog, course, run, asset, validation) land in M2.
//! The deployment composition root remains intentionally small until its later
//! work package.

/// Public-CDN and authorized short-lived asset delivery.
pub mod asset;
/// Authentication, sessions, and the middleware stack.
pub mod auth;
/// Protected instructor answer presentation for private workspace drafts.
pub mod author_preview;
/// Catalog, publication, taxonomy, and content-lifecycle routes.
pub mod catalog;
/// Server-side dispatch across installed trusted question backends.
pub mod composite_backend;
/// Tenant courses, course-local membership, and assignment routes.
pub mod course;
/// Instructor-authorized asynchronous assignment export requests.
pub mod export;
/// Frozen assignment export preparation and atomic four-artifact finalization.
pub mod export_worker;
/// Policy-redacted server feedback projections; persistence and routes consume it later.
pub mod feedback;
/// Readiness reporting for the container health check.
pub mod health;
/// Server-only durable iMathAS broker bridge.  It is intentionally not wired
/// into the production backend registry until its same-origin launch profile
/// is configured.
pub mod imathas_backend;
/// Server composition bridge for first-party native question families.
pub mod native_backend;
/// Immutable published-QTI replay and server-side grading bridge.
pub mod qti_backend;
/// Private QTI archive staging worker and claim-bound committer.
pub mod qti_import;
/// Server-only QTI publication preparation; generic publication stays closed.
pub mod qti_publication;
/// Instructor-facing retention policy control and status APIs.
pub mod retention;
/// Private worker handler for staged retention notification and exact cleanup.
pub mod retention_worker;
/// Student runs, question attempts, submissions, and grading summaries.
pub mod run;
/// Authenticated, key-free fallbacks for browser-safe pure validation.
pub mod validation;
/// Persisted-source bridge for the isolated WeBWorK renderer.
pub mod webwork_backend;
/// Bounded server-side execution of durable, tenant-attributed worker jobs.
pub mod worker;
/// Author-only private unversioned workspace draft routes.
pub mod workspace;
