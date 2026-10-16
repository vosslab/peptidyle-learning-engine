//! Literal name narrowing and bounded, visibility-reattested Blueprint pages.

use super::{
    BlueprintCourseRouteState, instructor_session_hash, route_error, store_error_response,
    summary_view, unavailable,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::{Query, QueryRejection};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    BlueprintCourseFacetChoice, BlueprintCourseFacetChoiceRequest, BlueprintCourseFacetField,
    BlueprintCourseListCursorPosition, BlueprintCourseListRequest, BlueprintCourseListSort,
    BlueprintCourseStore, Cursor, DiscoveryPageRequest, DiscoveryPageSize,
};
use question_model::BlueprintCourseSummaryView;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(default, rename = "discipline_uuids")]
    discipline_uuids: Vec<Uuid>,
    #[serde(default, rename = "subject_uuids")]
    subject_uuids: Vec<Uuid>,
    #[serde(default, rename = "topic_uuids")]
    topic_uuids: Vec<Uuid>,
    #[serde(default, rename = "subtopic_uuids")]
    subtopic_uuids: Vec<Uuid>,
    #[serde(default)]
    tags: Vec<String>,
    choice_search: Option<String>,
    choice_cursor: Option<String>,
}

const BLUEPRINT_LIST_CURSOR_VERSION: u8 = 7;
const BLUEPRINT_TRANSPORT_CURSOR_MAX: usize = 16_384;

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
    discipline_uuids: Vec<Uuid>,
    subject_uuids: Vec<Uuid>,
    topic_uuids: Vec<Uuid>,
    subtopic_uuids: Vec<Uuid>,
    tags: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse {
    items: Vec<BlueprintCourseSummaryView>,
    next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FacetCursor {
    version: u8,
    field: BlueprintCourseFacetField,
    query: ListQuery,
    after_label: String,
    after_key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FacetResponse {
    choices: Vec<BlueprintCourseFacetChoice>,
    selected: Vec<BlueprintCourseFacetChoice>,
    next_cursor: Option<String>,
}

pub(super) async fn facet_choices(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(field): Path<String>,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Response {
    let field = match field.as_str() {
        "disciplineUuids" => BlueprintCourseFacetField::DisciplineUuids,
        "subjectUuids" => BlueprintCourseFacetField::SubjectUuids,
        "topicUuids" => BlueprintCourseFacetField::TopicUuids,
        "subtopicUuids" => BlueprintCourseFacetField::SubtopicUuids,
        "tags" => BlueprintCourseFacetField::Tags,
        _ => return route_error(StatusCode::BAD_REQUEST, "Blueprint facet field is invalid"),
    };
    let Some(Query(mut query)) = query.ok() else {
        return route_error(StatusCode::BAD_REQUEST, "Blueprint facet query is invalid");
    };
    if query.cursor.is_some() || query.page_size.is_some_and(|size| size > 100) {
        return route_error(StatusCode::BAD_REQUEST, "Blueprint facet query is invalid");
    }
    let choice_search = query
        .choice_search
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if choice_search.chars().count() > 128 {
        return route_error(
            StatusCode::BAD_REQUEST,
            "Blueprint choice search is invalid",
        );
    }
    query.choice_search = Some(choice_search.clone());
    let after_position = match query.choice_cursor.take() {
        Some(token) => {
            if token.len() > BLUEPRINT_TRANSPORT_CURSOR_MAX {
                return route_error(
                    StatusCode::BAD_REQUEST,
                    "Blueprint choice cursor is invalid",
                );
            }
            let Ok(bytes) = URL_SAFE_NO_PAD.decode(token) else {
                return route_error(
                    StatusCode::BAD_REQUEST,
                    "Blueprint choice cursor is invalid",
                );
            };
            let Ok(cursor) = serde_json::from_slice::<FacetCursor>(&bytes) else {
                return route_error(
                    StatusCode::BAD_REQUEST,
                    "Blueprint choice cursor is invalid",
                );
            };
            if cursor.version != 1 || cursor.field != field || cursor.query != query {
                return route_error(
                    StatusCode::BAD_REQUEST,
                    "Blueprint choice cursor is invalid",
                );
            }
            Some((cursor.after_label, cursor.after_key))
        }
        None => None,
    };
    let Some(search) = list_request(query.clone()) else {
        return route_error(StatusCode::BAD_REQUEST, "Blueprint facet query is invalid");
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .blueprints
        .list_blueprint_course_facet_choices(
            session,
            BlueprintCourseFacetChoiceRequest {
                search,
                field,
                choice_search,
                after_position,
            },
        )
        .await
    {
        Ok(page) => {
            let next_cursor = page.next_position.and_then(|(after_label, after_key)| {
                let cursor = FacetCursor {
                    version: 1,
                    field,
                    query,
                    after_label,
                    after_key,
                };
                serde_json::to_vec(&cursor)
                    .ok()
                    .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
            });
            if next_cursor
                .as_ref()
                .is_some_and(|value| value.len() > BLUEPRINT_TRANSPORT_CURSOR_MAX)
            {
                return unavailable();
            }
            crate::auth::no_store(
                Json(FacetResponse {
                    choices: page.choices,
                    selected: page.selected,
                    next_cursor,
                })
                .into_response(),
            )
        }
        Err(error) => store_error_response(error),
    }
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
        request.discipline_uuids.clone(),
        request.subject_uuids.clone(),
        request.topic_uuids.clone(),
        request.subtopic_uuids.clone(),
        request.tags.clone(),
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
                        discipline_uuids: binding.12,
                        subject_uuids: binding.13,
                        topic_uuids: binding.14,
                        subtopic_uuids: binding.15,
                        tags: binding.16,
                    };
                    match serde_json::to_vec(&cursor) {
                        Ok(bytes) => Some(URL_SAFE_NO_PAD.encode(bytes)),
                        Err(_) => return unavailable(),
                    }
                }
                None => None,
            };
            if next_cursor
                .as_ref()
                .is_some_and(|value| value.len() > BLUEPRINT_TRANSPORT_CURSOR_MAX)
            {
                return unavailable();
            }
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
        || query.discipline_uuids.len() > 20
        || query.subject_uuids.len() > 20
        || query.topic_uuids.len() > 20
        || query.subtopic_uuids.len() > 20
        || query.tags.len() > 20
        || query.tags.iter().any(|value| {
            value.is_empty()
                || value.trim_matches(' ') != value
                || value.chars().count() > 120
                || value.chars().any(char::is_control)
        })
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
            if token.len() > BLUEPRINT_TRANSPORT_CURSOR_MAX {
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
                || cursor.discipline_uuids != query.discipline_uuids
                || cursor.subject_uuids != query.subject_uuids
                || cursor.topic_uuids != query.topic_uuids
                || cursor.subtopic_uuids != query.subtopic_uuids
                || cursor.tags != query.tags
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
        discipline_uuids: query.discipline_uuids,
        subject_uuids: query.subject_uuids,
        topic_uuids: query.topic_uuids,
        subtopic_uuids: query.subtopic_uuids,
        tags: query.tags,
    })
}

fn valid_cursor_position(
    position: &BlueprintCourseListCursorPosition,
    sort: BlueprintCourseListSort,
) -> bool {
    match (sort, position) {
        (
            BlueprintCourseListSort::Name | BlueprintCourseListSort::NameDescending,
            BlueprintCourseListCursorPosition::Name { long_name, .. },
        ) => long_name.chars().count() <= 500,
        (
            BlueprintCourseListSort::Adoptions | BlueprintCourseListSort::AdoptionsAscending,
            BlueprintCourseListCursorPosition::Adoptions { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Students | BlueprintCourseListSort::StudentsAscending,
            BlueprintCourseListCursorPosition::Students { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Stars | BlueprintCourseListSort::StarsAscending,
            BlueprintCourseListCursorPosition::Stars { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Watches | BlueprintCourseListSort::WatchesAscending,
            BlueprintCourseListCursorPosition::Watches { long_name, .. },
        )
        | (
            BlueprintCourseListSort::RecentEdits | BlueprintCourseListSort::RecentEditsAscending,
            BlueprintCourseListCursorPosition::RecentEdits { long_name, .. },
        ) => long_name.chars().count() <= 500,
        _ => false,
    }
}
