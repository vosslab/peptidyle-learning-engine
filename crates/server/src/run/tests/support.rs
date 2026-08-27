//! Shared HTTP and authenticated-session helpers for run-route behavior tests.

use super::*;
use std::sync::Arc;
use std::time::Duration;

use crate::accepted_submission_worker::AcceptedSubmissionExecutionWorker;
use crate::worker::WorkerSettings;
use learning_data_access::WorkerId;

pub(super) async fn issued_cookie(store: &MemoryStore, user: UserId, name: &str) -> String {
    issued_cookie_for(store, TenantId::from_uuid(id(1)), user, name).await
}

pub(super) async fn issued_cookie_for(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    name: &str,
) -> String {
    let subject =
        SessionSubject::new(tenant, user, name, vec![UserRole::Student]).expect("session subject");
    let issued = crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
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

pub(super) async fn json(response: Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), 256 * 1_024)
        .await
        .expect("response bytes");
    serde_json::from_slice(&bytes).expect("JSON response")
}

/// Drains one accepted submission through the same sealed worker used by the
/// production composition and requires its durable commit to be acknowledged.
///
/// The worker receives a clone of the in-memory store because `MemoryStore`
/// clones share their state while retaining the worker's capability boundary.
/// A fixed worker identity and bounded settings keep this lifecycle helper
/// deterministic and free of polling or sleeps.
pub(super) async fn drain_one_accepted_submission<B>(store: &Arc<MemoryStore>, backend: Arc<B>)
where
    B: RunBackend + 'static,
{
    let settings = WorkerSettings::new(60, Duration::from_secs(5), 1)
        .expect("bounded accepted-submission worker settings");
    let worker = AcceptedSubmissionExecutionWorker::new(
        (**store).clone(),
        backend,
        WorkerId::from_uuid(id(70_001)),
        settings,
    )
    .expect("accepted-submission worker");
    let report = worker.drain_one().await.expect("accepted-submission drain");
    assert_eq!(
        report.committed, 1,
        "worker must commit one accepted execution"
    );
    assert_eq!(report.no_claim, 0);
    assert_eq!(report.rescheduled, 0);
    assert_eq!(report.terminal, 0);
    assert_eq!(report.stale_claim, 0);
    assert_eq!(report.outcome_unknown, 0);
}

/// Exercises the canonical asynchronous learner lifecycle through HTTP:
/// accepted pending response, one server-owned worker execution, and the
/// answer-free completed status projection.
pub(super) struct AcceptedSubmissionRoute<'a> {
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) attempt: QuestionAttemptId,
    pub(super) cookie: &'a str,
    pub(super) idempotency_key: &'a str,
    pub(super) response: serde_json::Value,
}

pub(super) async fn submit_and_complete_accepted_submission<B>(
    app: &Router,
    store: &Arc<MemoryStore>,
    backend: Arc<B>,
    route: AcceptedSubmissionRoute<'_>,
) -> serde_json::Value
where
    B: RunBackend + 'static,
{
    let accepted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(submission_path(
                    route.course,
                    route.assignment,
                    route.attempt,
                ))
                .header("cookie", route.cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", route.idempotency_key)
                .body(Body::from(
                    serde_json::json!({ "response": route.response }).to_string(),
                ))
                .expect("submission request"),
        )
        .await
        .expect("accepted submission response");
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    assert_eq!(json(accepted).await["kind"], "accepted_pending");

    drain_one_accepted_submission(store, backend).await;

    let completed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(submission_status_path(
                    route.course,
                    route.assignment,
                    route.attempt,
                ))
                .header("cookie", route.cookie)
                .body(Body::empty())
                .expect("submission status request"),
        )
        .await
        .expect("submission status response");
    assert_eq!(completed.status(), StatusCode::OK);
    json(completed).await
}

#[tokio::test]
async fn accepted_pending_replay_returns_answer_free_202_without_backend_call() {
    let (store, backend, app, student_cookie, _, assignment, _) = fixture().await;
    let course = CourseId::from_uuid(id(5));
    let attempt = active_attempt_for(&app, course, assignment, &student_cookie).await;
    let response = StudentResponse::Numeric { value: 18.0 };
    let key = learning_data_access::SubmissionIdempotencyKey::parse("pending-replay")
        .expect("bounded idempotency key");
    learning_data_access::AutomatedGradingStore::accept_automated_submission(
        store.as_ref(),
        TenantContext::from_authenticated_session(TenantId::from_uuid(id(1))),
        learning_data_access::AcceptedSubmissionCommand {
            actor: UserId::from_uuid(id(3)),
            course,
            assignment,
            attempt: attempt.id,
            idempotency_key: key,
            response,
            execution_job: learning_data_access::JobId::from_uuid(id(700)),
        },
    )
    .await
    .expect("accepts pending submission");

    let replay = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(submission_path(course, assignment, attempt.id))
                .header("cookie", student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "pending-replay")
                .body(Body::from(
                    serde_json::json!({ "response": { "kind": "numeric", "value": 18.0 } })
                        .to_string(),
                ))
                .expect("replay request"),
        )
        .await
        .expect("pending replay response");

    assert_eq!(replay.status(), StatusCode::ACCEPTED);
    assert_eq!(replay.headers()["cache-control"], "no-store");
    assert_eq!(
        json(replay).await,
        serde_json::json!({
            "kind": "accepted_pending",
            "accepted": true,
            "attemptId": attempt.id,
            "automatedGradingStatus": "pending",
            "nextAction": "check_status",
        })
    );
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);
}

/// Starts an assignment run through its complete, server-owned route binding.
/// The start command has no browser body contract.
pub(super) fn start_run_request(
    course: CourseId,
    assignment: AssignmentId,
    cookie: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/api/courses/{course}/assignments/{assignment}/runs"
        ))
        .header("cookie", cookie)
        .body(Body::empty())
        .expect("start run request")
}

pub(super) fn prefetch_path(
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
) -> String {
    format!("/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/prefetch-next")
}

pub(super) fn submission_path(
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
) -> String {
    format!("/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/submissions")
}

pub(super) fn submission_status_path(
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
) -> String {
    format!("/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/submission-status")
}

/// Builds the explicit learner-work route binding for an issued presentation.
pub(super) fn question_path(
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
) -> String {
    format!("/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/question")
}

pub(super) fn external_tool_launch_path(
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
) -> String {
    format!(
        "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch"
    )
}

pub(super) async fn active_attempt_for(
    app: &Router,
    course: CourseId,
    assignment: AssignmentId,
    cookie: &str,
) -> QuestionAttempt {
    let run_response = app
        .clone()
        .oneshot(start_run_request(course, assignment, cookie))
        .await
        .expect("start run response");
    assert_eq!(run_response.status(), StatusCode::CREATED);
    let run: AssignmentRun =
        serde_json::from_value(json(run_response).await).expect("run contract");
    let attempts_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/attempts", run.id))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("attempt request"),
        )
        .await
        .expect("attempt response");
    assert_eq!(attempts_response.status(), StatusCode::OK);
    let attempts: Page<QuestionAttempt> =
        serde_json::from_value(json(attempts_response).await).expect("attempt page");
    attempts.items.into_iter().next().expect("active attempt")
}

/// Returns an ID from the answer-free schema actually issued to the browser.
/// Native durable IDs are deliberately not accepted by the submit route.
pub(super) async fn presented_choice_id(
    app: &Router,
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
    cookie: &str,
    position: usize,
) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(question_path(course, assignment, attempt))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("issued question request"),
        )
        .await
        .expect("issued question response");
    assert_eq!(response.status(), StatusCode::OK);
    json(response).await["response"]["choices"]
        .as_array()
        .and_then(|choices| choices.get(position))
        .and_then(|choice| choice["id"].as_str())
        .expect("issued choice identifier")
        .to_string()
}
