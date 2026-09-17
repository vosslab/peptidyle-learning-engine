//! Vetted-Instructor Library Object improvement threads and impact notices.
//!
//! The route accepts only an exact canonical public Library Object ID. SQL
//! derives the actor, visible identity, object owner, and Sysadmin authority.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use learning_data_access::{
    LibraryDiscussionStore, LibraryDiscussionTarget, LibraryDiscussionView, LibraryImpactNotice,
    LibraryImpactNoticeState, LibraryImprovementPost, LibraryImprovementThread,
    LibraryImprovementThreadState, SessionTokenHash, StoreError,
    postgres::{PostgresLibraryDiscussionStore, PostgresSessionStore},
};
use question_model::{LibraryObjectKind, ProductRole, QuestionId, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

// Four thousand Unicode scalar values can require sixteen KiB in UTF-8;
// this leaves bounded JSON-envelope room without rejecting valid text first.
const MAX_MUTATION_BYTES: usize = 20 * 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    discussions: PostgresLibraryDiscussionStore,
}

/// Registers the one retained discussion surface shared by Questions and Pools.
pub fn library_discussion_router(
    sessions: Arc<PostgresSessionStore>,
    discussions: PostgresLibraryDiscussionStore,
) -> Router {
    Router::new()
        .route(
            "/api/library-objects/{kind}/{public_id}/discussions",
            get(read_discussions),
        )
        .route(
            "/api/library-objects/{kind}/{public_id}/improvement-threads",
            post(create_thread),
        )
        .route(
            "/api/library-objects/{kind}/{public_id}/improvement-threads/{thread_id}/posts",
            post(reply_to_thread),
        )
        .route(
            "/api/library-objects/{kind}/{public_id}/improvement-posts/{post_id}",
            put(edit_post),
        )
        .route(
            "/api/library-objects/{kind}/{public_id}/improvement-threads/{thread_id}/state",
            put(set_thread_state),
        )
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
            discussions,
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TextInput {
    body: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ThreadStateInput {
    resolved: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImpactNoticeInput {
    affected_revision_number: Option<u64>,
    body: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscussionResponse {
    viewer_may_manage: bool,
    threads: Vec<ThreadResponse>,
    impact_notices: Vec<ImpactNoticeResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ThreadResponse {
    thread_id: String,
    creation_revision_number: u64,
    state: &'static str,
    created_at: i64,
    resolved_at: Option<i64>,
    viewer_may_resolve: bool,
    posts: Vec<PostResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PostResponse {
    post_id: String,
    author_display_name: String,
    body: String,
    created_at: i64,
    updated_at: Option<i64>,
    viewer_may_edit: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImpactNoticeResponse {
    impact_notice_id: String,
    affected_revision_number: Option<u64>,
    author_display_name: String,
    body: String,
    state: &'static str,
    created_at: i64,
    updated_at: i64,
    cancelled_at: Option<i64>,
    viewer_may_manage: bool,
}

impl From<LibraryDiscussionView> for DiscussionResponse {
    fn from(value: LibraryDiscussionView) -> Self {
        Self {
            viewer_may_manage: value.viewer_may_manage,
            threads: value
                .threads
                .into_iter()
                .map(ThreadResponse::from)
                .collect(),
            impact_notices: value
                .impact_notices
                .into_iter()
                .map(ImpactNoticeResponse::from)
                .collect(),
        }
    }
}

impl From<LibraryImprovementThread> for ThreadResponse {
    fn from(value: LibraryImprovementThread) -> Self {
        Self {
            thread_id: value.thread_id.to_string(),
            creation_revision_number: value.creation_revision_number,
            state: match value.state {
                LibraryImprovementThreadState::Open => "open",
                LibraryImprovementThreadState::Resolved => "resolved",
            },
            created_at: millis(value.created_at),
            resolved_at: value.resolved_at.map(millis),
            viewer_may_resolve: value.viewer_may_resolve,
            posts: value.posts.into_iter().map(PostResponse::from).collect(),
        }
    }
}

impl From<LibraryImprovementPost> for PostResponse {
    fn from(value: LibraryImprovementPost) -> Self {
        Self {
            post_id: value.post_id.to_string(),
            author_display_name: value.author_display_name,
            body: value.body,
            created_at: millis(value.created_at),
            updated_at: value.updated_at.map(millis),
            viewer_may_edit: value.viewer_may_edit,
        }
    }
}

impl From<LibraryImpactNotice> for ImpactNoticeResponse {
    fn from(value: LibraryImpactNotice) -> Self {
        Self {
            impact_notice_id: value.impact_notice_id.to_string(),
            affected_revision_number: value.affected_revision_number,
            author_display_name: value.author_display_name,
            body: value.body,
            state: match value.state {
                LibraryImpactNoticeState::Active => "active",
                LibraryImpactNoticeState::Cancelled => "cancelled",
            },
            created_at: millis(value.created_at),
            updated_at: millis(value.updated_at),
            cancelled_at: value.cancelled_at.map(millis),
            viewer_may_manage: value.viewer_may_manage,
        }
    }
}

fn millis(value: Timestamp) -> i64 {
    value.as_unix_millis()
}

async fn read_discussions(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path((kind, public_id)): Path<(String, String)>,
) -> Response {
    let target = match target(&kind, &public_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_or_sysadmin_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .discussions
        .library_discussion_view(session, &target)
        .await
    {
        Ok(value) => crate::auth::no_store(Json(DiscussionResponse::from(value)).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn create_thread(
    State(state): State<RouteState>,
    Path((kind, public_id)): Path<(String, String)>,
    request: Request,
) -> Response {
    let target = match target(&kind, &public_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let input = match json_input::<TextInput>(request).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .discussions
        .create_improvement_thread(session, &target, &input.body)
        .await
    {
        Ok(_) => accepted(),
        Err(error) => store_error_response(error),
    }
}

async fn reply_to_thread(
    State(state): State<RouteState>,
    Path((kind, public_id, thread_id)): Path<(String, String, String)>,
    request: Request,
) -> Response {
    let target = match target(&kind, &public_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let thread_id = match Uuid::parse_str(&thread_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let input = match json_input::<TextInput>(request).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .discussions
        .reply_to_improvement_thread(session, &target, thread_id, &input.body)
        .await
    {
        Ok(_) => accepted(),
        Err(error) => store_error_response(error),
    }
}

async fn edit_post(
    State(state): State<RouteState>,
    Path((kind, public_id, post_id)): Path<(String, String, String)>,
    request: Request,
) -> Response {
    let target = match target(&kind, &public_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let post_id = match Uuid::parse_str(&post_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let input = match json_input::<TextInput>(request).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .discussions
        .edit_own_improvement_post(session, &target, post_id, &input.body)
        .await
    {
        Ok(()) => accepted(),
        Err(error) => store_error_response(error),
    }
}

async fn set_thread_state(
    State(state): State<RouteState>,
    Path((kind, public_id, thread_id)): Path<(String, String, String)>,
    request: Request,
) -> Response {
    let target = match target(&kind, &public_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let thread_id = match Uuid::parse_str(&thread_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session = match instructor_or_sysadmin_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let input = match json_input::<ThreadStateInput>(request).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .discussions
        .set_improvement_thread_resolved(session, &target, thread_id, input.resolved)
        .await
    {
        Ok(()) => accepted(),
        Err(error) => store_error_response(error),
    }
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
        .discussions
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
        .discussions
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
    if request
        .headers()
        .get("content-length")
        .is_some_and(|value| value != "0")
    {
        return invalid_input();
    }
    match state
        .discussions
        .cancel_impact_notice(session, &target, notice_id)
        .await
    {
        Ok(()) => accepted(),
        Err(error) => store_error_response(error),
    }
}

fn target(kind: &str, public_id: &str) -> Option<LibraryDiscussionTarget> {
    let kind = match kind {
        "question" => LibraryObjectKind::Question,
        "questionPool" => LibraryObjectKind::QuestionPool,
        _ => return None,
    };
    let public_id = public_id.parse::<QuestionId>().ok()?;
    Some(LibraryDiscussionTarget { kind, public_id })
}

async fn json_input<T: serde::de::DeserializeOwned>(request: Request) -> Result<T, Box<Response>> {
    if !has_json_content_type(request.headers()) {
        return Err(Box::new(route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Library discussion is invalid",
        )));
    }
    let bytes = to_bytes(request.into_body(), MAX_MUTATION_BYTES)
        .await
        .map_err(|_| {
            Box::new(route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Library discussion is too large",
            ))
        })?;
    serde_json::from_slice(&bytes).map_err(|_| Box::new(invalid_input()))
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
            "Library discussion authentication unavailable",
        ))),
    }
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
                session.record.product_role,
                ProductRole::Instructor | ProductRole::Sysadmin
            ) =>
        {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Library discussion authentication unavailable",
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
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Library discussion changed",
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Library discussion conflict")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Library discussion unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Library discussion not found")
}

fn invalid_input() -> Response {
    route_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Library discussion is invalid",
    )
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
