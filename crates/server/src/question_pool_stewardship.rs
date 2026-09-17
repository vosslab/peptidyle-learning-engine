//! Vetted-Instructor Pool endorsements and actor-private Watch state.
//! Watch notifications are a separate capability, not delivered by these routes.

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
    QuestionPoolStarProjection, QuestionPoolStewardshipStore, QuestionPoolWatchProjection,
    SessionTokenHash, StoreError,
    postgres::{PostgresQuestionPoolStewardshipStore, PostgresSessionStore},
};
use question_model::{ProductRole, QuestionId};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::HmacQuestionIdIssuer,
};

const MAX_UPDATE_BYTES: usize = 128;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    stewardship: PostgresQuestionPoolStewardshipStore,
    issuer: HmacQuestionIdIssuer,
}

/// Registers closed own-state commands and public vetted-name endorsements.
pub fn question_pool_stewardship_router(
    sessions: Arc<PostgresSessionStore>,
    stewardship: PostgresQuestionPoolStewardshipStore,
    issuer: HmacQuestionIdIssuer,
) -> Router {
    Router::new()
        .route(
            "/api/question-pools/{question_pool_id}/stewardship/star",
            get(read_star).put(set_star),
        )
        .route(
            "/api/question-pools/{question_pool_id}/stewardship/watch",
            get(read_watch).put(set_watch),
        )
        .with_state(RouteState {
            sessions,
            stewardship,
            issuer,
        })
}

// ASVS 2.2.1 and 15.3.3: the caller chooses only its own explicit boolean.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetStarInput {
    starred: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetWatchInput {
    watching: bool,
}

// ASVS 8.2.3 and 15.3.1: no Account, email, Profile, or other-actor Watch data.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StarResponse {
    star_count: u64,
    viewer_has_starred: bool,
    starred_instructors: Vec<StarredInstructorResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StarredInstructorResponse {
    display_name: String,
}

#[derive(Serialize)]
struct WatchResponse {
    watching: bool,
}

impl From<QuestionPoolStarProjection> for StarResponse {
    fn from(value: QuestionPoolStarProjection) -> Self {
        Self {
            star_count: value.star_count,
            viewer_has_starred: value.viewer_has_starred,
            starred_instructors: value
                .starred_instructors
                .into_iter()
                .map(|instructor| StarredInstructorResponse {
                    display_name: instructor.display_name,
                })
                .collect(),
        }
    }
}

impl From<QuestionPoolWatchProjection> for WatchResponse {
    fn from(value: QuestionPoolWatchProjection) -> Self {
        Self {
            watching: value.watching,
        }
    }
}

async fn read_star(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(raw_pool_id): Path<String>,
) -> Response {
    let Some(pool_id) = verified_pool_id(&state.issuer, &raw_pool_id) else {
        return concealed();
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .question_pool_star_projection(session, &pool_id)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(StarResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn read_watch(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(raw_pool_id): Path<String>,
) -> Response {
    let Some(pool_id) = verified_pool_id(&state.issuer, &raw_pool_id) else {
        return concealed();
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .question_pool_watch_projection(session, &pool_id)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(WatchResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn set_star(
    State(state): State<RouteState>,
    Path(raw_pool_id): Path<String>,
    request: Request,
) -> Response {
    let Some(pool_id) = verified_pool_id(&state.issuer, &raw_pool_id) else {
        return concealed();
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !has_json_content_type(request.headers()) {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Question Pool stewardship is invalid",
        );
    }
    let input = match to_bytes(request.into_body(), MAX_UPDATE_BYTES).await {
        Ok(bytes) => match serde_json::from_slice::<SetStarInput>(&bytes) {
            Ok(value) => value,
            Err(_) => return invalid_input(),
        },
        Err(_) => return oversized_input(),
    };
    match state
        .stewardship
        .set_current_question_pool_star_projection(session, &pool_id, input.starred)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(StarResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn set_watch(
    State(state): State<RouteState>,
    Path(raw_pool_id): Path<String>,
    request: Request,
) -> Response {
    let Some(pool_id) = verified_pool_id(&state.issuer, &raw_pool_id) else {
        return concealed();
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !has_json_content_type(request.headers()) {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Question Pool stewardship is invalid",
        );
    }
    let input = match to_bytes(request.into_body(), MAX_UPDATE_BYTES).await {
        Ok(bytes) => match serde_json::from_slice::<SetWatchInput>(&bytes) {
            Ok(value) => value,
            Err(_) => return invalid_input(),
        },
        Err(_) => return oversized_input(),
    };
    match state
        .stewardship
        .set_current_question_pool_watch_projection(session, &pool_id, input.watching)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(WatchResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

/// ASVS 2.2.2: syntax-only public IDs do not suffice at the trusted boundary.
fn verified_pool_id(issuer: &HmacQuestionIdIssuer, value: &str) -> Option<QuestionId> {
    let pool_id = value.parse::<QuestionId>().ok()?;
    issuer.validates_question_id(&pool_id).then_some(pool_id)
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
        // ASVS 8.2.1 and 8.3.1: actor is session-derived; SQL independently
        // rechecks active Instructor status and immutable completed vetting.
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Pool stewardship authentication unavailable",
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

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => invalid_input(),
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Question Pool stewardship changed",
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Question Pool stewardship conflict")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Pool stewardship unavailable",
        ),
    }
}

fn invalid_input() -> Response {
    route_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Question Pool stewardship is invalid",
    )
}

fn oversized_input() -> Response {
    route_error(
        StatusCode::PAYLOAD_TOO_LARGE,
        "Question Pool stewardship is too large",
    )
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Pool stewardship not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    // ASVS 14.3.2 and 16.5.1: private own-state responses are never cached;
    // unavailable and denied resources expose no persistence diagnostics.
    crate::auth::no_store((status, message).into_response())
}
