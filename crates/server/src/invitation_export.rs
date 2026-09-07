//! Protected direct-Instructor download of pending Student invitation mailer input.

use std::{str::FromStr, sync::Arc};

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    InvitationExportStore, SessionTokenHash, StoreError,
    postgres::{PostgresInvitationExportStore, PostgresSessionStore},
};
use question_model::{CourseInstanceReference, ProductRole};

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct StateData {
    sessions: Arc<PostgresSessionStore>,
    exports: PostgresInvitationExportStore,
    signup_url: Arc<str>,
}

/// Registers the private JSON download consumed by the attended mailer.
pub fn invitation_export_router(
    sessions: Arc<PostgresSessionStore>,
    exports: PostgresInvitationExportStore,
    signup_url: Arc<str>,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course}/invitation-export",
            get(download_pending_invitations),
        )
        .with_state(StateData {
            sessions,
            exports,
            signup_url,
        })
}

async fn download_pending_invitations(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match CourseInstanceReference::from_str(&course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .exports
        .export_pending_course_invitations(token, course)
        .await
    {
        Ok(export) => download_response(export.into_mailer_export(state.signup_url.to_string())),
        Err(error) => store_error(error),
    }
}

fn download_response(export: learning_data_access::InvitationMailerExport) -> Response {
    // ASVS 8.3.1: attachment disposition prevents a sensitive recipient list
    // from being rendered as a navigable API document in the browser.
    let mut response = crate::auth::no_store(axum::Json(export).into_response());
    response.headers_mut().insert(
        "content-disposition",
        HeaderValue::from_static("attachment; filename=ple-invitations.json"),
    );
    response
}

async fn instructor(
    state: &StateData,
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
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Invitation export authentication unavailable",
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

fn store_error(error_value: StoreError) -> Response {
    match error_value {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => {
            error(StatusCode::PRECONDITION_FAILED, "Invitation export changed")
        }
        StoreError::InvalidRecord(_) | StoreError::AlreadyExists => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invitation export is invalid",
        ),
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Invitation export unavailable",
            )
        }
    }
}

fn concealed() -> Response {
    error(StatusCode::NOT_FOUND, "Invitation export not found")
}

fn error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
