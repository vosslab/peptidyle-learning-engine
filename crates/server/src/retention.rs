//! Instructor-facing retention control and status routes (MOD-RETENTION).
//!
//! The public facade keeps the established route registration path stable.
//! Focused children own authorization, strict request parsing, browser-safe
//! projections, and route mutations.

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::{
    Router,
    routing::{get, patch, post},
};
use learning_data_access::{RetentionApiStore, RetentionStore, SessionStore, Store};

pub(super) const MAX_RETENTION_BODY_BYTES: usize = 64 * 1_024;

pub(super) struct RetentionRouteState<S> {
    pub(super) store: Arc<S>,
}

impl<S> Clone for RetentionRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

/// Builds the course-level retention route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    Router::new()
        .route(
            "/api/courses/{course}/retention",
            get(routes::get_retention::<S>),
        )
        .route(
            "/api/courses/{course}/retention/end",
            post(routes::end_course_retention::<S>),
        )
        .route(
            "/api/courses/{course}/retention/archive",
            post(routes::request_archive::<S>),
        )
        .route(
            "/api/courses/{course}/retention/delete",
            post(routes::request_delete::<S>),
        )
        .route(
            "/api/courses/{course}/retention/extend",
            patch(routes::request_extend::<S>),
        )
        .layer(DefaultBodyLimit::max(MAX_RETENTION_BODY_BYTES))
        .layer(middleware::map_response(projection::no_store_response))
        .with_state(RetentionRouteState { store })
}

mod access;
mod parsing;
mod projection;
mod routes;

#[cfg(test)]
#[path = "retention/route_tests/mod.rs"]
mod route_tests;
