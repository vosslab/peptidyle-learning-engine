//! Current-Course Instructor Gradebook evidence route.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    LiveDemoGradebookStore, SessionTokenHash, StoreError,
    postgres::{PostgresLiveDemoGradebookStore, PostgresSessionStore},
};
use question_model::{CourseInstanceReference, ProductRole};

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    gradebook: PostgresLiveDemoGradebookStore,
}

pub fn live_gradebook_router(
    sessions: Arc<PostgresSessionStore>,
    gradebook: PostgresLiveDemoGradebookStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{reference}/gradebook",
            get(read_gradebook),
        )
        .with_state(RouteState {
            sessions,
            gradebook,
        })
}

async fn read_gradebook(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let course = match CourseInstanceReference::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.gradebook.live_demo_gradebook(token, course).await {
        // ASVS 4.1.3/8.2.1: the Store procedure repeats exact current Course
        // Instructor authorization; this response exposes only answer-free
        // immutable Grading Result aggregates.
        Ok(gradebook) => crate::auth::no_store(Json(gradebook).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn instructor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Gradebook authentication unavailable",
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
            route_error(StatusCode::PRECONDITION_FAILED, "Gradebook changed")
        }
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Gradebook is invalid")
        }
        StoreError::AlreadyExists => route_error(StatusCode::CONFLICT, "Gradebook conflict"),
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            route_error(StatusCode::SERVICE_UNAVAILABLE, "Gradebook unavailable")
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Gradebook not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
