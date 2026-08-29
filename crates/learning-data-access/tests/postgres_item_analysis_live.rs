#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for automated-only course item analysis.
//!
//! Store APIs own course, assignment, issued-attempt, accepted-submission,
//! worker, and report behavior. Direct SQL identifies this fixture tenant's
//! safe queue references and attests persisted aggregate shape and database
//! RLS; it never reads response material.

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;
use published_assignment::create_published_assignment;
#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;

use acceptance_runtime::load as load_acceptance_runtime;
use learning_data_access::postgres::{
    PostgresAcceptedSubmissionRecoveryStore, PostgresStore, lazy_pool,
    local_accepted_submission_recovery_pool, verify_application_schema,
};
use learning_data_access::{
    AcceptedSubmissionCommand, AcceptedSubmissionExecutionDisposition,
    AcceptedSubmissionExecutionOutcome, AcceptedSubmissionExecutionRecoveryClaimStore,
    AcceptedSubmissionExecutionStore, AcceptedSubmissionGrade, AssignmentRecord,
    AssignmentScoringCommitOutcome, AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore,
    AutomatedGradingStore, CatalogStore, CourseItemAnalysisCommitOutcome, CourseItemAnalysisStore,
    CourseItemAnalysisWorkerCommand, CourseItemAnalysisWorkerStore, CourseRecord,
    CourseRosterStore, CreateCourseCommand, DraftRecord, FlatGradingCapability,
    ForceSubmitAttemptCommand, IssueQuestionAttemptCommand, IssuedQuestionFamilyWitnessV1,
    IssuedQuestionSnapshotV1, JobId, JobKind, JobLeaseDuration, JobPayload, JobStore,
    NativeExecutionEnvelopeCapability, PresentationCapability, PublishDraftCommand,
    QtiGradingCapability, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store,
    StudentWorkRoutingBinding, SubmissionIdempotencyKey, TenantContext, UpsertCourseMember,
    WebworkGradingCapability, WorkerId, canonical_attempt_result_json,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::{ResponseDefinition, StudentResponse};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentScoringMode,
    AttemptProvenance, AttemptResult, BackendCapabilities, Capability, CourseId,
    DraftQuestionDefinition, DraftQuestionSource, FeedbackContent, GradingDefinition,
    ImplementationVersion, PointValue, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionAttempt, QuestionAttemptId, QuestionMetadata, QuestionSource, RunId, TenantId, UserId,
    UserRole, VersionId, WorkspaceId,
};
use sqlx::Row;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn assert_no_private_report_keys(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(fields) => {
            for (field, nested) in fields {
                assert!(
                    !matches!(
                        field.as_str(),
                        "student_id"
                            | "user_id"
                            | "learner_id"
                            | "attempt_id"
                            | "response"
                            | "raw_response"
                            | "answer"
                            | "answer_key"
                            | "object_key"
                    ),
                    "persisted aggregate report excludes private field {field}"
                );
                assert_no_private_report_keys(nested);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                assert_no_private_report_keys(nested);
            }
        }
        _ => {}
    }
}

fn implementation(name: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: name.to_string(),
        version: "1".to_string(),
    }
}

fn provenance(label: &str) -> AttemptProvenance {
    AttemptProvenance {
        adapter: implementation("live-native"),
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: implementation("numeric"),
        rendered_question_sha256: format!("item-analysis-live-{label}"),
    }
}

fn draft_question(workspace: WorkspaceId, title: &str) -> DraftQuestionDefinition {
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: "numeric".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: format!("Live item-analysis {title}"),
        }],
        response: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Absolute { epsilon: 0.01 },
            unit: None,
        },
        attempt_policy: AttemptPolicy { max_attempts: None },
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

async fn publish_question(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
    title: &str,
) -> (ProblemVersionRef, IssuedQuestionSnapshotV1) {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let draft = DraftRecord {
        tenant,
        question: draft_question(WorkspaceId::from_uuid(id()), title),
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("save live draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft.clone(),
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "numeric".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: instructor,
                scope: PublicationScope::Institution,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("byline"),
                ])
                .expect("valid byline"),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish live question");
    let snapshot = IssuedQuestionSnapshotV1::new(
        question_model::QuestionDefinition::from_draft(
            draft.question,
            reference.problem,
            reference.version,
            QuestionSource::Native {
                family: "numeric".to_string(),
            },
        ),
        IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("issued snapshot");
    (reference, snapshot)
}

fn item(reference: ProblemVersionRef, position: u32) -> AssignmentItem {
    AssignmentItem {
        id: AssignmentItemId::from_uuid(id()),
        reference,
        position,
        points_possible: PointValue::from_whole(1),
        delivery_state: AssignmentDeliveryState::Active,
        scoring_mode: AssignmentScoringMode::Normal,
    }
}

struct IssuedAttemptFixture<'a> {
    store: &'a PostgresStore,
    context: TenantContext,
    student: UserId,
    course: CourseId,
    assignment: AssignmentId,
    run: RunId,
}

impl IssuedAttemptFixture<'_> {
    async fn issue(
        &self,
        position: u32,
        reference: ProblemVersionRef,
        snapshot: IssuedQuestionSnapshotV1,
    ) -> QuestionAttempt {
        self.store
            .issue_or_resume_question_attempt(
                self.context,
                IssueQuestionAttemptCommand {
                    actor: self.student,
                    binding: StudentWorkRoutingBinding::new(self.course, self.assignment),
                    attempt: QuestionAttemptId::from_uuid(id()),
                    run: self.run,
                    assignment_position: position,
                    problem: reference.problem,
                    question_version: reference.version,
                    issued_question_snapshot: snapshot,
                    seed: u64::from(position) + 1,
                    presentation_capability: PresentationCapability::NotApplicable,
                    presentation: None,
                    presentation_snapshot: None,
                    grading_envelope: None,
                    native_execution_envelope_capability:
                        NativeExecutionEnvelopeCapability::Required,
                    flat_grading: None,
                    flat_grading_capability: FlatGradingCapability::NotApplicable,
                    webwork_grading: None,
                    webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                    qti_grading: None,
                    qti_grading_capability: QtiGradingCapability::NotApplicable,
                    parameter_hash: format!("live-analysis-{position}"),
                    provenance: provenance(&position.to_string()),
                    webwork_replay: None,
                    prefetched: None,
                    predecessor_submission: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("issue live item {position}: {error:?}"))
    }
}

async fn accept(
    store: &PostgresStore,
    context: TenantContext,
    student: UserId,
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
    key: &str,
) {
    store
        .accept_automated_submission(
            context,
            AcceptedSubmissionCommand {
                actor: student,
                course,
                assignment,
                attempt,
                idempotency_key: SubmissionIdempotencyKey::parse(key).expect("idempotency key"),
                response: StudentResponse::Numeric { value: 42.0 },
                execution_job: learning_data_access::JobId::from_uuid(id()),
            },
        )
        .await
        .expect("accept automated submission");
}

async fn complete(
    store: &PostgresAcceptedSubmissionRecoveryStore,
    context: TenantContext,
    terminal_failure: bool,
) {
    let claim = store
        .claim_next_accepted_submission_execution(
            WorkerId::from_uuid(id()),
            JobLeaseDuration::from_seconds(30).expect("worker lease"),
        )
        .await
        .expect("claim execution")
        .expect("execution is claimable");
    let outcome = if terminal_failure {
        AcceptedSubmissionExecutionOutcome::TerminalFailure
    } else {
        AcceptedSubmissionExecutionOutcome::Evaluated {
            grade: AcceptedSubmissionGrade {
                evidence: canonical_attempt_result_json(AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                })
                .expect("canonical grade"),
                feedback: FeedbackContent::default(),
            },
        }
    };
    assert!(matches!(
        store
            .commit_or_fail_accepted_submission_execution(context, claim, outcome)
            .await
            .expect("complete execution"),
        AcceptedSubmissionExecutionDisposition::Committed
            | AcceptedSubmissionExecutionDisposition::Terminal
    ));
}

async fn fixture_job_id(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    assignment: AssignmentId,
    kind: JobKind,
) -> Option<JobId> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT job_id FROM worker_job \
         WHERE tenant_id = $1 AND state = 'ready' \
           AND available_at <= transaction_timestamp() \
           AND payload ->> 'kind' = $2 AND payload ->> 'assignment' = $3 \
         ORDER BY available_at, job_id LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .bind(kind.database_name())
    .bind(assignment.to_string())
    .fetch_optional(pool)
    .await
    .expect("read fixture-owned queue reference")
    .map(JobId::from_uuid)
}

async fn claim_fixture_job(
    pool: &sqlx::PgPool,
    store: &PostgresStore,
    tenant: TenantId,
    assignment: AssignmentId,
    kind: JobKind,
) -> Option<learning_data_access::ClaimedJob> {
    let job = fixture_job_id(pool, tenant, assignment, kind).await?;
    store
        .claim_exact_job(
            job,
            kind,
            JobLeaseDuration::from_seconds(30).expect("worker lease"),
        )
        .await
        .expect("claim exact fixture job")
}

async fn run_analysis(
    pool: &sqlx::PgPool,
    store: &PostgresStore,
    context: TenantContext,
    assignment: AssignmentId,
) {
    while let Some(claim) = claim_fixture_job(
        pool,
        store,
        context.tenant_id(),
        assignment,
        JobKind::RecalculateAssignment,
    )
    .await
    {
        let JobPayload::RecalculateAssignment {
            assignment: job_assignment,
            generation,
        } = claim.payload
        else {
            panic!("exact scoring claim returned another job family")
        };
        let command = AssignmentScoringWorkerCommand {
            job: claim.id,
            lease: claim.lease_token,
            assignment: job_assignment,
            generation,
        };
        store
            .prepare_assignment_scoring(context, command)
            .await
            .expect("prepare scoring");
        assert!(matches!(
            store
                .commit_assignment_scoring(context, command)
                .await
                .expect("commit scoring"),
            AssignmentScoringCommitOutcome::Committed | AssignmentScoringCommitOutcome::Superseded
        ));
    }

    loop {
        let claim = claim_fixture_job(
            pool,
            store,
            context.tenant_id(),
            assignment,
            JobKind::RecalculateCourseItemAnalysis,
        )
        .await
        .expect("fixture item-analysis job");
        let JobPayload::RecalculateCourseItemAnalysis {
            assignment: job_assignment,
            generation,
        } = claim.payload
        else {
            panic!("exact analysis claim returned another job family")
        };
        let command = CourseItemAnalysisWorkerCommand {
            job: claim.id,
            lease: claim.lease_token,
            assignment: job_assignment,
            generation,
        };
        store
            .prepare_course_item_analysis(context, command)
            .await
            .expect("prepare analysis");
        let outcome = store
            .commit_course_item_analysis(context, command)
            .await
            .expect("commit analysis");
        if outcome == CourseItemAnalysisCommitOutcome::Committed {
            return;
        }
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_item_analysis_is_current_private_and_generation_fenced() {
    let runtime = load_acceptance_runtime();
    let pool = lazy_pool(runtime.admin_url().expose()).expect("PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated PostgreSQL schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x42; 32]);
    let recovery_pool = local_accepted_submission_recovery_pool(runtime.recovery_url().expose())
        .await
        .expect("accepted-submission recovery pool");
    let recovery_store = PostgresAcceptedSubmissionRecoveryStore::from_recovery_pool(recovery_pool);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(TenantId::from_uuid(id()));
    let instructor = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    let assignment = AssignmentId::from_uuid(id());
    let references = vec![
        publish_question(&store, context, tenant, instructor, "graded").await,
        publish_question(&store, context, tenant, instructor, "exception").await,
        publish_question(&store, context, tenant, instructor, "pending").await,
        publish_question(&store, context, tenant, instructor, "unanswered").await,
    ];
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Live automated analysis".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("course term"),
                },
                authority: sysadmin_course_creation_authority(&store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("create course");
    create_published_assignment(
        &store,
        context,
        instructor,
        AssignmentRecord {
            id: assignment,
            tenant,
            course_id: course,
            title: "Live automated analysis assignment".to_string(),
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            audience: question_model::AssignmentAudience::CourseWide,
            items: references
                .iter()
                .enumerate()
                .map(|(position, (reference, _))| {
                    item(*reference, u32::try_from(position).expect("position"))
                })
                .collect(),
            selection_groups: Vec::new(),
            disclosure_policy: question_model::StudentDisclosurePolicy::default(),
            policies: RunPolicies {
                completion: CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: ContinuedPractice::Unlimited,
                variation: VariationPolicy::NewSeeds,
            },
        },
        question_model::BaseAssignmentPolicy::default(),
    )
    .await
    .expect("publish assignment");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Live analysis student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("enroll student");
    let session = SessionTokenHash::compute(b"live-item-analysis-instructor");
    store
        .create_session(
            session,
            SessionSubject::new(
                tenant,
                instructor,
                "Live analysis Instructor",
                vec![UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("instructor session");
    let student_session = SessionTokenHash::compute(b"live-item-analysis-student");
    store
        .create_session(
            student_session,
            SessionSubject::new(
                tenant,
                student,
                "Live analysis Student",
                vec![UserRole::Student],
            )
            .expect("student session subject"),
            SessionLifetime::from_seconds(3_600).expect("student session lifetime"),
        )
        .await
        .expect("student session");
    let run = store
        .start_or_resume_run(
            context,
            student,
            StudentWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(id()),
        )
        .await
        .expect("start run");
    let issued_attempts = IssuedAttemptFixture {
        store: &store,
        context,
        student,
        course,
        assignment,
        run: run.id,
    };
    let [
        (graded_reference, graded_snapshot),
        (exception_reference, exception_snapshot),
        (pending_reference, pending_snapshot),
        (unanswered_reference, unanswered_snapshot),
    ] = references
        .try_into()
        .expect("four item-analysis question definitions");
    let graded_attempt = issued_attempts
        .issue(0, graded_reference, graded_snapshot)
        .await;
    accept(
        &store,
        context,
        student,
        course,
        assignment,
        graded_attempt.id,
        "live-analysis-graded",
    )
    .await;
    complete(&recovery_store, context, false).await;
    let exception_attempt = issued_attempts
        .issue(1, exception_reference, exception_snapshot)
        .await;
    accept(
        &store,
        context,
        student,
        course,
        assignment,
        exception_attempt.id,
        "live-analysis-exception",
    )
    .await;
    complete(&recovery_store, context, true).await;
    let pending_attempt = issued_attempts
        .issue(2, pending_reference, pending_snapshot)
        .await;
    accept(
        &store,
        context,
        student,
        course,
        assignment,
        pending_attempt.id,
        "live-analysis-pending",
    )
    .await;
    let unanswered_attempt = issued_attempts
        .issue(3, unanswered_reference, unanswered_snapshot)
        .await;
    store
        .force_submit_attempt(
            context,
            ForceSubmitAttemptCommand {
                action: learning_data_access::AttemptSupportActionId::from_uuid(id()),
                actor: instructor,
                attempt: unanswered_attempt.id,
            },
        )
        .await
        .expect("force-submit unanswered attempt");
    run_analysis(&pool, &store, context, assignment).await;

    let report = store
        .course_item_analysis(context, session, course, assignment)
        .await
        .expect("read report")
        .expect("current report");
    assert!(report.incomplete_scoring);
    assert_eq!(report.assignment_average_score, None);
    assert_eq!(
        report
            .items
            .iter()
            .map(|row| row.unscored_attempt_count)
            .sum::<u32>(),
        2
    );
    assert_eq!(
        report
            .items
            .iter()
            .map(|row| row.response_distribution.unanswered)
            .sum::<u32>(),
        1
    );
    assert_eq!(
        store
            .student_class_statistics(context, student, course, assignment)
            .await
            .expect("student statistics"),
        question_model::StudentClassStatistics::InsufficientEvidence
    );
    assert_eq!(
        store
            .course_item_analysis(foreign_context, session, course, assignment)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .course_item_analysis(context, student_session, course, assignment)
            .await,
        Ok(None),
        "same-tenant Student membership does not grant Instructor analysis access"
    );

    // A later automated completion advances assignment scoring while this
    // immutable report remains fenced to its published generation.
    complete(&recovery_store, context, false).await;
    assert!(
        store
            .course_item_analysis(context, session, course, assignment)
            .await
            .expect("read fenced report")
            .expect("previous generation remains readable")
            .recent_rescoring
    );

    let persisted: serde_json::Value = sqlx::query_scalar("SELECT report_payload FROM course_item_analysis_current WHERE tenant_id = $1 AND assignment_id = $2")
        .bind(tenant.as_uuid()).bind(assignment.as_uuid()).fetch_one(&pool).await.expect("persisted report");
    let serialized = persisted.to_string();
    assert!(serialized.contains("incomplete_scoring"));
    assert!(serialized.contains("unscored_attempt_count"));
    assert_no_private_report_keys(&persisted);
    for private_value in [student.to_string(), graded_attempt.id.to_string()] {
        assert!(
            !serialized.contains(&private_value),
            "aggregate report excludes private answer or identity"
        );
    }
    let report_table = sqlx::query("SELECT relrowsecurity, relforcerowsecurity FROM pg_class WHERE oid = 'public.course_item_analysis_current'::regclass")
        .fetch_one(&pool).await.expect("current report table");
    assert!(
        report_table
            .try_get::<bool, _>("relrowsecurity")
            .expect("RLS enabled")
    );
    assert!(
        report_table
            .try_get::<bool, _>("relforcerowsecurity")
            .expect("RLS forced")
    );
    let mut restricted = pool.begin().await.expect("restricted RLS transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *restricted)
        .await
        .expect("application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(foreign_context.tenant_id().to_string())
        .execute(&mut *restricted)
        .await
        .expect("foreign tenant fence");
    let foreign_visible: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM course_item_analysis_current \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_one(&mut *restricted)
    .await
    .expect("restricted report query");
    assert_eq!(foreign_visible, 0, "RLS conceals the known foreign report");
    restricted.rollback().await.expect("end RLS transaction");
}
