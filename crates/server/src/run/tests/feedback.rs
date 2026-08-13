use super::*;

#[tokio::test]
async fn native_feedback_http_policy_matrix_is_allowlisted_and_replay_safe() {
    for policy in [
        FeedbackDisclosure::ImmediateCorrectness,
        FeedbackDisclosure::ImmediateFull,
        FeedbackDisclosure::Deferred,
        FeedbackDisclosure::OnRelease,
    ] {
        let (store, backend, app, student_cookie, _outsider_cookie, assignment) =
            native_feedback_fixture(policy).await;
        let first = active_attempt_for(&app, assignment, &student_cookie).await;
        let ester = presented_choice_id(&app, first.id, &student_cookie, 0).await;
        let submit = |attempt: QuestionAttemptId, key: &str, choice: &str| {
            Request::builder()
                .method("POST")
                .uri(format!("/api/submissions/{attempt}"))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", key)
                .body(Body::from(
                    serde_json::json!({
                        "response": { "kind": "multipleChoice", "selected": [choice] }
                    })
                    .to_string(),
                ))
                .expect("native submission")
        };
        let first_response = app
            .clone()
            .oneshot(submit(first.id, "native-feedback-first", &ester))
            .await
            .expect("first submission");
        assert_eq!(first_response.status(), StatusCode::OK);
        let first_receipt = json(first_response).await;
        let first_raw = first_receipt.to_string();
        for forbidden in [
            "answerKey",
            "expected",
            "checker",
            "provider",
            "solution",
            "feedbackContent",
        ] {
            assert!(
                !first_raw.contains(forbidden),
                "{policy:?} receipt leaked {forbidden}"
            );
        }
        assert_eq!(backend.submissions.load(Ordering::SeqCst), 1);
        match policy {
            FeedbackDisclosure::ImmediateCorrectness => {
                assert_eq!(first_receipt["attempt"]["result"], serde_json::Value::Null);
                assert_eq!(first_receipt["feedback"]["correctness"], false);
                assert!(first_receipt["feedback"].get("hint").is_some());
                for prohibited in [
                    "pointsEarned",
                    "pointsPossible",
                    "correctResponse",
                    "rationale",
                ] {
                    assert!(first_receipt["feedback"].get(prohibited).is_none());
                }
            }
            FeedbackDisclosure::ImmediateFull => {
                assert_eq!(first_receipt["feedback"]["correctness"], false);
                assert_eq!(first_receipt["feedback"]["pointsEarned"], 0.0);
                assert_eq!(first_receipt["feedback"]["pointsPossible"], 2.0);
                assert!(first_receipt["feedback"]["hint"].is_array());
                assert_eq!(
                    first_receipt["feedback"]["correctResponse"][0]["markdown"],
                    "The peptide linkage"
                );
                assert!(
                    first_receipt["feedback"]["rationale"][0]["markdown"]
                        .as_str()
                        .is_some_and(|text| text.contains("resonance") && text.contains("planar"))
                );
            }
            FeedbackDisclosure::Deferred | FeedbackDisclosure::OnRelease => {
                assert_eq!(first_receipt["feedback"], serde_json::Value::Null);
                assert_eq!(first_receipt["attempt"]["result"], serde_json::Value::Null);
            }
        }
        if !matches!(policy, FeedbackDisclosure::Deferred) {
            let replay = app
                .clone()
                .oneshot(submit(first.id, "native-feedback-first", &ester))
                .await
                .expect("idempotent replay");
            assert_eq!(replay.status(), StatusCode::OK);
            assert_eq!(json(replay).await, first_receipt);
            assert_eq!(backend.submissions.load(Ordering::SeqCst), 1);
        }

        let foreign_cookie = issued_cookie_for(
            store.as_ref(),
            TenantId::from_uuid(id(299)),
            UserId::from_uuid(id(298)),
            "Foreign",
        )
        .await;
        let foreign = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/submissions/{}", first.id))
                    .header("cookie", foreign_cookie)
                    .header("content-type", "application/json")
                    .header("idempotency-key", "foreign-feedback")
                    .body(Body::from(
                        serde_json::json!({
                            "response": { "kind": "multipleChoice", "selected": [ester] }
                        })
                        .to_string(),
                    ))
                    .expect("foreign submission"),
            )
            .await
            .expect("foreign response");
        assert_eq!(foreign.status(), StatusCode::NOT_FOUND);

        let next = next_active_attempt(&app, first.run, &student_cookie).await;
        let amide = presented_choice_id(&app, next.id, &student_cookie, 1).await;
        let completion = app
            .clone()
            .oneshot(submit(next.id, "native-feedback-complete", &amide))
            .await
            .expect("completion submission");
        assert_eq!(completion.status(), StatusCode::OK);
        assert_eq!(backend.submissions.load(Ordering::SeqCst), 2);

        if matches!(policy, FeedbackDisclosure::Deferred) {
            let stored = store
                .replay_submission(
                    TenantContext::from_authenticated_session(TenantId::from_uuid(id(201))),
                    UserId::from_uuid(id(203)),
                    first.id,
                    &StudentResponse::MultipleChoice {
                        selected: vec![ChoiceId::new(ester.clone())],
                    },
                    &SubmissionIdempotencyKey::parse("native-feedback-first")
                        .expect("valid replay key"),
                )
                .await
                .expect("direct stored replay")
                .expect("first receipt");
            assert!(stored.run.completed_at.is_none());
        }

        let replay = app
            .clone()
            .oneshot(submit(first.id, "native-feedback-first", &ester))
            .await
            .expect("post-completion replay");
        assert_eq!(replay.status(), StatusCode::OK);
        let replay_receipt = json(replay).await;
        if matches!(policy, FeedbackDisclosure::OnRelease) {
            assert_eq!(replay_receipt["feedback"], serde_json::Value::Null);
        } else {
            assert_eq!(replay_receipt, first_receipt);
        }
        assert_eq!(backend.submissions.load(Ordering::SeqCst), 2);
    }
}

#[tokio::test]
async fn run_summary_projects_current_disclosure_and_release_without_rewriting_receipts() {
    for policy in [
        FeedbackDisclosure::ImmediateCorrectness,
        FeedbackDisclosure::ImmediateFull,
        FeedbackDisclosure::Deferred,
        FeedbackDisclosure::OnRelease,
    ] {
        let (store, _backend, app, student_cookie, outsider_cookie, assignment) =
            native_feedback_fixture(policy).await;
        let instructor_cookie = issued_cookie_for(
            store.as_ref(),
            TenantId::from_uuid(id(201)),
            UserId::from_uuid(id(202)),
            "Instructor",
        )
        .await;
        let first = active_attempt_for(&app, assignment, &student_cookie).await;
        let ester = presented_choice_id(&app, first.id, &student_cookie, 0).await;
        let submit = |attempt: QuestionAttemptId, key: &str, choice: &str| {
            Request::builder()
                .method("POST")
                .uri(format!("/api/submissions/{attempt}"))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", key)
                .body(Body::from(
                    serde_json::json!({
                        "response": { "kind": "multipleChoice", "selected": [choice] }
                    })
                    .to_string(),
                ))
                .expect("submission request")
        };
        let first_receipt = json(
            app.clone()
                .oneshot(submit(first.id, "summary-first", &ester))
                .await
                .expect("first submission"),
        )
        .await;

        let summary_path = format!("/api/runs/{}/summary?pageSize=1", first.run);
        let before = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&summary_path)
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("summary request"),
            )
            .await
            .expect("summary response");
        assert_eq!(before.status(), StatusCode::OK);
        assert_eq!(before.headers()["cache-control"], "no-store");
        let before = json(before).await;
        assert_eq!(
            before["course"]["summary"]["id"],
            CourseId::from_uuid(id(205)).to_string()
        );
        assert_eq!(before["course"]["summary"]["role"], "student");
        assert_eq!(before["course"]["appearance"]["theme"], "grass");
        assert_eq!(before["course"]["appearance"]["revision"], "1");
        assert!(before["course"]["appearance"]["banner"].is_null());
        let instructor_summary = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&summary_path)
                    .header("cookie", &instructor_cookie)
                    .body(Body::empty())
                    .expect("instructor summary request"),
            )
            .await
            .expect("instructor summary response");
        assert_eq!(instructor_summary.status(), StatusCode::OK);
        assert_eq!(instructor_summary.headers()["cache-control"], "no-store");
        let instructor_summary = json(instructor_summary).await;
        assert_eq!(instructor_summary["run"]["id"], first.run.to_string());
        assert_eq!(
            instructor_summary["course"]["summary"]["role"],
            "instructor"
        );
        assert_eq!(
            before["outcomes"]["items"].as_array().map(Vec::len),
            Some(1)
        );
        let feedback_before = &before["outcomes"]["items"][0]["feedback"];
        match policy {
            FeedbackDisclosure::ImmediateCorrectness | FeedbackDisclosure::ImmediateFull => {
                assert!(feedback_before.is_object());
            }
            FeedbackDisclosure::Deferred | FeedbackDisclosure::OnRelease => {
                assert_eq!(feedback_before, &serde_json::Value::Null);
            }
        }
        let raw_before = before.to_string();
        for forbidden in [
            "answerKey",
            "checker",
            "provider",
            "provenance",
            "source",
            "launchUrl",
            "feedbackContent",
        ] {
            assert!(
                !raw_before.contains(forbidden),
                "run summary leaked {forbidden}"
            );
        }
        let cursor = before["outcomes"]["nextCursor"]
            .as_str()
            .expect("bounded page cursor");
        let continuation = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/runs/{}/summary?pageSize=1&cursor={cursor}",
                        first.run
                    ))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("summary continuation"),
            )
            .await
            .expect("summary continuation response");
        assert_eq!(continuation.status(), StatusCode::OK);

        let student_release = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/attempts/{}/feedback-release", first.id))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student release request"),
            )
            .await
            .expect("student release response");
        assert_eq!(student_release.status(), StatusCode::NOT_FOUND);
        let foreign_summary = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&summary_path)
                    .header("cookie", &outsider_cookie)
                    .body(Body::empty())
                    .expect("outsider summary request"),
            )
            .await
            .expect("outsider summary response");
        assert_eq!(foreign_summary.status(), StatusCode::NOT_FOUND);
        let foreign_tenant_cookie = issued_cookie_for(
            store.as_ref(),
            TenantId::from_uuid(id(299)),
            UserId::from_uuid(id(298)),
            "Foreign tenant",
        )
        .await;
        let foreign_tenant_summary = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&summary_path)
                    .header("cookie", &foreign_tenant_cookie)
                    .body(Body::empty())
                    .expect("foreign-tenant summary request"),
            )
            .await
            .expect("foreign-tenant summary response");
        assert_eq!(foreign_tenant_summary.status(), StatusCode::NOT_FOUND);

        let next = next_active_attempt(&app, first.run, &student_cookie).await;
        let amide = presented_choice_id(&app, next.id, &student_cookie, 1).await;
        let completed = app
            .clone()
            .oneshot(submit(next.id, "summary-complete", &amide))
            .await
            .expect("completion submission");
        assert_eq!(completed.status(), StatusCode::OK);

        if matches!(policy, FeedbackDisclosure::OnRelease) {
            let release = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/attempts/{}/feedback-release", first.id))
                        .header("cookie", &instructor_cookie)
                        .body(Body::empty())
                        .expect("instructor release request"),
                )
                .await
                .expect("instructor release response");
            assert_eq!(release.status(), StatusCode::OK);
            assert_eq!(release.headers()["cache-control"], "no-store");
            assert_eq!(json(release).await, serde_json::json!({ "released": true }));

            let repeated_release = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/attempts/{}/feedback-release", first.id))
                        .header("cookie", &instructor_cookie)
                        .body(Body::empty())
                        .expect("repeated instructor release request"),
                )
                .await
                .expect("repeated instructor release response");
            assert_eq!(repeated_release.status(), StatusCode::OK);
            assert_eq!(repeated_release.headers()["cache-control"], "no-store");
            assert_eq!(
                json(repeated_release).await,
                serde_json::json!({ "released": true })
            );
        }

        let after = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/runs/{}/summary", first.run))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("completed summary request"),
            )
            .await
            .expect("completed summary response");
        assert_eq!(after.status(), StatusCode::OK);
        let after = json(after).await;
        assert_eq!(after["practiceAllowed"], true);
        let first_feedback = &after["outcomes"]["items"][0]["feedback"];
        match policy {
            FeedbackDisclosure::OnRelease => {
                assert!(first_feedback.get("correctResponse").is_some());
                let replay = json(
                    app.clone()
                        .oneshot(submit(first.id, "summary-first", &ester))
                        .await
                        .expect("receipt replay"),
                )
                .await;
                assert_eq!(
                    replay, first_receipt,
                    "release must not rewrite the receipt"
                );
            }
            FeedbackDisclosure::Deferred => {
                assert!(first_feedback.get("correctResponse").is_some())
            }
            FeedbackDisclosure::ImmediateCorrectness => {
                assert!(first_feedback.get("correctResponse").is_none())
            }
            FeedbackDisclosure::ImmediateFull => {
                assert!(first_feedback.get("correctResponse").is_some())
            }
        }
    }
}
