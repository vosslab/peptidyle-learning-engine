//! Instructor command for Topic, Subtopic, and Tags on many Question Pools.
//!
//! Discipline and Subject stay the values established by the first member.
//! The Pool metadata token is checked and advanced once per selected Pool.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    BulkQuestionPoolSearchMetadataInput, BulkQuestionPoolSearchMetadataPatch,
    BulkQuestionPoolSearchMetadataSelection, BulkQuestionPoolSearchMetadataStore, SessionStore,
    SessionTokenHash, StoreError,
};
use question_model::{
    MAX_BULK_QUESTION_METADATA_ITEMS, QuestionPoolId, QuestionPoolMetadataEditNumber, UserRole,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::{AuthError, resolve_session};

const MAX_POOL_SEARCH_METADATA_BYTES: usize = 256 * 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<dyn SessionStore>,
    store: Arc<dyn BulkQuestionPoolSearchMetadataStore>,
}

/// Registers the Question Pool shared search-metadata command.
pub fn question_pool_search_metadata_router(
    sessions: Arc<dyn SessionStore>,
    store: Arc<dyn BulkQuestionPoolSearchMetadataStore>,
) -> Router {
    Router::new()
        .route(
            "/api/question-pools/bulk-search-metadata",
            post(bulk_replace_search_metadata),
        )
        .with_state(RouteState { sessions, store })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SearchMetadataRequest {
    selection: Vec<Value>,
    patch: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchMetadataResponse {
    results: Vec<SearchMetadataResultResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchMetadataResultResponse {
    question_pool_id: String,
    question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber,
}

async fn bulk_replace_search_metadata(
    State(state): State<RouteState>,
    request: Request,
) -> Response {
    // ASVS 2.2.1 and 4.1.1: accept only the documented bounded JSON command shape.
    if !has_json_content_type(request.headers()) {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Question Pool search metadata requires JSON",
        );
    }
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let body = match to_bytes(request.into_body(), MAX_POOL_SEARCH_METADATA_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Question Pool search metadata is too large",
            );
        }
    };
    let request = match serde_json::from_slice::<SearchMetadataRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return route_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Question Pool search metadata is invalid",
            );
        }
    };
    let input = match decode_input(request) {
        Some(input) => input,
        None => {
            return route_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Question Pool search metadata is invalid",
            );
        }
    };
    match state
        .store
        .bulk_replace_question_pool_search_metadata(session, input)
        .await
    {
        Ok(results) => crate::auth::no_store(
            Json(SearchMetadataResponse {
                results: results
                    .into_iter()
                    .map(|result| SearchMetadataResultResponse {
                        question_pool_id: result.question_pool_id.to_string(),
                        question_pool_metadata_edit_number: result
                            .question_pool_metadata_edit_number,
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

fn decode_input(request: SearchMetadataRequest) -> Option<BulkQuestionPoolSearchMetadataInput> {
    if request.selection.is_empty() || request.selection.len() > MAX_BULK_QUESTION_METADATA_ITEMS {
        return None;
    }
    let selection = request
        .selection
        .iter()
        .map(decode_selection_item)
        .collect::<Option<Vec<_>>>()?;
    let patch = decode_patch(request.patch)?;
    let input = BulkQuestionPoolSearchMetadataInput { selection, patch };
    input.validate().ok()?;
    Some(input)
}

fn decode_selection_item(value: &Value) -> Option<BulkQuestionPoolSearchMetadataSelection> {
    let fields = value.as_object()?;
    // ASVS 2.2.1: a Pool row names only its ID and current Edit Number.
    if fields.keys().any(|key| {
        !matches!(
            key.as_str(),
            "questionPoolId" | "questionPoolMetadataEditNumber"
        )
    }) {
        return None;
    }
    let question_pool_id = fields
        .get("questionPoolId")?
        .as_str()?
        .parse::<QuestionPoolId>()
        .ok()?;
    let question_pool_metadata_edit_number = QuestionPoolMetadataEditNumber::new(
        fields.get("questionPoolMetadataEditNumber")?.as_u64()?,
    )
    .ok()?;
    Some(BulkQuestionPoolSearchMetadataSelection {
        question_pool_id,
        question_pool_metadata_edit_number,
    })
}

fn decode_patch(value: Value) -> Option<BulkQuestionPoolSearchMetadataPatch> {
    let fields = value.as_object()?;
    // ASVS 2.2.1/8.2.3: Discipline and Subject are established and are not accepted here.
    if fields.is_empty()
        || fields.len() > 3
        || fields
            .keys()
            .any(|key| !matches!(key.as_str(), "tags" | "topicUuid" | "subtopicUuid"))
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
    let optional_uuid = |name: &str| match fields.get(name) {
        Some(Value::Null) => Some(Some(None)),
        Some(Value::String(value)) => Some(Some(Some(canonical_uuid(value)?))),
        Some(_) => None,
        None => Some(None),
    };
    Some(BulkQuestionPoolSearchMetadataPatch {
        tags,
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
        crate::auth::joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.user_role == UserRole::Instructor => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Pool search metadata authentication is unavailable",
        ))),
    }
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
            "Question Pool search metadata is invalid",
        ),
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Question Pool search metadata changed",
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => route_error(
            StatusCode::CONFLICT,
            "Question Pool search metadata conflicts",
        ),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Pool search metadata is unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(
        StatusCode::NOT_FOUND,
        "Question Pool search metadata not found",
    )
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}
