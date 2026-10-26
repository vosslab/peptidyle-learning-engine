//! Private, queryable Watch activity for the active Instructor.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    LibraryWatchActivity, LibraryWatchDirection, LibraryWatchEventKind, LibraryWatchFacetCount,
    LibraryWatchInboxQuery, LibraryWatchInboxStore, LibraryWatchNotification,
    LibraryWatchTargetKind, NumberedPageRequest, PageNumber, PageSize, SessionStore,
    SessionTokenHash, StoreError,
    postgres::{PostgresLibraryWatchNotificationStore, PostgresSessionStore},
};
use question_model::UserRole;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

const DEFAULT_PAGE_SIZE: u16 = 50;

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
    items: Vec<NotificationResponse>,
    page_number: u32,
    page_size: u16,
    total_count: u64,
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

#[derive(Debug)]
struct ParsedQuery {
    search: String,
    target_kinds: Vec<LibraryWatchTargetKind>,
    event_kinds: Vec<LibraryWatchEventKind>,
    direction: LibraryWatchDirection,
    page: NumberedPageRequest,
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
    let store_query = LibraryWatchInboxQuery {
        search: query.search.clone(),
        target_kinds: query.target_kinds.clone(),
        event_kinds: query.event_kinds.clone(),
        direction: query.direction,
        page: query.page.clone(),
    };
    match state
        .inbox
        .library_watch_notifications(session, store_query)
        .await
    {
        Ok(page) => crate::auth::no_store(
            Json(InboxResponse {
                items: page.page.items.into_iter().map(Into::into).collect(),
                page_number: page.page.page_number.get(),
                page_size: page.page.page_size.get(),
                total_count: page.page.total_count,
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
        page: NumberedPageRequest::first(
            PageSize::new(DEFAULT_PAGE_SIZE).expect("valid default page size"),
        ),
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
            "pageNumber" if seen.insert("pageNumber") => {
                query.page.page_number = value
                    .parse::<u32>()
                    .ok()
                    .and_then(|value| PageNumber::new(value).ok())
                    .ok_or_else(|| Box::new(invalid()))?;
            }
            "pageSize" if seen.insert("pageSize") => {
                query.page.page_size = value
                    .parse::<u16>()
                    .ok()
                    .and_then(|value| PageSize::new(value).ok())
                    .ok_or_else(|| Box::new(invalid()))?;
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
    match resolve_session(
        sessions,
        crate::auth::joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.user_role == UserRole::Instructor => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Library Watch inbox unavailable",
        ))),
    }
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
