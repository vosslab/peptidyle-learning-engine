use super::*;

#[tokio::test]
async fn file_upload_submission_refuses_untrusted_object_key_before_backend_or_store_mutation() {
    let (store, backend, app, student_cookie, _outsider_cookie, assignment, _enrollment) =
        fixture_with_response(
            ResponseDefinition::FileUpload {
                max_bytes: 1_024,
                accepted_extensions: vec!["pdf".to_string()],
            },
            false,
        )
        .await;
    let attempt = active_attempt_for(&app, assignment, &student_cookie).await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/submissions/{}", attempt.id))
                .header("cookie", student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "forged-file-upload")
                .body(Body::from(
                    serde_json::json!({
                        "response": {
                            "kind": "fileUpload",
                            "objectKey": "student-records/foreign-tenant/private.pdf",
                        }
                    })
                    .to_string(),
                ))
                .expect("forged file-upload request"),
        )
        .await
        .expect("forged file-upload response");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json(response).await,
        serde_json::json!({ "error": "file upload submissions are unavailable" })
    );
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        store
            .get_question_attempt(
                TenantContext::from_authenticated_session(TenantId::from_uuid(id(1))),
                attempt.id,
            )
            .await
            .expect("attempt read"),
        Some(attempt)
    );
}

#[tokio::test]
async fn submission_validates_attempt_specific_rendered_choice_ids() {
    let published_response = ResponseDefinition::MultipleChoice {
        choices: vec![
            ChoiceOption {
                id: ChoiceId::new("published-a"),
                body: Vec::new(),
            },
            ChoiceOption {
                id: ChoiceId::new("published-b"),
                body: Vec::new(),
            },
        ],
        selection: SelectionCardinality::ExactlyOne,
    };
    let rendered_response = ResponseDefinition::MultipleChoice {
        choices: vec![
            ChoiceOption {
                id: ChoiceId::new("rendered-a"),
                body: Vec::new(),
            },
            ChoiceOption {
                id: ChoiceId::new("rendered-b"),
                body: Vec::new(),
            },
        ],
        selection: SelectionCardinality::ExactlyOne,
    };
    let (_store, backend, app, student_cookie, _, assignment, _) =
        fixture_with_response(published_response, false).await;
    *backend
        .issued_response
        .lock()
        .expect("issued response fixture") = Some(rendered_response);
    let attempt = active_attempt_for(&app, assignment, &student_cookie).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/submissions/{}", attempt.id))
                .header("cookie", student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "rendered-choice-id")
                .body(Body::from(
                    serde_json::json!({
                        "response": {
                            "kind": "multipleChoice",
                            "selected": ["rendered-a"],
                        }
                    })
                    .to_string(),
                ))
                .expect("rendered choice submission"),
        )
        .await
        .expect("rendered choice response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(backend.reproduce_calls.load(Ordering::SeqCst), 1);
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn runs_resume_submit_idempotently_and_keep_keys_server_only() {
    let (store, backend, app, student_cookie, outsider_cookie, assignment, enrollment) =
        fixture().await;
    let first_response = app
        .clone()
        .oneshot(post_json(
            "/api/runs",
            &student_cookie,
            serde_json::json!({ "assignmentId": assignment }),
        ))
        .await
        .expect("start response");
    assert_eq!(first_response.status(), StatusCode::CREATED);
    let first: AssignmentRun =
        serde_json::from_value(json(first_response).await).expect("run contract");
    assert_eq!(
        first.started_at,
        ActivityTimestamp::from_unix_millis(10_000)
    );

    let attempts_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/attempts", first.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("attempt response");
    let attempts: Page<QuestionAttempt> =
        serde_json::from_value(json(attempts_response).await).expect("attempt page");
    let issued = attempts.items.first().expect("issued attempt");
    assert_eq!(issued.timer.issued_at, first.started_at);
    assert!(issued.response.is_none());
    assert!(issued.result.is_none());

    let question_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/attempts/{}/question", issued.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("issued question request"),
        )
        .await
        .expect("issued question response");
    assert_eq!(question_response.status(), StatusCode::OK);
    let envelope = json(question_response).await;
    assert_eq!(
        envelope["version"],
        serde_json::json!(issued.question_version)
    );
    assert_eq!(envelope["seed"], serde_json::json!(issued.seed));
    assert_eq!(envelope["response"]["kind"], "numeric");
    let serialized_envelope = envelope.to_string();
    for answer_bearing_field in ["answerKey", "expected", "rubric", "grading"] {
        assert!(!serialized_envelope.contains(answer_bearing_field));
    }

    let submission_body = serde_json::json!({
        "response": { "kind": "numeric", "value": 18.0 }
    });
    let submit = |key: &str, body: serde_json::Value| {
        Request::builder()
            .method("POST")
            .uri(format!("/api/submissions/{}", issued.id))
            .header("cookie", &student_cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", key)
            .body(Body::from(body.to_string()))
            .expect("request")
    };
    let malformed_submission = app
        .clone()
        .oneshot(submit(
            "malformed-request",
            serde_json::json!({
                "response": { "kind": "shortText", "text": "eighteen" }
            }),
        ))
        .await
        .expect("malformed submission response");
    assert_eq!(
        malformed_submission.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);

    let first_submission = app
        .clone()
        .oneshot(submit("same-request", submission_body.clone()))
        .await
        .expect("submission response");
    assert_eq!(first_submission.status(), StatusCode::OK);
    let first_receipt = json(first_submission).await;
    assert_eq!(first_receipt["accepted"], true);
    assert_eq!(first_receipt["attempt"]["result"]["correct"], true);
    // The generic NumericBackend takes the default server grade path: it
    // may honestly disclose the grade, but it cannot fabricate native
    // teaching blocks it did not produce.
    assert_eq!(first_receipt["feedback"]["correctness"], true);
    assert_eq!(first_receipt["feedback"]["pointsEarned"], 1.0);
    assert!(first_receipt["feedback"].get("hint").is_none());
    assert!(first_receipt["feedback"].get("correctResponse").is_none());
    assert!(first_receipt["feedback"].get("rationale").is_none());
    let serialized_receipt = first_receipt.to_string();
    for answer_bearing_field in ["answerKey", "expected", "rubric", "feedbackContent"] {
        assert!(!serialized_receipt.contains(answer_bearing_field));
    }
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);

    let replay = app
        .clone()
        .oneshot(submit("same-request", submission_body.clone()))
        .await
        .expect("replay response");
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(json(replay).await, first_receipt);
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);

    let changed = app
        .clone()
        .oneshot(submit(
            "same-request",
            serde_json::json!({
                "response": { "kind": "numeric", "value": 19.0 }
            }),
        ))
        .await
        .expect("changed replay response");
    assert_eq!(changed.status(), StatusCode::CONFLICT);
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);

    let outsider = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}", first.id))
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("outsider response");
    assert_eq!(outsider.status(), StatusCode::NOT_FOUND);

    let outsider_question = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/attempts/{}/question", issued.id))
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("outsider question request"),
        )
        .await
        .expect("outsider question response");
    assert_eq!(outsider_question.status(), StatusCode::NOT_FOUND);

    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(20_000))
        .expect("advance clock");
    let practice_response = app
        .clone()
        .oneshot(post_json(
            "/api/runs",
            &student_cookie,
            serde_json::json!({ "assignmentId": assignment }),
        ))
        .await
        .expect("practice response");
    let practice: AssignmentRun =
        serde_json::from_value(json(practice_response).await).expect("practice run");
    assert_eq!(practice.run_number, 2);
    assert_eq!(
        practice.started_at,
        ActivityTimestamp::from_unix_millis(20_000)
    );
    let seeds = backend.issued_seeds.lock().expect("seed record").clone();
    assert_eq!(seeds.len(), 2);
    assert_ne!(seeds[0], seeds[1]);

    let first_history_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/enrollments/{enrollment}/runs?pageSize=1"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("run history request"),
        )
        .await
        .expect("first run history response");
    let first_history: Page<AssignmentRun> =
        serde_json::from_value(json(first_history_response).await).expect("run history page");
    let cursor = first_history
        .next_cursor
        .expect("first run history page should continue");
    let second_history_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/enrollments/{enrollment}/runs?pageSize=1&cursor={}",
                    cursor.as_str()
                ))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("continued run history request"),
        )
        .await
        .expect("continued run history response");
    let second_history: Page<AssignmentRun> =
        serde_json::from_value(json(second_history_response).await)
            .expect("continued run history page");
    assert_eq!(
        (
            first_history.items[0].run_number,
            second_history.items[0].run_number,
            second_history.next_cursor,
        ),
        (1, 2, None)
    );

    for path in [
        format!("/api/runs/{}/attempts", first.id),
        format!("/api/enrollments/{enrollment}/runs"),
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

    let summary_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/grading/summaries/{enrollment}"))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("summary response");
    let summary: StudentAssignmentSummary =
        serde_json::from_value(json(summary_response).await).expect("summary");
    assert_eq!(
        (summary.completed_run_count, summary.total_question_attempts),
        (1, 1)
    );
}

#[tokio::test]
async fn a_run_issues_only_one_active_question_then_advances() {
    let (store, backend, app, student_cookie, _, assignment_id, _) = fixture().await;
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(1)));
    let stored_assignment = store
        .get_assignment_for_edit(context, assignment_id)
        .await
        .expect("assignment read")
        .expect("fixture assignment");
    let mut items = stored_assignment.record.items.clone();
    let mut duplicate = items[0].clone();
    duplicate.id = question_model::AssignmentItemId::from_uuid(id(1_100_000));
    duplicate.position = u32::try_from(items.len()).expect("test assignment position fits u32");
    items.push(duplicate);
    store
        .replace_assignment(
            context,
            stored_assignment.record.course_id,
            assignment_id,
            stored_assignment.revision,
            learning_data_access::AssignmentUpdate {
                title: stored_assignment.record.title,
                items,
                selection_groups: stored_assignment.record.selection_groups,
                policies: stored_assignment.record.policies,
            },
        )
        .await
        .expect("two-position assignment");

    let started = app
        .clone()
        .oneshot(post_json(
            "/api/runs",
            &student_cookie,
            serde_json::json!({ "assignmentId": assignment_id }),
        ))
        .await
        .expect("start response");
    let run: AssignmentRun = serde_json::from_value(json(started).await).expect("run response");
    let first_page_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/attempts?pageSize=1", run.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("attempt request"),
        )
        .await
        .expect("first attempt page");
    let first_page: Page<QuestionAttempt> =
        serde_json::from_value(json(first_page_response).await).expect("attempt page");
    assert_eq!(first_page.items.len(), 1);
    assert_eq!(first_page.items[0].assignment_position, 0);

    let submission = Request::builder()
        .method("POST")
        .uri(format!("/api/submissions/{}", first_page.items[0].id))
        .header("cookie", &student_cookie)
        .header("content-type", "application/json")
        .header("idempotency-key", "advance-to-second")
        .body(Body::from(
            serde_json::json!({
                "response": { "kind": "numeric", "value": 18.0 }
            })
            .to_string(),
        ))
        .expect("submission request");
    let submission_response = app
        .clone()
        .oneshot(submission)
        .await
        .expect("submission response");
    assert_eq!(submission_response.status(), StatusCode::OK);

    let second_page_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/attempts?pageSize=1", run.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("second attempt request"),
        )
        .await
        .expect("second attempt page");
    let second_page: Page<QuestionAttempt> =
        serde_json::from_value(json(second_page_response).await).expect("attempt page");
    assert_eq!(second_page.items.len(), 1);
    assert!(second_page.items[0].response.is_some());
    let cursor = second_page
        .next_cursor
        .expect("bounded first attempt page must continue");
    let continued_page_response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/runs/{}/attempts?pageSize=1&cursor={}",
                    run.id,
                    cursor.as_str()
                ))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("continued attempt request"),
        )
        .await
        .expect("continued attempt page");
    let continued_page: Page<QuestionAttempt> =
        serde_json::from_value(json(continued_page_response).await).expect("attempt page");
    assert_eq!(continued_page.items.len(), 1);
    assert_ne!(second_page.items[0].id, continued_page.items[0].id);
    assert_eq!(continued_page.items[0].assignment_position, 1);
    assert!(continued_page.items[0].response.is_none());
    assert_eq!(continued_page.next_cursor, None);
    assert_eq!(backend.issued_seeds.lock().expect("seed record").len(), 2);
}
