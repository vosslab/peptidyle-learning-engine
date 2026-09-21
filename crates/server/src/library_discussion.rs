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
    LibraryImpactNoticeLifecycle, LibraryImprovementPost, LibraryImprovementThread,
    LibraryImprovementThreadLifecycle, SessionTokenHash, StoreError,
    postgres::{PostgresLibraryDiscussionStore, PostgresSessionStore},
};
use question_model::{LibraryObjectKind, ProductRole, PublishedQuestionId, Timestamp};
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
        let (state, resolved_at) = match value.lifecycle {
            LibraryImprovementThreadLifecycle::Open => ("open", None),
            LibraryImprovementThreadLifecycle::Resolved { resolved_at } => {
                ("resolved", Some(millis(resolved_at)))
            }
        };
        Self {
            thread_id: value.thread_id.to_string(),
            creation_revision_number: value.creation_revision_number,
            state,
            created_at: millis(value.created_at),
            resolved_at,
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
        let (state, cancelled_at) = match value.lifecycle {
            LibraryImpactNoticeLifecycle::Active => ("active", None),
            LibraryImpactNoticeLifecycle::Cancelled { cancelled_at } => {
                ("cancelled", Some(millis(cancelled_at)))
            }
        };
        Self {
            impact_notice_id: value.impact_notice_id.to_string(),
            affected_revision_number: value.affected_revision_number,
            author_display_name: value.author_display_name,
            body: value.body,
            state,
            created_at: millis(value.created_at),
            updated_at: millis(value.updated_at),
            cancelled_at,
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
    if let Err(response) = empty_body(request).await {
        return *response;
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

async fn empty_body(request: Request) -> Result<(), Box<Response>> {
    let has_json_content_type = has_json_content_type(request.headers());
    // ASVS 2.2.1/4.2.1: consume the framed body under the same bound as the
    // other mutations. Content-Length is never trusted as body evidence.
    let bytes = to_bytes(request.into_body(), MAX_MUTATION_BYTES)
        .await
        .map_err(|_| {
            Box::new(route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Library discussion is too large",
            ))
        })?;
    if bytes.is_empty() {
        return Ok(());
    }
    if !has_json_content_type {
        return Err(Box::new(route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Library discussion is invalid",
        )));
    }
    Err(Box::new(invalid_input()))
}

fn target(kind: &str, public_id: &str) -> Option<LibraryDiscussionTarget> {
    let kind = match kind {
        "question" => LibraryObjectKind::Question,
        "questionPool" => LibraryObjectKind::QuestionPool,
        _ => return None,
    };
    let public_id = public_id.parse::<PublishedQuestionId>().ok()?;
    Some(LibraryDiscussionTarget { kind, public_id })
}

async fn json_input<T: serde::de::DeserializeOwned>(request: Request) -> Result<T, Box<Response>> {
    // ASVS 1.5.2/2.2.1/4.1.1: accept only bounded JSON into closed,
    // deny-unknown-fields request types at this trusted service boundary.
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
    // ASVS 8.2.1/8.3.1: this is only an early role rejection. SQL repeats
    // participant authorization from the installed session and exact target.
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
    // ASVS 8.2.1/8.3.1: this prefilter does not grant target authority;
    // the Store's reader or manager operation remains authoritative.
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
        StoreError::Conflict => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Library discussion changed",
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Library discussion conflict")
        }
        StoreError::RetryableTransaction
        | StoreError::AssessmentActivity(_)
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
    // ASVS 14.2.2/16.5.1: every outcome is non-cacheable and public errors
    // expose no target, authorization, SQL, or authentication diagnostics.
    crate::auth::no_store((status, message).into_response())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use axum::http::header::CACHE_CONTROL;
    use serde_json::json;

    use super::*;

    fn post() -> LibraryImprovementPost {
        LibraryImprovementPost {
            post_id: Uuid::from_u128(2),
            author_display_name: "Instructor Example".to_owned(),
            body: "Clarify this prompt.".to_owned(),
            created_at: Timestamp::from_unix_millis(10),
            updated_at: None,
            viewer_may_edit: true,
        }
    }

    fn thread(lifecycle: LibraryImprovementThreadLifecycle) -> LibraryImprovementThread {
        LibraryImprovementThread {
            thread_id: Uuid::from_u128(1),
            creation_revision_number: 3,
            lifecycle,
            created_at: Timestamp::from_unix_millis(10),
            viewer_may_resolve: true,
            posts: vec![post()],
        }
    }

    fn notice(lifecycle: LibraryImpactNoticeLifecycle) -> LibraryImpactNotice {
        LibraryImpactNotice {
            impact_notice_id: Uuid::from_u128(3),
            affected_revision_number: Some(4),
            author_display_name: "Sysadmin".to_owned(),
            body: "This issue affects revision 4.".to_owned(),
            lifecycle,
            created_at: Timestamp::from_unix_millis(20),
            updated_at: Timestamp::from_unix_millis(30),
            viewer_may_manage: false,
        }
    }

    #[test]
    fn thread_lifecycle_serializes_to_the_stable_wire_shape() {
        let open = serde_json::to_value(ThreadResponse::from(thread(
            LibraryImprovementThreadLifecycle::Open,
        )))
        .expect("open thread serializes");
        let resolved = serde_json::to_value(ThreadResponse::from(thread(
            LibraryImprovementThreadLifecycle::Resolved {
                resolved_at: Timestamp::from_unix_millis(15),
            },
        )))
        .expect("resolved thread serializes");

        assert_eq!(open["state"], "open");
        assert_eq!(open["resolvedAt"], serde_json::Value::Null);
        assert_eq!(resolved["state"], "resolved");
        assert_eq!(resolved["resolvedAt"], 15);
        assert_eq!(
            open.as_object()
                .expect("thread response is an object")
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            [
                "createdAt",
                "creationRevisionNumber",
                "posts",
                "resolvedAt",
                "state",
                "threadId",
                "viewerMayResolve",
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn impact_notice_lifecycle_serializes_to_the_stable_wire_shape() {
        let active = serde_json::to_value(ImpactNoticeResponse::from(notice(
            LibraryImpactNoticeLifecycle::Active,
        )))
        .expect("active notice serializes");
        let cancelled = serde_json::to_value(ImpactNoticeResponse::from(notice(
            LibraryImpactNoticeLifecycle::Cancelled {
                cancelled_at: Timestamp::from_unix_millis(30),
            },
        )))
        .expect("cancelled notice serializes");

        assert_eq!(active["state"], "active");
        assert_eq!(active["cancelledAt"], serde_json::Value::Null);
        assert_eq!(cancelled["state"], "cancelled");
        assert_eq!(cancelled["cancelledAt"], 30);
        assert_eq!(
            cancelled,
            json!({
                "impactNoticeId": Uuid::from_u128(3).to_string(),
                "affectedRevisionNumber": 4,
                "authorDisplayName": "Sysadmin",
                "body": "This issue affects revision 4.",
                "state": "cancelled",
                "createdAt": 20,
                "updatedAt": 30,
                "cancelledAt": 30,
                "viewerMayManage": false,
            })
        );
    }

    #[test]
    fn store_errors_follow_the_concealed_http_status_contract() {
        let cases = [
            (StoreError::NotFound, StatusCode::NOT_FOUND),
            (StoreError::Forbidden, StatusCode::NOT_FOUND),
            (StoreError::OwnershipMismatch, StatusCode::NOT_FOUND),
            (
                StoreError::InvalidRecord("invalid".to_owned()),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (StoreError::LifecycleConflict, StatusCode::CONFLICT),
            (StoreError::AlreadyExists, StatusCode::CONFLICT),
            (StoreError::Conflict, StatusCode::PRECONDITION_FAILED),
            (
                StoreError::RetryableTransaction,
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                StoreError::Unavailable("offline".to_owned()),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        ];

        for (error, expected) in cases {
            let response = store_error_response(error);
            assert_eq!(response.status(), expected);
            assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        }
    }

    #[tokio::test]
    async fn mutation_json_boundary_enforces_media_shape_and_byte_limit() {
        let unsupported = json_input::<TextInput>(
            Request::builder()
                .body(Body::from(r#"{"body":"valid"}"#))
                .expect("request builds"),
        )
        .await
        .expect_err("missing JSON content type is rejected");
        assert_eq!(unsupported.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(unsupported.headers()[CACHE_CONTROL], "no-store");

        let unknown = json_input::<TextInput>(
            Request::builder()
                .header("content-type", "application/json; charset=utf-8")
                .body(Body::from(r#"{"body":"valid","extra":true}"#))
                .expect("request builds"),
        )
        .await
        .expect_err("unknown JSON fields are rejected");
        assert_eq!(unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(unknown.headers()[CACHE_CONTROL], "no-store");

        let oversized = json_input::<TextInput>(
            Request::builder()
                .header("content-type", "application/json")
                .body(Body::from(vec![b' '; MAX_MUTATION_BYTES + 1]))
                .expect("request builds"),
        )
        .await
        .expect_err("oversized JSON envelope is rejected");
        assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(oversized.headers()[CACHE_CONTROL], "no-store");
    }

    #[tokio::test]
    async fn cancellation_accepts_only_an_actually_empty_bounded_body() {
        empty_body(
            Request::builder()
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("empty cancellation body is accepted");

        let missing_media_type = empty_body(
            Request::builder()
                .header("content-length", "0")
                .body(Body::from("{}"))
                .expect("request builds"),
        )
        .await
        .expect_err("actual bytes override a false empty Content-Length");
        assert_eq!(
            missing_media_type.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(missing_media_type.headers()[CACHE_CONTROL], "no-store");

        let wrong_media_type = empty_body(
            Request::builder()
                .header("content-type", "text/plain")
                .body(Body::from("payload"))
                .expect("request builds"),
        )
        .await
        .expect_err("non-JSON cancellation body is rejected");
        assert_eq!(
            wrong_media_type.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(wrong_media_type.headers()[CACHE_CONTROL], "no-store");

        let json_body = empty_body(
            Request::builder()
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request builds"),
        )
        .await
        .expect_err("cancellation has no JSON body shape");
        assert_eq!(json_body.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json_body.headers()[CACHE_CONTROL], "no-store");

        let oversized = empty_body(
            Request::builder()
                .header("content-type", "application/json")
                .body(Body::from(vec![b' '; MAX_MUTATION_BYTES + 1]))
                .expect("request builds"),
        )
        .await
        .expect_err("oversized cancellation body is rejected");
        assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(oversized.headers()[CACHE_CONTROL], "no-store");
    }
}
