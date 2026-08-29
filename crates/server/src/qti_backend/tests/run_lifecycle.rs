use super::*;

#[tokio::test]
async fn published_qti_runs_grade_server_side_and_replay_without_a_second_private_lookup() {
    use learning_data_access::{DraftRecord, ProblemVersionRef};
    use question_model::generation::RandomizationDefinition;
    use question_model::run_policy::{
        AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
        TimingPolicy, VariationPolicy,
    };
    use question_model::{DraftQuestionDefinition, DraftQuestionSource};

    let tenant = TenantId::from_uuid(uuid::Uuid::from_u128(7_200));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(uuid::Uuid::from_u128(7_201));
    let student = UserId::from_uuid(uuid::Uuid::from_u128(7_202));
    let workspace = WorkspaceId::from_uuid(uuid::Uuid::from_u128(7_203));
    let import = WorkspaceImportId::from_uuid(uuid::Uuid::from_u128(7_204));
    let source_object = ObjectId::from_uuid(uuid::Uuid::from_u128(7_205));
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid::Uuid::from_u128(7_206)),
        version: VersionId::from_uuid(uuid::Uuid::from_u128(7_207)),
    };
    let (store, grader) = MemoryStore::with_qti_grader();
    let store = Arc::new(store);
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
        .expect("fixture clock");
    let grader_calls = Arc::new(AtomicUsize::new(0));
    let grader = Arc::new(CountingQtiGrader {
        inner: Arc::new(grader),
        calls: Arc::clone(&grader_calls),
    });
    let objects = Arc::new(MemoryObjectStore::default());
    let bytes = STANDARD.decode(PACKAGE.trim()).expect("QTI ZIP fixture");
    let parsed = adapter_qti::QtiImporter::default()
        .import(&bytes)
        .expect("QTI fixture parses");
    let imported = parsed.questions.first().expect("QTI item").clone();
    let ResponseDefinition::MultipleChoice { choices, .. } = &imported.response else {
        panic!("fixture is a choice item");
    };
    let correct = parsed
        .worker_correct_choice(&imported.item_id)
        .expect("private correct fixture choice");
    let wrong = choices
        .iter()
        .map(|choice| choice.id.clone())
        .find(|choice| choice != &correct)
        .expect("fixture has a wrong choice");
    objects
        .put(PutObject {
            key: ObjectKey::WorkspaceSource {
                tenant,
                workspace,
                import,
                object: source_object,
            },
            bytes,
            media_type: "application/zip".to_string(),
            license: "private-workspace-import".to_string(),
            provenance: "QTI run integration fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        })
        .await
        .expect("QTI source persists");
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Qti {
                item_id: imported.item_id.clone(),
                import_id: import,
            },
            prompt: imported.prompt.clone(),
            response: imported.response.clone(),
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "QTI run integration".to_string(),
                tags: vec![],
                taxonomy: vec![],
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("QTI draft before preparation");
    QtiImportHandler::new(Arc::clone(&store), Arc::clone(&objects))
        .prepare(
            context,
            JobPayload::QtiImport {
                workspace,
                import,
                source_object,
            },
            JobExecution::new(),
        )
        .await
        .expect("QTI import prepares");
    let job = store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::QtiImport {
                    workspace,
                    import,
                    source_object,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("QTI commit job");
    let claim = store
        .claim_next_job(
            &learning_data_access::JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("lease"),
        )
        .await
        .expect("claim")
        .expect("claimed QTI job");
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: learning_data_access::QtiImportRef {
                        tenant,
                        workspace,
                        import
                    },
                    source_object,
                },
            )
            .await
            .expect("QTI import commit"),
        CommitPreparedQtiImportOutcome::Committed
    );
    let preparer = QtiPublicationPreparer::new(Arc::clone(&store), Arc::clone(&objects));
    let validated = preparer
        .validate(context, &draft.question, import, &imported.item_id)
        .await
        .expect("exact QTI validation");
    let prepared = preparer
        .copy_candidates(&draft, reference, PublicationScope::Institution, validated)
        .await
        .expect("candidate copy");
    assert!(
        prepared.promotion.assets.iter().all(|asset| matches!(
            asset.object.key,
            ObjectKey::RestrictedProblemAsset { .. }
        ) && asset.object.bucket
            == objects::Bucket::PrivateContent),
        "institution QTI assets must be physically private before catalog promotion"
    );
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: prepared.published_source,
                source_artifact: Some(prepared.source_artifact),
                qti_promotion: Some(prepared.promotion),
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Institution,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: question_model::BackendCapabilities::from_iter([
                    question_model::Capability::ServerGrading,
                ]),
            },
        )
        .await
        .expect("QTI publish");
    let foreign = TenantContext::from_authenticated_session(TenantId::from_uuid(
        uuid::Uuid::from_u128(7_299),
    ));
    assert!(
        store
            .get_catalog_problem(foreign, reference)
            .await
            .expect("foreign catalog lookup")
            .is_none()
    );
    assert!(
        store
            .get_published_problem(reference.problem, reference.version)
            .await
            .expect("public catalog lookup")
            .is_none()
    );

    let course = CourseId::from_uuid(uuid::Uuid::from_u128(7_208));
    let assignment = AssignmentId::from_uuid(uuid::Uuid::from_u128(7_209));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "QTI course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: crate::test_fixtures::sysadmin_course_creation_authority(
                    store.as_ref(),
                    tenant,
                    course,
                    publisher,
                )
                .await,
            },
        )
        .await
        .expect("course");
    store
        .upsert_course_member(
            context,
            publisher,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "QTI learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student roster membership");
    store
        .create_assignment(
            context,
            learning_data_access::CreateAssignmentCommand {
                actor: publisher,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: question_model::AssignmentAudience::CourseWide,
                    title: "QTI assignment".to_string(),
                    lifecycle: question_model::AssignmentLifecycle::Draft,
                    instructions: question_model::AssignmentInstructions::default(),
                    items: vec![question_model::AssignmentItem {
                        id: question_model::AssignmentItemId::from_uuid(uuid::Uuid::from_u128(
                            7_210,
                        )),
                        reference,
                        position: 0,
                        points_possible: question_model::PointValue::from_whole(1),
                        delivery_state: question_model::AssignmentDeliveryState::Active,
                        scoring_mode: question_model::AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::StudentDisclosurePolicy::default(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AllCorrect,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
                base_policy: question_model::BaseAssignmentPolicy::default(),
            },
        )
        .await
        .expect("assignment");
    crate::course::tests::fixtures::publish_assignment(
        store.as_ref(),
        context,
        publisher,
        course,
        assignment,
        question_model::AssignmentTeachingSettings {
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            base_policy: question_model::BaseAssignmentPolicy::default(),
        },
    )
    .await;
    let cookie = issue_session(
        store.as_ref(),
        SessionSubject::new(
            tenant,
            student,
            "QTI learner",
            vec![question_model::UserRole::Student],
        )
        .expect("session subject"),
        SessionConfig::new(
            SessionLifetime::from_seconds(3600).expect("lifetime"),
            CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("session")
    .set_cookie
    .split(';')
    .next()
    .expect("cookie")
    .to_string();
    let backend = Arc::new(QtiBackend::new(
        Arc::clone(&store),
        Arc::clone(&grader),
        Arc::clone(&objects),
    ));
    let app = run_router(
        Arc::clone(&store),
        Arc::clone(&backend),
        Arc::new(
            learning_data_access::in_memory::MemorySealedPrivateExecutionStore::new(Arc::clone(
                &store,
            )),
        ),
        Arc::clone(&store) as Arc<dyn learning_data_access::StudentSubmissionStatusStore>,
        Arc::clone(&store) as Arc<dyn learning_data_access::AutomatedGradingStore>,
    );
    let run = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/runs"
                ))
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .expect("start request"),
        )
        .await
        .expect("start response");
    assert_eq!(run.status(), axum::http::StatusCode::CREATED);
    let run: AssignmentRun = serde_json::from_slice(
        &axum::body::to_bytes(run.into_body(), 64 * 1024)
            .await
            .expect("run body"),
    )
    .expect("run JSON");
    let attempt = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/runs/{}/attempts", run.id))
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .expect("attempt list"),
        )
        .await
        .expect("attempt list response");
    let attempt: QuestionAttempt = serde_json::from_value(
        serde_json::from_slice::<serde_json::Value>(
            &axum::body::to_bytes(attempt.into_body(), 64 * 1024)
                .await
                .expect("attempt body"),
        )
        .expect("attempt JSON")["items"][0]
            .clone(),
    )
    .expect("attempt projection");
    let envelope = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/attempts/{}/question",
                    attempt.id
                ))
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .expect("question request"),
        )
        .await
        .expect("question response");
    assert_eq!(envelope.status(), axum::http::StatusCode::OK);
    let envelope_json: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(envelope.into_body(), 64 * 1024)
            .await
            .expect("envelope body"),
    )
    .expect("envelope JSON");
    let public_choice_for = |durable: &ChoiceId| {
        let body = choices
            .iter()
            .find(|choice| &choice.id == durable)
            .expect("durable QTI choice exists")
            .body
            .clone();
        let expected_body = serde_json::to_value(&body).expect("QTI choice serializes");
        let identifier = envelope_json
            .pointer("/response/choices")
            .and_then(serde_json::Value::as_array)
            .and_then(|issued_choices| {
                issued_choices.iter().find_map(|choice| {
                    (choice.get("body") == Some(&expected_body))
                        .then(|| choice.get("id").and_then(serde_json::Value::as_str))
                        .flatten()
                })
            })
            .expect("issued public QTI choice exists");
        ChoiceId::new(identifier)
    };
    let rendered_wrong = public_choice_for(&wrong);
    assert_ne!(
        rendered_wrong, wrong,
        "the browser must submit the presentation-specific rendered choice ID"
    );
    for forbidden in [
        "answerKey",
        "correctResponse",
        "gradingPayload",
        "privateGrading",
    ] {
        assert!(
            !envelope_json.to_string().contains(forbidden),
            "envelope leaked {forbidden}"
        );
    }
    let submit = |attempt_id: &QuestionAttemptId, choice: &ChoiceId, key: &str| {
        axum::http::Request::builder()
            .method("POST")
            .uri(format!(
                "/api/courses/{course}/assignments/{assignment}/attempts/{attempt_id}/submissions"
            ))
            .header("cookie", &cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", key)
            .body(axum::body::Body::from(
                serde_json::json!({
                    "response": {
                        "kind": "multipleChoice",
                        "selected": [choice],
                    }
                })
                .to_string(),
            ))
            .expect("submit request")
    };
    let wrong_submission = app
        .clone()
        .oneshot(submit(&attempt.id, &rendered_wrong, "qti-wrong"))
        .await
        .expect("wrong response");
    assert_eq!(wrong_submission.status(), axum::http::StatusCode::ACCEPTED);
    let wrong_pending = serde_json::from_slice::<serde_json::Value>(
        &axum::body::to_bytes(wrong_submission.into_body(), 64 * 1024)
            .await
            .expect("pending JSON"),
    )
    .expect("pending response");
    assert_eq!(
        wrong_pending,
        serde_json::json!({
            "kind": "accepted_pending",
            "accepted": true,
            "attemptId": attempt.id,
            "automatedGradingStatus": "pending",
            "nextAction": "check_status",
        })
    );
    crate::test_fixtures::drain_one_accepted_submission(&store, Arc::clone(&backend)).await;
    let wrong_status = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/attempts/{}/submission-status",
                    attempt.id
                ))
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .expect("wrong status request"),
        )
        .await
        .expect("wrong status response");
    assert_eq!(wrong_status.status(), axum::http::StatusCode::OK);
    let wrong_json = axum::body::to_bytes(wrong_status.into_body(), 64 * 1024)
        .await
        .expect("wrong JSON");
    let wrong_text = String::from_utf8_lossy(&wrong_json);
    assert!(
        !wrong_text.contains(&format!("\"correct\":\"{}\"", correct.as_str()))
            && !wrong_text.contains("correct-choice")
            && !wrong_text.contains("answerKey")
            && !wrong_text.contains("gradingPayload"),
        "receipt leaked QTI private grading material"
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&wrong_json).expect("receipt")["feedback"]["correctness"],
        serde_json::Value::Null,
        "correctness remains withheld until the score is current"
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&wrong_json).expect("receipt")["scoringStatus"],
        "recalculating"
    );
    assert_eq!(
        grader_calls.load(Ordering::SeqCst),
        1,
        "accepted grading reuses the active attempt's sealed QTI evidence"
    );
    let resumed = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/runs"
                ))
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .expect("resume QTI run request"),
        )
        .await
        .expect("resume QTI run response");
    assert_eq!(resumed.status(), axum::http::StatusCode::CREATED);
    assert_eq!(
        grader_calls.load(Ordering::SeqCst),
        2,
        "server-owned resume issues the retry and seals its QTI grading evidence"
    );
    let replay = app
        .clone()
        .oneshot(submit(&attempt.id, &rendered_wrong, "qti-wrong"))
        .await
        .expect("replay response");
    assert_eq!(replay.status(), axum::http::StatusCode::OK);
    let replay_json: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(replay.into_body(), 64 * 1024)
            .await
            .expect("replay body"),
    )
    .expect("replay receipt");
    let wrong_receipt: serde_json::Value =
        serde_json::from_slice(&wrong_json).expect("wrong receipt");
    assert_eq!(replay_json["attempt"], wrong_receipt["attempt"]);
    assert_eq!(replay_json["feedback"], wrong_receipt["feedback"]);
    assert!(replay_json["nextIssued"].is_object());
    assert_eq!(
        grader_calls.load(Ordering::SeqCst),
        2,
        "replay must not regrade"
    );
    let next = store
        .list_question_attempts(
            context,
            run.id,
            learning_data_access::PageRequest::first(
                learning_data_access::PageSize::new(10).expect("page"),
            ),
        )
        .await
        .expect("attempts");
    let retry = next
        .items
        .into_iter()
        .find(|value| value.status == question_model::AttemptStatus::InProgress)
        .expect("retry after wrong response");
    let retry_question = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/attempts/{}/question",
                    retry.id
                ))
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .expect("retry question request"),
        )
        .await
        .expect("retry question response");
    assert_eq!(retry_question.status(), axum::http::StatusCode::OK);
    let retry_envelope: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(retry_question.into_body(), 64 * 1024)
            .await
            .expect("retry envelope body"),
    )
    .expect("retry public QTI envelope");
    let correct_body = choices
        .iter()
        .find(|choice| choice.id == correct)
        .expect("durable correct QTI choice exists")
        .body
        .clone();
    let expected_correct_body =
        serde_json::to_value(&correct_body).expect("QTI correct choice serializes");
    let rendered_correct = retry_envelope
        .pointer("/response/choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|retry_choices| {
            retry_choices.iter().find_map(|choice| {
                (choice.get("body") == Some(&expected_correct_body))
                    .then(|| choice.get("id").and_then(serde_json::Value::as_str))
                    .flatten()
            })
        })
        .map(ChoiceId::new)
        .expect("retry public correct QTI choice exists");
    assert_ne!(
        rendered_correct, correct,
        "the retry also receives a presentation-specific rendered choice ID"
    );
    let correct_submission = app
        .clone()
        .oneshot(submit(&retry.id, &rendered_correct, "qti-correct"))
        .await
        .expect("correct response");
    assert_eq!(
        correct_submission.status(),
        axum::http::StatusCode::ACCEPTED
    );
    let correct_pending = serde_json::from_slice::<serde_json::Value>(
        &axum::body::to_bytes(correct_submission.into_body(), 64 * 1024)
            .await
            .expect("correct pending body"),
    )
    .expect("correct pending response");
    assert_eq!(
        correct_pending,
        serde_json::json!({
            "kind": "accepted_pending",
            "accepted": true,
            "attemptId": retry.id,
            "automatedGradingStatus": "pending",
            "nextAction": "check_status",
        })
    );
    crate::test_fixtures::drain_one_accepted_submission(&store, Arc::clone(&backend)).await;
    let correct_status = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/attempts/{}/submission-status",
                    retry.id
                ))
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .expect("correct status request"),
        )
        .await
        .expect("correct status response");
    assert_eq!(correct_status.status(), axum::http::StatusCode::OK);
    let correct_json = serde_json::from_slice::<serde_json::Value>(
        &axum::body::to_bytes(correct_status.into_body(), 64 * 1024)
            .await
            .expect("correct body"),
    )
    .expect("correct receipt");
    assert_eq!(
        correct_json["feedback"]["correctness"],
        serde_json::Value::Null,
        "correctness remains withheld until the score is current"
    );
    assert_eq!(correct_json["scoringStatus"], "recalculating");
    assert_eq!(grader_calls.load(Ordering::SeqCst), 2);
}
