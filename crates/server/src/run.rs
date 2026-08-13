//! Authenticated run, attempt, submission, and grading-summary routes (MOD-API-RUN).
//!
//! The store owns timestamps, run numbers, one-active-question enforcement,
//! idempotency, and transactional summary changes. A pluggable server-only
//! backend owns rendering provenance and correctness so this route group does
//! not choose the first native family or expose an answer key.

#[cfg(test)]
use async_trait::async_trait;
#[cfg(test)]
use axum::Router;
#[cfg(test)]
use axum::http::HeaderValue;
#[cfg(test)]
use grading::GradeOutcome;
#[cfg(test)]
use question_model::{AssignmentId, AttemptProvenance};
mod contracts;
pub use contracts::{
    GradeReceipt, IssuedAttemptMetadata, RunBackend, RunBackendError, RunSubmission,
    SubmissionDisposition,
};
mod prefetch;
#[cfg(test)]
use prefetch::*;
mod queries;
mod routes;
pub use routes::router;
mod submission;
mod support;
#[cfg(test)]
use support::{MAX_JSON_SAFE_INTEGER, *};
mod external_tool;
mod manual_grading;
pub(crate) use external_tool::EXTERNAL_LAUNCH_COOKIE;
pub use external_tool::{
    ExternalToolLaunch, ExternalToolLaunchBackend, router as external_tool_router,
};

#[cfg(test)]
#[path = "run/tests/mod.rs"]
mod tests;
