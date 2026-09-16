//! Literal name narrowing and bounded, visibility-reattested Blueprint pages.

use super::{
    BlueprintCourseRouteState, instructor_session_hash, route_error, store_error_response,
    summary_view, unavailable,
};
use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    BlueprintCourseListRequest, BlueprintCourseStore, Cursor, PageRequest, PageSize,
};
use question_model::{BlueprintCourseReference, BlueprintCourseSummaryView};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ListQuery {
    #[serde(default)]
    include_archived: bool,
    #[serde(default)]
    public_only: bool,
    #[serde(default)]
    query: String,
    cursor: Option<String>,
    page_size: Option<u16>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListCursor {
    version: u8,
    query: String,
    include_archived: bool,
    public_only: bool,
    page_size: u16,
    after: (String, BlueprintCourseReference),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse {
    items: Vec<BlueprintCourseSummaryView>,
    next_cursor: Option<String>,
}

pub(super) async fn list_blueprints(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Response {
    let request = match query.ok().and_then(|Query(query)| list_request(query)) {
        Some(value) => value,
        None => return route_error(StatusCode::BAD_REQUEST, "Blueprint Course page is invalid"),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let binding = (
        request.query.clone(),
        request.include_archived,
        request.public_only,
        request.page.size.get(),
    );
    match state
        .blueprints
        .list_blueprint_courses(session, request)
        .await
    {
        Ok(page) => {
            let next_cursor = match page.next_cursor {
                Some(after) => {
                    let after = match serde_json::from_str(after.as_str()) {
                        Ok(value) => value,
                        Err(_) => return unavailable(),
                    };
                    let cursor = ListCursor {
                        version: 1,
                        query: binding.0,
                        include_archived: binding.1,
                        public_only: binding.2,
                        page_size: binding.3,
                        after,
                    };
                    match serde_json::to_vec(&cursor) {
                        Ok(bytes) => Some(URL_SAFE_NO_PAD.encode(bytes)),
                        Err(_) => return unavailable(),
                    }
                }
                None => None,
            };
            crate::auth::no_store(
                Json(ListResponse {
                    items: page.items.into_iter().map(summary_view).collect(),
                    next_cursor,
                })
                .into_response(),
            )
        }
        Err(error) => store_error_response(error),
    }
}

fn list_request(query: ListQuery) -> Option<BlueprintCourseListRequest> {
    // ASVS 2.2.1/2/3: trusted bounds and cursor binding to the full normalized request.
    if query.query.chars().count() > 256
        || query.query.chars().any(char::is_control)
        || (query.public_only && query.include_archived)
    {
        return None;
    }
    let normalized = query.query.trim().to_lowercase();
    if normalized.chars().count() > 256 {
        return None;
    }
    let size = PageSize::new(query.page_size.unwrap_or(50)).ok()?;
    let after = match query.cursor {
        Some(token) => {
            if token.len() > 8192 {
                return None;
            }
            let cursor: ListCursor =
                serde_json::from_slice(&URL_SAFE_NO_PAD.decode(token).ok()?).ok()?;
            if cursor.version != 1
                || cursor.query != normalized
                || cursor.include_archived != query.include_archived
                || cursor.public_only != query.public_only
                || cursor.page_size != size.get()
                || cursor.after.0.chars().count() > 500
            {
                return None;
            }
            Some(Cursor::parse(serde_json::to_string(&cursor.after).ok()?).ok()?)
        }
        None => None,
    };
    Some(BlueprintCourseListRequest {
        page: PageRequest { after, size },
        query: normalized,
        include_archived: query.include_archived,
        public_only: query.public_only,
    })
}
