//! Authenticated private workspace draft routes (MOD-UI-EDITOR).
//!
//! A workspace is deliberately unversioned authoring state.  The route only
//! accepts the browser-safe draft definition and derives the tenant from the
//! resolved server session; publication and source preparation remain owned by
//! the catalog and adapter boundaries.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use question_model::catalog::QuestionBackend;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::{License, TaxonomyTerm};
use question_model::{DraftQuestionDefinition, UserRole, WorkspaceId};
use serde::{Deserialize, Serialize};
use store::{
    CatalogStore, Cursor, DraftRecord, PageRequest, PageSize, PaginationError, SessionStore, Store,
    StoreError, WorkspaceDraft, WorkspaceDraftRevision,
};

use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::catalog::{BackendRegistry, BackendRegistryError};

const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_WORKSPACE_BODY_BYTES: usize = 64 * 1_024;

/// Builds the author-only private workspace route group.
pub fn router<S, B>(store: Arc<S>, backends: Arc<B>) -> Router
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    Router::new()
        .route("/api/workspaces", get(list_workspaces::<S, B>))
        .route(
            "/api/workspaces/{workspace}",
            get(get_workspace::<S, B>)
                .put(save_workspace::<S, B>)
                .delete(delete_workspace::<S, B>),
        )
        .route(
            "/api/workspaces/{workspace}/publication-validation",
            post(validate_publication::<S, B>),
        )
        .route(
            "/api/workspaces/{workspace}/publication-diff",
            get(publication_diff::<S, B>),
        )
        .layer(DefaultBodyLimit::max(MAX_WORKSPACE_BODY_BYTES))
        // This also covers extractor rejections (invalid JSON, oversized
        // bodies, and malformed path values), which never reach a handler.
        .layer(middleware::map_response(no_store_response))
        .with_state(WorkspaceRouteState { store, backends })
}

struct WorkspaceRouteState<S, B> {
    store: Arc<S>,
    backends: Arc<B>,
}

impl<S, B> Clone for WorkspaceRouteState<S, B> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            backends: Arc::clone(&self.backends),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkspaceQuery {
    cursor: Option<String>,
    page_size: Option<u16>,
}

async fn list_workspaces<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Query(query): Query<WorkspaceQuery>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    match state
        .store
        .list_drafts(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            page,
        )
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn get_workspace<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    match state
        .store
        .get_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
        )
        .await
    {
        Ok(Some(draft)) => draft_response(draft),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "workspace not found"),
        Err(error) => store_error_response(error),
    }
}

async fn save_workspace<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
    Json(value): Json<serde_json::Value>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let question = match strict_draft_definition(value) {
        Ok(question) => question,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "workspace draft body is invalid",
            );
        }
    };
    if question.workspace != workspace {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "workspace path does not match the draft body",
        );
    }
    let expected_revision = match expected_revision(&headers) {
        Ok(revision) => revision,
        Err(()) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    // The browser never supplies lineage.  A refresh must not sever a draft's
    // pending revision or attribution relationship before publication.
    let existing = match state
        .store
        .get_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
        )
        .await
    {
        Ok(existing) => existing,
        Err(error) => return store_error_response(error),
    };
    let draft = DraftRecord {
        tenant: authenticated.tenant_context.tenant_id(),
        question: question.clone(),
        revises: existing.as_ref().and_then(|draft| draft.record.revises),
        derived_from: existing.and_then(|draft| draft.record.derived_from),
    };
    match state
        .store
        .upsert_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            expected_revision,
            draft,
        )
        .await
    {
        Ok(saved) => draft_response(saved),
        Err(error) => store_error_response(error),
    }
}

async fn delete_workspace<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let expected_revision = match required_revision(&headers) {
        Ok(revision) => revision,
        Err(RequiredRevisionError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match is required to delete a workspace",
            );
        }
        Err(RequiredRevisionError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    match state
        .store
        .delete_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
            expected_revision,
        )
        .await
    {
        Ok(true) => no_store(StatusCode::NO_CONTENT.into_response()),
        // A foreign tenant intentionally has the same result as an absent row.
        Ok(false) => error_response(StatusCode::NOT_FOUND, "workspace not found"),
        Err(error) => store_error_response(error),
    }
}

/// Validates whether the stored draft can cross the publication capability
/// boundary. The request is intentionally bodyless: the server validates the
/// exact persisted draft, never a browser-supplied shadow copy.
async fn validate_publication<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
    body: Bytes,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    if !body.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "publication validation does not accept a request body",
        );
    }
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let draft = match state
        .store
        .get_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
        )
        .await
    {
        Ok(Some(draft)) => draft,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "workspace not found"),
        Err(error) => return store_error_response(error),
    };
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
    if violations.is_empty()
        && let Err(message) = crate::catalog::prepare_published_source(draft.record.question.source)
    {
        return error_response(StatusCode::UNPROCESSABLE_ENTITY, message);
    }
    revisioned_response(draft.revision, PublicationValidationReport { violations })
}

/// Returns the safe, semantic before/after projection used by the publishing
/// confirmation. Source locators, artifacts, providers, and grading material
/// are deliberately absent even though this is an author-only route.
async fn publication_diff<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let draft = match state
        .store
        .get_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
        )
        .await
    {
        Ok(Some(draft)) => draft,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "workspace not found"),
        Err(error) => return store_error_response(error),
    };
    let revision = draft.revision;
    let draft = draft.record;
    let current = PublicationSemanticProjection::from_draft(&draft.question);
    let Some(revises) = draft.revises else {
        return revisioned_response(
            revision,
            PublicationDiff {
                draft_revision: revision,
                baseline: PublicationDiffBaseline::FirstPublication,
                prior: None,
                previous: None,
                current,
                changed: Vec::new(),
            },
        );
    };
    let previous = match state
        .store
        .get_catalog_problem(authenticated.tenant_context, revises)
        .await
    {
        Ok(Some(record)) => PublicationSemanticProjection::from_published(&record.question),
        // A revision cannot be meaningfully described as a first publication
        // when its immutable predecessor is unavailable to this tenant.
        Ok(None) => {
            return error_response(
                StatusCode::CONFLICT,
                "publication predecessor is not available",
            );
        }
        Err(error) => return store_error_response(error),
    };
    let changed = previous.changed_fields(&current);
    revisioned_response(
        revision,
        PublicationDiff {
            draft_revision: revision,
            baseline: PublicationDiffBaseline::Revision,
            prior: Some(revises),
            previous: Some(previous),
            current,
            changed,
        },
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationValidationReport {
    violations: Vec<domain::policy::PublicationViolation>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationDiff {
    /// The exact private draft revision represented by this comparison.  The
    /// browser must revalidate after a draft save changes this value before it
    /// can treat the confirmation as current.
    draft_revision: WorkspaceDraftRevision,
    baseline: PublicationDiffBaseline,
    prior: Option<question_model::ProblemVersionRef>,
    previous: Option<PublicationSemanticProjection>,
    current: PublicationSemanticProjection,
    changed: Vec<PublicationSemanticField>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum PublicationDiffBaseline {
    FirstPublication,
    Revision,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum PublicationSemanticField {
    SourceBackend,
    Title,
    Prompt,
    Response,
    AttemptPolicy,
    TimingPolicy,
    Randomization,
    Metadata,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationSemanticProjection {
    source_backend: QuestionBackend,
    title: String,
    prompt: PromptShape,
    response: ResponseShape,
    attempt_policy: AttemptPolicy,
    timing_policy: TimingPolicy,
    randomization: RandomizationShape,
    metadata: MetadataShape,
}

impl PublicationSemanticProjection {
    fn from_draft(question: &DraftQuestionDefinition) -> Self {
        Self::from_content(
            QuestionBackend::from(&question.source),
            &question.metadata.title,
            &question.prompt,
            &question.response,
            question.attempt_policy,
            question.timing_policy,
            &question.randomization,
            &question.metadata,
        )
    }

    fn from_published(question: &question_model::QuestionDefinition) -> Self {
        Self::from_content(
            QuestionBackend::from(&question.source),
            &question.metadata.title,
            &question.prompt,
            &question.response,
            question.attempt_policy,
            question.timing_policy,
            &question.randomization,
            &question.metadata,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_content(
        source_backend: QuestionBackend,
        title: &str,
        prompt: &[ContentBlock],
        response: &ResponseDefinition,
        attempt_policy: AttemptPolicy,
        timing_policy: TimingPolicy,
        randomization: &RandomizationDefinition,
        metadata: &question_model::QuestionMetadata,
    ) -> Self {
        Self {
            source_backend,
            title: title.to_string(),
            prompt: PromptShape::from_blocks(prompt),
            response: ResponseShape::from_definition(response),
            attempt_policy,
            timing_policy,
            randomization: RandomizationShape::from_definition(randomization),
            metadata: MetadataShape::from_metadata(metadata),
        }
    }

    fn changed_fields(&self, current: &Self) -> Vec<PublicationSemanticField> {
        let mut changed = Vec::new();
        if self.source_backend != current.source_backend {
            changed.push(PublicationSemanticField::SourceBackend);
        }
        if self.title != current.title {
            changed.push(PublicationSemanticField::Title);
        }
        if self.prompt != current.prompt {
            changed.push(PublicationSemanticField::Prompt);
        }
        if self.response != current.response {
            changed.push(PublicationSemanticField::Response);
        }
        if self.attempt_policy != current.attempt_policy {
            changed.push(PublicationSemanticField::AttemptPolicy);
        }
        if self.timing_policy != current.timing_policy {
            changed.push(PublicationSemanticField::TimingPolicy);
        }
        if self.randomization != current.randomization {
            changed.push(PublicationSemanticField::Randomization);
        }
        if self.metadata != current.metadata {
            changed.push(PublicationSemanticField::Metadata);
        }
        changed
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptShape {
    blocks: Vec<PromptBlockKind>,
}

impl PromptShape {
    fn from_blocks(blocks: &[ContentBlock]) -> Self {
        Self {
            blocks: blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { .. } => PromptBlockKind::Text,
                    ContentBlock::Math { .. } => PromptBlockKind::Math,
                    ContentBlock::Image { .. } => PromptBlockKind::Image,
                    ContentBlock::Code { .. } => PromptBlockKind::Code,
                    ContentBlock::Table { .. } => PromptBlockKind::Table,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum PromptBlockKind {
    Text,
    Math,
    Image,
    Code,
    Table,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseShape {
    kind: ResponseKind,
    option_count: Option<usize>,
}

impl ResponseShape {
    fn from_definition(definition: &ResponseDefinition) -> Self {
        match definition {
            ResponseDefinition::Numeric { .. } => Self {
                kind: ResponseKind::Numeric,
                option_count: None,
            },
            ResponseDefinition::MultipleChoice { choices, .. } => Self {
                kind: ResponseKind::MultipleChoice,
                option_count: Some(choices.len()),
            },
            ResponseDefinition::ShortText { .. } => Self {
                kind: ResponseKind::ShortText,
                option_count: None,
            },
            ResponseDefinition::Ordering { items } => Self {
                kind: ResponseKind::Ordering,
                option_count: Some(items.len()),
            },
            ResponseDefinition::FileUpload { .. } => Self {
                kind: ResponseKind::FileUpload,
                option_count: None,
            },
            ResponseDefinition::ExternalTool {} => Self {
                kind: ResponseKind::ExternalTool,
                option_count: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum ResponseKind {
    Numeric,
    MultipleChoice,
    ShortText,
    Ordering,
    FileUpload,
    ExternalTool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RandomizationShape {
    kind: RandomizationKind,
}

impl RandomizationShape {
    fn from_definition(definition: &RandomizationDefinition) -> Self {
        let kind = match definition {
            RandomizationDefinition::Static => RandomizationKind::Static,
            RandomizationDefinition::Seeded { .. } => RandomizationKind::Seeded,
        };
        Self { kind }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum RandomizationKind {
    Static,
    Seeded,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MetadataShape {
    tags: Vec<String>,
    taxonomy: Vec<TaxonomyTerm>,
    license: License,
    language: String,
}

impl MetadataShape {
    fn from_metadata(metadata: &question_model::QuestionMetadata) -> Self {
        Self {
            tags: metadata
                .tags
                .iter()
                .map(|tag| tag.as_str().to_string())
                .collect(),
            taxonomy: metadata.taxonomy.clone(),
            license: metadata.license.clone(),
            language: metadata.language.clone(),
        }
    }
}

fn may_author_workspaces(roles: &[UserRole]) -> bool {
    roles.iter().any(|role| {
        matches!(
            role,
            UserRole::Instructor | UserRole::Publisher | UserRole::Administrator
        )
    })
}

fn page_request(query: WorkspaceQuery) -> Result<PageRequest, PaginationError> {
    let size = PageSize::new(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))?;
    match query.cursor {
        Some(cursor) => Ok(PageRequest::after(Cursor::parse(cursor)?, size)),
        None => Ok(PageRequest::first(size)),
    }
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound => error_response(StatusCode::NOT_FOUND, "workspace not found"),
        StoreError::AlreadyExists => {
            error_response(StatusCode::CONFLICT, "workspace already exists")
        }
        StoreError::Conflict => {
            error_response(StatusCode::CONFLICT, "workspace changed; reload it")
        }
        // Workspace visibility is governed by persisted owner/collaborator
        // bindings. Returning not-found keeps an unshared same-tenant draft
        // indistinguishable from an absent or foreign draft.
        StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::NOT_FOUND, "workspace not found")
        }
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::RunModel(error) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
        }
        StoreError::TimedOut => {
            error_response(StatusCode::CONFLICT, "workspace operation timed out")
        }
        StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "workspace storage unavailable",
        ),
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

/// Returns the browser-safe draft while keeping the concurrency token in a
/// standard response header rather than authored JSON. A later PUT must echo
/// this exact strong ETag in `If-Match` to replace an existing draft.
fn draft_response(draft: WorkspaceDraft) -> Response {
    revisioned_response(draft.revision, draft.record.question)
}

/// Attaches the exact private-draft revision represented by an authoring
/// response. Validation and publication diff are snapshots too, so their
/// strong ETag must be treated with the same freshness semantics as detail.
fn revisioned_response<T>(revision: WorkspaceDraftRevision, body: T) -> Response
where
    T: Serialize,
{
    let revision = HeaderValue::from_str(&format!("\"{}\"", revision.value()))
        .expect("a decimal workspace revision is always a valid ETag");
    let mut response = Json(body).into_response();
    response.headers_mut().insert(ETAG, revision);
    no_store(response)
}

/// Parses the single strong ETag accepted by workspace PUT.
///
/// Omitting the precondition asks storage to create a new draft. Storage
/// rejects that request with 409 when the workspace already exists, so this
/// never becomes a last-writer-wins update path.
fn expected_revision(headers: &HeaderMap) -> Result<Option<WorkspaceDraftRevision>, ()> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(());
    }
    let value = value.to_str().map_err(|_| ())?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(());
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(());
    }
    let numeric = value.parse::<u64>().map_err(|_| ())?;
    if numeric == 0 || numeric > i64::MAX as u64 {
        return Err(());
    }
    serde_json::from_str(value).map(Some).map_err(|_| ())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequiredRevisionError {
    Missing,
    Malformed,
}

/// Parses the one current strong ETag that must accompany destructive draft
/// deletion.  A missing precondition is distinguishable from malformed input
/// so the browser can refresh rather than mistaking a stale tab for a valid
/// delete request.
fn required_revision(headers: &HeaderMap) -> Result<WorkspaceDraftRevision, RequiredRevisionError> {
    match expected_revision(headers) {
        Ok(Some(revision)) => Ok(revision),
        Ok(None) => Err(RequiredRevisionError::Missing),
        Err(()) => Err(RequiredRevisionError::Malformed),
    }
}

async fn no_store_response(response: Response) -> Response {
    no_store(response)
}

/// Decodes exactly the browser workspace contract. Serde's ordinary model
/// deserialization tolerates additional fields for storage evolution; this
/// HTTP boundary compares the typed canonical form to received JSON so
/// unknown fields are rejected at every nested level.
fn strict_draft_definition(value: serde_json::Value) -> Result<DraftQuestionDefinition, ()> {
    let question: DraftQuestionDefinition =
        serde_json::from_value(value.clone()).map_err(|_| ())?;
    let canonical = serde_json::to_value(&question).map_err(|_| ())?;
    if value == canonical {
        Ok(question)
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use question_model::answer::TextMatchMode;
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::response::ResponseDefinition;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        BackendCapabilities, Capability, DraftQuestionSource, GradingDefinition, ProblemId,
        ProblemVersionRef, QuestionMetadata, TenantId, UserId, VersionId,
    };
    use store::memory::MemoryStore;
    use store::{SessionLifetime, SessionSubject, TenantContext};
    use tower::ServiceExt;
    use uuid::Uuid;

    #[derive(Debug, Default)]
    struct FixtureRegistry {
        capabilities: BackendCapabilities,
    }

    impl BackendRegistry for FixtureRegistry {
        fn capabilities(
            &self,
            _source: &DraftQuestionSource,
        ) -> Result<BackendCapabilities, BackendRegistryError> {
            Ok(self.capabilities.clone())
        }
    }

    fn test_router(store: Arc<MemoryStore>) -> Router {
        router(
            store,
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            }),
        )
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn draft(workspace: WorkspaceId, title: &str) -> DraftQuestionDefinition {
        DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: "workspace-fixture".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Name the bond joining amino acids.".to_string(),
            }],
            response: ResponseDefinition::ShortText {
                match_mode: TextMatchMode::Normalized,
                max_length: 64,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: title.to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        }
    }

    async fn issued_cookie(
        store: &MemoryStore,
        tenant: TenantId,
        roles: Vec<UserRole>,
        user: UserId,
    ) -> String {
        let subject = SessionSubject::new(tenant, user, "Workspace Fixture", roles)
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
    async fn author_can_save_list_refresh_and_delete_its_workspace() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let workspace = WorkspaceId::from_uuid(id(2));
        let cookie = issued_cookie(
            &store,
            tenant,
            vec![UserRole::Instructor],
            UserId::from_uuid(id(3)),
        )
        .await;
        let candidate = draft(workspace, "Peptide bond draft");
        let app = test_router(Arc::clone(&store));
        let prior_revision = ProblemVersionRef {
            problem: ProblemId::from_uuid(id(20)),
            version: VersionId::from_uuid(id(21)),
        };
        store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                UserId::from_uuid(id(3)),
                None,
                DraftRecord {
                    tenant,
                    question: draft(workspace, "Earlier title"),
                    revises: Some(prior_revision),
                    derived_from: None,
                },
            )
            .await
            .expect("seed prior draft lineage");
        let initial_revision = store
            .get_draft(
                TenantContext::from_authenticated_session(tenant),
                UserId::from_uuid(id(3)),
                workspace,
            )
            .await
            .expect("seed draft lookup")
            .expect("seed draft exists")
            .revision;

        let saved = app
            .clone()
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .header(IF_MATCH, format!("\"{}\"", initial_revision.value()))
                    .body(Body::from(
                        serde_json::to_vec(&candidate).expect("draft JSON"),
                    ))
                    .expect("save request"),
            )
            .await
            .expect("save response");
        assert_eq!(saved.status(), StatusCode::OK);
        assert_eq!(saved.headers().get("cache-control").unwrap(), "no-store");
        let saved_revision = saved
            .headers()
            .get(ETAG)
            .expect("save response revision")
            .to_str()
            .expect("revision is ASCII")
            .to_string();
        assert_eq!(response_json(saved).await, serde_json::json!(candidate));
        assert_eq!(
            store
                .get_draft(
                    TenantContext::from_authenticated_session(tenant),
                    UserId::from_uuid(id(3)),
                    workspace,
                )
                .await
                .expect("saved draft lookup")
                .expect("saved draft exists")
                .record
                .revises,
            Some(prior_revision),
            "browser refresh must retain server-owned draft lineage"
        );

        let listed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/workspaces?pageSize=1")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("list request"),
            )
            .await
            .expect("list response");
        assert_eq!(listed.status(), StatusCode::OK);
        let listed = response_json(listed).await;
        assert_eq!(
            listed["items"][0]["workspace"],
            serde_json::json!(workspace)
        );
        assert_eq!(listed["items"][0]["title"], "Peptide bond draft");
        assert_eq!(listed["items"][0]["sourceBackend"], "native");
        assert!(listed["items"][0].get("problem").is_none());
        assert!(listed["items"][0].get("version").is_none());

        let refreshed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("get request"),
            )
            .await
            .expect("get response");
        assert_eq!(refreshed.status(), StatusCode::OK);
        assert_eq!(
            refreshed
                .headers()
                .get(ETAG)
                .unwrap()
                .to_str()
                .expect("revision is ASCII"),
            saved_revision
        );
        assert_eq!(response_json(refreshed).await, serde_json::json!(candidate));

        let deleted = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .header(IF_MATCH, &saved_revision)
                    .body(Body::empty())
                    .expect("delete request"),
            )
            .await
            .expect("delete response");
        assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            store
                .get_draft(
                    TenantContext::from_authenticated_session(tenant),
                    UserId::from_uuid(id(3)),
                    workspace,
                )
                .await
                .expect("draft lookup"),
            None
        );
    }

    #[tokio::test]
    async fn workspace_delete_requires_one_strong_etag() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(22));
        let workspace = WorkspaceId::from_uuid(id(23));
        let actor = UserId::from_uuid(id(24));
        let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], actor).await;
        store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                actor,
                None,
                DraftRecord {
                    tenant,
                    question: draft(workspace, "Deletion precondition fixture"),
                    revises: None,
                    derived_from: None,
                },
            )
            .await
            .expect("draft save");
        let app = test_router(Arc::clone(&store));

        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("missing precondition request"),
            )
            .await
            .expect("missing precondition response");
        assert_eq!(missing.status(), StatusCode::PRECONDITION_REQUIRED);
        assert_eq!(missing.headers().get("cache-control").unwrap(), "no-store");

        let malformed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .header(IF_MATCH, "W/\"1\"")
                    .body(Body::empty())
                    .expect("malformed precondition request"),
            )
            .await
            .expect("malformed precondition response");
        assert_eq!(malformed.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            malformed.headers().get("cache-control").unwrap(),
            "no-store"
        );

        for malformed_revision in ["\"0\"", "\"9223372036854775808\""] {
            let malformed = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("DELETE")
                        .uri(format!("/api/workspaces/{workspace}"))
                        .header("cookie", &cookie)
                        .header(IF_MATCH, malformed_revision)
                        .body(Body::empty())
                        .expect("out-of-range precondition request"),
                )
                .await
                .expect("out-of-range precondition response");
            assert_eq!(malformed.status(), StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                malformed.headers().get("cache-control").unwrap(),
                "no-store"
            );
            assert!(
                store
                    .get_draft(
                        TenantContext::from_authenticated_session(tenant),
                        actor,
                        workspace,
                    )
                    .await
                    .expect("draft lookup")
                    .is_some(),
                "malformed deletion revision must not remove the draft"
            );
        }
    }

    #[tokio::test]
    async fn authoring_route_rejects_path_mismatch_unknown_fields_and_bad_cursors() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let workspace = WorkspaceId::from_uuid(id(2));
        let other_workspace = WorkspaceId::from_uuid(id(3));
        let cookie = issued_cookie(
            &store,
            tenant,
            vec![UserRole::Publisher],
            UserId::from_uuid(id(4)),
        )
        .await;
        let app = test_router(store);
        let candidate = draft(other_workspace, "Mismatch");

        let mismatch = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&candidate).expect("draft JSON"),
                    ))
                    .expect("mismatch request"),
            )
            .await
            .expect("mismatch response");
        assert_eq!(mismatch.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let mut unknown_candidate =
            serde_json::to_value(draft(workspace, "Unknown")).expect("draft JSON value");
        unknown_candidate["metadata"]
            .as_object_mut()
            .expect("draft metadata JSON object")
            .insert("answerKey".to_string(), serde_json::json!("private-answer"));
        let unknown = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&unknown_candidate).expect("unknown draft JSON"),
                    ))
                    .expect("unknown request"),
            )
            .await
            .expect("unknown response");
        let unknown_status = unknown.status();
        let unknown_cache = unknown.headers().get("cache-control").cloned();
        assert_eq!(unknown_status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            unknown_cache.as_ref().and_then(|value| value.to_str().ok()),
            Some("no-store")
        );

        let bad_cursor = app
            .oneshot(
                Request::builder()
                    .uri("/api/workspaces?cursor=not-a-valid-cursor")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("cursor request"),
            )
            .await
            .expect("cursor response");
        assert_eq!(bad_cursor.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn students_and_foreign_tenants_cannot_enumerate_private_workspaces() {
        let store = Arc::new(MemoryStore::default());
        let owner_tenant = TenantId::from_uuid(id(1));
        let foreign_tenant = TenantId::from_uuid(id(9));
        let workspace = WorkspaceId::from_uuid(id(2));
        let owner_cookie = issued_cookie(
            &store,
            owner_tenant,
            vec![UserRole::Instructor],
            UserId::from_uuid(id(3)),
        )
        .await;
        let student_cookie = issued_cookie(
            &store,
            owner_tenant,
            vec![UserRole::Student],
            UserId::from_uuid(id(4)),
        )
        .await;
        let foreign_cookie = issued_cookie(
            &store,
            foreign_tenant,
            vec![UserRole::Instructor],
            UserId::from_uuid(id(5)),
        )
        .await;
        let second_instructor_cookie = issued_cookie(
            &store,
            owner_tenant,
            vec![UserRole::Instructor],
            UserId::from_uuid(id(6)),
        )
        .await;
        let app = test_router(Arc::clone(&store));
        let candidate = draft(workspace, "Private draft");
        let saved = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &owner_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&candidate).expect("draft JSON"),
                    ))
                    .expect("save request"),
            )
            .await
            .expect("save response");
        assert_eq!(saved.status(), StatusCode::OK);
        let current_revision = saved
            .headers()
            .get(ETAG)
            .expect("save response revision")
            .to_str()
            .expect("revision is ASCII")
            .to_string();

        let student_put = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &student_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&draft(workspace, "Student overwrite"))
                            .expect("draft JSON"),
                    ))
                    .expect("student save request"),
            )
            .await
            .expect("student save response");
        assert_eq!(student_put.status(), StatusCode::FORBIDDEN);
        let student_delete = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student delete request"),
            )
            .await
            .expect("student delete response");
        assert_eq!(student_delete.status(), StatusCode::FORBIDDEN);

        let second_instructor_put = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &second_instructor_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&draft(workspace, "Unauthorized overwrite"))
                            .expect("draft JSON"),
                    ))
                    .expect("second instructor save request"),
            )
            .await
            .expect("second instructor save response");
        assert_eq!(second_instructor_put.status(), StatusCode::NOT_FOUND);
        let second_instructor_delete = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &second_instructor_cookie)
                    .header(IF_MATCH, &current_revision)
                    .body(Body::empty())
                    .expect("second instructor delete request"),
            )
            .await
            .expect("second instructor delete response");
        assert_eq!(second_instructor_delete.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            store
                .get_draft(
                    TenantContext::from_authenticated_session(owner_tenant),
                    UserId::from_uuid(id(3)),
                    workspace,
                )
                .await
                .expect("owner draft lookup")
                .expect("owner draft remains")
                .record
                .question
                .metadata
                .title,
            "Private draft",
            "nonowners cannot mutate or delete a private workspace"
        );

        let student_list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/workspaces")
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student list request"),
            )
            .await
            .expect("student list response");
        assert_eq!(student_list.status(), StatusCode::FORBIDDEN);

        let student_get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student get request"),
            )
            .await
            .expect("student get response");
        assert_eq!(student_get.status(), StatusCode::FORBIDDEN);

        let foreign_get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &foreign_cookie)
                    .body(Body::empty())
                    .expect("foreign get request"),
            )
            .await
            .expect("foreign get response");
        assert_eq!(foreign_get.status(), StatusCode::NOT_FOUND);

        let foreign_delete = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &foreign_cookie)
                    .header(IF_MATCH, &current_revision)
                    .body(Body::empty())
                    .expect("foreign delete request"),
            )
            .await
            .expect("foreign delete response");
        assert_eq!(foreign_delete.status(), StatusCode::NOT_FOUND);

        let foreign_list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/workspaces")
                    .header("cookie", &foreign_cookie)
                    .body(Body::empty())
                    .expect("foreign list request"),
            )
            .await
            .expect("foreign list response");
        assert_eq!(foreign_list.status(), StatusCode::OK);
        assert_eq!(
            response_json(foreign_list).await["items"],
            serde_json::json!([])
        );
    }

    #[tokio::test]
    async fn workspace_save_requires_a_fresh_revision_and_preserves_newer_content() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(40));
        let workspace = WorkspaceId::from_uuid(id(41));
        let actor = UserId::from_uuid(id(42));
        let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], actor).await;
        let app = test_router(Arc::clone(&store));
        let original = draft(workspace, "Original");

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&original).expect("draft JSON"),
                    ))
                    .expect("create request"),
            )
            .await
            .expect("create response");
        assert_eq!(created.status(), StatusCode::OK);
        let stale_revision = created
            .headers()
            .get(ETAG)
            .expect("create response revision")
            .to_str()
            .expect("revision is ASCII")
            .to_string();

        let newer = draft(workspace, "Newer author edit");
        let fresh = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .header(IF_MATCH, &stale_revision)
                    .body(Body::from(serde_json::to_vec(&newer).expect("draft JSON")))
                    .expect("fresh save request"),
            )
            .await
            .expect("fresh save response");
        assert_eq!(fresh.status(), StatusCode::OK);

        let stale = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .header(IF_MATCH, stale_revision)
                    .body(Body::from(
                        serde_json::to_vec(&draft(workspace, "Stale overwrite"))
                            .expect("draft JSON"),
                    ))
                    .expect("stale save request"),
            )
            .await
            .expect("stale save response");
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        assert_eq!(stale.headers().get("cache-control").unwrap(), "no-store");
        assert_eq!(
            store
                .get_draft(
                    TenantContext::from_authenticated_session(tenant),
                    actor,
                    workspace
                )
                .await
                .expect("owner lookup")
                .expect("workspace remains")
                .record
                .question
                .metadata
                .title,
            "Newer author edit"
        );
    }

    #[tokio::test]
    async fn invited_collaborator_can_read_and_save_with_the_issued_revision() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(45));
        let workspace = WorkspaceId::from_uuid(id(46));
        let owner = UserId::from_uuid(id(47));
        let collaborator = UserId::from_uuid(id(48));
        let owner_cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], owner).await;
        let collaborator_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], collaborator).await;
        store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                owner,
                None,
                DraftRecord {
                    tenant,
                    question: draft(workspace, "Owner draft"),
                    revises: None,
                    derived_from: None,
                },
            )
            .await
            .expect("owner draft creation");
        store
            .grant_draft_collaborator(
                TenantContext::from_authenticated_session(tenant),
                owner,
                workspace,
                collaborator,
            )
            .await
            .expect("owner invitation");
        let app = test_router(Arc::clone(&store));

        let loaded = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &collaborator_cookie)
                    .body(Body::empty())
                    .expect("collaborator get request"),
            )
            .await
            .expect("collaborator get response");
        assert_eq!(loaded.status(), StatusCode::OK);
        let revision = loaded
            .headers()
            .get(ETAG)
            .expect("collaborator read revision")
            .to_str()
            .expect("revision is ASCII")
            .to_string();

        let saved = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &collaborator_cookie)
                    .header("content-type", "application/json")
                    .header(IF_MATCH, &revision)
                    .body(Body::from(
                        serde_json::to_vec(&draft(workspace, "Collaborator revision"))
                            .expect("draft JSON"),
                    ))
                    .expect("collaborator save request"),
            )
            .await
            .expect("collaborator save response");
        assert_eq!(saved.status(), StatusCode::OK);
        assert_eq!(saved.headers().get("cache-control").unwrap(), "no-store");
        let collaborator_revision = saved
            .headers()
            .get(ETAG)
            .expect("collaborator save revision")
            .to_str()
            .expect("revision is ASCII")
            .to_string();

        let stale_owner_delete = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &owner_cookie)
                    .header(IF_MATCH, &revision)
                    .body(Body::empty())
                    .expect("stale owner delete request"),
            )
            .await
            .expect("stale owner delete response");
        assert_eq!(stale_owner_delete.status(), StatusCode::CONFLICT);

        let collaborator_delete = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &collaborator_cookie)
                    .header(IF_MATCH, &collaborator_revision)
                    .body(Body::empty())
                    .expect("collaborator delete request"),
            )
            .await
            .expect("collaborator delete response");
        assert_eq!(collaborator_delete.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            store
                .get_draft(
                    TenantContext::from_authenticated_session(tenant),
                    owner,
                    workspace
                )
                .await
                .expect("owner lookup")
                .expect("owner workspace remains")
                .record
                .question
                .metadata
                .title,
            "Collaborator revision"
        );

        // Keep the owner session exercised as a reminder that the actor is a
        // persisted ACL input, not whichever authoring role happened to issue
        // the last save.
        let owner_list = app
            .oneshot(
                Request::builder()
                    .uri("/api/workspaces")
                    .header("cookie", &owner_cookie)
                    .body(Body::empty())
                    .expect("owner list request"),
            )
            .await
            .expect("owner list response");
        assert_eq!(owner_list.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn workspace_body_limit_rejects_without_storing_or_caching() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(50));
        let workspace = WorkspaceId::from_uuid(id(51));
        let actor = UserId::from_uuid(id(52));
        let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], actor).await;
        let app = test_router(Arc::clone(&store));
        let oversized = format!("\"{}\"", "x".repeat(MAX_WORKSPACE_BODY_BYTES));

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(oversized))
                    .expect("oversized request"),
            )
            .await
            .expect("oversized response");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
        assert_eq!(
            store
                .get_draft(
                    TenantContext::from_authenticated_session(tenant),
                    actor,
                    workspace
                )
                .await
                .expect("workspace lookup"),
            None
        );
    }

    #[tokio::test]
    async fn publication_validation_and_diff_use_persisted_draft_safe_semantics() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(60));
        let actor = UserId::from_uuid(id(61));
        let prior_workspace = WorkspaceId::from_uuid(id(62));
        let workspace = WorkspaceId::from_uuid(id(63));
        let prior_reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(id(64)),
            version: VersionId::from_uuid(id(65)),
        };
        let context = TenantContext::from_authenticated_session(tenant);
        let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], actor).await;
        let prior_draft = DraftRecord {
            tenant,
            question: draft(prior_workspace, "Earlier peptide question"),
            revises: None,
            derived_from: None,
        };
        let saved = store
            .upsert_draft(context, actor, None, prior_draft.clone())
            .await
            .expect("prior draft save");
        store
            .publish_draft(
                context,
                actor,
                store::PublishDraftCommand {
                    expected_draft: prior_draft,
                    expected_revision: saved.revision,
                    publication: prior_reference,
                    published_source: question_model::QuestionSource::Native {
                        family: "workspace-fixture".to_string(),
                    },
                    source_artifact: None,
                    qti_promotion: None,
                    publisher: actor,
                    scope: question_model::PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await
            .expect("prior publication");
        let mut revised_question = draft(workspace, "Revised peptide question");
        revised_question.timing_policy = TimingPolicy::PerQuestion {
            seconds: 90,
            grace_seconds: 5,
        };
        store
            .upsert_draft(
                context,
                actor,
                None,
                DraftRecord {
                    tenant,
                    question: revised_question,
                    revises: Some(prior_reference),
                    derived_from: None,
                },
            )
            .await
            .expect("revision draft save");
        let app = test_router(Arc::clone(&store));

        let validation = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/workspaces/{workspace}/publication-validation"
                    ))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("validation request"),
            )
            .await
            .expect("validation response");
        assert_eq!(validation.status(), StatusCode::OK);
        assert_eq!(
            validation.headers().get("cache-control").unwrap(),
            "no-store"
        );
        assert_eq!(validation.headers().get(ETAG).unwrap(), "\"1\"");
        assert_eq!(
            response_json(validation).await,
            serde_json::json!({
                "violations": [
                    {
                        "workspace": workspace,
                        "title": "Revised peptide question",
                        "capability": "hints"
                    },
                    {
                        "workspace": workspace,
                        "title": "Revised peptide question",
                        "capability": "perQuestionTiming"
                    }
                ]
            })
        );

        let nonempty_validation = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/workspaces/{workspace}/publication-validation"
                    ))
                    .header("cookie", &cookie)
                    .body(Body::from("{}"))
                    .expect("nonempty validation request"),
            )
            .await
            .expect("nonempty validation response");
        assert_eq!(nonempty_validation.status(), StatusCode::BAD_REQUEST);

        let diff = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/workspaces/{workspace}/publication-diff"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("diff request"),
            )
            .await
            .expect("diff response");
        assert_eq!(diff.status(), StatusCode::OK);
        assert_eq!(diff.headers().get(ETAG).unwrap(), "\"1\"");
        let diff = response_json(diff).await;
        assert_eq!(diff["draftRevision"], 1);
        assert_eq!(diff["baseline"], "revision");
        assert_eq!(diff["prior"], serde_json::json!(prior_reference));
        assert_eq!(diff["current"]["sourceBackend"], "native");
        assert!(
            diff["changed"]
                .as_array()
                .expect("changed fields")
                .contains(&serde_json::json!("title"))
        );
        assert!(
            diff["changed"]
                .as_array()
                .expect("changed fields")
                .contains(&serde_json::json!("timingPolicy"))
        );
        let serialized = diff.to_string();
        for forbidden in [
            r#""source":"#,
            r#""family":"#,
            r#""provider":"#,
            r#""itemRef":"#,
            r#""grading":"#,
            r#""answerKey":"#,
            r#""artifact":"#,
        ] {
            assert!(
                !serialized.contains(forbidden),
                "semantic diff leaked forbidden field {forbidden}"
            );
        }
    }

    #[tokio::test]
    async fn external_publication_validation_refuses_without_publishing_or_changing_draft() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(66));
        let actor = UserId::from_uuid(id(67));
        let workspace = WorkspaceId::from_uuid(id(68));
        let context = TenantContext::from_authenticated_session(tenant);
        let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], actor).await;
        let mut question = draft(workspace, "iMathAS source snapshot fixture");
        question.source = DraftQuestionSource::Imathas {
            provider: "institution-imathas".to_string(),
            item_ref: "1842".to_string(),
        };
        let candidate = DraftRecord {
            tenant,
            question,
            revises: None,
            derived_from: None,
        };
        store
            .upsert_draft(context, actor, None, candidate.clone())
            .await
            .expect("external draft save");
        let app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([
                    Capability::ServerGrading,
                    Capability::Hints,
                ]),
            }),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/workspaces/{workspace}/publication-validation"
                    ))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("external validation request"),
            )
            .await
            .expect("external validation response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
        assert!(
            store
                .list_catalog(
                    context,
                    PageRequest::first(PageSize::new(10).expect("valid page size")),
                )
                .await
                .expect("catalog listing")
                .items
                .is_empty(),
            "publication validation must not mint a catalog record"
        );
        assert_eq!(
            store
                .get_draft(context, actor, workspace)
                .await
                .expect("external draft lookup")
                .map(|draft| draft.record),
            Some(candidate),
            "publication validation must leave its stored draft unchanged"
        );
    }
}
