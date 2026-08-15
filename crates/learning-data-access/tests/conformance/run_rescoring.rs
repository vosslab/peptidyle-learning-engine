use super::*;

pub(super) async fn exercise_run_rescoring<S>(store: &S, fixture: &RunApiFixture)
where
    S: Store + CatalogStore + JobStore + AssignmentScoringWorkerStore,
{
    let fixture_offset = fixture.fixture_offset;
    let context = fixture.context;
    let student_user = fixture.student_user;
    let assignment = fixture.assignment;
    let course = fixture.course;
    let problem = fixture.problem;
    let version = fixture.version;
    let reservation = &fixture.reservation;
    let response = &fixture.response;
    let locked_assignment = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("locked assignment read")
        .expect("run assignment exists");
    let mut rescored_items = locked_assignment.record.items.clone();
    rescored_items[0].points_possible = PointValue::from_whole(2);
    let rescored = store
        .replace_assignment_preserving_timing(
            context,
            course,
            assignment,
            locked_assignment.revision,
            AssignmentUpdate {
                title: locked_assignment.record.title.clone(),
                items: rescored_items.clone(),
                selection_groups: locked_assignment.record.selection_groups.clone(),
                policies: locked_assignment.record.policies,
            },
        )
        .await
        .expect("point edits remain valid after a run exists");
    assert_eq!(
        rescored.scoring_status,
        question_model::ScoringStatus::Recalculating
    );
    let scoring_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("scoring lease"),
        )
        .await
        .expect("claim scoring job")
        .expect("point edit queues scoring work");
    let (queued_assignment, generation) = match scoring_job.payload {
        JobPayload::RecalculateAssignment {
            assignment,
            generation,
        } => (assignment, generation),
        payload => panic!("expected scoring job, got {payload:?}"),
    };
    assert_eq!(
        (queued_assignment, generation),
        (assignment, rescored.scoring_generation)
    );
    let scoring_command = AssignmentScoringWorkerCommand {
        job: scoring_job.id,
        lease: scoring_job.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(context, scoring_command)
        .await
        .expect("scoring generation stages privately");
    assert!(matches!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("staged assignment read")
            .expect("staged assignment exists")
            .scoring_status,
        question_model::ScoringStatus::Recalculating
    ));
    let mut superseding_items = rescored.record.items.clone();
    superseding_items[0].points_possible = PointValue::from_whole(3);
    let superseding = store
        .replace_assignment_preserving_timing(
            context,
            course,
            assignment,
            rescored.revision,
            AssignmentUpdate {
                title: rescored.record.title.clone(),
                items: superseding_items.clone(),
                selection_groups: rescored.record.selection_groups.clone(),
                policies: rescored.record.policies,
            },
        )
        .await
        .expect("a newer scoring definition supersedes staged work");
    assert!(superseding.scoring_generation > generation);
    assert!(matches!(
        store
            .commit_assignment_scoring(context, scoring_command)
            .await,
        Ok(AssignmentScoringCommitOutcome::Superseded)
    ));
    let still_pending = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("superseded assignment read")
        .expect("superseded assignment exists");
    assert_eq!(
        (
            still_pending.scoring_generation,
            still_pending.scoring_status
        ),
        (
            superseding.scoring_generation,
            question_model::ScoringStatus::Recalculating
        ),
        "discarding old staging must leave the new generation pending"
    );
    let current_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("current scoring lease"),
        )
        .await
        .expect("claim current scoring job")
        .expect("superseding edit queues scoring work");
    let current_generation = match current_job.payload {
        JobPayload::RecalculateAssignment {
            assignment: queued_assignment,
            generation,
        } => {
            assert_eq!(queued_assignment, assignment);
            generation
        }
        payload => panic!("expected superseding scoring job, got {payload:?}"),
    };
    assert_eq!(current_generation, superseding.scoring_generation);
    let current_command = AssignmentScoringWorkerCommand {
        job: current_job.id,
        lease: current_job.lease_token,
        assignment,
        generation: current_generation,
    };
    store
        .prepare_assignment_scoring(context, current_command)
        .await
        .expect("current scoring generation stages privately");
    let concurrent_run = store
        .start_or_resume_run(
            context,
            student_user,
            assignment,
            RunId::from_uuid(uuid(89_970 + fixture_offset)),
        )
        .await
        .expect("student activity may continue during recalculation");
    let concurrent_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(89_971 + fixture_offset)),
                run: concurrent_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: 996,
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                parameter_hash: "concurrent-scoring-parameter-hash".to_string(),
                provenance: reservation.provenance.clone(),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("concurrent scoring attempt issues");
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: concurrent_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("submission-during-scoring")
                    .expect("valid concurrent scoring key"),
            },
        )
        .await
        .expect("submission may commit while recalculation is pending");
    assert_eq!(
        store
            .commit_assignment_scoring(context, current_command)
            .await,
        Err(StoreError::Conflict),
        "staging prepared before a new submission must not publish an incomplete generation"
    );
    store
        .prepare_assignment_scoring(context, current_command)
        .await
        .expect("same live claim restages after concurrent activity");
    assert_eq!(
        store
            .commit_assignment_scoring(context, current_command)
            .await,
        Ok(AssignmentScoringCommitOutcome::Committed)
    );
    let current_assignment = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("rescored assignment read")
        .expect("rescored assignment exists");
    assert_eq!(
        (
            current_assignment.scoring_generation,
            current_assignment.scoring_status
        ),
        (current_generation, question_model::ScoringStatus::Current)
    );
    let mut added_items = superseding_items;
    let mut added = added_items[0].clone();
    added.id = AssignmentItemId::from_uuid(uuid(89_980 + fixture_offset));
    added.position = u32::try_from(added_items.len()).expect("fixture position fits");
    added_items.push(added);
    assert!(matches!(
        store
            .replace_assignment_preserving_timing(
                context,
                course,
                assignment,
                superseding.revision,
                AssignmentUpdate {
                    title: superseding.record.title.clone(),
                    items: added_items,
                    selection_groups: superseding.record.selection_groups.clone(),
                    policies: superseding.record.policies,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}
