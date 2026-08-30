use axum::Json;
use axum::extract::{Path, Query, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    CatalogStore, Cursor, PageRequest, PageSize, PaginationError, SessionStore, Store,
};
use question_model::{
    Capability, CatalogAuthorship, CatalogEvidenceAvailability, CatalogLicenseValue,
    CatalogResponseFamily, CatalogSearchQuery, CatalogTaxonomyFilter, CatalogUsedInMyCourses,
    ProblemDisplayRef, ProblemVersionRef, QuestionBackend, UserRole,
};
use serde::Deserialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

use super::routes::CatalogRouteState;
use super::{BackendRegistry, PublicReviewGate};
use super::{error_response, store_error_response};

const DEFAULT_PAGE_SIZE: u16 = 50;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogQuery {
    cursor: Option<String>,
    page_size: Option<u16>,
}

/// Query-string transport for strict catalog search. Repeated collection keys
/// keep URLs inspectable while the model receives typed exact filters only
/// after this boundary validates every browser value.
#[derive(Debug, Default)]
pub(super) struct CatalogSearchHttpQuery {
    text: Option<String>,
    bylines: Vec<String>,
    backends: Vec<QuestionBackend>,
    tags: Vec<String>,
    response_families: Vec<CatalogResponseFamily>,
    taxonomy: Vec<String>,
    capabilities: Vec<Capability>,
    licenses: Vec<CatalogLicenseValue>,
    evidence: Option<CatalogEvidenceAvailability>,
    used_in_my_courses: Option<CatalogUsedInMyCourses>,
    authorship: Option<CatalogAuthorship>,
    cursor: Option<String>,
    page_size: Option<u16>,
}

impl CatalogSearchHttpQuery {
    /// Parses the compact repeated-key browser transport without allowing an
    /// ambiguous scalar value to change the authenticated search meaning.
    fn from_raw_query(raw_query: Option<&str>) -> Result<Self, &'static str> {
        let mut query = Self::default();
        for (key, value) in url::form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
            let value = value.into_owned();
            match key.as_ref() {
                "text" => set_catalog_scalar(&mut query.text, value)?,
                "bylines" => query.bylines.push(value),
                "backends" => query.backends.push(parse_catalog_enum(
                    &value,
                    "catalog backend is not recognized",
                )?),
                "tags" => query.tags.push(value),
                "response_families" => query.response_families.push(parse_catalog_enum(
                    &value,
                    "catalog response family is not recognized",
                )?),
                "taxonomy" => query.taxonomy.push(value),
                "capabilities" => query.capabilities.push(parse_catalog_enum(
                    &value,
                    "catalog capability is not recognized",
                )?),
                "licenses" => query.licenses.push(parse_catalog_enum(
                    &value,
                    "catalog license is not recognized",
                )?),
                "evidence" => set_catalog_scalar(
                    &mut query.evidence,
                    parse_catalog_enum(&value, "catalog evidence availability is not recognized")?,
                )?,
                "used_in_my_courses" => set_catalog_scalar(
                    &mut query.used_in_my_courses,
                    parse_catalog_enum(&value, "catalog course-use filter is not recognized")?,
                )?,
                "authorship" => set_catalog_scalar(
                    &mut query.authorship,
                    parse_catalog_enum(&value, "catalog authorship scope is not recognized")?,
                )?,
                "cursor" => set_catalog_scalar(&mut query.cursor, value)?,
                "page_size" => set_catalog_scalar(
                    &mut query.page_size,
                    value
                        .parse::<u16>()
                        .map_err(|_| "catalog page_size must be an unsigned integer")?,
                )?,
                _ => return Err("catalog query contains an unknown key"),
            }
        }
        Ok(query)
    }
}

fn set_catalog_scalar<T>(slot: &mut Option<T>, value: T) -> Result<(), &'static str> {
    if slot.replace(value).is_some() {
        return Err("catalog query scalar key may appear only once");
    }
    Ok(())
}

/// Reads a closed browser value from its canonical Serde wire contract, so a
/// future enum variant is available here without a second match table.
fn parse_catalog_enum<T>(value: &str, error: &'static str) -> Result<T, &'static str>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(serde_json::Value::String(value.to_string())).map_err(|_| error)
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
            bylines: query.bylines,
            backends: query.backends,
            tags: query.tags,
            response_families: query.response_families,
            taxonomy,
            capabilities: query.capabilities,
            licenses: query.licenses,
            evidence: query.evidence.unwrap_or_default(),
            used_in_my_courses: query.used_in_my_courses.unwrap_or_default(),
            authorship: query.authorship.unwrap_or_default(),
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
    if !may_read_catalog(authenticated.record.subject.role()) {
        return error_response(StatusCode::FORBIDDEN, "catalog access is not authorized");
    }
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
    if !may_read_catalog(authenticated.record.subject.role()) {
        return error_response(StatusCode::FORBIDDEN, "catalog access is not authorized");
    }
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
    RawQuery(raw_query): RawQuery,
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
    if !may_read_catalog(authenticated.record.subject.role()) {
        return error_response(StatusCode::FORBIDDEN, "catalog access is not authorized");
    }
    let query = match CatalogSearchHttpQuery::from_raw_query(raw_query.as_deref())
        .and_then(CatalogSearchQuery::try_from)
    {
        Ok(query) => query,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, error),
    };
    match state
        .store
        .search_catalog(
            authenticated.tenant_context,
            authenticated.session_hash,
            query,
        )
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
    if !may_read_catalog(authenticated.record.subject.role()) {
        return error_response(StatusCode::FORBIDDEN, "catalog access is not authorized");
    }
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
    if !may_read_catalog(authenticated.record.subject.role()) {
        return error_response(StatusCode::FORBIDDEN, "catalog access is not authorized");
    }
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
            authenticated.session_hash,
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

fn may_read_catalog(role: UserRole) -> bool {
    matches!(role, UserRole::Instructor | UserRole::Sysadmin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_scope_accepts_only_its_closed_browser_value() {
        let query =
            CatalogSearchHttpQuery::from_raw_query(Some("authorship=authoredByCurrentActor"))
                .and_then(CatalogSearchQuery::try_from)
                .expect("closed authorship scope parses");
        assert_eq!(query.authorship, CatalogAuthorship::AuthoredByCurrentActor);
        assert!(
            CatalogSearchHttpQuery::from_raw_query(Some("authorship=other-user"))
                .and_then(CatalogSearchQuery::try_from)
                .is_err()
        );
        assert!(
            CatalogSearchHttpQuery::from_raw_query(Some(
                "authorship=any&authorship=authoredByCurrentActor",
            ))
            .is_err()
        );
    }

    #[test]
    fn search_query_uses_canonical_snake_case_and_rejects_retired_or_camel_case_keys() {
        let query = CatalogSearchHttpQuery::from_raw_query(Some(
            "response_families=numeric&used_in_my_courses=any&page_size=1",
        ))
        .and_then(CatalogSearchQuery::try_from)
        .expect("canonical search query parses");
        assert_eq!(
            query.response_families,
            vec![CatalogResponseFamily::Numeric]
        );
        assert_eq!(query.used_in_my_courses, CatalogUsedInMyCourses::Any);
        assert_eq!(query.page_size, Some(1));
        for legacy_key in [
            "publicationScopes=public",
            "publication_scopes=public",
            "responseFamilies=numeric",
            "usedInMyCourses=any",
            "pageSize=1",
        ] {
            assert!(CatalogSearchHttpQuery::from_raw_query(Some(legacy_key)).is_err());
        }
    }
}
