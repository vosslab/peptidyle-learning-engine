//! Deterministic trusted backend for registered Gradebook route tests.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{FlatGradingCapability, QtiGradingCapability, WebworkGradingCapability};
use learning_data_access::{NavigationReferenceStore, TenantContext};
use question_model::{
    AssignmentId, AttemptProvenance, AttemptResult, CourseId, ImplementationVersion,
    QuestionAttempt, QuestionAttemptId, QuestionDefinition, QuestionEnvelope, RunId, RunReference,
    StudentResponse, TenantId, UserId, generation::Seed,
};
use std::sync::Arc;
use tower::ServiceExt;

pub(super) fn course_app_with_runs(
    store: &Arc<MemoryStore>,
    backend: &Arc<AlwaysCorrectBackend>,
) -> axum::Router {
    crate::course::router(Arc::clone(store)).merge(
        crate::run::router_with_accepted_submission_fast_path(
            Arc::clone(store),
            Arc::clone(backend),
            Arc::new(
                learning_data_access::in_memory::MemorySealedPrivateExecutionStore::new(
                    Arc::clone(store),
                ),
            ),
            store.clone(),
            store.clone(),
            Arc::new(DeferredAcceptedSubmission),
        ),
    )
}

pub(super) fn same_origin(request: Request<Body>) -> Request<Body> {
    let (mut parts, body) = request.into_parts();
    parts
        .headers
        .insert("sec-fetch-site", "same-origin".parse().expect("header"));
    parts
        .headers
        .insert("sec-fetch-mode", "cors".parse().expect("header"));
    parts
        .headers
        .insert("sec-fetch-dest", "empty".parse().expect("header"));
    Request::from_parts(parts, body)
}

pub(super) struct GradebookRouteHarness<'a> {
    app: &'a axum::Router,
    store: &'a MemoryStore,
    backend: &'a Arc<AlwaysCorrectBackend>,
}

impl<'a> GradebookRouteHarness<'a> {
    pub(super) fn new(
        app: &'a axum::Router,
        store: &'a MemoryStore,
        backend: &'a Arc<AlwaysCorrectBackend>,
    ) -> Self {
        Self {
            app,
            store,
            backend,
        }
    }

    pub(super) async fn completed_run_reference(
        &self,
        completion: CompletedRunIdentity<'_>,
    ) -> RunReference {
        let CompletedRunIdentity {
            tenant,
            student,
            course,
            assignment,
            cookie,
        } = completion;
        let started = self
            .app
            .clone()
            .oneshot(
                Request::post(format!(
                    "/api/courses/{course}/assignments/{assignment}/runs"
                ))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("run start request"),
            )
            .await
            .expect("run start response");
        assert_eq!(started.status(), StatusCode::CREATED);
        let started: serde_json::Value = serde_json::from_slice(
            &to_bytes(started.into_body(), 64 * 1024)
                .await
                .expect("run start body"),
        )
        .expect("run start JSON");
        let run = RunId::from_uuid(
            uuid::Uuid::parse_str(started["id"].as_str().expect("run ID")).expect("UUID run ID"),
        );
        let attempts = self
            .app
            .clone()
            .oneshot(
                Request::get(format!("/api/runs/{run}/attempts"))
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("attempt list request"),
            )
            .await
            .expect("attempt list response");
        assert_eq!(attempts.status(), StatusCode::OK);
        let attempts: serde_json::Value = serde_json::from_slice(
            &to_bytes(attempts.into_body(), 64 * 1024)
                .await
                .expect("attempt list body"),
        )
        .expect("attempt list JSON");
        let attempt = QuestionAttemptId::from_uuid(
            uuid::Uuid::parse_str(attempts["items"][0]["id"].as_str().expect("attempt ID"))
                .expect("UUID attempt ID"),
        );
        let submitted = self
            .app
            .clone()
            .oneshot(
                Request::post(format!(
                    "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/submissions"
                ))
                .header("cookie", cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "gradebook-route-completion")
                .body(Body::from(
                    r#"{"response":{"kind":"numeric","value":18.0}}"#,
                ))
                .expect("submission request"),
            )
            .await
            .expect("submission response");
        assert_eq!(submitted.status(), StatusCode::ACCEPTED);
        crate::test_fixtures::drain_one_accepted_submission(
            &Arc::new(self.store.clone()),
            Arc::clone(self.backend),
        )
        .await;
        self.store
            .run_reference(
                TenantContext::from_authenticated_session(tenant),
                student,
                run,
            )
            .await
            .expect("run reference lookup")
            .expect("Student run reference")
    }
}

pub(super) struct CompletedRunIdentity<'a> {
    pub(super) tenant: TenantId,
    pub(super) student: UserId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) cookie: &'a str,
}

#[derive(Default)]
pub(super) struct AlwaysCorrectBackend;

pub(super) struct DeferredAcceptedSubmission;

#[async_trait::async_trait]
impl crate::accepted_submission_worker::AcceptedSubmissionFastPath for DeferredAcceptedSubmission {
    async fn execute_accepted_submission(
        &self,
        _: learning_data_access::AcceptedSubmissionExecutionTarget,
    ) -> Result<
        crate::accepted_submission_worker::AcceptedSubmissionHandlerResult,
        learning_data_access::StoreError,
    > {
        Ok(crate::accepted_submission_worker::AcceptedSubmissionHandlerResult::ClaimNoLongerActive)
    }
}

#[async_trait::async_trait]
impl crate::run::RunBackend for AlwaysCorrectBackend {
    async fn issue(
        &self,
        _: TenantContext,
        _: question_model::ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<crate::run::IssuedAttemptMetadata, crate::run::RunBackendError> {
        Ok(issued_attempt_metadata(question, seed))
    }

    async fn reproduce(
        &self,
        _: TenantContext,
        _: question_model::ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, crate::run::RunBackendError> {
        Ok(question_envelope(question, attempt.seed))
    }

    async fn grade(
        &self,
        _: TenantContext,
        _: question_model::ProblemVersionRef,
        _: &QuestionDefinition,
        _: &QuestionAttempt,
        _: &StudentResponse,
    ) -> Result<grading::GradeOutcome, crate::run::RunBackendError> {
        Ok(grading::GradeOutcome::Graded(AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 1.0,
        }))
    }
}

fn issued_attempt_metadata(
    question: &QuestionDefinition,
    seed: u64,
) -> crate::run::IssuedAttemptMetadata {
    let implementation = ImplementationVersion {
        id: "gradebook-route-test".to_string(),
        version: "1".to_string(),
    };
    crate::run::IssuedAttemptMetadata {
        envelope: question_envelope(question, seed),
        parameter_hash: format!("gradebook-route-{seed}"),
        provenance: AttemptProvenance {
            adapter: implementation.clone(),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation,
            rendered_question_sha256: format!("gradebook-route-render-{seed}"),
        },
        webwork_replay: None,
        flat_grading: None,
        flat_grading_capability: FlatGradingCapability::NotApplicable,
        webwork_grading: None,
        webwork_grading_capability: WebworkGradingCapability::NotApplicable,
        qti_grading: None,
        qti_grading_capability: QtiGradingCapability::NotApplicable,
    }
}

fn question_envelope(question: &QuestionDefinition, seed: u64) -> QuestionEnvelope {
    QuestionEnvelope {
        version: question.version,
        seed: Seed::new(seed),
        title: question.metadata.title.clone(),
        prompt: question.prompt.clone(),
        response: question.response.clone(),
    }
}
