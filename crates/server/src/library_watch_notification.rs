//! Private newest-first Watch inbox for active Instructors.
//!
//! This route has no Watch directory, recipient, email, or Student surface.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    LibraryWatchEventKind, LibraryWatchInboxStore, LibraryWatchNotification,
    LibraryWatchTargetKind, SessionTokenHash, StoreError,
    postgres::{PostgresLibraryWatchNotificationStore, PostgresSessionStore},
};
use question_model::ProductRole;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InboxQuery {
    limit: Option<u16>,
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
    revision_number: Option<u64>,
    forked_public_id: Option<String>,
    activity_id: Option<String>,
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
    Fork,
    ImprovementThread,
    ImpactNotice,
}

impl From<LibraryWatchTargetKind> for TargetKindResponse {
    fn from(value: LibraryWatchTargetKind) -> Self {
        match value {
            LibraryWatchTargetKind::Question => Self::Question,
            LibraryWatchTargetKind::QuestionPool => Self::QuestionPool,
        }
    }
}

impl From<LibraryWatchEventKind> for EventKindResponse {
    fn from(value: LibraryWatchEventKind) -> Self {
        match value {
            LibraryWatchEventKind::Revision => Self::Revision,
            LibraryWatchEventKind::Fork => Self::Fork,
            LibraryWatchEventKind::ImprovementThread => Self::ImprovementThread,
            LibraryWatchEventKind::ImpactNotice => Self::ImpactNotice,
        }
    }
}

impl From<LibraryWatchNotification> for NotificationResponse {
    fn from(value: LibraryWatchNotification) -> Self {
        Self {
            target_kind: value.target_kind.into(),
            target_public_id: value.target_public_id.to_string(),
            event_kind: value.event_kind.into(),
            revision_number: value.revision_number,
            forked_public_id: value
                .forked_public_id
                .map(|question_id| question_id.to_string()),
            activity_id: value.activity_id.map(|activity_id| activity_id.to_string()),
            occurred_at: value.occurred_at.as_unix_millis(),
        }
    }
}

async fn read_inbox(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Query(query): Query<InboxQuery>,
) -> Response {
    let limit = match query.limit {
        Some(value) if value == 0 || value > MAX_LIMIT => return invalid(),
        Some(value) => value,
        None => DEFAULT_LIMIT,
    };
    let session = match instructor_session_hash(&state, &headers).await {
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
        // ASVS 8.2.1/8.3.1: the trusted SQL boundary repeats this predicate
        // from the installed session; no browser-provided actor can enter it.
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
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
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Library Watch inbox changed",
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Library Watch inbox conflict")
        }
        StoreError::AssessmentActivity(_)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_response_uses_canonical_display_question_ids() {
        let question_id: question_model::QuestionId =
            "0000-Q000".parse().expect("canonical Question ID");
        let response = serde_json::to_value(NotificationResponse::from(LibraryWatchNotification {
            target_kind: LibraryWatchTargetKind::Question,
            target_public_id: question_id.clone(),
            event_kind: LibraryWatchEventKind::Fork,
            revision_number: Some(3),
            forked_public_id: Some(question_id),
            activity_id: None,
            occurred_at: question_model::Timestamp::from_unix_millis(1),
        }))
        .expect("Watch response serializes");

        assert_eq!(response["targetPublicId"], "0000-Q000");
        assert_eq!(response["forkedPublicId"], "0000-Q000");
    }
}
