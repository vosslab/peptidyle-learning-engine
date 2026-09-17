//! Bounded current shared-metadata read for bulk Question Library editing.

use std::collections::BTreeSet;

use axum::{
    Json,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use learning_data_access::QuestionLibraryStore;
use question_model::{
    MAX_BULK_QUESTION_METADATA_ITEMS, PublishedQuestionSharedMetadata, QuestionId,
};
use serde::{Deserialize, Serialize};

use super::{
    QuestionLibraryRouteState, instructor_session_hash, route_error, store_error_response,
    verified_question_id,
};

const MAX_CURRENT_SHARED_METADATA_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CurrentSharedMetadataRequest {
    question_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentSharedMetadataResponse {
    items: Vec<PublishedQuestionSharedMetadata>,
}

pub(super) async fn load_current_shared_metadata(
    State(state): State<QuestionLibraryRouteState>,
    request: Request,
) -> Response {
    // ASVS 2.2.1 and 4.1.1: accept only the bounded closed JSON request.
    if !has_json_content_type(request.headers()) {
        return invalid_response(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let session_hash = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let body = match to_bytes(
        request.into_body(),
        MAX_CURRENT_SHARED_METADATA_REQUEST_BYTES,
    )
    .await
    {
        Ok(body) => body,
        Err(_) => return invalid_response(StatusCode::PAYLOAD_TOO_LARGE),
    };
    let request = match serde_json::from_slice::<CurrentSharedMetadataRequest>(&body) {
        Ok(request) => request,
        Err(_) => return invalid_response(StatusCode::UNPROCESSABLE_ENTITY),
    };
    let Some(question_ids) = decode_question_ids(request.question_ids) else {
        return invalid_response(StatusCode::UNPROCESSABLE_ENTITY);
    };

    match state
        .store
        .load_current_published_question_shared_metadata(session_hash, &question_ids)
        .await
    {
        Ok(items) => {
            crate::auth::no_store(Json(CurrentSharedMetadataResponse { items }).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

fn decode_question_ids(values: Vec<String>) -> Option<Vec<QuestionId>> {
    if values.is_empty() || values.len() > MAX_BULK_QUESTION_METADATA_ITEMS {
        return None;
    }
    let mut distinct = BTreeSet::new();
    for value in values {
        // The shared exact checksum is parsed before the Store can perform any
        // existence, availability, or metadata lookup.
        let question_id = verified_question_id(&value)?;
        if !distinct.insert(question_id) {
            return None;
        }
    }
    Some(distinct.into_iter().collect())
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

fn invalid_response(status: StatusCode) -> Response {
    route_error(status, "Current Question metadata request is invalid")
}
