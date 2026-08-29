use super::*;

struct ClaimLostAcceptedSubmissionFastPath;

#[async_trait::async_trait]
impl crate::accepted_submission_worker::AcceptedSubmissionFastPath
    for ClaimLostAcceptedSubmissionFastPath
{
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

#[tokio::test]
async fn valid_first_submission_executes_once_and_returns_the_shared_completed_projection() {
    let (store, backend, _app, student_cookie, _, assignment, _) = fixture().await;
    let sealed = Arc::new(CountingSealedExecution {
        inner: sealed_memory(&store),
        calls: AtomicUsize::new(0),
        refuse: AtomicBool::new(false),
    });
    let app = router_with_accepted_submission_fast_path(
        Arc::clone(&store),
        Arc::clone(&backend),
        sealed.clone(),
        student_submission_status(&store),
        automated_grading(&store),
        accepted_submission_fast_path(&store, Arc::clone(&backend)),
    );
    let attempt = active_attempt_for(
        &app,
        CourseId::from_uuid(id(5)),
        assignment,
        &student_cookie,
    )
    .await;
    let request = || {
        Request::builder()
            .method("POST")
            .uri(submission_path(
                CourseId::from_uuid(id(5)),
                assignment,
                attempt.id,
            ))
            .header("cookie", &student_cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", "sealed-replay")
            .body(Body::from(
                serde_json::json!({ "response": { "kind": "numeric", "value": 18.0 } }).to_string(),
            ))
            .expect("submission request")
    };

    let accepted = app
        .clone()
        .oneshot(request())
        .await
        .expect("first submission");
    assert_eq!(accepted.status(), StatusCode::OK);
    assert_eq!(accepted.headers()["cache-control"], "no-store");
    let receipt = json(accepted).await;
    assert_eq!(receipt["kind"], "completed");
    assert_eq!(receipt["accepted"], true);
    assert_eq!(receipt["attempt"]["id"], serde_json::json!(attempt.id));
    assert_eq!(receipt["attempt"]["status"], "submitted");
    assert!(receipt.pointer("/response").is_none());
    assert!(receipt.pointer("/result").is_none());
    assert_eq!(sealed.calls.load(Ordering::SeqCst), 0);
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);

    let replay = app
        .clone()
        .oneshot(request())
        .await
        .expect("idempotent submission replay");
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_receipt = json(replay).await;
    assert_eq!(replay_receipt["kind"], "completed");
    assert!(replay_receipt.pointer("/response").is_none());
    assert!(replay_receipt.pointer("/result").is_none());
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);

    let status = app
        .oneshot(
            Request::builder()
                .uri(submission_status_path(
                    CourseId::from_uuid(id(5)),
                    assignment,
                    attempt.id,
                ))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("durable status request"),
        )
        .await
        .expect("durable status response");
    assert_eq!(status.status(), StatusCode::OK);
    assert_eq!(json(status).await["kind"], "completed");
}

#[tokio::test]
async fn exact_fast_path_claim_loss_returns_the_answer_free_pending_projection() {
    let (store, backend, _app, student_cookie, _, assignment, _) = fixture().await;
    let app = router_with_accepted_submission_fast_path(
        Arc::clone(&store),
        Arc::clone(&backend),
        sealed_memory(&store),
        student_submission_status(&store),
        automated_grading(&store),
        Arc::new(ClaimLostAcceptedSubmissionFastPath),
    );
    let course = CourseId::from_uuid(id(5));
    let attempt = active_attempt_for(&app, course, assignment, &student_cookie).await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(submission_path(course, assignment, attempt.id))
                .header("cookie", student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "exact-claim-loss")
                .body(Body::from(
                    serde_json::json!({ "response": { "kind": "numeric", "value": 18.0 } })
                        .to_string(),
                ))
                .expect("submission request"),
        )
        .await
        .expect("submission response");

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(json(response).await["kind"], "accepted_pending");
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn terminal_fast_path_execution_returns_the_shared_instructor_attention_projection() {
    let (store, _, _app, student_cookie, _, assignment, _) = fixture().await;
    let backend = Arc::new(NumericBackend {
        unsupported_grading: true,
        ..NumericBackend::default()
    });
    let app = router_with_accepted_submission_fast_path(
        Arc::clone(&store),
        Arc::clone(&backend),
        sealed_memory(&store),
        student_submission_status(&store),
        automated_grading(&store),
        accepted_submission_fast_path(&store, Arc::clone(&backend)),
    );
    let course = CourseId::from_uuid(id(5));
    let attempt = active_attempt_for(&app, course, assignment, &student_cookie).await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(submission_path(course, assignment, attempt.id))
                .header("cookie", student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "terminal-fast-path")
                .body(Body::from(
                    serde_json::json!({ "response": { "kind": "numeric", "value": 18.0 } })
                        .to_string(),
                ))
                .expect("submission request"),
        )
        .await
        .expect("submission response");

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let body = json(response).await;
    assert_eq!(body["kind"], "instructor_attention");
    assert_eq!(body["automatedGradingStatus"], "instructor_attention");
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);
}
