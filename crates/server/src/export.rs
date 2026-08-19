//! Instructor-authorized assignment export requests (MOD-EXPORT).
//!
//! An assignment is a tenant-owned course artifact that already holds exact
//! immutable published-version references.  The browser supplies only its
//! assignment ID: the store atomically freezes those references, reserves the
//! student-record delivery targets, and enqueues the durable export work.  A
//! response never contains an object key, signed URL, lease, source payload,
//! or answer-bearing material.

use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use learning_data_access::{
    CreateAssignmentExport, ExportId, ExportJobStore, SessionStore, Store, StoreError,
};
use question_model::{AssignmentId, CourseMembershipRole};

use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};

/// A deliberately small retry budget for the initial asynchronous export
/// producer.  It is server policy rather than browser input.
const EXPORT_MAX_ATTEMPTS: u16 = 3;
const MAX_EMPTY_REQUEST_BODY_BYTES: usize = 64;

/// Builds the authenticated assignment-export route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: Store + ExportJobStore + SessionStore + 'static,
{
    Router::new()
        .route(
            "/api/assignments/{assignment}/exports",
            post(create_export::<S>),
        )
        .route("/api/exports/{export}", get(get_export::<S>))
        .with_state(ExportRouteState { store })
}

struct ExportRouteState<S> {
    store: Arc<S>,
}

impl<S> Clone for ExportRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

async fn create_export<S>(
    State(state): State<ExportRouteState<S>>,
    Path(assignment): Path<AssignmentId>,
    request: Request,
) -> Response
where
    S: Store + ExportJobStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    // Reject hostile request bodies early, but do not rely on this projection:
    // `create_assignment_export` repeats session and course authority inside
    // its atomic persistence boundary before it queues anything.
    if let Err(response) =
        require_assignment_management(state.store.as_ref(), &authenticated, assignment).await
    {
        return response;
    }
    // Authorize before consuming the body. This endpoint deliberately has no
    // request schema: accepting arbitrary fields would create an illusion
    // that callers may choose source versions, formats, or delivery targets.
    // Keeping validation last prevents a hostile body from becoming an
    // authorization or cross-tenant existence oracle.
    let body = match to_bytes(request.into_body(), MAX_EMPTY_REQUEST_BODY_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return error_response(StatusCode::PAYLOAD_TOO_LARGE, "export request is invalid");
        }
    };
    if !body.is_empty() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "export request body must be empty",
        );
    }
    match state
        .store
        .create_assignment_export(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            CreateAssignmentExport {
                assignment,
                max_attempts: EXPORT_MAX_ATTEMPTS,
            },
        )
        .await
    {
        // `StudentExportView` is the storage contract's redacted browser
        // projection.  It exposes ready delivery IDs and file metadata only.
        Ok(view) => no_store((StatusCode::ACCEPTED, Json(view)).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn get_export<S>(
    State(state): State<ExportRouteState<S>>,
    headers: HeaderMap,
    Path(export): Path<ExportId>,
) -> Response
where
    S: Store + ExportJobStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let Some(view) = (match state
        .store
        .get_assignment_export_for_requester(
            authenticated.tenant_context,
            export,
            authenticated.record.subject.user(),
        )
        .await
    {
        Ok(view) => view,
        Err(error) => return store_error_response(error),
    }) else {
        return error_response(StatusCode::NOT_FOUND, "export not found");
    };
    no_store(Json(view).into_response())
}

/// The export record is tenant-scoped before this method runs.  Assignment and
/// course checks keep the HTTP boundary quiet before consuming its body. The
/// authoritative duplicate is in `ExportJobStore`, where session, membership,
/// assignment, frozen payload, and job insertion are bound atomically.
async fn require_assignment_management<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    assignment: AssignmentId,
) -> Result<(), Response>
where
    S: Store,
{
    let assignment = match store
        .get_assignment(authenticated.tenant_context, assignment)
        .await
    {
        Ok(Some(assignment)) => assignment,
        Ok(None) => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "assignment not found",
            ));
        }
        Err(error) => return Err(store_error_response(error)),
    };
    let manages = matches!(
        store
            .get_current_course_membership(
                authenticated.tenant_context,
                assignment.course_id,
                authenticated.record.subject.user(),
            )
            .await
            .map_err(store_error_response)?
            .map(|membership| membership.role),
        Some(CourseMembershipRole::Instructor)
    );
    manages
        .then_some(())
        .ok_or_else(|| error_response(StatusCode::FORBIDDEN, "assignment export is not authorized"))
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::NOT_FOUND, "export not found")
        }
        StoreError::AlreadyExists | StoreError::Conflict => {
            error_response(StatusCode::CONFLICT, "export request changed; retry it")
        }
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::RunModel(error) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
        }
        StoreError::TimedOut | StoreError::RetryableTransaction | StoreError::Unavailable(_) => {
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "export service unavailable",
            )
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
    use learning_data_access::in_memory::MemoryStore;
    use learning_data_access::{
        AssignmentRecord, CatalogStore, CourseRecord, CourseRosterStore, CreateCourseCommand,
        DraftRecord, JobLeaseDuration, JobPayload, JobStore, PublishDraftCommand,
        RetentionWorkerCommand, RetentionWorkerStore, SessionLifetime, SessionSubject,
        TenantContext, UpsertCourseMember,
    };
    use question_model::answer::NumericTolerance;
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::response::ResponseDefinition;
    use question_model::run_policy::{
        AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, TimingPolicy,
        VariationPolicy,
    };
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, BackendCapabilities, Capability, CourseId, DraftQuestionDefinition,
        DraftQuestionSource, GradingDefinition, ObjectId, ProblemId, ProblemVersionRef,
        PublicationScope, QuestionMetadata, QuestionSource, RunPolicies, TenantId, UserId,
        UserRole, VersionId, WorkspaceId,
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    async fn cookie(store: &MemoryStore, tenant: TenantId, user: UserId, role: UserRole) -> String {
        let issued = crate::auth::issue_session(
            store,
            SessionSubject::new(tenant, user, "Export fixture", vec![role])
                .expect("fixture identity"),
            crate::auth::SessionConfig::new(
                SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
                crate::auth::CookieTransport::LocalHttp,
            ),
        )
        .await
        .expect("session");
        issued
            .set_cookie
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string()
    }

    async fn json(response: Response) -> serde_json::Value {
        serde_json::from_slice(
            &to_bytes(response.into_body(), 128 * 1_024)
                .await
                .expect("response body"),
        )
        .expect("JSON response")
    }

    fn policies() -> RunPolicies {
        RunPolicies {
            completion: CompletionRequirement::AllCorrect,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        }
    }

    async fn publish_fixture(
        store: &MemoryStore,
        context: TenantContext,
        tenant: TenantId,
        publisher: UserId,
    ) -> ProblemVersionRef {
        let reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(id(20)),
            version: VersionId::from_uuid(id(21)),
        };
        let draft = DraftRecord {
            tenant,
            question: DraftQuestionDefinition {
                workspace: WorkspaceId::from_uuid(id(22)),
                source: DraftQuestionSource::Native {
                    family: "export-fixture".to_string(),
                },
                prompt: vec![ContentBlock::Text {
                    markdown: "Identify the peptide bond.".to_string(),
                }],
                response: ResponseDefinition::Numeric {
                    tolerance: NumericTolerance::Absolute { epsilon: 0.0 },
                    unit: None,
                },
                attempt_policy: AttemptPolicy { max_attempts: None },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Static,
                grading: GradingDefinition::AllOrNothing { points: 1.0 },
                metadata: QuestionMetadata {
                    title: "Export fixture".to_string(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::CcBySa,
                    language: "en-US".to_string(),
                },
            },
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
                    publication: reference,
                    published_source: QuestionSource::Native {
                        family: "export-fixture".to_string(),
                    },
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    byline: question_model::PublicByline::new(vec![
                        question_model::PublicAuthorName::new("PLE fixture".to_string())
                            .expect("valid test byline"),
                    ])
                    .expect("valid test byline"),
                    capabilities: BackendCapabilities::from_iter([
                        Capability::ServerGrading,
                        Capability::PrintExport,
                    ]),
                },
            )
            .await
            .expect("fixture publication");
        reference
    }

    #[tokio::test]
    async fn export_route_freezes_an_authorized_assignment_and_hides_private_worker_state() {
        let store = Arc::new(MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
            .expect("fixture clock");
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let requester = UserId::from_uuid(id(2));
        let other_instructor = UserId::from_uuid(id(3));
        let student = UserId::from_uuid(id(4));
        let foreign_tenant = TenantId::from_uuid(id(40));
        let foreign_instructor = UserId::from_uuid(id(41));
        let course = CourseId::from_uuid(id(5));
        let assignment = AssignmentId::from_uuid(id(6));
        let requester_cookie = cookie(&store, tenant, requester, UserRole::Instructor).await;
        let other_cookie = cookie(&store, tenant, other_instructor, UserRole::Instructor).await;
        let student_cookie = cookie(&store, tenant, student, UserRole::Student).await;
        let foreign_cookie = cookie(
            &store,
            foreign_tenant,
            foreign_instructor,
            UserRole::Instructor,
        )
        .await;
        store
            .create_course(
                context,
                CreateCourseCommand {
                    course: CourseRecord {
                        id: course,
                        tenant,
                        title: "BIOC 301".to_string(),
                        term: question_model::CourseTerm::from_parts(
                            "2026-08-24",
                            "2026-12-18",
                            "America/Chicago",
                        )
                        .expect("explicit fixture course term"),
                    },
                    initial_instructor: requester,
                },
            )
            .await
            .expect("course save");
        store
            .upsert_course_member(
                context,
                UpsertCourseMember {
                    course,
                    user: student,
                    display_name: "Export learner".to_string(),
                    roster_contact: None,
                },
            )
            .await
            .expect("student roster membership");
        let reference = publish_fixture(&store, context, tenant, requester).await;
        store
            .create_untimed_assignment(
                context,
                AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: question_model::AssignmentAudience::CourseWide,
                    title: "Peptide bond exam".to_string(),
                    items: vec![question_model::AssignmentItem {
                        id: question_model::AssignmentItemId::from_uuid(id(105)),
                        reference,
                        position: 0,
                        points_possible: question_model::PointValue::from_whole(1),
                        delivery_state: question_model::AssignmentDeliveryState::Active,
                        scoring_mode: question_model::AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
            )
            .await
            .expect("assignment save");
        let app = router(Arc::clone(&store));

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/assignments/{assignment}/exports"))
                    .header("cookie", &requester_cookie)
                    .body(Body::empty())
                    .expect("export request"),
            )
            .await
            .expect("export response");
        assert_eq!(created.status(), StatusCode::ACCEPTED);
        let created = json(created).await;
        assert_eq!(created["assignment"], serde_json::json!(assignment));
        assert_eq!(created["state"], "queued");
        assert!(created["artifacts"].is_null());
        let encoded = created.to_string();
        for forbidden in ["object", "key", "url", "lease", "source", "answer"] {
            assert!(
                !encoded.to_ascii_lowercase().contains(forbidden),
                "export status must not disclose {forbidden}"
            );
        }
        let export: ExportId = serde_json::from_value(created["id"].clone()).expect("export ID");

        let owner_read = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/exports/{export}"))
                    .header("cookie", &requester_cookie)
                    .body(Body::empty())
                    .expect("owner status request"),
            )
            .await
            .expect("owner status response");
        assert_eq!(owner_read.status(), StatusCode::OK);

        let nonempty_body = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/assignments/{assignment}/exports"))
                    .header("cookie", &requester_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"format":"pdf"}"#))
                    .expect("nonempty export request"),
            )
            .await
            .expect("nonempty export response");
        assert_eq!(nonempty_body.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let unauthenticated_body = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/assignments/{assignment}/exports"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"format":"pdf"}"#))
                    .expect("unauthenticated export request"),
            )
            .await
            .expect("unauthenticated export response");
        assert_eq!(
            unauthenticated_body.status(),
            StatusCode::UNAUTHORIZED,
            "authentication must precede rejected request-body handling"
        );

        let other_read = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/exports/{export}"))
                    .header("cookie", &other_cookie)
                    .body(Body::empty())
                    .expect("other-instructor status request"),
            )
            .await
            .expect("other-instructor status response");
        assert_eq!(
            other_read.status(),
            StatusCode::NOT_FOUND,
            "a delivery ACL is requester-specific, so another instructor receives no status oracle"
        );

        let student_create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/assignments/{assignment}/exports"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student export request"),
            )
            .await
            .expect("student export response");
        assert_eq!(student_create.status(), StatusCode::FORBIDDEN);

        let student_hostile_body = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/assignments/{assignment}/exports"))
                    .header("cookie", &student_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"format":"pdf"}"#))
                    .expect("student hostile export request"),
            )
            .await
            .expect("student hostile export response");
        assert_eq!(
            student_hostile_body.status(),
            StatusCode::FORBIDDEN,
            "authorization must precede request-body handling for a student"
        );

        let foreign_hostile_body = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/assignments/{assignment}/exports"))
                    .header("cookie", &foreign_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"format":"pdf"}"#))
                    .expect("foreign hostile export request"),
            )
            .await
            .expect("foreign hostile export response");
        assert_eq!(
            foreign_hostile_body.status(),
            StatusCode::NOT_FOUND,
            "tenant isolation must precede request-body handling"
        );

        for request in [
            Request::builder()
                .method("POST")
                .uri(format!("/api/assignments/{assignment}/exports"))
                .header("cookie", &foreign_cookie)
                .body(Body::empty())
                .expect("foreign export request"),
            Request::builder()
                .uri(format!("/api/exports/{export}"))
                .header("cookie", &foreign_cookie)
                .body(Body::empty())
                .expect("foreign export status request"),
        ] {
            assert_eq!(
                app.clone()
                    .oneshot(request)
                    .await
                    .expect("foreign response")
                    .status(),
                StatusCode::NOT_FOUND,
                "foreign tenant must not enumerate assignment exports"
            );
        }

        store
            .seed_retention_cleanup_for_test(
                tenant,
                course,
                (0..4)
                    .map(|offset| ObjectId::from_uuid(id(90 + offset)))
                    .collect(),
            )
            .expect("archive cleanup fixture");
        let command = loop {
            let claim = store
                .claim_next_job(
                    &learning_data_access::JobClaimFilter::all(),
                    JobLeaseDuration::from_seconds(30).expect("lease duration"),
                )
                .await
                .expect("archive claim")
                .expect("queued job");
            match claim.payload {
                JobPayload::Retention {
                    course: claimed_course,
                    stage,
                    generation,
                } => {
                    assert_eq!(claimed_course, course);
                    break RetentionWorkerCommand {
                        tenant,
                        course,
                        stage,
                        generation,
                        job: claim.id,
                        lease: claim.lease_token,
                    };
                }
                _ => store
                    .complete_job(claim.id, claim.lease_token)
                    .await
                    .expect("finish unrelated fixture job"),
            }
        };
        store
            .prepare_retention_work(command)
            .await
            .expect("archive prepare fence");

        for request in [
            Request::builder()
                .method("POST")
                .uri(format!("/api/assignments/{assignment}/exports"))
                .header("cookie", &requester_cookie)
                .body(Body::empty())
                .expect("archived export create request"),
            Request::builder()
                .uri(format!("/api/exports/{export}"))
                .header("cookie", &requester_cookie)
                .body(Body::empty())
                .expect("archived export read request"),
        ] {
            let response = app
                .clone()
                .oneshot(request)
                .await
                .expect("archived export response");
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(response.headers()["cache-control"], "no-store");
        }
    }
}
