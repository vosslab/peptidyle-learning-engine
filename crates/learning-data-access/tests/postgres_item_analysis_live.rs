#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for the course-local item-analysis projection.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, AssignmentScoringCommitOutcome, AssignmentScoringWorkerCommand,
    AssignmentScoringWorkerStore, CatalogStore, CourseItemAnalysisCommitOutcome,
    CourseItemAnalysisStore, CourseItemAnalysisWorkerCommand, CourseItemAnalysisWorkerStore,
    CourseRecord, DraftRecord, IssueQuestionAttemptCommand, JobClaimFilter, JobLeaseDuration,
    JobPayload, JobStore, ManualCredit, ManualGradeActionId, ManualGradingStore,
    PublishDraftCommand, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
    SetManualGradeCommand, Store, SubmissionIdempotencyKey,
    SubmitPendingManualQuestionAttemptCommand, SubmitQuestionAttemptCommand, TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::{ResponseDefinition, StudentResponse};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
    RunPolicies, TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentScoringMode,
    AttemptProvenance, AttemptResult, BackendCapabilities, Capability, CourseId, CourseMembership,
    CourseMembershipRole, DraftQuestionDefinition, DraftQuestionSource, FeedbackContent,
    GradingDefinition, ImplementationVersion, PointValue, PresentationBindingV1,
    PresentationDigestV1, PresentationNonceV1, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionAttemptId, QuestionMetadata, QuestionSource, RunId, UserId, UserRole, VersionId,
    WorkspaceId,
};
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture randomness");
    Uuid::from_bytes(bytes)
}

fn implementation(name: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: name.to_string(),
        version: "1".to_string(),
    }
}

fn presentation_binding(marker: u8) -> PresentationBindingV1 {
    PresentationBindingV1::new(
        PresentationNonceV1::from_bytes([marker; 16]),
        PresentationDigestV1::compute(&[marker]),
    )
}

fn provenance(name: &str) -> AttemptProvenance {
    AttemptProvenance {
        adapter: implementation("item-analysis-live"),
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: implementation(name),
        rendered_question_sha256: format!("item-analysis-live-{name}"),
    }
}

fn draft(
    workspace: WorkspaceId,
    title: &str,
    response: ResponseDefinition,
) -> DraftQuestionDefinition {
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: "item_analysis_live".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: format!("Live item analysis: {title}"),
        }],
        response,
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::Deferred,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: title.to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    }
}

async fn publish(
    store: &PostgresStore,
    context: TenantContext,
    tenant: question_model::TenantId,
    instructor: UserId,
    title: &str,
    response: ResponseDefinition,
) -> ProblemVersionRef {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let record = DraftRecord {
        tenant,
        question: draft(WorkspaceId::from_uuid(id()), title, response),
        revises: None,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, instructor, None, record.clone())
        .await
        .expect("save fixture draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: record,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "item_analysis_live".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: instructor,
                scope: PublicationScope::Institution,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish fixture question");
    reference
}

async fn session(
    store: &PostgresStore,
    tenant: question_model::TenantId,
    user: UserId,
    roles: Vec<UserRole>,
    label: &'static [u8],
) -> SessionTokenHash {
    let token = SessionTokenHash::compute(label);
    store
        .create_session(
            token,
            SessionSubject::new(tenant, user, "Item analysis PostgreSQL fixture", roles)
                .expect("valid session subject"),
            SessionLifetime::from_seconds(300).expect("positive lifetime"),
        )
        .await
        .expect("persist fixture session");
    token
}

async fn issue(
    store: &PostgresStore,
    context: TenantContext,
    student: UserId,
    run: RunId,
    reference: ProblemVersionRef,
    position: u32,
    predecessor: Option<QuestionAttemptId>,
) -> question_model::QuestionAttempt {
    store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: QuestionAttemptId::from_uuid(id()),
                run,
                assignment_position: position,
                problem: reference.problem,
                question_version: reference.version,
                seed: u64::from(position) + 1,
                presentation: Some(presentation_binding(position as u8)),
                parameter_hash: format!("item-analysis-parameters-{position}"),
                provenance: provenance(if position == 0 { "automatic" } else { "manual" }),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: predecessor,
            },
        )
        .await
        .expect("issue fixture attempt")
}

async fn claim_scoring(
    store: &PostgresStore,
    assignment: AssignmentId,
) -> AssignmentScoringWorkerCommand {
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("claim scoring job")
        .expect("scoring job available");
    let JobPayload::RecalculateAssignment {
        assignment: job_assignment,
        generation,
    } = claim.payload
    else {
        panic!("scoring must run before lower-priority analysis");
    };
    assert_eq!(job_assignment, assignment);
    AssignmentScoringWorkerCommand {
        job: claim.id,
        lease: claim.lease_token,
        assignment,
        generation,
    }
}

async fn claim_analysis(
    store: &PostgresStore,
    assignment: AssignmentId,
) -> CourseItemAnalysisWorkerCommand {
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("claim analysis job")
        .expect("analysis job available");
    let JobPayload::RecalculateCourseItemAnalysis {
        assignment: job_assignment,
        generation,
    } = claim.payload
    else {
        panic!("expected course item-analysis job");
    };
    assert_eq!(job_assignment, assignment);
    CourseItemAnalysisWorkerCommand {
        job: claim.id,
        lease: claim.lease_token,
        assignment,
        generation,
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_item_analysis_is_current_private_and_generation_fenced() {
    let url = std::env::var("PLE_TEST_DATABASE_URL").expect("disposable database URL");
    let pool = lazy_pool(&url).expect("valid PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("baseline schema");
    let store = PostgresStore::new(pool.clone());
    let tenant = question_model::TenantId::from_uuid(id());
    let foreign_tenant = question_model::TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let instructor = UserId::from_uuid(id());
    let administrator = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    let outsider = UserId::from_uuid(id());
    let foreign_user = UserId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    let assignment = AssignmentId::from_uuid(id());
    let automatic = publish(
        &store,
        context,
        tenant,
        instructor,
        "Automatic",
        ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Absolute { epsilon: 0.01 },
            unit: None,
        },
    )
    .await;
    let manual = publish(
        &store,
        context,
        tenant,
        instructor,
        "Manual",
        ResponseDefinition::FileUpload {
            max_bytes: 10_000,
            accepted_extensions: vec!["pdf".to_string()],
        },
    )
    .await;
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Item-analysis live course".to_string(),
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
        .expect("create course");
    let items = vec![
        AssignmentItem {
            id: AssignmentItemId::from_uuid(id()),
            reference: automatic,
            position: 0,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        },
        AssignmentItem {
            id: AssignmentItemId::from_uuid(id()),
            reference: manual,
            position: 1,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        },
    ];
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Mixed item-analysis assignment".to_string(),
                items,
                selection_groups: Vec::new(),
                policies: RunPolicies {
                    completion: CompletionRequirement::AnswerAll,
                    grade: GradePolicy::Highest,
                    continued_practice: ContinuedPractice::Unlimited,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("create assignment");
    let run = store
        .start_or_resume_run(context, student, assignment, RunId::from_uuid(id()))
        .await
        .expect("start mixed run");
    let automatic_attempt = issue(&store, context, student, run.id, automatic, 0, None).await;
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: automatic_attempt.id,
                response: StudentResponse::Numeric { value: 42.0 },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("item-analysis-auto")
                    .expect("key"),
            },
        )
        .await
        .expect("submit automatic response");
    let manual_attempt = issue(
        &store,
        context,
        student,
        run.id,
        manual,
        1,
        Some(automatic_attempt.id),
    )
    .await;
    let raw_response = StudentResponse::FileUpload {
        object_key: "student-records/item-analysis-private.pdf".to_string(),
    };
    let pending = store
        .submit_pending_manual_question_attempt(
            context,
            SubmitPendingManualQuestionAttemptCommand {
                actor: student,
                attempt: manual_attempt.id,
                response: raw_response,
                idempotency_key: SubmissionIdempotencyKey::parse("item-analysis-manual")
                    .expect("key"),
            },
        )
        .await
        .expect("manual response stays pending");
    let terminal_submission_elapsed = pending
        .attempt
        .timer
        .submitted_at
        .expect("pending manual response has a terminal submission time")
        .as_unix_millis()
        .checked_sub(run.started_at.as_unix_millis())
        .and_then(|elapsed| u64::try_from(elapsed).ok())
        .expect("terminal submission follows run start");
    assert_eq!(
        store
            .get_run(context, run.id)
            .await
            .expect("read pending run")
            .expect("run")
            .score,
        None
    );
    assert!(
        store
            .course_item_analysis(
                context,
                SessionTokenHash::compute(b"missing"),
                course,
                assignment
            )
            .await
            .expect("missing session non-enumerating")
            .is_none()
    );

    let evaluation = store
        .get_manual_evaluation_for_edit(context, instructor, manual_attempt.id)
        .await
        .expect("read pending evaluation")
        .expect("manual evaluation");
    let first = store
        .set_manual_grade(
            context,
            SetManualGradeCommand {
                action: ManualGradeActionId::from_uuid(id()),
                actor: instructor,
                attempt: manual_attempt.id,
                expected_revision: evaluation.revision,
                credit: ManualCredit::parse("0.25").expect("credit"),
            },
        )
        .await
        .expect("first manual grade");
    let scoring_one = claim_scoring(&store, assignment).await;
    store
        .prepare_assignment_scoring(context, scoring_one)
        .await
        .expect("stage scoring generation one");
    assert_eq!(
        store.commit_assignment_scoring(context, scoring_one).await,
        Ok(AssignmentScoringCommitOutcome::Committed)
    );
    let stale_analysis = claim_analysis(&store, assignment).await;
    store
        .prepare_course_item_analysis(context, stale_analysis)
        .await
        .expect("stage analysis generation one");

    let correction = store
        .set_manual_grade(
            context,
            SetManualGradeCommand {
                action: ManualGradeActionId::from_uuid(id()),
                actor: instructor,
                attempt: manual_attempt.id,
                expected_revision: first.resulting_revision,
                credit: ManualCredit::parse("0.5").expect("corrected credit"),
            },
        )
        .await
        .expect("manual correction advances scoring generation");
    assert_eq!(
        store
            .commit_course_item_analysis(context, stale_analysis)
            .await,
        Ok(CourseItemAnalysisCommitOutcome::Superseded)
    );
    let scoring_two = claim_scoring(&store, assignment).await;
    assert_eq!(scoring_two.generation, correction.scoring_generation);
    store
        .prepare_assignment_scoring(context, scoring_two)
        .await
        .expect("stage corrected scoring");
    assert_eq!(
        store.commit_assignment_scoring(context, scoring_two).await,
        Ok(AssignmentScoringCommitOutcome::Committed)
    );
    let analysis = claim_analysis(&store, assignment).await;
    store
        .prepare_course_item_analysis(context, analysis)
        .await
        .expect("stage corrected analysis");
    assert_eq!(
        store.commit_course_item_analysis(context, analysis).await,
        Ok(CourseItemAnalysisCommitOutcome::Committed)
    );

    let instructor_session = session(
        &store,
        tenant,
        instructor,
        vec![UserRole::Instructor],
        b"item-analysis-instructor",
    )
    .await;
    let admin_session = session(
        &store,
        tenant,
        administrator,
        vec![UserRole::Administrator],
        b"item-analysis-admin",
    )
    .await;
    let student_session = session(
        &store,
        tenant,
        student,
        vec![UserRole::Student],
        b"item-analysis-student",
    )
    .await;
    let outsider_session = session(
        &store,
        tenant,
        outsider,
        vec![UserRole::Instructor],
        b"item-analysis-outsider",
    )
    .await;
    let foreign_session = session(
        &store,
        foreign_tenant,
        foreign_user,
        vec![UserRole::Administrator],
        b"item-analysis-foreign",
    )
    .await;
    let report = store
        .course_item_analysis(context, instructor_session, course, assignment)
        .await
        .expect("instructor report read")
        .expect("current report");
    assert_eq!(
        report.source_scoring_generation,
        correction.scoring_generation
    );
    assert_eq!(report.completed_run_count, 1);
    assert!(!report.incomplete_manual_grading);
    assert_eq!(report.assignment_average_score, Some(0.75));
    let completion_millis = report
        .average_completion_time_millis
        .expect("terminal mixed run has a completion interval");
    assert_eq!(completion_millis, terminal_submission_elapsed);
    assert_eq!(report.items.len(), 2);
    let manual_item = report
        .items
        .iter()
        .find(|item| item.reference == manual)
        .expect("manual item report");
    assert_eq!(manual_item.average_credit, Some(0.5));
    assert_eq!(
        manual_item.average_completion_time_millis,
        Some(completion_millis)
    );
    assert!(
        store
            .course_item_analysis(context, admin_session, course, assignment)
            .await
            .expect("admin read")
            .is_some()
    );
    assert!(
        store
            .course_item_analysis(context, student_session, course, assignment)
            .await
            .expect("student read")
            .is_none()
    );
    assert!(
        store
            .course_item_analysis(context, outsider_session, course, assignment)
            .await
            .expect("outsider read")
            .is_none()
    );
    assert!(
        store
            .course_item_analysis(foreign_context, foreign_session, course, assignment)
            .await
            .expect("foreign read")
            .is_none()
    );

    let rows: (i64, i64, String) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM course_item_analysis_current WHERE tenant_id = $1 AND assignment_id = $2), \
                (SELECT count(*) FROM course_item_analysis_staging WHERE tenant_id = $1), \
                (SELECT report_payload::text FROM course_item_analysis_current WHERE tenant_id = $1 AND assignment_id = $2)",
    ).bind(tenant.as_uuid()).bind(assignment.as_uuid()).fetch_one(&pool).await.expect("inspect current projection");
    assert_eq!((rows.0, rows.1), (1, 0));
    for forbidden in [
        "student-records/item-analysis-private.pdf",
        &manual_attempt.id.to_string(),
        &student.as_uuid().to_string(),
        "object_key",
    ] {
        assert!(
            !rows.2.contains(forbidden),
            "report leaked protected material: {forbidden}"
        );
    }
    let force_rls: bool = sqlx::query_scalar(
        "SELECT relforcerowsecurity FROM pg_class WHERE oid = 'public.course_item_analysis_current'::regclass",
    ).fetch_one(&pool).await.expect("inspect forced RLS");
    assert!(force_rls);
    let mut transaction = pool.begin().await.expect("RLS probe transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("assume application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(foreign_tenant.as_uuid().to_string())
        .execute(&mut *transaction)
        .await
        .expect("foreign tenant context");
    let visible: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM course_item_analysis_current WHERE assignment_id = $1",
    )
    .bind(assignment.as_uuid())
    .fetch_one(&mut *transaction)
    .await
    .expect("RLS read probe");
    assert_eq!(
        visible, 0,
        "forced RLS hides the tenant-owned report from a foreign application context"
    );
    transaction.rollback().await.expect("finish RLS probe");
}
