use super::*;

#[tokio::test]
async fn legacy_flat_start_route_is_not_available() {
    let (_store, _backend, app, student_cookie, _, _, _) = fixture().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/runs")
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("legacy start request"),
        )
        .await
        .expect("legacy start response");

    assert!(!response.status().is_success());
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn start_body_cannot_override_nested_binding_or_reveal_mismatch() {
    let (store, _backend, app, student_cookie, _, assignment, _) = fixture().await;
    let course = CourseId::from_uuid(id(5));
    let hostile_course = CourseId::from_uuid(id(500));
    let hostile_assignment = AssignmentId::from_uuid(id(600));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/runs"
                ))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "courseId": hostile_course,
                        "assignmentId": hostile_assignment,
                    })
                    .to_string(),
                ))
                .expect("hostile nested start request"),
        )
        .await
        .expect("hostile nested start response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let run: AssignmentRun = serde_json::from_value(json(response).await).expect("run response");
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(1)));
    let enrollment = store
        .get_enrollment(context, run.enrollment)
        .await
        .expect("enrollment lookup")
        .expect("started enrollment");
    assert_eq!(enrollment.assignment, assignment);
    let assignment_record = store
        .get_assignment_for_edit(context, enrollment.assignment)
        .await
        .expect("assignment lookup")
        .expect("started assignment");
    assert_eq!(assignment_record.record.course_id, course);

    let concealed_mismatch = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/courses/{course}/assignments/{hostile_assignment}/runs"
                ))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("mismatched nested start request"),
        )
        .await
        .expect("mismatched nested start response");
    assert_eq!(concealed_mismatch.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn follow_on_routes_require_the_exact_nested_binding_before_mutation() {
    let (store, backend, app, student_cookie, _, assignment, _) = fixture().await;
    let course = CourseId::from_uuid(id(5));
    let attempt = active_attempt_for(&app, course, assignment, &student_cookie).await;
    let issues_before = backend.issued_seeds.lock().expect("issue counter").len();
    let wrong_course = CourseId::from_uuid(id(500));

    let prefetch = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(prefetch_path(wrong_course, assignment, attempt.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("wrong-binding prefetch request"),
        )
        .await
        .expect("wrong-binding prefetch response");
    assert_eq!(prefetch.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        backend.issued_seeds.lock().expect("issue counter").len(),
        issues_before,
        "a mismatched route cannot reserve or render a successor",
    );

    let submission = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(submission_path(wrong_course, assignment, attempt.id))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "wrong-nested-binding")
                .body(Body::from(
                    serde_json::json!({
                        "response": { "kind": "numeric", "value": 18.0 }
                    })
                    .to_string(),
                ))
                .expect("wrong-binding submission request"),
        )
        .await
        .expect("wrong-binding submission response");
    assert_eq!(submission.status(), StatusCode::NOT_FOUND);
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);
    let stored = store
        .learner_get_question_attempt(
            TenantContext::from_authenticated_session(TenantId::from_uuid(id(1))),
            UserId::from_uuid(id(3)),
            attempt.id,
        )
        .await
        .expect("attempt read")
        .expect("attempt");
    assert!(stored.response.is_none());
}

#[tokio::test]
async fn legacy_flat_follow_on_routes_are_not_available() {
    let (_store, _backend, app, student_cookie, _, assignment, _) = fixture().await;
    let attempt = active_attempt_for(
        &app,
        CourseId::from_uuid(id(5)),
        assignment,
        &student_cookie,
    )
    .await;
    for request in [
        Request::builder()
            .method("POST")
            .uri(format!("/api/attempts/{}/prefetch-next", attempt.id))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("legacy prefetch request"),
        Request::builder()
            .method("POST")
            .uri(format!("/api/submissions/{}", attempt.id))
            .header("cookie", &student_cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", "legacy-flat-submission")
            .body(Body::from(
                serde_json::json!({
                    "response": { "kind": "numeric", "value": 18.0 }
                })
                .to_string(),
            ))
            .expect("legacy submission request"),
    ] {
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("legacy route response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
