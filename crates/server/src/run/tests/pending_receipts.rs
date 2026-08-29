//! Accepted-pending route tests that keep learner-visible work answer-free.

use super::*;

struct PendingSubmission<'a> {
    store: &'a MemoryStore,
    context: TenantContext,
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
    actor: UserId,
    response: StudentResponse,
    key: &'a str,
    job: u128,
}

async fn accept_pending_submission(submission: PendingSubmission<'_>) {
    learning_data_access::AutomatedGradingStore::accept_automated_submission(
        submission.store,
        submission.context,
        learning_data_access::AcceptedSubmissionCommand {
            actor: submission.actor,
            course: submission.course,
            assignment: submission.assignment,
            attempt: submission.attempt,
            idempotency_key: learning_data_access::SubmissionIdempotencyKey::parse(submission.key)
                .expect("bounded idempotency key"),
            response: submission.response,
            execution_job: learning_data_access::JobId::from_uuid(id(submission.job)),
        },
    )
    .await
    .expect("accepts pending submission");
}

fn assert_answer_free_pending_attempt(attempt: &serde_json::Value) {
    assert_eq!(attempt["status"], "submitted");
    assert!(attempt["response"].is_null() && attempt["result"].is_null());
}

#[tokio::test]
async fn student_pending_receipt_reads_remain_submitted_and_answer_free() {
    let (store, _backend, app, student_cookie, _, assignment, _) = fixture().await;
    let course = CourseId::from_uuid(id(5));
    let attempt = active_attempt_for(&app, course, assignment, &student_cookie).await;
    accept_pending_submission(PendingSubmission {
        store: store.as_ref(),
        context: TenantContext::from_authenticated_session(TenantId::from_uuid(id(1))),
        course,
        assignment,
        attempt: attempt.id,
        actor: UserId::from_uuid(id(3)),
        response: StudentResponse::Numeric { value: 18.0 },
        key: "pending-learner-read",
        job: 701,
    })
    .await;

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/attempts", attempt.run))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("attempt list request"),
        )
        .await
        .expect("attempt list response");
    let detail = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/attempts/{}", attempt.id))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("attempt detail request"),
        )
        .await
        .expect("attempt detail response");

    assert_answer_free_pending_attempt(&json(list).await["items"][0]);
    assert_answer_free_pending_attempt(&json(detail).await);
}

#[tokio::test]
async fn submission_status_is_route_bound_answer_free_and_requires_the_student() {
    let (store, _backend, app, student_cookie, outsider_cookie, assignment, _) = fixture().await;
    let course = CourseId::from_uuid(id(5));
    let attempt = active_attempt_for(&app, course, assignment, &student_cookie).await;
    accept_pending_submission(PendingSubmission {
        store: store.as_ref(),
        context: TenantContext::from_authenticated_session(TenantId::from_uuid(id(1))),
        course,
        assignment,
        attempt: attempt.id,
        actor: UserId::from_uuid(id(3)),
        response: StudentResponse::Numeric { value: 18.0 },
        key: "pending-status-read",
        job: 702,
    })
    .await;

    let status = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(submission_status_path(course, assignment, attempt.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("status request"),
        )
        .await
        .expect("status response");
    assert_eq!(status.status(), StatusCode::ACCEPTED);
    assert_eq!(status.headers()["cache-control"], "no-store");
    assert_eq!(
        json(status).await,
        serde_json::json!({
            "kind": "accepted_pending",
            "accepted": true,
            "attemptId": attempt.id,
            "automatedGradingStatus": "pending",
            "nextAction": "check_status",
        })
    );

    let outsider = app
        .oneshot(
            Request::builder()
                .uri(submission_status_path(course, assignment, attempt.id))
                .header("cookie", outsider_cookie)
                .body(Body::empty())
                .expect("outsider status request"),
        )
        .await
        .expect("outsider status response");
    assert_eq!(outsider.status(), StatusCode::NOT_FOUND);
    assert_eq!(outsider.headers()["cache-control"], "no-store");
}

#[tokio::test]
async fn external_tool_pending_replay_is_answer_free_without_provider_retrieval() {
    use adapter_imathas::test_support::RecordedContractedTransportMode;

    let fixture = contracted_route_fixture(RecordedContractedTransportMode::Verified).await;
    accept_pending_submission(PendingSubmission {
        store: fixture.store.as_ref(),
        context: fixture.context,
        course: fixture.course,
        assignment: fixture.assignment,
        attempt: fixture.attempt.id,
        actor: UserId::from_uuid(id(803)),
        response: StudentResponse::ExternalTool {},
        key: "pending-external-replay",
        job: 871,
    })
    .await;

    let replay = fixture
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "{}/submission",
                    external_tool_launch_path(
                        fixture.course,
                        fixture.assignment,
                        fixture.attempt.id,
                    )
                ))
                .header("cookie", fixture.student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "pending-external-replay")
                .body(Body::from(r#"{"response":{"kind":"externalTool"}}"#))
                .expect("external replay request"),
        )
        .await
        .expect("external replay response");

    assert_eq!(replay.status(), StatusCode::ACCEPTED);
    assert_eq!(replay.headers()["cache-control"], "no-store");
    assert_eq!(
        json(replay).await,
        serde_json::json!({
            "kind": "accepted_pending",
            "accepted": true,
            "attemptId": fixture.attempt.id,
            "automatedGradingStatus": "pending",
            "nextAction": "check_status",
        })
    );
    assert_eq!(
        (
            fixture.transport.proxy_calls(),
            fixture.transport.result_calls(),
            fixture
                .route_backend
                .submission_calls
                .load(Ordering::SeqCst),
        ),
        (0, 0, 0)
    );
}

#[tokio::test]
async fn generic_submission_rejects_external_tool_before_acceptance_or_provider_calls() {
    use adapter_imathas::test_support::RecordedContractedTransportMode;

    let fixture = contracted_route_fixture(RecordedContractedTransportMode::Verified).await;
    let response = fixture
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(submission_path(
                    fixture.course,
                    fixture.assignment,
                    fixture.attempt.id,
                ))
                .header("cookie", fixture.student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "generic-external-tool")
                .body(Body::from(r#"{"response":{"kind":"externalTool"}}"#))
                .expect("generic external-tool submission"),
        )
        .await
        .expect("generic external-tool response");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json(response).await,
        serde_json::json!({ "error": "external-tool submissions require the launch route" })
    );
    assert_eq!(fixture.transport.result_calls(), 0);
    assert_eq!(
        fixture
            .route_backend
            .submission_calls
            .load(Ordering::SeqCst),
        0
    );
}
