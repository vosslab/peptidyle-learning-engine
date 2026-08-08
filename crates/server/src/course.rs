//! Authenticated course and assignment routes (MOD-API-COURSE).
//!
//! Sessions establish the tenant and authenticated user. Course membership is
//! a separate tenant record, so a coarse instructor role does not grant access
//! to unrelated courses. Assignment requests carry exact immutable
//! `(ProblemId, VersionId)` references and never copy question payloads.

use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use question_model::{
    AssignmentId, AssignmentSummary, CourseId, CourseMembership, CourseMembershipRole, CourseRole,
    ProblemVersionRef, RunPolicies, UserRole,
};
use serde::{Deserialize, Serialize};
use store::{
    AssignmentRecord, CourseListScope, CourseRecord, Cursor, Page, PageRequest, PageSize,
    PaginationError, SessionStore, Store, StoreError,
};

use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};

const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_COURSE_BODY_BYTES: usize = 64 * 1_024;

/// Builds the authenticated course and assignment route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: Store + SessionStore + 'static,
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
        .route("/api/courses/{course}", get(get_course::<S>))
        .route("/api/assignments/{assignment}", get(get_assignment::<S>))
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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
struct CreateAssignmentRequest {
    title: String,
    problems: Vec<ProblemVersionRef>,
    policies: RunPolicies,
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
    Json(request): Json<CreateAssignmentRequest>,
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
    let assignment = AssignmentRecord {
        id: AssignmentId::generate(),
        tenant: authenticated.tenant_context.tenant_id(),
        course_id: course,
        title: request.title,
        problems: request.problems,
        policies: request.policies,
    };
    match state
        .store
        .upsert_assignment(authenticated.tenant_context, assignment.clone())
        .await
    {
        Ok(()) => no_store((StatusCode::CREATED, Json(assignment.summary())).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn get_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(assignment): Path<AssignmentId>,
) -> Response
where
    S: Store + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let assignment = match state
        .store
        .get_assignment(authenticated.tenant_context, assignment)
        .await
    {
        Ok(Some(assignment)) => assignment,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    if let Err(response) = require_course_access(
        state.store.as_ref(),
        &authenticated,
        assignment.course_id,
        false,
    )
    .await
    {
        return response;
    }
    no_store(Json(assignment.summary()).into_response())
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
        ActivityTimestamp, BackendCapabilities, Capability, GradingDefinition, ProblemId,
        PublicationScope, QuestionDefinition, QuestionMetadata, QuestionSource, TenantId, UserId,
        VersionId, WorkspaceId,
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
        let subject =
            SessionSubject::new(TenantId::from_uuid(id(1)), user, "Course Fixture", roles)
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
            question: QuestionDefinition {
                version,
                problem: None,
                workspace,
                source: QuestionSource::Native {
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
        store
            .upsert_draft(context, draft.clone())
            .await
            .expect("draft save");
        store
            .publish_draft(
                context,
                PublishDraftCommand {
                    expected_draft: draft,
                    problem,
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
        let instructor_cookie = issued_cookie(&store, vec![UserRole::Instructor], instructor).await;
        let student_cookie = issued_cookie(&store, vec![UserRole::Student], student).await;
        let outsider_cookie = issued_cookie(&store, vec![UserRole::Instructor], outsider).await;
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
        let created_assignment = response_json(created_assignment).await;
        let assignment: AssignmentId = serde_json::from_value(created_assignment["id"].clone())
            .expect("assignment ID response");
        assert_eq!(created_assignment["courseId"], serde_json::json!(course));
        assert_eq!(
            created_assignment["problems"],
            serde_json::json!([reference]),
            "the course artifact must retain exact IDs rather than copy a question"
        );

        let student_courses = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/courses")
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student courses request"),
            )
            .await
            .expect("student courses response");
        let student_courses = response_json(student_courses).await;
        assert_eq!(student_courses["items"][0]["role"], "student");

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
                    .uri(format!("/api/courses/{course}/assignments"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student assignments request"),
            )
            .await
            .expect("student assignments response");
        assert_eq!(student_assignments.status(), StatusCode::OK);

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
}
