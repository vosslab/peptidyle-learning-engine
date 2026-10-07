//! Owner-authorized ordinary Published Question metadata replacement.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{QuestionMetadataStore, SessionStore, SessionTokenHash, StoreError};
use question_model::{SaveQuestionMetadataRequest, UserRole};
use serde::Serialize;

use crate::auth::{AuthError, resolve_session};

const MAX_QUESTION_METADATA_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<dyn SessionStore>,
    store: Arc<dyn QuestionMetadataStore>,
}

/// Registers the ordinary single-Question metadata save capability.
pub fn question_metadata_router(
    sessions: Arc<dyn SessionStore>,
    store: Arc<dyn QuestionMetadataStore>,
) -> Router {
    Router::new()
        .route(
            "/api/questions/by-id/{question_id}/metadata",
            post(save_question_metadata),
        )
        .with_state(RouteState { sessions, store })
}

async fn save_question_metadata(
    State(state): State<RouteState>,
    Path(raw_question_id): Path<String>,
    request: Request,
) -> Response {
    if !has_json_content_type(request.headers()) {
        return invalid(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let session_hash = match metadata_editor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let body = match to_bytes(request.into_body(), MAX_QUESTION_METADATA_REQUEST_BYTES).await {
        Ok(body) => body,
        Err(_) => return invalid(StatusCode::PAYLOAD_TOO_LARGE),
    };
    let save = match serde_json::from_slice::<SaveQuestionMetadataRequest>(&body) {
        Ok(value) => value,
        Err(_) => return invalid(StatusCode::UNPROCESSABLE_ENTITY),
    };
    if save
        .published_question_revision_tuple
        .published_question_id
        .as_str()
        != raw_question_id
    {
        return invalid(StatusCode::UNPROCESSABLE_ENTITY);
    }
    match state.store.save_question_metadata(session_hash, save).await {
        Ok(receipt) => crate::auth::no_store(Json(receipt).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn metadata_editor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session)
            if matches!(
                session.record.user_role,
                UserRole::Instructor | UserRole::Sysadmin
            ) =>
        {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => {
            Err(Box::new(invalid(StatusCode::SERVICE_UNAVAILABLE)))
        }
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

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

fn concealed() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "unavailable",
        }),
    )
        .into_response()
}

fn invalid(status: StatusCode) -> Response {
    (
        status,
        Json(ErrorResponse {
            error: "invalid_question_metadata",
        }),
    )
        .into_response()
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::Conflict => (
            StatusCode::PRECONDITION_FAILED,
            Json(ErrorResponse {
                error: "metadata_conflict",
            }),
        )
            .into_response(),
        StoreError::Forbidden | StoreError::NotFound | StoreError::OwnershipMismatch => concealed(),
        StoreError::LifecycleConflict => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "question_unavailable",
            }),
        )
            .into_response(),
        StoreError::InvalidRecord(_) => invalid(StatusCode::UNPROCESSABLE_ENTITY),
        StoreError::RetryableTransaction => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse { error: "retry" }),
        )
            .into_response(),
        StoreError::Unavailable(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "unavailable",
            }),
        )
            .into_response(),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "unavailable",
            }),
        )
            .into_response(),
    }
}

fn has_json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(';').next().is_some_and(|media_type| {
                media_type.trim().eq_ignore_ascii_case("application/json")
            })
        })
}
