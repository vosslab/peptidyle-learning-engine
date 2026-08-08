//! Authenticated course and assignment routes (MOD-API-COURSE).
//!
//! Sessions establish the tenant and authenticated user. Course membership is
//! a separate tenant record, so a coarse instructor role does not grant access
//! to unrelated courses. Assignment requests carry exact immutable
//! `(ProblemId, VersionId)` references and never copy question payloads.

use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use question_model::{
    AssignmentId, AssignmentSummary, Capability, CourseId, CourseMembership, CourseMembershipRole,
    CourseRole, ProblemVersionRef, RunPolicies, UserRole,
};
use serde::{Deserialize, Serialize};
use store::{
    AssignmentRecord, AssignmentRevision, CatalogStore, CourseListScope, CourseRecord, Cursor,
    Page, PageRequest, PageSize, PaginationError, SessionStore, Store, StoreError,
    StoredAssignment,
};

use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};

const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_COURSE_BODY_BYTES: usize = 64 * 1_024;

/// Builds the authenticated course and assignment route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    Router::new()
        .route(
            "/api/courses",
            get(list_courses::<S>).post(create_course::<S>),
        )
        .route(
            "/api/courses/{course}/assignments",
            get(list_assignments::<S>).post(create_assignment::<S>),
        )
        .route("/api/courses/{course}/gradebook", get(list_gradebook::<S>))
        .route("/api/courses/{course}", get(get_course::<S>))
        .route("/api/assignments/{assignment}", get(get_assignment::<S>))
        .route(
            "/api/courses/{course}/assignments/{assignment}",
            put(update_assignment::<S>),
        )
        .layer(DefaultBodyLimit::max(MAX_COURSE_BODY_BYTES))
        .with_state(CourseRouteState { store })
}

struct CourseRouteState<S> {
    store: Arc<S>,
}

impl<S> Clone for CourseRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct CourseQuery {
    cursor: Option<String>,
    page_size: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCourseRequest {
    title: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct CreateAssignmentRequest {
    title: String,
    problems: Vec<ProblemVersionRef>,
    policies: RunPolicies,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateAssignmentRequest {
    title: String,
    problems: Vec<ProblemVersionRef>,
    policies: RunPolicies,
}

/// Rejects unknown fields at every level by comparing the request to the
/// canonical wire form of the typed model, mirroring the workspace boundary.
fn strict_assignment_request<T>(value: serde_json::Value) -> Result<T, ()>
where
    T: serde::de::DeserializeOwned + Serialize,
{
    let request = serde_json::from_value(value.clone()).map_err(|_| ())?;
    if serde_json::to_value(&request).map_err(|_| ())? == value {
        Ok(request)
    } else {
        Err(())
    }
}

async fn list_courses<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Query(query): Query<CourseQuery>,
) -> Response
where
    S: Store + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    let scope = if is_tenant_administrator(&authenticated) {
        CourseListScope::TenantAdministrator
    } else {
        CourseListScope::Member(authenticated.record.subject.user())
    };
    match state
        .store
        .list_courses(authenticated.tenant_context, scope, page)
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn create_course<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<CreateCourseRequest>,
) -> Response
where
    S: Store + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_create_course(&authenticated) {
        return error_response(StatusCode::FORBIDDEN, "course creation is not authorized");
    }
    let course = CourseRecord {
        id: CourseId::generate(),
        tenant: authenticated.tenant_context.tenant_id(),
        title: request.title,
        members: vec![CourseMembership {
            user: authenticated.record.subject.user(),
            role: CourseMembershipRole::Instructor,
        }],
    };
    match state
        .store
        .upsert_course(authenticated.tenant_context, course.clone())
        .await
    {
        Ok(()) => no_store(
            (
                StatusCode::CREATED,
                Json(course.summary(CourseRole::Instructor)),
            )
                .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn get_course<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
) -> Response
where
    S: Store + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let record = match state
        .store
        .get_course(authenticated.tenant_context, course)
        .await
    {
        Ok(Some(record)) => record,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course not found"),
        Err(error) => return store_error_response(error),
    };
    let role = if is_tenant_administrator(&authenticated) {
        CourseRole::Administrator
    } else if let Some(role) = record.role_for(authenticated.record.subject.user()) {
        role
    } else {
        return error_response(StatusCode::NOT_FOUND, "course not found");
    };
    no_store(Json(record.summary(role)).into_response())
}

async fn list_assignments<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Query(query): Query<CourseQuery>,
) -> Response
where
    S: Store + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, false).await
    {
        return response;
    }
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    match state
        .store
        .list_assignments(authenticated.tenant_context, course, page)
        .await
    {
        Ok(page) => no_store(Json(assignment_page(page)).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn create_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Json(value): Json<serde_json::Value>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
    }
    let request = match strict_assignment_request::<CreateAssignmentRequest>(value) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment request is invalid",
            );
        }
    };
    let assignment = AssignmentRecord {
        id: AssignmentId::generate(),
        tenant: authenticated.tenant_context.tenant_id(),
        course_id: course,
        title: request.title,
        problems: request.problems,
        policies: request.policies,
    };
    if let Err(response) =
        validate_assignment_request(&state, authenticated.tenant_context, &assignment).await
    {
        return response;
    }
    match state
        .store
        .create_assignment(authenticated.tenant_context, assignment)
        .await
    {
        Ok(assignment) => assignment_response(StatusCode::CREATED, assignment),
        Err(error) => store_error_response(error),
    }
}

/// Lists the compact, browser-safe gradebook projection for one managed course.
///
/// The store owns the bounded assignment/enrollment/summary join.  This route
/// intentionally neither loads historical runs nor accepts student or tenant
/// identifiers as authority inputs.
async fn list_gradebook<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Query(query): Query<CourseQuery>,
) -> Response
where
    S: Store + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
    }
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    match state
        .store
        .list_gradebook_rows(authenticated.tenant_context, course, page)
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn get_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(assignment): Path<AssignmentId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let assignment = match state
        .store
        .get_assignment_for_edit(authenticated.tenant_context, assignment)
        .await
    {
        Ok(Some(assignment)) => assignment,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    if let Err(response) = require_course_access(
        state.store.as_ref(),
        &authenticated,
        assignment.record.course_id,
        false,
    )
    .await
    {
        return response;
    }
    assignment_response(StatusCode::OK, assignment)
}

async fn update_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
    Json(value): Json<serde_json::Value>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
    }
    let expected_revision = match required_assignment_revision(&headers) {
        Ok(revision) => revision,
        Err(AssignmentRevisionHeaderError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match assignment revision is required",
            );
        }
        Err(AssignmentRevisionHeaderError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match assignment revision is invalid",
            );
        }
    };
    let request = match strict_assignment_request::<UpdateAssignmentRequest>(value) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment request is invalid",
            );
        }
    };
    let replacement = AssignmentRecord {
        id: assignment,
        tenant: authenticated.tenant_context.tenant_id(),
        course_id: course,
        title: request.title.clone(),
        problems: request.problems.clone(),
        policies: request.policies,
    };
    if let Err(response) =
        validate_assignment_request(&state, authenticated.tenant_context, &replacement).await
    {
        return response;
    }
    match state
        .store
        .replace_assignment(
            authenticated.tenant_context,
            course,
            assignment,
            expected_revision,
            store::AssignmentUpdate {
                title: request.title,
                problems: request.problems,
                policies: request.policies,
            },
        )
        .await
    {
        Ok(assignment) => assignment_response(StatusCode::OK, assignment),
        Err(StoreError::Conflict) => {
            error_response(StatusCode::CONFLICT, "assignment changed; reload it")
        }
        Err(error) => store_error_response(error),
    }
}

async fn require_course_access<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
    manage: bool,
) -> Result<(), Response>
where
    S: Store,
{
    let record = match store.get_course(authenticated.tenant_context, course).await {
        Ok(Some(record)) => record,
        Ok(None) => return Err(error_response(StatusCode::NOT_FOUND, "course not found")),
        Err(error) => return Err(store_error_response(error)),
    };
    let role = if is_tenant_administrator(authenticated) {
        Some(CourseRole::Administrator)
    } else {
        record.role_for(authenticated.record.subject.user())
    };
    match (role, manage) {
        (Some(CourseRole::Instructor | CourseRole::Administrator), _)
        | (Some(CourseRole::Student), false) => Ok(()),
        (Some(CourseRole::Student), true) => Err(error_response(
            StatusCode::FORBIDDEN,
            "assignment change is not authorized",
        )),
        (None, _) => Err(error_response(StatusCode::NOT_FOUND, "course not found")),
    }
}

fn may_create_course(authenticated: &AuthenticatedSession) -> bool {
    authenticated
        .record
        .subject
        .roles()
        .iter()
        .any(|role| matches!(role, UserRole::Instructor | UserRole::Administrator))
}

fn is_tenant_administrator(authenticated: &AuthenticatedSession) -> bool {
    authenticated
        .record
        .subject
        .roles()
        .contains(&UserRole::Administrator)
}

fn page_request(query: CourseQuery) -> Result<PageRequest, PaginationError> {
    let size = PageSize::new(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))?;
    match query.cursor {
        Some(cursor) => Ok(PageRequest::after(Cursor::parse(cursor)?, size)),
        None => Ok(PageRequest::first(size)),
    }
}

fn assignment_page(page: Page<AssignmentRecord>) -> Page<AssignmentSummary> {
    Page {
        items: page
            .items
            .into_iter()
            .map(|assignment| assignment.summary())
            .collect(),
        next_cursor: page.next_cursor,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssignmentRevisionHeaderError {
    Missing,
    Malformed,
}

fn required_assignment_revision(
    headers: &HeaderMap,
) -> Result<AssignmentRevision, AssignmentRevisionHeaderError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(AssignmentRevisionHeaderError::Missing);
    };
    if values.next().is_some() {
        return Err(AssignmentRevisionHeaderError::Malformed);
    }
    let value = value
        .to_str()
        .map_err(|_| AssignmentRevisionHeaderError::Malformed)?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(AssignmentRevisionHeaderError::Malformed);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AssignmentRevisionHeaderError::Malformed);
    }
    let numeric = value
        .parse::<u64>()
        .map_err(|_| AssignmentRevisionHeaderError::Malformed)?;
    if numeric == 0 || numeric > i64::MAX as u64 {
        return Err(AssignmentRevisionHeaderError::Malformed);
    }
    serde_json::from_str(value).map_err(|_| AssignmentRevisionHeaderError::Malformed)
}

fn assignment_response(status: StatusCode, assignment: StoredAssignment) -> Response {
    let mut response = (status, Json(assignment.record.summary())).into_response();
    let etag = format!("\"{}\"", assignment.revision.value());
    let header = HeaderValue::from_str(&etag).expect("positive revision produces a valid ETag");
    response.headers_mut().insert(ETAG, header);
    no_store(response)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentCapabilityViolation {
    title: String,
    reference: ProblemVersionRef,
    capability: Capability,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentValidationFailure {
    error: &'static str,
    violations: Vec<AssignmentCapabilityViolation>,
}

async fn validate_assignment_request<S>(
    state: &CourseRouteState<S>,
    context: store::TenantContext,
    assignment: &AssignmentRecord,
) -> Result<(), Response>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let mut selected = Vec::with_capacity(assignment.problems.len());
    let mut titles = std::collections::BTreeMap::new();
    for reference in &assignment.problems {
        let Some(published) = state
            .store
            .get_catalog_problem(context, *reference)
            .await
            .map_err(store_error_response)?
        else {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment references a missing or hidden published version",
            ));
        };
        if !published.lifecycle.is_assignable() {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment references a nonassignable published version",
            ));
        }
        titles.insert(*reference, published.question.metadata.title.clone());
        selected.push(domain::policy::AssignmentQuestionConfig {
            question: published.question,
            backend_capabilities: published.capabilities,
        });
    }
    let violations =
        domain::policy::validate_assignment_config(&domain::policy::AssignmentConfig {
            questions: selected,
            required_capabilities: Vec::new(),
        })
        .into_iter()
        .map(|violation| {
            let reference = assignment
                .problems
                .iter()
                .find(|reference| reference.version == violation.question)
                .expect("domain validation only reports a selected question version");
            let title = titles
                .get(reference)
                .expect("every selected question has its immutable title")
                .clone();
            AssignmentCapabilityViolation {
                title,
                reference: *reference,
                capability: violation.capability,
            }
        })
        .collect::<Vec<_>>();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(no_store(
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(AssignmentValidationFailure {
                    error: "assignment configuration is not supported",
                    violations,
                }),
            )
                .into_response(),
        ))
    }
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound => error_response(StatusCode::NOT_FOUND, "record not found"),
        StoreError::AlreadyExists => error_response(StatusCode::CONFLICT, "record already exists"),
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
        StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "course storage unavailable",
        ),
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
    use question_model::run_policy::{
        AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
        TimingPolicy, VariationPolicy,
    };
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, BackendCapabilities, Capability, DraftQuestionDefinition,
        DraftQuestionSource, GradingDefinition, ProblemId, PublicationScope, QuestionMetadata,
        QuestionSource, StudentId, TenantId, UserId, VersionId, WorkspaceId,
    };
    use store::memory::MemoryStore;
    use store::{
        CatalogStore, DraftRecord, PublishDraftCommand, SessionLifetime, SessionSubject,
        TenantContext,
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    async fn issued_cookie(store: &MemoryStore, roles: Vec<UserRole>, user: UserId) -> String {
        issued_cookie_for_tenant(store, TenantId::from_uuid(id(1)), roles, user).await
    }

    async fn issued_cookie_for_tenant(
        store: &MemoryStore,
        tenant: TenantId,
        roles: Vec<UserRole>,
        user: UserId,
    ) -> String {
        let subject =
            SessionSubject::new(tenant, user, "Course Fixture", roles).expect("fixture identity");
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

    async fn publish_fixture(
        store: &MemoryStore,
        context: TenantContext,
        tenant: TenantId,
        publisher: UserId,
    ) -> ProblemVersionRef {
        let problem = ProblemId::from_uuid(id(20));
        let version = VersionId::from_uuid(id(21));
        let workspace = WorkspaceId::from_uuid(id(22));
        let draft = DraftRecord {
            tenant,
            question: DraftQuestionDefinition {
                workspace,
                source: DraftQuestionSource::Native {
                    family: "course-fixture".to_string(),
                },
                prompt: vec![ContentBlock::Text {
                    markdown: "What is a peptide bond?".to_string(),
                }],
                response: ResponseDefinition::Numeric {
                    tolerance: NumericTolerance::Absolute { epsilon: 0.0 },
                    unit: None,
                },
                attempt_policy: AttemptPolicy {
                    max_attempts: None,
                    feedback: FeedbackDisclosure::ImmediateFull,
                },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Static,
                grading: GradingDefinition::AllOrNothing { points: 1.0 },
                metadata: QuestionMetadata {
                    title: "Peptide bond fixture".to_string(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::CcBySa,
                    language: "en-US".to_string(),
                },
            },
            revises: None,
            derived_from: None,
        };
        let saved = store
            .upsert_draft(context, publisher, None, draft.clone())
            .await
            .expect("draft save");
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved.revision,
                    publication: question_model::ProblemVersionRef { problem, version },
                    published_source: QuestionSource::Native {
                        family: "course-fixture".to_string(),
                    },
                    source_artifact: None,
                    qti_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await
            .expect("fixture publication");
        ProblemVersionRef { problem, version }
    }

    fn policies() -> RunPolicies {
        RunPolicies {
            completion: CompletionRequirement::AllCorrect,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        }
    }

    #[tokio::test]
    async fn membership_scopes_courses_and_exact_assignment_references_survive() {
        let store = Arc::new(MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
            .expect("fixture clock");
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(id(2));
        let student = UserId::from_uuid(id(3));
        let outsider = UserId::from_uuid(id(4));
        let administrator = UserId::from_uuid(id(5));
        let foreign_tenant = TenantId::from_uuid(id(6));
        let foreign_user = UserId::from_uuid(id(7));
        let instructor_cookie = issued_cookie(&store, vec![UserRole::Instructor], instructor).await;
        let student_cookie = issued_cookie(&store, vec![UserRole::Student], student).await;
        let outsider_cookie = issued_cookie(&store, vec![UserRole::Instructor], outsider).await;
        let administrator_cookie =
            issued_cookie(&store, vec![UserRole::Administrator], administrator).await;
        let foreign_cookie = issued_cookie_for_tenant(
            &store,
            foreign_tenant,
            vec![UserRole::Instructor],
            foreign_user,
        )
        .await;
        let app = router(Arc::clone(&store));

        let created_course = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/courses")
                    .header("cookie", &instructor_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"BIOC 301: Biochemistry"}"#))
                    .expect("course request"),
            )
            .await
            .expect("course response");
        assert_eq!(created_course.status(), StatusCode::CREATED);
        let created_course = response_json(created_course).await;
        let course: CourseId =
            serde_json::from_value(created_course["id"].clone()).expect("course ID response");
        assert_eq!(created_course["role"], "instructor");

        let mut course_record = store
            .get_course(context, course)
            .await
            .expect("course lookup")
            .expect("course exists");
        course_record.members.push(CourseMembership {
            user: student,
            role: CourseMembershipRole::Student,
        });
        store
            .upsert_course(context, course_record)
            .await
            .expect("student membership save");
        let reference = publish_fixture(&store, context, tenant, instructor).await;

        let assignment_request = CreateAssignmentRequest {
            title: "Peptide bond mastery".to_string(),
            problems: vec![reference],
            policies: policies(),
        };
        let created_assignment = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/courses/{course}/assignments"))
                    .header("cookie", &instructor_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&assignment_request)
                            .expect("assignment request serialization"),
                    ))
                    .expect("assignment request"),
            )
            .await
            .expect("assignment response");
        assert_eq!(created_assignment.status(), StatusCode::CREATED);
        let assignment_etag = created_assignment
            .headers()
            .get(ETAG)
            .expect("created assignment ETag")
            .to_str()
            .expect("ASCII ETag")
            .to_string();
        let created_assignment = response_json(created_assignment).await;
        let assignment: AssignmentId = serde_json::from_value(created_assignment["id"].clone())
            .expect("assignment ID response");
        assert_eq!(created_assignment["courseId"], serde_json::json!(course));
        assert_eq!(
            created_assignment["problems"],
            serde_json::json!([reference]),
            "the course artifact must retain exact IDs rather than copy a question"
        );

        for request in [
            Request::builder()
                .uri(format!("/api/assignments/{assignment}"))
                .header("cookie", &foreign_cookie)
                .body(Body::empty())
                .expect("foreign exact request"),
            Request::builder()
                .method("PUT")
                .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                .header("cookie", &foreign_cookie)
                .header(IF_MATCH, &assignment_etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "foreign", "problems": [reference], "policies": policies(),
                    })
                    .to_string(),
                ))
                .expect("foreign update request"),
            Request::builder()
                .method("PUT")
                .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                .header("cookie", &foreign_cookie)
                .header(IF_MATCH, "W/\"1\"")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "foreign malformed", "problems": [reference], "policies": policies(),
                    })
                    .to_string(),
                ))
                .expect("foreign malformed update request"),
        ] {
            assert_eq!(
                app.clone()
                    .oneshot(request)
                    .await
                    .expect("foreign response")
                    .status(),
                StatusCode::NOT_FOUND,
                "foreign tenant must not enumerate an assignment"
            );
        }

        let nested_unknown = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                    .header("cookie", &instructor_cookie)
                    .header(IF_MATCH, &assignment_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({
                        "title": "Peptide bond mastery",
                        "problems": [{"problem": reference.problem, "version": reference.version, "capabilities": ["serverGrading"]}],
                        "policies": policies(),
                    }).to_string()))
                    .expect("nested unknown request"),
            )
            .await
            .expect("nested unknown response");
        assert_eq!(nested_unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let updated = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                    .header("cookie", &instructor_cookie)
                    .header(IF_MATCH, &assignment_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "Peptide bond mastery revised",
                            "problems": [reference],
                            "policies": policies(),
                        })
                        .to_string(),
                    ))
                    .expect("assignment update request"),
            )
            .await
            .expect("assignment update response");
        assert_eq!(updated.status(), StatusCode::OK);
        let updated_etag = updated.headers().get(ETAG).expect("updated ETag");
        assert_ne!(updated_etag.to_str().expect("ASCII ETag"), assignment_etag);

        let stale = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                    .header("cookie", &instructor_cookie)
                    .header(IF_MATCH, &assignment_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({
                        "title": "stale overwrite", "problems": [reference], "policies": policies(),
                    }).to_string()))
                    .expect("stale update request"),
            )
            .await
            .expect("stale update response");
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        assert_eq!(
            store
                .get_assignment(context, assignment)
                .await
                .expect("stored assignment")
                .expect("assignment")
                .title,
            "Peptide bond mastery revised"
        );

        let administrator_get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/assignments/{assignment}"))
                    .header("cookie", &administrator_cookie)
                    .body(Body::empty())
                    .expect("administrator assignment request"),
            )
            .await
            .expect("administrator assignment response");
        assert_eq!(administrator_get.status(), StatusCode::OK);
        let administrator_etag = administrator_get
            .headers()
            .get(ETAG)
            .expect("administrator ETag");
        let administrator_update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                    .header("cookie", &administrator_cookie)
                    .header(IF_MATCH, administrator_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({
                        "title": "Administrator revised", "problems": [reference], "policies": policies(),
                    }).to_string()))
                    .expect("administrator update request"),
            )
            .await
            .expect("administrator update response");
        assert_eq!(administrator_update.status(), StatusCode::OK);

        let wrong_course = CourseId::from_uuid(id(99));
        store
            .upsert_course(
                context,
                CourseRecord {
                    id: wrong_course,
                    tenant,
                    title: "BIOC 399: Wrong course".to_string(),
                    members: vec![CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    }],
                },
            )
            .await
            .expect("wrong-course fixture");
        let wrong_course_update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{wrong_course}/assignments/{assignment}"))
                    .header("cookie", &instructor_cookie)
                    .header(IF_MATCH, updated_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({
                        "title": "must not move course", "problems": [reference], "policies": policies(),
                    }).to_string()))
                    .expect("wrong-course update request"),
            )
            .await
            .expect("wrong-course update response");
        assert_eq!(wrong_course_update.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            store
                .get_assignment(context, assignment)
                .await
                .expect("stored assignment")
                .expect("assignment")
                .course_id,
            course
        );

        store
            .create_enrollment(
                context,
                question_model::AssignmentEnrollment {
                    id: question_model::EnrollmentId::from_uuid(id(40)),
                    tenant,
                    assignment,
                    user: student,
                    student: StudentId::from_uuid(id(41)),
                    first_completed_at: None,
                    current_grade_run: None,
                    best_grade_run: None,
                },
            )
            .await
            .expect("gradebook fixture enrollment");

        let gradebook = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/courses/{course}/gradebook"))
                    .header("cookie", &instructor_cookie)
                    .body(Body::empty())
                    .expect("gradebook request"),
            )
            .await
            .expect("gradebook response");
        assert_eq!(gradebook.status(), StatusCode::OK);
        assert_eq!(
            gradebook
                .headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
        let gradebook = response_json(gradebook).await;
        let rows = gradebook["items"].as_array().expect("gradebook rows");
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        let row_fields: std::collections::BTreeSet<_> = row
            .as_object()
            .expect("gradebook row object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            row_fields,
            std::collections::BTreeSet::from([
                "tenant",
                "courseId",
                "enrollmentId",
                "studentId",
                "assignmentId",
                "assignmentTitle",
                "summary",
            ])
        );
        assert_eq!(row["summary"]["tenant"], row["tenant"]);

        let administrator_gradebook = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/courses/{course}/gradebook"))
                    .header("cookie", &administrator_cookie)
                    .body(Body::empty())
                    .expect("administrator gradebook request"),
            )
            .await
            .expect("administrator gradebook response");
        assert_eq!(administrator_gradebook.status(), StatusCode::OK);

        let student_gradebook = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/courses/{course}/gradebook"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student gradebook request"),
            )
            .await
            .expect("student gradebook response");
        assert_eq!(student_gradebook.status(), StatusCode::FORBIDDEN);

        let outsider_gradebook = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/courses/{course}/gradebook"))
                    .header("cookie", &outsider_cookie)
                    .body(Body::empty())
                    .expect("outsider gradebook request"),
            )
            .await
            .expect("outsider gradebook response");
        assert_eq!(outsider_gradebook.status(), StatusCode::NOT_FOUND);

        let second_assignment = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/courses/{course}/assignments"))
                    .header("cookie", &instructor_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&assignment_request)
                            .expect("second assignment request serialization"),
                    ))
                    .expect("second assignment request"),
            )
            .await
            .expect("second assignment response");
        assert_eq!(second_assignment.status(), StatusCode::CREATED);

        let second_course = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/courses")
                    .header("cookie", &instructor_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"BIOC 302: Enzymes"}"#))
                    .expect("second course request"),
            )
            .await
            .expect("second course response");
        assert_eq!(second_course.status(), StatusCode::CREATED);
        let second_course = response_json(second_course).await;
        let second_course: CourseId =
            serde_json::from_value(second_course["id"].clone()).expect("second course ID response");
        let mut second_course_record = store
            .get_course(context, second_course)
            .await
            .expect("second course lookup")
            .expect("second course exists");
        second_course_record.members.push(CourseMembership {
            user: student,
            role: CourseMembershipRole::Student,
        });
        store
            .upsert_course(context, second_course_record)
            .await
            .expect("second student membership save");

        let student_courses = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/courses?pageSize=1")
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student courses request"),
            )
            .await
            .expect("student courses response");
        let student_courses = response_json(student_courses).await;
        assert_eq!(student_courses["items"][0]["role"], "student");
        assert_eq!(student_courses["items"].as_array().map(Vec::len), Some(1));
        let course_cursor = student_courses["nextCursor"]
            .as_str()
            .expect("course continuation cursor");
        let continued_courses = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/courses?pageSize=1&cursor={course_cursor}"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("course continuation request"),
            )
            .await
            .expect("course continuation response");
        assert_eq!(continued_courses.status(), StatusCode::OK);
        let continued_courses = response_json(continued_courses).await;
        assert_eq!(continued_courses["items"].as_array().map(Vec::len), Some(1));
        assert_ne!(student_courses["items"][0], continued_courses["items"][0]);
        assert_eq!(continued_courses["nextCursor"], serde_json::Value::Null);

        let exact_course = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/courses/{course}"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("exact course request"),
            )
            .await
            .expect("exact course response");
        assert_eq!(exact_course.status(), StatusCode::OK);
        assert_eq!(response_json(exact_course).await["role"], "student");

        let student_assignments = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/courses/{course}/assignments?pageSize=1"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student assignments request"),
            )
            .await
            .expect("student assignments response");
        assert_eq!(student_assignments.status(), StatusCode::OK);
        let student_assignments = response_json(student_assignments).await;
        assert_eq!(
            student_assignments["items"].as_array().map(Vec::len),
            Some(1)
        );
        let assignment_cursor = student_assignments["nextCursor"]
            .as_str()
            .expect("assignment continuation cursor");
        let continued_assignments = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/courses/{course}/assignments?pageSize=1&cursor={assignment_cursor}"
                    ))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("assignment continuation request"),
            )
            .await
            .expect("assignment continuation response");
        assert_eq!(continued_assignments.status(), StatusCode::OK);
        let continued_assignments = response_json(continued_assignments).await;
        assert_eq!(
            continued_assignments["items"].as_array().map(Vec::len),
            Some(1)
        );
        assert_ne!(
            student_assignments["items"][0],
            continued_assignments["items"][0]
        );
        assert_eq!(continued_assignments["nextCursor"], serde_json::Value::Null);

        for path in [
            "/api/courses".to_string(),
            format!("/api/courses/{course}/assignments"),
        ] {
            for query in ["pageSize=0", "pageSize=101", "cursor=", "offset=1"] {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri(format!("{path}?{query}"))
                            .header("cookie", &student_cookie)
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

        let exact = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/assignments/{assignment}"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("exact assignment request"),
            )
            .await
            .expect("exact assignment response");
        assert_eq!(exact.status(), StatusCode::OK);

        let outsider_courses = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/courses")
                    .header("cookie", &outsider_cookie)
                    .body(Body::empty())
                    .expect("outsider courses request"),
            )
            .await
            .expect("outsider courses response");
        assert!(
            response_json(outsider_courses).await["items"]
                .as_array()
                .expect("course items")
                .is_empty()
        );

        let hidden_course = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/courses/{course}"))
                    .header("cookie", &outsider_cookie)
                    .body(Body::empty())
                    .expect("hidden course request"),
            )
            .await
            .expect("hidden course response");
        assert_eq!(hidden_course.status(), StatusCode::NOT_FOUND);

        let hidden = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/courses/{course}/assignments"))
                    .header("cookie", &outsider_cookie)
                    .body(Body::empty())
                    .expect("hidden assignments request"),
            )
            .await
            .expect("hidden assignments response");
        assert_eq!(hidden.status(), StatusCode::NOT_FOUND);

        let student_update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                    .header("cookie", &student_cookie)
                    .header(IF_MATCH, &assignment_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({
                        "title": "student overwrite", "problems": [reference], "policies": policies(),
                    }).to_string()))
                    .expect("student update request"),
            )
            .await
            .expect("student update response");
        assert_eq!(student_update.status(), StatusCode::FORBIDDEN);

        let student_missing_revision = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                    .header("cookie", &student_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "student missing revision", "problems": [reference], "policies": policies(),
                        })
                        .to_string(),
                    ))
                    .expect("student missing revision request"),
            )
            .await
            .expect("student missing revision response");
        assert_eq!(student_missing_revision.status(), StatusCode::FORBIDDEN);

        let student_write = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/courses/{course}/assignments"))
                    .header("cookie", student_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&assignment_request)
                            .expect("assignment request serialization"),
                    ))
                    .expect("student write request"),
            )
            .await
            .expect("student write response");
        assert_eq!(student_write.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn assignment_revision_requires_one_positive_strong_etag() {
        let accepted = HeaderMap::from_iter([(IF_MATCH, HeaderValue::from_static("\"7\""))]);
        assert_eq!(
            required_assignment_revision(&accepted).expect("strong revision"),
            serde_json::from_str("7").expect("revision")
        );
        for value in ["7", "W/\"7\"", "\"0\"", "\"-1\"", "\"9223372036854775808\""] {
            let headers = HeaderMap::from_iter([(
                IF_MATCH,
                HeaderValue::from_str(value).expect("test header"),
            )]);
            assert_eq!(
                required_assignment_revision(&headers),
                Err(AssignmentRevisionHeaderError::Malformed)
            );
        }
        assert_eq!(
            required_assignment_revision(&HeaderMap::new()),
            Err(AssignmentRevisionHeaderError::Missing)
        );
    }
}
