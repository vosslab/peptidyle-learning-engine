//! Authenticated catalog, publication, and lifecycle routes (MOD-API-CAT).
//!
//! The route layer resolves capabilities from a server-owned registry, runs
//! the shared domain validator, and passes the exact validated draft into one
//! atomic store transaction. A browser never declares its own capabilities or
//! chooses a new `ProblemId`.

use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use domain::policy::{AssignmentConfig, AssignmentQuestionConfig, Violation};
use question_model::{
    BackendCapabilities, ProblemId, ProblemVersionRef, PublicationScope, QuestionSource, UserId,
    UserRole, VersionId, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use store::{
    CatalogStore, CatalogTransition, Cursor, DraftRecord, PageRequest, PageSize, PaginationError,
    PublishDraftCommand, SessionStore, Store, StoreError, TenantContext,
};

use crate::auth::{auth_error_response, no_store, resolve_request_session};

const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_CATALOG_BODY_BYTES: usize = 64 * 1_024;

/// Resolves trusted capabilities for the adapter owning one question.
pub trait BackendRegistry: Send + Sync {
    /// Returns the server's capability declaration for this source.
    fn capabilities(
        &self,
        source: &QuestionSource,
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
        .route(
            "/api/problems/{problem}/versions/{version}",
            get(get_problem::<S, B, R>),
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
struct CatalogQuery {
    cursor: Option<String>,
    page_size: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishProblemRequest {
    scope: PublicationScope,
}

#[derive(Debug, Deserialize)]
struct DeprecateProblemRequest {
    reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationValidationFailure {
    error: &'static str,
    violations: Vec<Violation>,
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
    let draft = match state
        .store
        .get_draft(authenticated.tenant_context, workspace)
        .await
    {
        Ok(Some(draft)) => draft,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "draft not found"),
        Err(error) => return store_error_response(error),
    };
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
    let violations = domain::policy::validate_assignment_config(&AssignmentConfig {
        questions: vec![AssignmentQuestionConfig {
            question: draft.question.clone(),
            backend_capabilities: capabilities.clone(),
        }],
        required_capabilities: Vec::new(),
    });
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
    let publisher = authenticated.record.subject.user();
    if request.scope == PublicationScope::Public {
        match state
            .review_gate
            .allows_publication(authenticated.tenant_context, publisher, &draft)
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
    let problem = draft
        .revises
        .map_or_else(ProblemId::generate, |reference| reference.problem);
    let command = PublishDraftCommand {
        expected_draft: draft,
        problem,
        publisher,
        scope: request.scope,
        capabilities,
    };
    match state
        .store
        .publish_draft(authenticated.tenant_context, command)
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

fn may_publish(roles: &[UserRole], scope: PublicationScope) -> bool {
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

fn store_error_response(error: StoreError) -> Response {
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
        StoreError::Unavailable(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "catalog unavailable")
        }
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use question_model::answer::NumericTolerance;
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::response::ResponseDefinition;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::{License, TaxonomyTerm};
    use question_model::{
        ActivityTimestamp, Capability, GradingDefinition, QuestionDefinition, QuestionMetadata,
        TenantId,
    };
    use store::memory::MemoryStore;
    use store::{SessionLifetime, SessionSubject};
    use tower::ServiceExt;
    use uuid::Uuid;

    #[derive(Clone)]
    struct FixtureRegistry {
        capabilities: BackendCapabilities,
    }

    struct ReviewRequired;

    impl BackendRegistry for FixtureRegistry {
        fn capabilities(
            &self,
            _source: &QuestionSource,
        ) -> Result<BackendCapabilities, BackendRegistryError> {
            Ok(self.capabilities.clone())
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

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn draft(tenant: TenantId, workspace: WorkspaceId, version: VersionId) -> DraftRecord {
        DraftRecord {
            tenant,
            question: QuestionDefinition {
                version,
                problem: None,
                workspace,
                source: QuestionSource::Native {
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
        let mut candidate = draft(tenant, workspace, version);
        candidate.question.grading = GradingDefinition::PartialCredit { points: 1.0 };
        store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                candidate.clone(),
            )
            .await
            .expect("draft save");
        let publisher = UserId::from_uuid(id(4));
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
                    "question": version,
                    "capability": "serverGrading"
                },
                {
                    "question": version,
                    "capability": "partialCredit"
                }
            ])
        );
        let still_draft = store
            .get_draft(TenantContext::from_authenticated_session(tenant), workspace)
            .await
            .expect("draft lookup")
            .expect("validation failure retains draft");
        assert_eq!(still_draft.question.problem, None);

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
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"public"}"#))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");
        assert_eq!(published.status(), StatusCode::CREATED);
        let published = response_json(published).await;
        assert_ne!(published["problem"], serde_json::Value::Null);
        assert_eq!(published["version"], serde_json::json!(version));
        assert_eq!(published["workspace"], serde_json::json!(workspace));
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
            store
                .upsert_draft(context, draft(tenant, workspace, version))
                .await
                .expect("draft save");
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/problems/{workspace}/publish"))
                        .header("cookie", &cookie)
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
}
