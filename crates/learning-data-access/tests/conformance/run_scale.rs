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
    // the cursor helper: a later practice run may contain far more outcomes
    // than an ordinary small assignment. `apply_activity_transition` supplies
    // persisted, server-owned attempt records without invoking a grader.
    let scale_run_id = RunId::from_uuid(uuid(90_000 + fixture_offset));
    let scale_problems = vec![ProblemVersionRef { problem, version }; 51];
    let scale_assignment = AssignmentId::from_uuid(uuid(89_990 + fixture_offset));
    let scale_enrollment = EnrollmentId::from_uuid(uuid(89_991 + fixture_offset));
    store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: scale_assignment,
                tenant,
                course_id: course,
                title: "Run summary scale fixture".to_string(),
                items: fixed_items(scale_problems),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("independent scale assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: scale_enrollment,
                tenant,
                assignment: scale_assignment,
                user: student_user,
                student: StudentId::from_uuid(uuid(89_992 + fixture_offset)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("independent scale enrollment");
    let scale_run = store
        .start_or_resume_run(context, student_user, scale_assignment, scale_run_id)
        .await
        .expect("post-completion scale practice run");
    for position in 0_u32..51 {
        store
            .apply_activity_transition(
                context,
                ActivityTransition::RecordQuestionAttempt {
                    attempt: Box::new(QuestionAttempt {
                        id: QuestionAttemptId::from_uuid(uuid(
                            90_100 + fixture_offset + u128::from(position),
                        )),
                        tenant,
                        run: scale_run.id,
                        problem,
                        question_version: version,
                        assignment_position: position,
                        seed: u64::from(position),
                        parameter_hash: format!("scale-parameter-{position}"),
                        response: None,
                        status: question_model::AttemptStatus::InProgress,
                        result: None,
                        timer: AttemptTimerRecord {
                            issued_at: ActivityTimestamp::from_unix_millis(i64::from(position)),
                            deadline: None,
                            submitted_at: None,
                        },
                        provenance: AttemptProvenance {
                            adapter: implementation("native"),
                            renderer: None,
                            generator: None,
                            source_artifact: None,
                            asset_objects: Vec::new(),
                            grading: implementation("numeric"),
                            rendered_question_sha256: format!("scale-rendered-{position}"),
                        },
                    }),
                },
            )
            .await
            .expect("persisted scale attempt");
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
