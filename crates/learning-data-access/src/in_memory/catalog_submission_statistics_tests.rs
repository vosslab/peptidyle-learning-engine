use super::catalog_search_tests::{
    StatisticsSubmissionFixture, insert_statistics_issued_authority, record, statistics_attempt,
    submit_statistics_attempt,
};
use super::*;
use question_model::{AssignmentRun, AttemptResult, StudentId};

#[test]
fn first_assigned_completion_records_collapsed_statistics_once() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(72_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let actor = UserId::from_uuid(Uuid::from_u128(72_002));
    let assignment_id = AssignmentId::from_uuid(Uuid::from_u128(72_003));
    let enrollment_id = EnrollmentId::from_uuid(Uuid::from_u128(72_004));
    let assigned_run = RunId::from_uuid(Uuid::from_u128(72_005));
    let mut published_a = record(72_010);
    let mut published_b = record(72_011);
    published_a.scope = PublicationScope::Public;
    published_b.scope = PublicationScope::Public;
    let a = ProblemVersionRef {
        problem: published_a.problem,
        version: published_a.version,
    };
    let b = ProblemVersionRef {
        problem: published_b.problem,
        version: published_b.version,
    };
    let issued_snapshot_a = crate::IssuedQuestionSnapshotV1::new(
        published_a.question.clone(),
        crate::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("statistics fixture issued snapshot A");
    let issued_snapshot_b = crate::IssuedQuestionSnapshotV1::new(
        published_b.question.clone(),
        crate::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("statistics fixture issued snapshot B");
    let assignment = AssignmentRecord {
        id: assignment_id,
        tenant,
        course_id: CourseId::from_uuid(Uuid::from_u128(72_006)),
        title: "Statistics completion fixture".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Draft,
        instructions: question_model::AssignmentInstructions::default(),
        audience: question_model::AssignmentAudience::CourseWide,
        items: [a, b, a]
            .into_iter()
            .enumerate()
            .map(|(position, reference)| question_model::AssignmentItem {
                id: question_model::AssignmentItemId::from_uuid(Uuid::from_u128(
                    72_100 + position as u128,
                )),
                reference,
                position: u32::try_from(position).expect("fixture position fits"),
                points_possible: question_model::PointValue::from_whole(1),
                delivery_state: question_model::AssignmentDeliveryState::Active,
                scoring_mode: question_model::AssignmentScoringMode::Normal,
            })
            .collect(),
        selection_groups: Vec::new(),
        disclosure_policy: question_model::StudentDisclosurePolicy::default(),
        policies: question_model::RunPolicies {
            completion: question_model::CompletionRequirement::AnswerAll,
            grade: question_model::GradePolicy::First,
            continued_practice: question_model::ContinuedPractice::Unlimited,
            variation: question_model::VariationPolicy::NewSeeds,
        },
    };
    let enrollment = AssignmentEnrollment {
        id: enrollment_id,
        tenant,
        assignment: assignment_id,
        user: actor,
        student: StudentId::from_uuid(Uuid::from_u128(72_007)),
        first_completed_at: None,
        current_grade_run: None,
        best_grade_run: None,
    };
    let run = AssignmentRun {
        id: assigned_run,
        reference: question_model::RunReference::new(1).expect("valid run reference"),
        tenant,
        enrollment: enrollment_id,
        run_number: 1,
        started_at: ActivityTimestamp::from_unix_millis(0),
        completed_at: None,
        score: None,
        mode: RunMode::Assigned,
        variation: question_model::VariationPolicy::NewSeeds,
    };
    let run_items =
        select_assignment_run_items(&assignment, &run).expect("statistics fixture run items");
    {
        let mut state = store.write_state().expect("statistics fixture state");
        state.courses.insert(
            (tenant, assignment.course_id),
            CourseRecord {
                id: assignment.course_id,
                tenant,
                title: "Statistics fixture course".to_string(),
                term: question_model::CourseTerm::from_parts(
                    "2026-08-24",
                    "2026-12-18",
                    "America/Chicago",
                )
                .expect("explicit fixture course term"),
            },
        );
        let membership = CourseMembershipId::from_uuid(Uuid::from_u128(72_008));
        state.course_memberships.insert(
            (tenant, membership),
            CourseMembershipRecord {
                id: membership,
                tenant,
                course: assignment.course_id,
                user: actor,
                student: Some(StudentId::from_uuid(Uuid::from_u128(72_007))),
                role: CourseMembershipRole::Student,
                roster_id: None,
                status: CourseMemberStatus::Active,
                joined_at: ActivityTimestamp::from_unix_millis(0),
                revoked_at: None,
            },
        );
        state
            .active_course_membership_by_user
            .insert((tenant, assignment.course_id, actor), membership);
        state
            .published
            .insert((published_a.problem, published_a.version), published_a);
        state
            .published
            .insert((published_b.problem, published_b.version), published_b);
        state
            .assignments
            .insert((tenant, assignment_id), assignment);
        state.assignment_scoring.insert(
            (tenant, assignment_id),
            (ScoringGeneration::INITIAL, ScoringStatus::Current),
        );
        state
            .enrollments
            .insert((tenant, enrollment_id), enrollment);
        state.runs.insert((tenant, assigned_run), run);
        state.run_items.insert((tenant, assigned_run), run_items);
        state.summaries.insert(
            (tenant, enrollment_id),
            StudentAssignmentSummary::empty(tenant, enrollment_id),
        );
        for (number, snapshot) in [
            (72_098, issued_snapshot_a.clone()),
            (72_099, issued_snapshot_a.clone()),
            (72_100, issued_snapshot_a.clone()),
            (72_101, issued_snapshot_a.clone()),
            (72_102, issued_snapshot_b.clone()),
            (72_103, issued_snapshot_a.clone()),
            (72_201, issued_snapshot_a.clone()),
            (72_202, issued_snapshot_b.clone()),
            (72_203, issued_snapshot_a.clone()),
        ] {
            state.attempt_issued_question_snapshots.insert(
                (
                    tenant,
                    QuestionAttemptId::from_uuid(Uuid::from_u128(number)),
                ),
                snapshot,
            );
        }
    }

    let regressive = statistics_attempt(72_099, tenant, assigned_run, a, 0, 2_000);
    let binding =
        StudentWorkRoutingBinding::new(CourseId::from_uuid(Uuid::from_u128(72_006)), assignment_id);
    let regressive_command = SubmitQuestionAttemptCommand {
        actor,
        binding,
        attempt: regressive.id,
        response: StudentResponse::Numeric { value: 0.0 },
        result: AttemptResult {
            correct: false,
            points_earned: 0.0,
            points_possible: 2.0,
        },
        feedback: question_model::FeedbackContent::default(),
        idempotency_key: SubmissionIdempotencyKey::parse("statistics-regressive-time")
            .expect("valid fixture idempotency key"),
    };
    let missing_timing = statistics_attempt(72_098, tenant, assigned_run, a, 0, 1_000);
    {
        let mut state = store.write_state().expect("missing timing fixture state");
        state.authoritative_time = ActivityTimestamp::from_unix_millis(1_500);
        insert_statistics_issued_authority(&mut state, &missing_timing);
        state.attempt_timing.remove(&(tenant, missing_timing.id));
        state
            .attempts
            .insert((tenant, missing_timing.id), missing_timing.clone());
        assert!(matches!(
            submit_question_attempt_locked(
                &mut state,
                context,
                SubmitQuestionAttemptCommand {
                    binding,
                    attempt: missing_timing.id,
                    idempotency_key: SubmissionIdempotencyKey::parse("statistics-missing-timing")
                        .expect("valid missing-timing key"),
                    ..regressive_command.clone()
                },
            ),
            Err(StoreError::Unavailable(_))
        ));
        assert!(
            !state.submissions.contains_key(&(tenant, missing_timing.id)),
            "missing issued timing must fail before receipt mutation"
        );
    }
    {
        let mut state = store.write_state().expect("regressive statistics state");
        state.authoritative_time = ActivityTimestamp::from_unix_millis(1_500);
        state
            .attempts
            .insert((tenant, regressive.id), regressive.clone());
        insert_statistics_issued_authority(&mut state, &regressive);
        assert!(matches!(
            submit_question_attempt_locked(&mut state, context, regressive_command),
            Err(StoreError::InvalidRecord(_))
        ));
        assert!(!state.submissions.contains_key(&(tenant, regressive.id)));
        assert_eq!(
            state.summaries[&(tenant, enrollment_id)],
            StudentAssignmentSummary::empty(tenant, enrollment_id)
        );
        assert_eq!(state.runs[&(tenant, assigned_run)].completed_at, None);
        assert!(state.question_statistics.is_empty());
        assert!(state.question_statistics_receipts.is_empty());
    }

    {
        let mut state = store
            .write_state()
            .expect("withdrawn catalog fixture state");
        state.published.clear();
    }

    // Every first submission below succeeds with only its issued timing and
    // receipt authority. No current catalog policy remains to reconstruct.
    submit_statistics_attempt(
        &store,
        StatisticsSubmissionFixture {
            context,
            actor,
            binding,
            attempt: statistics_attempt(72_100, tenant, assigned_run, a, 0, 0),
        },
        1_500,
        0.0,
        2.0,
    );
    submit_statistics_attempt(
        &store,
        StatisticsSubmissionFixture {
            context,
            actor,
            binding,
            attempt: statistics_attempt(72_101, tenant, assigned_run, a, 0, 2_000),
        },
        4_500,
        1.0,
        2.0,
    );
    submit_statistics_attempt(
        &store,
        StatisticsSubmissionFixture {
            context,
            actor,
            binding,
            attempt: statistics_attempt(72_102, tenant, assigned_run, b, 1, 5_000),
        },
        6_000,
        1.0,
        4.0,
    );
    let (_, final_command) = submit_statistics_attempt(
        &store,
        StatisticsSubmissionFixture {
            context,
            actor,
            binding,
            attempt: statistics_attempt(72_103, tenant, assigned_run, a, 2, 7_000),
        },
        100_007_000,
        2.0,
        2.0,
    );

    let completed_statistics = {
        let state = store.read_state().expect("completed statistics state");
        assert_eq!(state.question_statistics_receipts.len(), 2);
        let a_snapshot = state.question_statistics[&(a.problem, a.version)].snapshot();
        assert_eq!(a_snapshot.cohort_size, 1);
        assert_eq!(a_snapshot.score_sum, 0.0);
        assert_eq!(a_snapshot.attempts_sum, 2);
        assert_eq!(a_snapshot.discrimination.count, 1);
        assert_eq!(a_snapshot.discrimination.mean_x, 0.0);
        assert_eq!(a_snapshot.discrimination.mean_y, 0.25);
        let b_snapshot = state.question_statistics[&(b.problem, b.version)].snapshot();
        assert_eq!(b_snapshot.cohort_size, 1);
        assert_eq!(b_snapshot.score_sum, 0.25);
        assert_eq!(b_snapshot.attempts_sum, 1);
        assert_eq!(b_snapshot.durations.bins[0], 1);
        assert_eq!(b_snapshot.discrimination.mean_x, 0.25);
        assert_eq!(b_snapshot.discrimination.mean_y, 0.0);
        (
            state.question_statistics.clone(),
            state.question_statistics_receipts.clone(),
        )
    };

    {
        let mut state = store.write_state().expect("replay statistics state");
        let replay = submit_question_attempt_locked(&mut state, context, final_command)
            .expect("exact completed submission replay");
        assert_eq!(replay.run.id, assigned_run);
        assert_eq!(state.question_statistics, completed_statistics.0);
        assert_eq!(state.question_statistics_receipts.len(), 2);
    }

    let practice_run = RunId::from_uuid(Uuid::from_u128(72_200));
    {
        let mut state = store.write_state().expect("practice statistics state");
        let practice_items = state.run_items[&(tenant, assigned_run)]
            .iter()
            .cloned()
            .map(|mut item| {
                item.run = practice_run;
                item
            })
            .collect();
        state.runs.insert(
            (tenant, practice_run),
            AssignmentRun {
                id: practice_run,
                reference: question_model::RunReference::new(2).expect("valid run reference"),
                tenant,
                enrollment: enrollment_id,
                run_number: 2,
                started_at: ActivityTimestamp::from_unix_millis(200_000_000),
                completed_at: None,
                score: None,
                mode: RunMode::Practice,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        );
        state
            .run_items
            .insert((tenant, practice_run), practice_items);
    }
    for (number, reference, position, earned, possible) in [
        (72_201, a, 0, 1.0, 2.0),
        (72_202, b, 1, 1.0, 4.0),
        (72_203, a, 2, 2.0, 2.0),
    ] {
        submit_statistics_attempt(
            &store,
            StatisticsSubmissionFixture {
                context,
                actor,
                binding,
                attempt: statistics_attempt(
                    number,
                    tenant,
                    practice_run,
                    reference,
                    position,
                    200_000_000,
                ),
            },
            200_001_000 + i64::from(position),
            earned,
            possible,
        );
    }
    let state = store.read_state().expect("practice completion state");
    assert_eq!(state.question_statistics, completed_statistics.0);
    assert_eq!(
        state.question_statistics_receipts.len(),
        completed_statistics.1.len()
    );
}
