use super::*;

#[tokio::test]
async fn external_tool_launch_has_no_legacy_get_projection_or_stateful_get_alias() {
    let (_store, backend, app, student_cookie, _outsider_cookie, assignment, _enrollment) =
        fixture_with_response(ResponseDefinition::ExternalTool {}, true).await;
    let course = CourseId::from_uuid(id(5));
    let attempt = active_attempt_for(&app, course, assignment, &student_cookie).await;
    let legacy_path = format!("/api/attempts/{}/external-tool-launch", attempt.id);
    let retired_flat_path = format!("/api/attempts/{}/external-tool/launch", attempt.id);
    let launch_path = external_tool_launch_path(course, assignment, attempt.id);

    // The former discovery endpoint is absent.  More importantly, a Lax
    // cookie sent on a cross-site top-level GET cannot create broker state:
    // the GET shell refuses a request that has no POST-created launch binding.
    for path in [&legacy_path, &retired_flat_path, &launch_path] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("GET request"),
            )
            .await
            .expect("GET response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "GET {path}");
        assert!(response.headers().get("set-cookie").is_none());
    }
    assert_eq!(backend.external_launch_calls.load(Ordering::SeqCst), 0);
}
