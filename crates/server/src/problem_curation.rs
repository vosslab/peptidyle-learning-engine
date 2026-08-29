//! Authenticated Favorites, collections, and saved catalog-search routes.
//!
//! The browser supplies compact public references and Question IDs only.  The
//! store's session-derived broker owns actor identity, exact publication
//! resolution, and every revision-checked state transition.

use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, RawQuery, Request, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use learning_data_access::{
    Cursor, PageRequest, PageSize, ProblemCollectionReplacementTarget, ProblemCurationCapability,
    ProblemCurationStore, ReplaceProblemCollectionCommand, ReplaceSavedProblemSearchCommand,
    SessionStore, StoreError,
};
use question_model::{
    CatalogSearchFilter, ProblemCollectionReference, ProblemCollectionRevision,
    ProblemCollectionVisibility, QuestionId, SavedProblemSearchReference,
    SavedProblemSearchRevision,
};
use serde::Deserialize;

use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};
use crate::http_refusal::{HttpRefusal, HttpResult};

const MAX_CURATION_BODY_BYTES: usize = 64 * 1024;
const DEFAULT_PAGE_SIZE: u16 = 50;

/// Builds the authenticated D2 curation route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    Router::new()
        .route(
            "/api/problem-collections",
            get(list_collections::<S>).post(create_collection::<S>),
        )
        .route(
            "/api/problem-collections/favorites",
            post(create_favorites::<S>).put(replace_favorites::<S>),
        )
        .route(
            "/api/problem-collections/{collection}",
            get(get_collection::<S>)
                .put(replace_collection::<S>)
                .delete(delete_collection::<S>),
        )
        .route(
            "/api/problem-collections/{collection}/members",
            get(list_collection_members::<S>),
        )
        .route(
            "/api/saved-problem-searches",
            get(list_saved_searches::<S>).post(create_saved_search::<S>),
        )
        .route(
            "/api/saved-problem-searches/{search}",
            get(get_saved_search::<S>)
                .put(replace_saved_search::<S>)
                .delete(delete_saved_search::<S>),
        )
        .with_state(CurationRouteState { store })
}

struct CurationRouteState<S> {
    store: Arc<S>,
}

impl<S> Clone for CurationRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReplaceCollectionRequest {
    title: Option<String>,
    visibility: Option<ProblemCollectionVisibility>,
    question_ids: Vec<QuestionId>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplaceSavedSearchRequest {
    title: String,
    filter: CatalogSearchFilter,
}

async fn require_curation_capability<S>(
    state: &CurationRouteState<S>,
    authenticated: &AuthenticatedSession,
    capability: ProblemCurationCapability,
) -> HttpResult<()>
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    state
        .store
        .preflight_problem_curation(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            capability,
        )
        .await
        .map_err(|error| HttpRefusal::from(curation_authority_error(error)))
}

async fn list_collections<S>(
    State(state): State<CurationRouteState<S>>,
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::CatalogInstitutionRead,
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
        .list_problem_collections(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            page,
        )
        .await
    {
        Ok(value) => no_store(Json(value).into_response()),
        Err(error) => curation_store_error(error),
    }
}

async fn get_collection<S>(
    State(state): State<CurationRouteState<S>>,
    headers: HeaderMap,
    Path(raw_reference): Path<String>,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::CatalogInstitutionRead,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<ProblemCollectionReference>() {
        Ok(value) => value,
        Err(_) => return collection_not_found(),
    };
    match state
        .store
        .get_problem_collection_summary(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            reference,
        )
        .await
    {
        Ok(Some(value)) => collection_response(StatusCode::OK, value),
        Ok(None)
        | Err(StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden) => {
            collection_not_found()
        }
        Err(error) => curation_store_error(error),
    }
}

async fn list_collection_members<S>(
    State(state): State<CurationRouteState<S>>,
    headers: HeaderMap,
    Path(raw_reference): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::CatalogInstitutionRead,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<ProblemCollectionReference>() {
        Ok(value) => value,
        Err(_) => return collection_not_found(),
    };
    let page = match page_request(raw_query.as_deref()) {
        Ok(value) => value,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    match state
        .store
        .list_problem_collection_members(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            reference,
            page,
        )
        .await
    {
        Ok(Some(value)) => {
            collection_member_page_response(value.members, value.collection.revision)
        }
        Ok(None)
        | Err(StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden) => {
            collection_not_found()
        }
        Err(error) => curation_store_error(error),
    }
}

async fn create_collection<S>(
    State(state): State<CurationRouteState<S>>,
    request: Request,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::PersonalMutation,
    )
    .await
    {
        return response.into_response();
    }
    replace_collection_request(
        &state,
        authenticated,
        request,
        ProblemCollectionReplacementTarget::NewNamed,
        StatusCode::CREATED,
    )
    .await
}

async fn replace_favorites<S>(
    State(state): State<CurationRouteState<S>>,
    request: Request,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::PersonalMutation,
    )
    .await
    {
        return response.into_response();
    }
    replace_collection_request(
        &state,
        authenticated,
        request,
        ProblemCollectionReplacementTarget::Favorites,
        StatusCode::OK,
    )
    .await
}

async fn create_favorites<S>(
    State(state): State<CurationRouteState<S>>,
    headers: HeaderMap,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::PersonalMutation,
    )
    .await
    {
        return response.into_response();
    }
    match state
        .store
        .get_or_create_favorites(
            authenticated.tenant_context,
            authenticated.record.token_hash,
        )
        .await
    {
        Ok(value) => collection_response(StatusCode::OK, value),
        Err(error) => curation_store_error(error),
    }
}

async fn replace_collection<S>(
    State(state): State<CurationRouteState<S>>,
    Path(raw_reference): Path<String>,
    request: Request,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::PersonalMutation,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<ProblemCollectionReference>() {
        Ok(value) => value,
        Err(_) => return collection_not_found(),
    };
    replace_collection_request(
        &state,
        authenticated,
        request,
        ProblemCollectionReplacementTarget::Existing(reference),
        StatusCode::OK,
    )
    .await
}

async fn replace_collection_request<S>(
    state: &CurationRouteState<S>,
    authenticated: AuthenticatedSession,
    request: Request,
    target: ProblemCollectionReplacementTarget,
    success: StatusCode,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let expected_revision = match target {
        ProblemCollectionReplacementTarget::NewNamed => None,
        _ => match required_revision::<ProblemCollectionRevision>(request.headers()) {
            Ok(value) => Some(value),
            Err(RevisionHeaderError::Missing) => {
                return error_response(
                    StatusCode::PRECONDITION_REQUIRED,
                    "If-Match collection revision is required",
                );
            }
            Err(RevisionHeaderError::Malformed) => {
                return error_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "If-Match collection revision is invalid",
                );
            }
        },
    };
    let body = match strict_json_body::<ReplaceCollectionRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let command = ReplaceProblemCollectionCommand {
        target,
        expected_revision,
        title: body.title,
        visibility: body.visibility,
        question_ids: body.question_ids,
    };
    match state
        .store
        .replace_problem_collection(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            command,
        )
        .await
    {
        Ok(value) => collection_response(success, value),
        Err(error) => curation_store_error(error),
    }
}

async fn delete_collection<S>(
    State(state): State<CurationRouteState<S>>,
    Path(raw_reference): Path<String>,
    request: Request,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::PersonalMutation,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<ProblemCollectionReference>() {
        Ok(value) => value,
        Err(_) => return collection_not_found(),
    };
    let revision = match required_revision::<ProblemCollectionRevision>(request.headers()) {
        Ok(value) => value,
        Err(RevisionHeaderError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match collection revision is required",
            );
        }
        Err(RevisionHeaderError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match collection revision is invalid",
            );
        }
    };
    match state
        .store
        .delete_problem_collection(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            reference,
            revision,
        )
        .await
    {
        Ok(true) => no_store(StatusCode::NO_CONTENT.into_response()),
        Ok(false)
        | Err(StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden) => {
            collection_not_found()
        }
        Err(error) => curation_store_error(error),
    }
}

async fn list_saved_searches<S>(
    State(state): State<CurationRouteState<S>>,
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::PersonalMutation,
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
        .list_saved_problem_searches(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            page,
        )
        .await
    {
        Ok(value) => no_store(Json(value).into_response()),
        Err(error) => curation_store_error(error),
    }
}

async fn create_saved_search<S>(
    State(state): State<CurationRouteState<S>>,
    request: Request,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::PersonalMutation,
    )
    .await
    {
        return response.into_response();
    }
    replace_saved_search_request(&state, authenticated, request, None, StatusCode::CREATED).await
}

async fn get_saved_search<S>(
    State(state): State<CurationRouteState<S>>,
    headers: HeaderMap,
    Path(raw_reference): Path<String>,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::PersonalMutation,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<SavedProblemSearchReference>() {
        Ok(value) => value,
        Err(_) => return saved_search_not_found(),
    };
    match state
        .store
        .get_saved_problem_search(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            reference,
        )
        .await
    {
        Ok(Some(value)) => saved_search_response(StatusCode::OK, value),
        Ok(None)
        | Err(StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden) => {
            saved_search_not_found()
        }
        Err(error) => curation_store_error(error),
    }
}

async fn replace_saved_search<S>(
    State(state): State<CurationRouteState<S>>,
    Path(raw_reference): Path<String>,
    request: Request,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::PersonalMutation,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<SavedProblemSearchReference>() {
        Ok(value) => value,
        Err(_) => return saved_search_not_found(),
    };
    replace_saved_search_request(
        &state,
        authenticated,
        request,
        Some(reference),
        StatusCode::OK,
    )
    .await
}

async fn replace_saved_search_request<S>(
    state: &CurationRouteState<S>,
    authenticated: AuthenticatedSession,
    request: Request,
    reference: Option<SavedProblemSearchReference>,
    success: StatusCode,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let expected_revision = match reference {
        None => None,
        Some(_) => match required_revision::<SavedProblemSearchRevision>(request.headers()) {
            Ok(value) => Some(value),
            Err(RevisionHeaderError::Missing) => {
                return error_response(
                    StatusCode::PRECONDITION_REQUIRED,
                    "If-Match saved-search revision is required",
                );
            }
            Err(RevisionHeaderError::Malformed) => {
                return error_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "If-Match saved-search revision is invalid",
                );
            }
        },
    };
    let body = match strict_json_body::<ReplaceSavedSearchRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    match state
        .store
        .replace_saved_problem_search(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            ReplaceSavedProblemSearchCommand {
                reference,
                expected_revision,
                title: body.title,
                filter: body.filter,
            },
        )
        .await
    {
        Ok(value) => saved_search_response(success, value),
        Err(error) => curation_store_error(error),
    }
}

async fn delete_saved_search<S>(
    State(state): State<CurationRouteState<S>>,
    Path(raw_reference): Path<String>,
    request: Request,
) -> Response
where
    S: ProblemCurationStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_curation_capability(
        &state,
        &authenticated,
        ProblemCurationCapability::PersonalMutation,
    )
    .await
    {
        return response.into_response();
    }
    let reference = match raw_reference.parse::<SavedProblemSearchReference>() {
        Ok(value) => value,
        Err(_) => return saved_search_not_found(),
    };
    let revision = match required_revision::<SavedProblemSearchRevision>(request.headers()) {
        Ok(value) => value,
        Err(RevisionHeaderError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match saved-search revision is required",
            );
        }
        Err(RevisionHeaderError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match saved-search revision is invalid",
            );
        }
    };
    match state
        .store
        .delete_saved_problem_search(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            reference,
            revision,
        )
        .await
    {
        Ok(true) => no_store(StatusCode::NO_CONTENT.into_response()),
        Ok(false)
        | Err(StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden) => {
            saved_search_not_found()
        }
        Err(error) => curation_store_error(error),
    }
}

async fn strict_json_body<T: serde::de::DeserializeOwned>(request: Request) -> HttpResult<T> {
    let bytes = to_bytes(request.into_body(), MAX_CURATION_BODY_BYTES)
        .await
        .map_err(|_| {
            HttpRefusal::from(error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "curation request is too large",
            ))
        })?;
    serde_json::from_slice(&bytes).map_err(|_| {
        HttpRefusal::from(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "curation request must use the documented JSON shape",
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
    T: std::str::FromStr,
{
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(RevisionHeaderError::Missing);
    };
    if values.next().is_some() {
        return Err(RevisionHeaderError::Malformed);
    }
    let value = value.to_str().map_err(|_| RevisionHeaderError::Malformed)?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(RevisionHeaderError::Malformed);
    };
    value
        .parse::<T>()
        .map_err(|_| RevisionHeaderError::Malformed)
}

fn collection_response(
    status: StatusCode,
    value: question_model::ProblemCollectionSummaryView,
) -> Response {
    let etag = HeaderValue::from_str(&format!("\"{}\"", value.revision.value()))
        .expect("positive revision is a valid ETag");
    let mut response = (status, Json(value)).into_response();
    response.headers_mut().insert(ETAG, etag);
    no_store(response)
}

fn saved_search_response(
    status: StatusCode,
    value: question_model::SavedProblemSearchView,
) -> Response {
    let etag = HeaderValue::from_str(&format!("\"{}\"", value.revision.value()))
        .expect("positive revision is a valid ETag");
    let mut response = (status, Json(value)).into_response();
    response.headers_mut().insert(ETAG, etag);
    no_store(response)
}

fn collection_member_page_response(
    value: learning_data_access::Page<question_model::ProblemCollectionMemberView>,
    revision: ProblemCollectionRevision,
) -> Response {
    let etag = HeaderValue::from_str(&format!("\"{}\"", revision.value()))
        .expect("positive revision is a valid ETag");
    let mut response = Json(value).into_response();
    response.headers_mut().insert(ETAG, etag);
    no_store(response)
}

fn collection_not_found() -> Response {
    error_response(StatusCode::NOT_FOUND, "problem collection not found")
}
fn saved_search_not_found() -> Response {
    error_response(StatusCode::NOT_FOUND, "saved problem search not found")
}

fn curation_authority_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::FORBIDDEN, "problem curation is not authorized")
        }
        StoreError::RetryableTransaction | StoreError::TimedOut | StoreError::Unavailable(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "curation is unavailable")
        }
        StoreError::AlreadyExists
        | StoreError::Conflict
        | StoreError::InvalidRecord(_)
        | StoreError::RunModel(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "curation is unavailable")
        }
    }
}

fn curation_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::NOT_FOUND, "curation record not found")
        }
        StoreError::Conflict => error_response(
            StatusCode::PRECONDITION_FAILED,
            "curation record changed; reload it",
        ),
        StoreError::AlreadyExists => {
            error_response(StatusCode::CONFLICT, "curation title already exists")
        }
        StoreError::InvalidRecord(_) => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "curation request is invalid",
        ),
        StoreError::RunModel(error) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
        }
        StoreError::TimedOut => {
            error_response(StatusCode::CONFLICT, "curation operation timed out")
        }
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "curation is unavailable")
        }
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curation_pagination_rejects_ambiguous_and_unbounded_values() {
        assert!(page_request(Some("cursor=one&cursor=two")).is_err());
        assert!(page_request(Some("pageSize=101")).is_err());
        assert!(page_request(Some("unknown=value")).is_err());
        assert!(page_request(Some("pageSize=50")).is_ok());
    }

    #[test]
    fn revision_header_is_one_strong_positive_etag() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_MATCH, HeaderValue::from_static("\"7\""));
        assert_eq!(
            required_revision::<ProblemCollectionRevision>(&headers)
                .expect("revision")
                .value(),
            7
        );
        headers.insert(IF_MATCH, HeaderValue::from_static("7"));
        assert!(matches!(
            required_revision::<ProblemCollectionRevision>(&headers),
            Err(RevisionHeaderError::Malformed)
        ));
    }

    #[test]
    fn replacement_shapes_reject_unknown_fields() {
        assert!(
            serde_json::from_str::<ReplaceCollectionRequest>(
                r#"{"questionIds":[],"surprise":true}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<ReplaceSavedSearchRequest>(
                r#"{"title":"x","filter":{},"cursor":"bad"}"#
            )
            .is_err()
        );
        for legacy_filter in [
            r#"{"title":"x","filter":{"publication_scopes":[]}}"#,
            r#"{"title":"x","filter":{"publicationScopes":[]}}"#,
            r#"{"title":"x","filter":{"responseFamilies":[]}}"#,
            r#"{"title":"x","filter":{"usedInMyCourses":"any"}}"#,
        ] {
            assert!(
                serde_json::from_str::<ReplaceSavedSearchRequest>(legacy_filter).is_err(),
                "legacy saved-search filter must be refused: {legacy_filter}"
            );
        }
    }
}
