use super::*;
use learning_data_access::SubmitQuestionAttemptCommand;

#[tokio::test]
async fn prefetch_is_body_free_idempotent_and_binds_the_submission_replay() {
    let (store, backend, app, student_cookie, outsider_cookie, assignment) =
        native_feedback_fixture().await;
    let first = active_attempt_for(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        &student_cookie,
    )
    .await;
    let prefetch = || {
        Request::builder()
            .method("POST")
            .uri(prefetch_path(
                CourseId::from_uuid(id(205)),
                assignment,
                first.id,
            ))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("body-free prefetch request")
    };
    let (first_prefetch, concurrent_prefetch) = tokio::join!(
        app.clone().oneshot(prefetch()),
        app.clone().oneshot(prefetch()),
    );
    let cached = first_prefetch.expect("first concurrent prefetch response");
    let concurrent = concurrent_prefetch.expect("second concurrent prefetch response");
    assert_eq!(cached.status(), StatusCode::OK);
    assert_eq!(concurrent.status(), StatusCode::OK);
    assert_eq!(
        cached.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store"))
    );
    let cached = json(cached).await;
    assert_eq!(json(concurrent).await, cached);
    assert_eq!(cached["predecessor"], serde_json::json!(first.id));
    assert_eq!(cached["run"], serde_json::json!(first.run));
    let cached_json = cached.to_string();
    for forbidden in [
        "answer",
        "key",
        "provider",
        "provenance",
        "flatGrading",
        "webworkReplay",
        "webworkGrading",
        "qtiGrading",
    ] {
        assert!(
            !cached_json.contains(forbidden),
            "prefetch projection must not disclose {forbidden}"
        );
    }
    let repeated = json(
        app.clone()
            .oneshot(prefetch())
            .await
            .expect("repeat prefetch"),
    )
    .await;
    assert_eq!(
        repeated, cached,
        "a retry reproduces the same reserved variation"
    );

    let hostile = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(prefetch_path(
                    CourseId::from_uuid(id(205)),
                    assignment,
                    first.id,
                ))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("hostile prefetch"),
        )
        .await
        .expect("hostile response");
    assert_eq!(hostile.status(), StatusCode::BAD_REQUEST);
    let unauthenticated_hostile = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(prefetch_path(
                    CourseId::from_uuid(id(205)),
                    assignment,
                    first.id,
                ))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("unauthenticated hostile prefetch"),
        )
        .await
        .expect("unauthenticated response");
    assert_eq!(
        unauthenticated_hostile.status(),
        StatusCode::UNAUTHORIZED,
        "authentication occurs before body-shape validation",
    );
    let foreign = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(prefetch_path(
                    CourseId::from_uuid(id(205)),
                    assignment,
                    first.id,
                ))
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("foreign prefetch"),
        )
        .await
        .expect("foreign prefetch response");
    assert_eq!(
        foreign.status(),
        StatusCode::NOT_FOUND,
        "a foreign learner cannot enumerate an owned active attempt"
    );

    let submit = |attempt: QuestionAttemptId, key: &str, choice: &str| {
        Request::builder()
            .method("POST")
            .uri(submission_path(
                CourseId::from_uuid(id(205)),
                assignment,
                attempt,
            ))
            .header("cookie", &student_cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", key)
            .body(Body::from(
                serde_json::json!({"response":{"kind":"multipleChoice","selected":[choice]}})
                    .to_string(),
            ))
            .expect("submission")
    };
    let ester = presented_choice_id(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        first.id,
        &student_cookie,
        0,
    )
    .await;
    let first_response = app
        .clone()
        .oneshot(submit(first.id, "prefetch-first", &ester))
        .await
        .expect("first submit");
    assert_eq!(first_response.status(), StatusCode::ACCEPTED);
    assert_eq!(json(first_response).await["kind"], "accepted_pending");
    drain_one_accepted_submission(&store, Arc::clone(&backend)).await;
    let completed_status = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(submission_status_path(
                    CourseId::from_uuid(id(205)),
                    assignment,
                    first.id,
                ))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("completed status request"),
        )
        .await
        .expect("completed status response");
    assert_eq!(completed_status.status(), StatusCode::OK);
    assert_eq!(json(completed_status).await["kind"], "completed");
    let resumed = app
        .clone()
        .oneshot(start_run_request(
            CourseId::from_uuid(id(205)),
            assignment,
            &student_cookie,
        ))
        .await
        .expect("successor issuance response");
    assert_eq!(resumed.status(), StatusCode::CREATED);
    let first_receipt_response = app
        .clone()
        .oneshot(submit(first.id, "prefetch-first", &ester))
        .await
        .expect("first completed replay");
    assert_eq!(first_receipt_response.status(), StatusCode::OK);
    let first_receipt = json(first_receipt_response).await;
    assert_eq!(first_receipt["nextIssued"]["run"], cached["run"]);
    let next: QuestionAttemptId = serde_json::from_value(first_receipt["nextIssued"]["id"].clone())
        .expect("successor attempt id");
    assert_eq!(
        cached["envelope"]["version"],
        first_receipt["nextIssued"]["questionVersion"]
    );
    assert_eq!(
        cached["envelope"]["seed"],
        first_receipt["nextIssued"]["seed"]
    );
    let final_position_prefetch = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(prefetch_path(
                    CourseId::from_uuid(id(205)),
                    assignment,
                    next,
                ))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("final-position prefetch"),
        )
        .await
        .expect("final-position prefetch response");
    assert_eq!(final_position_prefetch.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        final_position_prefetch.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store")),
        "no-successor prefetches are not cacheable"
    );
    let amide = presented_choice_id(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        next,
        &student_cookie,
        1,
    )
    .await;
    let accepted = app
        .clone()
        .oneshot(submit(next, "prefetch-second", &amide))
        .await
        .expect("next submit");
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    drain_one_accepted_submission(&store, Arc::clone(&backend)).await;
    let completed = app
        .clone()
        .oneshot(submit(next, "prefetch-second", &amide))
        .await
        .expect("next completed replay");
    assert_eq!(completed.status(), StatusCode::OK);
    let replay = app
        .clone()
        .oneshot(submit(first.id, "prefetch-first", &ester))
        .await
        .expect("first replay");
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(
        json(replay).await,
        first_receipt,
        "later completion cannot rewrite the earlier nextIssued receipt"
    );
}

#[tokio::test]
async fn prefetch_preserves_a_backend_owned_render_hash() {
    let (store, backend, _app, student_cookie, _outsider_cookie, assignment) =
        native_feedback_fixture().await;
    let app = router(
        Arc::clone(&store),
        Arc::new(OpaqueRenderedHashBackend { inner: backend }),
        sealed_memory(&store),
        student_submission_status(&store),
        automated_grading(&store),
    );
    let first = active_attempt_for(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        &student_cookie,
    )
    .await;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(prefetch_path(
                    CourseId::from_uuid(id(205)),
                    assignment,
                    first.id,
                ))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("body-free prefetch request"),
        )
        .await
        .expect("prefetch response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert!(
        body["renderedQuestionSha256"]
            .as_str()
            .is_some_and(|value| value.starts_with("backend-owned-render-")),
        "the route preserves the trusted backend's canonical rendered-artifact hash",
    );
}

#[tokio::test]
async fn resumed_run_issues_successor_linked_to_durable_grade() {
    let (store, _backend, app, student_cookie, _outsider_cookie, assignment) =
        native_feedback_fixture().await;
    let first = active_attempt_for(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        &student_cookie,
    )
    .await;
    let ester = presented_choice_id(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        first.id,
        &student_cookie,
        0,
    )
    .await;
    let response = StudentResponse::MultipleChoice {
        selected: vec![ChoiceId::new(ester.clone())],
    };
    let key = SubmissionIdempotencyKey::parse("crash-before-successor-link").expect("valid key");
    store
        .submit_question_attempt(
            TenantContext::from_authenticated_session(TenantId::from_uuid(id(201))),
            SubmitQuestionAttemptCommand {
                actor: UserId::from_uuid(id(203)),
                attempt: first.id,
                binding: StudentWorkRoutingBinding::new(CourseId::from_uuid(id(205)), assignment),
                response,
                result: AttemptResult {
                    correct: false,
                    points_earned: 0.0,
                    points_possible: 2.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: key,
            },
        )
        .await
        .expect("arrange durable grade commit before process crash");
    let resumed = app
        .clone()
        .oneshot(start_run_request(
            CourseId::from_uuid(id(205)),
            assignment,
            &student_cookie,
        ))
        .await
        .expect("resume response");
    assert_eq!(resumed.status(), StatusCode::CREATED);
    let pending_replay = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(submission_path(
                    CourseId::from_uuid(id(205)),
                    assignment,
                    first.id,
                ))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "crash-before-successor-link")
                .body(Body::from(
                    serde_json::json!({"response":{"kind":"multipleChoice","selected":[ester]}})
                        .to_string(),
                ))
                .expect("pending replay"),
        )
        .await
        .expect("pending replay response");
    assert_eq!(pending_replay.status(), StatusCode::OK);
    let pending_receipt = json(pending_replay).await;
    assert_eq!(pending_receipt["nextPending"], false);
    assert!(
        pending_receipt["nextIssued"].is_object(),
        "the exact nested replay returns its bound successor"
    );
    let after_resume = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/attempts", first.run))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("attempt page"),
        )
        .await
        .expect("attempts after resume");
    let attempts: Page<QuestionAttempt> =
        serde_json::from_value(json(after_resume).await).expect("attempt page");
    assert_eq!(
        attempts.items.len(),
        2,
        "resume heals only through the durable pending predecessor link",
    );
    let replay = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(submission_path(
                    CourseId::from_uuid(id(205)),
                    assignment,
                    first.id,
                ))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "crash-before-successor-link")
                .body(Body::from(
                    serde_json::json!({"response":{"kind":"multipleChoice","selected":[ester]}})
                        .to_string(),
                ))
                .expect("replay"),
        )
        .await
        .expect("replay response");
    assert_eq!(replay.status(), StatusCode::OK);
    let receipt = json(replay).await;
    assert!(
        receipt["nextIssued"].is_object(),
        "later replay returns the exact healed successor link"
    );
    assert_eq!(receipt["nextIssued"], pending_receipt["nextIssued"]);
    assert_eq!(
        receipt["nextIssued"]["id"],
        pending_receipt["nextIssued"]["id"]
    );
}

#[tokio::test]
async fn successor_delivery_failure_returns_the_durable_receipt_without_regrading() {
    let (store, backend, app, student_cookie, _outsider_cookie, assignment) =
        native_feedback_fixture().await;
    let first = active_attempt_for(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        &student_cookie,
    )
    .await;
    let ester = presented_choice_id(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        first.id,
        &student_cookie,
        0,
    )
    .await;
    let unavailable_app = router(
        Arc::clone(&store),
        Arc::new(UnavailableSuccessorBackend {
            inner: Arc::clone(&backend),
            fail_next_issue: AtomicBool::new(true),
        }),
        sealed_memory(&store),
        student_submission_status(&store),
        automated_grading(&store),
    );
    let submit = || {
        Request::builder()
            .method("POST")
            .uri(submission_path(
                CourseId::from_uuid(id(205)),
                assignment,
                first.id,
            ))
            .header("cookie", &student_cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", "successor-delivery-outage")
            .body(Body::from(
                serde_json::json!({
                    "response": { "kind": "multipleChoice", "selected": [ester] }
                })
                .to_string(),
            ))
            .expect("submission")
    };

    let first_response = unavailable_app
        .clone()
        .oneshot(submit())
        .await
        .expect("first response after successor outage");
    assert_eq!(first_response.status(), StatusCode::ACCEPTED);
    assert_eq!(json(first_response).await["kind"], "accepted_pending");
    drain_one_accepted_submission(
        &store,
        Arc::new(UnavailableSuccessorBackend {
            inner: Arc::clone(&backend),
            fail_next_issue: AtomicBool::new(false),
        }),
    )
    .await;
    let first_replay = unavailable_app
        .clone()
        .oneshot(submit())
        .await
        .expect("first completed replay after successor outage");
    assert_eq!(first_replay.status(), StatusCode::OK);
    let first_receipt = json(first_replay).await;
    assert_eq!(first_receipt["accepted"], true);
    assert_eq!(first_receipt["nextPending"], true);
    assert!(first_receipt["nextIssued"].is_null());
    assert_eq!(backend.submissions.load(Ordering::SeqCst), 1);

    let replay = unavailable_app
        .clone()
        .oneshot(submit())
        .await
        .expect("pending replay response");
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_receipt = json(replay).await;
    assert_eq!(replay_receipt["accepted"], first_receipt["accepted"]);
    assert_eq!(replay_receipt["attempt"], first_receipt["attempt"]);
    assert_eq!(replay_receipt["nextPending"], false);
    assert!(
        replay_receipt["nextIssued"].is_object(),
        "the exact nested replay retries only bound successor delivery"
    );
    assert_eq!(
        backend.submissions.load(Ordering::SeqCst),
        1,
        "replaying a pending receipt must not invoke grading again",
    );

    let resumed = app
        .clone()
        .oneshot(start_run_request(
            CourseId::from_uuid(id(205)),
            assignment,
            &student_cookie,
        ))
        .await
        .expect("run recovery response");
    assert_eq!(resumed.status(), StatusCode::CREATED);
    let healed_replay = app
        .clone()
        .oneshot(submit())
        .await
        .expect("healed replay response");
    assert_eq!(healed_replay.status(), StatusCode::OK);
    let healed_receipt = json(healed_replay).await;
    assert_eq!(healed_receipt["nextIssued"], replay_receipt["nextIssued"]);
    assert_eq!(backend.submissions.load(Ordering::SeqCst), 1);
}
