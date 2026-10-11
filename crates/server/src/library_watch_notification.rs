//! Private newest-first Watch inbox for active Instructors.
//!
//! This route has no Watch directory, recipient, email, or Student surface.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    LibraryWatchActivity, LibraryWatchInboxStore, LibraryWatchNotification, LibraryWatchTargetKind,
    SessionStore, SessionTokenHash, StoreError,
    postgres::{PostgresLibraryWatchNotificationStore, PostgresSessionStore},
};
use question_model::UserRole;
use serde::Serialize;

use crate::auth::{AuthError, resolve_session};

const DEFAULT_LIMIT: u16 = 25;
const MAX_LIMIT: u16 = 100;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    inbox: PostgresLibraryWatchNotificationStore,
}

/// Registers one self-only, newest-first in-app Watch inbox route.
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NotificationResponse {
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

async fn read_inbox(State(state): State<RouteState>, request: Request) -> Response {
    // ASVS 8.2.1/8.3.1: conceal authentication and role before parsing the
    // private route's caller-controlled query shape.
    let (session, limit) = match authorized_inbox_limit(
        state.sessions.as_ref(),
        request.headers(),
        request.uri().query(),
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .inbox
        .library_watch_notifications(session, limit)
        .await
    {
        Ok(notifications) => crate::auth::no_store(
            Json(InboxResponse {
                notifications: notifications.into_iter().map(Into::into).collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn authorized_inbox_limit(
    sessions: &dyn SessionStore,
    headers: &HeaderMap,
    raw_query: Option<&str>,
) -> Result<(SessionTokenHash, u16), Box<Response>> {
    let session = instructor_session_hash(sessions, headers).await?;
    let limit = inbox_limit(raw_query)?;
    Ok((session, limit))
}

fn inbox_limit(raw_query: Option<&str>) -> Result<u16, Box<Response>> {
    let Some(raw_query) = raw_query.filter(|query| !query.is_empty()) else {
        return Ok(DEFAULT_LIMIT);
    };
    // ASVS 2.2.1/4.2.1: validate one closed query field, including percent
    // encoding and parameter multiplicity, before using it as a row bound.
    if raw_query.split('&').any(str::is_empty) || !has_valid_percent_encoding(raw_query) {
        return Err(Box::new(invalid()));
    }
    let pairs = url::form_urlencoded::parse(raw_query.as_bytes()).collect::<Vec<_>>();
    let [(name, value)] = pairs.as_slice() else {
        return Err(Box::new(invalid()));
    };
    if name != "limit" {
        return Err(Box::new(invalid()));
    }
    value
        .parse::<u16>()
        .ok()
        .filter(|value| (1..=MAX_LIMIT).contains(value))
        .ok_or_else(|| Box::new(invalid()))
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
            index += 1;
        }
    }
    true
}

async fn instructor_session_hash(
    sessions: &dyn SessionStore,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(sessions, joined_cookie_header(headers).as_deref()).await {
        // ASVS 8.2.1/8.3.1: the trusted SQL boundary repeats this predicate
        // from the installed session; no browser-provided actor can enter it.
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
    // ASVS 14.2.2/16.5.1: the private inbox is never cacheable and public
    // errors expose no recipient, role, target, or database diagnostics.
    crate::auth::no_store((status, message).into_response())
}
