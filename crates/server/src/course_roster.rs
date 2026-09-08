//! Course Roster Import, invitation claim, and access-revocation routes.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    CourseRosterImportInput, CourseRosterStore, SessionTokenHash, StoreError,
    postgres::{PostgresCourseRosterStore, PostgresSessionStore},
};
use question_model::{CourseInstanceReference, ProductRole};

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct CourseRosterRouteState {
    sessions: Arc<PostgresSessionStore>,
    roster: PostgresCourseRosterStore,
}

/// Registers current direct-Instructor roster operations and Student invitation claim.
pub fn course_roster_router(
    sessions: Arc<PostgresSessionStore>,
    roster: PostgresCourseRosterStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{reference}/roster",
            get(list_course_roster).post(import_course_roster),
        )
        .route(
            "/api/course-instances/{reference}/roster/claim",
            post(claim_course_invitation),
        )
        .route(
            "/api/course-instances/{reference}/roster/{roster_id}/revoke",
            post(revoke_course_roster_entry),
        )
        .with_state(CourseRosterRouteState { sessions, roster })
}

async fn list_course_roster(
    State(state): State<CourseRosterRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let course = match course_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash =
        match required_session_hash(&state, &headers, |role| role == ProductRole::Instructor).await
        {
            Ok(value) => value,
            Err(response) => return *response,
        };
    match state.roster.list_course_roster(session_hash, course).await {
        Ok(entries) => crate::auth::no_store(Json(entries).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn import_course_roster(
    State(state): State<CourseRosterRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(input): Json<CourseRosterImportInput>,
) -> Response {
    let course = match course_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash =
        match required_session_hash(&state, &headers, |role| role == ProductRole::Instructor).await
        {
            Ok(value) => value,
            Err(response) => return *response,
        };
    match state
        .roster
        .import_course_roster(session_hash, course, input)
        .await
    {
        Ok(entries) => crate::auth::no_store((StatusCode::CREATED, Json(entries)).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn claim_course_invitation(
    State(state): State<CourseRosterRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let course = match course_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash =
        match required_session_hash(&state, &headers, |role| role == ProductRole::Student).await {
            Ok(value) => value,
            Err(response) => return *response,
        };
    match state
        .roster
        .claim_course_invitation(session_hash, course)
        .await
    {
        Ok(result) => crate::auth::no_store(Json(result).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn revoke_course_roster_entry(
    State(state): State<CourseRosterRouteState>,
    headers: HeaderMap,
    Path((reference, roster_id)): Path<(String, String)>,
) -> Response {
    let course = match course_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash =
        match required_session_hash(&state, &headers, |role| role == ProductRole::Instructor).await
        {
            Ok(value) => value,
            Err(response) => return *response,
        };
    match state
        .roster
        .revoke_course_roster_entry(session_hash, course, roster_id)
        .await
    {
        Ok(()) => crate::auth::no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => store_error_response(error),
    }
}

fn course_reference(value: &str) -> Result<CourseInstanceReference, Box<Response>> {
    CourseInstanceReference::from_str(value).map_err(|_| Box::new(concealed()))
}

async fn required_session_hash(
    state: &CourseRosterRouteState,
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
            "Course Roster authentication unavailable",
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
            route_error(StatusCode::PRECONDITION_FAILED, "Course Roster changed")
        }
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Course Roster is invalid")
        }
        StoreError::AlreadyExists => route_error(StatusCode::CONFLICT, "Course Roster conflict"),
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            route_error(StatusCode::SERVICE_UNAVAILABLE, "Course Roster unavailable")
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Course Roster not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
