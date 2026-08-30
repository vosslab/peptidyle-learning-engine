use super::*;

pub(crate) async fn exercise_run_api_receipts<S>(
    store: &S,
    disclosure_policy: StudentDisclosurePolicy,
    fixture_offset: u128,
) -> RunApiFixture
where
    S: Store
        + CatalogStore
        + CourseRosterStore
        + JobStore
        + AssignmentScoringWorkerStore
        + SessionStore,
{
    exercise_run_api_receipts_with_grade_policy(
        store,
        disclosure_policy,
        fixture_offset,
        GradePolicy::Highest,
    )
    .await
}

pub(crate) async fn exercise_run_api_receipts_with_grade_policy<S>(
    store: &S,
    disclosure_policy: StudentDisclosurePolicy,
    fixture_offset: u128,
    grade_policy: GradePolicy,
) -> RunApiFixture
where
    S: Store
        + CatalogStore
        + CourseRosterStore
        + JobStore
        + AssignmentScoringWorkerStore
        + SessionStore,
{
    let tenant = TenantId::from_uuid(uuid(401 + fixture_offset));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(uuid(402));
    let student_user = UserId::from_uuid(uuid(403));
    let unrelated_user = UserId::from_uuid(uuid(10_403 + fixture_offset));
    let workspace = WorkspaceId::from_uuid(uuid(404));
    let problem = ProblemId::from_uuid(uuid(405 + fixture_offset));
    let version = VersionId::from_uuid(uuid(406 + fixture_offset));
    let course = CourseId::from_uuid(uuid(407));
    let course_creation_authority =
        sysadmin_course_creation_authority(store, tenant, course, publisher).await;
    let assignment = AssignmentId::from_uuid(uuid(408));
    let first_run = RunId::from_uuid(uuid(410));
    let ignored_resume_id = RunId::from_uuid(uuid(411));
    let attempt_id = QuestionAttemptId::from_uuid(uuid(412));
    let (attempt_presentation_binding, attempt_presentation) =
        receipt_presentation(version, 991, 7);

    let run_question = draft_question(workspace);
    let issued_question_snapshot = learning_data_access::IssuedQuestionSnapshotV1::new(
        QuestionDefinition::from_draft(run_question.clone(), problem, version, published_source()),
        learning_data_access::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("construct exact native issued-question snapshot");
    let draft = DraftRecord {
        tenant,
        question: run_question,
        derived_from: None,
    };
    let saved_draft = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("run fixture draft");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved_draft.revision,
                publication: ProblemVersionRef { problem, version },
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                byline: reviewed_byline(),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("run fixture publication");
    store
        .create_course(
            context,
            learning_data_access::CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Run API biochemistry".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: course_creation_authority,
            },
        )
        .await
        .expect("run fixture course");
    store
        .upsert_course_member(
            context,
            publisher,
            learning_data_access::UpsertCourseMember {
                course,
                user: student_user,
                display_name: "Run learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("run fixture learner membership");
    store
        .create_assignment_with_default_policy(
            context,
            publisher,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Run API assignment".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Published,
                instructions: question_model::AssignmentInstructions::default(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: fixed_items(vec![
                    ProblemVersionRef { problem, version },
                    ProblemVersionRef { problem, version },
                ]),
                selection_groups: Vec::new(),
                disclosure_policy,
                policies: RunPolicies {
                    grade: grade_policy,
                    ..policies()
                },
            },
        )
        .await
        .expect("run fixture assignment");
    let run = store
        .start_or_resume_run(
            context,
            student_user,
            StudentWorkRoutingBinding::new(course, assignment),
            first_run,
        )
        .await
        .expect("first run should start");
    let resumed = store
        .start_or_resume_run(
            context,
            student_user,
            StudentWorkRoutingBinding::new(course, assignment),
            ignored_resume_id,
        )
        .await
        .expect("active run should resume");
    assert_eq!(resumed, run);

    let issue = IssueQuestionAttemptCommand {
        actor: student_user,
        binding: StudentWorkRoutingBinding::new(course, assignment),
        attempt: attempt_id,
        run: run.id,
        assignment_position: 0,
        problem,
        question_version: version,
        issued_question_snapshot: issued_question_snapshot.clone(),
        seed: 991,
        presentation_capability: PresentationCapability::EnvelopeV1,
        presentation: Some(attempt_presentation_binding),
        presentation_snapshot: Some(attempt_presentation.clone()),
        grading_envelope: Some(grading_envelope(version, 991)),
        native_execution_envelope_capability: NativeExecutionEnvelopeCapability::Required,
        flat_grading: None,
        flat_grading_capability: FlatGradingCapability::NotApplicable,
        webwork_grading: None,
        webwork_grading_capability: learning_data_access::WebworkGradingCapability::NotApplicable,
        qti_grading: None,
        qti_grading_capability: QtiGradingCapability::NotApplicable,
        parameter_hash: "parameter-hash".to_string(),
        provenance: AttemptProvenance {
            adapter: implementation("native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("numeric"),
            rendered_question_sha256: "rendered-hash".to_string(),
        },
        webwork_replay: None,
        prefetched: None,
        predecessor_submission: None,
    };
    for binding in [
        StudentWorkRoutingBinding::new(CourseId::from_uuid(uuid(40_407)), assignment),
        StudentWorkRoutingBinding::new(course, AssignmentId::from_uuid(uuid(40_408))),
    ] {
        assert!(matches!(
            store
                .issue_or_resume_question_attempt(
                    context,
                    IssueQuestionAttemptCommand {
                        binding,
                        ..issue.clone()
                    },
                )
                .await,
            Err(StoreError::NotFound)
        ));
    }
    let attempt = store
        .issue_or_resume_question_attempt(context, issue.clone())
        .await
        .expect("question should issue");
    let resumed_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                attempt: QuestionAttemptId::from_uuid(uuid(414)),
                seed: 992,
                ..issue
            },
        )
        .await
        .expect("unanswered question should resume");
    assert_eq!(resumed_attempt, attempt);

    let before_submit = store
        .get_run_summary_page(
            context,
            student_user,
            run.id,
            PageRequest::first(PageSize::new(10).expect("valid bounded page")),
        )
        .await
        .expect("summary before submission");
    let before_submit_decision = before_submit.outcomes.items[0].disclosure.decision();
    assert_eq!(
        before_submit_decision.score,
        disclosure_policy.score == StudentDisclosureTiming::DuringAttempt,
        "an unsubmitted attempt only exposes a during-attempt score"
    );
    assert_eq!(
        before_submit_decision.per_item_correctness,
        disclosure_policy.per_item_correctness == StudentDisclosureTiming::DuringAttempt,
        "per-item correctness is independently evaluated before submit"
    );
    assert_eq!(
        before_submit_decision.feedback_text,
        disclosure_policy.feedback_text == StudentDisclosureTiming::DuringAttempt,
        "feedback text is assignment-owned rather than question-owned"
    );
    assert_eq!(
        before_submit_decision.solution,
        disclosure_policy.solution == StudentDisclosureTiming::DuringAttempt,
        "solutions remain independently withheld before submit"
    );
    assert!(!before_submit_decision.class_statistics);

    let blocked_second_position = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                binding: StudentWorkRoutingBinding::new(course, assignment),
                attempt: QuestionAttemptId::from_uuid(uuid(415)),
                run: run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                issued_question_snapshot: issued_question_snapshot.clone(),
                seed: 993,
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                native_execution_envelope_capability:
                    NativeExecutionEnvelopeCapability::NotApplicable,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                parameter_hash: "second-parameter-hash".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("numeric"),
                    rendered_question_sha256: "second-rendered-hash".to_string(),
                },
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await;
    assert!(matches!(
        blocked_second_position,
        Err(StoreError::InvalidRecord(message))
            if message == "another question attempt is already active in this run"
    ));

    let (reservation_presentation_binding, second_presentation) =
        receipt_presentation(version, 993, 9);
    let reservation = PrefetchedQuestionDescriptorV1 {
        run: run.id,
        predecessor: attempt.id,
        assignment_position: 1,
        problem,
        question_version: version,
        issued_question_snapshot: issued_question_snapshot.clone(),
        seed: 993,
        presentation_capability: PresentationCapability::EnvelopeV1,
        presentation: reservation_presentation_binding,
        presentation_snapshot: second_presentation.clone(),
        grading_envelope: grading_envelope(version, 993),
        native_execution_envelope_capability: NativeExecutionEnvelopeCapability::Required,
        flat_grading_capability: FlatGradingCapability::NotApplicable,
        webwork_grading_capability: learning_data_access::WebworkGradingCapability::NotApplicable,
        qti_grading_capability: QtiGradingCapability::NotApplicable,
        parameter_hash: "prefetched-parameter-hash".to_string(),
        provenance: AttemptProvenance {
            adapter: implementation("native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("numeric"),
            rendered_question_sha256: "prefetched-rendered-hash".to_string(),
        },
    };
    let reservation_private_execution = PrefetchedPrivateExecutionV1 {
        flat_grading: None,
        webwork_replay: None,
        webwork_grading: None,
        qti_grading: None,
    };
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: student_user,
                    binding: StudentWorkRoutingBinding::new(course, assignment),
                    reservation: reservation.clone(),
                    private_execution: reservation_private_execution.clone(),
                },
            )
            .await,
        Ok(reservation.clone()),
        "prefetch reserves immutable next-question inputs only",
    );
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: student_user,
                    binding: StudentWorkRoutingBinding::new(course, assignment),
                    reservation: reservation.clone(),
                    private_execution: reservation_private_execution.clone(),
                },
            )
            .await,
        Ok(reservation.clone()),
        "an identical prefetch retry is idempotent",
    );
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: student_user,
                    binding: StudentWorkRoutingBinding::new(course, assignment),
                    reservation: PrefetchedQuestionDescriptorV1 {
                        seed: reservation.seed + 1,
                        ..reservation.clone()
                    },
                    private_execution: reservation_private_execution.clone(),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a conflicting prefetch retry cannot rewrite its immutable variation",
    );
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: unrelated_user,
                    binding: StudentWorkRoutingBinding::new(course, assignment),
                    reservation: reservation.clone(),
                    private_execution: reservation_private_execution.clone(),
                },
            )
            .await,
        Err(StoreError::NotFound),
        "a different member has no learner entitlement to reserve a student's next question",
    );
    assert_eq!(
        store
            .list_question_attempts(
                context,
                run.id,
                PageRequest::first(PageSize::new(10).expect("valid page size")),
            )
            .await
            .expect("reservation leaves the attempt list readable")
            .items,
        vec![attempt.clone()],
        "reservation neither creates an attempt nor starts a timer",
    );

    let response = StudentResponse::Numeric { value: 18.0 };
    let key = SubmissionIdempotencyKey::parse("submission-401").expect("valid key");
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &key)
            .await,
        Ok(learning_data_access::SubmissionReceiptRead::Missing)
    );
    let invalid_result = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                binding: StudentWorkRoutingBinding::new(course, assignment),
                attempt: attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: false,
                    points_earned: 1_001.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: key.clone(),
            },
        )
        .await;
    assert!(matches!(invalid_result, Err(StoreError::InvalidRecord(_))));
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &key)
            .await,
        Ok(learning_data_access::SubmissionReceiptRead::Missing),
        "a rejected backend result must leave the attempt unsubmitted"
    );
    let hostile_feedback = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                binding: StudentWorkRoutingBinding::new(course, assignment),
                attempt: attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent {
                    hint: Some(vec![ContentBlock::Table {
                        headers: vec!["residue".to_string(), "charge".to_string()],
                        rows: vec![vec!["Lys".to_string()]],
                        description: "malformed structural feedback fixture".to_string(),
                    }]),
                    correct_response: None,
                    rationale: None,
                },
                idempotency_key: key.clone(),
            },
        )
        .await;
    assert!(matches!(
        hostile_feedback,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &key)
            .await,
        Ok(learning_data_access::SubmissionReceiptRead::Missing),
        "rejected feedback must not leave a submission, feedback, or summary partial write"
    );
    let submitted = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                binding: StudentWorkRoutingBinding::new(course, assignment),
                attempt: attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent {
                    hint: Some(vec![ContentBlock::Text {
                        markdown: "Check the units.".to_string(),
                    }]),
                    correct_response: None,
                    rationale: Some(vec![ContentBlock::Text {
                        markdown: "The recorded calculation is dimensionally consistent."
                            .to_string(),
                    }]),
                },
                idempotency_key: key.clone(),
            },
        )
        .await
        .expect("first response should commit");
    let submitted_debug = format!("{submitted:?}");
    for sensitive in [
        "18.0",
        "points_earned",
        "points_possible",
        "Check the units.",
    ] {
        assert!(
            !submitted_debug.contains(sensitive),
            "submission record Debug must redact `{sensitive}`: {submitted_debug}",
        );
    }
    let replay = store
        .replay_submission(context, student_user, attempt.id, &response, &key)
        .await
        .expect("replay lookup");
    let learning_data_access::SubmissionReceiptRead::Completed(replay) = replay else {
        panic!("first receipt should replay");
    };
    assert_eq!(replay.attempt, submitted.attempt);
    assert!(replay.feedback == submitted.feedback);
    let receipt_read = store
        .submission_record(context, student_user, attempt.id)
        .await
        .expect("owned receipt read");
    assert_eq!(
        receipt_read,
        learning_data_access::SubmissionReceiptRead::Completed(Box::new(submitted.clone())),
        "receipt reads return the immutable committed record without retry credentials"
    );
    assert_eq!(
        store
            .submission_record(context, unrelated_user, attempt.id)
            .await,
        Err(StoreError::NotFound),
        "another course member cannot use a receipt read as an attempt-existence oracle"
    );
    assert_eq!(
        replay.feedback.content().hint,
        Some(vec![ContentBlock::Text {
            markdown: "Check the units.".to_string(),
        }]),
        "an exact replay returns the stored private feedback rather than regrading"
    );
    let before_completion = store
        .get_run_summary_page(
            context,
            student_user,
            run.id,
            PageRequest::first(PageSize::new(10).expect("valid bounded page")),
        )
        .await
        .expect("summary before completion");
    assert_eq!(before_completion.run.completed_at, None);
    assert_eq!(before_completion.outcomes.items.len(), 1);
    let submitted_decision = before_completion.outcomes.items[0].disclosure.decision();
    assert_eq!(
        submitted_decision.score,
        matches!(
            disclosure_policy.score,
            StudentDisclosureTiming::DuringAttempt | StudentDisclosureTiming::AfterSubmit
        ),
        "current assignment policy, not the question's legacy feedback setting, controls score"
    );
    assert_eq!(
        submitted_decision.per_item_correctness,
        matches!(
            disclosure_policy.per_item_correctness,
            StudentDisclosureTiming::DuringAttempt | StudentDisclosureTiming::AfterSubmit
        )
    );
    assert_eq!(
        submitted_decision.feedback_text,
        matches!(
            disclosure_policy.feedback_text,
            StudentDisclosureTiming::DuringAttempt | StudentDisclosureTiming::AfterSubmit
        )
    );
    assert_eq!(
        submitted_decision.solution,
        matches!(
            disclosure_policy.solution,
            StudentDisclosureTiming::DuringAttempt | StudentDisclosureTiming::AfterSubmit
        )
    );
    assert!(!submitted_decision.class_statistics);
    assert!(before_completion.outcomes.items[0].feedback.is_some());
    if submitted_decision.feedback_text || submitted_decision.solution {
        assert_eq!(
            store
                .get_attempt_feedback_release(context, student_user, attempt.id)
                .await,
            Ok(None),
            "a student may observe only their exact unreleased attempt state"
        );
        assert_eq!(
            store
                .release_attempt_feedback(
                    TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(9_401))),
                    ReleaseAttemptFeedbackCommand {
                        actor: publisher,
                        attempt: attempt.id,
                    },
                )
                .await,
            Err(StoreError::NotFound),
            "a foreign tenant must not enumerate a release target"
        );
        assert_eq!(
            store
                .release_attempt_feedback(
                    context,
                    ReleaseAttemptFeedbackCommand {
                        actor: student_user,
                        attempt: attempt.id,
                    },
                )
                .await,
            Err(StoreError::NotFound),
            "an ordinary student cannot release feedback"
        );
        let released = store
            .release_attempt_feedback(
                context,
                ReleaseAttemptFeedbackCommand {
                    actor: publisher,
                    attempt: attempt.id,
                },
            )
            .await
            .expect("course instructor records a permitted feedback-release audit event");
        assert_eq!(
            store
                .release_attempt_feedback(
                    context,
                    ReleaseAttemptFeedbackCommand {
                        actor: publisher,
                        attempt: attempt.id,
                    },
                )
                .await,
            Ok(released.clone()),
            "same authorized actor release is idempotent"
        );
        assert_eq!(
            store
                .get_attempt_feedback_release(context, student_user, attempt.id)
                .await,
            Ok(Some(released)),
            "the owner can read current released state without listing feedback"
        );
        assert_eq!(
            store
                .get_run_summary_page(
                    context,
                    student_user,
                    run.id,
                    PageRequest::first(PageSize::new(10).expect("valid bounded page")),
                )
                .await
                .expect("audit-event summary")
                .outcomes
                .items[0]
                .disclosure
                .decision(),
            submitted_decision,
            "a feedback-release audit event never unlocks learner disclosure"
        );
    } else {
        assert!(matches!(
            store
                .release_attempt_feedback(
                    context,
                    ReleaseAttemptFeedbackCommand {
                        actor: publisher,
                        attempt: attempt.id,
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
    }
    assert!(
        store
            .submit_question_attempt(
                context,
                SubmitQuestionAttemptCommand {
                    actor: student_user,
                    binding: StudentWorkRoutingBinding::new(course, assignment),
                    attempt: attempt.id,
                    response: response.clone(),
                    result: AttemptResult {
                        correct: false,
                        points_earned: 0.0,
                        points_possible: 1.0,
                    },
                    feedback: FeedbackContent {
                        hint: Some(vec![ContentBlock::Text {
                            markdown: "a changed retry cannot replace this".to_string(),
                        }]),
                        correct_response: None,
                        rationale: None,
                    },
                    idempotency_key: key.clone(),
                },
            )
            .await
            .expect("exact replay should ignore the changed proposed grade")
            .feedback
            == submitted.feedback
    );
    assert_eq!(
        store
            .replay_submission(
                context,
                student_user,
                attempt.id,
                &StudentResponse::Numeric { value: 19.0 },
                &key,
            )
            .await,
        Err(StoreError::Conflict)
    );
    let changed_key =
        SubmissionIdempotencyKey::parse("submission-401-new").expect("valid changed key");
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &changed_key)
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(submitted.run.completed_at, None);
    assert_eq!(
        store
            .pending_submission_for_run(context, student_user, run.id)
            .await,
        Ok(Some(attempt.id)),
        "one committed predecessor without a receipt successor is recoverable",
    );
    assert_eq!(
        store
            .pending_submission_for_run(context, unrelated_user, run.id)
            .await,
        Err(StoreError::NotFound),
        "a different member cannot discover a student's pending submission",
    );

    super::receipt_lifecycle::complete_receipt_lifecycle(
        store,
        super::receipt_lifecycle::ReceiptLifecycleFixture {
            fixture_offset,
            tenant,
            context,
            publisher,
            student_user,
            unrelated_user,
            workspace,
            problem,
            version,
            course,
            assignment,
            grade_policy,
            run,
            attempt,
            reservation,
            reservation_private_execution,
            response,
            key,
            submitted,
        },
    )
    .await
}
