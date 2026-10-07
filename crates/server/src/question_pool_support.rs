//! Instructor read and replacement for one Question Pool's optional support texts.
//!
//! The Pool metadata token advances once. Membership and Question identities
//! are not part of the command.

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
    QuestionPoolPleManagedSupport, QuestionPoolSupportStore, SessionStore, SessionTokenHash,
    StoreError,
};
use question_model::{QuestionPoolId, QuestionPoolMetadataEditNumber, UserRole};
use serde::Serialize;
use serde_json::Value;

use crate::auth::{AuthError, resolve_session};

const MAX_POOL_SUPPORT_BYTES: usize = 64 * 1024;
const SUPPORT_KEYS: [&str; 4] = [
    "questionPoolMetadataEditNumber",
    "hint",
    "generalFeedback",
    "workedSolution",
];

#[derive(Clone)]
struct RouteState {
    sessions: Arc<dyn SessionStore>,
    store: Arc<dyn QuestionPoolSupportStore>,
}

/// Registers the Question Pool support read and replacement.
pub fn question_pool_support_router(
    sessions: Arc<dyn SessionStore>,
    store: Arc<dyn QuestionPoolSupportStore>,
) -> Router {
    Router::new()
        .route(
            "/api/question-pools/{question_pool_id}/ple-managed-support",
            get(read_support).put(save_support),
        )
        .with_state(RouteState { sessions, store })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SupportResponse {
    question_pool_id: String,
    question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber,
    hint: Option<String>,
    general_feedback: Option<String>,
    worked_solution: Option<String>,
}

async fn read_support(
    State(state): State<RouteState>,
    Path(question_pool_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let session = match pool_support_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Some(question_pool_id) = parse_pool_id(&question_pool_id) else {
        return invalid();
    };
    match state
        .store
        .read_question_pool_ple_managed_support(session, question_pool_id)
        .await
    {
        Ok(support) => support_response(support),
        Err(error) => store_error_response(error),
    }
}

async fn save_support(
    State(state): State<RouteState>,
    Path(question_pool_id): Path<String>,
    request: Request,
) -> Response {
    // ASVS 2.2.1 and 4.1.1: accept only the documented bounded JSON command.
    if !has_json_content_type(request.headers()) {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Question Pool support requires JSON",
        );
    }
    let session = match pool_support_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Some(question_pool_id) = parse_pool_id(&question_pool_id) else {
        return invalid();
    };
    let body = match to_bytes(request.into_body(), MAX_POOL_SUPPORT_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Question Pool support is too large",
            );
        }
    };
    let Some(support) = decode_replacement(&body, question_pool_id) else {
        return invalid();
    };
    match state
        .store
        .save_question_pool_ple_managed_support(session, support)
        .await
    {
        Ok(saved) => support_response(saved),
        Err(error) => store_error_response(error),
    }
}

fn parse_pool_id(value: &str) -> Option<QuestionPoolId> {
    value.parse::<QuestionPoolId>().ok()
}

fn decode_replacement(
    body: &[u8],
    question_pool_id: QuestionPoolId,
) -> Option<QuestionPoolPleManagedSupport> {
    let value = serde_json::from_slice::<Value>(body).ok()?;
    let fields = value.as_object()?;
    // ASVS 2.2.1 and 8.2.3: the command names the three texts and the Edit Number only.
    if fields.len() != SUPPORT_KEYS.len()
        || fields
            .keys()
            .any(|key| !SUPPORT_KEYS.contains(&key.as_str()))
    {
        return None;
    }
    let question_pool_metadata_edit_number = QuestionPoolMetadataEditNumber::new(
        fields.get("questionPoolMetadataEditNumber")?.as_u64()?,
    )
    .ok()?;
    let support = QuestionPoolPleManagedSupport {
        question_pool_id,
        question_pool_metadata_edit_number,
        hint: optional_support_text(fields.get("hint")?)?,
        general_feedback: optional_support_text(fields.get("generalFeedback")?)?,
        worked_solution: optional_support_text(fields.get("workedSolution")?)?,
    };
    support.validate().ok()?;
    Some(support)
}

fn optional_support_text(value: &Value) -> Option<Option<String>> {
    match value {
        Value::Null => Some(None),
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Some(None);
            }
            // ASVS 2.2.1: stored text is trimmed, at most 4000 characters, and control-free.
            if trimmed.chars().any(char::is_control) || trimmed.chars().count() > 4000 {
                return None;
            }
            Some(Some(trimmed.to_owned()))
        }
        _ => None,
    }
}

fn support_response(support: QuestionPoolPleManagedSupport) -> Response {
    crate::auth::no_store(
        Json(SupportResponse {
            question_pool_id: support.question_pool_id.to_string(),
            question_pool_metadata_edit_number: support.question_pool_metadata_edit_number,
            hint: support.hint,
            general_feedback: support.general_feedback,
            worked_solution: support.worked_solution,
        })
        .into_response(),
    )
}

async fn pool_support_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    // ASVS 8.2.1: Pool support is shared with Instructors and available to Sysadmins.
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
            "Question Pool support authentication is unavailable",
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
        StoreError::InvalidRecord(_) => invalid(),
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Question Pool support changed",
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Question Pool support conflicts")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Pool support is unavailable",
        ),
    }
}

fn invalid() -> Response {
    route_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Question Pool support is invalid",
    )
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Pool support not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use learning_data_access::{SessionId, SessionRecord, SessionTokenHash};
    use question_model::{AccountId, QuestionPoolMetadataEditNumber, Timestamp};
    use tower::ServiceExt;

    use super::*;

    struct RoleSession {
        role: UserRole,
    }

    #[async_trait]
    impl SessionStore for RoleSession {
        async fn create_session(
            &self,
            _token_hash: SessionTokenHash,
            _account: AccountId,
            _lifetime: learning_data_access::SessionLifetime,
        ) -> Result<SessionRecord, StoreError> {
            unreachable!("pool support does not create a session")
        }

        async fn resolve_session(
            &self,
            token_hash: SessionTokenHash,
        ) -> Result<Option<SessionRecord>, StoreError> {
            Ok(Some(SessionRecord {
                id: SessionId::generate()?,
                token_hash,
                account: AccountId::from_debug_serial(2),
                user_role: self.role,
                created_at: Timestamp::from_unix_millis(0),
                expires_at: Timestamp::from_unix_millis(60_000),
            }))
        }

        async fn revoke_session(&self, _token_hash: SessionTokenHash) -> Result<(), StoreError> {
            unreachable!("pool support does not revoke a session")
        }
    }

    struct RecordingStore {
        saved: Mutex<Option<QuestionPoolPleManagedSupport>>,
        saves: Mutex<usize>,
        reads: Mutex<usize>,
    }

    #[async_trait]
    impl QuestionPoolSupportStore for RecordingStore {
        async fn read_question_pool_ple_managed_support(
            &self,
            _session_token_hash: SessionTokenHash,
            question_pool_id: QuestionPoolId,
        ) -> Result<QuestionPoolPleManagedSupport, StoreError> {
            *self.reads.lock().expect("reads") += 1;
            Ok(self
                .saved
                .lock()
                .expect("saved")
                .clone()
                .unwrap_or(QuestionPoolPleManagedSupport {
                    question_pool_id,
                    question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber::new(4)
                        .expect("metadata token"),
                    hint: None,
                    general_feedback: None,
                    worked_solution: None,
                }))
        }

        async fn save_question_pool_ple_managed_support(
            &self,
            _session_token_hash: SessionTokenHash,
            mut support: QuestionPoolPleManagedSupport,
        ) -> Result<QuestionPoolPleManagedSupport, StoreError> {
            *self.saves.lock().expect("saves") += 1;
            support.question_pool_metadata_edit_number = QuestionPoolMetadataEditNumber::new(
                support.question_pool_metadata_edit_number.get() + 1,
            )
            .expect("next metadata token");
            *self.saved.lock().expect("saved") = Some(support.clone());
            Ok(support)
        }
    }

    fn pool_id() -> String {
        QuestionPoolId::from_random_identifier("0000001")
            .expect("Pool ID")
            .to_string()
    }

    fn session_cookie() -> String {
        format!("ple_session={}", URL_SAFE_NO_PAD.encode([7_u8; 32]))
    }

    fn router(role: UserRole, store: Arc<RecordingStore>) -> Router {
        question_pool_support_router(Arc::new(RoleSession { role }), store)
    }

    fn support_uri(id: &str) -> String {
        format!("/api/question-pools/{id}/ple-managed-support")
    }

    async fn send(app: Router, method: &str, id: &str, body: Option<String>) -> Response {
        let request = Request::builder()
            .method(method)
            .uri(support_uri(id))
            .header("cookie", session_cookie());
        let request = if let Some(body) = body {
            request
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap()
        } else {
            request.body(Body::empty()).unwrap()
        };
        app.oneshot(request).await.unwrap()
    }

    #[tokio::test]
    async fn pools_keep_optional_ple_managed_support_without_copying_member_questions() {
        let id = pool_id();
        let store = Arc::new(RecordingStore {
            saved: Mutex::new(None),
            saves: Mutex::new(0),
            reads: Mutex::new(0),
        });
        let response = send(
            router(UserRole::Instructor, Arc::clone(&store)),
            "GET",
            &id,
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let payload = json_body(response).await;
        assert_eq!(payload["questionPoolId"], id);
        assert_eq!(payload["questionPoolMetadataEditNumber"], 4);
        assert!(payload["hint"].is_null());
        assert!(payload["generalFeedback"].is_null());
        assert!(payload["workedSolution"].is_null());
        assert_eq!(*store.reads.lock().expect("reads"), 1);

        let response = send(
            router(UserRole::Instructor, Arc::clone(&store)),
            "PUT",
            &id,
            Some(
                serde_json::json!({
                    "questionPoolMetadataEditNumber": 4,
                    "hint": "  Look at the units.  ",
                    "generalFeedback": "Keep the units with the answer.",
                    "workedSolution": "Shown work."
                })
                .to_string(),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let payload = json_body(response).await;
        assert_eq!(payload["questionPoolMetadataEditNumber"], 5);
        assert_eq!(payload["hint"], "Look at the units.");
        assert_eq!(
            payload["generalFeedback"],
            "Keep the units with the answer."
        );
        assert_eq!(payload["workedSolution"], "Shown work.");
        let saved = store.saved.lock().expect("saved").clone().expect("save");
        assert_eq!(saved.question_pool_id.to_string(), id);
        assert_eq!(saved.question_pool_metadata_edit_number.get(), 5);
        assert_eq!(saved.hint.as_deref(), Some("Look at the units."));
        assert_eq!(
            saved.general_feedback.as_deref(),
            Some("Keep the units with the answer.")
        );
        assert_eq!(saved.worked_solution.as_deref(), Some("Shown work."));
        assert_eq!(*store.saves.lock().expect("saves"), 1);

        let response = send(
            router(UserRole::Instructor, Arc::clone(&store)),
            "PUT",
            &id,
            Some(
                serde_json::json!({
                    "questionPoolMetadataEditNumber": 4,
                    "hint": "",
                    "generalFeedback": "  ",
                    "workedSolution": "\n"
                })
                .to_string(),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let cleared = store.saved.lock().expect("saved").clone().expect("cleared");
        assert_eq!(cleared.question_pool_metadata_edit_number.get(), 5);
        assert!(cleared.hint.is_none());
        assert!(cleared.general_feedback.is_none());
        assert!(cleared.worked_solution.is_none());

        let response = send(
            router(UserRole::Instructor, Arc::clone(&store)),
            "PUT",
            &id,
            Some(
                serde_json::json!({
                    "questionPoolMetadataEditNumber": 4,
                    "generalFeedback": "Keep the units.",
                    "workedSolution": null
                })
                .to_string(),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let response = send(
            router(UserRole::Instructor, Arc::clone(&store)),
            "PUT",
            &id,
            Some(
                serde_json::json!({
                    "questionPoolMetadataEditNumber": 4,
                    "hint": null,
                    "generalFeedback": null,
                    "workedSolution": null,
                    "publishedQuestionId": "3S8B-24DZ"
                })
                .to_string(),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(*store.saves.lock().expect("saves"), 2);

        let denied = Arc::new(RecordingStore {
            saved: Mutex::new(None),
            saves: Mutex::new(0),
            reads: Mutex::new(0),
        });
        let response = send(
            router(UserRole::Student, Arc::clone(&denied)),
            "GET",
            &id,
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = send(
            router(UserRole::Student, Arc::clone(&denied)),
            "PUT",
            &id,
            Some(
                serde_json::json!({
                    "questionPoolMetadataEditNumber": 4,
                    "hint": "Look at the units.",
                    "generalFeedback": null,
                    "workedSolution": null
                })
                .to_string(),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(*denied.reads.lock().expect("reads"), 0);
        assert_eq!(*denied.saves.lock().expect("saves"), 0);
    }

    async fn json_body(response: Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }
}
