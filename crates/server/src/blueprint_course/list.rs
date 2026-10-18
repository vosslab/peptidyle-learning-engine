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
use browser_api_contract::blueprint_course::BlueprintCourseListPageView;
use learning_data_access::{
    BlueprintCourseFacetChoice, BlueprintCourseFacetChoiceRequest, BlueprintCourseFacetField,
    BlueprintCourseListRequest, BlueprintCourseListSort, BlueprintCourseStore, DiscoveryPageSize,
    NumberedDiscoveryPageRequest, PageNumber,
};
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
    page_number: Option<u32>,
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

const BLUEPRINT_TRANSPORT_CURSOR_MAX: usize = 16_384;

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
    if query.page_number.is_some() || query.page_size.is_some_and(|size| size > 100) {
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
    match state
        .blueprints
        .list_blueprint_courses(session, request)
        .await
    {
        Ok(page) => crate::auth::no_store(
            Json(BlueprintCourseListPageView {
                items: page.items.into_iter().map(summary_view).collect(),
                page_number: page.page_number.get(),
                page_size: page.page_size.get(),
                total_count: page.total_count,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

fn list_request(query: ListQuery) -> Option<BlueprintCourseListRequest> {
    // ASVS 2.2.1/2/3: validate bounded numbered pages and the complete query.
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
    let page_number = PageNumber::new(query.page_number.unwrap_or(1)).ok()?;
    let page_size = DiscoveryPageSize::new(query.page_size.unwrap_or(50)).ok()?;
    Some(BlueprintCourseListRequest {
        page: NumberedDiscoveryPageRequest {
            page_number,
            page_size,
        },
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
