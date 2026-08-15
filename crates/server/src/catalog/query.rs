use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    CatalogStore, Cursor, PageRequest, PageSize, PaginationError, SessionStore, Store,
};
use question_model::{
    Capability, CatalogLicenseValue, CatalogSearchQuery, CatalogStatisticsAvailability,
    CatalogTaxonomyFilter, ProblemDisplayRef, ProblemVersionRef,
};
use serde::Deserialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

use super::routes::CatalogRouteState;
use super::{BackendRegistry, PublicReviewGate};
use super::{error_response, store_error_response};

const DEFAULT_PAGE_SIZE: u16 = 50;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogQuery {
    cursor: Option<String>,
    page_size: Option<u16>,
}

/// Query-string transport for strict catalog search. Repeated scalar keys keep
/// URLs inspectable (`taxonomy=scheme:code&capabilities=serverGrading`) while
/// the model receives typed exact filters after this boundary validates them.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CatalogSearchHttpQuery {
    text: Option<String>,
    #[serde(default)]
    taxonomy: Vec<String>,
    #[serde(default)]
    capabilities: Vec<Capability>,
    #[serde(default)]
    licenses: Vec<CatalogLicenseValue>,
    #[serde(default)]
    statistics: CatalogStatisticsAvailability,
    cursor: Option<String>,
    page_size: Option<u16>,
}

impl TryFrom<CatalogSearchHttpQuery> for CatalogSearchQuery {
    type Error = &'static str;

    fn try_from(query: CatalogSearchHttpQuery) -> Result<Self, Self::Error> {
        let taxonomy = query
            .taxonomy
            .into_iter()
            .map(|value| {
                let (scheme, code) = value
                    .split_once(':')
                    .ok_or("taxonomy filter must be scheme:code")?;
                Ok::<_, &'static str>(CatalogTaxonomyFilter {
                    scheme: scheme.to_string(),
                    code: code.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CatalogSearchQuery {
            text: query.text,
            taxonomy,
            capabilities: query.capabilities,
            licenses: query.licenses,
            statistics: query.statistics,
            cursor: query.cursor,
            page_size: query.page_size,
        })
    }
}

pub(super) async fn list_problems<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Query(query): Query<CatalogQuery>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    match state
        .store
        .list_catalog(authenticated.tenant_context, page)
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn list_taxonomy<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Query(query): Query<CatalogQuery>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    match state
        .store
        .list_catalog_taxonomy(authenticated.tenant_context, page)
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(error) => store_error_response(error),
    }
}

/// Searches only hot catalog metadata. The store owns normalized-query cursor
/// binding and aggregate computation; this HTTP layer only authenticates and
/// ensures every browser response is non-cacheable.
pub(super) async fn search_problems<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Query(query): Query<CatalogSearchHttpQuery>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let query = match CatalogSearchQuery::try_from(query) {
        Ok(query) => query,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, error),
    };
    match state
        .store
        .search_catalog(authenticated.tenant_context, query)
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn resolve_problem_reference<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let reference = match reference.parse::<ProblemDisplayRef>() {
        Ok(reference) => reference,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, error),
    };
    match state
        .store
        .resolve_catalog_problem(authenticated.tenant_context, reference)
        .await
    {
        Ok(Some(record)) => no_store(Json(record.summary()).into_response()),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "problem reference not found"),
        Err(error) => store_error_response(error),
    }
}

/// Returns the exact safe catalog detail projection. It intentionally has a
/// separate path from the learner question-definition endpoint so neither a
/// source locator nor grading policy can leak into library browsing.
pub(super) async fn get_problem_detail<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let reference = match reference.parse::<ProblemDisplayRef>() {
        Ok(reference) => reference,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, error),
    };
    let publication = match state
        .store
        .resolve_catalog_problem(authenticated.tenant_context, reference)
        .await
    {
        Ok(Some(record)) => record,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "question ID not found"),
        Err(error) => return store_error_response(error),
    };
    match state
        .store
        .get_catalog_detail(
            authenticated.tenant_context,
            ProblemVersionRef {
                problem: publication.problem,
                version: publication.version,
            },
        )
        .await
    {
        Ok(Some(detail)) => no_store(Json(detail).into_response()),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "question ID not found"),
        Err(error) => store_error_response(error),
    }
}

fn page_request(query: CatalogQuery) -> Result<PageRequest, PaginationError> {
    let size = PageSize::new(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))?;
    match query.cursor {
        Some(cursor) => {
            let cursor = Cursor::parse(cursor)?;
            Ok(PageRequest::after(cursor, size))
        }
        None => Ok(PageRequest::first(size)),
    }
}
