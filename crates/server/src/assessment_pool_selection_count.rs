//! Count-only Instructor command for one Assessment-owned Question Pool entry.

use std::{num::NonZeroU32, sync::Arc};

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{
        HeaderMap, StatusCode,
        header::{ETAG, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::put,
};
use learning_data_access::{
    AssessmentPoolSelectionCountInput, AssessmentPoolSelectionCountStore, SessionTokenHash,
    StoreError, postgres::PostgresAssessmentPoolSelectionCountStore,
};
use question_model::{
    AssessmentEditNumber, AssessmentEntryId, AssessmentReference, CourseInstanceReference,
    ProductRole,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

const MAX_SELECTION_COUNT_REQUEST_BYTES: usize = 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<learning_data_access::postgres::PostgresSessionStore>,
    selection_counts: Arc<dyn AssessmentPoolSelectionCountStore>,
}

/// Registers the count-only Assessment Pool Entry mutation.
pub fn assessment_pool_selection_count_router(
    sessions: Arc<learning_data_access::postgres::PostgresSessionStore>,
    selection_counts: PostgresAssessmentPoolSelectionCountStore,
) -> Router {
    assessment_pool_selection_count_router_with_store(sessions, Arc::new(selection_counts))
}

pub(crate) fn assessment_pool_selection_count_router_with_store(
    sessions: Arc<learning_data_access::postgres::PostgresSessionStore>,
    selection_counts: Arc<dyn AssessmentPoolSelectionCountStore>,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/question-pool-forks/{entry}/selection-count",
            put(update_selection_count),
        )
        .with_state(RouteState {
            sessions,
            selection_counts,
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectionCountRequest {
    selection_count: NonZeroU32,
}

async fn update_selection_count(
    State(state): State<RouteState>,
    Path((course, assessment, entry)): Path<(String, String, String)>,
    request: Request,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Some(value) => value,
        None => return concealed(),
    };
    let assessment_entry = match Uuid::parse_str(&entry) {
        Ok(value) => AssessmentEntryId::from_uuid(value),
        Err(_) => return concealed(),
    };
    let expected_assessment_edit_number = match edit_header(request.headers()) {
        Some(value) => value,
        None => return concealed(),
    };
    let token = match instructor(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let body = match decode_json::<SelectionCountRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };

    match state
        .selection_counts
        .update_assessment_question_pool_selection_count(
            token,
            AssessmentPoolSelectionCountInput {
                course,
                assessment,
                assessment_entry,
                expected_assessment_edit_number,
                selection_count: body.selection_count,
            },
        )
        .await
    {
        Ok(receipt) => receipt_response(receipt),
        Err(error) => store_error(error),
    }
}

fn receipt_response(
    receipt: question_model::AssessmentQuestionPoolSelectionCountReceipt,
) -> Response {
    let mut response = crate::auth::no_store((StatusCode::OK, Json(receipt)).into_response());
    if let Ok(header) = format!("\"{}\"", receipt.assessment_edit_number.value()).parse() {
        response.headers_mut().insert(ETAG, header);
    }
    response
}

async fn decode_json<T: serde::de::DeserializeOwned>(request: Request) -> Result<T, Response> {
    let is_json = request
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json"))
        });
    if !is_json {
        return Err(error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Assessment Pool selection count requires JSON",
        ));
    }
    let body = to_bytes(request.into_body(), MAX_SELECTION_COUNT_REQUEST_BYTES)
        .await
        .map_err(|_| {
            error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Assessment Pool selection count is too large",
            )
        })?;
    serde_json::from_slice(&body).map_err(|_| invalid())
}

fn refs(course: &str, assessment: &str) -> Option<(CourseInstanceReference, AssessmentReference)> {
    Some((course.parse().ok()?, assessment.parse().ok()?))
}

fn edit_header(headers: &HeaderMap) -> Option<AssessmentEditNumber> {
    headers
        .get(IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
        })
        .and_then(|value| value.parse().ok())
}

async fn instructor(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(state.sessions.as_ref(), cookie(headers).as_deref()).await {
        Ok(value) if value.record.product_role == ProductRole::Instructor => Ok(value.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

fn cookie(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all("cookie")
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn store_error(error_value: StoreError) -> Response {
    match error_value {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => invalid(),
        StoreError::Conflict | StoreError::RetryableTransaction => error(
            StatusCode::PRECONDITION_FAILED,
            "Assessment Pool selection count changed",
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => error(
            StatusCode::CONFLICT,
            "Assessment Pool selection count conflicts",
        ),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => unavailable(),
    }
}

fn concealed() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "Assessment Pool selection count not found",
    )
}

fn invalid() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Assessment Pool selection count is invalid",
    )
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Assessment Pool selection count unavailable",
    )
}

fn error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
