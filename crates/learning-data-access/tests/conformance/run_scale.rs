use super::*;

pub(super) async fn exercise_run_summary_scale<S>(store: &S, fixture: &RunApiFixture)
where
    S: Store + CatalogStore + JobStore + AssignmentScoringWorkerStore,
{
    let fixture_offset = fixture.fixture_offset;
    let tenant = fixture.tenant;
    let context = fixture.context;
    let student_user = fixture.student_user;
    let course = fixture.course;
    let problem = fixture.problem;
    let version = fixture.version;
    let run = &fixture.run;
    // Scale behavior is deliberately exercised through the Store, not just
    // the cursor helper. Every synthetic outcome uses normal issuance so it
    // has the sealed/current S3 receipt required by learner disclosure; the
    // summary route must never invent a fallback for fixture-only attempts.
    let scale_run_id = RunId::from_uuid(uuid(90_000 + fixture_offset));
    let scale_problems = vec![ProblemVersionRef { problem, version }; 51];
    let scale_assignment = AssignmentId::from_uuid(uuid(89_990 + fixture_offset));
    store
        .create_assignment_with_default_policy(
            context,
            fixture.publisher,
            AssignmentRecord {
                id: scale_assignment,
                tenant,
                course_id: course,
                title: "Run summary scale fixture".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Published,
                instructions: question_model::AssignmentInstructions::default(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: fixed_items(scale_problems),
                selection_groups: Vec::new(),
                disclosure_policy: question_model::StudentDisclosurePolicy::default(),
                policies: policies(),
            },
        )
        .await
        .expect("independent scale assignment");
    let scale_run = store
        .start_or_resume_run(
            context,
            student_user,
            StudentWorkRoutingBinding::new(course, scale_assignment),
            scale_run_id,
        )
        .await
        .expect("post-completion scale practice run");
    let issued_question_snapshot = learning_data_access::IssuedQuestionSnapshotV1::new(
        store
            .get_catalog_problem(context, ProblemVersionRef { problem, version })
            .await
            .expect("scale fixture catalog question")
            .expect("scale fixture publication")
            .question,
        learning_data_access::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("scale fixture issued question snapshot");
    for position in 0_u32..51 {
        let attempt_id =
            QuestionAttemptId::from_uuid(uuid(90_100 + fixture_offset + u128::from(position)));
        let (presentation, snapshot) =
            receipt_presentation(version, 90_100 + u64::from(position), position as u8);
        store
            .issue_or_resume_question_attempt(
                context,
                IssueQuestionAttemptCommand {
                    actor: student_user,
                    binding: StudentWorkRoutingBinding::new(course, scale_assignment),
                    attempt: attempt_id,
                    run: scale_run.id,
                    assignment_position: position,
                    problem,
                    question_version: version,
                    issued_question_snapshot: issued_question_snapshot.clone(),
                    seed: 90_100 + u64::from(position),
                    presentation_capability: PresentationCapability::EnvelopeV1,
                    presentation: Some(presentation),
                    presentation_snapshot: Some(snapshot),
                    grading_envelope: Some(grading_envelope(version, 90_100 + u64::from(position))),
                    native_execution_envelope_capability:
                        learning_data_access::NativeExecutionEnvelopeCapability::Required,
                    flat_grading: None,
                    flat_grading_capability: FlatGradingCapability::NotApplicable,
                    webwork_grading: None,
                    webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                    qti_grading: None,
                    qti_grading_capability:
                        learning_data_access::QtiGradingCapability::NotApplicable,
                    parameter_hash: format!("scale-parameter-{position}"),
                    provenance: AttemptProvenance {
                        adapter: implementation("native"),
                        renderer: None,
                        generator: None,
                        source_artifact: None,
                        asset_objects: Vec::new(),
                        grading: implementation("numeric"),
                        rendered_question_sha256: format!("scale-rendered-{position}"),
                    },
                    webwork_replay: None,
                    prefetched: None,
                    predecessor_submission: None,
                },
            )
            .await
            .expect("issued scale attempt with effective-policy receipt");
        store
            .submit_question_attempt(
                context,
                SubmitQuestionAttemptCommand {
                    actor: student_user,
                    binding: StudentWorkRoutingBinding::new(course, scale_assignment),
                    attempt: attempt_id,
                    response: StudentResponse::Numeric { value: 1.0 },
                    result: AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    },
                    feedback: FeedbackContent::default(),
                    idempotency_key: SubmissionIdempotencyKey::parse(format!(
                        "scale-submission-{fixture_offset}-{position}"
                    ))
                    .expect("valid scale idempotency key"),
                },
            )
            .await
            .expect("submitted scale attempt");
    }
    let mut cursor = None;
    let mut positions = Vec::new();
    let mut first_scale_cursor = None;
    loop {
        let request = match cursor {
            Some(cursor) => PageRequest::after(cursor, PageSize::new(7).expect("bounded page")),
            None => PageRequest::first(PageSize::new(7).expect("bounded page")),
        };
        let page = store
            .get_run_summary_page(context, student_user, scale_run.id, request)
            .await
            .expect("scale summary page");
        assert!(page.outcomes.items.len() <= 7, "every page stays bounded");
        positions.extend(
            page.outcomes
                .items
                .iter()
                .map(|outcome| outcome.assignment_position),
        );
        if first_scale_cursor.is_none() {
            first_scale_cursor = page.outcomes.next_cursor.clone();
        }
        cursor = page.outcomes.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(positions, (0_u32..51).collect::<Vec<_>>());
    let scale_cursor = first_scale_cursor.expect("first scale page has continuation");
    assert!(matches!(
        store
            .get_run_summary_page(
                context,
                student_user,
                run.id,
                PageRequest::after(
                    scale_cursor.clone(),
                    PageSize::new(7).expect("bounded page")
                ),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let mut tampered = scale_cursor.as_str().as_bytes().to_vec();
    tampered[10] = if tampered[10] == b'A' { b'B' } else { b'A' };
    assert!(matches!(
        store
            .get_run_summary_page(
                context,
                student_user,
                scale_run.id,
                PageRequest::after(
                    Cursor::parse(String::from_utf8(tampered).expect("ASCII cursor"))
                        .expect("nonempty cursor"),
                    PageSize::new(7).expect("bounded page"),
                ),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}
