//! Private vetted-Instructor Question Watch HTTP boundary.
//!
//! This surface carries only the current Instructor's Watch boolean. It never
//! returns a watcher count, identity, list, activity, or notification record.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    QuestionWatchProjection, QuestionWatchStore, SessionTokenHash, StoreError,
    postgres::{PostgresQuestionWatchStore, PostgresSessionStore},
};
use question_model::{ProductRole, QuestionId};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::HmacQuestionIdIssuer,
};

const MAX_WATCH_UPDATE_BYTES: usize = 128;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    watches: PostgresQuestionWatchStore,
    question_id_issuer: HmacQuestionIdIssuer,
}

/// Registers the self-only Question Watch surface for Published Questions.
pub fn question_watch_router(
    sessions: Arc<PostgresSessionStore>,
    watches: PostgresQuestionWatchStore,
    question_id_issuer: HmacQuestionIdIssuer,
) -> Router {
    Router::new()
        .route(
            "/api/questions/by-id/{question_id}/stewardship/watch",
            get(read_watch).put(set_watch),
        )
        .with_state(RouteState {
            sessions,
            watches,
            question_id_issuer,
        })
}

/// Closed browser payload; clients select only their own desired Watch state.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetWatchInput {
    watching: bool,
}

/// Closed browser response for one current Instructor and Question lineage.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchResponse {
    watching: bool,
}

impl From<QuestionWatchProjection> for WatchResponse {
    fn from(value: QuestionWatchProjection) -> Self {
        Self {
            watching: value.watching,
        }
    }
}

async fn read_watch(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(raw_question_id): Path<String>,
) -> Response {
    let question_id = match verified_question_id(&state.question_id_issuer, &raw_question_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .watches
        .question_watch_projection(session, &question_id)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(WatchResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn set_watch(
    State(state): State<RouteState>,
    Path(raw_question_id): Path<String>,
    request: Request,
) -> Response {
    let question_id = match verified_question_id(&state.question_id_issuer, &raw_question_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !has_content_type(request.headers(), "application/json") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Question Watch is invalid",
        );
    }
    let input = match to_bytes(request.into_body(), MAX_WATCH_UPDATE_BYTES).await {
        Ok(bytes) => match serde_json::from_slice::<SetWatchInput>(&bytes) {
            Ok(value) => value,
            Err(_) => {
                return route_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Question Watch is invalid",
                );
            }
        },
        Err(_) => return route_error(StatusCode::PAYLOAD_TOO_LARGE, "Question Watch is too large"),
    };
    match state
        .watches
        .set_current_question_watch(session, &question_id, input.watching)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(WatchResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

/// Validates syntax and server-held HMAC before a private Watch lookup.
fn verified_question_id(
    question_id_issuer: &HmacQuestionIdIssuer,
    value: &str,
) -> Option<QuestionId> {
    let question_id = value.parse::<QuestionId>().ok()?;
    question_id_issuer
        .validates_question_id(&question_id)
        .then_some(question_id)
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
            "Question Watch authentication unavailable",
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

fn has_content_type(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(expected))
        })
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Question Watch is invalid",
        ),
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Question Watch changed")
        }
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Question Watch conflict")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Watch unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Watch not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
