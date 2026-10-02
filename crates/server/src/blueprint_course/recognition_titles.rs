//! Current Question and Pool titles for Instructor recognition.

use std::collections::BTreeSet;

use axum::{
    Json,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use learning_data_access::BlueprintCourseStore;
use question_model::{PublishedQuestionId, QuestionPoolId};
use serde::{Deserialize, Serialize};

use super::responses::{route_error, store_error_response};
use super::{BlueprintCourseRouteState, instructor_session_hash};

const MAX_RECOGNITION_TITLE_IDS: usize = 1000;
const MAX_RECOGNITION_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecognitionTitlesRequest {
    question_ids: Vec<String>,
    pool_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecognitionTitleItem {
    public_id: String,
    title: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecognitionTitlesResponse {
    questions: Vec<RecognitionTitleItem>,
    pools: Vec<RecognitionTitleItem>,
}

pub(super) async fn load_recognition_titles(
    State(state): State<BlueprintCourseRouteState>,
    request: Request,
) -> Response {
    // ASVS 2.2.1 and 4.1.1: accept only the bounded closed JSON request.
    if !has_json_content_type(request.headers()) {
        return invalid();
    }
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let body = match to_bytes(request.into_body(), MAX_RECOGNITION_REQUEST_BYTES).await {
        Ok(body) => body,
        Err(_) => return invalid(),
    };
    let request = match serde_json::from_slice::<RecognitionTitlesRequest>(&body) {
        Ok(request) => request,
        Err(_) => return invalid(),
    };
    let Some(question_ids) = canonical_ids::<PublishedQuestionId>(request.question_ids) else {
        return invalid();
    };
    let Some(pool_ids) = canonical_ids::<QuestionPoolId>(request.pool_ids) else {
        return invalid();
    };
    match state
        .blueprints
        .load_recognition_titles(session, &question_ids, &pool_ids)
        .await
    {
        Ok(titles) => crate::auth::no_store(
            Json(RecognitionTitlesResponse {
                questions: titles
                    .questions
                    .into_iter()
                    .map(|(id, title)| RecognitionTitleItem {
                        public_id: id.as_str().to_owned(),
                        title,
                    })
                    .collect(),
                pools: titles
                    .pools
                    .into_iter()
                    .map(|(id, title)| RecognitionTitleItem {
                        public_id: id.as_str().to_owned(),
                        title,
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

fn canonical_ids<T>(values: Vec<String>) -> Option<Vec<T>>
where
    T: std::str::FromStr + Ord,
{
    if values.len() > MAX_RECOGNITION_TITLE_IDS {
        return None;
    }
    let mut distinct = BTreeSet::new();
    for value in values {
        let id = value.parse::<T>().ok()?;
        if !distinct.insert(id) {
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

fn invalid() -> Response {
    route_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Recognition titles request is invalid",
    )
}
