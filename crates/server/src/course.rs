//! Authenticated course and assignment routes (MOD-API-COURSE).
//!
//! The facade preserves the route group's stable module path. Focused owners
//! keep routing, queries, mutations, access policy, and wire projections small.

mod assignments;
mod gradebook;
mod grading_operations;
mod invitation_capability;
pub(crate) mod invitation_delivery_worker;
mod invitation_email;
mod policy;
mod projection;
mod queries;
mod roster;
mod routing;
mod teaching_operations;

pub use invitation_capability::{
    CourseInvitationDelivery, CourseInvitationDeliveryAttempt, CourseInvitationDeliveryError,
    CourseInvitationIssuer, CourseInvitationSecret, UnavailableCourseInvitationDelivery,
};
pub use invitation_email::{
    SmtpCourseInvitationDelivery, SmtpCourseInvitationDeliveryConfig, SmtpTlsMode,
};
pub use routing::{router, router_with_invitations};

#[cfg(test)]
use assignments::{AssignmentRevisionHeaderError, required_assignment_revision};
#[cfg(test)]
use axum::http::header::IF_MATCH;
#[cfg(test)]
use axum::http::{HeaderMap, HeaderValue, StatusCode};
#[cfg(test)]
use learning_data_access::Store;
#[cfg(test)]
use question_model::{ProblemVersionRef, RunPolicies, UserRole};
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
#[path = "course/tests/mod.rs"]
pub(crate) mod tests;
