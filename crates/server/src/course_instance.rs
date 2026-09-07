//! Live Course Instance creation and initial teaching-team Server Routes.
//!
//! M8 exposes the smallest live-teaching boundary: an exact Blueprint Revision
//! becomes a Course Instance with one Assigned Instructor.  Roster, Assignment,
//! and Student delivery routes remain outside this module.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    CourseInstanceStore, CreateCourseInstanceInput, SessionTokenHash, StoreError,
    postgres::{PostgresCourseInstanceStore, PostgresSessionStore},
};
use question_model::{CourseInstanceReference, ProductRole};
use serde::Serialize;

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct CourseInstanceRouteState {
    sessions: Arc<PostgresSessionStore>,
    courses: PostgresCourseInstanceStore,
}

/// Registers the active Course Instance creation and teaching-team routes.
pub fn course_instance_router(
    sessions: Arc<PostgresSessionStore>,
    courses: PostgresCourseInstanceStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances",
            get(list_course_instances).post(create_course_instance),
        )
        .route(
            "/api/course-instances/{reference}",
            get(load_course_instance),
        )
        .route(
            "/api/course-instance-creation/instructors",
            get(list_course_creation_instructors),
        )
        .with_state(CourseInstanceRouteState { sessions, courses })
}

#[derive(Serialize)]
struct CourseInstanceListResponse<T> {
    items: Vec<T>,
    #[serde(rename = "nextCursor")]
    next_cursor: Option<String>,
}

async fn list_course_instances(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
) -> Response {
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.courses.list_course_instances(session_hash).await {
        Ok(items) => crate::auth::no_store(
            Json(CourseInstanceListResponse {
                items,
                next_cursor: None,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn create_course_instance(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Json(input): Json<CreateCourseInstanceInput>,
) -> Response {
    let session_hash = match course_creator_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .create_course_instance(session_hash, input)
        .await
    {
        Ok(created) => crate::auth::no_store((StatusCode::CREATED, Json(created)).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn load_course_instance(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let reference = match CourseInstanceReference::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .load_course_instance(session_hash, reference)
        .await
    {
        Ok(view) => crate::auth::no_store(Json(view).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn list_course_creation_instructors(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
) -> Response {
    let session_hash = match sysadmin_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .list_course_creation_instructors(session_hash)
        .await
    {
        Ok(items) => crate::auth::no_store(
            Json(CourseInstanceListResponse {
                items,
                next_cursor: None,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn instructor_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    required_session_hash(state, headers, |role| role == ProductRole::Instructor).await
}

async fn course_creator_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    required_session_hash(state, headers, |role| {
        matches!(role, ProductRole::Instructor | ProductRole::Sysadmin)
    })
    .await
}

async fn sysadmin_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    required_session_hash(state, headers, |role| role == ProductRole::Sysadmin).await
}

async fn required_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
    permitted: impl FnOnce(ProductRole) -> bool,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if permitted(session.record.product_role) => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Instance authentication unavailable",
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
            route_error(StatusCode::PRECONDITION_FAILED, "Course Instance changed")
        }
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Course Instance is invalid",
        ),
        StoreError::AlreadyExists => route_error(StatusCode::CONFLICT, "Course Instance conflict"),
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Course Instance unavailable",
            )
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Course Instance not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
