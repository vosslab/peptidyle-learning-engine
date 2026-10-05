//! Retained Library Object impact-notice controls.
//!
//! The route accepts only an exact canonical public Library Object ID. SQL
//! derives the actor, object owner, and Sysadmin authority.

use std::sync::Arc;

use axum::{
    Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{post, put},
};
use learning_data_access::{
    LibraryImpactNoticeStore, LibraryObjectTarget, SessionTokenHash, StoreError,
    postgres::{PostgresLibraryImpactNoticeStore, PostgresSessionStore},
};
use question_model::{LibraryObjectKind, PublishedQuestionId, UserRole};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

// Four thousand Unicode scalar values can require sixteen KiB in UTF-8;
// this leaves bounded JSON-envelope room without rejecting valid text first.
const MAX_MUTATION_BYTES: usize = 20 * 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    impact_notices: PostgresLibraryImpactNoticeStore,
}

/// Registers retained impact-notice controls for Questions and Pools.
pub fn library_impact_notice_router(
    sessions: Arc<PostgresSessionStore>,
    impact_notices: PostgresLibraryImpactNoticeStore,
) -> Router {
    Router::new()
        .route(
            "/api/library-objects/{kind}/{public_id}/impact-notices",
            post(create_impact_notice),
        )
        .route(
            "/api/library-objects/{kind}/{public_id}/impact-notices/{notice_id}",
            put(update_impact_notice),
        )
        .route(
            "/api/library-objects/{kind}/{public_id}/impact-notices/{notice_id}/cancel",
            put(cancel_impact_notice),
        )
        .with_state(RouteState {
            sessions,
            impact_notices,
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImpactNoticeInput {
    affected_revision_number: Option<u64>,
    body: String,
}

async fn create_impact_notice(
    State(state): State<RouteState>,
    Path((kind, public_id)): Path<(String, String)>,
    request: Request,
) -> Response {
    let target = match target(&kind, &public_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_or_sysadmin_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let input = match json_input::<ImpactNoticeInput>(request).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .impact_notices
        .create_impact_notice(
            session,
            &target,
            input.affected_revision_number,
            &input.body,
        )
        .await
    {
        Ok(_) => accepted(),
        Err(error) => store_error_response(error),
    }
}

async fn update_impact_notice(
    State(state): State<RouteState>,
    Path((kind, public_id, notice_id)): Path<(String, String, String)>,
    request: Request,
) -> Response {
    let target = match target(&kind, &public_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let notice_id = match Uuid::parse_str(&notice_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session = match instructor_or_sysadmin_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let input = match json_input::<ImpactNoticeInput>(request).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .impact_notices
        .update_impact_notice(
            session,
            &target,
            notice_id,
            input.affected_revision_number,
            &input.body,
        )
        .await
    {
        Ok(()) => accepted(),
        Err(error) => store_error_response(error),
    }
}

async fn cancel_impact_notice(
    State(state): State<RouteState>,
    Path((kind, public_id, notice_id)): Path<(String, String, String)>,
    request: Request,
) -> Response {
    let target = match target(&kind, &public_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let notice_id = match Uuid::parse_str(&notice_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session = match instructor_or_sysadmin_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if let Err(response) = empty_body(request).await {
        return *response;
    }
    match state
        .impact_notices
        .cancel_impact_notice(session, &target, notice_id)
        .await
    {
        Ok(()) => accepted(),
        Err(error) => store_error_response(error),
    }
}

async fn empty_body(request: Request) -> Result<(), Box<Response>> {
    let has_json_content_type = has_json_content_type(request.headers());
    let bytes = to_bytes(request.into_body(), MAX_MUTATION_BYTES)
        .await
        .map_err(|_| {
            Box::new(route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Impact notice is too large",
            ))
        })?;
    if bytes.is_empty() {
        return Ok(());
    }
    if !has_json_content_type {
        return Err(Box::new(route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Impact notice is invalid",
        )));
    }
    Err(Box::new(invalid_input()))
}

fn target(kind: &str, public_id: &str) -> Option<LibraryObjectTarget> {
    let kind = match kind {
        "question" => LibraryObjectKind::Question,
        "questionPool" => LibraryObjectKind::QuestionPool,
        _ => return None,
    };
    let public_id = public_id.parse::<PublishedQuestionId>().ok()?;
    Some(LibraryObjectTarget { kind, public_id })
}

async fn json_input<T: serde::de::DeserializeOwned>(request: Request) -> Result<T, Box<Response>> {
    if !has_json_content_type(request.headers()) {
        return Err(Box::new(route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Impact notice is invalid",
        )));
    }
    let bytes = to_bytes(request.into_body(), MAX_MUTATION_BYTES)
        .await
        .map_err(|_| {
            Box::new(route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Impact notice is too large",
            ))
        })?;
    serde_json::from_slice(&bytes).map_err(|_| Box::new(invalid_input()))
}

async fn instructor_or_sysadmin_session_hash(
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
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Impact notice authentication unavailable",
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
            value
                .split(';')
                .next()
                .is_some_and(|type_| type_.trim().eq_ignore_ascii_case("application/json"))
        })
}

fn accepted() -> Response {
    crate::auth::no_store(StatusCode::NO_CONTENT.into_response())
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => invalid_input(),
        StoreError::Conflict => {
            route_error(StatusCode::PRECONDITION_FAILED, "Impact notice changed")
        }
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Impact notice conflict")
        }
        StoreError::RetryableTransaction
        | StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => {
            route_error(StatusCode::SERVICE_UNAVAILABLE, "Impact notice unavailable")
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Impact notice not found")
}

fn invalid_input() -> Response {
    route_error(StatusCode::UNPROCESSABLE_ENTITY, "Impact notice is invalid")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
