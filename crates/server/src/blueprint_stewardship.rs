//! Vetted-Instructor Blueprint Course stewardship HTTP boundary.
//!
//! This surface deliberately separates public Star aggregate facts from the
//! caller's private Watch state. It exposes no Account identity, email,
//! reference, avatar, or another Instructor's Watch state.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    BlueprintCourseStarProjection, BlueprintCourseStarredInstructor, BlueprintCourseWatchEvent,
    BlueprintCourseWatchEventKind, BlueprintCourseWatchProjection, BlueprintStewardshipStore,
    SessionTokenHash, StoreError,
    postgres::{PostgresBlueprintStewardshipStore, PostgresSessionStore},
};
use question_model::{BlueprintCourseId, ProductRole};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthError, resolve_session};

const MAX_STEWARDSHIP_UPDATE_BYTES: usize = 128;
const DEFAULT_WATCH_EVENT_LIMIT: u16 = 25;
const MAX_WATCH_EVENT_LIMIT: u16 = 100;

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
            "/api/course-blueprints/{reference}/stewardship/star",
            get(read_star).put(set_star),
        )
        .route(
            "/api/course-blueprints/{reference}/stewardship/starred-instructors",
            get(read_starred_instructors),
        )
        .route(
            "/api/course-blueprints/{reference}/stewardship/watch",
            get(read_watch).put(set_watch),
        )
        .route(
            "/api/course-blueprints/{reference}/stewardship/watch-events",
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WatchEventsQuery {
    limit: Option<u16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchEventsResponse {
    events: Vec<WatchEventResponse>,
}

/// The event payload is self-only and contains no actor, Watch, or Blueprint
/// identity beyond the route target.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchEventResponse {
    kind: WatchEventKindResponse,
    occurred_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum WatchEventKindResponse {
    Revision,
    Published,
    Archived,
    Restored,
}

impl From<BlueprintCourseWatchEventKind> for WatchEventKindResponse {
    fn from(value: BlueprintCourseWatchEventKind) -> Self {
        match value {
            BlueprintCourseWatchEventKind::Revision => Self::Revision,
            BlueprintCourseWatchEventKind::Published => Self::Published,
            BlueprintCourseWatchEventKind::Archived => Self::Archived,
            BlueprintCourseWatchEventKind::Restored => Self::Restored,
        }
    }
}

impl From<BlueprintCourseWatchEvent> for WatchEventResponse {
    fn from(value: BlueprintCourseWatchEvent) -> Self {
        Self {
            kind: value.kind.into(),
            occurred_at: value.occurred_at.as_unix_millis(),
        }
    }
}

async fn read_star(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(raw_reference): Path<String>,
) -> Response {
    let reference = match blueprint_reference(&raw_reference) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .blueprint_course_star_projection(session, reference)
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
    Path(raw_reference): Path<String>,
    request: Request,
) -> Response {
    let reference = match blueprint_reference(&raw_reference) {
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
        .set_current_blueprint_course_star_projection(session, reference, input.starred)
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
    Path(raw_reference): Path<String>,
) -> Response {
    let reference = match blueprint_reference(&raw_reference) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .blueprint_course_starred_instructors(session, reference)
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
    Path(raw_reference): Path<String>,
) -> Response {
    let reference = match blueprint_reference(&raw_reference) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .blueprint_course_watch_projection(session, reference)
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
    Path(raw_reference): Path<String>,
    request: Request,
) -> Response {
    let reference = match blueprint_reference(&raw_reference) {
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
        .set_current_blueprint_course_watch_projection(session, reference, input.watching)
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
    headers: HeaderMap,
    Path(raw_reference): Path<String>,
    Query(query): Query<WatchEventsQuery>,
) -> Response {
    let reference = match blueprint_reference(&raw_reference) {
        Some(value) => value,
        None => return concealed(),
    };
    let limit = match query.limit {
        Some(value) if value == 0 || value > MAX_WATCH_EVENT_LIMIT => {
            return invalid("Blueprint Watch events");
        }
        Some(value) => value,
        None => DEFAULT_WATCH_EVENT_LIMIT,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stewardship
        .blueprint_course_watch_events(session, reference, limit)
        .await
    {
        Ok(events) => crate::auth::no_store(
            Json(WatchEventsResponse {
                events: events.into_iter().map(Into::into).collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

/// ASVS 2.2.1--2.2.2: parsing occurs before Store access and admits only the
/// exact canonical Blueprint Course reference shape.
fn blueprint_reference(value: &str) -> Option<BlueprintCourseId> {
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
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // ASVS 8.2.1--8.3.1: role derives only from the server session; the
        // Store independently enforces an active vetted-Instructor predicate.
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Blueprint stewardship authentication",
            "is unavailable",
        ))),
    }
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
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
