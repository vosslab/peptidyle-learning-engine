//! Authenticated direct-Instructor time-only accommodation reads and saves.
use crate::auth::{AuthError, resolve_session};
use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    AssessmentStudentTimeAccommodationStore, StoreError,
    postgres::{PostgresAssessmentStudentTimeAccommodationStore, PostgresSessionStore},
};
use question_model::{
    AssessmentId, CourseInstanceId, ProductRole, SaveAssessmentStudentTimeAccommodationInput,
};
use std::sync::Arc;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    store: PostgresAssessmentStudentTimeAccommodationStore,
}

pub fn assessment_student_time_accommodation_router(
    sessions: Arc<PostgresSessionStore>,
    store: PostgresAssessmentStudentTimeAccommodationStore,
) -> Router {
    Router::new().route(
        "/api/course-instances/{course_instance_id}/assessments/{assessment_id}/student-time-accommodations/{roster_id}",
        get(read).put(save),
    ).with_state(RouteState { sessions, store })
}

async fn read(
    State(state): State<RouteState>,
    Path(route): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    configuration(&state, route, &headers, None).await
}

async fn save(
    State(state): State<RouteState>,
    Path(route): Path<(String, String, String)>,
    request: Request,
) -> Response {
    // ASVS 2.2.1, 3.5.2: bounded closed JSON behind composition's shared
    // authenticated same-origin browser boundary.
    let headers = request.headers().clone();
    if !headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json"))
        })
    {
        return error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Student time configuration requires JSON",
        );
    }
    let bytes = match to_bytes(request.into_body(), 1024).await {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Student time configuration is too large",
            );
        }
    };
    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Student time configuration is invalid",
            );
        }
    };
    if !value.as_object().is_some_and(|fields| {
        fields.len() == 2
            && fields.contains_key("timeMultiplier")
            && fields.contains_key("expectedAccommodationEditNumber")
    }) {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Student time configuration is invalid",
        );
    }
    let input: SaveAssessmentStudentTimeAccommodationInput = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Student time configuration is invalid",
            );
        }
    };
    configuration(&state, route, &headers, Some(input)).await
}

async fn configuration(
    state: &RouteState,
    (course, assessment, roster_id): (String, String, String),
    headers: &HeaderMap,
    save: Option<SaveAssessmentStudentTimeAccommodationInput>,
) -> Response {
    let (course, assessment): (CourseInstanceId, AssessmentId) =
        match (course.parse(), assessment.parse()) {
            (Ok(course), Ok(assessment)) => (course, assessment),
            _ => return concealed(),
        };
    if roster_id.is_empty()
        || roster_id.len() > 64
        || !roster_id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || b"._-".contains(&value))
    {
        return concealed();
    }
    let cookie = headers
        .get_all("cookie")
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()
        .filter(|values| !values.is_empty())
        .map(|values| values.join("; "));
    let token = match resolve_session(state.sessions.as_ref(), cookie.as_deref()).await {
        Ok(value) if value.record.product_role == ProductRole::Instructor => value.session_hash,
        Ok(_) | Err(AuthError::Unauthenticated) => return concealed(),
        Err(_) => return unavailable(),
    };
    match state
        .store
        .student_time_configuration(token, course, assessment, roster_id, save)
        .await
    {
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            concealed()
        }
        Err(StoreError::Conflict | StoreError::RetryableTransaction) => error(
            StatusCode::PRECONDITION_FAILED,
            "Student time configuration changed; reload before saving",
        ),
        Err(StoreError::InvalidRecord(_)) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Student time configuration is invalid",
        ),
        Err(_) => unavailable(),
    }
}

fn concealed() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "Student time configuration is unavailable",
    )
}
fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Student time configuration is unavailable",
    )
}
fn error(status: StatusCode, message: &'static str) -> Response {
    // ASVS 14.2.2, 16.5.1: no cache or private-record/backend error disclosure.
    crate::auth::no_store((status, message).into_response())
}
