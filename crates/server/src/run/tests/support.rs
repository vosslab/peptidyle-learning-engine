//! Shared HTTP and authenticated-session helpers for run-route behavior tests.

use super::*;

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

pub(super) async fn next_active_attempt(app: &Router, run: RunId, cookie: &str) -> QuestionAttempt {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{run}/attempts"))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("attempt list request"),
        )
        .await
        .expect("attempt list response");
    let attempts: Page<QuestionAttempt> =
        serde_json::from_value(json(response).await).expect("attempt page");
    attempts
        .items
        .into_iter()
        .find(|attempt| attempt.response.is_none())
        .expect("next active attempt")
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
