//! Authenticated catalog, publication, and lifecycle routes (MOD-API-CAT).
//!
//! The route layer resolves capabilities from a server-owned registry, runs
//! the shared domain validator, and passes the exact validated draft into one
//! atomic store transaction. A browser never declares its own capabilities or
//! chooses a new `ProblemId`.

use std::sync::Arc;

#[cfg(test)]
use std::cell::Cell;

use async_trait::async_trait;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::header::IF_MATCH;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use domain::policy::PublicationViolation;
use learning_data_access::{
    CatalogStore, CatalogTransition, Cursor, DraftRecord, PageRequest, PageSize, PaginationError,
    PublishDraftCommand, SessionStore, Store, StoreError, TenantContext, WorkspaceDraftRevision,
};
use question_model::{
    BackendCapabilities, Capability, CatalogLicenseValue, CatalogSearchQuery,
    CatalogStatisticsAvailability, CatalogTaxonomyFilter, DraftQuestionSource, ProblemDisplayRef,
    ProblemId, ProblemVersionRef, PublicationScope, QuestionSource, UserId, UserRole, VersionId,
    WorkspaceId,
};
use serde::{Deserialize, Serialize};

use crate::auth::{auth_error_response, no_store, resolve_request_session};

const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_CATALOG_BODY_BYTES: usize = 64 * 1_024;

// Kept behind a small helper so publication has one auditable point at which
// durable identities can be minted. In particular, source preparation must
// finish before this function is reached.
pub(crate) fn mint_publication_reference(revises: Option<ProblemVersionRef>) -> ProblemVersionRef {
    #[cfg(test)]
    {
        PUBLICATION_MINT_COUNT.with(|count| count.set(count.get() + 1));
    }
    ProblemVersionRef {
        problem: revises.map_or_else(ProblemId::generate, |reference| reference.problem),
        version: VersionId::generate(),
    }
}

#[cfg(test)]
thread_local! {
    static PUBLICATION_MINT_COUNT: Cell<usize> = const { Cell::new(0) };
}

/// Converts a draft locator into publication inputs before any immutable ID is
/// minted. The generic route only owns native publication: every external
/// backend needs a server-prepared immutable source artifact supplied by its
/// dedicated import/broker workflow.
///
/// This deliberately runs before [`mint_publication_reference`]: an
/// unprepared external source (currently iMathAS) is a refused draft state,
/// not a partially-created published identity.
pub(crate) fn prepare_published_source(
    source: DraftQuestionSource,
) -> Result<QuestionSource, &'static str> {
    match source {
        DraftQuestionSource::Native { family } => Ok(QuestionSource::Native { family }),
        DraftQuestionSource::Imathas { .. } => {
            Err("iMathAS publication requires a verified source snapshot and integration profile")
        }
        DraftQuestionSource::Webwork { .. }
        | DraftQuestionSource::Qti { .. }
        | DraftQuestionSource::H5p { .. } => {
            Err("external publication requires a server-prepared immutable source artifact")
        }
    }
}

/// Resolves trusted capabilities for the adapter owning one question.
pub trait BackendRegistry: Send + Sync {
    /// Returns the server's capability declaration for this source.
    fn capabilities(
        &self,
        source: &DraftQuestionSource,
    ) -> Result<BackendCapabilities, BackendRegistryError>;
}

/// Failure to resolve a server-owned adapter declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendRegistryError {
    /// The source names no adapter installed in this server.
    Unsupported,
    /// Registry state could not be read.
    Unavailable(String),
}

impl std::fmt::Display for BackendRegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => formatter.write_str("question backend is not registered"),
            Self::Unavailable(message) => {
                write!(formatter, "backend registry unavailable: {message}")
            }
        }
    }
}

impl std::error::Error for BackendRegistryError {}

/// Institution-configurable public-catalog review boundary.
#[async_trait]
pub trait PublicReviewGate: Send + Sync {
    /// Returns true only when this exact publication may enter the public catalog.
    async fn allows_publication(
        &self,
        tenant: TenantContext,
        publisher: UserId,
        draft: &DraftRecord,
    ) -> Result<bool, ReviewGateError>;
}

/// Public-review dependency failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewGateError(pub String);

impl std::fmt::Display for ReviewGateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "publication review unavailable: {}", self.0)
    }
}

impl std::error::Error for ReviewGateError {}

/// Default policy for institutions that do not require editorial review.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReviewNotRequired;

#[async_trait]
impl PublicReviewGate for ReviewNotRequired {
    async fn allows_publication(
        &self,
        _tenant: TenantContext,
        _publisher: UserId,
        _draft: &DraftRecord,
    ) -> Result<bool, ReviewGateError> {
        Ok(true)
    }
}

/// Builds the authenticated `/api/problems` and `/api/taxonomy` route group.
pub fn router<S, B, R>(store: Arc<S>, backends: Arc<B>, review_gate: Arc<R>) -> Router
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let state = CatalogRouteState {
        store,
        backends,
        review_gate,
    };
    Router::new()
        .route("/api/problems", get(list_problems::<S, B, R>))
        .route("/api/problems/search", get(search_problems::<S, B, R>))
        .route(
            "/api/problems/by-id/{reference}",
            get(resolve_problem_reference::<S, B, R>),
        )
        .route(
            "/api/problems/{problem}/versions/{version}",
            get(get_problem::<S, B, R>),
        )
        .route(
            "/api/problems/{problem}/versions/{version}/detail",
            get(get_problem_detail::<S, B, R>),
        )
        .route(
            "/api/problems/{workspace}/publish",
            post(publish_problem::<S, B, R>),
        )
        .route(
            "/api/problems/{problem}/versions/{version}/deprecate",
            post(deprecate_problem::<S, B, R>),
        )
        .route(
            "/api/problems/{problem}/versions/{version}/archive",
            post(archive_problem::<S, B, R>),
        )
        .route("/api/taxonomy", get(list_taxonomy::<S, B, R>))
        .layer(DefaultBodyLimit::max(MAX_CATALOG_BODY_BYTES))
        .with_state(state)
}

struct CatalogRouteState<S, B, R> {
    store: Arc<S>,
    backends: Arc<B>,
    review_gate: Arc<R>,
}

impl<S, B, R> Clone for CatalogRouteState<S, B, R> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            backends: Arc::clone(&self.backends),
            review_gate: Arc::clone(&self.review_gate),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct CatalogQuery {
    cursor: Option<String>,
    page_size: Option<u16>,
}

/// Query-string transport for strict catalog search. Repeated scalar keys keep
/// URLs inspectable (`taxonomy=scheme:code&capabilities=serverGrading`) while
/// the model receives typed exact filters after this boundary validates them.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogSearchHttpQuery {
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishProblemRequest {
    scope: PublicationScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublishRevisionError {
    Missing,
    Malformed,
}

/// Parses the one strong workspace ETag required for publication.
///
/// Publication consumes an exact mutable draft to mint an immutable catalog
/// version. Requiring the current revision prevents a stale browser tab from
/// publishing a collaborator's later edit.
fn required_publish_revision(
    headers: &HeaderMap,
) -> Result<WorkspaceDraftRevision, PublishRevisionError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(PublishRevisionError::Missing);
    };
    if values.next().is_some() {
        return Err(PublishRevisionError::Malformed);
    }
    let value = value
        .to_str()
        .map_err(|_| PublishRevisionError::Malformed)?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(PublishRevisionError::Malformed);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(PublishRevisionError::Malformed);
    }
    let numeric = value
        .parse::<u64>()
        .map_err(|_| PublishRevisionError::Malformed)?;
    if numeric == 0 || numeric > i64::MAX as u64 {
        return Err(PublishRevisionError::Malformed);
    }
    serde_json::from_str(value).map_err(|_| PublishRevisionError::Malformed)
}

#[derive(Debug, Deserialize)]
struct DeprecateProblemRequest {
    reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationValidationFailure {
    error: &'static str,
    violations: Vec<PublicationViolation>,
}

async fn list_problems<S, B, R>(
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

async fn list_taxonomy<S, B, R>(
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
async fn search_problems<S, B, R>(
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

async fn get_problem<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Path((problem, version)): Path<(ProblemId, VersionId)>,
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
    let reference = ProblemVersionRef { problem, version };
    match state
        .store
        .get_catalog_problem(authenticated.tenant_context, reference)
        .await
    {
        Ok(Some(record)) => no_store(Json(record.question).into_response()),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "problem version not found"),
        Err(error) => store_error_response(error),
    }
}

async fn resolve_problem_reference<S, B, R>(
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
async fn get_problem_detail<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Path((problem, version)): Path<(ProblemId, VersionId)>,
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
    match state
        .store
        .get_catalog_detail(
            authenticated.tenant_context,
            ProblemVersionRef { problem, version },
        )
        .await
    {
        Ok(Some(detail)) => no_store(Json(detail).into_response()),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "problem version not found"),
        Err(error) => store_error_response(error),
    }
}

async fn publish_problem<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
    Json(request): Json<PublishProblemRequest>,
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
    if !may_publish(authenticated.record.subject.roles(), request.scope) {
        return error_response(StatusCode::FORBIDDEN, "publication is not authorized");
    }
    let expected_revision = match required_publish_revision(&headers) {
        Ok(revision) => revision,
        Err(PublishRevisionError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match is required to publish a workspace",
            );
        }
        Err(PublishRevisionError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    let publisher = authenticated.record.subject.user();
    let draft = match state
        .store
        .get_draft(authenticated.tenant_context, publisher, workspace)
        .await
    {
        Ok(Some(draft)) => draft,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "draft not found"),
        Err(error) => return store_error_response(error),
    };
    if draft.revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    // Storage validates this for normal writes, but old imports or a repaired
    // database can still contain a legacy record. Refuse it at the HTTP
    // boundary before source preparation or immutable ID minting.
    if draft.record.question.metadata.validate_title().is_err() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "question title is invalid",
        );
    }
    let capabilities = match state.backends.capabilities(&draft.record.question.source) {
        Ok(capabilities) => capabilities,
        Err(BackendRegistryError::Unsupported) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "question backend is not registered",
            );
        }
        Err(BackendRegistryError::Unavailable(_)) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "backend registry unavailable",
            );
        }
    };
    let violations =
        domain::policy::validate_draft_for_publication(&draft.record.question, &capabilities);
    if !violations.is_empty() {
        return no_store(
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(PublicationValidationFailure {
                    error: "publication validation failed",
                    violations,
                }),
            )
                .into_response(),
        );
    }
    if request.scope == PublicationScope::Public {
        match state
            .review_gate
            .allows_publication(authenticated.tenant_context, publisher, &draft.record)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                return error_response(
                    StatusCode::FORBIDDEN,
                    "public publication requires institutional review",
                );
            }
            Err(_) => {
                return error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "publication review unavailable",
                );
            }
        }
    }
    // A review gate can await an external institutional workflow, and adapter
    // declarations can change while that happens. Re-read the actor-visible
    // draft immediately before source preparation and identity minting. The
    // Store repeats the exact-record comparison in its publication
    // transaction, closing the remaining race between this read and commit.
    let current_draft = match state
        .store
        .get_draft(authenticated.tenant_context, publisher, workspace)
        .await
    {
        Ok(Some(current_draft)) => current_draft,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "draft not found"),
        Err(error) => return store_error_response(error),
    };
    if current_draft.revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    let draft = current_draft.record;
    if draft.question.metadata.validate_title().is_err() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "question title is invalid",
        );
    }
    let capabilities = match state.backends.capabilities(&draft.question.source) {
        Ok(capabilities) => capabilities,
        Err(BackendRegistryError::Unsupported) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "question backend is not registered",
            );
        }
        Err(BackendRegistryError::Unavailable(_)) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "backend registry unavailable",
            );
        }
    };
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if !violations.is_empty() {
        return no_store(
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(PublicationValidationFailure {
                    error: "publication validation failed",
                    violations,
                }),
            )
                .into_response(),
        );
    }
    // Validate and freeze the source before minting either immutable ID. An
    // iMathAS locator has to be prepared by its server-owned integration into
    // a snapshot-bearing QuestionSource first.
    let published_source = match prepare_published_source(draft.question.source.clone()) {
        Ok(source) => source,
        Err(message) => {
            return error_response(StatusCode::UNPROCESSABLE_ENTITY, message);
        }
    };
    let publication = mint_publication_reference(draft.revises);
    let command = PublishDraftCommand {
        expected_draft: draft,
        expected_revision,
        publication,
        published_source,
        // Source-backed adapters are not wired to this generic route yet;
        // storage rejects them before a version can be minted.
        source_artifact: None,
        qti_promotion: None,
        flat_question_promotion: None,
        publisher,
        scope: request.scope,
        capabilities,
    };
    match state
        .store
        .publish_draft(authenticated.tenant_context, publisher, command)
        .await
    {
        Ok(record) => no_store((StatusCode::CREATED, Json(record.question)).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn deprecate_problem<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Path((problem, version)): Path<(ProblemId, VersionId)>,
    Json(request): Json<DeprecateProblemRequest>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    transition_problem(
        state,
        headers,
        ProblemVersionRef { problem, version },
        CatalogTransition::Deprecate {
            reason: request.reason,
        },
    )
    .await
}

async fn archive_problem<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Path((problem, version)): Path<(ProblemId, VersionId)>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    transition_problem(
        state,
        headers,
        ProblemVersionRef { problem, version },
        CatalogTransition::Archive,
    )
    .await
}

async fn transition_problem<S, B, R>(
    state: CatalogRouteState<S, B, R>,
    headers: HeaderMap,
    reference: ProblemVersionRef,
    transition: CatalogTransition,
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
    if !may_manage_catalog(authenticated.record.subject.roles()) {
        return error_response(StatusCode::FORBIDDEN, "catalog change is not authorized");
    }
    let actor = authenticated.record.subject.user();
    match state
        .store
        .transition_catalog_problem(authenticated.tenant_context, actor, reference, transition)
        .await
    {
        Ok(record) => no_store(Json(record.summary()).into_response()),
        Err(error) => store_error_response(error),
    }
}

pub(crate) fn may_publish(roles: &[UserRole], scope: PublicationScope) -> bool {
    match scope {
        PublicationScope::Institution => roles.iter().any(|role| {
            matches!(
                role,
                UserRole::Instructor | UserRole::Publisher | UserRole::Administrator
            )
        }),
        PublicationScope::Public => roles
            .iter()
            .any(|role| matches!(role, UserRole::Publisher | UserRole::Administrator)),
    }
}

fn may_manage_catalog(roles: &[UserRole]) -> bool {
    roles.iter().any(|role| {
        matches!(
            role,
            UserRole::Instructor | UserRole::Publisher | UserRole::Administrator
        )
    })
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

pub(crate) fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound => error_response(StatusCode::NOT_FOUND, "record not found"),
        StoreError::AlreadyExists => {
            error_response(StatusCode::CONFLICT, "immutable record already exists")
        }
        StoreError::Conflict => error_response(StatusCode::CONFLICT, "record changed; reload it"),
        StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::FORBIDDEN, "operation is not authorized")
        }
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::RunModel(error) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
        }
        StoreError::TimedOut => error_response(StatusCode::CONFLICT, "question attempt timed out"),
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "catalog unavailable")
        }
    }
}

pub(crate) fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use learning_data_access::in_memory::MemoryStore;
    use learning_data_access::{SessionLifetime, SessionSubject};
    use question_model::answer::NumericTolerance;
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::response::ResponseDefinition;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::{License, TaxonomyTerm};
    use question_model::{
        ActivityTimestamp, Capability, DraftQuestionDefinition, GradingDefinition,
        QuestionMetadata, TenantId, WorkspaceImportId,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tower::ServiceExt;
    use uuid::Uuid;

    #[derive(Clone)]
    struct FixtureRegistry {
        capabilities: BackendCapabilities,
    }

    struct ReviewRequired;

    /// Delivers a later adapter declaration to prove publication does not
    /// trust a capability result obtained before its final draft re-read.
    struct ChangingRegistry {
        initial: BackendCapabilities,
        current: BackendCapabilities,
        calls: AtomicUsize,
    }

    /// Simulates a collaborator saving while an institutional public-review
    /// workflow is in flight. The route must re-check the browser's original
    /// revision after this gate returns, before minting an identity.
    struct CollaboratorEditingReviewGate {
        store: Arc<MemoryStore>,
        collaborator: UserId,
    }

    impl BackendRegistry for FixtureRegistry {
        fn capabilities(
            &self,
            _source: &DraftQuestionSource,
        ) -> Result<BackendCapabilities, BackendRegistryError> {
            Ok(self.capabilities.clone())
        }
    }

    impl BackendRegistry for ChangingRegistry {
        fn capabilities(
            &self,
            _source: &DraftQuestionSource,
        ) -> Result<BackendCapabilities, BackendRegistryError> {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(self.initial.clone())
            } else {
                Ok(self.current.clone())
            }
        }
    }

    #[async_trait]
    impl PublicReviewGate for ReviewRequired {
        async fn allows_publication(
            &self,
            _tenant: TenantContext,
            _publisher: UserId,
            _draft: &DraftRecord,
        ) -> Result<bool, ReviewGateError> {
            Ok(false)
        }
    }

    #[async_trait]
    impl PublicReviewGate for CollaboratorEditingReviewGate {
        async fn allows_publication(
            &self,
            tenant: TenantContext,
            _publisher: UserId,
            draft: &DraftRecord,
        ) -> Result<bool, ReviewGateError> {
            let current = self
                .store
                .get_draft(tenant, self.collaborator, draft.question.workspace)
                .await
                .map_err(|error| ReviewGateError(error.to_string()))?
                .ok_or_else(|| ReviewGateError("review draft disappeared".to_string()))?;
            let mut replacement = current.record;
            replacement
                .question
                .metadata
                .title
                .push_str(" reviewed edit");
            self.store
                .upsert_draft(
                    tenant,
                    self.collaborator,
                    Some(current.revision),
                    replacement,
                )
                .await
                .map_err(|error| ReviewGateError(error.to_string()))?;
            Ok(true)
        }
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn strong_if_match(revision: WorkspaceDraftRevision) -> String {
        format!("\"{}\"", revision.value())
    }

    fn draft(tenant: TenantId, workspace: WorkspaceId, version: VersionId) -> DraftRecord {
        DraftRecord {
            tenant,
            question: DraftQuestionDefinition {
                workspace,
                source: DraftQuestionSource::Native {
                    family: "catalog-fixture".to_string(),
                },
                prompt: vec![ContentBlock::Text {
                    markdown: "What is the molecular mass?".to_string(),
                }],
                response: ResponseDefinition::Numeric {
                    tolerance: NumericTolerance::Relative { fraction: 0.01 },
                    unit: Some("g/mol".to_string()),
                },
                attempt_policy: AttemptPolicy {
                    max_attempts: Some(2),
                    feedback: FeedbackDisclosure::Deferred,
                },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Static,
                grading: GradingDefinition::AllOrNothing { points: 1.0 },
                metadata: QuestionMetadata {
                    title: format!("Catalog fixture {version}"),
                    tags: Vec::new(),
                    taxonomy: vec![TaxonomyTerm {
                        scheme: "discipline".to_string(),
                        code: format!("BIO-{version}"),
                        label: "Biochemistry".to_string(),
                    }],
                    license: License::CcBySa,
                    language: "en-US".to_string(),
                },
            },
            revises: None,
            derived_from: None,
        }
    }

    async fn issued_cookie(store: &MemoryStore, roles: Vec<UserRole>, user: UserId) -> String {
        let subject =
            SessionSubject::new(TenantId::from_uuid(id(1)), user, "Catalog Fixture", roles)
                .expect("fixture identity");
        let issued = crate::auth::issue_session(
            store,
            subject,
            crate::auth::SessionConfig::new(
                SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
                crate::auth::CookieTransport::LocalHttp,
            ),
        )
        .await
        .expect("fixture session");
        issued
            .set_cookie
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string()
    }

    async fn response_json(response: Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), 128 * 1_024)
            .await
            .expect("response body");
        serde_json::from_slice(&body).expect("JSON response")
    }

    #[tokio::test]
    async fn publication_uses_server_capabilities_roles_and_fresh_problem_identity() {
        let store = Arc::new(MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
            .expect("fixture clock");
        let tenant = TenantId::from_uuid(id(1));
        let workspace = WorkspaceId::from_uuid(id(2));
        let version = VersionId::from_uuid(id(3));
        let publisher = UserId::from_uuid(id(4));
        let mut candidate = draft(tenant, workspace, version);
        candidate.question.grading = GradingDefinition::PartialCredit { points: 1.0 };
        let draft_revision = store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                publisher,
                None,
                candidate.clone(),
            )
            .await
            .expect("draft save")
            .revision;
        let cookie = issued_cookie(&store, vec![UserRole::Publisher], publisher).await;

        let failing_app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::none(),
            }),
            Arc::new(ReviewNotRequired),
        );
        let rejected = failing_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", &cookie)
                    .header(IF_MATCH, strong_if_match(draft_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"public"}"#))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");
        assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let rejected = response_json(rejected).await;
        assert_eq!(
            rejected["violations"],
            serde_json::json!([
                {
                    "workspace": workspace,
                    "title": format!("Catalog fixture {version}"),
                    "capability": "serverGrading"
                },
                {
                    "workspace": workspace,
                    "title": format!("Catalog fixture {version}"),
                    "capability": "partialCredit"
                }
            ])
        );
        let still_draft = store
            .get_draft(
                TenantContext::from_authenticated_session(tenant),
                publisher,
                workspace,
            )
            .await
            .expect("draft lookup")
            .expect("validation failure retains draft");
        assert_eq!(still_draft.record.question.workspace, workspace);

        let passing_app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([
                    Capability::ServerGrading,
                    Capability::PartialCredit,
                ]),
            }),
            Arc::new(ReviewNotRequired),
        );
        let instructor_cookie =
            issued_cookie(&store, vec![UserRole::Instructor], UserId::from_uuid(id(5))).await;
        let role_rejected = passing_app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", instructor_cookie)
                    .header(IF_MATCH, strong_if_match(draft_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"public"}"#))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");
        assert_eq!(role_rejected.status(), StatusCode::FORBIDDEN);

        let review_app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([
                    Capability::ServerGrading,
                    Capability::PartialCredit,
                ]),
            }),
            Arc::new(ReviewRequired),
        );
        let review_rejected = review_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", &cookie)
                    .header(IF_MATCH, strong_if_match(draft_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"public"}"#))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");
        assert_eq!(review_rejected.status(), StatusCode::FORBIDDEN);

        let published = passing_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", cookie)
                    .header(IF_MATCH, strong_if_match(draft_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"public"}"#))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");
        assert_eq!(published.status(), StatusCode::CREATED);
        let published = response_json(published).await;
        assert_ne!(published["problem"], serde_json::Value::Null);
        assert_ne!(published["version"], serde_json::json!(version));
        assert_ne!(published["version"], serde_json::Value::Null);
        assert_eq!(published["workspace"], serde_json::json!(workspace));
    }

    #[tokio::test]
    async fn publication_requires_a_current_strong_workspace_revision_before_minting() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let workspace = WorkspaceId::from_uuid(id(702));
        let publisher = UserId::from_uuid(id(703));
        let initial_revision = store
            .upsert_draft(
                context,
                publisher,
                None,
                draft(tenant, workspace, VersionId::from_uuid(id(704))),
            )
            .await
            .expect("draft save")
            .revision;
        let cookie = issued_cookie(&store, vec![UserRole::Publisher], publisher).await;
        let app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            }),
            Arc::new(ReviewNotRequired),
        );

        PUBLICATION_MINT_COUNT.with(|count| count.set(0));
        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"institution"}"#))
                    .expect("missing revision request"),
            )
            .await
            .expect("missing revision response");
        assert_eq!(missing.status(), StatusCode::PRECONDITION_REQUIRED);
        assert_eq!(missing.headers()["cache-control"], "no-store");

        for malformed in ["W/\"1\"", "\"0\"", "\"9223372036854775808\""] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/problems/{workspace}/publish"))
                        .header("cookie", &cookie)
                        .header(IF_MATCH, malformed)
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"scope":"institution"}"#))
                        .expect("malformed revision request"),
                )
                .await
                .expect("malformed revision response");
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(response.headers()["cache-control"], "no-store");
        }

        let current_revision = store
            .upsert_draft(
                context,
                publisher,
                Some(initial_revision),
                draft(tenant, workspace, VersionId::from_uuid(id(704))),
            )
            .await
            .expect("fixture update")
            .revision;
        assert_ne!(initial_revision, current_revision);

        let stale = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", cookie)
                    .header(IF_MATCH, strong_if_match(initial_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"institution"}"#))
                    .expect("stale revision request"),
            )
            .await
            .expect("stale revision response");
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        assert_eq!(stale.headers()["cache-control"], "no-store");
        assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
    }

    #[tokio::test]
    async fn publication_refuses_a_collaborator_edit_that_arrives_during_review_before_minting() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let workspace = WorkspaceId::from_uuid(id(712));
        let publisher = UserId::from_uuid(id(713));
        let collaborator = UserId::from_uuid(id(714));
        let published_revision = store
            .upsert_draft(
                context,
                publisher,
                None,
                draft(tenant, workspace, VersionId::from_uuid(id(715))),
            )
            .await
            .expect("draft save")
            .revision;
        store
            .grant_draft_collaborator(context, publisher, workspace, collaborator)
            .await
            .expect("collaborator grant");
        let cookie = issued_cookie(&store, vec![UserRole::Publisher], publisher).await;
        let app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            }),
            Arc::new(CollaboratorEditingReviewGate {
                store: Arc::clone(&store),
                collaborator,
            }),
        );

        PUBLICATION_MINT_COUNT.with(|count| count.set(0));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", cookie)
                    .header(IF_MATCH, strong_if_match(published_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"public"}"#))
                    .expect("publication request"),
            )
            .await
            .expect("publication response");
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
        assert_eq!(
            store
                .get_draft(context, publisher, workspace)
                .await
                .expect("draft reload")
                .expect("draft stays editable")
                .revision
                .value(),
            2
        );
    }

    #[tokio::test]
    async fn same_tenant_nonowner_publisher_cannot_mint_from_a_private_workspace() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let workspace = WorkspaceId::from_uuid(id(81));
        let owner = UserId::from_uuid(id(82));
        let owner_revision = store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                owner,
                None,
                draft(tenant, workspace, VersionId::from_uuid(id(83))),
            )
            .await
            .expect("owner draft save")
            .revision;
        let nonowner = UserId::from_uuid(id(84));
        let cookie = issued_cookie(&store, vec![UserRole::Publisher], nonowner).await;
        let app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            }),
            Arc::new(ReviewNotRequired),
        );

        PUBLICATION_MINT_COUNT.with(|count| count.set(0));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", cookie)
                    .header(IF_MATCH, strong_if_match(owner_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"institution"}"#))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
    }

    #[tokio::test]
    async fn changed_server_capabilities_refuse_before_minting_and_preserve_the_draft() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let workspace = WorkspaceId::from_uuid(id(85));
        let publisher = UserId::from_uuid(id(86));
        let candidate = draft(tenant, workspace, VersionId::from_uuid(id(87)));
        let draft_revision = store
            .upsert_draft(context, publisher, None, candidate.clone())
            .await
            .expect("draft save")
            .revision;
        let cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;
        let registry = Arc::new(ChangingRegistry {
            initial: BackendCapabilities::from_iter([Capability::ServerGrading]),
            current: BackendCapabilities::none(),
            calls: AtomicUsize::new(0),
        });
        let app = router(
            Arc::clone(&store),
            Arc::clone(&registry),
            Arc::new(ReviewNotRequired),
        );

        PUBLICATION_MINT_COUNT.with(|count| count.set(0));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", cookie)
                    .header(IF_MATCH, strong_if_match(draft_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"institution"}"#))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
        assert_eq!(registry.calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            store
                .get_draft(context, publisher, workspace)
                .await
                .map(|draft| draft.map(|draft| draft.record)),
            Ok(Some(candidate)),
        );
    }

    #[tokio::test]
    async fn unprepared_imathas_refusal_preserves_draft_without_minting_an_identity() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let workspace = WorkspaceId::from_uuid(id(91));
        let mut candidate = draft(tenant, workspace, VersionId::from_uuid(id(92)));
        candidate.question.source = DraftQuestionSource::Imathas {
            provider: "institution-imathas".to_string(),
            item_ref: "1842".to_string(),
        };
        let publisher = UserId::from_uuid(id(93));
        let draft_revision = store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                publisher,
                None,
                candidate,
            )
            .await
            .expect("draft save")
            .revision;
        let cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;
        let app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            }),
            Arc::new(ReviewNotRequired),
        );

        // The thread-local seam observes this route task only. Source
        // preparation is intentionally before mint_publication_reference.
        PUBLICATION_MINT_COUNT.with(|count| count.set(0));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", cookie)
                    .header(IF_MATCH, strong_if_match(draft_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"institution"}"#))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
        assert!(
            store
                .get_draft(
                    TenantContext::from_authenticated_session(tenant),
                    publisher,
                    workspace,
                )
                .await
                .expect("draft lookup")
                .is_some()
        );
    }

    #[tokio::test]
    async fn corrupt_legacy_titles_refuse_at_http_boundary_before_minting() {
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        for (offset, title) in [(0_u128, " \t\n ".to_string()), (1, "\u{1F9EC}".repeat(513))] {
            let store = Arc::new(MemoryStore::default());
            let workspace = WorkspaceId::from_uuid(id(300 + offset));
            let mut legacy = draft(tenant, workspace, VersionId::from_uuid(id(310 + offset)));
            legacy.question.metadata.title = title;
            store
                .insert_legacy_draft_for_test(legacy.clone())
                .expect("legacy injection is test-only");
            let cookie = issued_cookie(
                &store,
                vec![UserRole::Instructor],
                UserId::from_uuid(id(320 + offset)),
            )
            .await;
            let app = router(
                Arc::clone(&store),
                Arc::new(FixtureRegistry {
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                }),
                Arc::new(ReviewNotRequired),
            );

            PUBLICATION_MINT_COUNT.with(|count| count.set(0));
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/problems/{workspace}/publish"))
                        .header("cookie", cookie)
                        .header(IF_MATCH, "\"1\"")
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"scope":"institution"}"#))
                        .expect("publish request"),
                )
                .await
                .expect("publish response");

            // Legacy rows without an explicitly migrated owner remain absent
            // to every actor; a later caller must never acquire them merely
            // by attempting publication.
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
            assert!(
                store
                    .get_draft(context, UserId::from_uuid(id(320 + offset)), workspace)
                    .await
                    .expect("draft lookup")
                    .is_none(),
                "unowned legacy data must not become visible to the caller"
            );
        }
    }

    #[tokio::test]
    async fn every_unprepared_source_backed_draft_refuses_before_identity_minting() {
        // `issued_cookie` deliberately models the fixture institution tenant.
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let sources = [
            DraftQuestionSource::Webwork {
                pg_path: "Library/Calc/test.pg".to_string(),
            },
            DraftQuestionSource::Qti {
                item_id: "item-1".to_string(),
                import_id: WorkspaceImportId::from_uuid(id(511)),
            },
            DraftQuestionSource::H5p {
                content_type: "H5P.MultiChoice".to_string(),
            },
            DraftQuestionSource::Imathas {
                provider: "institution-imathas".to_string(),
                item_ref: "1842".to_string(),
            },
        ];
        for (offset, source) in sources.into_iter().enumerate() {
            let store = Arc::new(MemoryStore::default());
            let workspace = WorkspaceId::from_uuid(id(121 + offset as u128));
            let mut candidate = draft(
                tenant,
                workspace,
                VersionId::from_uuid(id(130 + offset as u128)),
            );
            candidate.question.source = source;
            let publisher = UserId::from_uuid(id(140 + offset as u128));
            let draft_revision = store
                .upsert_draft(context, publisher, None, candidate.clone())
                .await
                .expect("source-backed draft should save")
                .revision;
            let cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;
            let app = router(
                Arc::clone(&store),
                Arc::new(FixtureRegistry {
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                }),
                Arc::new(ReviewNotRequired),
            );
            PUBLICATION_MINT_COUNT.with(|count| count.set(0));
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/problems/{workspace}/publish"))
                        .header("cookie", cookie)
                        .header(IF_MATCH, strong_if_match(draft_revision))
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"scope":"institution"}"#))
                        .expect("publish request"),
                )
                .await
                .expect("publish response");
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
            assert_eq!(
                store
                    .get_draft(context, publisher, workspace)
                    .await
                    .map(|draft| draft.map(|draft| draft.record)),
                Ok(Some(candidate))
            );
        }
    }

    #[tokio::test]
    async fn catalog_and_taxonomy_lists_use_cursors_and_hide_deprecated_versions() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let publisher = UserId::from_uuid(id(10));
        let cookie = issued_cookie(&store, vec![UserRole::Publisher], publisher).await;
        let app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            }),
            Arc::new(ReviewNotRequired),
        );

        let mut published_references = Vec::new();
        for value in [20_u128, 30_u128] {
            let workspace = WorkspaceId::from_uuid(id(value));
            let version = VersionId::from_uuid(id(value + 1));
            let draft_revision = store
                .upsert_draft(context, publisher, None, draft(tenant, workspace, version))
                .await
                .expect("draft save")
                .revision;
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/problems/{workspace}/publish"))
                        .header("cookie", &cookie)
                        .header(IF_MATCH, strong_if_match(draft_revision))
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"scope":"public"}"#))
                        .expect("publish request"),
                )
                .await
                .expect("publish response");
            assert_eq!(response.status(), StatusCode::CREATED);
            let response = response_json(response).await;
            published_references.push((
                response["problem"]
                    .as_str()
                    .expect("published problem ID")
                    .to_string(),
                response["version"]
                    .as_str()
                    .expect("published version ID")
                    .to_string(),
            ));
        }

        let first = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/problems?pageSize=1")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("list request"),
            )
            .await
            .expect("list response");
        assert_eq!(first.status(), StatusCode::OK);
        let first = response_json(first).await;
        assert_eq!(first["items"].as_array().map(Vec::len), Some(1));
        let cursor = first["nextCursor"]
            .as_str()
            .expect("first page cursor")
            .to_string();
        let second = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/problems?pageSize=1&cursor={}",
                        cursor.replace('/', "%2F")
                    ))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("list request"),
            )
            .await
            .expect("list response");
        let second = response_json(second).await;
        assert_eq!(second["items"].as_array().map(Vec::len), Some(1));
        assert_ne!(first["items"][0], second["items"][0]);
        assert_eq!(second["nextCursor"], serde_json::Value::Null);

        let taxonomy = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/taxonomy?pageSize=1")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("taxonomy request"),
            )
            .await
            .expect("taxonomy response");
        assert_eq!(taxonomy.status(), StatusCode::OK);
        let taxonomy = response_json(taxonomy).await;
        assert_eq!(taxonomy["items"].as_array().map(Vec::len), Some(1));
        assert!(taxonomy["nextCursor"].is_string());
        let taxonomy_cursor = taxonomy["nextCursor"]
            .as_str()
            .expect("taxonomy cursor")
            .to_string();
        let taxonomy_second = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/taxonomy?pageSize=1&cursor={taxonomy_cursor}"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("taxonomy continuation request"),
            )
            .await
            .expect("taxonomy continuation response");
        assert_eq!(taxonomy_second.status(), StatusCode::OK);
        let taxonomy_second = response_json(taxonomy_second).await;
        assert_eq!(taxonomy_second["items"].as_array().map(Vec::len), Some(1));
        assert_ne!(taxonomy["items"][0], taxonomy_second["items"][0]);
        assert_eq!(taxonomy_second["nextCursor"], serde_json::Value::Null);

        for path in ["/api/problems", "/api/taxonomy"] {
            for query in ["pageSize=0", "pageSize=101", "cursor=", "offset=1"] {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri(format!("{path}?{query}"))
                            .header("cookie", &cookie)
                            .body(Body::empty())
                            .expect("invalid pagination request"),
                    )
                    .await
                    .expect("invalid pagination response");
                assert_eq!(
                    response.status(),
                    StatusCode::BAD_REQUEST,
                    "{path}?{query} must be rejected"
                );
            }
        }

        let (problem, version) = &published_references[0];
        let deprecated = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/problems/{problem}/versions/{version}/deprecate"
                    ))
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"reason":"Correction available"}"#))
                    .expect("deprecate request"),
            )
            .await
            .expect("deprecate response");
        assert_eq!(deprecated.status(), StatusCode::OK);

        let browse = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/problems?pageSize=10")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("catalog request"),
            )
            .await
            .expect("catalog response");
        let browse = response_json(browse).await;
        assert_eq!(browse["items"].as_array().map(Vec::len), Some(1));

        let exact = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/problems/{problem}/versions/{version}"))
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("exact request"),
            )
            .await
            .expect("exact response");
        assert_eq!(exact.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn catalog_search_and_safe_detail_are_authenticated_bounded_and_non_cacheable() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let publisher = UserId::from_uuid(id(901));
        let cookie = issued_cookie(&store, vec![UserRole::Publisher], publisher).await;
        let workspace = WorkspaceId::from_uuid(id(902));
        let version = VersionId::from_uuid(id(903));
        let draft_revision = store
            .upsert_draft(context, publisher, None, draft(tenant, workspace, version))
            .await
            .expect("draft save")
            .revision;
        let app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            }),
            Arc::new(ReviewNotRequired),
        );
        let published = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", &cookie)
                    .header(IF_MATCH, strong_if_match(draft_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"public"}"#))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");
        let published = response_json(published).await;
        let problem = published["problem"].as_str().expect("problem id");
        let version = published["version"].as_str().expect("version id");

        let search = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/problems/search?text=catalog&pageSize=1")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("search request"),
            )
            .await
            .expect("search response");
        assert_eq!(search.status(), StatusCode::OK);
        assert_eq!(search.headers()["cache-control"], "no-store");
        let search = response_json(search).await;
        assert_eq!(search["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(search["facets"]["statistics"]["available"], 0);
        assert_eq!(search["facets"]["statistics"]["unavailable"], 1);

        let detail = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/problems/{problem}/versions/{version}/detail"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("detail request"),
            )
            .await
            .expect("detail response");
        assert_eq!(detail.status(), StatusCode::OK);
        assert_eq!(detail.headers()["cache-control"], "no-store");
        let detail = response_json(detail).await;
        for forbidden in ["source", "response", "grading", "answerKey", "provider"] {
            assert!(detail.get(forbidden).is_none(), "detail leaked {forbidden}");
        }
        assert_eq!(detail["statistics"], "unavailable");

        let hostile = app
            .oneshot(
                Request::builder()
                    .uri("/api/problems/search?offset=1")
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("hostile request"),
            )
            .await
            .expect("hostile response");
        assert_eq!(hostile.status(), StatusCode::BAD_REQUEST);
    }
}
