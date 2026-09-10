//! Authorized public Course-reference navigation resolution.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    CourseInstanceStore, SessionTokenHash, StoreError,
    postgres::{PostgresCourseInstanceStore, PostgresSessionStore},
};
use question_model::{CourseInstanceReference, NavigationResolution};

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct NavigationRouteState {
    sessions: Arc<PostgresSessionStore>,
    courses: PostgresCourseInstanceStore,
}

/// Registers the exact public Course-reference resolver.
pub fn navigation_router(
    sessions: Arc<PostgresSessionStore>,
    courses: PostgresCourseInstanceStore,
) -> Router {
    Router::new()
        .route(
            "/api/navigation/{reference}",
            get(resolve_course_navigation),
        )
        .with_state(NavigationRouteState { sessions, courses })
}

async fn resolve_course_navigation(
    State(state): State<NavigationRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let reference = match CourseInstanceReference::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match authenticated_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .resolve_course_navigation(session_hash, reference)
        .await
    {
        Ok(course_id) => {
            crate::auth::no_store(Json(NavigationResolution::Course { course_id }).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn authenticated_session_hash(
    state: &NavigationRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // ASVS 8.2.2 and 8.3.1: Course Membership is the trusted resolver
        // authority, so every authenticated Product Role reaches the Store.
        Ok(session) => Ok(session.session_hash),
        Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course navigation authentication unavailable",
        ))),
    }
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Course navigation changed")
        }
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Course navigation is invalid",
        ),
        StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Course navigation conflict")
        }
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Course navigation unavailable",
            )
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Course navigation not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
