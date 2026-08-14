//! API server core.
//!
//! Route groups own browser-facing behavior while the small composition root
//! supplies their production dependencies and security boundaries.

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
/// Production dependency construction shared by the API and future worker entry points.
pub mod composition;
/// Tenant courses, course-local membership, and assignment routes.
pub mod course;
pub mod course_appearance;
/// Instructor-authorized asynchronous assignment export requests.
pub mod export;
/// Frozen assignment export preparation and atomic four-artifact finalization.
pub mod export_worker;
/// Policy-redacted server feedback projections; persistence and routes consume it later.
pub mod feedback;
/// Author-only registration of immutable native flat-question image assets.
pub mod flat_question_assets;
/// Dedicated authoring and immutable publication routes for PLE flat questions.
pub mod flat_question_publication;
/// Readiness reporting for the container health check.
pub mod health;
/// Server-owned verification of original instructional images used by native hotspots.
pub(crate) mod hotspot_image;
/// Uniform non-cacheable API response and browser hardening headers.
pub(crate) mod http_security;
/// Server-only durable iMathAS broker bridge.  It is intentionally not wired
/// into the production backend registry until its same-origin launch profile
/// is configured.
pub mod imathas_backend;
/// Instructor-only course-local aggregate item-analysis report.
pub mod item_analysis;
/// Private staged and generation-fenced course item-analysis worker.
pub mod item_analysis_worker;
/// Server composition bridge for first-party native question families.
pub mod native_backend;
pub mod navigation;
/// Post-commit materialization of immutable CDN-readable catalog assets.
pub(crate) mod public_asset_publication_worker;
/// Immutable published-QTI replay and server-side grading bridge.
pub mod qti_backend;
/// Private QTI archive staging worker and claim-bound committer.
pub mod qti_import;
/// Author-only conversion of recognized QTI profile items into native flat drafts.
pub mod qti_profile_conversion;
/// Pure server-only bridge from validated QTI profiles to native flat questions.
pub(crate) mod qti_profile_flat_bridge;
/// Author-only QTI archive upload and answer-free profile reports.
pub mod qti_profile_import;
#[cfg(test)]
mod qti_profile_postgres_live;
/// Server-only QTI publication preparation; generic publication stays closed.
pub mod qti_publication;
/// Process-owned request correlation and bounded shutdown behavior.  This is
/// deliberately outside individual route groups so future routes cannot opt
/// out by convention.
pub mod request_lifecycle;
/// Instructor-facing retention policy control and status APIs.
pub mod retention;
/// Private worker handler for staged retention notification and exact cleanup.
pub mod retention_worker;
/// Typed security contract for every public route's HTTP method.
pub(crate) mod route_policy;
/// Student runs, question attempts, submissions, and grading summaries.
pub mod run;
/// Current-score worker with private staging and generation-fenced publication.
pub mod scoring_worker;
/// Server-owned, generation-fenced deadline finalization.
pub mod timing_worker;
/// Authenticated, key-free fallbacks for browser-safe pure validation.
pub mod validation;
/// Persisted-source bridge for the isolated WeBWorK renderer.
pub mod webwork_backend;
/// Bounded server-side execution of durable, tenant-attributed worker jobs.
pub mod worker;
/// Author-only private unversioned workspace draft routes.
pub mod workspace;
