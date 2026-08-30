use super::run_receipts::receipt_presentation;
use super::*;
use learning_data_access::SealedPrivateExecutionStore;

pub(super) async fn exercise_attempt_support<S, P>(
    store: &S,
    sealed_private_execution: &P,
    fixture: &RunApiFixture,
) where
    S: Store + CatalogStore + JobStore + AssignmentScoringWorkerStore,
    P: SealedPrivateExecutionStore,
{
    let fixture_offset = fixture.fixture_offset;
    let tenant = fixture.tenant;
    let context = fixture.context;
    let publisher = fixture.publisher;
    let student_user = fixture.student_user;
    let course = fixture.course;
    let reservation = &fixture.reservation;
    let response = &fixture.response;
    let problem = ProblemId::from_uuid(uuid(89_968 + fixture_offset));
    let version = VersionId::from_uuid(uuid(89_969 + fixture_offset));
    let reference = ProblemVersionRef { problem, version };
    let mut question = draft_question(WorkspaceId::from_uuid(uuid(89_970 + fixture_offset)));
    question.source = DraftQuestionSource::Webwork {
        pg_path: "Library/PLE/replay-contract.pg".to_string(),
    };
    let draft = DraftRecord {
        tenant,
        question,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("WeBWorK replay contract draft");
    let artifact = source_artifact(
        reference,
        QuestionBackend::Webwork,
        ObjectId::from_uuid(uuid(89_971 + fixture_offset)),
    );
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft.clone(),
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Webwork {
                    pg_path: "Library/PLE/replay-contract.pg".to_string(),
                },
                source_artifact: Some(artifact.clone()),
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Institution,
                byline: reviewed_byline(),
                capabilities: BackendCapabilities::from_iter([
                    Capability::AlgorithmicGeneration,
                    Capability::ServerGrading,
                ]),
            },
        )
        .await
        .expect("WeBWorK replay contract publication");
    let support_webwork_grading = learning_data_access::IssuedWebworkGradingContract::new(
        question_model::QuestionDefinition::from_draft(
            draft.question.clone(),
            problem,
            version,
            QuestionSource::Webwork {
                pg_path: "Library/PLE/replay-contract.pg".to_string(),
            },
        ),
    )
    .expect("published WebWork fixture has an immutable grading definition");
    let support_issued_question_snapshot = learning_data_access::IssuedQuestionSnapshotV1::new(
        question_model::QuestionDefinition::from_draft(
            draft.question.clone(),
            problem,
            version,
            QuestionSource::Webwork {
                pg_path: "Library/PLE/replay-contract.pg".to_string(),
            },
        ),
        learning_data_access::IssuedQuestionFamilyWitnessV1::Webwork {},
    )
    .expect("published WebWork fixture has an immutable issued snapshot");
    let support_assignment = AssignmentId::from_uuid(uuid(89_972 + fixture_offset));
    let support_run_id = RunId::from_uuid(uuid(89_974 + fixture_offset));
    store
        .create_assignment_with_default_policy(
            context,
            publisher,
            AssignmentRecord {
                id: support_assignment,
                tenant,
                course_id: course,
                title: "Attempt support fixture".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Published,
                instructions: question_model::AssignmentInstructions::default(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: fixed_items(vec![
                    ProblemVersionRef { problem, version },
                    ProblemVersionRef { problem, version },
                ]),
                selection_groups: Vec::new(),
                disclosure_policy: question_model::StudentDisclosurePolicy::default(),
                policies: policies(),
            },
        )
        .await
        .expect("attempt support assignment");
    let support_run = store
        .start_or_resume_run(
            context,
            student_user,
            StudentWorkRoutingBinding::new(course, support_assignment),
            support_run_id,
        )
        .await
        .expect("attempt support run");
    let (support_presentation, support_snapshot) = receipt_presentation(version, 999, 1);
    let support_replay = WebworkReplayMappingV1::SingleChoice {
        items: vec![
            WebworkReplayControlV1 {
                item: RenderedItemIdV1::parse("a1b2").expect("rendered choice ID"),
                field: "AnSwEr0001".to_string(),
                value: "0".to_string(),
            },
            WebworkReplayControlV1 {
                item: RenderedItemIdV1::parse("c3d4").expect("rendered choice ID"),
                field: "AnSwEr0001".to_string(),
                value: "1".to_string(),
            },
        ],
    };
    let mut support_provenance = reservation.provenance.clone();
    support_provenance.source_artifact = Some(SourceArtifact {
        object: artifact.object.id,
        sha256: artifact.object.sha256.to_string(),
    });
    support_provenance.renderer = Some(implementation("webwork-renderer"));
    let support_issue = IssueQuestionAttemptCommand {
        actor: student_user,
        binding: StudentWorkRoutingBinding::new(course, support_assignment),
        attempt: QuestionAttemptId::from_uuid(uuid(89_976 + fixture_offset)),
        run: support_run.id,
        assignment_position: 0,
        problem,
        question_version: version,
        issued_question_snapshot: support_issued_question_snapshot.clone(),
        seed: 999,
        presentation_capability: PresentationCapability::EnvelopeV1,
        presentation: Some(support_presentation),
        presentation_snapshot: Some(support_snapshot),
        grading_envelope: Some(grading_envelope(version, 999)),
        native_execution_envelope_capability:
            learning_data_access::NativeExecutionEnvelopeCapability::NotApplicable,
        flat_grading: None,
        flat_grading_capability: FlatGradingCapability::NotApplicable,
        webwork_grading: Some(support_webwork_grading.clone()),
        webwork_grading_capability: WebworkGradingCapability::Required,
        qti_grading: None,
        qti_grading_capability: learning_data_access::QtiGradingCapability::NotApplicable,
        parameter_hash: "force-submit-active".to_string(),
        provenance: support_provenance.clone(),
        webwork_replay: Some(support_replay.clone()),
        prefetched: None,
        predecessor_submission: None,
    };
    assert!(matches!(
        store
            .issue_or_resume_question_attempt(
                context,
                IssueQuestionAttemptCommand {
                    attempt: QuestionAttemptId::from_uuid(uuid(89_979 + fixture_offset)),
                    webwork_replay: None,
                    ..support_issue.clone()
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(
        store
            .list_question_attempts(
                context,
                support_run.id,
                PageRequest::first(PageSize::new(10).expect("valid page size")),
            )
            .await
            .expect("missing replay leaves the run readable")
            .items
            .is_empty(),
        "missing required WeBWorK replay state must not create an attempt"
    );
    let support_attempt = store
        .issue_or_resume_question_attempt(context, support_issue)
        .await
        .expect("attempt support question");
    let stored_evidence = store
        .read_issued_attempt_evidence(
            context,
            student_user,
            StudentWorkRoutingBinding::new(course, support_assignment),
            support_attempt.id,
        )
        .await
        .expect("attempt owner reads issued evidence");
    let learning_data_access::IssuedAttemptRead::Active(stored_evidence) = stored_evidence else {
        panic!("fresh WebWork attempt returns active issued evidence");
    };
    assert!(
        stored_evidence.presentation_snapshot().is_some(),
        "ordinary issued evidence retains only its answer-free presentation projection"
    );
    let sealed_idempotency_key = SubmissionIdempotencyKey::parse("sealed-support-preparation")
        .expect("valid sealed preparation key");
    let ordinary_preparation = store
        .prepare_question_submission(
            context,
            student_user,
            StudentWorkRoutingBinding::new(course, support_assignment),
            support_attempt.id,
            response,
            &sealed_idempotency_key,
        )
        .await
        .expect("ordinary store authorizes first effect without private execution material");
    let learning_data_access::SubmissionPreparation::FirstEffect(authorized_intent) =
        ordinary_preparation
    else {
        panic!("fresh support attempt authorizes a first grading effect");
    };
    let sealed_preparation = sealed_private_execution
        .prepare_sealed_private_execution(
            context,
            student_user,
            StudentWorkRoutingBinding::new(course, support_assignment),
            *authorized_intent,
            response,
            &sealed_idempotency_key,
        )
        .await
        .expect("sealed grader capability projects private WebWork execution material");
    let learning_data_access::SealedPrivateExecutionPreparation::Grade(prepared_submission) =
        sealed_preparation
    else {
        panic!("fresh sealed support preparation is a grading effect");
    };
    let sealed_replay = prepared_submission
        .webwork_replay
        .as_ref()
        .expect("sealed WebWork preparation retains replay authority");
    assert_eq!(sealed_replay.mapping, support_replay);
    assert_eq!(
        sealed_replay.presentation_digest,
        support_presentation.digest(),
        "the sealed replay mapping stays bound to the issued presentation"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(
                context,
                student_user,
                StudentWorkRoutingBinding::new(course, fixture.assignment),
                support_attempt.id,
            )
            .await,
        Err(StoreError::NotFound),
        "a route with the same course but a different assignment cannot read issued evidence"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(
                context,
                student_user,
                StudentWorkRoutingBinding::new(
                    CourseId::from_uuid(uuid(89_980 + fixture_offset)),
                    support_assignment,
                ),
                support_attempt.id,
            )
            .await,
        Err(StoreError::NotFound),
        "a swapped course route cannot read issued evidence"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(
                context,
                student_user,
                StudentWorkRoutingBinding::new(course, support_assignment),
                QuestionAttemptId::from_uuid(uuid(89_981 + fixture_offset)),
            )
            .await,
        Err(StoreError::NotFound),
        "an unknown attempt cannot be enumerated through the route-bound capability"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(
                context,
                publisher,
                StudentWorkRoutingBinding::new(course, support_assignment),
                support_attempt.id,
            )
            .await,
        Err(StoreError::NotFound),
        "an instructor cannot discover learner-bound private replay state"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(
                TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(
                    89_978 + fixture_offset,
                ))),
                student_user,
                StudentWorkRoutingBinding::new(course, support_assignment),
                support_attempt.id,
            )
            .await,
        Err(StoreError::NotFound),
        "a foreign tenant cannot enumerate private replay state"
    );
    let force_action = AttemptSupportActionId::from_uuid(uuid(89_977 + fixture_offset));
    assert_eq!(
        store
            .force_submit_attempt(
                context,
                ForceSubmitAttemptCommand {
                    action: force_action,
                    actor: student_user,
                    attempt: support_attempt.id,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "a student cannot force-submit an educational record"
    );
    assert_eq!(
        store
            .force_submit_attempt(
                TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(
                    89_978 + fixture_offset,
                ))),
                ForceSubmitAttemptCommand {
                    action: force_action,
                    actor: publisher,
                    attempt: support_attempt.id,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "a foreign tenant cannot enumerate a support target"
    );
    let forced = store
        .force_submit_attempt(
            context,
            ForceSubmitAttemptCommand {
                action: force_action,
                actor: publisher,
                attempt: support_attempt.id,
            },
        )
        .await
        .expect("direct course instructor force-submits");
    assert_eq!(
        (forced.kind, forced.previous_status, forced.resulting_status),
        (
            AttemptSupportAction::ForceSubmit,
            AttemptStatus::InProgress,
            AttemptStatus::AutoSubmitted,
        )
    );
    assert_eq!(
        store
            .force_submit_attempt(
                context,
                ForceSubmitAttemptCommand {
                    action: force_action,
                    actor: publisher,
                    attempt: support_attempt.id,
                },
            )
            .await,
        Ok(forced),
        "an exact support retry returns the original audit record"
    );
    assert_eq!(
        store
            .clear_attempt(
                context,
                ClearAttemptCommand {
                    action: force_action,
                    actor: publisher,
                    attempt: support_attempt.id,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "one stable action identity cannot be reused for a different mutation"
    );
    assert_eq!(
        store
            .submit_question_attempt(
                context,
                SubmitQuestionAttemptCommand {
                    actor: student_user,
                    binding: StudentWorkRoutingBinding::new(course, support_assignment),
                    attempt: support_attempt.id,
                    response: response.clone(),
                    result: AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    },
                    feedback: FeedbackContent::default(),
                    idempotency_key: SubmissionIdempotencyKey::parse(
                        "submission-after-force-submit",
                    )
                    .expect("valid force-submit conflict key"),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "force-submit closes the ordinary student submission path"
    );
    let forced_current = store
        .get_question_attempt(context, support_attempt.id)
        .await
        .expect("force-submitted attempt read")
        .expect("force-submitted attempt exists");
    assert_eq!(forced_current.status, AttemptStatus::AutoSubmitted);
    assert!(forced_current.response.is_none());
    assert!(forced_current.result.is_none());
    assert_eq!(forced_current.timer.submitted_at, Some(forced.occurred_at));
    assert!(
        matches!(
            store
                .read_issued_attempt_evidence(
                    context,
                    student_user,
                    StudentWorkRoutingBinding::new(course, support_assignment),
                    support_attempt.id,
                )
                .await,
            Ok(learning_data_access::IssuedAttemptRead::TerminalWithoutReceipt(ref read))
                if read.status() == AttemptStatus::AutoSubmitted
        ),
        "terminal support action returns no active replay authority"
    );

    let clear_forced_action = AttemptSupportActionId::from_uuid(uuid(89_979 + fixture_offset));
    let cleared_forced = store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: clear_forced_action,
                actor: publisher,
                attempt: support_attempt.id,
            },
        )
        .await
        .expect("instructor clears force-submitted attempt");
    assert_eq!(
        (
            cleared_forced.previous_status,
            cleared_forced.resulting_status
        ),
        (AttemptStatus::AutoSubmitted, AttemptStatus::Cleared)
    );
    assert_eq!(
        store
            .clear_attempt(
                context,
                ClearAttemptCommand {
                    action: clear_forced_action,
                    actor: publisher,
                    attempt: support_attempt.id,
                },
            )
            .await,
        Ok(cleared_forced),
        "an exact clear retry is harmless"
    );
    assert!(
        store
            .get_run_summary_page(
                context,
                student_user,
                support_run.id,
                PageRequest::first(PageSize::new(10).expect("support student page")),
            )
            .await
            .expect("student support summary")
            .outcomes
            .items
            .is_empty(),
        "cleared evidence is absent from the ordinary student summary"
    );
    assert_eq!(
        store
            .get_run_summary_page(
                context,
                publisher,
                support_run.id,
                PageRequest::first(PageSize::new(10).expect("support instructor page")),
            )
            .await
            .expect("instructor support summary")
            .outcomes
            .items
            .len(),
        1,
        "the instructor retains raw evidence access after clear"
    );

    let (replacement_presentation, replacement_snapshot) = receipt_presentation(version, 1_000, 2);
    let replacement_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                binding: StudentWorkRoutingBinding::new(course, support_assignment),
                attempt: QuestionAttemptId::from_uuid(uuid(89_981 + fixture_offset)),
                run: support_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                issued_question_snapshot: support_issued_question_snapshot.clone(),
                seed: 1_000,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(replacement_presentation),
                presentation_snapshot: Some(replacement_snapshot),
                grading_envelope: Some(grading_envelope(version, 1_000)),
                native_execution_envelope_capability:
                    learning_data_access::NativeExecutionEnvelopeCapability::NotApplicable,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: Some(support_webwork_grading.clone()),
                webwork_grading_capability: WebworkGradingCapability::Required,
                qti_grading: None,
                qti_grading_capability: learning_data_access::QtiGradingCapability::NotApplicable,
                parameter_hash: "replacement-after-clear".to_string(),
                provenance: support_provenance.clone(),
                webwork_replay: Some(support_replay.clone()),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("a cleared position may issue a replacement");
    assert!(matches!(
        store
            .read_issued_attempt_evidence(
                context,
                student_user,
                StudentWorkRoutingBinding::new(course, support_assignment),
                replacement_attempt.id,
            )
            .await,
        Ok(learning_data_access::IssuedAttemptRead::Active(_))
    ));
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                binding: StudentWorkRoutingBinding::new(course, support_assignment),
                attempt: replacement_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("submission-support-replacement")
                    .expect("valid support replacement key"),
            },
        )
        .await
        .expect("replacement attempt submits");
    assert!(
        matches!(
            store
                .read_issued_attempt_evidence(
                    context,
                    student_user,
                    StudentWorkRoutingBinding::new(course, support_assignment),
                    replacement_attempt.id,
                )
                .await,
            Ok(learning_data_access::IssuedAttemptRead::Submitted(ref read))
                if read.presentation().is_some()
        ),
        "successful submission uses its immutable receipt without replay authority"
    );
    assert_eq!(
        store
            .clear_attempt(
                context,
                ClearAttemptCommand {
                    action: AttemptSupportActionId::from_uuid(uuid(89_982 + fixture_offset,)),
                    actor: student_user,
                    attempt: replacement_attempt.id,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "a student cannot clear a submitted evaluation"
    );
    let clear_scored_action = AttemptSupportActionId::from_uuid(uuid(89_983 + fixture_offset));
    let cleared_scored = store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: clear_scored_action,
                actor: publisher,
                attempt: replacement_attempt.id,
            },
        )
        .await
        .expect("instructor clears submitted evaluation");
    assert_eq!(cleared_scored.previous_status, AttemptStatus::Submitted);
    assert_eq!(cleared_scored.resulting_status, AttemptStatus::Cleared);
    assert_eq!(
        store
            .clear_attempt(
                context,
                ClearAttemptCommand {
                    action: clear_scored_action,
                    actor: publisher,
                    attempt: replacement_attempt.id,
                },
            )
            .await,
        Ok(cleared_scored),
        "a clear retry neither advances the generation nor queues duplicate work"
    );
    let (post_clear_presentation, post_clear_snapshot) = receipt_presentation(version, 1_001, 3);
    let post_clear_replacement = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                binding: StudentWorkRoutingBinding::new(course, support_assignment),
                attempt: QuestionAttemptId::from_uuid(uuid(89_984 + fixture_offset)),
                run: support_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                issued_question_snapshot: support_issued_question_snapshot,
                seed: 1_001,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(post_clear_presentation),
                presentation_snapshot: Some(post_clear_snapshot),
                grading_envelope: Some(grading_envelope(version, 1_001)),
                native_execution_envelope_capability:
                    learning_data_access::NativeExecutionEnvelopeCapability::NotApplicable,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: Some(support_webwork_grading),
                webwork_grading_capability: WebworkGradingCapability::Required,
                qti_grading: None,
                qti_grading_capability: learning_data_access::QtiGradingCapability::NotApplicable,
                parameter_hash: "replacement-after-scored-clear".to_string(),
                provenance: support_provenance,
                webwork_replay: Some(support_replay),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("a cleared correct response does not block a replacement");
    assert_eq!(post_clear_replacement.status, AttemptStatus::InProgress);
    let support_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("support scoring lease"),
        )
        .await
        .expect("claim support scoring job")
        .expect("clearing a scored attempt queues recalculation");
    let support_generation = match support_job.payload {
        JobPayload::RecalculateAssignment {
            assignment: queued_assignment,
            generation,
        } => {
            assert_eq!(queued_assignment, support_assignment);
            generation
        }
        payload => panic!("expected attempt-clear scoring job, got {payload:?}"),
    };
    let support_scoring = AssignmentScoringWorkerCommand {
        job: support_job.id,
        lease: support_job.lease_token,
        assignment: support_assignment,
        generation: support_generation,
    };
    store
        .prepare_assignment_scoring(support_scoring)
        .await
        .expect("attempt-clear scoring stages without the cleared result");
    assert_eq!(
        store.commit_assignment_scoring(support_scoring).await,
        Ok(AssignmentScoringCommitOutcome::Committed)
    );
    let support_assignment_current = store
        .get_assignment_for_edit(context, support_assignment)
        .await
        .expect("support assignment state read")
        .expect("support assignment exists");
    assert_eq!(
        (
            support_assignment_current.scoring_generation,
            support_assignment_current.scoring_status,
        ),
        (support_generation, question_model::ScoringStatus::Current,)
    );
    assert_eq!(
        store
            .get_run_summary_page(
                context,
                publisher,
                support_run.id,
                PageRequest::first(PageSize::new(10).expect("retained support evidence page")),
            )
            .await
            .expect("retained support evidence summary")
            .outcomes
            .items
            .len(),
        3,
        "the instructor sees both cleared records and the active replacement"
    );
}
