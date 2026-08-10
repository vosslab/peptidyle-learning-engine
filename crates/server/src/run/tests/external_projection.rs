use super::*;

#[tokio::test]
async fn external_tool_launch_projection_is_owner_only_and_key_free() {
    let (store, _backend, app, student_cookie, outsider_cookie, assignment, _enrollment) =
        fixture_with_response(ResponseDefinition::ExternalTool {}, true).await;
    let attempt = active_attempt_for(&app, assignment, &student_cookie).await;
    let projection_path = format!("/api/attempts/{}/external-tool-launch", attempt.id);

    let owner_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&projection_path)
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("owner projection request"),
        )
        .await
        .expect("owner projection response");
    assert_eq!(owner_response.status(), StatusCode::OK);
    assert_eq!(owner_response.headers()["cache-control"], "no-store");
    let projection = json(owner_response).await;
    assert_eq!(
        projection,
        serde_json::json!({
            "launchUrl": format!("/api/attempts/{}/external-tool/launch", attempt.id),
        })
    );
    let serialized = projection.to_string();
    for forbidden in [
        "provider",
        "itemRef",
        "snapshot",
        "answer",
        "solution",
        "token",
        "nonce",
        "credential",
        "score",
        "http://",
        "https://",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "projection leaked {forbidden}"
        );
    }

    let outsider_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&projection_path)
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("outsider projection request"),
        )
        .await
        .expect("outsider projection response");
    assert_eq!(outsider_response.status(), StatusCode::NOT_FOUND);

    let anonymous_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&projection_path)
                .body(Body::empty())
                .expect("anonymous projection request"),
        )
        .await
        .expect("anonymous projection response");
    assert_eq!(anonymous_response.status(), StatusCode::UNAUTHORIZED);

    let cross_tenant_subject = SessionSubject::new(
        TenantId::from_uuid(id(101)),
        UserId::from_uuid(id(102)),
        "Other tenant",
        vec![UserRole::Student],
    )
    .expect("cross-tenant subject");
    let cross_tenant = crate::auth::issue_session(
        store.as_ref(),
        cross_tenant_subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
            crate::auth::CookieTransport::LocalHttp,
        ),
    )
    .await
    .expect("cross-tenant session")
    .set_cookie
    .split(';')
    .next()
    .expect("cross-tenant cookie pair")
    .to_string();
    let cross_tenant_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&projection_path)
                .header("cookie", cross_tenant)
                .body(Body::empty())
                .expect("cross-tenant projection request"),
        )
        .await
        .expect("cross-tenant projection response");
    assert_eq!(cross_tenant_response.status(), StatusCode::NOT_FOUND);

    let copied_broker_path = format!("/api/attempts/{}/external-tool/launch", attempt.id);
    let copied_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(copied_broker_path)
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("unimplemented broker request"),
        )
        .await
        .expect("unimplemented broker response");
    assert_eq!(copied_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn external_tool_launch_refuses_non_external_and_unsupported_attempts() {
    let (_store, _backend, app, student_cookie, _outsider_cookie, assignment, _enrollment) =
        fixture().await;
    let numeric_attempt = active_attempt_for(&app, assignment, &student_cookie).await;
    let non_external = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/attempts/{}/external-tool-launch",
                    numeric_attempt.id
                ))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("non-external request"),
        )
        .await
        .expect("non-external response");
    assert_eq!(non_external.status(), StatusCode::NOT_FOUND);

    let (_store, _backend, app, student_cookie, _outsider_cookie, assignment, _enrollment) =
        fixture_with_response(ResponseDefinition::ExternalTool {}, false).await;
    let unsupported_attempt = active_attempt_for(&app, assignment, &student_cookie).await;
    let unsupported = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/attempts/{}/external-tool-launch",
                    unsupported_attempt.id
                ))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("unsupported request"),
        )
        .await
        .expect("unsupported response");
    assert_eq!(unsupported.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
