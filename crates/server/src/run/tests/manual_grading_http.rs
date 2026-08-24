use super::*;
use learning_data_access::{PageRequest, PageSize, RevokeCourseMember, SessionStore};

const INSTRUCTOR: u128 = 2;

fn manual_grade_request(
    attempt: QuestionAttemptId,
    cookie: &str,
    if_match: Option<&str>,
    action: Option<Uuid>,
    body: serde_json::Value,
) -> Request<Body> {
    let mut request = Request::builder()
        .method("PUT")
        .uri(format!("/api/attempts/{attempt}/manual-grade"))
        .header("cookie", cookie)
        .header("content-type", "application/json");
    if let Some(if_match) = if_match {
        request = request.header("if-match", if_match);
    }
    if let Some(action) = action {
        request = request.header("idempotency-key", action.to_string());
    }
    request
        .body(Body::from(body.to_string()))
        .expect("manual grade request")
}

async fn pending_manual_fixture() -> (
    Arc<MemoryStore>,
    Arc<NumericBackend>,
    Router,
    String,
    String,
    String,
    QuestionAttempt,
) {
    let (store, _, _, student_cookie, outsider_cookie, assignment, _) = fixture().await;
    let backend = Arc::new(NumericBackend {
        manual_grading_required: true,
        ..NumericBackend::default()
    });
    let app = router(Arc::clone(&store), Arc::clone(&backend));
    let attempt = active_attempt_for(
        &app,
        CourseId::from_uuid(id(5)),
        assignment,
        &student_cookie,
    )
    .await;
    let submit = || {
        Request::builder()
            .method("POST")
            .uri(submission_path(
                CourseId::from_uuid(id(5)),
                assignment,
                attempt.id,
            ))
            .header("cookie", &student_cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", "manual-http-pending")
            .body(Body::from(
                serde_json::json!({
                    "response": { "kind": "numeric", "value": 18.0 }
                })
                .to_string(),
            ))
            .expect("pending manual submission request")
    };
    let submitted = app
        .clone()
        .oneshot(submit())
        .await
        .expect("pending manual submission response");
    assert_eq!(submitted.status(), StatusCode::OK);
    let submitted = json(submitted).await;
    assert_eq!(submitted["attempt"]["status"], "needs_manual_grading");
    assert_eq!(submitted["attempt"]["result"], serde_json::Value::Null);
    assert_eq!(
        submitted["attempt"]["response"],
        serde_json::json!({ "kind": "numeric", "value": 18.0 })
    );
    let replayed = app
        .clone()
        .oneshot(submit())
        .await
        .expect("pending manual submission replay response");
    assert_eq!(replayed.status(), StatusCode::OK);
    assert_eq!(json(replayed).await, submitted);
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);
    let instructor_cookie = issued_cookie(
        store.as_ref(),
        UserId::from_uuid(id(INSTRUCTOR)),
        "Instructor",
    )
    .await;
    (
        store,
        backend,
        app,
        instructor_cookie,
        student_cookie,
        outsider_cookie,
        attempt,
    )
}

#[tokio::test]
async fn manual_grade_http_is_private_revisioned_and_replay_safe() {
    let (_store, _backend, app, instructor_cookie, student_cookie, outsider_cookie, attempt) =
        pending_manual_fixture().await;
    let path = format!("/api/attempts/{}/manual-grade", attempt.id);

    for cookie in [&student_cookie, &outsider_cookie] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&path)
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("unauthorized manual evaluation request"),
            )
            .await
            .expect("unauthorized manual evaluation response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get("cache-control"),
            Some(&HeaderValue::from_static("no-store"))
        );
        assert_eq!(
            json(response).await,
            serde_json::json!({ "error": "attempt not found" })
        );

        let response = app
            .clone()
            .oneshot(manual_grade_request(
                attempt.id,
                cookie,
                Some("\"1\""),
                Some(Uuid::from_u128(99)),
                serde_json::json!({ "creditFraction": "0.75" }),
            ))
            .await
            .expect("unauthorized manual grade response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get("cache-control"),
            Some(&HeaderValue::from_static("no-store"))
        );
        assert_eq!(
            json(response).await,
            serde_json::json!({ "error": "attempt not found" })
        );
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&path)
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("manual evaluation request"),
        )
        .await
        .expect("manual evaluation response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("etag").unwrap(), "\"1\"");
    assert_eq!(
        response.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store"))
    );
    let pending = json(response).await;
    assert_eq!(pending["attempt"], attempt.id.to_string());
    assert_eq!(
        pending["response"],
        serde_json::json!({ "kind": "numeric", "value": 18.0 })
    );
    assert_eq!(pending["status"], "needsManualGrading");
    assert_eq!(pending["creditFraction"], serde_json::Value::Null);
    assert_eq!(pending["revision"], 1);
    for forbidden in ["answer", "answerKey", "grading", "rubric", "correct"] {
        assert!(
            pending.get(forbidden).is_none(),
            "response leaked {forbidden}"
        );
    }

    let missing_precondition = app
        .clone()
        .oneshot(manual_grade_request(
            attempt.id,
            &instructor_cookie,
            None,
            Some(Uuid::from_u128(100)),
            serde_json::json!({ "creditFraction": "0.75" }),
        ))
        .await
        .expect("missing precondition response");
    assert_eq!(
        missing_precondition.status(),
        StatusCode::PRECONDITION_REQUIRED
    );

    let missing_action = app
        .clone()
        .oneshot(manual_grade_request(
            attempt.id,
            &instructor_cookie,
            Some("\"1\""),
            None,
            serde_json::json!({ "creditFraction": "0.75" }),
        ))
        .await
        .expect("missing action response");
    assert_eq!(missing_action.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        missing_action.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store"))
    );

    let unknown_field = app
        .clone()
        .oneshot(manual_grade_request(
            attempt.id,
            &instructor_cookie,
            Some("\"1\""),
            Some(Uuid::from_u128(103)),
            serde_json::json!({ "creditFraction": "0.75", "comment": "must not persist" }),
        ))
        .await
        .expect("unknown field response");
    assert_eq!(unknown_field.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        unknown_field.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store"))
    );

    let malformed_json = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&path)
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .header("if-match", "\"1\"")
                .header("idempotency-key", Uuid::from_u128(104).to_string())
                .body(Body::from("{"))
                .expect("malformed JSON request"),
        )
        .await
        .expect("malformed JSON response");
    assert!(malformed_json.status().is_client_error());
    assert_eq!(
        malformed_json.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store"))
    );

    let oversized_json = format!(
        "{{\"creditFraction\":\"{}\"}}",
        "1".repeat(MAX_SUBMISSION_BODY_BYTES)
    );
    let oversized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&path)
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .header("if-match", "\"1\"")
                .header("idempotency-key", Uuid::from_u128(105).to_string())
                .body(Body::from(oversized_json))
                .expect("oversized JSON request"),
        )
        .await
        .expect("oversized JSON response");
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        oversized.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store"))
    );

    for (index, invalid) in [" 0.75", "+0.75", "00.75", ".75", "7.5e-1"]
        .into_iter()
        .enumerate()
    {
        let response = app
            .clone()
            .oneshot(manual_grade_request(
                attempt.id,
                &instructor_cookie,
                Some("\"1\""),
                Some(Uuid::from_u128(200 + index as u128)),
                serde_json::json!({ "creditFraction": invalid }),
            ))
            .await
            .expect("invalid decimal response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response.headers().get("cache-control"),
            Some(&HeaderValue::from_static("no-store"))
        );
    }

    let action = Uuid::from_u128(101);
    let grade = || {
        manual_grade_request(
            attempt.id,
            &instructor_cookie,
            Some("\"1\""),
            Some(action),
            serde_json::json!({ "creditFraction": "0.750000000000" }),
        )
    };
    let response = app
        .clone()
        .oneshot(grade())
        .await
        .expect("manual grade response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("etag").unwrap(), "\"2\"");
    assert_eq!(
        response.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store"))
    );
    let receipt = json(response).await;
    assert_eq!(receipt["action"], action.to_string());
    assert_eq!(receipt["attempt"], attempt.id.to_string());
    assert_eq!(receipt["resultingRevision"], 2);
    assert!(receipt["scoringGeneration"].as_u64().is_some());
    for forbidden in ["credit", "creditFraction", "response", "result", "correct"] {
        assert!(
            receipt.get(forbidden).is_none(),
            "receipt leaked {forbidden}"
        );
    }

    let replay = app
        .clone()
        .oneshot(grade())
        .await
        .expect("manual grade replay response");
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(replay.headers().get("etag").unwrap(), "\"2\"");
    assert_eq!(json(replay).await, receipt);

    let changed_replay = app
        .clone()
        .oneshot(manual_grade_request(
            attempt.id,
            &instructor_cookie,
            Some("\"1\""),
            Some(action),
            serde_json::json!({ "creditFraction": "0.5" }),
        ))
        .await
        .expect("changed replay response");
    assert_eq!(changed_replay.status(), StatusCode::CONFLICT);

    let stale = app
        .clone()
        .oneshot(manual_grade_request(
            attempt.id,
            &instructor_cookie,
            Some("\"1\""),
            Some(Uuid::from_u128(102)),
            serde_json::json!({ "creditFraction": "0.5" }),
        ))
        .await
        .expect("stale revision response");
    assert_eq!(stale.status(), StatusCode::CONFLICT);

    let current = app
        .oneshot(
            Request::builder()
                .uri(path)
                .header("cookie", instructor_cookie)
                .body(Body::empty())
                .expect("current manual evaluation request"),
        )
        .await
        .expect("current manual evaluation response");
    assert_eq!(current.status(), StatusCode::OK);
    assert_eq!(current.headers().get("etag").unwrap(), "\"2\"");
    let current = json(current).await;
    assert_eq!(current["status"], "graded");
    assert_eq!(current["creditFraction"], "0.75");
    assert_eq!(current["revision"], 2);
}

#[tokio::test]
async fn terminal_attempt_without_delivery_receipt_returns_a_no_store_conflict() {
    let (_store, _backend, app, _instructor_cookie, student_cookie, _outsider_cookie, attempt) =
        pending_manual_fixture().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri(question_path(
                    CourseId::from_uuid(id(5)),
                    AssignmentId::from_uuid(id(6)),
                    attempt.id,
                ))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("terminal question request"),
        )
        .await
        .expect("terminal question response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store")),
    );
    let body = json(response).await;
    assert!(body.get("response").is_none());
    assert!(body.get("gradingEnvelope").is_none());
}

#[tokio::test]
async fn revoked_student_question_delivery_is_concealed_without_an_envelope() {
    let (store, _backend, app, _instructor_cookie, student_cookie, _outsider_cookie, attempt) =
        pending_manual_fixture().await;
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(1)));
    let course = CourseId::from_uuid(id(5));
    let instructor = UserId::from_uuid(id(INSTRUCTOR));
    let instructor_session = SessionTokenHash::compute(b"issued-evidence-route-revocation");
    store
        .create_session(
            instructor_session,
            SessionSubject::new(
                context.tenant_id(),
                instructor,
                "Question delivery instructor",
                vec![UserRole::Instructor],
            )
            .expect("instructor session subject"),
            SessionLifetime::from_seconds(3_600).expect("instructor session lifetime"),
        )
        .await
        .expect("instructor session");
    let roster = store
        .list_course_roster(
            context,
            instructor_session,
            course,
            PageRequest::first(PageSize::new(20).expect("roster page size")),
        )
        .await
        .expect("read current roster");
    let member = roster
        .entries
        .items
        .into_iter()
        .find_map(|entry| match entry {
            learning_data_access::CourseRosterEntry::Member(member)
                if member.user == UserId::from_uuid(id(3)) =>
            {
                Some(member.id)
            }
            learning_data_access::CourseRosterEntry::Member(_)
            | learning_data_access::CourseRosterEntry::Invitation(_) => None,
        })
        .expect("student roster member");
    store
        .revoke_course_member(
            context,
            instructor_session,
            RevokeCourseMember {
                course,
                member,
                expected_revision: roster.policy.revision,
            },
        )
        .await
        .expect("revoke the issued learner membership");

    let response = app
        .oneshot(
            Request::builder()
                .uri(question_path(
                    course,
                    AssignmentId::from_uuid(id(6)),
                    attempt.id,
                ))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("revoked question request"),
        )
        .await
        .expect("revoked question response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store")),
    );
    let body = json(response).await;
    assert!(body.get("response").is_none());
    assert!(body.get("gradingEnvelope").is_none());
}
