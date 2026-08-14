use super::*;
use crate::Store;
use question_model::answer::NumericTolerance;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure};
use question_model::taxonomy::{License, Tag};
use question_model::{
    AssignmentRun, AttemptProvenance, AttemptResult, AttemptTimerRecord, BackendCapabilities,
    Capability, CourseMembership, CourseMembershipRole, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, ImplementationVersion, QuestionDefinition,
    QuestionMetadata, ResponseDefinition, StudentId,
};

pub(super) fn record(number: u128) -> PublishedProblemRecord {
    let problem = ProblemId::from_uuid(Uuid::from_u128(number));
    let version = VersionId::from_uuid(Uuid::from_u128(20_000 + number));
    let question = QuestionDefinition::from_draft(
        DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(30_000 + number)),
            source: DraftQuestionSource::Native {
                family: "catalog_fixture".to_string(),
            },
            prompt: Vec::new(),
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.1 },
                unit: None,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: format!("Peptide catalog item {number}"),
                tags: vec![Tag::new("peptide")],
                taxonomy: vec![TaxonomyTerm {
                    scheme: "discipline".to_string(),
                    code: "biochemistry".to_string(),
                    label: "Biochemistry".to_string(),
                }],
                license: License::CcBy,
                language: "en".to_string(),
            },
        },
        problem,
        version,
        question_model::QuestionSource::Native {
            family: "catalog_fixture".to_string(),
        },
    );
    PublishedProblemRecord {
        problem,
        question_id: {
            let mut value = u32::try_from(number).expect("fixture Question ID fits 30 bits");
            let mut bytes = [b'0'; 6];
            for output in bytes.iter_mut().rev() {
                *output = question_model::QUESTION_ID_ALPHABET[(value & 0x1f) as usize];
                value >>= 5;
            }
            crate::QuestionIdCodec::from_server_secret([0x42; 32])
                .issue_for_identifier(std::str::from_utf8(&bytes).expect("alphabet is ASCII"))
                .expect("fixture Question ID issues")
        },
        public_id: ProblemPublicId::new(
            u64::try_from(number).expect("fixture number fits a public ID") + 1,
        )
        .expect("fixture public ID is positive"),
        version,
        version_number: ProblemVersionNumber::new(1).expect("fixture version is positive"),
        question,
        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        scope: PublicationScope::Public,
        lifecycle: CatalogLifecycle::Published,
        authors: vec![UserId::from_uuid(Uuid::from_u128(40_000))],
        previous_version: None,
        derived_from: None,
        published_at: ActivityTimestamp::from_unix_millis(0),
    }
}

fn statistics_attempt(
    number: u128,
    tenant: TenantId,
    run: RunId,
    reference: ProblemVersionRef,
    position: u32,
    issued_at: i64,
) -> QuestionAttempt {
    QuestionAttempt {
        id: QuestionAttemptId::from_uuid(Uuid::from_u128(number)),
        tenant,
        run,
        problem: reference.problem,
        question_version: reference.version,
        assignment_position: position,
        seed: number as u64,
        parameter_hash: format!("statistics-parameters-{number}"),
        response: None,
        status: AttemptStatus::InProgress,
        result: None,
        timer: AttemptTimerRecord {
            issued_at: ActivityTimestamp::from_unix_millis(issued_at),
            deadline: None,
            submitted_at: None,
        },
        provenance: AttemptProvenance {
            adapter: ImplementationVersion {
                id: "native".to_string(),
                version: "1".to_string(),
            },
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: ImplementationVersion {
                id: "numeric".to_string(),
                version: "1".to_string(),
            },
            rendered_question_sha256: format!("statistics-render-{number}"),
        },
        issued_capability: question_model::IssuedAttemptCapabilityV1::NotApplicable,
    }
}

fn submit_statistics_attempt(
    store: &MemoryStore,
    context: TenantContext,
    actor: UserId,
    attempt: QuestionAttempt,
    submitted_at: i64,
    earned: f64,
    possible: f64,
) -> (SubmissionRecord, SubmitQuestionAttemptCommand) {
    let command = SubmitQuestionAttemptCommand {
        actor,
        attempt: attempt.id,
        response: StudentResponse::Numeric { value: earned },
        result: AttemptResult {
            correct: earned == possible,
            points_earned: earned,
            points_possible: possible,
        },
        feedback: question_model::FeedbackContent::default(),
        idempotency_key: SubmissionIdempotencyKey::parse(format!(
            "statistics-submission-{}",
            attempt.id
        ))
        .expect("valid fixture idempotency key"),
    };
    let mut state = store.write_state().expect("statistics fixture state");
    state.authoritative_time = ActivityTimestamp::from_unix_millis(submitted_at);
    insert_statistics_issued_authority(&mut state, &attempt);
    state.attempts.insert((attempt.tenant, attempt.id), attempt);
    let record = submit_question_attempt_locked(&mut state, context, command.clone())
        .expect("statistics fixture submission");
    (record, command)
}

/// Statistics fixtures install only the authority a real issue transaction
/// persists. Submission must therefore succeed even after the catalog record
/// is unavailable; it has no permission to recover timing from current policy.
fn insert_statistics_issued_authority(state: &mut State, attempt: &QuestionAttempt) {
    let run = state
        .runs
        .get(&(attempt.tenant, attempt.run))
        .expect("statistics attempt has a run");
    let enrollment = state
        .enrollments
        .get(&(attempt.tenant, run.enrollment))
        .expect("statistics run has an enrollment");
    state.attempt_presentation_capabilities.insert(
        (attempt.tenant, attempt.id),
        crate::PresentationCapability::NotApplicable,
    );
    state.attempt_flat_grading_capabilities.insert(
        (attempt.tenant, attempt.id),
        crate::FlatGradingCapability::NotApplicable,
    );
    state.attempt_webwork_grading_capabilities.insert(
        (attempt.tenant, attempt.id),
        crate::WebworkGradingCapability::NotApplicable,
    );
    state.attempt_feedback_disclosures.insert(
        (attempt.tenant, attempt.id),
        FeedbackDisclosure::ImmediateCorrectness,
    );
    state.attempt_timing.insert(
        (attempt.tenant, attempt.id),
        MemoryAttemptTiming {
            assignment: enrollment.assignment,
            authored_deadline: None,
            authored_grace_seconds: 0,
            effective_deadline: None,
            effective_grace_seconds: 0,
            auto_submit_at: None,
            generation: 1,
            job: None,
        },
    );
}

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
    let assignment = AssignmentRecord {
        id: assignment_id,
        tenant,
        course_id: CourseId::from_uuid(Uuid::from_u128(72_006)),
        title: "Statistics completion fixture".to_string(),
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
        public_id: question_model::RunPublicId::new(1).expect("valid public run ID"),
        tenant,
        enrollment: enrollment_id,
        run_number: 1,
        started_at: ActivityTimestamp::from_unix_millis(0),
        completed_at: None,
        score: None,
        mode: RunMode::Assigned,
        variation: question_model::VariationPolicy::NewSeeds,
    };
    let run_items = select_assignment_run_items(&assignment, assigned_run)
        .expect("statistics fixture run items");
    {
        let mut state = store.write_state().expect("statistics fixture state");
        state.courses.insert(
            (tenant, assignment.course_id),
            CourseRecord {
                id: assignment.course_id,
                tenant,
                title: "Statistics fixture course".to_string(),
                members: vec![CourseMembership {
                    user: actor,
                    role: CourseMembershipRole::Student,
                }],
            },
        );
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
    }

    let regressive = statistics_attempt(72_099, tenant, assigned_run, a, 0, 2_000);
    let regressive_command = SubmitQuestionAttemptCommand {
        actor,
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
        context,
        actor,
        statistics_attempt(72_100, tenant, assigned_run, a, 0, 0),
        1_500,
        0.0,
        2.0,
    );
    submit_statistics_attempt(
        &store,
        context,
        actor,
        statistics_attempt(72_101, tenant, assigned_run, a, 0, 2_000),
        4_500,
        1.0,
        2.0,
    );
    submit_statistics_attempt(
        &store,
        context,
        actor,
        statistics_attempt(72_102, tenant, assigned_run, b, 1, 5_000),
        6_000,
        1.0,
        4.0,
    );
    let (_, final_command) = submit_statistics_attempt(
        &store,
        context,
        actor,
        statistics_attempt(72_103, tenant, assigned_run, a, 2, 7_000),
        100_007_000,
        2.0,
        2.0,
    );

    let completed_statistics = {
        let state = store.read_state().expect("completed statistics state");
        assert_eq!(state.question_statistics_receipts.len(), 2);
        let a_snapshot = state.question_statistics[&(a.problem, a.version)].snapshot();
        assert_eq!(a_snapshot.cohort_size, 1);
        assert_eq!(a_snapshot.score_sum, 0.75);
        assert_eq!(a_snapshot.attempts_sum, 3);
        assert_eq!(a_snapshot.durations.bins[9], 1);
        assert_eq!(a_snapshot.discrimination.count, 1);
        assert_eq!(a_snapshot.discrimination.mean_x, 0.75);
        assert_eq!(a_snapshot.discrimination.mean_y, 0.25);
        let b_snapshot = state.question_statistics[&(b.problem, b.version)].snapshot();
        assert_eq!(b_snapshot.cohort_size, 1);
        assert_eq!(b_snapshot.score_sum, 0.25);
        assert_eq!(b_snapshot.attempts_sum, 1);
        assert_eq!(b_snapshot.durations.bins[0], 1);
        assert_eq!(b_snapshot.discrimination.mean_x, 0.25);
        assert_eq!(b_snapshot.discrimination.mean_y, 0.75);
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
                public_id: question_model::RunPublicId::new(2).expect("valid public run ID"),
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
            context,
            actor,
            statistics_attempt(
                number,
                tenant,
                practice_run,
                reference,
                position,
                200_000_000,
            ),
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

#[tokio::test]
async fn statistics_receipts_are_exactly_once_and_disclose_only_at_k_five() {
    let store = MemoryStore::default();
    let mut record = record(71_000);
    record.scope = PublicationScope::Institution;
    let reference = ProblemVersionRef {
        problem: record.problem,
        version: record.version,
    };
    let tenant = TenantId::from_uuid(Uuid::from_u128(71_001));
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .write_state()
        .expect("test state")
        .published
        .insert((reference.problem, reference.version), record);
    store
        .write_state()
        .expect("test state")
        .catalog_grants
        .insert((tenant, reference.problem, reference.version));

    let first = CollapsedQuestionObservation::new(0.5, 2, 30, Some(0.4))
        .expect("valid collapsed observation");
    assert!(
        store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(71_010)),
                RunId::from_uuid(Uuid::from_u128(71_020)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
                reference,
                first,
            )
            .expect("first receipt records")
    );
    assert!(
        !store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(71_010)),
                RunId::from_uuid(Uuid::from_u128(71_020)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
                reference,
                first,
            )
            .expect("exact replay is harmless")
    );
    let before_conflict = store.read_state().expect("test state").question_statistics
        [&(reference.problem, reference.version)]
        .snapshot();
    assert_eq!(
        store.record_question_statistics_contribution(
            tenant,
            EnrollmentId::from_uuid(Uuid::from_u128(71_010)),
            RunId::from_uuid(Uuid::from_u128(71_020)),
            QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
            reference,
            CollapsedQuestionObservation::new(0.6, 2, 30, Some(0.4))
                .expect("valid conflicting observation"),
        ),
        Err(StoreError::Conflict)
    );
    assert_eq!(
        store.read_state().expect("test state").question_statistics
            [&(reference.problem, reference.version)]
            .snapshot(),
        before_conflict
    );
    for number in 1..4_u128 {
        assert!(
            store
                .record_question_statistics_contribution(
                    tenant,
                    EnrollmentId::from_uuid(Uuid::from_u128(71_010 + number)),
                    RunId::from_uuid(Uuid::from_u128(71_020 + number)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(71_030 + number)),
                    reference,
                    CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.4))
                        .expect("valid contribution"),
                )
                .expect("distinct receipt records")
        );
    }
    assert_eq!(
        store
            .question_statistics(context, reference)
            .await
            .expect("safe statistics read at four"),
        QuestionStatisticsDisclosure::Suppressed
    );
    assert!(
        store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(71_014)),
                RunId::from_uuid(Uuid::from_u128(71_024)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(71_034)),
                reference,
                CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.4))
                    .expect("valid fifth contribution"),
            )
            .expect("fifth receipt records")
    );
    let disclosure = store
        .question_statistics(context, reference)
        .await
        .expect("safe statistics read");
    assert!(matches!(
        disclosure,
        QuestionStatisticsDisclosure::Available(view) if view.cohort_size == 5
    ));
    {
        let state = store.read_state().expect("test state");
        assert_eq!(state.question_statistics_receipts.len(), 5);
        assert_eq!(
            state.question_statistics[&(reference.problem, reference.version)].cohort_size(),
            5
        );
    }
    let second_reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(Uuid::from_u128(71_100)),
        version: VersionId::from_uuid(Uuid::from_u128(71_101)),
    };
    assert!(
        store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(71_100)),
                RunId::from_uuid(Uuid::from_u128(71_020)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
                second_reference,
                first,
            )
            .expect("one completion trigger can contribute another version")
    );
    let foreign_context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(71_999)));
    assert_eq!(
        store
            .question_statistics(foreign_context, reference)
            .await
            .expect("foreign safe statistics read"),
        QuestionStatisticsDisclosure::Suppressed
    );
}

#[tokio::test]
async fn catalog_search_finds_an_exact_question_id_beyond_the_first_page() {
    let store = MemoryStore::default();
    let context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(73_000)));
    let mut decoy = record(70);
    let exact = record(71);
    let exact_reference = exact.question_id.to_string();
    {
        let mut state = store.write_state().expect("catalog search state");
        decoy.question.metadata.title = exact_reference.clone();
        state
            .published
            .insert((decoy.problem, decoy.version), decoy);
        state
            .published
            .insert((exact.problem, exact.version), exact);
    }

    let page = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                text: Some(exact_reference.clone()),
                page_size: Some(1),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("exact display reference search");

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].question_id.to_string(), exact_reference);
    assert_eq!(page.next_cursor, None);
}

#[tokio::test]
async fn catalog_statistics_filter_facets_and_detail_use_only_k_gated_aggregates() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(73_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let published = record(73_002);
    let reference = ProblemVersionRef {
        problem: published.problem,
        version: published.version,
    };
    store
        .write_state()
        .expect("catalog statistics state")
        .published
        .insert((reference.problem, reference.version), published);

    for number in 0..4_u128 {
        store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(73_100 + number)),
                RunId::from_uuid(Uuid::from_u128(73_200 + number)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(73_300 + number)),
                reference,
                CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.5))
                    .expect("valid observation"),
            )
            .expect("statistics receipt");
    }
    let below_k = store
        .search_catalog(context, CatalogSearchQuery::default())
        .await
        .expect("below-k catalog search");
    assert_eq!(below_k.facets.statistics.available, 0);
    assert_eq!(below_k.facets.statistics.unavailable, 1);
    assert!(
        store
            .search_catalog(
                context,
                CatalogSearchQuery {
                    statistics: CatalogStatisticsAvailability::Available,
                    ..CatalogSearchQuery::default()
                },
            )
            .await
            .expect("below-k available filter")
            .items
            .is_empty()
    );
    assert!(matches!(
        store
            .get_catalog_detail(context, reference)
            .await
            .expect("below-k detail")
            .expect("visible detail")
            .statistics,
        question_model::CatalogStatisticsStatus::Unavailable
    ));

    store
        .record_question_statistics_contribution(
            tenant,
            EnrollmentId::from_uuid(Uuid::from_u128(73_104)),
            RunId::from_uuid(Uuid::from_u128(73_204)),
            QuestionAttemptId::from_uuid(Uuid::from_u128(73_304)),
            reference,
            CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.5))
                .expect("valid fifth observation"),
        )
        .expect("fifth statistics receipt");
    let at_k = store
        .search_catalog(context, CatalogSearchQuery::default())
        .await
        .expect("at-k catalog search");
    assert_eq!(at_k.facets.statistics.available, 1);
    assert_eq!(at_k.facets.statistics.unavailable, 0);
    assert_eq!(
        store
            .search_catalog(
                context,
                CatalogSearchQuery {
                    statistics: CatalogStatisticsAvailability::Available,
                    ..CatalogSearchQuery::default()
                },
            )
            .await
            .expect("at-k available filter")
            .items
            .len(),
        1
    );
    assert!(matches!(
        store
            .get_catalog_detail(context, reference)
            .await
            .expect("at-k detail")
            .expect("visible detail")
            .statistics,
        question_model::CatalogStatisticsStatus::Available(view) if view.cohort_size == 5
    ));
}

#[tokio::test]
async fn ten_thousand_catalog_rows_return_one_bounded_page_with_server_facets() {
    let store = MemoryStore::default();
    {
        let mut state = store.write_state().expect("test state");
        for number in 1..=10_000 {
            let record = record(number);
            state
                .published
                .insert((record.problem, record.version), record);
        }
        let mut institution_only = record(10_001);
        institution_only.scope = PublicationScope::Institution;
        state.catalog_grants.insert((
            TenantId::from_uuid(Uuid::from_u128(50_001)),
            institution_only.problem,
            institution_only.version,
        ));
        state.published.insert(
            (institution_only.problem, institution_only.version),
            institution_only,
        );
        for number in 0..65_u128 {
            let mut distinct = record(11_000 + number);
            distinct.question.metadata.taxonomy = vec![TaxonomyTerm {
                scheme: "extra".to_string(),
                code: format!("{number:02}"),
                label: if number == 0 { "Zulu" } else { "Term" }.to_string(),
            }];
            state
                .published
                .insert((distinct.problem, distinct.version), distinct);
        }
        let mut duplicate_label = record(12_000);
        duplicate_label.question.metadata.taxonomy = vec![TaxonomyTerm {
            scheme: "extra".to_string(),
            code: "00".to_string(),
            label: "Alpha".to_string(),
        }];
        state.published.insert(
            (duplicate_label.problem, duplicate_label.version),
            duplicate_label,
        );
    }
    let context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(50_000)));
    let first = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                text: Some(" peptide   catalog ".to_string()),
                page_size: Some(37),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("bounded search");
    assert_eq!(first.items.len(), 37);
    assert_eq!(first.facets.statistics.available, 0);
    assert_eq!(first.facets.statistics.unavailable, 10_066);
    assert_eq!(first.facets.taxonomy[0].count, 10_000);
    assert_eq!(first.facets.taxonomy.len(), MAX_CATALOG_TAXONOMY_FACETS);
    assert_eq!(first.facets.taxonomy[1].term.code, "00");
    assert_eq!(first.facets.taxonomy[1].term.label, "Alpha");
    assert_eq!(first.facets.taxonomy[1].count, 2);
    assert_eq!(
        first.facets.taxonomy[1..]
            .iter()
            .map(|facet| facet.term.code.clone())
            .collect::<Vec<_>>(),
        (0..=62)
            .map(|number| format!("{number:02}"))
            .collect::<Vec<_>>(),
    );
    assert!(
        first
            .facets
            .taxonomy
            .iter()
            .all(|facet| facet.term.code != "63" && facet.term.code != "64")
    );
    assert_eq!(first.facets.capabilities[0].count, 10_066);
    assert_eq!(first.facets.licenses[0].count, 10_066);
    assert!(
        first
            .items
            .iter()
            .all(|item| item.scope == PublicationScope::Public)
    );
    let cursor = first.next_cursor.clone().expect("next cursor");
    let mut tampered = cursor.clone().into_bytes();
    let last = tampered.len() - 1;
    tampered[last] = if tampered[last] == b'A' { b'B' } else { b'A' };
    assert!(matches!(
        store
            .search_catalog(
                context,
                CatalogSearchQuery {
                    text: Some("peptide catalog".to_string()),
                    cursor: Some(String::from_utf8(tampered).expect("url-safe cursor")),
                    ..CatalogSearchQuery::default()
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let second = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                text: Some("peptide catalog".to_string()),
                cursor: Some(cursor),
                page_size: Some(37),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("second bounded search");
    assert_eq!(second.items.len(), 37);
    assert!(
        first
            .items
            .iter()
            .all(|left| second
                .items
                .iter()
                .all(|right| (left.problem, left.version) != (right.problem, right.version)))
    );
    assert!(matches!(
        store
            .search_catalog(
                context,
                CatalogSearchQuery {
                    text: Some("different query".to_string()),
                    cursor: first.next_cursor,
                    ..CatalogSearchQuery::default()
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}
