//! Memory-only current item-analysis contract checks.

use super::*;

struct AnalysisFixture {
    context: TenantContext,
    foreign_context: TenantContext,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    instructor: UserId,
    student: UserId,
    enrollment: EnrollmentId,
    automatic: ProblemVersionRef,
    manual: ProblemVersionRef,
    automatic_item: AssignmentItemId,
    manual_item: AssignmentItemId,
    instructor_session: SessionTokenHash,
    administrator_session: SessionTokenHash,
    student_session: SessionTokenHash,
    outsider_session: SessionTokenHash,
}

async fn analysis_fixture(store: &MemoryStore) -> AnalysisFixture {
    let tenant = TenantId::from_uuid(uuid(80_001));
    let foreign_tenant = TenantId::from_uuid(uuid(80_002));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let instructor = UserId::from_uuid(uuid(80_003));
    let administrator = UserId::from_uuid(uuid(80_004));
    let student = UserId::from_uuid(uuid(80_005));
    let outsider = UserId::from_uuid(uuid(80_006));
    let workspace = WorkspaceId::from_uuid(uuid(80_007));
    let course = CourseId::from_uuid(uuid(80_008));
    let assignment = AssignmentId::from_uuid(uuid(80_009));
    let enrollment = EnrollmentId::from_uuid(uuid(80_010));
    let automatic = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(80_011)),
        version: VersionId::from_uuid(uuid(80_012)),
    };
    let manual = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(80_013)),
        version: VersionId::from_uuid(uuid(80_014)),
    };
    for reference in [automatic, manual] {
        let draft = DraftRecord {
            tenant,
            question: draft_question(workspace),
            revises: None,
            derived_from: None,
        };
        let saved = store
            .upsert_draft(context, instructor, None, draft.clone())
            .await
            .expect("analysis fixture draft");
        store
            .publish_draft(
                context,
                instructor,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved.revision,
                    publication: reference,
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher: instructor,
                    scope: PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await
            .expect("analysis fixture publication");
    }
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Item analysis course".to_string(),
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
        .expect("analysis fixture course");
    let mut policy = policies();
    policy.completion = CompletionRequirement::AnswerAll;
    let items = fixed_items(vec![automatic, manual]);
    let automatic_item = items[0].id;
    let manual_item = items[1].id;
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Item analysis assignment".to_string(),
                items,
                selection_groups: Vec::new(),
                policies: policy,
            },
        )
        .await
        .expect("analysis fixture assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment,
                tenant,
                assignment,
                user: student,
                student: StudentId::from_uuid(uuid(80_015)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("analysis fixture enrollment");

    async fn session(
        store: &MemoryStore,
        tenant: TenantId,
        user: UserId,
        roles: Vec<UserRole>,
        key: &'static [u8],
    ) -> SessionTokenHash {
        let token = SessionTokenHash::compute(key);
        store
            .create_session(
                token,
                SessionSubject::new(tenant, user, "Item analysis fixture", roles)
                    .expect("fixture session subject"),
                SessionLifetime::from_seconds(60).expect("fixture session lifetime"),
            )
            .await
            .expect("fixture session");
        token
    }

    AnalysisFixture {
        context,
        foreign_context,
        tenant,
        course,
        assignment,
        instructor,
        student,
        enrollment,
        automatic,
        manual,
        automatic_item,
        manual_item,
        instructor_session: session(
            store,
            tenant,
            instructor,
            vec![UserRole::Instructor],
            b"analysis-instructor",
        )
        .await,
        administrator_session: session(
            store,
            tenant,
            administrator,
            vec![UserRole::Administrator],
            b"analysis-administrator",
        )
        .await,
        student_session: session(
            store,
            tenant,
            student,
            vec![UserRole::Student],
            b"analysis-student",
        )
        .await,
        outsider_session: session(
            store,
            tenant,
            outsider,
            vec![UserRole::Instructor],
            b"analysis-outsider",
        )
        .await,
    }
}

fn provenance(label: &str) -> AttemptProvenance {
    AttemptProvenance {
        adapter: implementation("native"),
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: implementation("numeric"),
        rendered_question_sha256: format!("item-analysis-rendered-{label}"),
    }
}

async fn issue(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
    run: RunId,
    position: u32,
    reference: ProblemVersionRef,
    id: u128,
) -> QuestionAttempt {
    store
        .issue_or_resume_question_attempt(
            fixture.context,
            IssueQuestionAttemptCommand {
                actor: fixture.student,
                attempt: QuestionAttemptId::from_uuid(uuid(id)),
                run,
                assignment_position: position,
                problem: reference.problem,
                question_version: reference.version,
                seed: u64::try_from(id).expect("fixture seed"),
                parameter_hash: format!("item-analysis-parameters-{id}"),
                provenance: provenance(&id.to_string()),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("analysis fixture issue")
}

async fn submit_auto(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
    attempt: QuestionAttemptId,
    key: &str,
) {
    store
        .submit_question_attempt(
            fixture.context,
            SubmitQuestionAttemptCommand {
                actor: fixture.student,
                attempt,
                response: StudentResponse::Numeric { value: 42.0 },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse(key).expect("fixture key"),
            },
        )
        .await
        .expect("automatic item submit");
}

async fn run_analysis_job(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
) -> CourseItemAnalysisCommitOutcome {
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("analysis lease"),
        )
        .await
        .expect("analysis job claim")
        .expect("analysis job available");
    let JobPayload::RecalculateCourseItemAnalysis {
        assignment,
        generation,
    } = claim.payload
    else {
        panic!("expected item-analysis job")
    };
    let command = CourseItemAnalysisWorkerCommand {
        job: claim.id,
        lease: claim.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_course_item_analysis(fixture.context, command)
        .await
        .expect("analysis staging");
    store
        .commit_course_item_analysis(fixture.context, command)
        .await
        .expect("analysis publication")
}

async fn run_scoring_job(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
) -> AssignmentScoringCommitOutcome {
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("scoring lease"),
        )
        .await
        .expect("scoring job claim")
        .expect("scoring job available");
    let JobPayload::RecalculateAssignment {
        assignment,
        generation,
    } = claim.payload
    else {
        panic!("expected scoring job")
    };
    let command = AssignmentScoringWorkerCommand {
        job: claim.id,
        lease: claim.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(fixture.context, command)
        .await
        .expect("scoring staging");
    store
        .commit_assignment_scoring(fixture.context, command)
        .await
        .expect("scoring publication")
}

async fn current_report(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
) -> domain::item_analysis::CourseItemAnalysisReport {
    store
        .course_item_analysis(
            fixture.context,
            fixture.instructor_session,
            fixture.course,
            fixture.assignment,
        )
        .await
        .expect("analysis read")
        .expect("analysis is current")
}

#[tokio::test]
async fn memory_item_analysis_tracks_pending_manual_then_corrected_current_scoring() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;
    let run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            fixture.assignment,
            RunId::from_uuid(uuid(80_020)),
        )
        .await
        .expect("analysis run");
    let automatic = issue(&store, &fixture, run.id, 0, fixture.automatic, 80_021).await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_000))
        .expect("automatic submission time");
    submit_auto(&store, &fixture, automatic.id, "analysis-auto").await;
    let manual = issue(&store, &fixture, run.id, 1, fixture.manual, 80_022).await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(3_000))
        .expect("manual submission time");
    store
        .submit_pending_manual_question_attempt(
            fixture.context,
            SubmitPendingManualQuestionAttemptCommand {
                actor: fixture.student,
                attempt: manual.id,
                response: StudentResponse::Numeric { value: 7.0 },
                idempotency_key: SubmissionIdempotencyKey::parse("analysis-manual")
                    .expect("fixture key"),
            },
        )
        .await
        .expect("manual pending submission");

    store
        .enqueue_job(
            fixture.context,
            EnqueueJob {
                tenant: fixture.tenant,
                payload: JobPayload::RecalculateCourseItemAnalysis {
                    assignment: fixture.assignment,
                    generation: question_model::ScoringGeneration::INITIAL,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("initial analysis job");
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    let pending = current_report(&store, &fixture).await;
    assert!(pending.incomplete_manual_grading);
    let automatic_row = pending
        .items
        .iter()
        .find(|row| row.assignment_item == fixture.automatic_item)
        .expect("automatic row");
    assert_eq!(automatic_row.graded_attempt_count, 1);
    let manual_row = pending
        .items
        .iter()
        .find(|row| row.assignment_item == fixture.manual_item)
        .expect("manual row");
    assert_eq!(manual_row.graded_attempt_count, 0);
    assert_eq!(manual_row.pending_manual_attempt_count, 1);
    assert_eq!(pending.average_completion_time_millis, Some(2_000));
    assert_eq!(manual_row.average_completion_time_millis, Some(2_000));

    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(5_000))
        .expect("manual grading time");
    let evaluation = store
        .get_manual_evaluation_for_edit(fixture.context, fixture.instructor, manual.id)
        .await
        .expect("evaluation lookup")
        .expect("pending evaluation");
    store
        .set_manual_grade(
            fixture.context,
            SetManualGradeCommand {
                action: ManualGradeActionId::from_uuid(uuid(80_023)),
                actor: fixture.instructor,
                attempt: manual.id,
                expected_revision: evaluation.revision,
                credit: ManualCredit::parse("0.5").expect("manual credit"),
            },
        )
        .await
        .expect("manual grading");
    assert_eq!(
        run_scoring_job(&store, &fixture).await,
        AssignmentScoringCommitOutcome::Committed
    );
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    let graded = current_report(&store, &fixture).await;
    assert!(!graded.incomplete_manual_grading);
    let manual_row = graded
        .items
        .iter()
        .find(|row| row.assignment_item == fixture.manual_item)
        .expect("graded manual row");
    assert_eq!(manual_row.graded_attempt_count, 1);
    assert_eq!(manual_row.pending_manual_attempt_count, 0);
    assert_eq!(manual_row.response_distribution.partial, 1);
    assert_eq!(graded.average_completion_time_millis, Some(2_000));
    assert_eq!(manual_row.average_completion_time_millis, Some(2_000));
}

#[tokio::test]
async fn memory_item_analysis_is_instructor_only_and_report_is_identity_free() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;
    let run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            fixture.assignment,
            RunId::from_uuid(uuid(80_030)),
        )
        .await
        .expect("analysis run");
    let automatic = issue(&store, &fixture, run.id, 0, fixture.automatic, 80_031).await;
    submit_auto(&store, &fixture, automatic.id, "analysis-auth-auto").await;
    let manual = issue(&store, &fixture, run.id, 1, fixture.manual, 80_032).await;
    store
        .force_submit_attempt(
            fixture.context,
            ForceSubmitAttemptCommand {
                action: AttemptSupportActionId::from_uuid(uuid(80_033)),
                actor: fixture.instructor,
                attempt: manual.id,
            },
        )
        .await
        .expect("force submit produces unanswered evidence");
    store
        .enqueue_job(
            fixture.context,
            EnqueueJob {
                tenant: fixture.tenant,
                payload: JobPayload::RecalculateCourseItemAnalysis {
                    assignment: fixture.assignment,
                    generation: question_model::ScoringGeneration::INITIAL,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("analysis job");
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    let report = current_report(&store, &fixture).await;
    let manual_row = report
        .items
        .iter()
        .find(|row| row.assignment_item == fixture.manual_item)
        .expect("manual row");
    assert_eq!(manual_row.unanswered_attempt_count, 1);
    store
        .clear_attempt(
            fixture.context,
            ClearAttemptCommand {
                action: AttemptSupportActionId::from_uuid(uuid(80_034)),
                actor: fixture.instructor,
                attempt: manual.id,
            },
        )
        .await
        .expect("clearing keeps protected evidence but removes current analysis observation");
    store
        .enqueue_job(
            fixture.context,
            EnqueueJob {
                tenant: fixture.tenant,
                payload: JobPayload::RecalculateCourseItemAnalysis {
                    assignment: fixture.assignment,
                    generation: question_model::ScoringGeneration::INITIAL,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("post-clear analysis job");
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    let report = current_report(&store, &fixture).await;
    let cleared_row = report
        .items
        .iter()
        .find(|row| row.assignment_item == fixture.manual_item)
        .expect("cleared item remains structurally visible");
    assert_eq!(cleared_row.graded_attempt_count, 0);
    assert_eq!(cleared_row.unanswered_attempt_count, 0);
    assert_eq!(cleared_row.pending_manual_attempt_count, 0);
    assert_eq!(
        store
            .course_item_analysis(
                fixture.context,
                fixture.administrator_session,
                fixture.course,
                fixture.assignment
            )
            .await,
        Ok(Some(report.clone())),
        "tenant administrators have report access"
    );
    for (context, session, label) in [
        (fixture.context, fixture.student_session, "student"),
        (fixture.context, fixture.outsider_session, "outsider"),
        (
            fixture.foreign_context,
            fixture.instructor_session,
            "foreign tenant",
        ),
    ] {
        assert_eq!(
            store
                .course_item_analysis(context, session, fixture.course, fixture.assignment)
                .await,
            Ok(None),
            "{label} cannot enumerate course analysis"
        );
    }
    let serialized = serde_json::to_string(&report).expect("report serialization");
    for private_value in [
        fixture.student.to_string(),
        fixture.enrollment.to_string(),
        run.id.to_string(),
        automatic.id.to_string(),
        manual.id.to_string(),
        "analysis-auth-auto".to_string(),
        "feedback".to_string(),
    ] {
        assert!(
            !serialized.contains(&private_value),
            "course analysis must not serialize private learner, attempt, response, or feedback data: {private_value}"
        );
    }
}

#[tokio::test]
async fn memory_item_analysis_stale_generation_cannot_replace_current_report() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;
    store
        .enqueue_job(
            fixture.context,
            EnqueueJob {
                tenant: fixture.tenant,
                payload: JobPayload::RecalculateCourseItemAnalysis {
                    assignment: fixture.assignment,
                    generation: question_model::ScoringGeneration::INITIAL,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("stale analysis job");
    let stale = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("claim")
        .expect("job");
    let JobPayload::RecalculateCourseItemAnalysis {
        assignment,
        generation,
    } = stale.payload
    else {
        panic!("analysis job")
    };
    let stale_command = CourseItemAnalysisWorkerCommand {
        job: stale.id,
        lease: stale.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_course_item_analysis(fixture.context, stale_command)
        .await
        .expect("stale staging");

    let run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            fixture.assignment,
            RunId::from_uuid(uuid(80_040)),
        )
        .await
        .expect("run");
    let automatic = issue(&store, &fixture, run.id, 0, fixture.automatic, 80_041).await;
    submit_auto(&store, &fixture, automatic.id, "analysis-stale-auto").await;
    let manual = issue(&store, &fixture, run.id, 1, fixture.manual, 80_042).await;
    store
        .submit_pending_manual_question_attempt(
            fixture.context,
            SubmitPendingManualQuestionAttemptCommand {
                actor: fixture.student,
                attempt: manual.id,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: SubmissionIdempotencyKey::parse("analysis-stale-manual")
                    .expect("key"),
            },
        )
        .await
        .expect("pending");
    let evaluation = store
        .get_manual_evaluation_for_edit(fixture.context, fixture.instructor, manual.id)
        .await
        .expect("evaluation")
        .expect("pending evaluation");
    store
        .set_manual_grade(
            fixture.context,
            SetManualGradeCommand {
                action: ManualGradeActionId::from_uuid(uuid(80_043)),
                actor: fixture.instructor,
                attempt: manual.id,
                expected_revision: evaluation.revision,
                credit: ManualCredit::parse("1").expect("credit"),
            },
        )
        .await
        .expect("grade updates generation");
    assert_eq!(
        store
            .commit_course_item_analysis(fixture.context, stale_command)
            .await,
        Ok(CourseItemAnalysisCommitOutcome::Superseded),
        "prepared analysis from an older scoring generation cannot publish"
    );
    assert_eq!(
        run_scoring_job(&store, &fixture).await,
        AssignmentScoringCommitOutcome::Committed
    );
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    assert_eq!(
        current_report(&store, &fixture)
            .await
            .source_scoring_generation
            .value(),
        2
    );
}

#[tokio::test]
async fn memory_item_analysis_uses_only_each_learners_latest_run_when_it_is_active() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;
    let completed_run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            fixture.assignment,
            RunId::from_uuid(uuid(80_050)),
        )
        .await
        .expect("completed fixture run");
    let automatic = issue(
        &store,
        &fixture,
        completed_run.id,
        0,
        fixture.automatic,
        80_051,
    )
    .await;
    submit_auto(&store, &fixture, automatic.id, "analysis-old-auto").await;
    let manual = issue(
        &store,
        &fixture,
        completed_run.id,
        1,
        fixture.manual,
        80_052,
    )
    .await;
    store
        .submit_pending_manual_question_attempt(
            fixture.context,
            SubmitPendingManualQuestionAttemptCommand {
                actor: fixture.student,
                attempt: manual.id,
                response: StudentResponse::Numeric { value: 5.0 },
                idempotency_key: SubmissionIdempotencyKey::parse("analysis-old-manual")
                    .expect("fixture key"),
            },
        )
        .await
        .expect("old pending submission");
    let evaluation = store
        .get_manual_evaluation_for_edit(fixture.context, fixture.instructor, manual.id)
        .await
        .expect("old evaluation lookup")
        .expect("old pending evaluation");
    store
        .set_manual_grade(
            fixture.context,
            SetManualGradeCommand {
                action: ManualGradeActionId::from_uuid(uuid(80_053)),
                actor: fixture.instructor,
                attempt: manual.id,
                expected_revision: evaluation.revision,
                credit: ManualCredit::parse("1").expect("manual credit"),
            },
        )
        .await
        .expect("old manual grade");
    assert_eq!(
        run_scoring_job(&store, &fixture).await,
        AssignmentScoringCommitOutcome::Committed
    );
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    assert_eq!(
        current_report(&store, &fixture).await.completed_run_count,
        1
    );

    let latest_run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            fixture.assignment,
            RunId::from_uuid(uuid(80_054)),
        )
        .await
        .expect("newer run starts after the completed run");
    let _active = issue(
        &store,
        &fixture,
        latest_run.id,
        0,
        fixture.automatic,
        80_055,
    )
    .await;
    store
        .enqueue_job(
            fixture.context,
            EnqueueJob {
                tenant: fixture.tenant,
                payload: JobPayload::RecalculateCourseItemAnalysis {
                    assignment: fixture.assignment,
                    generation: question_model::ScoringGeneration::new(2)
                        .expect("scoring generation"),
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("active-latest analysis job");
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    let report = current_report(&store, &fixture).await;
    assert_eq!(report.completed_run_count, 0);
    assert_eq!(report.in_progress_run_count, 1);
    assert!(
        report.items.is_empty(),
        "an active newer run suppresses the learner's older completed observations"
    );
}
