//! Private, queryable Watch activity for the active Instructor.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    LibraryWatchActivity, LibraryWatchCursorPosition, LibraryWatchDirection, LibraryWatchEventKind,
    LibraryWatchFacetCount, LibraryWatchInboxQuery, LibraryWatchInboxStore,
    LibraryWatchNotification, LibraryWatchTargetKind, SessionStore, SessionTokenHash, StoreError,
    postgres::{PostgresLibraryWatchNotificationStore, PostgresSessionStore},
};
use question_model::UserRole;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

const DEFAULT_LIMIT: u16 = 25;
const MAX_LIMIT: u16 = 250;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    inbox: PostgresLibraryWatchNotificationStore,
}

pub fn library_watch_notification_router(
    sessions: Arc<PostgresSessionStore>,
    inbox: PostgresLibraryWatchNotificationStore,
) -> Router {
    Router::new()
        .route("/api/library/watch-notifications", get(read_inbox))
        .with_state(RouteState { sessions, inbox })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InboxResponse {
    notifications: Vec<NotificationResponse>,
    next_cursor: Option<String>,
    target_kind_facets: Vec<FacetResponse>,
    event_kind_facets: Vec<FacetResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FacetResponse {
    value: String,
    count: u64,
}

impl From<LibraryWatchFacetCount> for FacetResponse {
    fn from(value: LibraryWatchFacetCount) -> Self {
        Self {
            value: match value.value.as_str() {
                "question_pool" => "questionPool".to_owned(),
                "members_changed" => "membersChanged".to_owned(),
                _ => value.value,
            },
            count: value.count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NotificationResponse {
    event_id: Uuid,
    target_kind: TargetKindResponse,
    target_public_id: String,
    event_kind: EventKindResponse,
    question_revision_number: Option<u32>,
    question_pool_edit_number: Option<u64>,
    forked_public_id: Option<String>,
    occurred_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum TargetKindResponse {
    Question,
    QuestionPool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum EventKindResponse {
    Revision,
    MembersChanged,
    Fork,
}

impl From<LibraryWatchTargetKind> for TargetKindResponse {
    fn from(value: LibraryWatchTargetKind) -> Self {
        match value {
            LibraryWatchTargetKind::Question => Self::Question,
            LibraryWatchTargetKind::QuestionPool => Self::QuestionPool,
        }
    }
}

impl From<LibraryWatchNotification> for NotificationResponse {
    fn from(value: LibraryWatchNotification) -> Self {
        let (event_kind, question_revision_number, question_pool_edit_number, forked_public_id) =
            match value.activity {
                LibraryWatchActivity::Revision {
                    question_revision_number,
                } => (
                    EventKindResponse::Revision,
                    Some(question_revision_number.get()),
                    None,
                    None,
                ),
                LibraryWatchActivity::MembersChanged {
                    question_pool_edit_number,
                } => (
                    EventKindResponse::MembersChanged,
                    None,
                    Some(question_pool_edit_number.get()),
                    None,
                ),
                LibraryWatchActivity::QuestionFork {
                    source_question_revision_number,
                    forked_public_id,
                } => (
                    EventKindResponse::Fork,
                    Some(source_question_revision_number.get()),
                    None,
                    Some(forked_public_id.to_string()),
                ),
                LibraryWatchActivity::QuestionPoolFork {
                    source_question_pool_edit_number,
                    forked_public_id,
                } => (
                    EventKindResponse::Fork,
                    None,
                    Some(source_question_pool_edit_number.get()),
                    Some(forked_public_id.to_string()),
                ),
            };
        Self {
            event_id: value.event_id,
            target_kind: value.target_kind.into(),
            target_public_id: value.target_public_id.to_string(),
            event_kind,
            question_revision_number,
            question_pool_edit_number,
            forked_public_id,
            occurred_at: value.occurred_at.as_unix_millis(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WatchCursor {
    version: u8,
    query_digest: [u8; 32],
    occurred_at: String,
    event_id: Uuid,
}

#[derive(Debug)]
struct ParsedQuery {
    search: String,
    target_kinds: Vec<LibraryWatchTargetKind>,
    event_kinds: Vec<LibraryWatchEventKind>,
    direction: LibraryWatchDirection,
    cursor: Option<String>,
    limit: u16,
}

fn query_digest(query: &ParsedQuery, session: SessionTokenHash) -> [u8; 32] {
    let keys = (
        &query.search,
        query
            .target_kinds
            .iter()
            .map(|kind| match kind {
                LibraryWatchTargetKind::Question => "question",
                LibraryWatchTargetKind::QuestionPool => "questionPool",
            })
            .collect::<Vec<_>>(),
        query
            .event_kinds
            .iter()
            .map(|kind| match kind {
                LibraryWatchEventKind::Revision => "revision",
                LibraryWatchEventKind::MembersChanged => "membersChanged",
                LibraryWatchEventKind::Fork => "fork",
            })
            .collect::<Vec<_>>(),
        query.direction == LibraryWatchDirection::Ascending,
        query.limit,
        session.to_string(),
    );
    Sha256::digest(serde_json::to_vec(&keys).expect("Watch query serializes")).into()
}

fn cursor_position(
    query: &ParsedQuery,
    session: SessionTokenHash,
) -> Result<Option<LibraryWatchCursorPosition>, Box<Response>> {
    let Some(token) = query.cursor.as_deref() else {
        return Ok(None);
    };
    if token.len() > 1024 {
        return Err(Box::new(invalid()));
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| Box::new(invalid()))?;
    if bytes.len() > 768 || URL_SAFE_NO_PAD.encode(&bytes) != token {
        return Err(Box::new(invalid()));
    }
    let cursor: WatchCursor = serde_json::from_slice(&bytes).map_err(|_| Box::new(invalid()))?;
    let valid_time = chrono::DateTime::parse_from_rfc3339(&cursor.occurred_at)
        .is_ok_and(|at| at.format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string() == cursor.occurred_at);
    if cursor.version != 1 || cursor.query_digest != query_digest(query, session) || !valid_time {
        return Err(Box::new(invalid()));
    }
    Ok(Some(LibraryWatchCursorPosition {
        occurred_at: cursor.occurred_at,
        event_id: cursor.event_id,
    }))
}

fn encode_cursor(
    query: &ParsedQuery,
    session: SessionTokenHash,
    position: LibraryWatchCursorPosition,
) -> String {
    let cursor = WatchCursor {
        version: 1,
        query_digest: query_digest(query, session),
        occurred_at: position.occurred_at,
        event_id: position.event_id,
    };
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).expect("Watch cursor serializes"))
}

async fn read_inbox(State(state): State<RouteState>, request: Request) -> Response {
    // Conceal this private route before parsing caller-controlled query data.
    let session = match instructor_session_hash(state.sessions.as_ref(), request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let query = match parse_query(request.uri().query()) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let after = match cursor_position(&query, session) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let store_query = LibraryWatchInboxQuery {
        search: query.search.clone(),
        target_kinds: query.target_kinds.clone(),
        event_kinds: query.event_kinds.clone(),
        direction: query.direction,
        after,
        limit: query.limit,
    };
    match state
        .inbox
        .library_watch_notifications(session, store_query)
        .await
    {
        Ok(page) => crate::auth::no_store(
            Json(InboxResponse {
                notifications: page.notifications.into_iter().map(Into::into).collect(),
                next_cursor: page
                    .next_position
                    .map(|position| encode_cursor(&query, session, position)),
                target_kind_facets: page
                    .target_kind_facets
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                event_kind_facets: page.event_kind_facets.into_iter().map(Into::into).collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

fn parse_query(raw_query: Option<&str>) -> Result<ParsedQuery, Box<Response>> {
    let mut query = ParsedQuery {
        search: String::new(),
        target_kinds: Vec::new(),
        event_kinds: Vec::new(),
        direction: LibraryWatchDirection::Descending,
        cursor: None,
        limit: DEFAULT_LIMIT,
    };
    let Some(raw) = raw_query.filter(|value| !value.is_empty()) else {
        return Ok(query);
    };
    if raw.len() > 4096 || raw.split('&').any(str::is_empty) || !has_valid_percent_encoding(raw) {
        return Err(Box::new(invalid()));
    }
    let mut seen = std::collections::HashSet::new();
    for (name, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        match name.as_ref() {
            "q" if seen.insert("q") => {
                query.search = value.trim().to_lowercase();
                if query.search.chars().count() > 120 || query.search.chars().any(char::is_control)
                {
                    return Err(Box::new(invalid()));
                }
            }
            "targetKind" => query.target_kinds.push(match value.as_ref() {
                "question" => LibraryWatchTargetKind::Question,
                "questionPool" => LibraryWatchTargetKind::QuestionPool,
                _ => return Err(Box::new(invalid())),
            }),
            "eventKind" => query.event_kinds.push(match value.as_ref() {
                "revision" => LibraryWatchEventKind::Revision,
                "membersChanged" => LibraryWatchEventKind::MembersChanged,
                "fork" => LibraryWatchEventKind::Fork,
                _ => return Err(Box::new(invalid())),
            }),
            "direction" if seen.insert("direction") => {
                query.direction = match value.as_ref() {
                    "ascending" => LibraryWatchDirection::Ascending,
                    "descending" => LibraryWatchDirection::Descending,
                    _ => return Err(Box::new(invalid())),
                }
            }
            "cursor" if seen.insert("cursor") && !value.is_empty() => {
                query.cursor = Some(value.into_owned())
            }
            "limit" if seen.insert("limit") => {
                query.limit = value
                    .parse::<u16>()
                    .ok()
                    .filter(|value| (1..=MAX_LIMIT).contains(value))
                    .ok_or_else(|| Box::new(invalid()))?
            }
            _ => return Err(Box::new(invalid())),
        }
    }
    query.target_kinds.sort_by_key(|kind| match kind {
        LibraryWatchTargetKind::Question => 0,
        LibraryWatchTargetKind::QuestionPool => 1,
    });
    query.target_kinds.dedup();
    query.event_kinds.sort_by_key(|kind| match kind {
        LibraryWatchEventKind::Revision => 0,
        LibraryWatchEventKind::MembersChanged => 1,
        LibraryWatchEventKind::Fork => 2,
    });
    query.event_kinds.dedup();
    Ok(query)
}

fn has_valid_percent_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if bytes
                .get(index + 1..index + 3)
                .is_none_or(|pair| !pair.iter().all(u8::is_ascii_hexdigit))
            {
                return false;
            }
            index += 3;
        } else {
            index += 1
        }
    }
    true
}

async fn instructor_session_hash(
    sessions: &dyn SessionStore,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(sessions, joined_cookie_header(headers).as_deref()).await {
        Ok(session) if session.record.user_role == UserRole::Instructor => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Library Watch inbox unavailable",
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
        StoreError::InvalidRecord(_) => invalid(),
        StoreError::Conflict => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Library Watch inbox changed",
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Library Watch inbox conflict")
        }
        StoreError::RetryableTransaction
        | StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Library Watch inbox unavailable",
        ),
    }
}

fn invalid() -> Response {
    route_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Library Watch inbox is invalid",
    )
}
fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Library Watch inbox not found")
}
fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
