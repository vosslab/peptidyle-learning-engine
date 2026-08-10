//! Authenticated course and assignment routes (MOD-API-COURSE).
//!
//! The facade preserves the route group's stable module path. Focused owners
//! keep routing, queries, mutations, access policy, and wire projections small.

mod assignments;
mod policy;
mod projection;
mod queries;
mod routing;

pub use routing::router;

#[cfg(test)]
use assignments::{AssignmentRevisionHeaderError, required_assignment_revision};
#[cfg(test)]
use axum::http::header::{ETAG, IF_MATCH};
#[cfg(test)]
use axum::http::{HeaderMap, HeaderValue, StatusCode};
#[cfg(test)]
use axum::response::Response;
#[cfg(test)]
use learning_data_access::{CourseRecord, Store};
#[cfg(test)]
use question_model::{
    AssignmentId, CourseId, CourseMembership, CourseMembershipRole, ProblemVersionRef, RunPolicies,
    UserRole,
};
#[cfg(test)]
use routing::CreateAssignmentRequest;
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
#[path = "course/tests/mod.rs"]
mod tests;
