//! Direct-Instructor issuance and revocation of one closed support capability.

use crate::auth::{AuthError, resolve_session};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    IssueSupportCapabilityInput, SessionTokenHash, StoreError, SupportCapabilityStore,
    postgres::{PostgresSessionStore, PostgresSupportCapabilityStore},
};
use question_model::{CourseInstanceReference, ProductRole};
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    support: PostgresSupportCapabilityStore,
}

pub fn support_capability_router(
    sessions: Arc<PostgresSessionStore>,
    support: PostgresSupportCapabilityStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{reference}/support-capabilities",
            post(issue),
        )
        .route(
            "/api/course-instances/{reference}/support-capabilities/{capability_id}/revoke",
            post(revoke),
        )
        .route(
            "/api/support-capabilities/{capability_id}/course-roster",
            get(read_roster),
        )
        .with_state(RouteState { sessions, support })
}

async fn read_roster(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(capability_id): Path<String>,
) -> Response {
    let capability_id = match Uuid::parse_str(&capability_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .support
        .read_course_roster_support(token, capability_id)
        .await
    {
        Ok(entries) if entries.is_empty() => concealed(),
        Ok(entries) => crate::auth::no_store(Json(entries).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn issue(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(input): Json<IssueSupportCapabilityInput>,
) -> Response {
    let course = match CourseInstanceReference::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .support
        .issue_course_roster_support(token, course, input)
        .await
    {
        Ok(receipt) => crate::auth::no_store((StatusCode::CREATED, Json(receipt)).into_response()),
        Err(error) => store_error_response(error),
    }
}
async fn sysadmin_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.product_role == ProductRole::Sysadmin => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Support capability authentication unavailable",
        ))),
    }
}

async fn revoke(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path((reference, capability_id)): Path<(String, String)>,
) -> Response {
    let course = match CourseInstanceReference::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let capability_id = match Uuid::parse_str(&capability_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .support
        .revoke_course_roster_support(token, course, capability_id)
        .await
    {
        Ok(receipt) => crate::auth::no_store(Json(receipt).into_response()),
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
        // ASVS 8.2.1: the procedure repeats current direct membership atomically.
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Support capability authentication unavailable",
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
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Support capability changed",
        ),
        StoreError::LifecycleConflict => route_error(
            StatusCode::CONFLICT,
            "Support capability lifecycle conflict",
        ),
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Support capability is invalid",
        ),
        StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Support capability conflict")
        }
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Support capability unavailable",
            )
        }
    }
}
fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Support capability not found")
}
fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
