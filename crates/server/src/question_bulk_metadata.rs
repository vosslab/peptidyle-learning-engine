//! Active-Instructor bulk shared-metadata command.
//!
//! The browser can replace or clear only shared search metadata on a bounded
//! selection of Published Questions.  It cannot carry source, Revision,
//! availability, ownership, backend, or arbitrary JSON mutations.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    BulkPublishedQuestionMetadataInput, BulkPublishedQuestionMetadataPatch,
    BulkPublishedQuestionMetadataSelection, BulkPublishedQuestionMetadataStore, SessionStore,
    SessionTokenHash, StoreError,
};
use question_model::{MAX_BULK_QUESTION_METADATA_ITEMS, PublishedQuestionId, UserRole};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::{AuthError, resolve_session};

const MAX_BULK_QUESTION_METADATA_BYTES: usize = 256 * 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<dyn SessionStore>,
    store: Arc<dyn BulkPublishedQuestionMetadataStore>,
}

/// Registers the sole browser command for atomic shared Question metadata edits.
pub fn question_bulk_metadata_router(
    sessions: Arc<dyn SessionStore>,
    store: Arc<dyn BulkPublishedQuestionMetadataStore>,
) -> Router {
    Router::new()
        .route("/api/questions/bulk-metadata", post(bulk_replace_metadata))
        .with_state(RouteState { sessions, store })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BulkMetadataRequest {
    selection: Vec<Value>,
    patch: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BulkMetadataResponse {
    results: Vec<BulkMetadataResultResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BulkMetadataResultResponse {
    question_id: String,
    metadata_edit_number: u64,
}

async fn bulk_replace_metadata(State(state): State<RouteState>, request: Request) -> Response {
    // ASVS 2.2.1 and 4.1.1: accept only the documented bounded JSON command shape.
    if !has_json_content_type(request.headers()) {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Bulk Question metadata requires JSON",
        );
    }
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let body = match to_bytes(request.into_body(), MAX_BULK_QUESTION_METADATA_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Bulk Question metadata is too large",
            );
        }
    };
    let request = match serde_json::from_slice::<BulkMetadataRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return route_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Bulk Question metadata is invalid",
            );
        }
    };
    let input = match decode_input(request) {
        Some(input) => input,
        None => {
            return route_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Bulk Question metadata is invalid",
            );
        }
    };
    match state
        .store
        .bulk_replace_published_question_metadata(session, input)
        .await
    {
        Ok(results) => crate::auth::no_store(
            Json(BulkMetadataResponse {
                results: results
                    .into_iter()
                    .map(|result| BulkMetadataResultResponse {
                        question_id: result.question_id.to_string(),
                        metadata_edit_number: result.metadata_edit_number,
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

fn decode_input(request: BulkMetadataRequest) -> Option<BulkPublishedQuestionMetadataInput> {
    if request.selection.is_empty() || request.selection.len() > MAX_BULK_QUESTION_METADATA_ITEMS {
        return None;
    }
    let selection = request
        .selection
        .iter()
        .map(decode_selection_item)
        .collect::<Option<Vec<_>>>()?;
    let patch = decode_patch(request.patch)?;
    let input = BulkPublishedQuestionMetadataInput { selection, patch };
    input.validate().ok()?;
    Some(input)
}

fn decode_selection_item(value: &Value) -> Option<BulkPublishedQuestionMetadataSelection> {
    let fields = value.as_object()?;
    // ASVS 1.5.2/2.2.1: Title and Description are optional per Question; null is not a clear.
    if fields.keys().any(|key| {
        !matches!(
            key.as_str(),
            "questionId" | "metadataEditNumber" | "questionTitle" | "questionDescription"
        )
    }) {
        return None;
    }
    let question_id = fields
        .get("questionId")?
        .as_str()?
        .parse::<PublishedQuestionId>()
        .ok()?;
    let metadata_edit_number = fields.get("metadataEditNumber")?.as_u64()?;
    Some(BulkPublishedQuestionMetadataSelection {
        question_id,
        metadata_edit_number,
        question_title: optional_search_text(fields, "questionTitle")?,
        question_description: optional_search_text(fields, "questionDescription")?,
    })
}

fn optional_search_text(
    fields: &serde_json::Map<String, Value>,
    name: &str,
) -> Option<Option<String>> {
    match fields.get(name) {
        None => Some(None),
        Some(Value::String(value)) => Some(Some(value.clone())),
        Some(_) => None,
    }
}

fn decode_patch(value: Value) -> Option<BulkPublishedQuestionMetadataPatch> {
    let fields = value.as_object()?;
    if fields.len() > 5
        || fields.keys().any(|key| {
            !matches!(
                key.as_str(),
                "tags" | "disciplineUuid" | "subjectUuid" | "topicUuid" | "subtopicUuid"
            )
        })
    {
        return None;
    }
    let tags = match fields.get("tags") {
        Some(Value::Array(values)) => Some(
            values
                .iter()
                .map(|value| value.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?,
        ),
        Some(_) => return None,
        None => None,
    };
    let canonical_uuid = |value: &str| {
        let uuid = uuid::Uuid::parse_str(value).ok()?;
        (uuid.to_string() == value).then_some(uuid)
    };
    // ASVS 2.2.1/2.2.2: allow only canonical identifiers; required parents cannot clear.
    let required_uuid = |name: &str| match fields.get(name) {
        Some(Value::String(value)) => Some(Some(canonical_uuid(value)?)),
        Some(_) => None,
        None => Some(None),
    };
    let optional_uuid = |name: &str| match fields.get(name) {
        Some(Value::Null) => Some(Some(None)),
        Some(Value::String(value)) => Some(Some(Some(canonical_uuid(value)?))),
        Some(_) => None,
        None => Some(None),
    };
    Some(BulkPublishedQuestionMetadataPatch {
        tags,
        discipline_uuid: required_uuid("disciplineUuid")?,
        subject_uuid: required_uuid("subjectUuid")?,
        topic_uuid: optional_uuid("topicUuid")?,
        subtopic_uuid: optional_uuid("subtopicUuid")?,
    })
}

async fn instructor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    // ASVS 8.2.1: the trusted service boundary permits only an Instructor session.
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.user_role == UserRole::Instructor => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Bulk Question metadata authentication is unavailable",
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
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Bulk Question metadata is invalid",
        ),
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Bulk Question metadata changed",
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Bulk Question metadata conflicts")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Bulk Question metadata is unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Bulk Question metadata not found")
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
    use learning_data_access::{
        BulkPublishedQuestionMetadataResult, SessionId, SessionRecord, SessionTokenHash,
    };
    use question_model::{AccountId, Timestamp};
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
            unreachable!("bulk metadata does not create a session")
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
            unreachable!("bulk metadata does not revoke a session")
        }
    }

    struct RecordingStore {
        calls: Mutex<Vec<Vec<String>>>,
    }

    #[async_trait]
    impl BulkPublishedQuestionMetadataStore for RecordingStore {
        async fn bulk_replace_published_question_metadata(
            &self,
            _session_token_hash: SessionTokenHash,
            input: BulkPublishedQuestionMetadataInput,
        ) -> Result<Vec<BulkPublishedQuestionMetadataResult>, StoreError> {
            let ids = input
                .selection
                .iter()
                .map(|item| item.question_id.to_string())
                .collect::<Vec<_>>();
            self.calls.lock().expect("bulk calls").push(ids);
            Ok(input
                .selection
                .into_iter()
                .map(|item| BulkPublishedQuestionMetadataResult {
                    question_id: item.question_id,
                    metadata_edit_number: item.metadata_edit_number + 1,
                })
                .collect())
        }
    }

    fn question_id(index: usize) -> String {
        PublishedQuestionId::from_random_identifier(format!("{index:07}"))
            .expect("Question ID")
            .to_string()
    }

    fn session_cookie() -> String {
        format!("ple_session={}", URL_SAFE_NO_PAD.encode([7_u8; 32]))
    }

    fn command(selection: serde_json::Value) -> String {
        serde_json::json!({
            "selection": selection,
            "patch": { "tags": ["review"] }
        })
        .to_string()
    }

    fn router(role: UserRole, store: Arc<RecordingStore>) -> Router {
        question_bulk_metadata_router(Arc::new(RoleSession { role }), store)
    }

    async fn post(app: Router, body: String) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/questions/bulk-metadata")
                .header("content-type", "application/json")
                .header("cookie", session_cookie())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn question_library_workflows_support_bulk_operations_for_many_questions() {
        let first = question_id(1);
        let second = question_id(2);
        let store = Arc::new(RecordingStore {
            calls: Mutex::new(Vec::new()),
        });
        let response = post(
            router(UserRole::Instructor, Arc::clone(&store)),
            command(serde_json::json!([
                { "questionId": first, "metadataEditNumber": 4 },
                { "questionId": second, "metadataEditNumber": 9 }
            ])),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload["results"]
                .as_array()
                .unwrap()
                .iter()
                .map(|result| result["questionId"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec![first.as_str(), second.as_str()]
        );
        assert_eq!(
            store.calls.lock().expect("bulk calls").as_slice(),
            &[vec![first.clone(), second.clone()]]
        );

        // ASVS 8.2.1: a Student session cannot run the Instructor bulk command.
        let denied = Arc::new(RecordingStore {
            calls: Mutex::new(Vec::new()),
        });
        let response = post(
            router(UserRole::Student, Arc::clone(&denied)),
            command(serde_json::json!([
                { "questionId": first, "metadataEditNumber": 4 }
            ])),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(denied.calls.lock().expect("denied calls").is_empty());

        let overflow = (0..=MAX_BULK_QUESTION_METADATA_ITEMS)
            .map(|index| {
                serde_json::json!({
                    "questionId": question_id(index),
                    "metadataEditNumber": 1
                })
            })
            .collect::<Vec<_>>();
        let before = store.calls.lock().expect("bulk calls").len();
        let response = post(
            router(UserRole::Instructor, Arc::clone(&store)),
            command(serde_json::Value::Array(overflow)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(store.calls.lock().expect("bulk calls").len(), before);
        assert_eq!(MAX_BULK_QUESTION_METADATA_ITEMS, 1000);
    }
}
