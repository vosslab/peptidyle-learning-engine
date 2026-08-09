use super::external_tool::external_tool_fixture;
use super::*;

#[tokio::test]
async fn memory_manual_grading_is_response_bearing_revisioned_and_generation_fenced() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
        .expect("manual grading fixture clock");
    let fixture = external_tool_fixture(&store).await;
    let instructor = UserId::from_uuid(uuid(10_015));
    let pending_command = SubmitPendingManualQuestionAttemptCommand {
        actor: fixture.actor,
        attempt: fixture.attempt,
        response: StudentResponse::ExternalTool {},
        idempotency_key: SubmissionIdempotencyKey::parse("manual-pending-response")
            .expect("valid pending key"),
    };
    let pending = store
        .submit_pending_manual_question_attempt(fixture.context, pending_command.clone())
        .await
        .expect("response-bearing manual submission commits");
    assert_eq!(pending.attempt.status, AttemptStatus::NeedsManualGrading);
    assert_eq!(pending.attempt.result, None);
    assert_eq!(pending.run.score, None);
    assert_eq!(pending.summary.current_score, None);
    assert_eq!(
        store
            .submit_pending_manual_question_attempt(fixture.context, pending_command)
            .await,
        Ok(pending.clone()),
        "an exact pending response retry returns its first receipt"
    );
    assert_eq!(
        store
            .get_manual_evaluation_for_edit(fixture.context, fixture.actor, fixture.attempt)
            .await,
        Err(StoreError::NotFound),
        "a student cannot enumerate a gradeable evaluation"
    );
    assert_eq!(
        store
            .get_manual_evaluation_for_edit(fixture.foreign_context, instructor, fixture.attempt)
            .await,
        Err(StoreError::NotFound),
        "a foreign tenant cannot enumerate a gradeable evaluation"
    );
    let pending_evaluation = store
        .get_manual_evaluation_for_edit(fixture.context, instructor, fixture.attempt)
        .await
        .expect("instructor evaluation read")
        .expect("submitted response creates a pending evaluation");
    assert_eq!(pending_evaluation.revision, EvaluationRevision::INITIAL);
    assert_eq!(pending_evaluation.credit, None);

    let first_command = SetManualGradeCommand {
        action: ManualGradeActionId::from_uuid(uuid(10_101)),
        actor: instructor,
        attempt: fixture.attempt,
        expected_revision: pending_evaluation.revision,
        credit: ManualCredit::parse("1.000").expect("valid manual credit"),
    };
    let first = store
        .set_manual_grade(fixture.context, first_command.clone())
        .await
        .expect("direct instructor grades submitted evidence");
    assert_eq!(first.attempt, fixture.attempt);
    assert_eq!(first.resulting_revision.as_u64(), 2);
    assert!(
        !format!("{first:?}").contains("credit"),
        "the replay receipt must not expose a grade value"
    );
    let pending_summary = store
        .get_summary(fixture.context, pending.run.enrollment)
        .await
        .expect("pending summary read")
        .expect("pending summary exists");
    let pending_enrollment = store
        .get_enrollment(fixture.context, pending.run.enrollment)
        .await
        .expect("pending enrollment read")
        .expect("pending enrollment exists");
    assert_eq!(pending_summary.current_score, None);
    assert_eq!(pending_summary.completed_run_count, 0);
    assert_eq!(pending_enrollment.first_completed_at, None);
    assert_eq!(pending_enrollment.current_grade_run, None);
    assert_eq!(
        store
            .get_assignment_for_edit(fixture.context, pending_enrollment.assignment)
            .await
            .expect("pending assignment read")
            .expect("pending assignment exists")
            .scoring_status,
        question_model::ScoringStatus::Recalculating,
        "manual completion remains pending until its scoring generation commits"
    );
    assert_eq!(
        store
            .set_manual_grade(fixture.context, first_command.clone())
            .await,
        Ok(first),
        "an exact action replay returns its original minimal receipt"
    );
    let mut changed_replay = first_command.clone();
    changed_replay.credit = ManualCredit::parse("0.5").expect("valid changed credit");
    assert_eq!(
        store
            .set_manual_grade(fixture.context, changed_replay)
            .await,
        Err(StoreError::Conflict),
        "an action identity cannot be reused for another grade"
    );
    assert_eq!(
        store
            .set_manual_grade(
                fixture.context,
                SetManualGradeCommand {
                    action: ManualGradeActionId::from_uuid(uuid(10_102)),
                    actor: instructor,
                    attempt: fixture.attempt,
                    expected_revision: EvaluationRevision::INITIAL,
                    credit: ManualCredit::parse("0.5").expect("valid stale credit"),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a stale evaluation revision cannot overwrite the current grade"
    );
    let first_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("manual scoring lease"),
        )
        .await
        .expect("manual scoring job claim")
        .expect("manual grade queues scoring work");
    let JobPayload::RecalculateAssignment {
        assignment,
        generation,
    } = first_job.payload
    else {
        panic!("manual grade must queue assignment scoring");
    };
    assert_eq!(generation, first.scoring_generation);
    let first_worker = AssignmentScoringWorkerCommand {
        job: first_job.id,
        lease: first_job.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(fixture.context, first_worker)
        .await
        .expect("manual generation stages privately");

    let corrected = store
        .set_manual_grade(
            fixture.context,
            SetManualGradeCommand {
                action: ManualGradeActionId::from_uuid(uuid(10_103)),
                actor: instructor,
                attempt: fixture.attempt,
                expected_revision: first.resulting_revision,
                credit: ManualCredit::parse("0.123456789012").expect("exact corrected credit"),
            },
        )
        .await
        .expect("current revision replaces the current manual result");
    assert_eq!(corrected.resulting_revision.as_u64(), 3);
    let current = store
        .get_manual_evaluation_for_edit(fixture.context, instructor, fixture.attempt)
        .await
        .expect("current evaluation read")
        .expect("current evaluation exists");
    assert_eq!(
        current
            .credit
            .as_ref()
            .expect("graded current evaluation has exact credit")
            .as_canonical_decimal(),
        "0.123456789012"
    );
    let corrected_run = store
        .get_run(fixture.context, pending.run.id)
        .await
        .expect("corrected run read")
        .expect("corrected run exists");
    let corrected_score = corrected_run
        .score
        .expect("completed run has current score");
    assert!(
        (corrected_score - 0.1235).abs() < 1e-12,
        "a manual correction replaces the current run score before worker publication: {corrected_score}"
    );
    assert_eq!(
        store
            .get_summary(fixture.context, pending.run.enrollment)
            .await
            .expect("still-pending summary read")
            .expect("still-pending summary exists")
            .current_score,
        None,
        "the corrected run score does not publish the assignment summary early"
    );
    assert_eq!(
        store.set_manual_grade(fixture.context, first_command).await,
        Ok(first),
        "later corrections cannot change an earlier action receipt"
    );
    assert_eq!(
        store
            .commit_assignment_scoring(fixture.context, first_worker)
            .await,
        Ok(AssignmentScoringCommitOutcome::Superseded),
        "a prepared generation cannot publish after a manual correction"
    );
    let current_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("corrected scoring lease"),
        )
        .await
        .expect("corrected scoring job claim")
        .expect("manual correction queues a replacement generation");
    let JobPayload::RecalculateAssignment {
        assignment,
        generation,
    } = current_job.payload
    else {
        panic!("manual correction must queue assignment scoring");
    };
    let current_worker = AssignmentScoringWorkerCommand {
        job: current_job.id,
        lease: current_job.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(fixture.context, current_worker)
        .await
        .expect("corrected generation stages");
    assert_eq!(
        store
            .commit_assignment_scoring(fixture.context, current_worker)
            .await,
        Ok(AssignmentScoringCommitOutcome::Committed),
        "the current manual result publishes only through the scoring worker"
    );
    let published_summary = store
        .get_summary(fixture.context, pending.run.enrollment)
        .await
        .expect("published summary read")
        .expect("published summary exists");
    let published_enrollment = store
        .get_enrollment(fixture.context, pending.run.enrollment)
        .await
        .expect("published enrollment read")
        .expect("published enrollment exists");
    assert_eq!(published_summary.completed_run_count, 1);
    assert_eq!(
        published_enrollment.first_completed_at,
        Some(ActivityTimestamp::from_unix_millis(500))
    );
    assert_eq!(published_enrollment.current_grade_run, Some(pending.run.id));
    assert_eq!(published_enrollment.best_grade_run, Some(pending.run.id));

    let force_store = MemoryStore::default();
    force_store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
        .expect("force-submit fixture clock");
    let force_fixture = external_tool_fixture(&force_store).await;
    force_store
        .force_submit_attempt(
            force_fixture.context,
            ForceSubmitAttemptCommand {
                action: AttemptSupportActionId::from_uuid(uuid(10_104)),
                actor: instructor,
                attempt: force_fixture.attempt,
            },
        )
        .await
        .expect("instructor force-submit");
    assert_eq!(
        force_store
            .set_manual_grade(
                force_fixture.context,
                SetManualGradeCommand {
                    action: ManualGradeActionId::from_uuid(uuid(10_105)),
                    actor: instructor,
                    attempt: force_fixture.attempt,
                    expected_revision: EvaluationRevision::INITIAL,
                    credit: ManualCredit::parse("1").expect("valid credit"),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a force-submit without learner response evidence remains ungradeable"
    );
}
