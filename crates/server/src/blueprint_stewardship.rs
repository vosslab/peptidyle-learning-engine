//! Active-Instructor Blueprint Course stewardship HTTP boundary.
//!
//! This surface deliberately separates public Star aggregate facts from the
//! caller's private Watch state. It exposes no Account identity, email,
//! ID, avatar, or another Instructor's Watch state.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    BlueprintCourseStarProjection, BlueprintCourseStarredInstructor, BlueprintCourseWatchDirection,
    BlueprintCourseWatchEvent, BlueprintCourseWatchEventFacet, BlueprintCourseWatchEventKind,
    BlueprintCourseWatchEventsQuery, BlueprintCourseWatchProjection, BlueprintStewardshipStore,
    NumberedPageRequest, PageNumber, PageSize, SessionTokenHash, StoreError,
    postgres::{PostgresBlueprintStewardshipStore, PostgresSessionStore},
};
use question_model::{BlueprintCourseId, UserRole};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthError, resolve_session};

const MAX_STEWARDSHIP_UPDATE_BYTES: usize = 128;
const DEFAULT_WATCH_EVENT_PAGE_SIZE: u16 = 50;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    stewardship: PostgresBlueprintStewardshipStore,
}

/// Registers closed Blueprint Star and self-only Watch routes.
pub fn blueprint_stewardship_router(
    sessions: Arc<PostgresSessionStore>,
    stewardship: PostgresBlueprintStewardshipStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-blueprints/{blueprint_course_id}/stewardship/star",
            get(read_star).put(set_star),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/stewardship/starred-instructors",
            get(read_starred_instructors),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/stewardship/watch",
            get(read_watch).put(set_watch),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/stewardship/watch-events",
            get(read_watch_events),
        )
        .with_state(RouteState {
            sessions,
            stewardship,
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetStarInput {
    starred: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetWatchInput {
    watching: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StarResponse {
    star_count: u64,
    viewer_has_starred: bool,
}

impl From<BlueprintCourseStarProjection> for StarResponse {
    fn from(value: BlueprintCourseStarProjection) -> Self {
        Self {
            star_count: value.star_count,
            viewer_has_starred: value.viewer_has_starred,
        }
    }
}

/// C856's identity-only Star projection. It has no linked Account or Profile
/// identity and does not include aggregate or Watch facts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StarredInstructorsResponse {
    starred_instructors: Vec<StarredInstructorResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StarredInstructorResponse {
    display_name: String,
}

impl From<BlueprintCourseStarredInstructor> for StarredInstructorResponse {
    fn from(value: BlueprintCourseStarredInstructor) -> Self {
        Self {
            display_name: value.display_name,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchResponse {
    watching: bool,
}

impl From<BlueprintCourseWatchProjection> for WatchResponse {
    fn from(value: BlueprintCourseWatchProjection) -> Self {
        Self {
            watching: value.watching,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchEventsResponse {
    items: Vec<WatchEventResponse>,
    page_number: u32,
    page_size: u16,
    total_count: u64,
    event_kind_facets: Vec<WatchEventFacetResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchEventFacetResponse {
    value: WatchEventKindResponse,
    count: u64,
}

impl From<BlueprintCourseWatchEventFacet> for WatchEventFacetResponse {
    fn from(value: BlueprintCourseWatchEventFacet) -> Self {
        Self {
            value: value.value.into(),
            count: value.count,
        }
    }
}

/// The event payload is self-only and contains no actor, Watch, or Blueprint
/// identity beyond the route target.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchEventResponse {
    event_id: uuid::Uuid,
    kind: WatchEventKindResponse,
    occurred_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum WatchEventKindResponse {
    Revision,
    Archived,
    Restored,
}

impl From<BlueprintCourseWatchEventKind> for WatchEventKindResponse {
    fn from(value: BlueprintCourseWatchEventKind) -> Self {
        match value {
            BlueprintCourseWatchEventKind::Revision => Self::Revision,
            BlueprintCourseWatchEventKind::Archived => Self::Archived,
            BlueprintCourseWatchEventKind::Restored => Self::Restored,
        }
    }
}

impl From<BlueprintCourseWatchEvent> for WatchEventResponse {
    fn from(value: BlueprintCourseWatchEvent) -> Self {
        Self {
            event_id: value.event_id,
            kind: value.kind.into(),
            occurred_at: value.occurred_at.as_unix_millis(),
        }
    }
}

async fn read_star(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(raw_blueprint_course_id): Path<String>,
) -> Response {
    let blueprint_course_id = match blueprint_course_id(&raw_blueprint_course_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .blueprint_course_star_projection(session, blueprint_course_id)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(StarResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn set_star(
    State(state): State<RouteState>,
    Path(raw_blueprint_course_id): Path<String>,
    request: Request,
) -> Response {
    let blueprint_course_id = match blueprint_course_id(&raw_blueprint_course_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let input = match json_input::<SetStarInput>(request, "Blueprint Star").await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .set_current_blueprint_course_star_projection(session, blueprint_course_id, input.starred)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(StarResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn read_starred_instructors(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(raw_blueprint_course_id): Path<String>,
) -> Response {
    let blueprint_course_id = match blueprint_course_id(&raw_blueprint_course_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .blueprint_course_starred_instructors(session, blueprint_course_id)
        .await
    {
        Ok(starred_instructors) => crate::auth::no_store(
            Json(StarredInstructorsResponse {
                starred_instructors: starred_instructors.into_iter().map(Into::into).collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn read_watch(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(raw_blueprint_course_id): Path<String>,
) -> Response {
    let blueprint_course_id = match blueprint_course_id(&raw_blueprint_course_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .blueprint_course_watch_projection(session, blueprint_course_id)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(WatchResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn set_watch(
    State(state): State<RouteState>,
    Path(raw_blueprint_course_id): Path<String>,
    request: Request,
) -> Response {
    let blueprint_course_id = match blueprint_course_id(&raw_blueprint_course_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let input = match json_input::<SetWatchInput>(request, "Blueprint Watch").await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .set_current_blueprint_course_watch_projection(session, blueprint_course_id, input.watching)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(WatchResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn read_watch_events(
    State(state): State<RouteState>,
    Path(raw_blueprint_course_id): Path<String>,
    request: Request,
) -> Response {
    // ASVS 8.2.1/8.3.1: conceal the private route before parsing criteria.
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let blueprint_course_id = match blueprint_course_id(&raw_blueprint_course_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let query = match parse_watch_events_query(request.uri().query()) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .blueprint_course_watch_events(session, blueprint_course_id, query)
        .await
    {
        Ok(page) => crate::auth::no_store(
            Json(WatchEventsResponse {
                items: page.page.items.into_iter().map(Into::into).collect(),
                page_number: page.page.page_number.get(),
                page_size: page.page.page_size.get(),
                total_count: page.page.total_count,
                event_kind_facets: page.event_kind_facets.into_iter().map(Into::into).collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

fn parse_watch_events_query(
    raw_query: Option<&str>,
) -> Result<BlueprintCourseWatchEventsQuery, Box<Response>> {
    let mut query = BlueprintCourseWatchEventsQuery {
        search: String::new(),
        event_kinds: Vec::new(),
        direction: BlueprintCourseWatchDirection::Descending,
        page: NumberedPageRequest::first(
            PageSize::new(DEFAULT_WATCH_EVENT_PAGE_SIZE).expect("valid default page size"),
        ),
    };
    let Some(raw) = raw_query.filter(|value| !value.is_empty()) else {
        return Ok(query);
    };
    // ASVS 2.2.1/2.2.2: bound and allowlist the complete request at this boundary.
    if raw.len() > 4096 || raw.split('&').any(str::is_empty) || !has_valid_percent_encoding(raw) {
        return Err(Box::new(invalid("Blueprint Watch events")));
    }
    let mut seen = std::collections::HashSet::new();
    for (name, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        match name.as_ref() {
            "q" if seen.insert("q") => {
                query.search = value.trim().to_lowercase();
                if query.search.chars().count() > 120 || query.search.chars().any(char::is_control)
                {
                    return Err(Box::new(invalid("Blueprint Watch events")));
                }
            }
            "eventKind" => query.event_kinds.push(match value.as_ref() {
                "revision" => BlueprintCourseWatchEventKind::Revision,
                "archived" => BlueprintCourseWatchEventKind::Archived,
                "restored" => BlueprintCourseWatchEventKind::Restored,
                _ => return Err(Box::new(invalid("Blueprint Watch events"))),
            }),
            "direction" if seen.insert("direction") => {
                query.direction = match value.as_ref() {
                    "ascending" => BlueprintCourseWatchDirection::Ascending,
                    "descending" => BlueprintCourseWatchDirection::Descending,
                    _ => return Err(Box::new(invalid("Blueprint Watch events"))),
                }
            }
            "pageNumber" if seen.insert("pageNumber") => {
                query.page.page_number = value
                    .parse::<u32>()
                    .ok()
                    .and_then(|value| PageNumber::new(value).ok())
                    .ok_or_else(|| Box::new(invalid("Blueprint Watch events")))?;
            }
            "pageSize" if seen.insert("pageSize") => {
                query.page.page_size = value
                    .parse::<u16>()
                    .ok()
                    .and_then(|value| PageSize::new(value).ok())
                    .ok_or_else(|| Box::new(invalid("Blueprint Watch events")))?;
            }
            _ => return Err(Box::new(invalid("Blueprint Watch events"))),
        }
    }
    query.event_kinds.sort_by_key(|kind| match kind {
        BlueprintCourseWatchEventKind::Revision => 0,
        BlueprintCourseWatchEventKind::Archived => 1,
        BlueprintCourseWatchEventKind::Restored => 2,
    });
    query.event_kinds.dedup();
    Ok(query)
}

fn has_valid_percent_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if bytes
                .get(index + 1..index + 3)
                .is_none_or(|pair| !pair.iter().all(u8::is_ascii_hexdigit))
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

/// ASVS 2.2.1--2.2.2: parsing occurs before Store access and admits only the
/// exact canonical Blueprint Course ID shape.
fn blueprint_course_id(value: &str) -> Option<BlueprintCourseId> {
    value.parse().ok()
}

async fn json_input<T: serde::de::DeserializeOwned>(
    request: Request,
    surface: &'static str,
) -> Result<T, Box<Response>> {
    if !has_content_type(request.headers(), "application/json") {
        return Err(Box::new(route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            surface,
            "is invalid",
        )));
    }
    let bytes = to_bytes(request.into_body(), MAX_STEWARDSHIP_UPDATE_BYTES)
        .await
        .map_err(|_| {
            Box::new(route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                surface,
                "is too large",
            ))
        })?;
    serde_json::from_slice(&bytes).map_err(|_| Box::new(invalid(surface)))
}

async fn instructor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        crate::auth::joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // ASVS 8.2.1--8.3.1: role derives only from the server session; the
        // Store independently enforces the active-Instructor predicate.
        Ok(session) if session.record.user_role == UserRole::Instructor => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Blueprint stewardship authentication",
            "is unavailable",
        ))),
    }
}

fn has_content_type(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(expected))
        })
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => invalid("Blueprint stewardship"),
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Blueprint stewardship",
            "changed",
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Blueprint stewardship", "conflict")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Blueprint stewardship",
            "is unavailable",
        ),
    }
}

fn invalid(surface: &'static str) -> Response {
    route_error(StatusCode::UNPROCESSABLE_ENTITY, surface, "is invalid")
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Blueprint Course", "not found")
}

fn route_error(status: StatusCode, subject: &'static str, predicate: &'static str) -> Response {
    crate::auth::no_store((status, format!("{subject} {predicate}")).into_response())
}
