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

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use axum::http::header::CACHE_CONTROL;
    use learning_data_access::{SessionLifetime, SessionRecord};
    use question_model::AccountId;
    use serde_json::{Value, json};

    use super::*;

    struct UnusedSessionStore;

    #[async_trait]
    impl SessionStore for UnusedSessionStore {
        async fn create_session(
            &self,
            _token_hash: SessionTokenHash,
            _account: AccountId,
            _lifetime: SessionLifetime,
        ) -> Result<SessionRecord, StoreError> {
            unreachable!("an anonymous request must not reach session storage")
        }

        async fn resolve_session(
            &self,
            _token_hash: SessionTokenHash,
        ) -> Result<Option<SessionRecord>, StoreError> {
            unreachable!("an anonymous request must not reach session storage")
        }

        async fn revoke_session(&self, _token_hash: SessionTokenHash) -> Result<(), StoreError> {
            unreachable!("the inbox never revokes a session")
        }
    }

    fn notification(activity: LibraryWatchActivity) -> LibraryWatchNotification {
        let public_id = question_model::LibraryObjectId::from_random_identifier("0000000")
            .expect("Library Object ID");
        let target_kind = if matches!(
            activity,
            LibraryWatchActivity::MembersChanged { .. }
                | LibraryWatchActivity::QuestionPoolFork { .. }
        ) {
            LibraryWatchTargetKind::QuestionPool
        } else {
            LibraryWatchTargetKind::Question
        };
        LibraryWatchNotification {
            target_kind,
            target_public_id: public_id,
            activity,
            occurred_at: question_model::Timestamp::from_unix_millis(1),
        }
    }

    #[test]
    fn every_watch_activity_serializes_target_specific_number_fields() {
        let forked_id = question_model::LibraryObjectId::from_random_identifier("0000001")
            .expect("Library Object ID");
        let cases = [
            (
                LibraryWatchActivity::Revision {
                    question_revision_number: question_model::QuestionRevisionNumber::new(2)
                        .expect("revision"),
                },
                "revision",
                "question",
                json!(2),
                Value::Null,
                None,
            ),
            (
                LibraryWatchActivity::MembersChanged {
                    question_pool_edit_number: question_model::QuestionPoolEditNumber::new(5)
                        .expect("Pool Edit Number"),
                },
                "membersChanged",
                "questionPool",
                Value::Null,
                json!(5),
                None,
            ),
            (
                LibraryWatchActivity::QuestionFork {
                    source_question_revision_number: question_model::QuestionRevisionNumber::new(3)
                        .expect("revision"),
                    forked_public_id: forked_id.clone(),
                },
                "fork",
                "question",
                json!(3),
                Value::Null,
                Some(forked_id.to_string()),
            ),
            (
                LibraryWatchActivity::QuestionPoolFork {
                    source_question_pool_edit_number: question_model::QuestionPoolEditNumber::new(
                        7,
                    )
                    .expect("Pool Edit Number"),
                    forked_public_id: forked_id.clone(),
                },
                "fork",
                "questionPool",
                Value::Null,
                json!(7),
                Some(forked_id.to_string()),
            ),
        ];

        for (activity, event_kind, target_kind, question_revision, pool_edit, forked) in cases {
            let response = serde_json::to_value(NotificationResponse::from(notification(activity)))
                .expect("Watch response serializes");
            assert_eq!(response["eventKind"], event_kind);
            assert_eq!(response["targetKind"], target_kind);
            assert_eq!(response["questionRevisionNumber"], question_revision);
            assert_eq!(response["questionPoolEditNumber"], pool_edit);
            assert_eq!(response["forkedPublicId"], option_string(forked));
            assert_eq!(
                response
                    .as_object()
                    .expect("notification response is an object")
                    .keys()
                    .map(String::as_str)
                    .collect::<std::collections::BTreeSet<_>>(),
                [
                    "eventKind",
                    "forkedPublicId",
                    "occurredAt",
                    "questionPoolEditNumber",
                    "questionRevisionNumber",
                    "targetKind",
                    "targetPublicId",
                ]
                .into_iter()
                .collect()
            );
        }
    }

    fn option_string(value: Option<String>) -> Value {
        value.map_or(Value::Null, Value::String)
    }

    #[test]
    fn notification_response_serializes_a_pool_fork_with_library_object_ids() {
        let source_pool_id = question_model::LibraryObjectId::from_random_identifier("0000000")
            .expect("source Pool ID");
        let forked_pool_id = question_model::LibraryObjectId::from_random_identifier("0000001")
            .expect("forked Pool ID");
        let response = serde_json::to_value(NotificationResponse::from(LibraryWatchNotification {
            target_kind: LibraryWatchTargetKind::QuestionPool,
            target_public_id: source_pool_id.clone(),
            activity: LibraryWatchActivity::QuestionPoolFork {
                source_question_pool_edit_number: question_model::QuestionPoolEditNumber::new(3)
                    .expect("Pool Edit Number"),
                forked_public_id: forked_pool_id.clone(),
            },
            occurred_at: question_model::Timestamp::from_unix_millis(1),
        }))
        .expect("Watch response serializes");

        assert_eq!(response["targetKind"], "questionPool");
        assert_eq!(response["targetPublicId"], source_pool_id.to_string());
        assert_eq!(response["forkedPublicId"], forked_pool_id.to_string());
    }

    #[test]
    fn inbox_store_errors_follow_the_private_status_contract() {
        let cases = [
            (StoreError::NotFound, StatusCode::NOT_FOUND),
            (StoreError::Forbidden, StatusCode::NOT_FOUND),
            (StoreError::OwnershipMismatch, StatusCode::NOT_FOUND),
            (
                StoreError::InvalidRecord("invalid".to_owned()),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
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

    #[test]
    fn inbox_query_accepts_only_one_bounded_limit() {
        assert_eq!(inbox_limit(None).expect("missing query uses default"), 25);
        assert_eq!(inbox_limit(Some("")).expect("empty query uses default"), 25);
        assert_eq!(inbox_limit(Some("limit=1")).expect("lower bound"), 1);
        assert_eq!(inbox_limit(Some("limit=100")).expect("upper bound"), 100);

        for query in [
            "limit=0",
            "limit=101",
            "limit=65536",
            "limit=abc",
            "limit=",
            "limit",
            "other=25",
            "limit=1&limit=2",
            "limit=1&other=2",
            "limit=25&",
            "&limit=25",
            "limit=%",
            "limit=%ZZ",
        ] {
            let response = inbox_limit(Some(query)).expect_err("query must be rejected");
            assert_eq!(
                response.status(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "{query}"
            );
            assert_eq!(response.headers()[CACHE_CONTROL], "no-store", "{query}");
        }
    }

    #[tokio::test]
    async fn anonymous_malformed_query_is_concealed_before_query_validation() {
        let response = authorized_inbox_limit(
            &UnusedSessionStore,
            &HeaderMap::new(),
            Some("limit=1&limit=2"),
        )
        .await
        .expect_err("anonymous inbox is concealed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    }
}
