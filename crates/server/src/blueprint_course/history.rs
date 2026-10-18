//! Whole-result, visibility-reattested discovery of saved Revisions and metadata events.

use axum::{
    Json,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use browser_api_contract::blueprint_course::{
    BlueprintHistoryAvailabilityFacets, BlueprintHistoryPageView,
};
use learning_data_access::{
    BlueprintHistoryKind, BlueprintHistoryQuery, BlueprintHistorySort, BlueprintHistoryStore,
    NumberedPageRequest, PageNumber, PageSize,
};
use serde::Deserialize;

use super::{
    BlueprintCourseRouteState, instructor_session_hash, parse_blueprint_course_id, route_error,
    store_error_response,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum HistoryKind {
    #[default]
    Revisions,
    Metadata,
}

impl HistoryKind {
    fn store_kind(self) -> BlueprintHistoryKind {
        match self {
            Self::Revisions => BlueprintHistoryKind::Revisions,
            Self::Metadata => BlueprintHistoryKind::Metadata,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum HistorySort {
    #[default]
    Sequence,
    RecordedAt,
    Name,
}

impl HistorySort {
    fn store_sort(self) -> BlueprintHistorySort {
        match self {
            Self::Sequence => BlueprintHistorySort::Sequence,
            Self::RecordedAt => BlueprintHistorySort::RecordedAt,
            Self::Name => BlueprintHistorySort::Name,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum HistoryDirection {
    Ascending,
    #[default]
    Descending,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HistoryQuery {
    #[serde(default)]
    kind: HistoryKind,
    #[serde(default)]
    q: String,
    #[serde(default)]
    sort: HistorySort,
    #[serde(default)]
    direction: HistoryDirection,
    availability: Option<String>,
    page_number: Option<u32>,
    page_size: Option<u16>,
}

pub(super) async fn list_history(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(blueprint_course_id): Path<String>,
    query: Result<Query<HistoryQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(value) => value,
        Err(_) => return invalid_page(),
    };
    let blueprint_course_id = match parse_blueprint_course_id(&blueprint_course_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let search = match clean_query(&query) {
        Some(value) => value,
        None => return invalid_page(),
    };
    let page = match page_request(&query) {
        Some(value) => value,
        None => return invalid_page(),
    };
    match state
        .blueprints
        .list_blueprint_history(
            session,
            blueprint_course_id,
            query.kind.store_kind(),
            search,
            page,
        )
        .await
    {
        Ok(result) => crate::auth::no_store(
            Json(BlueprintHistoryPageView {
                items: result.page.items,
                page_number: result.page.page_number.get(),
                page_size: result.page.page_size.get(),
                total_count: result.page.total_count,
                availability_facets: BlueprintHistoryAvailabilityFacets {
                    active_count: result.active_count,
                    archived_count: result.archived_count,
                },
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

fn invalid_page() -> Response {
    route_error(StatusCode::BAD_REQUEST, "Blueprint history page is invalid")
}

fn clean_query(query: &HistoryQuery) -> Option<BlueprintHistoryQuery> {
    if query.kind == HistoryKind::Revisions && query.sort == HistorySort::Name {
        return None;
    }
    let text = query.q.trim().to_lowercase();
    if text.chars().count() > 120 || text.chars().any(char::is_control) {
        return None;
    }
    let mut availability = match &query.availability {
        Some(value) if query.kind == HistoryKind::Metadata => {
            value.split(',').map(str::to_owned).collect::<Vec<_>>()
        }
        Some(_) => return None,
        None => Vec::new(),
    };
    if availability.len() > 2
        || availability
            .iter()
            .any(|value| value != "active" && value != "archived")
    {
        return None;
    }
    availability.sort();
    availability.dedup();
    Some(BlueprintHistoryQuery {
        text,
        sort: query.sort.store_sort(),
        ascending: query.direction == HistoryDirection::Ascending,
        availability,
    })
}

fn page_request(query: &HistoryQuery) -> Option<NumberedPageRequest> {
    let size = PageSize::new(query.page_size.unwrap_or(50)).ok()?;
    let page_number = PageNumber::new(query.page_number.unwrap_or(1)).ok()?;
    Some(NumberedPageRequest {
        page_number,
        page_size: size,
    })
}
