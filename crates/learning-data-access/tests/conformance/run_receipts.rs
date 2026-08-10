use super::*;

pub(super) async fn exercise_run_api_receipts<S>(
    store: &S,
    feedback_disclosure: FeedbackDisclosure,
) -> RunApiFixture
where
    S: Store + CatalogStore + JobStore + AssignmentScoringWorkerStore,
{
    let fixture_offset = if feedback_disclosure == FeedbackDisclosure::OnRelease {
        10_000
    } else {
        0
    };
    let tenant = TenantId::from_uuid(uuid(401 + fixture_offset));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(uuid(402));
    let student_user = UserId::from_uuid(uuid(403));
    let second_instructor = UserId::from_uuid(uuid(10_403 + fixture_offset));
    let workspace = WorkspaceId::from_uuid(uuid(404));
    let problem = ProblemId::from_uuid(uuid(405 + fixture_offset));
    let version = VersionId::from_uuid(uuid(406 + fixture_offset));
    let course = CourseId::from_uuid(uuid(407));
    let assignment = AssignmentId::from_uuid(uuid(408));
    let enrollment = EnrollmentId::from_uuid(uuid(409));
    let first_run = RunId::from_uuid(uuid(410));
    let ignored_resume_id = RunId::from_uuid(uuid(411));
    let attempt_id = QuestionAttemptId::from_uuid(uuid(412));

    let mut run_question = draft_question(workspace);
    // This fixture specifically proves receipt-time replay behavior: a later
    // completion must not unlock deferred feedback on the earlier receipt.
    run_question.attempt_policy.feedback = feedback_disclosure;
    let draft = DraftRecord {
        tenant,
        question: run_question,
        revises: None,
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
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("run fixture publication");
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Run API biochemistry".to_string(),
                members: vec![
                    CourseMembership {
                        user: publisher,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: second_instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: student_user,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("run fixture course");
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Run API assignment".to_string(),
                items: fixed_items(vec![
                    ProblemVersionRef { problem, version },
                    ProblemVersionRef { problem, version },
                ]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("run fixture assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment,
                tenant,
                assignment,
                user: student_user,
                student: StudentId::from_uuid(uuid(413)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("run fixture enrollment");

    let run = store
        .start_or_resume_run(context, student_user, assignment, first_run)
        .await
        .expect("first run should start");
    let resumed = store
        .start_or_resume_run(context, student_user, assignment, ignored_resume_id)
        .await
        .expect("active run should resume");
    assert_eq!(resumed, run);

    let issue = IssueQuestionAttemptCommand {
        actor: student_user,
        attempt: attempt_id,
        run: run.id,
        assignment_position: 0,
        problem,
        question_version: version,
        seed: 991,
        presentation: Some(presentation_binding(7)),
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

    let blocked_second_position = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(415)),
                run: run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                seed: 993,
                presentation: Some(presentation_binding(8)),
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

    let reservation = PrefetchedQuestion {
        tenant,
        run: run.id,
        predecessor: attempt.id,
        assignment_position: 1,
        problem,
        question_version: version,
        seed: 993,
        presentation: presentation_binding(9),
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
        webwork_replay: None,
    };
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: student_user,
                    reservation: reservation.clone(),
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
                    reservation: reservation.clone(),
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
                    reservation: PrefetchedQuestion {
                        seed: reservation.seed + 1,
                        ..reservation.clone()
                    },
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
                    actor: second_instructor,
                    reservation: reservation.clone(),
                },
            )
            .await,
        Err(StoreError::Forbidden),
        "another course member cannot reserve a student's next question",
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
        Ok(None)
    );
    let invalid_result = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
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
        Ok(None),
        "a rejected backend result must leave the attempt unsubmitted"
    );
    let hostile_feedback = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
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
        Ok(None),
        "rejected feedback must not leave a submission, feedback, or summary partial write"
    );
    let submitted = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
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
    let replay = store
        .replay_submission(context, student_user, attempt.id, &response, &key)
        .await
        .expect("replay lookup")
        .expect("first receipt should replay");
    assert_eq!(replay.attempt, submitted.attempt);
    assert!(replay.feedback == submitted.feedback);
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
    assert_eq!(
        before_completion.outcomes.items[0].feedback_policy, feedback_disclosure,
        "every policy must survive in the private redactor input"
    );
    assert!(before_completion.outcomes.items[0].feedback.is_some());
    assert_eq!(before_completion.outcomes.items[0].release, None);
    if feedback_disclosure == FeedbackDisclosure::OnRelease {
        assert_eq!(
            store
                .get_attempt_feedback_release(context, student_user, attempt.id)
                .await,
            Ok(None),
            "a student may observe only their exact unreleased attempt state"
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
                .expect("unreleased summary")
                .outcomes
                .items[0]
                .release,
            None,
            "summary redaction input reflects current unreleased state"
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
            .expect("course instructor releases on-release feedback");
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
                .release_attempt_feedback(
                    context,
                    ReleaseAttemptFeedbackCommand {
                        actor: second_instructor,
                        attempt: attempt.id,
                    },
                )
                .await,
            Err(StoreError::Conflict),
            "a release remains immutable for a different authorized instructor"
        );
        assert_eq!(
            store
                .get_attempt_feedback_release(context, student_user, attempt.id)
                .await,
            Ok(Some(released)),
            "the owner can read current released state without listing feedback"
        );
        assert!(
            store
                .get_run_summary_page(
                    context,
                    student_user,
                    run.id,
                    PageRequest::first(PageSize::new(10).expect("valid bounded page")),
                )
                .await
                .expect("released summary")
                .outcomes
                .items[0]
                .release
                .is_some(),
            "summary redaction input reads current release state, not receipt state"
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
            .pending_submission_for_run(context, second_instructor, run.id)
            .await,
        Err(StoreError::Forbidden),
        "another course member cannot discover a student's pending submission",
    );

    let second_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(415)),
                run: run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                seed: 0,
                presentation: Some(reservation.presentation),
                parameter_hash: "ignored-by-prefetch".to_string(),
                provenance: reservation.provenance.clone(),
                webwork_replay: None,
                prefetched: Some(reservation.clone()),
                predecessor_submission: Some(attempt.id),
            },
        )
        .await
        .expect("the next position should issue after the active response commits");
    assert_eq!(second_attempt.seed, reservation.seed);
    assert_eq!(second_attempt.parameter_hash, reservation.parameter_hash);
    assert_eq!(
        store
            .submission_next_attempt(context, student_user, attempt.id)
            .await,
        Ok(learning_data_access::SubmissionNextAttempt::Issued(
            second_attempt.id
        )),
        "promotion atomically fixes the predecessor receipt successor",
    );
    assert_eq!(
        store
            .pending_submission_for_run(context, student_user, run.id)
            .await,
        Ok(None),
        "promotion consumes the only pending receipt rather than leaving recovery ambiguous",
    );
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: student_user,
                    reservation: reservation.clone(),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "an already-attempted target position cannot be reserved again",
    );
    assert_eq!(
        store
            .issue_or_resume_question_attempt(
                context,
                IssueQuestionAttemptCommand {
                    actor: student_user,
                    attempt: QuestionAttemptId::from_uuid(uuid(416)),
                    run: run.id,
                    assignment_position: 1,
                    problem,
                    question_version: version,
                    seed: 0,
                    presentation: Some(reservation.presentation),
                    parameter_hash: "ignored-by-prefetch".to_string(),
                    provenance: reservation.provenance.clone(),
                    webwork_replay: None,
                    prefetched: Some(reservation.clone()),
                    predecessor_submission: None,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a reservation cannot be consumed or resumed under another receipt predecessor",
    );
    let completed = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: second_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("submission-402")
                    .expect("valid second key"),
            },
        )
        .await
        .expect("second response should complete the run");
    assert_eq!(
        completed.run.completed_at,
        completed.attempt.timer.submitted_at
    );
    assert_eq!(
        store
            .pending_submission_for_run(context, student_user, run.id)
            .await,
        Ok(Some(second_attempt.id)),
        "a terminal committed submission is the sole recoverable receipt until finalized",
    );
    assert_eq!(
        store
            .finalize_submission_next_attempt(context, second_instructor, second_attempt.id, None)
            .await,
        Err(StoreError::NotFound),
        "another course member cannot enumerate or finalize a student's pending receipt",
    );
    let cross_run = store
        .start_or_resume_run(
            context,
            student_user,
            assignment,
            RunId::from_uuid(uuid(417)),
        )
        .await
        .expect("a completed run permits a new run");
    let cross_run_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(418)),
                run: cross_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: 994,
                presentation: Some(presentation_binding(10)),
                parameter_hash: "cross-run-parameter-hash".to_string(),
                provenance: reservation.provenance.clone(),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("cross-run active attempt");
    assert_eq!(
        store
            .finalize_submission_next_attempt(
                context,
                student_user,
                second_attempt.id,
                Some(cross_run_attempt.id),
            )
            .await,
        Err(StoreError::Conflict),
        "a receipt cannot link to an attempt from another run",
    );
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: cross_run_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("submission-cross-run-1")
                    .expect("valid cross-run key"),
            },
        )
        .await
        .expect("first deliberately unfinalized recovery fixture submission");
    let cross_run_second = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(419)),
                run: cross_run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                seed: 995,
                presentation: Some(presentation_binding(11)),
                parameter_hash: "cross-run-second-parameter-hash".to_string(),
                provenance: reservation.provenance.clone(),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("a recovery fixture can reproduce a second issue after a lost finalization");
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: cross_run_second.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("submission-cross-run-2")
                    .expect("valid second cross-run key"),
            },
        )
        .await
        .expect("second deliberately unfinalized recovery fixture submission");
    assert_eq!(
        store
            .pending_submission_for_run(context, student_user, cross_run.id)
            .await,
        Err(StoreError::Conflict),
        "multiple unresolved receipt links are ambiguous and must never be guessed",
    );
    assert_eq!(
        store
            .finalize_submission_next_attempt(context, student_user, second_attempt.id, None)
            .await,
        Ok(()),
        "a terminal submission records its explicit no-successor receipt state",
    );
    assert_eq!(
        store
            .finalize_submission_next_attempt(context, student_user, second_attempt.id, None)
            .await,
        Ok(()),
        "the explicit no-successor receipt state is idempotent",
    );
    assert_eq!(
        store
            .finalize_submission_next_attempt(
                context,
                student_user,
                second_attempt.id,
                Some(attempt.id),
            )
            .await,
        Err(StoreError::Conflict),
        "a finalized no-successor receipt cannot later point at an attempt",
    );
    assert_eq!(
        store
            .submission_next_attempt(context, student_user, attempt.id)
            .await,
        Ok(learning_data_access::SubmissionNextAttempt::Issued(
            second_attempt.id
        )),
        "the first receipt keeps its original successor after that successor is submitted",
    );
    assert_eq!(
        (
            completed.summary.completed_run_count,
            completed.summary.total_question_attempts,
            completed.summary.current_score,
        ),
        (1, 2, Some(1.0))
    );
    let replay_after_completion = store
        .replay_submission(context, student_user, attempt.id, &response, &key)
        .await
        .expect("first submission replay after later completion")
        .expect("first submission receipt remains available");
    assert_eq!(replay_after_completion.attempt, submitted.attempt);
    assert_eq!(replay_after_completion.run, submitted.run);
    assert_eq!(replay_after_completion.summary, submitted.summary);
    assert!(replay_after_completion.feedback == submitted.feedback);
    let attempt_page = store
        .list_question_attempts(
            context,
            run.id,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("attempt page");
    assert_eq!(
        attempt_page.items,
        vec![submitted.attempt, completed.attempt]
    );
    let first_summary_page = store
        .get_run_summary_page(
            context,
            student_user,
            run.id,
            PageRequest::first(PageSize::new(1).expect("valid bounded page")),
        )
        .await
        .expect("owner summary page");
    assert_eq!(first_summary_page.run, completed.run);
    // The receipt retains the summary observed when it committed. The
    // enrollment summary is live and has since observed the deliberately
    // completed independent recovery fixture run above.
    assert_eq!(first_summary_page.summary.completed_run_count, 2);
    assert_eq!(first_summary_page.summary.total_question_attempts, 4);
    assert!(first_summary_page.practice_allowed);
    assert_eq!(first_summary_page.outcomes.items.len(), 1);
    assert!(first_summary_page.outcomes.items[0].response.is_some());
    assert!(first_summary_page.outcomes.items[0].feedback.is_some());
    let continuation = first_summary_page
        .outcomes
        .next_cursor
        .expect("two outcomes require a cursor");
    let second_summary_page = store
        .get_run_summary_page(
            context,
            student_user,
            run.id,
            PageRequest::after(continuation, PageSize::new(1).expect("valid bounded page")),
        )
        .await
        .expect("owner summary continuation");
    assert_eq!(second_summary_page.outcomes.items.len(), 1);
    assert_ne!(
        first_summary_page.outcomes.items[0].attempt, second_summary_page.outcomes.items[0].attempt,
        "keyset pages must not duplicate outcomes"
    );
    assert_eq!(second_summary_page.outcomes.next_cursor, None);
    let instructor_summary = store
        .get_run_summary_page(
            context,
            publisher,
            run.id,
            PageRequest::first(PageSize::new(10).expect("valid bounded page")),
        )
        .await
        .expect("direct course instructor summary");
    assert_eq!(instructor_summary.outcomes.items.len(), 2);
    let foreign_actor = UserId::from_uuid(uuid(99_999 + fixture_offset));
    assert!(matches!(
        store
            .get_run_summary_page(
                context,
                foreign_actor,
                run.id,
                PageRequest::first(PageSize::new(10).expect("valid bounded page")),
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store
            .get_run_summary_page(
                TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(
                    99_998 + fixture_offset,
                ))),
                student_user,
                run.id,
                PageRequest::first(PageSize::new(10).expect("valid bounded page")),
            )
            .await,
        Err(StoreError::NotFound)
    ));

    RunApiFixture {
        fixture_offset,
        tenant,
        context,
        publisher,
        student_user,
        problem,
        version,
        course,
        assignment,
        reservation,
        response,
        run,
    }
}
