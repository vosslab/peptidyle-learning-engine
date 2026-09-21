//! Vetted-Instructor Question Star HTTP boundary.
//!
//! This route exposes only an Instructor's own Star state plus an aggregate
//! count and approved public display identities. It has no Watch read path and
//! serializes no account identifier, email, credential, source, or Student fact.

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
    QuestionStarProjection, QuestionStarStore, QuestionStarredInstructor, SessionTokenHash,
    StoreError,
    postgres::{PostgresQuestionStarStore, PostgresSessionStore},
};
use question_model::{ProductRole, PublishedQuestionId};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthError, resolve_session};

const MAX_STAR_UPDATE_BYTES: usize = 128;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    stars: PostgresQuestionStarStore,
}

/// Registers the Question Star surface. It is intentionally separate from
/// Question Library source/detail delivery and has no Watch route.
pub fn question_stewardship_router(
    sessions: Arc<PostgresSessionStore>,
    stars: PostgresQuestionStarStore,
) -> Router {
    Router::new()
        .route(
            "/api/questions/by-id/{question_id}/stewardship/star",
            get(read_star).put(set_star),
        )
        .with_state(RouteState { sessions, stars })
}

/// Closed browser payload. The caller may choose only its own desired state.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetStarInput {
    starred: bool,
}

/// Browser-safe Star state. This is a closed projection: only active vetted
/// Instructor display names can be included, never durable Account identity.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StarResponse {
    star_count: u64,
    viewer_has_starred: bool,
    starred_instructors: Vec<StarredInstructorResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StarredInstructorResponse {
    display_name: String,
}

impl From<QuestionStarredInstructor> for StarredInstructorResponse {
    fn from(value: QuestionStarredInstructor) -> Self {
        Self {
            display_name: value.display_name,
        }
    }
}

impl From<QuestionStarProjection> for StarResponse {
    fn from(value: QuestionStarProjection) -> Self {
        Self {
            star_count: value.star_count,
            viewer_has_starred: value.viewer_has_starred,
            starred_instructors: value
                .starred_instructors
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

async fn read_star(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(raw_question_id): Path<String>,
) -> Response {
    let question_id = match verified_question_id(&raw_question_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stars
        .question_star_projection(session, &question_id)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(StarResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn set_star(
    State(state): State<RouteState>,
    Path(raw_question_id): Path<String>,
    request: Request,
) -> Response {
    let question_id = match verified_question_id(&raw_question_id) {
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
            "Question Star is invalid",
        );
    }
    let input = match to_bytes(request.into_body(), MAX_STAR_UPDATE_BYTES).await {
        Ok(bytes) => match serde_json::from_slice::<SetStarInput>(&bytes) {
            Ok(value) => value,
            Err(_) => {
                return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Star is invalid");
            }
        },
        Err(_) => return route_error(StatusCode::PAYLOAD_TOO_LARGE, "Question Star is too large"),
    };
    match state
        .stars
        .set_current_question_star(session, &question_id, input.starred)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(StarResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

/// Parses the exact checksum-bearing ID before any persistence lookup.
/// ASVS 2.2.1--2.2.2: only an exact canonical Question identity reaches the
/// Star Store.
fn verified_question_id(value: &str) -> Option<PublishedQuestionId> {
    value.parse::<PublishedQuestionId>().ok()
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
        // ASVS 8.2.1 and 8.3.1: role authorization is server-derived. The
        // subsequent database procedure independently rechecks active status.
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Star authentication unavailable",
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
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Star is invalid")
        }
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Question Star changed")
        }
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Question Star conflict")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => {
            route_error(StatusCode::SERVICE_UNAVAILABLE, "Question Star unavailable")
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Star not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
