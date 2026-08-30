//! Authenticated, revision-bound curriculum-adoption transport.
//!
//! The route facade establishes the security boundary once: it authenticates
//! and preflights an Instructor before decoding protected locators or JSON.
//! Feature modules then bind the decoded values to their public route.

use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use learning_data_access::{CurriculumAdoptionStore, SessionStore, StoreError};
use question_model::{
    AssignmentReference, CourseReference, CourseTermShiftPreviewOutcome,
    CurriculumAdoptionCommandError, CurriculumAdoptionIdempotencyKey,
};
use serde::Deserialize;

use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};

mod imports;
mod lifecycle;
mod source_adoption;

const MAX_CURRICULUM_ADOPTION_BODY_BYTES: usize = 64 * 1024;

/// Builds the B2 source-to-teaching-course adoption routes.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    Router::new()
        .route(
            "/api/alpha-courses/{alpha}/fork/preview",
            post(source_adoption::preview_fork_alpha::<S>),
        )
        .route(
            "/api/alpha-courses/{alpha}/fork/apply",
            post(source_adoption::apply_fork_alpha::<S>),
        )
        .route(
            "/api/course-blueprints/{blueprint}/instantiate/preview",
            post(source_adoption::preview_blueprint_instantiation::<S>),
        )
        .route(
            "/api/course-blueprints/{blueprint}/instantiate/apply",
            post(source_adoption::apply_blueprint_instantiation::<S>),
        )
        .route(
            "/api/alpha-courses/{alpha}/instantiate/preview",
            post(source_adoption::preview_alpha_instantiation::<S>),
        )
        .route(
            "/api/alpha-courses/{alpha}/instantiate/apply",
            post(source_adoption::apply_alpha_instantiation::<S>),
        )
        .route(
            "/api/courses/{course}/curriculum-rollover/preview",
            post(lifecycle::preview_course_rollover::<S>),
        )
        .route(
            "/api/courses/{course}/curriculum-rollover/apply",
            post(lifecycle::apply_course_rollover::<S>),
        )
        .route(
            "/api/courses/{course}/curriculum-term-shift/preview",
            post(lifecycle::preview_course_term_shift::<S>),
        )
        .route(
            "/api/courses/{course}/curriculum-term-shift/apply",
            post(lifecycle::apply_course_term_shift::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/curriculum-fast-forward/preview",
            post(imports::preview_assignment_fast_forward::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/curriculum-fast-forward/apply",
            post(imports::apply_assignment_fast_forward::<S>),
        )
        .route(
            "/api/courses/{course}/curriculum-source-derived-assignment/preview",
            post(imports::preview_source_derived_assignment::<S>),
        )
        .route(
            "/api/courses/{course}/curriculum-source-derived-assignment/apply",
            post(imports::create_source_derived_assignment::<S>),
        )
        .route(
            "/api/courses/{course}/curriculum-imports",
            get(imports::inspect_curriculum_imports::<S>),
        )
        .route(
            "/api/curriculum-adoption/reconcile",
            post(imports::reconcile_curriculum_adoption::<S>),
        )
        .with_state(CurriculumAdoptionRouteState { store })
}

pub(super) struct CurriculumAdoptionRouteState<S> {
    pub(super) store: Arc<S>,
}

impl<S> Clone for CurriculumAdoptionRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

/// Apply carries the exact answer-free preview and a bounded retry key.
/// qmodel rebuilds the command and the Store rechecks its witnesses atomically
/// (ASVS 1.5.2, 2.3.1, and 2.3.3).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ApplyBody<P> {
    pub(super) preview: P,
    pub(super) idempotency_key: CurriculumAdoptionIdempotencyKey,
}

#[allow(clippy::result_large_err)]
pub(super) async fn authenticate_and_preflight<S>(
    state: &CurriculumAdoptionRouteState<S>,
    headers: &HeaderMap,
) -> Result<AuthenticatedSession, Response>
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = resolve_request_session(state.store.as_ref(), headers)
        .await
        .map_err(auth_error_response)?;
    state
        .store
        .preflight_curriculum_adoption(
            authenticated.tenant_context,
            authenticated.record.token_hash,
        )
        .await
        .map_err(preflight_error)?;
    Ok(authenticated)
}

#[allow(clippy::result_large_err)]
pub(super) async fn strict_json_body<T>(request: Request) -> Result<T, Response>
where
    T: serde::de::DeserializeOwned,
{
    let bytes = to_bytes(request.into_body(), MAX_CURRICULUM_ADOPTION_BODY_BYTES)
        .await
        .map_err(|_| {
            error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "curriculum adoption request is too large",
            )
        })?;
    serde_json::from_slice(&bytes).map_err(|_| {
        error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "curriculum adoption request must use the documented JSON shape",
        )
    })
}

#[allow(clippy::result_large_err)]
pub(super) fn parse_reference<T>(raw: &str) -> Result<T, Response>
where
    T: std::str::FromStr,
{
    raw.parse().map_err(|_| binding_refused())
}

#[allow(clippy::result_large_err)]
pub(super) fn parse_course_assignment(
    raw_course: &str,
    raw_assignment: &str,
) -> Result<(CourseReference, AssignmentReference), Response> {
    Ok((
        parse_reference(raw_course)?,
        parse_reference(raw_assignment)?,
    ))
}

pub(super) fn outcome_course(outcome: &CourseTermShiftPreviewOutcome) -> CourseReference {
    match outcome {
        CourseTermShiftPreviewOutcome::Eligible { preview } => preview.witness.course,
        CourseTermShiftPreviewOutcome::Ineligible { course, .. } => *course,
    }
}

pub(super) fn response_from_store<T>(result: Result<T, StoreError>) -> Response
where
    T: serde::Serialize,
{
    match result {
        Ok(value) => no_store(Json(value).into_response()),
        Err(error) => store_error(error),
    }
}

pub(super) fn command_refused(error: CurriculumAdoptionCommandError) -> Response {
    match error {
        CurriculumAdoptionCommandError::CorrectionsRequired => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "curriculum adoption preview requires correction",
        ),
        CurriculumAdoptionCommandError::FastForwardNotEligible => error_response(
            StatusCode::CONFLICT,
            "curriculum import cannot fast-forward; create a source-derived assignment",
        ),
        CurriculumAdoptionCommandError::TermShiftNotEligible => error_response(
            StatusCode::CONFLICT,
            "course term cannot shift after learner work; create a rollover",
        ),
    }
}

fn preflight_error(error: StoreError) -> Response {
    match error {
        StoreError::RetryableTransaction | StoreError::TimedOut | StoreError::Unavailable(_) => {
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "curriculum adoption is unavailable",
            )
        }
        StoreError::NotFound
        | StoreError::OwnershipMismatch
        | StoreError::Forbidden
        | StoreError::AlreadyExists
        | StoreError::Conflict
        | StoreError::InvalidRecord(_)
        | StoreError::RunModel(_) => error_response(
            StatusCode::FORBIDDEN,
            "curriculum adoption is not authorized",
        ),
    }
}

pub(super) fn store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::OwnershipMismatch | StoreError::Forbidden => {
            error_response(
                StatusCode::NOT_FOUND,
                "curriculum adoption target not found",
            )
        }
        StoreError::Conflict => error_response(
            StatusCode::PRECONDITION_FAILED,
            "curriculum adoption state changed; reload the preview",
        ),
        StoreError::AlreadyExists => error_response(
            StatusCode::CONFLICT,
            "curriculum adoption destination already exists",
        ),
        StoreError::InvalidRecord(_) | StoreError::RunModel(_) => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "curriculum adoption request is invalid",
        ),
        StoreError::TimedOut => error_response(
            StatusCode::CONFLICT,
            "curriculum adoption operation timed out",
        ),
        error @ (StoreError::RetryableTransaction | StoreError::Unavailable(_)) => {
            tracing::warn!(event = "curriculum_adoption_store_unavailable", error = ?error);
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "curriculum adoption is unavailable",
            )
        }
    }
}

pub(super) fn binding_refused() -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        "curriculum adoption target not found",
    )
}

pub(super) fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests;
