//! Course Roster Import, invitation claim, and access-revocation routes.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    CourseRosterImportInput, CourseRosterStore, InvitationMailerExport, InvitationMailerRecipient,
    SessionTokenHash, StoreError,
    postgres::{PostgresCourseRosterStore, PostgresSessionStore},
};
use question_model::{CourseInstanceId, UserRole};

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct CourseRosterRouteState {
    sessions: Arc<PostgresSessionStore>,
    roster: PostgresCourseRosterStore,
    signup_url: Arc<str>,
}

/// Registers current direct-Instructor roster operations and Student invitation claim.
pub fn course_roster_router(
    sessions: Arc<PostgresSessionStore>,
    roster: PostgresCourseRosterStore,
    signup_url: Arc<str>,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course_instance_id}/roster",
            get(list_course_roster).post(import_course_roster),
        )
        .route(
            "/api/course-instances/{course_instance_id}/roster/claim",
            post(claim_course_invitation),
        )
        .route(
            "/api/course-instances/{course_instance_id}/roster/{roster_id}/signup-reset",
            post(reset_student_signup_access),
        )
        .route(
            "/api/course-instances/{course_instance_id}/roster/{roster_id}/restore",
            post(restore_student_course_access),
        )
        .route(
            "/api/course-instances/{course_instance_id}/roster/{roster_id}/revoke",
            post(revoke_course_roster_entry),
        )
        .with_state(CourseRosterRouteState {
            sessions,
            roster,
            signup_url,
        })
        // Fifty rows of bounded Unicode names plus emails/IDs and JSON escaping.
        .layer(DefaultBodyLimit::max(256 * 1024))
}

async fn list_course_roster(
    State(state): State<CourseRosterRouteState>,
    headers: HeaderMap,
    Path(raw_course_instance_id): Path<String>,
) -> Response {
    let course = match course_instance_id(&raw_course_instance_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash =
        match required_session_hash(&state, &headers, |role| role == UserRole::Instructor).await {
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
    Path(raw_course_instance_id): Path<String>,
    Json(input): Json<CourseRosterImportInput>,
) -> Response {
    let course = match course_instance_id(&raw_course_instance_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash =
        match required_session_hash(&state, &headers, |role| role == UserRole::Instructor).await {
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
    Path(raw_course_instance_id): Path<String>,
) -> Response {
    let course = match course_instance_id(&raw_course_instance_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash =
        match required_session_hash(&state, &headers, |role| role == UserRole::Student).await {
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

async fn reset_student_signup_access(
    State(state): State<CourseRosterRouteState>,
    headers: HeaderMap,
    Path((raw_course_instance_id, roster_id)): Path<(String, String)>,
) -> Response {
    let course = match course_instance_id(&raw_course_instance_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash =
        match required_session_hash(&state, &headers, |role| role == UserRole::Instructor).await {
            Ok(value) => value,
            Err(response) => return *response,
        };
    match state
        .roster
        .reset_student_signup_access(session_hash, course, roster_id)
        .await
    {
        Ok(reset) => signup_download(InvitationMailerExport {
            course_name: reset.course_name,
            students: vec![InvitationMailerRecipient {
                email: reset.email,
                signup_url: state.signup_url.to_string(),
                display_name: None,
                roster_id: reset.roster_id,
            }],
        }),
        Err(error) => store_error_response(error),
    }
}

async fn restore_student_course_access(
    State(state): State<CourseRosterRouteState>,
    headers: HeaderMap,
    Path((raw_course_instance_id, roster_id)): Path<(String, String)>,
) -> Response {
    let course = match course_instance_id(&raw_course_instance_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash =
        match required_session_hash(&state, &headers, |role| role == UserRole::Instructor).await {
            Ok(value) => value,
            Err(response) => return *response,
        };
    match state
        .roster
        .restore_student_course_access(session_hash, course, roster_id)
        .await
    {
        Ok(()) => crate::auth::no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn revoke_course_roster_entry(
    State(state): State<CourseRosterRouteState>,
    headers: HeaderMap,
    Path((raw_course_instance_id, roster_id)): Path<(String, String)>,
) -> Response {
    let course = match course_instance_id(&raw_course_instance_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash =
        match required_session_hash(&state, &headers, |role| role == UserRole::Instructor).await {
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

fn signup_download(export: InvitationMailerExport) -> Response {
    let mut response = crate::auth::no_store(Json(export).into_response());
    response.headers_mut().insert(
        "content-disposition",
        axum::http::HeaderValue::from_static("attachment; filename=ple-signup-reset.json"),
    );
    response
}

fn course_instance_id(value: &str) -> Result<CourseInstanceId, Box<Response>> {
    CourseInstanceId::from_str(value).map_err(|_| Box::new(concealed()))
}

async fn required_session_hash(
    state: &CourseRosterRouteState,
    headers: &HeaderMap,
    permitted: impl FnOnce(UserRole) -> bool,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if permitted(session.record.user_role) => Ok(session.session_hash),
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
        StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Course Roster lifecycle conflict")
        }
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Course Roster is invalid")
        }
        StoreError::AlreadyExists => route_error(StatusCode::CONFLICT, "Course Roster conflict"),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signup_reset_mailer_document_carries_one_fresh_code() {
        let export = InvitationMailerExport {
            course_name: "Biology 301".to_string(),
            students: vec![InvitationMailerRecipient {
                email: "student@biology.roosevelt.edu".to_string(),
                signup_url: "https://live-demo.example/signup".to_string(),
                display_name: None,
                roster_id: "bio301-student".to_string(),
            }],
        };
        let response = signup_download(export);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-disposition")
                .map(|value| value.as_bytes()),
            Some(b"attachment; filename=ple-signup-reset.json".as_slice())
        );
    }
}
