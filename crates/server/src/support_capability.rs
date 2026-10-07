//! Confirmed, audited Sysadmin access to one named Student roster record.

use crate::auth::{AuthError, resolve_session};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    ConfirmedStudentDataAccess, SessionTokenHash, StoreError, SysadminStudentDataStore,
    postgres::{PostgresSessionStore, PostgresSysadminStudentDataStore},
};
use question_model::{CourseInstanceId, UserRole};
use std::{str::FromStr, sync::Arc};

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    student_data: PostgresSysadminStudentDataStore,
}

pub fn support_capability_router(
    sessions: Arc<PostgresSessionStore>,
    student_data: PostgresSysadminStudentDataStore,
) -> Router {
    Router::new()
        .route(
            "/api/sysadmin/course-instances/{course_instance_id}/roster/{roster_id}/student-data",
            post(read_student_data),
        )
        .with_state(RouteState {
            sessions,
            student_data,
        })
}

/// ASVS 2.2.2/2.3.3/8.3.1/16.2.1/16.2.5: validate at the trusted server,
/// then let the Store commit the protected read and minimal audit event together.
async fn read_student_data(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path((course_instance_id, roster_id)): Path<(String, String)>,
    Json(confirmation): Json<ConfirmedStudentDataAccess>,
) -> Response {
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if let Err(error) = confirmation.validate() {
        return store_error_response(error);
    }
    let course_instance_id = match CourseInstanceId::from_str(&course_instance_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    match state
        .student_data
        .read_sysadmin_student_roster_record(token, course_instance_id, roster_id, confirmation)
        .await
    {
        Ok(Some(record)) => crate::auth::no_store(Json(record).into_response()),
        Ok(None) => concealed(),
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
        Ok(session) if session.record.user_role == UserRole::Sysadmin => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student-data access authentication unavailable",
        ))),
    }
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values: Vec<_> = headers.get_all(COOKIE).iter().cloned().collect();
    if values.is_empty() {
        return None;
    }
    let mut joined = Vec::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            joined.extend_from_slice(b"; ");
        }
        joined.extend_from_slice(value.as_bytes());
    }
    String::from_utf8(joined).ok()
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Student record not found")
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Student-data access request is invalid",
        ),
        StoreError::Forbidden | StoreError::NotFound => concealed(),
        _ => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student-data access is temporarily unavailable",
        ),
    }
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, Json(serde_json::json!({"error": message}))).into_response())
}
