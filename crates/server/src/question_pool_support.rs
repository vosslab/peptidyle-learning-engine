//! Instructor read and replacement for one Question Pool's optional support texts.
//!
//! The Pool metadata token advances once. Membership and Question identities
//! are not part of the command.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
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
        crate::auth::joined_cookie_header(headers).as_deref(),
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
