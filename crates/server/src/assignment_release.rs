//! Direct-Instructor Assignment Workspace routes.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{
        HeaderMap, StatusCode,
        header::{COOKIE, ETAG, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use learning_data_access::{
    CreateLiveAssignmentInput, LiveAssignmentStore, SaveLiveAssignmentInlineInput,
    SaveLiveAssignmentInput, SessionTokenHash, StoreError,
    postgres::{PostgresLiveAssignmentStore, PostgresSessionStore},
};
use question_model::{
    AssignmentEditNumber, AssignmentReference, CourseInstanceReference, ProductRole,
};

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct StateData {
    sessions: Arc<PostgresSessionStore>,
    assignments: PostgresLiveAssignmentStore,
}

/// Registers answer-free current Assignment authoring and immutable release routes.
pub fn assignment_release_router(
    sessions: Arc<PostgresSessionStore>,
    assignments: PostgresLiveAssignmentStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course}/assignment-question-picker",
            get(list_picker),
        )
        .route(
            "/api/course-instances/{course}/assignments",
            get(list_assignments).post(create_assignment),
        )
        .route("/api/assignments/due-soon", get(list_assignments_due_soon))
        .route(
            "/api/course-instances/{course}/assignments/{assignment}",
            get(load_assignment).put(save_assignment),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/inline",
            put(save_assignment_inline),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/release-validation",
            get(validate_release),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/preview",
            get(assignment_preview),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/release",
            post(release_assignment),
        )
        .with_state(StateData {
            sessions,
            assignments,
        })
}

async fn list_assignments_due_soon(State(state): State<StateData>, headers: HeaderMap) -> Response {
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state.assignments.list_assignments_due_soon(token).await {
        // ASVS 4.1.1 and 8.3.1: Axum serializes this closed JSON projection;
        // it includes no Student, Attempt, response, answer, or grading record.
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}

async fn list_assignments(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match course_reference(&course) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .list_course_assignments(token, course)
        .await
    {
        // ASVS 8.3.1 and 8.3.4: this closed projection carries no Student,
        // response, answer, grading, or other protected educational record.
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}

async fn list_picker(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match course_reference(&course) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .list_assignment_question_picker(token, course)
        .await
    {
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}
async fn create_assignment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(course): Path<String>,
    Json(input): Json<CreateLiveAssignmentInput>,
) -> Response {
    let course = match course_reference(&course) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .create_live_assignment(token, course, input)
        .await
    {
        Ok(v) => workspace_response(StatusCode::CREATED, &v),
        Err(e) => store_error(e),
    }
}
async fn load_assignment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .load_live_assignment(token, course, assignment)
        .await
    {
        Ok(v) => workspace_response(StatusCode::OK, &v),
        Err(e) => store_error(e),
    }
}
async fn save_assignment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
    Json(mut input): Json<SaveLiveAssignmentInput>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let expected = match edit_header(&headers) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    input.expected_edit_number = expected;
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .save_live_assignment(token, course, assignment, input)
        .await
    {
        Ok(v) => workspace_response(StatusCode::OK, &v),
        Err(e) => store_error(e),
    }
}
async fn save_assignment_inline(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
    Json(input): Json<SaveLiveAssignmentInlineInput>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected = match edit_header(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .assignments
        .save_live_assignment_inline(token, course, assignment, expected, input)
        .await
    {
        Ok(value) => summary_response(&value),
        Err(error_value) => store_error(error_value),
    }
}
async fn validate_release(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .validate_live_assignment_release(token, course, assignment)
        .await
    {
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}
async fn assignment_preview(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .load_live_assignment_preview(token, course, assignment)
        .await
    {
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}
async fn release_assignment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let expected = match edit_header(&headers) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .release_live_assignment(token, course, assignment, expected)
        .await
    {
        Ok(v) => crate::auth::no_store((StatusCode::CREATED, Json(v)).into_response()),
        Err(e) => store_error(e),
    }
}

fn workspace_response(
    status: StatusCode,
    value: &learning_data_access::LiveAssignmentWorkspace,
) -> Response {
    let mut response = crate::auth::no_store((status, Json(value)).into_response());
    if let Ok(header) = format!("\"{}\"", value.edit_number.value()).parse() {
        response.headers_mut().insert(ETAG, header);
    }
    response
}
fn summary_response(value: &learning_data_access::CourseAssignmentSummary) -> Response {
    let mut response = crate::auth::no_store(Json(value).into_response());
    if let Ok(header) = format!("\"{}\"", value.edit_number.value()).parse() {
        response.headers_mut().insert(ETAG, header);
    }
    response
}
fn course_reference(value: &str) -> Result<CourseInstanceReference, Box<Response>> {
    CourseInstanceReference::from_str(value).map_err(|_| Box::new(concealed()))
}
fn refs(
    course_value: &str,
    assignment_value: &str,
) -> Result<(CourseInstanceReference, AssignmentReference), Box<Response>> {
    Ok((
        course_reference(course_value)?,
        AssignmentReference::from_str(assignment_value).map_err(|_| Box::new(concealed()))?,
    ))
}
fn edit_header(headers: &HeaderMap) -> Result<AssignmentEditNumber, Box<Response>> {
    let value = headers
        .get(IF_MATCH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix('"').and_then(|x| x.strip_suffix('"')))
        .ok_or_else(|| Box::new(concealed()))?;
    AssignmentEditNumber::from_str(value).map_err(|_| Box::new(concealed()))
}
async fn instructor(
    state: &StateData,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(state.sessions.as_ref(), cookie(headers).as_deref()).await {
        Ok(v) if v.record.product_role == ProductRole::Instructor => Ok(v.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Assignment Workspace authentication unavailable",
        ))),
    }
}
fn cookie(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|v| v.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}
fn store_error(error_value: StoreError) -> Response {
    match error_value {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => error(
            StatusCode::PRECONDITION_FAILED,
            "Assignment Workspace changed",
        ),
        StoreError::InvalidRecord(_) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Assignment Workspace is invalid",
        ),
        StoreError::AlreadyExists => error(StatusCode::CONFLICT, "Assignment Workspace conflict"),
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Assignment Workspace unavailable",
            )
        }
    }
}
fn concealed() -> Response {
    error(StatusCode::NOT_FOUND, "Assignment Workspace not found")
}
fn error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
