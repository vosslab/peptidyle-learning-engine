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
    BlueprintCourseListCursorPosition, BlueprintCourseListRequest, BlueprintCourseListSort,
    BlueprintCourseStore, Cursor, DiscoveryPageRequest, DiscoveryPageSize,
};
use question_model::BlueprintCourseSummaryView;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ListQuery {
    #[serde(default)]
    include_archived: bool,
    #[serde(default)]
    public_only: bool,
    #[serde(default)]
    promoted_only: bool,
    #[serde(default)]
    query: String,
    cursor: Option<String>,
    page_size: Option<u16>,
    discipline_uuid: Option<Uuid>,
    subject_uuid: Option<Uuid>,
    topic_uuid: Option<Uuid>,
    subtopic_uuid: Option<Uuid>,
    #[serde(default)]
    cross_discipline: bool,
    #[serde(default)]
    sort: BlueprintCourseListSort,
    #[serde(default)]
    tag: String,
}

const BLUEPRINT_LIST_CURSOR_VERSION: u8 = 6;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListCursor {
    version: u8,
    query: String,
    include_archived: bool,
    public_only: bool,
    promoted_only: bool,
    page_size: u16,
    after: BlueprintCourseListCursorPosition,
    discipline_uuid: Option<Uuid>,
    subject_uuid: Option<Uuid>,
    topic_uuid: Option<Uuid>,
    subtopic_uuid: Option<Uuid>,
    cross_discipline: bool,
    sort: BlueprintCourseListSort,
    tag: String,
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
        request.promoted_only,
        request.page.size.get(),
        request.discipline_uuid,
        request.subject_uuid,
        request.topic_uuid,
        request.subtopic_uuid,
        request.cross_discipline,
        request.sort,
        request.tag.clone(),
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
                        version: BLUEPRINT_LIST_CURSOR_VERSION,
                        query: binding.0,
                        include_archived: binding.1,
                        public_only: binding.2,
                        promoted_only: binding.3,
                        page_size: binding.4,
                        after,
                        discipline_uuid: binding.5,
                        subject_uuid: binding.6,
                        topic_uuid: binding.7,
                        subtopic_uuid: binding.8,
                        cross_discipline: binding.9,
                        sort: binding.10,
                        tag: binding.11,
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
    let tag = query.tag.trim_matches(' ').to_owned();
    if query.query.chars().count() > 256
        || query.query.chars().any(char::is_control)
        || tag.chars().count() > 120
        || tag.chars().any(char::is_control)
        || (query.public_only && query.include_archived)
        || (query.subject_uuid.is_some() && query.discipline_uuid.is_none())
        || (query.topic_uuid.is_some() && query.subject_uuid.is_none())
        || (query.subtopic_uuid.is_some() && query.topic_uuid.is_none())
        || (query.cross_discipline
            && (query.discipline_uuid.is_none() || query.subject_uuid.is_none()))
    {
        return None;
    }
    let normalized = query.query.trim().to_lowercase();
    if normalized.chars().count() > 256 {
        return None;
    }
    let size = DiscoveryPageSize::new(query.page_size.unwrap_or(50)).ok()?;
    let after = match query.cursor {
        Some(token) => {
            if token.len() > 8192 {
                return None;
            }
            let cursor: ListCursor =
                serde_json::from_slice(&URL_SAFE_NO_PAD.decode(token).ok()?).ok()?;
            if cursor.version != BLUEPRINT_LIST_CURSOR_VERSION
                || cursor.query != normalized
                || cursor.include_archived != query.include_archived
                || cursor.public_only != query.public_only
                || cursor.promoted_only != query.promoted_only
                || cursor.page_size != size.get()
                || cursor.discipline_uuid != query.discipline_uuid
                || cursor.subject_uuid != query.subject_uuid
                || cursor.topic_uuid != query.topic_uuid
                || cursor.subtopic_uuid != query.subtopic_uuid
                || cursor.cross_discipline != query.cross_discipline
                || cursor.sort != query.sort
                || cursor.tag != tag
                || !valid_cursor_position(&cursor.after, query.sort)
            {
                return None;
            }
            Some(Cursor::parse(serde_json::to_string(&cursor.after).ok()?).ok()?)
        }
        None => None,
    };
    Some(BlueprintCourseListRequest {
        page: DiscoveryPageRequest { after, size },
        sort: query.sort,
        query: normalized,
        include_archived: query.include_archived,
        public_only: query.public_only,
        promoted_only: query.promoted_only,
        discipline_uuid: query.discipline_uuid,
        subject_uuid: query.subject_uuid,
        topic_uuid: query.topic_uuid,
        subtopic_uuid: query.subtopic_uuid,
        cross_discipline: query.cross_discipline,
        tag,
    })
}

fn valid_cursor_position(
    position: &BlueprintCourseListCursorPosition,
    sort: BlueprintCourseListSort,
) -> bool {
    match (sort, position) {
        (
            BlueprintCourseListSort::Name,
            BlueprintCourseListCursorPosition::Name { long_name, .. },
        ) => long_name.chars().count() <= 500,
        (
            BlueprintCourseListSort::Adoptions,
            BlueprintCourseListCursorPosition::Adoptions { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Students,
            BlueprintCourseListCursorPosition::Students { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Stars,
            BlueprintCourseListCursorPosition::Stars { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Watches,
            BlueprintCourseListCursorPosition::Watches { long_name, .. },
        )
        | (
            BlueprintCourseListSort::RecentEdits,
            BlueprintCourseListCursorPosition::RecentEdits { long_name, .. },
        ) => long_name.chars().count() <= 500,
        _ => false,
    }
}
