use super::*;

#[tokio::test]
async fn contracted_imathas_submission_retrieves_once_commits_and_replays_after_revoke() {
    use adapter_imathas::test_support::RecordedContractedTransportMode;

    let fixture = contracted_route_fixture(RecordedContractedTransportMode::Verified).await;
    let launch_path = format!("/api/attempts/{}/external-tool/launch", fixture.attempt.id);
    let launch = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&launch_path)
                .header("cookie", &fixture.student_cookie)
                .body(Body::empty())
                .expect("launch request"),
        )
        .await
        .expect("launch response");
    assert_eq!(launch.status(), StatusCode::OK);
    let launch_cookie = launch.headers()["set-cookie"]
        .to_str()
        .expect("launch cookie")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned();
    let submission_path = format!("{launch_path}/submission");
    let request = || {
        Request::builder()
            .method("POST")
            .uri(&submission_path)
            .header(
                "cookie",
                format!("{}; {launch_cookie}", fixture.student_cookie),
            )
            .header("idempotency-key", "recorded-contracted-submit")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"response":{"kind":"externalTool"}}"#))
            .expect("submission request")
    };
    let first = fixture
        .app
        .clone()
        .oneshot(request())
        .await
        .expect("first response");
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(first.headers()["cache-control"], "no-store");
    let first_body = to_bytes(first.into_body(), 256 * 1024)
        .await
        .expect("first body");
    assert!(
        std::str::from_utf8(&first_body)
            .expect("receipt UTF-8")
            .contains("\"accepted\":true")
    );
    assert_eq!(fixture.transport.result_calls(), 1);

    let replay = fixture
        .app
        .clone()
        .oneshot(request())
        .await
        .expect("replay response");
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_body = to_bytes(replay.into_body(), 256 * 1024)
        .await
        .expect("replay body");
    assert_eq!(replay_body, first_body);
    assert_eq!(fixture.transport.result_calls(), 1);
}

#[tokio::test]
async fn contracted_imathas_submission_refuses_missing_copied_and_malformed_markers_before_provider()
 {
    use adapter_imathas::test_support::RecordedContractedTransportMode;

    let fixture = contracted_route_fixture(RecordedContractedTransportMode::Verified).await;
    let launch_path = format!("/api/attempts/{}/external-tool/launch", fixture.attempt.id);
    let launch = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&launch_path)
                .header("cookie", &fixture.student_cookie)
                .body(Body::empty())
                .expect("launch"),
        )
        .await
        .expect("launch response");
    let launch_cookie = launch.headers()["set-cookie"]
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("pair")
        .to_owned();
    let path = format!("{launch_path}/submission");
    for (cookie, body) in [
        (
            fixture.student_cookie.clone(),
            r#"{"response":{"kind":"externalTool"}}"#,
        ),
        (
            format!("{}; {launch_cookie}", fixture.outsider_cookie),
            r#"{"response":{"kind":"externalTool"}}"#,
        ),
        (
            format!("{}; {launch_cookie}", fixture.student_cookie),
            r#"{"response":{"kind":"externalTool","score":1}}"#,
        ),
    ] {
        let response = fixture
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&path)
                    .header("cookie", cookie)
                    .header("idempotency-key", "refused-marker")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert!(
            matches!(
                response.status(),
                StatusCode::NOT_FOUND | StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
            ),
            "unexpected status: {}",
            response.status()
        );
    }
    assert_eq!(fixture.transport.result_calls(), 0);
}

#[tokio::test]
async fn archive_fence_refuses_external_tool_routes_before_provider_calls() {
    use adapter_imathas::test_support::RecordedContractedTransportMode;

    let fixture = contracted_route_fixture(RecordedContractedTransportMode::Verified).await;
    let calls_before = (
        fixture.transport.proxy_calls(),
        fixture.transport.result_calls(),
        fixture.route_backend.create_calls.load(Ordering::SeqCst),
        fixture.route_backend.proxy_calls.load(Ordering::SeqCst),
        fixture
            .route_backend
            .submission_calls
            .load(Ordering::SeqCst),
    );
    prepare_archive_fence(
        fixture.store.as_ref(),
        TenantId::from_uuid(id(801)),
        CourseId::from_uuid(id(809)),
    )
    .await;

    let launch_path = format!("/api/attempts/{}/external-tool/launch", fixture.attempt.id);
    let requests = vec![
        Request::builder()
            .uri(&launch_path)
            .header("cookie", &fixture.student_cookie)
            .body(Body::empty())
            .expect("archived shell request"),
        Request::builder()
            .uri(format!("{launch_path}/activity"))
            .header("cookie", &fixture.student_cookie)
            .body(Body::empty())
            .expect("archived activity GET"),
        Request::builder()
            .method("POST")
            .uri(format!("{launch_path}/activity"))
            .header("cookie", &fixture.student_cookie)
            .body(Body::empty())
            .expect("archived activity POST"),
        Request::builder()
            .method("POST")
            .uri(format!("{launch_path}/submission"))
            .header("cookie", &fixture.student_cookie)
            .header("idempotency-key", "archived-external")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"response":{"kind":"externalTool"}}"#))
            .expect("archived external submission"),
    ];
    for request in requests {
        let response = fixture
            .app
            .clone()
            .oneshot(request)
            .await
            .expect("archived external response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    assert_eq!(
        (
            fixture.transport.proxy_calls(),
            fixture.transport.result_calls(),
            fixture.route_backend.create_calls.load(Ordering::SeqCst),
            fixture.route_backend.proxy_calls.load(Ordering::SeqCst),
            fixture
                .route_backend
                .submission_calls
                .load(Ordering::SeqCst),
        ),
        calls_before
    );
}

#[tokio::test]
async fn contracted_imathas_result_outage_stays_ungraded_and_replica_does_not_reretrieve() {
    use adapter_imathas::test_support::RecordedContractedTransportMode;

    let fixture =
        contracted_route_fixture(RecordedContractedTransportMode::ResultUnavailable).await;
    let actor = UserId::from_uuid(id(803));
    let reference = ProblemVersionRef {
        problem: fixture.attempt.problem,
        version: fixture.attempt.question_version,
    };
    let created = fixture
        .backend
        .create_contracted_launch_session(
            fixture.context,
            actor,
            reference,
            &fixture.question,
            &fixture.attempt,
            fixture.aead.as_ref(),
        )
        .await
        .expect("protected launch");
    let proof = learning_data_access::ExternalToolLaunchProof {
        session_id: created.id,
        token: created.token,
    };
    let key = learning_data_access::SubmissionIdempotencyKey::parse("recorded-contracted-outage")
        .expect("idempotency key");
    let first = fixture
        .backend
        .submit_external_tool(
            fixture.context,
            actor,
            reference,
            &fixture.question,
            &fixture.attempt,
            key.clone(),
            proof.clone(),
            fixture.aead.as_ref(),
        )
        .await;
    assert!(matches!(first, Err(RunBackendError::Unavailable(_))));
    assert_eq!(fixture.transport.result_calls(), 1);
    assert!(fixture.attempt.result.is_none());

    let replica = fixture
        .backend
        .submit_external_tool(
            fixture.context,
            actor,
            reference,
            &fixture.question,
            &fixture.attempt,
            key,
            proof,
            fixture.aead.as_ref(),
        )
        .await;
    assert!(matches!(replica, Err(RunBackendError::Unavailable(_))));
    assert_eq!(fixture.transport.result_calls(), 1);
}

#[tokio::test]
async fn contracted_imathas_verified_pending_recovers_without_a_second_retrieval() {
    use adapter_imathas::test_support::RecordedContractedTransportMode;
    use adapter_imathas::{CorrelationIssuer, GradeBinding};
    use learning_data_access::{
        BeginExternalToolGradeCommand, ExternalToolBegin, ExternalToolBrokerStore,
        PersistedCorrelation, StageExternalToolVerificationCommand,
    };
    use objects::Sha256Digest;

    let fixture = contracted_route_fixture(RecordedContractedTransportMode::Verified).await;
    let actor = UserId::from_uuid(id(803));
    let reference = ProblemVersionRef {
        problem: fixture.attempt.problem,
        version: fixture.attempt.question_version,
    };
    let QuestionSource::Imathas {
        provider,
        snapshot,
        snapshot_sha256,
        integration_profile,
        ..
    } = &fixture.question.source
    else {
        panic!("contracted fixture source")
    };
    let response = StudentResponse::ExternalTool {};
    let binding = learning_data_access::ExternalToolBinding {
        provider: provider.clone(),
        problem: fixture.question.problem,
        version: fixture.question.version,
        seed: fixture.attempt.seed,
        source_object: *snapshot,
        source_sha256: snapshot_sha256.clone(),
        integration_profile: integration_profile.clone(),
        response_sha256: Sha256Digest::compute(&serde_json::to_vec(&response).expect("response")),
    };
    let grade_binding = GradeBinding {
        tenant: fixture.context.tenant_id(),
        attempt: fixture.attempt.id,
        problem: fixture.question.problem,
        version: fixture.question.version,
        seed: Seed::new(fixture.attempt.seed),
    };
    let issuer = CorrelationIssuer::from_server_secret([83; 32]);
    let correlation =
        PersistedCorrelation::new(issuer.begin(grade_binding).to_storage_value().into_bytes())
            .expect("correlation");
    let key = learning_data_access::SubmissionIdempotencyKey::parse("recorded-contracted-pending")
        .expect("key");
    let ExternalToolBegin::Lease(lease) = fixture
        .store
        .begin_or_resume_external_grade(
            fixture.context,
            BeginExternalToolGradeCommand {
                actor,
                attempt: fixture.attempt.id,
                response: response.clone(),
                idempotency_key: key.clone(),
                binding: binding.clone(),
                proposed_correlation: correlation,
                lease_millis: 30_000,
            },
        )
        .await
        .expect("lease")
    else {
        panic!("fresh broker lease")
    };
    fixture
        .store
        .stage_external_tool_verification(
            fixture.context,
            StageExternalToolVerificationCommand {
                actor,
                attempt: fixture.attempt.id,
                response,
                idempotency_key: key.clone(),
                binding,
                correlation: lease.correlation,
                lease_token: lease.token,
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
            },
        )
        .await
        .expect("stage pending");
    let created = fixture
        .backend
        .create_contracted_launch_session(
            fixture.context,
            actor,
            reference,
            &fixture.question,
            &fixture.attempt,
            fixture.aead.as_ref(),
        )
        .await
        .expect("launch proof");
    let recovered = fixture
        .backend
        .submit_external_tool(
            fixture.context,
            actor,
            reference,
            &fixture.question,
            &fixture.attempt,
            key,
            learning_data_access::ExternalToolLaunchProof {
                session_id: created.id,
                token: created.token,
            },
            fixture.aead.as_ref(),
        )
        .await
        .expect("commit staged grade");
    assert!(matches!(recovered, SubmissionDisposition::Committed(_)));
    assert_eq!(fixture.transport.result_calls(), 0);
}
