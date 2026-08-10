use super::*;

#[tokio::test]
async fn memory_assignment_timing_edits_and_auto_submit_are_generation_fenced() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(uuid(95_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(95_001));
    let student = UserId::from_uuid(uuid(95_002));
    let course = CourseId::from_uuid(uuid(95_003));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Mutable timing course".to_string(),
                members: vec![
                    CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: student,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("timing course");
    let reference = publish_assignment_version(
        &store,
        context,
        tenant,
        instructor,
        95_010,
        PublicationScope::Public,
    )
    .await;
    let assignment = AssignmentId::from_uuid(uuid(95_020));
    let initial = store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Server-owned deadlines".to_string(),
                items: fixed_items(vec![reference, reference, reference]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("timing assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: EnrollmentId::from_uuid(uuid(95_021)),
                tenant,
                assignment,
                user: student,
                student: StudentId::from_uuid(uuid(95_022)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("timing enrollment");

    let ten_seconds = AssignmentTimingPolicy {
        time_limit_seconds: Some(10),
        ..AssignmentTimingPolicy::default()
    };
    let initial_command = UpdateAssignmentTimingCommand {
        actor: instructor,
        course,
        assignment,
        expected_revision: initial.revision,
        policy: ten_seconds,
    };
    assert_eq!(
        store
            .update_assignment_timing(
                context,
                UpdateAssignmentTimingCommand {
                    actor: student,
                    ..initial_command
                },
            )
            .await,
        Err(StoreError::NotFound),
        "students cannot mutate the server timing policy"
    );
    let timed = store
        .update_assignment_timing(context, initial_command)
        .await
        .expect("initial time limit");
    assert_eq!(
        store
            .update_assignment_timing(context, initial_command)
            .await,
        Ok(timed),
        "an exact retry neither increments the revision nor duplicates work"
    );
    let run = store
        .start_or_resume_run(context, student, assignment, RunId::from_uuid(uuid(95_023)))
        .await
        .expect("timed run");
    let issue = |attempt, position, seed| IssueQuestionAttemptCommand {
        actor: student,
        attempt,
        run: run.id,
        assignment_position: position,
        problem: reference.problem,
        question_version: reference.version,
        seed,
        presentation: Some(presentation_binding(position as u8)),
        parameter_hash: format!("timing-parameters-{position}"),
        provenance: AttemptProvenance {
            adapter: implementation("timing-native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("timing-numeric"),
            rendered_question_sha256: format!("timing-render-{position}"),
        },
        webwork_replay: None,
        prefetched: None,
        predecessor_submission: None,
    };
    let first = store
        .issue_or_resume_question_attempt(
            context,
            issue(QuestionAttemptId::from_uuid(uuid(95_024)), 0, 1),
        )
        .await
        .expect("first timed question");
    assert_eq!(
        first.timer.deadline,
        Some(ActivityTimestamp::from_unix_millis(11_000))
    );
    assert!(
        store
            .claim_next_job(
                &JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).expect("lease")
            )
            .await
            .expect("queue read")
            .is_none(),
        "a deadline job is not claimable early"
    );

    let twenty_seconds = AssignmentTimingPolicy {
        time_limit_seconds: Some(20),
        ..AssignmentTimingPolicy::default()
    };
    let extended = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: timed.revision,
                policy: twenty_seconds,
                ..initial_command
            },
        )
        .await
        .expect("active extension");
    assert_eq!(
        store
            .get_question_attempt(context, first.id)
            .await
            .expect("extended attempt read")
            .expect("extended attempt")
            .timer
            .deadline,
        Some(ActivityTimestamp::from_unix_millis(21_000))
    );
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(15_000))
        .expect("advance past shortened limit");
    let shortened = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: extended.revision,
                policy: ten_seconds,
                ..initial_command
            },
        )
        .await
        .expect("shortening is an immediate transaction");
    let current = store
        .get_question_attempt(context, first.id)
        .await
        .expect("shortened attempt read")
        .expect("shortened attempt");
    assert_eq!(current.status, AttemptStatus::AutoSubmitted);
    assert!(current.response.is_none());
    assert!(current.result.is_none());
    assert_eq!(
        current.timer.submitted_at,
        Some(ActivityTimestamp::from_unix_millis(15_000))
    );

    let closes_at = |millis| AssignmentTimingPolicy {
        closes_at: Some(ActivityTimestamp::from_unix_millis(millis)),
        late_submission: LateSubmissionPolicy::Accept,
        ..AssignmentTimingPolicy::default()
    };
    let closes_sixteen = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: shortened.revision,
                policy: closes_at(16_000),
                ..initial_command
            },
        )
        .await
        .expect("move the next question to a close boundary");
    let second = store
        .issue_or_resume_question_attempt(
            context,
            issue(QuestionAttemptId::from_uuid(uuid(95_025)), 1, 2),
        )
        .await
        .expect("second timed question");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(16_000))
        .expect("reach close boundary");
    let due = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("due queue read")
        .expect("deadline job is due");
    let timing_generation = match due.payload {
        JobPayload::AutoSubmitAttempt {
            attempt,
            timing_generation,
        } => {
            assert_eq!(attempt, second.id);
            timing_generation
        }
        payload => panic!("expected attempt auto-submit, got {payload:?}"),
    };
    assert_eq!(
        store
            .commit_attempt_auto_submit(
                context,
                AttemptAutoSubmitWorkerCommand {
                    job: due.id,
                    lease: due.lease_token,
                    attempt: second.id,
                    timing_generation,
                },
            )
            .await,
        Ok(AttemptAutoSubmitCommitOutcome::AutoSubmitted)
    );
    assert_eq!(
        store
            .submit_question_attempt(
                context,
                SubmitQuestionAttemptCommand {
                    actor: student,
                    attempt: second.id,
                    response: StudentResponse::Numeric { value: 18.0 },
                    result: AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    },
                    feedback: FeedbackContent::default(),
                    idempotency_key: SubmissionIdempotencyKey::parse("after-auto-submit")
                        .expect("submission key"),
                },
            )
            .await,
        Err(StoreError::Conflict)
    );

    let closes_seventeen = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: closes_sixteen.revision,
                policy: closes_at(17_000),
                ..initial_command
            },
        )
        .await
        .expect("open a third bounded question");
    let third = store
        .issue_or_resume_question_attempt(
            context,
            issue(QuestionAttemptId::from_uuid(uuid(95_026)), 2, 3),
        )
        .await
        .expect("third timed question");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(17_000))
        .expect("reach original third deadline");
    let stale = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("stale queue read")
        .expect("third deadline job");
    let stale_generation = match stale.payload {
        JobPayload::AutoSubmitAttempt {
            attempt,
            timing_generation,
        } => {
            assert_eq!(attempt, third.id);
            timing_generation
        }
        payload => panic!("expected attempt auto-submit, got {payload:?}"),
    };
    store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: closes_seventeen.revision,
                policy: closes_at(20_000),
                ..initial_command
            },
        )
        .await
        .expect("extension races safely with a leased old generation");
    assert_eq!(
        store
            .commit_attempt_auto_submit(
                context,
                AttemptAutoSubmitWorkerCommand {
                    job: stale.id,
                    lease: stale.lease_token,
                    attempt: third.id,
                    timing_generation: stale_generation,
                },
            )
            .await,
        Ok(AttemptAutoSubmitCommitOutcome::Rescheduled)
    );
    assert_eq!(
        store
            .get_question_attempt(context, third.id)
            .await
            .expect("extended third read")
            .expect("extended third")
            .status,
        AttemptStatus::InProgress
    );
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(20_000))
        .expect("reach extended deadline");
    let current = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("current queue read")
        .expect("rescheduled job is due");
    assert_eq!(current.id, stale.id, "the extension reuses the durable job");
    let current_generation = match current.payload {
        JobPayload::AutoSubmitAttempt {
            attempt,
            timing_generation,
        } => {
            assert_eq!(attempt, third.id);
            timing_generation
        }
        payload => panic!("expected attempt auto-submit, got {payload:?}"),
    };
    assert!(current_generation > stale_generation);
    assert_eq!(
        store
            .commit_attempt_auto_submit(
                context,
                AttemptAutoSubmitWorkerCommand {
                    job: current.id,
                    lease: current.lease_token,
                    attempt: third.id,
                    timing_generation: current_generation,
                },
            )
            .await,
        Ok(AttemptAutoSubmitCommitOutcome::AutoSubmitted)
    );

    let limited_assignment = AssignmentId::from_uuid(uuid(95_030));
    let limited = store
        .create_assignment(
            context,
            AssignmentRecord {
                id: limited_assignment,
                tenant,
                course_id: course,
                title: "One allowed run".to_string(),
                items: fixed_items(vec![reference]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("attempt-limited assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: EnrollmentId::from_uuid(uuid(95_031)),
                tenant,
                assignment: limited_assignment,
                user: student,
                student: StudentId::from_uuid(uuid(95_032)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("attempt-limited enrollment");
    store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                actor: instructor,
                course,
                assignment: limited_assignment,
                expected_revision: limited.revision,
                policy: AssignmentTimingPolicy {
                    attempt_limit: Some(1),
                    ..AssignmentTimingPolicy::default()
                },
            },
        )
        .await
        .expect("one-run limit");
    let limited_run = store
        .start_or_resume_run(
            context,
            student,
            limited_assignment,
            RunId::from_uuid(uuid(95_033)),
        )
        .await
        .expect("first allowed run");
    let limited_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: QuestionAttemptId::from_uuid(uuid(95_034)),
                run: limited_run.id,
                assignment_position: 0,
                problem: reference.problem,
                question_version: reference.version,
                seed: 4,
                presentation: Some(presentation_binding(12)),
                parameter_hash: "attempt-limit-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("timing-native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("timing-numeric"),
                    rendered_question_sha256: "attempt-limit-render".to_string(),
                },
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("question in the first allowed run");
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: limited_attempt.id,
                response: StudentResponse::Numeric { value: 18.0 },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("finish-limited-run")
                    .expect("submission key"),
            },
        )
        .await
        .expect("complete the only allowed run");
    assert!(matches!(
        store
            .start_or_resume_run(
                context,
                student,
                limited_assignment,
                RunId::from_uuid(uuid(95_035)),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}
