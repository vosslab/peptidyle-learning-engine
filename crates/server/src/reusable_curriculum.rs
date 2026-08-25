//! Authenticated routes for private course Blueprints and shared Alpha curricula.
//!
//! These handlers deliberately decode all protected request material only after
//! the Store has derived the active session's approved-Instructor authority.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, RawQuery, Request, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use learning_data_access::{
    Cursor, PageRequest, PageSize, ReplaceAlphaCourseCommand, ReplaceBlueprintCommand,
    ReusableCurriculumCapability, ReusableCurriculumStore, SessionStore, StoreError,
};
use question_model::{
    AlphaCourseDefinitionInput, AlphaCourseReference, AlphaCourseRevision, AlphaCourseView,
    BlueprintDefinitionInput, BlueprintReference, BlueprintRevision, BlueprintView,
};

use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};
use crate::http_refusal::{HttpRefusal, HttpResult};

const MAX_REUSABLE_CURRICULUM_BODY_BYTES: usize = 64 * 1024;
const DEFAULT_PAGE_SIZE: u16 = 50;

/// Builds the authenticated reusable-curriculum route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    Router::new()
        .route(
            "/api/course-blueprints",
            get(list_blueprints::<S>).post(create_blueprint::<S>),
        )
        .route(
            "/api/course-blueprints/{blueprint}",
            get(get_blueprint::<S>)
                .put(replace_blueprint::<S>)
                .delete(delete_blueprint::<S>),
        )
        .route(
            "/api/alpha-courses",
            get(list_alpha_courses::<S>).post(create_alpha_course::<S>),
        )
        .route(
            "/api/alpha-courses/{alpha}",
            get(get_alpha_course::<S>).put(replace_alpha_course::<S>),
        )
        .with_state(ReusableCurriculumRouteState { store })
}

struct ReusableCurriculumRouteState<S> {
    store: Arc<S>,
}

impl<S> Clone for ReusableCurriculumRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

async fn require_capability<S>(
    state: &ReusableCurriculumRouteState<S>,
    authenticated: &AuthenticatedSession,
    capability: ReusableCurriculumCapability,
) -> HttpResult<()>
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    state
        .store
        .preflight_reusable_curriculum(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            capability,
        )
        .await
        .map_err(|error| HttpRefusal::from(authority_error(error)))
}

async fn list_blueprints<S>(
    State(state): State<ReusableCurriculumRouteState<S>>,
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    let authenticated = match authenticate(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_capability(
        &state,
        &authenticated,
        ReusableCurriculumCapability::BlueprintPersonal,
    )
    .await
    {
        return response.into_response();
    }
    let page = match page_request(raw_query.as_deref()) {
        Ok(value) => value,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    match state
        .store
        .list_blueprints(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            page,
        )
        .await
    {
        Ok(value) => no_store(Json(value).into_response()),
        Err(error) => store_error(error),
    }
}

async fn create_blueprint<S>(
    State(state): State<ReusableCurriculumRouteState<S>>,
    request: Request,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    let headers = request.headers().clone();
    let authenticated = match authenticate(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_capability(
        &state,
        &authenticated,
        ReusableCurriculumCapability::BlueprintPersonal,
    )
    .await
    {
        return response.into_response();
    }
    let definition = match strict_json_body(request).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    replace_blueprint_command(
        &state,
        authenticated,
        None,
        None,
        definition,
        StatusCode::CREATED,
    )
    .await
}

async fn get_blueprint<S>(
    State(state): State<ReusableCurriculumRouteState<S>>,
    headers: HeaderMap,
    Path(raw_reference): Path<String>,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    let authenticated = match authenticate(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_capability(
        &state,
        &authenticated,
        ReusableCurriculumCapability::BlueprintPersonal,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<BlueprintReference>() {
        Ok(value) => value,
        Err(_) => return record_not_found(),
    };
    match state
        .store
        .get_blueprint(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            reference,
        )
        .await
    {
        Ok(Some(value)) => blueprint_response(StatusCode::OK, value),
        Ok(None)
        | Err(StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden) => {
            record_not_found()
        }
        Err(error) => store_error(error),
    }
}

async fn replace_blueprint<S>(
    State(state): State<ReusableCurriculumRouteState<S>>,
    Path(raw_reference): Path<String>,
    request: Request,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    let headers = request.headers().clone();
    let authenticated = match authenticate(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_capability(
        &state,
        &authenticated,
        ReusableCurriculumCapability::BlueprintPersonal,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<BlueprintReference>() {
        Ok(value) => value,
        Err(_) => return record_not_found(),
    };
    let expected_revision = match required_revision::<BlueprintRevision>(request.headers()) {
        Ok(value) => value,
        Err(error) => return required_revision_error(error),
    };
    let definition = match strict_json_body(request).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    replace_blueprint_command(
        &state,
        authenticated,
        Some(reference),
        Some(expected_revision),
        definition,
        StatusCode::OK,
    )
    .await
}

async fn replace_blueprint_command<S>(
    state: &ReusableCurriculumRouteState<S>,
    authenticated: AuthenticatedSession,
    reference: Option<BlueprintReference>,
    expected_revision: Option<BlueprintRevision>,
    definition: BlueprintDefinitionInput,
    status: StatusCode,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    match state
        .store
        .replace_blueprint(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            ReplaceBlueprintCommand {
                reference,
                expected_revision,
                definition,
            },
        )
        .await
    {
        Ok(value) => blueprint_response(status, value),
        Err(error) => store_error(error),
    }
}

async fn delete_blueprint<S>(
    State(state): State<ReusableCurriculumRouteState<S>>,
    Path(raw_reference): Path<String>,
    request: Request,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    let headers = request.headers().clone();
    let authenticated = match authenticate(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_capability(
        &state,
        &authenticated,
        ReusableCurriculumCapability::BlueprintPersonal,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<BlueprintReference>() {
        Ok(value) => value,
        Err(_) => return record_not_found(),
    };
    let expected_revision = match required_revision::<BlueprintRevision>(request.headers()) {
        Ok(value) => value,
        Err(error) => return required_revision_error(error),
    };
    match state
        .store
        .delete_blueprint(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            reference,
            expected_revision,
        )
        .await
    {
        Ok(true) => no_store(StatusCode::NO_CONTENT.into_response()),
        Ok(false)
        | Err(StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden) => {
            record_not_found()
        }
        Err(error) => store_error(error),
    }
}

async fn list_alpha_courses<S>(
    State(state): State<ReusableCurriculumRouteState<S>>,
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    let authenticated = match authenticate(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_capability(
        &state,
        &authenticated,
        ReusableCurriculumCapability::AlphaRead,
    )
    .await
    {
        return response.into_response();
    }
    let page = match page_request(raw_query.as_deref()) {
        Ok(value) => value,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    match state
        .store
        .list_alpha_courses(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            page,
        )
        .await
    {
        Ok(value) => no_store(Json(value).into_response()),
        Err(error) => store_error(error),
    }
}

async fn create_alpha_course<S>(
    State(state): State<ReusableCurriculumRouteState<S>>,
    request: Request,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    let headers = request.headers().clone();
    let authenticated = match authenticate(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_capability(
        &state,
        &authenticated,
        ReusableCurriculumCapability::AlphaCreatorWrite,
    )
    .await
    {
        return response.into_response();
    }
    let definition = match strict_json_body(request).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    replace_alpha_course_command(
        &state,
        authenticated,
        None,
        None,
        definition,
        StatusCode::CREATED,
    )
    .await
}

async fn get_alpha_course<S>(
    State(state): State<ReusableCurriculumRouteState<S>>,
    headers: HeaderMap,
    Path(raw_reference): Path<String>,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    let authenticated = match authenticate(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_capability(
        &state,
        &authenticated,
        ReusableCurriculumCapability::AlphaRead,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<AlphaCourseReference>() {
        Ok(value) => value,
        Err(_) => return record_not_found(),
    };
    match state
        .store
        .get_alpha_course(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            reference,
        )
        .await
    {
        Ok(Some(value)) => alpha_course_response(StatusCode::OK, value),
        Ok(None)
        | Err(StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden) => {
            record_not_found()
        }
        Err(error) => store_error(error),
    }
}

async fn replace_alpha_course<S>(
    State(state): State<ReusableCurriculumRouteState<S>>,
    Path(raw_reference): Path<String>,
    request: Request,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    let headers = request.headers().clone();
    let authenticated = match authenticate(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_capability(
        &state,
        &authenticated,
        ReusableCurriculumCapability::AlphaCreatorWrite,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<AlphaCourseReference>() {
        Ok(value) => value,
        Err(_) => return record_not_found(),
    };
    let expected_revision = match required_revision::<AlphaCourseRevision>(request.headers()) {
        Ok(value) => value,
        Err(error) => return required_revision_error(error),
    };
    let definition = match strict_json_body(request).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    replace_alpha_course_command(
        &state,
        authenticated,
        Some(reference),
        Some(expected_revision),
        definition,
        StatusCode::OK,
    )
    .await
}

async fn replace_alpha_course_command<S>(
    state: &ReusableCurriculumRouteState<S>,
    authenticated: AuthenticatedSession,
    reference: Option<AlphaCourseReference>,
    expected_revision: Option<AlphaCourseRevision>,
    definition: AlphaCourseDefinitionInput,
    status: StatusCode,
) -> Response
where
    S: ReusableCurriculumStore + SessionStore + 'static,
{
    match state
        .store
        .replace_alpha_course(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            ReplaceAlphaCourseCommand {
                reference,
                expected_revision,
                definition,
            },
        )
        .await
    {
        Ok(value) => alpha_course_response(status, value),
        Err(error) => store_error(error),
    }
}

async fn authenticate<S>(store: &S, headers: &HeaderMap) -> HttpResult<AuthenticatedSession>
where
    S: SessionStore,
{
    resolve_request_session(store, headers)
        .await
        .map_err(|error| HttpRefusal::from(auth_error_response(error)))
}

async fn strict_json_body<T>(request: Request) -> HttpResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let bytes = to_bytes(request.into_body(), MAX_REUSABLE_CURRICULUM_BODY_BYTES)
        .await
        .map_err(|_| {
            HttpRefusal::from(error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "curriculum request is too large",
            ))
        })?;
    serde_json::from_slice(&bytes).map_err(|_| {
        HttpRefusal::from(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "curriculum request must use the documented JSON shape",
        ))
    })
}

fn page_request(raw_query: Option<&str>) -> Result<PageRequest, &'static str> {
    let mut cursor = None;
    let mut page_size = None;
    for (key, value) in url::form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
        let value = value.into_owned();
        match key.as_ref() {
            "cursor" => {
                if cursor.replace(value).is_some() {
                    return Err("pagination keys may appear only once");
                }
            }
            "pageSize" => {
                if page_size.replace(value).is_some() {
                    return Err("pagination keys may appear only once");
                }
            }
            _ => return Err("pagination query contains an unknown key"),
        }
    }
    let size = page_size
        .map(|value| {
            value
                .parse::<u16>()
                .map_err(|_| "pageSize must be an unsigned integer")
        })
        .transpose()?
        .unwrap_or(DEFAULT_PAGE_SIZE);
    let size = PageSize::new(size).map_err(|_| "pageSize must be between 1 and 100")?;
    match cursor {
        Some(value) => Cursor::parse(value)
            .map(|value| PageRequest::after(value, size))
            .map_err(|_| "cursor must not be empty"),
        None => Ok(PageRequest::first(size)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevisionHeaderError {
    Missing,
    Malformed,
}

fn required_revision<T>(headers: &HeaderMap) -> Result<T, RevisionHeaderError>
where
    T: FromStr,
{
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(RevisionHeaderError::Missing);
    };
    if values.next().is_some() {
        return Err(RevisionHeaderError::Malformed);
    }
    let value = value.to_str().map_err(|_| RevisionHeaderError::Malformed)?;
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or(RevisionHeaderError::Malformed)?
        .parse()
        .map_err(|_| RevisionHeaderError::Malformed)
}

fn required_revision_error(error: RevisionHeaderError) -> Response {
    match error {
        RevisionHeaderError::Missing => error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "If-Match curriculum revision is required",
        ),
        RevisionHeaderError::Malformed => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "If-Match curriculum revision is invalid",
        ),
    }
}

fn blueprint_response(status: StatusCode, value: BlueprintView) -> Response {
    revisioned_response(status, value.revision.value(), value)
}

fn alpha_course_response(status: StatusCode, value: AlphaCourseView) -> Response {
    revisioned_response(status, value.revision.value(), value)
}

fn revisioned_response<T>(status: StatusCode, revision: u64, value: T) -> Response
where
    T: serde::Serialize,
{
    let etag = HeaderValue::from_str(&format!("\"{revision}\""))
        .expect("positive curriculum revision is a valid ETag");
    let mut response = (status, Json(value)).into_response();
    response.headers_mut().insert(ETAG, etag);
    no_store(response)
}

fn authority_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::FORBIDDEN, "curriculum is not authorized")
        }
        StoreError::RetryableTransaction | StoreError::TimedOut | StoreError::Unavailable(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "curriculum is unavailable")
        }
        StoreError::AlreadyExists
        | StoreError::Conflict
        | StoreError::InvalidRecord(_)
        | StoreError::RunModel(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "curriculum is unavailable")
        }
    }
}

fn store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden => {
            record_not_found()
        }
        StoreError::Conflict => error_response(
            StatusCode::PRECONDITION_FAILED,
            "curriculum record changed; reload it",
        ),
        StoreError::AlreadyExists => {
            error_response(StatusCode::CONFLICT, "curriculum title already exists")
        }
        StoreError::InvalidRecord(_) | StoreError::RunModel(_) => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "curriculum request is invalid",
        ),
        StoreError::TimedOut => {
            error_response(StatusCode::CONFLICT, "curriculum operation timed out")
        }
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "curriculum is unavailable")
        }
    }
}

fn record_not_found() -> Response {
    error_response(StatusCode::NOT_FOUND, "curriculum record not found")
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_rejects_ambiguous_or_unbounded_values() {
        assert!(page_request(Some("cursor=one&cursor=two")).is_err());
        assert!(page_request(Some("pageSize=101")).is_err());
        assert!(page_request(Some("unknown=value")).is_err());
        assert!(page_request(Some("pageSize=50")).is_ok());
    }

    #[test]
    fn revision_header_requires_one_strong_positive_etag() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_MATCH, HeaderValue::from_static("\"7\""));
        assert_eq!(
            required_revision::<BlueprintRevision>(&headers)
                .expect("revision")
                .value(),
            7
        );
        headers.insert(IF_MATCH, HeaderValue::from_static("7"));
        assert!(matches!(
            required_revision::<BlueprintRevision>(&headers),
            Err(RevisionHeaderError::Malformed)
        ));
    }

    #[test]
    fn request_definitions_reject_unknown_fields() {
        assert!(serde_json::from_str::<BlueprintDefinitionInput>(
            r#"{"definition":{"title":"x","instructions":"x","entries":[],"defaults":{},"schedule":{}},"surprise":true}"#
        )
        .is_err());
        assert!(
            serde_json::from_str::<AlphaCourseDefinitionInput>(
                r#"{"title":"x","modules":[],"surprise":true}"#
            )
            .is_err()
        );
    }

    #[test]
    fn revisioned_responses_use_strong_decimal_etags_and_no_store() {
        let response = revisioned_response(StatusCode::OK, 7, serde_json::json!({"ok": true}));
        assert_eq!(
            response
                .headers()
                .get(ETAG)
                .and_then(|value| value.to_str().ok()),
            Some("\"7\"")
        );
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
    }
}
