//! Whole-result, visibility-reattested discovery of saved Revisions and metadata events.

use axum::{
    Json,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use browser_api_contract::blueprint_course::{
    BlueprintHistoryAvailabilityFacets, BlueprintHistoryPageView,
};
use learning_data_access::{
    BlueprintHistoryKind, BlueprintHistoryQuery, BlueprintHistorySort, BlueprintHistoryStore,
    Cursor, PageRequest, PageSize, SessionTokenHash,
};
use question_model::BlueprintCourseId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    BlueprintCourseRouteState, instructor_session_hash, parse_blueprint_course_id, route_error,
    store_error_response, unavailable,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    page_size: Option<u16>,
    cursor: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HistoryCursor {
    version: u8,
    query_fingerprint: String,
    page_size: u16,
    after: String,
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
    let fingerprint = query_fingerprint(&blueprint_course_id, &query, &search, session);
    let page = match page_request(&query, &fingerprint) {
        Some(value) => value,
        None => return invalid_page(),
    };
    let size = page.size.get();
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
        Ok(result) => {
            let next_cursor = match result.page.next_cursor {
                Some(after) => match encode_cursor(&fingerprint, size, after) {
                    Some(value) => Some(value),
                    None => return unavailable(),
                },
                None => None,
            };
            crate::auth::no_store(
                Json(BlueprintHistoryPageView {
                    items: result.page.items,
                    next_cursor,
                    availability_facets: BlueprintHistoryAvailabilityFacets {
                        public_count: result.public_count,
                        archived_count: result.archived_count,
                    },
                })
                .into_response(),
            )
        }
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
            .any(|value| value != "public" && value != "archived")
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

fn query_fingerprint(
    blueprint_course_id: &BlueprintCourseId,
    query: &HistoryQuery,
    search: &BlueprintHistoryQuery,
    session: SessionTokenHash,
) -> String {
    let value = serde_json::to_vec(&(
        blueprint_course_id,
        query.kind,
        query.sort,
        query.direction,
        &search.text,
        &search.availability,
        session.to_string(),
    ))
    .expect("Blueprint history query serializes");
    URL_SAFE_NO_PAD.encode(Sha256::digest(value))
}

fn page_request(query: &HistoryQuery, fingerprint: &str) -> Option<PageRequest> {
    let size = PageSize::new(query.page_size.unwrap_or(50)).ok()?;
    let after = match query.cursor.as_ref() {
        Some(token) => {
            if token.len() > 8192 {
                return None;
            }
            let bytes = URL_SAFE_NO_PAD.decode(token).ok()?;
            let cursor: HistoryCursor = serde_json::from_slice(&bytes).ok()?;
            if cursor.version != 2
                || cursor.query_fingerprint != fingerprint
                || cursor.page_size != size.get()
            {
                return None;
            }
            let (key, sequence): (String, i64) = serde_json::from_str(&cursor.after).ok()?;
            if key.chars().count() > 500 || sequence <= 0 {
                return None;
            }
            Some(Cursor::parse(cursor.after).ok()?)
        }
        None => None,
    };
    Some(PageRequest { after, size })
}

fn encode_cursor(fingerprint: &str, page_size: u16, after: Cursor) -> Option<String> {
    let cursor = HistoryCursor {
        version: 2,
        query_fingerprint: fingerprint.to_owned(),
        page_size,
        after: after.as_str().to_owned(),
    };
    serde_json::to_vec(&cursor)
        .ok()
        .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
}
