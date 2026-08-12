use super::*;

pub(super) async fn exercise_delete_and_regrade<S>(store: &S, fixture: &RunApiFixture)
where
    S: Store + CatalogStore + JobStore + AssignmentScoringWorkerStore,
{
    let fixture_offset = fixture.fixture_offset;
    let tenant = fixture.tenant;
    let context = fixture.context;
    let publisher = fixture.publisher;
    let student_user = fixture.student_user;
    let course = fixture.course;
    let problem = fixture.problem;
    let version = fixture.version;
    let reservation = &fixture.reservation;
    let response = &fixture.response;
    let delete_assignment = AssignmentId::from_uuid(uuid(89_960 + fixture_offset));
    let delete_enrollment = EnrollmentId::from_uuid(uuid(89_961 + fixture_offset));
    let delete_run_id = RunId::from_uuid(uuid(89_962 + fixture_offset));
    let delete_items = fixed_items(vec![
        ProblemVersionRef { problem, version },
        ProblemVersionRef { problem, version },
    ]);
    let retired_item = delete_items[0].id;
    let retained_item = delete_items[1].id;
    store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: delete_assignment,
                tenant,
                course_id: course,
                title: "Delete and Regrade fixture".to_string(),
                items: delete_items,
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("Delete and Regrade assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: delete_enrollment,
                tenant,
                assignment: delete_assignment,
                user: student_user,
                student: StudentId::from_uuid(uuid(89_963 + fixture_offset)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("Delete and Regrade enrollment");
    let delete_run = store
        .start_or_resume_run(context, student_user, delete_assignment, delete_run_id)
        .await
        .expect("Delete and Regrade run");
    let affected_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(89_964 + fixture_offset)),
                run: delete_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: 997,
                presentation: Some(presentation_binding(4)),
                parameter_hash: "delete-and-regrade-active".to_string(),
                provenance: reservation.provenance.clone(),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("affected active attempt");
    let before_delete = store
        .get_assignment_for_edit(context, delete_assignment)
        .await
        .expect("Delete and Regrade edit read")
        .expect("Delete and Regrade assignment exists");
    let delete_command = DeleteAndRegradeAssignmentItemCommand {
        course,
        assignment: delete_assignment,
        item: retired_item,
        expected_revision: before_delete.revision,
    };
    assert_eq!(
        store
            .delete_and_regrade_assignment_item(context, delete_command)
            .await,
        Err(StoreError::Conflict),
        "an affected in-progress attempt blocks retirement"
    );
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: affected_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse(
                    "submission-delete-and-regrade-affected",
                )
                .expect("valid Delete and Regrade key"),
            },
        )
        .await
        .expect("submitted evidence permits retirement");
    let retired = store
        .delete_and_regrade_assignment_item(context, delete_command)
        .await
        .expect("Delete and Regrade after submission");
    let retired_record = retired
        .record
        .items
        .iter()
        .find(|item| item.id == retired_item)
        .expect("retired item remains a tombstone");
    assert_eq!(
        (
            retired_record.delivery_state,
            retired_record.scoring_mode,
            retired.scoring_status
        ),
        (
            AssignmentDeliveryState::Retired,
            AssignmentScoringMode::Excluded,
            question_model::ScoringStatus::Recalculating
        )
    );
    assert_eq!(
        store
            .delete_and_regrade_assignment_item(
                context,
                DeleteAndRegradeAssignmentItemCommand {
                    expected_revision: retired.revision,
                    ..delete_command
                },
            )
            .await,
        Ok(retired.clone()),
        "an exact retry does not create another revision or generation"
    );
    let delete_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("Delete and Regrade lease"),
        )
        .await
        .expect("claim Delete and Regrade scoring job")
        .expect("Delete and Regrade queues scoring work");
    let delete_generation = match delete_job.payload {
        JobPayload::RecalculateAssignment {
            assignment: queued_assignment,
            generation,
        } => {
            assert_eq!(queued_assignment, delete_assignment);
            generation
        }
        payload => panic!("expected Delete and Regrade scoring job, got {payload:?}"),
    };
    let delete_scoring = AssignmentScoringWorkerCommand {
        job: delete_job.id,
        lease: delete_job.lease_token,
        assignment: delete_assignment,
        generation: delete_generation,
    };
    store
        .prepare_assignment_scoring(context, delete_scoring)
        .await
        .expect("Delete and Regrade scoring stages");
    assert_eq!(
        store
            .commit_assignment_scoring(context, delete_scoring)
            .await,
        Ok(AssignmentScoringCommitOutcome::Committed)
    );
    assert!(
        store
            .get_run_summary_page(
                context,
                student_user,
                delete_run.id,
                PageRequest::first(PageSize::new(10).expect("Delete and Regrade page")),
            )
            .await
            .expect("student Delete and Regrade summary")
            .outcomes
            .items
            .is_empty(),
        "normal student feedback hides retired evidence"
    );
    assert_eq!(
        store
            .get_run_summary_page(
                context,
                publisher,
                delete_run.id,
                PageRequest::first(PageSize::new(10).expect("support evidence page")),
            )
            .await
            .expect("instructor retained-evidence summary")
            .outcomes
            .items
            .len(),
        1,
        "authorized instructors retain support access to protected evidence"
    );
    let unaffected_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(89_965 + fixture_offset)),
                run: delete_run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                seed: 998,
                presentation: Some(presentation_binding(5)),
                parameter_hash: "delete-and-regrade-unaffected".to_string(),
                provenance: reservation.provenance.clone(),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("unaffected immutable run item remains answerable");
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: unaffected_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse(
                    "submission-delete-and-regrade-unaffected",
                )
                .expect("valid unaffected key"),
            },
        )
        .await
        .expect("existing run completes with retired evidence excluded");
    let future_delete_run = store
        .start_or_resume_run(
            context,
            student_user,
            delete_assignment,
            RunId::from_uuid(uuid(89_966 + fixture_offset)),
        )
        .await
        .expect("future run after Delete and Regrade");
    assert_eq!(
        store
            .assignment_run_items(context, future_delete_run.id)
            .await
            .expect("future Delete and Regrade run items")
            .iter()
            .map(|item| item.assignment_item)
            .collect::<Vec<_>>(),
        vec![retained_item],
        "future runs omit the tombstone while old evidence remains"
    );
}
