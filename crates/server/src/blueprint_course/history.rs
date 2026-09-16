//! Independent bounded pages of saved Revisions and recorded metadata facts.

use axum::{
    Json,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use browser_api_contract::blueprint_course::BlueprintHistoryPageView;
use learning_data_access::{
    BlueprintHistoryKind, BlueprintHistoryStore, Cursor, PageRequest, PageSize,
};
use question_model::BlueprintCourseReference;
use serde::{Deserialize, Serialize};

use super::{
    BlueprintCourseRouteState, instructor_session_hash, parse_reference, route_error,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HistoryQuery {
    #[serde(default)]
    kind: HistoryKind,
    page_size: Option<u16>,
    cursor: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HistoryCursor {
    version: u8,
    reference: BlueprintCourseReference,
    kind: HistoryKind,
    page_size: u16,
    after: String,
}

pub(super) async fn list_history(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    query: Result<Query<HistoryQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(value) => value,
        Err(_) => return route_error(StatusCode::BAD_REQUEST, "Blueprint history page is invalid"),
    };
    let reference = match parse_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let page = match page_request(reference, &query) {
        Some(value) => value,
        None => return route_error(StatusCode::BAD_REQUEST, "Blueprint history page is invalid"),
    };
    let size = page.size.get();
    // ASVS 8.2.1/2/3, 8.3.1/2: each page reattests ordinary current visibility.
    match state
        .blueprints
        .list_blueprint_history(session, reference, query.kind.store_kind(), page)
        .await
    {
        Ok(page) => {
            let next_cursor = match page.next_cursor {
                Some(after) => match encode_cursor(reference, query.kind, size, after) {
                    Some(value) => Some(value),
                    None => return unavailable(),
                },
                None => None,
            };
            // ASVS 4.1.1, 14.3.2: typed, answer-free facts are not cached.
            crate::auth::no_store(
                Json(BlueprintHistoryPageView {
                    items: page.items,
                    next_cursor,
                })
                .into_response(),
            )
        }
        Err(error) => store_error_response(error),
    }
}

fn page_request(reference: BlueprintCourseReference, query: &HistoryQuery) -> Option<PageRequest> {
    // ASVS 2.2.1/2/3: bound cursor size and bind the sequence/course/page limit.
    let size = PageSize::new(query.page_size.unwrap_or(50)).ok()?;
    let after = match query.cursor.as_ref() {
        Some(token) => {
            if token.len() > 512 {
                return None;
            }
            let bytes = URL_SAFE_NO_PAD.decode(token).ok()?;
            let cursor: HistoryCursor = serde_json::from_slice(&bytes).ok()?;
            if cursor.version != 1
                || cursor.reference != reference
                || cursor.kind != query.kind
                || cursor.page_size != size.get()
                || !valid_key(cursor.kind, &cursor.after)
            {
                return None;
            }
            Some(Cursor::parse(cursor.after).ok()?)
        }
        None => None,
    };
    Some(PageRequest { after, size })
}

fn valid_key(kind: HistoryKind, key: &str) -> bool {
    match kind {
        HistoryKind::Revisions => {
            key.parse::<i64>().is_ok_and(|number| number > 0)
                && key
                    .as_bytes()
                    .first()
                    .is_some_and(|byte| matches!(byte, b'1'..=b'9'))
        }
        HistoryKind::Metadata => {
            uuid::Uuid::parse_str(key).is_ok_and(|value| value.to_string() == key)
        }
    }
}

fn encode_cursor(
    reference: BlueprintCourseReference,
    kind: HistoryKind,
    page_size: u16,
    after: Cursor,
) -> Option<String> {
    let cursor = HistoryCursor {
        version: 1,
        reference,
        kind,
        page_size,
        after: after.as_str().to_owned(),
    };
    serde_json::to_vec(&cursor)
        .ok()
        .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
}
