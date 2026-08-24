use super::*;

#[tokio::test]
async fn archive_fence_refuses_run_aliases_before_any_backend_call() {
    let (store, backend, app, student_cookie, _, assignment, enrollment) = fixture().await;
    let active = active_attempt_for(
        &app,
        CourseId::from_uuid(id(5)),
        assignment,
        &student_cookie,
    )
    .await;
    let issued_before = backend.issued_seeds.lock().expect("seed record").len();
    assert_eq!(issued_before, 1);
    assert_eq!(backend.reproduce_calls.load(Ordering::SeqCst), 0);
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);
    assert_eq!(backend.external_launch_calls.load(Ordering::SeqCst), 0);

    prepare_archive_fence(
        store.as_ref(),
        TenantId::from_uuid(id(1)),
        CourseId::from_uuid(id(5)),
    )
    .await;

    let requests = vec![
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/courses/{}/assignments/{assignment}/runs",
                CourseId::from_uuid(id(5))
            ))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived start request"),
        Request::builder()
            .uri(format!("/api/runs/{}", active.run))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived run request"),
        Request::builder()
            .uri(format!("/api/runs/{}/summary", active.run))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived summary request"),
        Request::builder()
            .uri(format!("/api/runs/{}/attempts", active.run))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived attempt list request"),
        Request::builder()
            .uri(format!("/api/attempts/{}", active.id))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived attempt request"),
        Request::builder()
            .uri(question_path(
                CourseId::from_uuid(id(5)),
                assignment,
                active.id,
            ))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived question request"),
        Request::builder()
            .method("POST")
            .uri(prefetch_path(
                CourseId::from_uuid(id(5)),
                assignment,
                active.id,
            ))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived prefetch request"),
        Request::builder()
            .method("POST")
            .uri(format!("/api/attempts/{}/external-tool-launch", active.id))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived external projection request"),
        Request::builder()
            .method("POST")
            .uri(submission_path(
                CourseId::from_uuid(id(5)),
                assignment,
                active.id,
            ))
            .header("cookie", &student_cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", "archive-refusal")
            .body(Body::from(
                serde_json::json!({
                    "response": { "kind": "numeric", "value": 18.0 }
                })
                .to_string(),
            ))
            .expect("archived submission request"),
        Request::builder()
            .method("POST")
            .uri(format!("/api/attempts/{}/feedback-release", active.id))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived feedback release request"),
        Request::builder()
            .uri(format!("/api/grading/summaries/{enrollment}"))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived grading summary request"),
        Request::builder()
            .uri(format!("/api/enrollments/{enrollment}"))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived enrollment request"),
        Request::builder()
            .uri(format!("/api/enrollments/{enrollment}/runs"))
            .header("cookie", &student_cookie)
            .body(Body::empty())
            .expect("archived enrollment runs request"),
    ];
    for request in requests {
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("archived alias response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get("cache-control"),
            Some(&HeaderValue::from_static("no-store"))
        );
    }

    assert_eq!(
        backend.issued_seeds.lock().expect("seed record").len(),
        issued_before
    );
    assert_eq!(backend.reproduce_calls.load(Ordering::SeqCst), 0);
    assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);
    assert_eq!(backend.external_launch_calls.load(Ordering::SeqCst), 0);
}
