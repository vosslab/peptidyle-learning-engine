//! Active-vetted-Instructor bulk shared-metadata command.
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
    BulkPublishedQuestionMetadataSelection, BulkPublishedQuestionMetadataStore, SessionTokenHash,
    StoreError,
    postgres::{PostgresBulkPublishedQuestionMetadataStore, PostgresSessionStore},
};
use question_model::{MAX_BULK_QUESTION_METADATA_ITEMS, ProductRole, QuestionId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::HmacQuestionIdIssuer,
};

const MAX_BULK_QUESTION_METADATA_BYTES: usize = 256 * 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    store: Arc<dyn BulkPublishedQuestionMetadataStore>,
    question_id_issuer: HmacQuestionIdIssuer,
}

/// Registers the sole browser command for atomic shared Question metadata edits.
pub fn question_bulk_metadata_router(
    sessions: Arc<PostgresSessionStore>,
    store: PostgresBulkPublishedQuestionMetadataStore,
    question_id_issuer: HmacQuestionIdIssuer,
) -> Router {
    Router::new()
        .route("/api/questions/bulk-metadata", post(bulk_replace_metadata))
        .with_state(RouteState {
            sessions,
            store: Arc::new(store),
            question_id_issuer,
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BulkMetadataRequest {
    selection: Vec<BulkMetadataSelectionRequest>,
    patch: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BulkMetadataSelectionRequest {
    question_id: String,
    metadata_edit_number: u64,
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
    let input = match decode_input(&state.question_id_issuer, request) {
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

fn decode_input(
    issuer: &HmacQuestionIdIssuer,
    request: BulkMetadataRequest,
) -> Option<BulkPublishedQuestionMetadataInput> {
    if request.selection.is_empty() || request.selection.len() > MAX_BULK_QUESTION_METADATA_ITEMS {
        return None;
    }
    let selection = request
        .selection
        .into_iter()
        .map(|selected| {
            let question_id = selected.question_id.parse::<QuestionId>().ok()?;
            issuer.validates_question_id(&question_id).then_some(
                BulkPublishedQuestionMetadataSelection {
                    question_id,
                    metadata_edit_number: selected.metadata_edit_number,
                },
            )
        })
        .collect::<Option<Vec<_>>>()?;
    let patch = decode_patch(request.patch)?;
    let input = BulkPublishedQuestionMetadataInput { selection, patch };
    input.validate().ok()?;
    Some(input)
}

fn decode_patch(value: Value) -> Option<BulkPublishedQuestionMetadataPatch> {
    let fields = value.as_object()?;
    if fields.is_empty()
        || fields.len() > 3
        || fields
            .keys()
            .any(|key| !matches!(key.as_str(), "tags" | "subject" | "topic"))
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
    let optional_text = |name: &str| match fields.get(name) {
        Some(Value::Null) => Some(Some(None)),
        Some(Value::String(value)) => Some(Some(Some(value.clone()))),
        Some(_) => None,
        None => Some(None),
    };
    Some(BulkPublishedQuestionMetadataPatch {
        tags,
        subject: optional_text("subject")?,
        topic: optional_text("topic")?,
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
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
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
