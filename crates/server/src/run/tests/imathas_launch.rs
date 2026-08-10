use super::*;

#[tokio::test]
async fn contracted_imathas_launch_route_is_same_origin_replica_safe_and_secret_free() {
    use adapter_imathas::test_support::RecordedContractedTransportMode;

    let fixture = contracted_route_fixture(RecordedContractedTransportMode::Available).await;
    fixture
        .backend
        .reproduce(
            fixture.context,
            ProblemVersionRef {
                problem: fixture.attempt.problem,
                version: fixture.attempt.question_version,
            },
            &fixture.question,
            &fixture.attempt,
        )
        .await
        .expect("preflight reproduce");
    let path = format!("/api/attempts/{}/external-tool/launch", fixture.attempt.id);
    let shell = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&path)
                .header("cookie", &fixture.student_cookie)
                .body(Body::empty())
                .expect("shell request"),
        )
        .await
        .expect("shell response");
    if shell.status() != StatusCode::OK {
        let status = shell.status();
        let body = to_bytes(shell.into_body(), 256 * 1_024)
            .await
            .expect("error body");
        panic!(
            "shell returned {status}: {}",
            String::from_utf8_lossy(&body)
        );
    }
    assert_eq!(shell.headers()["cache-control"], "no-store");
    let set_cookie = shell.headers()["set-cookie"]
        .to_str()
        .expect("set cookie")
        .to_owned();
    assert!(set_cookie.starts_with("ple_external_launch="));
    assert!(
        set_cookie.contains("HttpOnly")
            && set_cookie.contains("Secure")
            && set_cookie.contains("SameSite=Strict")
    );
    assert!(set_cookie.contains(&format!("Path={path}")));
    let csp = shell.headers()["content-security-policy"]
        .to_str()
        .expect("csp")
        .to_owned();
    let shell_bytes = to_bytes(shell.into_body(), 256 * 1_024)
        .await
        .expect("shell body");
    let shell_body = std::str::from_utf8(&shell_bytes).expect("shell utf8");
    let nonce = shell_body
        .split("<script nonce=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .expect("script nonce");
    assert!(csp.contains(&format!("'nonce-{nonce}'")));
    assert!(shell_body.contains(&format!("src=\"{path}/activity\"")));
    assert!(shell_body.contains("kind:'ple.externalTool.ready'"));
    assert!(shell_body.contains("ple.externalTool.activityReady"));
    assert!(shell_body.contains(&format!("attemptId:'{}'", fixture.attempt.id)));
    assert!(shell_body.contains("event.source!==frame.contentWindow"));
    assert!(shell_body.contains("event.origin!=='null'"));
    assert!(!shell_body.contains("addEventListener('load'"));
    assert!(!shell_body.contains("allow-same-origin"));
    for secret in [
        "institution-imathas",
        "recorded-proxy-session",
        "jwt",
        "source_sha",
        "score",
        "answer",
    ] {
        assert!(
            !shell_body.to_ascii_lowercase().contains(secret),
            "shell leaked {secret}"
        );
        assert!(
            !set_cookie.to_ascii_lowercase().contains(secret),
            "cookie leaked {secret}"
        );
    }
    let launch_cookie = set_cookie.split(';').next().expect("cookie pair");
    let activity_path = format!("{path}/activity");
    let replica = external_tool_router(
        Arc::clone(&fixture.store),
        Arc::clone(&fixture.backend),
        Arc::new(
            crate::imathas_backend::LaunchStateAead::from_server_secret([84; 32])
                .expect("replica aead"),
        ),
    );
    for request in [
        Request::builder()
            .uri(&activity_path)
            .header(
                "cookie",
                format!("{}; {launch_cookie}", fixture.student_cookie),
            )
            .body(Body::empty())
            .expect("GET"),
        Request::builder()
            .method("POST")
            .uri(&activity_path)
            .header(
                "cookie",
                format!("{}; {launch_cookie}", fixture.student_cookie),
            )
            .body(Body::from("answer=kept-local"))
            .expect("POST"),
    ] {
        let response = replica
            .clone()
            .oneshot(request)
            .await
            .expect("activity response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(
            response.headers()["content-type"],
            "text/html; charset=utf-8"
        );
        let activity_csp = response.headers()["content-security-policy"]
            .to_str()
            .expect("activity CSP");
        assert!(activity_csp.starts_with("default-src 'none'; script-src 'nonce-"));
        assert!(!activity_csp.contains("unsafe-inline"));
        let body = to_bytes(response.into_body(), 256 * 1024)
            .await
            .expect("activity body");
        let body = std::str::from_utf8(&body).expect("activity UTF-8");
        assert!(body.starts_with("<!doctype html><title>Recorded protected activity</title>"));
        assert!(body.contains("kind:'ple.externalTool.activityReady'"));
        assert!(body.contains(&format!("attemptId:'{}'", fixture.attempt.id)));
    }
    for (cookie, target) in [
        (
            format!("{}; ple_external_launch=bad", fixture.student_cookie),
            activity_path.clone(),
        ),
        (
            format!("{}; {launch_cookie}", fixture.outsider_cookie),
            activity_path.clone(),
        ),
        (
            format!("{}; {launch_cookie}", fixture.student_cookie),
            format!(
                "/api/attempts/{}/external-tool/launch/activity",
                QuestionAttemptId::from_uuid(id(899))
            ),
        ),
    ] {
        let response = fixture
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(target)
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("copied cookie request"),
            )
            .await
            .expect("copied cookie response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), 256 * 1024)
            .await
            .expect("copied error body");
        assert!(!String::from_utf8_lossy(&body).contains("activityReady"));
    }
    fixture
        .store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(40_001))
        .expect("advance clock");
    let expired = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&activity_path)
                .header(
                    "cookie",
                    format!("{}; {launch_cookie}", fixture.student_cookie),
                )
                .body(Body::empty())
                .expect("expired request"),
        )
        .await
        .expect("expired response");
    assert_eq!(expired.status(), StatusCode::NOT_FOUND);
    let expired_body = to_bytes(expired.into_body(), 256 * 1024)
        .await
        .expect("expired body");
    assert!(!String::from_utf8_lossy(&expired_body).contains("activityReady"));

    let created = fixture
        .backend
        .create_external_tool_launch(
            fixture.context,
            UserId::from_uuid(id(803)),
            ProblemVersionRef {
                problem: fixture.attempt.problem,
                version: fixture.attempt.question_version,
            },
            &fixture.question,
            &fixture.attempt,
            fixture.aead.as_ref(),
        )
        .await
        .expect("fresh launch session");
    let revoked_cookie = crate::imathas_backend::launch_cookie_value(
        fixture.aead.as_ref(),
        fixture.context,
        UserId::from_uuid(id(803)),
        fixture.attempt.id,
        &created,
    )
    .expect("launch cookie");
    learning_data_access::ExternalToolLaunchSessionStore::revoke_external_tool_launch_session(
        fixture.store.as_ref(),
        fixture.context,
        UserId::from_uuid(id(803)),
        fixture.attempt.id,
        created.id,
    )
    .await
    .expect("revoke launch session");
    let revoked = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&activity_path)
                .header(
                    "cookie",
                    format!(
                        "{}; ple_external_launch={revoked_cookie}",
                        fixture.student_cookie
                    ),
                )
                .body(Body::empty())
                .expect("revoked request"),
        )
        .await
        .expect("revoked response");
    assert_eq!(revoked.status(), StatusCode::NOT_FOUND);
    let revoked_body = to_bytes(revoked.into_body(), 256 * 1024)
        .await
        .expect("revoked body");
    assert!(!String::from_utf8_lossy(&revoked_body).contains("activityReady"));

    let mutated = contracted_route_fixture(RecordedContractedTransportMode::Available).await;
    let mutated_path = format!("/api/attempts/{}/external-tool/launch", mutated.attempt.id);
    let source_shell = mutated
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&mutated_path)
                .header("cookie", &mutated.student_cookie)
                .body(Body::empty())
                .expect("source shell request"),
        )
        .await
        .expect("source shell response");
    assert_eq!(source_shell.status(), StatusCode::OK);
    let source_cookie = source_shell.headers()["set-cookie"]
        .to_str()
        .expect("source cookie")
        .split(';')
        .next()
        .expect("source cookie pair")
        .to_owned();
    objects::ObjectStore::delete(mutated.objects.as_ref(), &mutated.source_key)
        .await
        .expect("remove source for mutation gate");
    let source_mutated = mutated
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("{mutated_path}/activity"))
                .header(
                    "cookie",
                    format!("{}; {source_cookie}", mutated.student_cookie),
                )
                .body(Body::empty())
                .expect("mutated source activity"),
        )
        .await
        .expect("mutated source response");
    // A missing immutable object is a local backend outage, but the
    // route refuses before restoring/proxying provider state.
    assert_eq!(source_mutated.status(), StatusCode::SERVICE_UNAVAILABLE);
    let source_mutated_body = to_bytes(source_mutated.into_body(), 256 * 1024)
        .await
        .expect("source mutation body");
    assert!(!String::from_utf8_lossy(&source_mutated_body).contains("activityReady"));
}

#[tokio::test]
async fn contracted_imathas_launch_outage_is_question_local_and_secret_free() {
    use adapter_imathas::test_support::RecordedContractedTransportMode;
    let fixture = contracted_route_fixture(RecordedContractedTransportMode::Unavailable).await;
    let response = fixture
        .app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/attempts/{}/external-tool/launch",
                    fixture.attempt.id
                ))
                .header("cookie", fixture.student_cookie)
                .body(Body::empty())
                .expect("outage request"),
        )
        .await
        .expect("outage response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("outage body");
    let body = std::str::from_utf8(&body).expect("outage utf8");
    assert!(!body.contains("activityReady"));
    for secret in [
        "institution-imathas",
        "recorded-proxy-session",
        "jwt",
        "source",
        "score",
        "answer",
    ] {
        assert!(
            !body.to_ascii_lowercase().contains(secret),
            "outage leaked {secret}"
        );
    }
}
