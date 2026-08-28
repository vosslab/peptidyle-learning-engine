#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for Instructor automated-grading operations.
//!
//! The normal Store path creates the course, published item, enrollment, and
//! issued attempt. The test then makes one accepted submission through the
//! server-owned broker and drives its recovery with the sealed worker
//! capability that owns terminal execution transitions. This keeps the
//! fixture small while exercising the public W5 capabilities with their real
//! roles, RLS policies, and lifecycle triggers.

#[path = "postgres_automated_grading_operations_live/broker.rs"]
mod broker;
use broker::{InstructorBroker, app_transaction};
#[path = "postgres_automated_grading_operations_live/accepted_completion.rs"]
mod accepted_completion;
#[path = "postgres_automated_grading_operations_live/assignment_definition.rs"]
mod assignment_definition;
#[path = "postgres_automated_grading_operations_live/manual_support.rs"]
mod manual_support;
#[path = "postgres_automated_grading_operations_live/receipt_integrity.rs"]
mod receipt_integrity;
#[path = "postgres_automated_grading_operations_live/recovery_worker.rs"]
mod recovery_worker;
#[path = "postgres_automated_grading_operations_live/retry.rs"]
mod retry;
#[path = "postgres_automated_grading_operations_live/scoring_worker.rs"]
mod scoring_worker;
#[path = "postgres_automated_grading_operations_live/source_witness_denials.rs"]
mod source_witness_denials;

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;
use published_assignment::create_published_assignment;

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;

use domain::effective_assignment_policy::BaseAssignmentPolicy;
use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AcceptedSubmissionCommand, AssignmentRecord, AutomatedGradingStore, CatalogStore, CourseRecord,
    CourseRosterStore, CreateCourseCommand, DraftRecord, FlatGradingCapability,
    GradingOperationGroupBy, GradingOperationStore, IssueQuestionAttemptCommand,
    IssuedQuestionFamilyWitnessV1, IssuedQuestionSnapshotV1, JobId, LearnerWorkRoutingBinding,
    ListInstructorGradingOperationsCommand, NativeExecutionEnvelopeCapability, PageRequest,
    PageSize, PresentationCapability, PublishDraftCommand, QtiGradingCapability, SessionTokenHash,
    Store, SubmissionIdempotencyKey, TenantContext, UpsertCourseMember, WebworkGradingCapability,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentInstructions,
    AssignmentItem, AssignmentItemId, AssignmentLifecycle, AssignmentScoringMode,
    AttemptProvenance, BackendCapabilities, Capability, CourseId, CourseTerm,
    DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, ImplementationVersion,
    PointValue, ProblemId, ProblemVersionRef, PublicationScope, QuestionAttemptId,
    QuestionMetadata, QuestionSource, RunId, ScoringStatus, StudentResponse, TenantId, UserId,
    UserRole, VersionId, WorkspaceId,
};
use sqlx::{PgPool, Row};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

fn fresh_uuid() -> Uuid {
    let sequence = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
    Uuid::from_u128(0x7a11_0000_0000_4000_8000_0000_0000_0000 | u128::from(sequence))
}

fn session_hash() -> String {
    format!("{:064x}", fresh_uuid().as_u128())
}

fn implementation(name: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: name.to_string(),
        version: "1".to_string(),
    }
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AnswerAll,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

async fn publish_question(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
) -> (ProblemVersionRef, IssuedQuestionSnapshotV1) {
    let publication = ProblemVersionRef {
        problem: ProblemId::from_uuid(fresh_uuid()),
        version: VersionId::from_uuid(fresh_uuid()),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(fresh_uuid()),
            source: DraftQuestionSource::Native {
                family: "automated_grading_operations_live".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Connected grading-operation fixture".to_string(),
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
                title: "Connected recovery item".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("save connected fixture draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft.clone(),
                expected_revision: saved.revision,
                publication,
                published_source: QuestionSource::Native {
                    family: "automated_grading_operations_live".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: instructor,
                scope: PublicationScope::Institution,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE connected fixture".to_string())
                        .expect("valid fixture byline"),
                ])
                .expect("valid fixture byline"),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish connected fixture question");
    let snapshot = IssuedQuestionSnapshotV1::new(
        question_model::QuestionDefinition::from_draft(
            draft.question,
            publication.problem,
            publication.version,
            QuestionSource::Native {
                family: "automated_grading_operations_live".to_string(),
            },
        ),
        IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("build exact issued fixture snapshot");
    (publication, snapshot)
}

async fn insert_session(
    pool: &PgPool,
    tenant: TenantId,
    user: UserId,
    roles: &[UserRole],
) -> String {
    let hash = session_hash();
    let role_names: Vec<&str> = roles
        .iter()
        .map(|role| match role {
            UserRole::Student => "student",
            UserRole::Instructor => "instructor",
            UserRole::Sysadmin => "sysadmin",
        })
        .collect();
    sqlx::query(
        "INSERT INTO public.auth_session \
         (session_hash, tenant_id, user_id, display_name, roles, created_at, expires_at) \
         VALUES ($1, $2, $3, 'Connected fixture', to_jsonb($4::text[]), \
                 transaction_timestamp(), transaction_timestamp() + interval '1 hour')",
    )
    .bind(&hash)
    .bind(tenant.as_uuid())
    .bind(user.as_uuid())
    .bind(role_names)
    .execute(pool)
    .await
    .expect("insert live session");
    hash
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_automated_grading_operations_live_oracle_is_brokered_replay_safe_and_projected() {
    let runtime = load_acceptance_runtime();
    let pool = lazy_pool(runtime.admin_url().expose()).expect("PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated PostgreSQL schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x51; 32]);

    let tenant = TenantId::from_uuid(fresh_uuid());
    let foreign_tenant = TenantId::from_uuid(fresh_uuid());
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(fresh_uuid());
    let other_instructor = UserId::from_uuid(fresh_uuid());
    let student = UserId::from_uuid(fresh_uuid());
    let outsider = UserId::from_uuid(fresh_uuid());
    let course = CourseId::from_uuid(fresh_uuid());
    let assignment = AssignmentId::from_uuid(fresh_uuid());

    let (question, snapshot) = publish_question(&store, context, tenant, instructor).await;
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Connected automated-grading course".to_string(),
                    term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                        .expect("fixture term"),
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
            title: "Connected recovery assignment".to_string(),
            lifecycle: AssignmentLifecycle::Published,
            instructions: AssignmentInstructions::default(),
            audience: AssignmentAudience::CourseWide,
            items: vec![AssignmentItem {
                id: AssignmentItemId::from_uuid(fresh_uuid()),
                reference: question,
                position: 0,
                points_possible: PointValue::from_whole(1),
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            selection_groups: Vec::new(),
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: policies(),
        },
        BaseAssignmentPolicy::default(),
    )
    .await
    .expect("create published assignment");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Connected learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create student membership and enrollment");

    // A second Instructor membership exercises action-id actor binding without
    // giving the fixture an extra course or assignment.
    sqlx::query(
        "INSERT INTO public.course_member \
         (tenant_id, course_id, course_membership_id, user_id, role, status, joined_at) \
         VALUES ($1,$2,$3,$4,'instructor','active',transaction_timestamp())",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(fresh_uuid())
    .bind(other_instructor.as_uuid())
    .execute(&pool)
    .await
    .expect("create second instructor membership");

    let run = store
        .start_or_resume_run(
            context,
            student,
            LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(fresh_uuid()),
        )
        .await
        .expect("create learner run");
    let attempt = QuestionAttemptId::from_uuid(fresh_uuid());
    store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                binding: LearnerWorkRoutingBinding::new(course, assignment),
                attempt,
                run: run.id,
                assignment_position: 0,
                problem: question.problem,
                question_version: question.version,
                issued_question_snapshot: snapshot.clone(),
                seed: 1,
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                native_execution_envelope_capability:
                    NativeExecutionEnvelopeCapability::NotApplicable,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                parameter_hash: "connected-grading-operation".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("connected-native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("connected-grade"),
                    rendered_question_sha256: "connected-render".to_string(),
                },
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("issue exact attempt");
    let original_job = JobId::from_uuid(fresh_uuid());
    let accepted = store
        .accept_automated_submission(
            context,
            AcceptedSubmissionCommand {
                actor: student,
                course,
                assignment,
                attempt,
                idempotency_key: SubmissionIdempotencyKey::parse("connected-grading-operation")
                    .expect("idempotency key"),
                response: StudentResponse::Numeric { value: 5.0 },
                execution_job: original_job,
            },
        )
        .await
        .expect("accept server-owned response");
    assert_eq!(
        accepted.attempt, attempt,
        "accepted record binds the issued attempt"
    );

    // The sealed recovery adapter owns both eligible-state selection and the
    // terminal transition. This connected oracle observes database evidence,
    // but never hand-assembles a worker claim or lifecycle transition.
    let recovery = recovery_worker::RecoveryWorker::connect(&runtime, tenant).await;
    recovery
        .fail_deterministically(original_job, accepted.submission)
        .await;
    let operation: i32 = sqlx::query_scalar(
        "SELECT grading_operation_id::integer FROM public.grading_operation \
         WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3 AND target_kind='submission'",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(accepted.submission.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("read canonical Instructor recovery thread");

    receipt_integrity::prove_subtype_shape_rejection(&pool, tenant, operation, course, instructor)
        .await;

    let instructor_session =
        insert_session(&pool, tenant, instructor, &[UserRole::Instructor]).await;
    let sysadmin_session = insert_session(&pool, tenant, instructor, &[UserRole::Sysadmin]).await;
    let rotated_instructor_session =
        insert_session(&pool, tenant, instructor, &[UserRole::Instructor]).await;
    let student_session = insert_session(&pool, tenant, student, &[UserRole::Student]).await;
    let outsider_session = insert_session(&pool, tenant, outsider, &[UserRole::Instructor]).await;
    let other_instructor_session =
        insert_session(&pool, tenant, other_instructor, &[UserRole::Instructor]).await;
    let foreign_session =
        insert_session(&pool, foreign_tenant, outsider, &[UserRole::Instructor]).await;

    let instructor_broker = InstructorBroker {
        pool: &pool,
        tenant,
        session: &instructor_session,
        course,
        assignment,
    };
    let rotated_instructor_broker = InstructorBroker {
        session: &rotated_instructor_session,
        ..instructor_broker
    };
    let sysadmin_broker = InstructorBroker {
        session: &sysadmin_session,
        ..instructor_broker
    };
    let student_broker = InstructorBroker {
        session: &student_session,
        ..instructor_broker
    };
    let outsider_broker = InstructorBroker {
        session: &outsider_session,
        ..instructor_broker
    };
    let other_instructor_broker = InstructorBroker {
        session: &other_instructor_session,
        ..instructor_broker
    };
    let foreign_broker = InstructorBroker {
        tenant: foreign_tenant,
        session: &foreign_session,
        ..instructor_broker
    };

    // The public page-size maximum must reach PostgreSQL as 100. The broker
    // owns the additional one-row overfetch used to derive next_cursor.
    let maximum_page = store
        .list_instructor_grading_operations(
            context,
            ListInstructorGradingOperationsCommand {
                tenant,
                session: SessionTokenHash::from_hex(&instructor_session)
                    .expect("valid instructor session hash"),
                course,
                assignment,
                group_by: GradingOperationGroupBy::Question,
                page: PageRequest::first(PageSize::new(PageSize::MAX).expect("maximum page size")),
            },
        )
        .await
        .expect("maximum Instructor page size remains accepted");
    assert_eq!(maximum_page.items.len(), 1);
    assert!(maximum_page.next_cursor.is_none());

    let question_rows = instructor_broker.list("question", None).await;
    let question_row = question_rows
        .first()
        .expect("Instructor sees exact recovery operation");
    assert_eq!(
        question_row.try_get::<String, _>("group_kind").unwrap(),
        "question"
    );
    assert_eq!(
        question_row.try_get::<String, _>("question_title").unwrap(),
        "Connected recovery item"
    );
    assert!(
        question_row
            .try_get::<Option<String>, _>("learner_display_name")
            .unwrap()
            .is_some()
    );
    assert!(
        question_row
            .try_get::<Option<i64>, _>("execution_generation")
            .unwrap()
            .is_some()
    );
    assert!(
        instructor_broker
            .list("learner", None)
            .await
            .first()
            .expect("learner grouping")
            .try_get::<String, _>("group_key")
            .unwrap()
            .starts_with("l:")
    );
    assert!(
        student_broker.list("question", None).await.is_empty(),
        "Student receives concealed operation list"
    );
    assert!(
        sysadmin_broker.list("question", None).await.is_empty(),
        "Sysadmin role without explicit Instructor authority receives concealed operation list"
    );
    assert!(
        outsider_broker.list("question", None).await.is_empty(),
        "unrelated Instructor receives concealed operation list"
    );
    assert!(
        foreign_broker.list("question", None).await.is_empty(),
        "foreign tenant cannot name a local course"
    );

    retry::prove_retry_and_replay(retry::RetryScenario {
        pool: &pool,
        tenant,
        operation,
        attempt,
        submission: accepted.submission.as_uuid(),
        instructor_broker: &instructor_broker,
        rotated_instructor_broker: &rotated_instructor_broker,
        other_instructor_broker: &other_instructor_broker,
    })
    .await;

    let retry_job: Uuid = sqlx::query_scalar(
        "SELECT current_job_id FROM public.grading_execution \
         WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(accepted.submission.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("read retry-owned execution job");
    recovery
        .fail_terminally(JobId::from_uuid(retry_job), accepted.submission)
        .await;
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT state FROM public.grading_operation WHERE tenant_id=$1 AND grading_operation_id=$2",
        )
        .bind(tenant.as_uuid())
        .bind(i64::from(operation))
        .fetch_one(&pool)
        .await
        .expect("read reopened projection"),
        "actionable",
        "the later sealed terminal failure reopens the original Instructor thread"
    );

    let assignment_revision: i64 = sqlx::query_scalar(
        "SELECT revision FROM public.assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("read current assignment revision");
    let recalculate_action = fresh_uuid();
    let accepted_recalculation = instructor_broker
        .recalculate(assignment_revision, recalculate_action)
        .await
        .expect("recalculation accepted");
    let scoring_generation = accepted_recalculation
        .try_get::<i64, _>("scoring_generation")
        .expect("scoring generation");
    let scoring_operation = accepted_recalculation
        .try_get::<i32, _>("operation_reference")
        .expect("scoring operation reference");
    assert_eq!(
        accepted_recalculation
            .try_get::<String, _>("disposition")
            .unwrap(),
        "accepted"
    );
    let origin = sqlx::query(
        "SELECT origin_kind, origin_id, actor_id, scoring_generation, \
                recalculation_job_id, grading_operation_id \
         FROM public.scoring_invalidation_origin \
         WHERE tenant_id=$1 AND origin_kind='instructor_recalculation' AND origin_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(recalculate_action)
    .fetch_one(&pool)
    .await
    .expect("read immutable Instructor invalidation origin");
    assert_eq!(
        origin.try_get::<String, _>("origin_kind").unwrap(),
        "instructor_recalculation",
        "the source-specific broker retains a typed origin rather than free text"
    );
    assert_eq!(
        origin.try_get::<Uuid, _>("origin_id").unwrap(),
        recalculate_action,
        "the action is the stable causal identity"
    );
    assert_eq!(
        origin.try_get::<Uuid, _>("actor_id").unwrap(),
        instructor.as_uuid(),
        "Instructor-origin evidence retains its actor"
    );
    assert_eq!(
        origin.try_get::<i64, _>("scoring_generation").unwrap(),
        scoring_generation,
        "the origin links the exact requested generation"
    );
    assert_eq!(
        origin.try_get::<Uuid, _>("recalculation_job_id").unwrap(),
        recalculate_action,
        "the origin links the deterministic 1830 job"
    );
    assert_eq!(
        origin.try_get::<i64, _>("grading_operation_id").unwrap(),
        i64::from(scoring_operation),
        "the origin links the Instructor-visible thread"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT scoring_status FROM public.assignment WHERE tenant_id=$1 AND assignment_id=$2",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .fetch_one(&pool)
        .await
        .expect("recalculation has not directly published a score"),
        "recalculating"
    );
    assert_eq!(
        rotated_instructor_broker
            .recalculate(assignment_revision, recalculate_action)
            .await
            .expect("same actor recalculation replay")
            .try_get::<String, _>("disposition")
            .unwrap(),
        "replayed"
    );
    assert!(
        instructor_broker
            .recalculate(assignment_revision, fresh_uuid())
            .await
            .is_err(),
        "recalculating assignment refuses a second action"
    );
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>(
            "SELECT job_id FROM public.worker_job WHERE tenant_id=$1 AND job_id=$2",
        )
        .bind(tenant.as_uuid())
        .bind(recalculate_action)
        .fetch_one(&pool)
        .await
        .expect("one deterministic recalculation job"),
        recalculate_action
    );
    scoring_worker::publish(
        &store,
        context,
        assignment,
        recalculate_action,
        scoring_generation,
    )
    .await;
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT state FROM public.grading_operation WHERE tenant_id=$1 AND grading_operation_id=$2",
        )
        .bind(tenant.as_uuid())
        .bind(i64::from(scoring_operation))
        .fetch_one(&pool)
        .await
        .expect("read completed scoring projection"),
        "completed"
    );
    let accepted_completion_origin = accepted_completion::prove_accepted_completion_origin(
        accepted_completion::AcceptedCompletionScenario {
            runtime: &runtime,
            store: &store,
            pool: &pool,
            context,
            tenant,
            instructor,
            course,
            assignment,
            question,
            snapshot: snapshot.clone(),
        },
    )
    .await;
    scoring_worker::publish(
        &store,
        context,
        assignment,
        accepted_completion_origin.recalculation_job,
        accepted_completion_origin.scoring_generation,
    )
    .await;
    let learner_summary = store
        .learner_get_summary(
            context,
            accepted_completion_origin.student,
            accepted_completion_origin.enrollment,
        )
        .await
        .expect("read learner-visible summary after scoring publication")
        .expect("accepted-completion learner remains enrolled");
    assert_eq!(learner_summary.scoring_status, ScoringStatus::Current);
    assert_eq!(learner_summary.summary.current_score, Some(1.0));
    assert_eq!(learner_summary.summary.completed_run_count, 1);
    assignment_definition::prove_assignment_definition_origin(
        assignment_definition::AssignmentDefinitionScenario {
            store: &store,
            pool: &pool,
            context,
            tenant,
            instructor,
            course,
            assignment,
        },
    )
    .await;
    manual_support::prove_manual_grade_and_support_origins(manual_support::ManualSupportScenario {
        store: &store,
        pool: &pool,
        context,
        tenant,
        instructor,
        course,
        assignment,
        question,
        snapshot,
    })
    .await;

    let mut app = app_transaction(&pool, tenant, &instructor_session).await;
    assert!(
        sqlx::query("SELECT * FROM public.grading_operation")
            .fetch_all(&mut *app)
            .await
            .is_err(),
        "ple_app has no direct grading-operation read"
    );
    app.rollback()
        .await
        .expect("rollback denied grading-operation read");
    let mut app = app_transaction(&pool, tenant, &instructor_session).await;
    assert!(
        sqlx::query("SELECT nextval('public.grading_operation_grading_operation_id_seq')")
            .execute(&mut *app)
            .await
            .is_err(),
        "ple_app has no grading-operation sequence capability"
    );
    app.rollback()
        .await
        .expect("rollback denied grading-operation sequence access");
    let mut app = app_transaction(&pool, tenant, &instructor_session).await;
    assert!(
        sqlx::query("SELECT public.ple_instructor_grading_operation_actor_v1($1,$2,$3,$4)")
            .bind(tenant.as_uuid())
            .bind(&instructor_session)
            .bind(course.as_uuid())
            .bind(assignment.as_uuid())
            .execute(&mut *app)
            .await
            .is_err(),
        "only the three public W5 capabilities are executable by ple_app"
    );
    app.rollback()
        .await
        .expect("rollback denied private function execution");
    let mut app = app_transaction(&pool, tenant, &instructor_session).await;
    assert!(
        sqlx::query(
            "UPDATE public.scoring_invalidation_origin \
             SET actor_id=$3 WHERE tenant_id=$1 AND origin_id=$2",
        )
        .bind(tenant.as_uuid())
        .bind(recalculate_action)
        .bind(other_instructor.as_uuid())
        .execute(&mut *app)
        .await
        .is_err(),
        "ple_app cannot mutate immutable causal evidence"
    );
    app.rollback()
        .await
        .expect("rollback denied causal-table mutation");
    source_witness_denials::prove_source_witness_denials(
        source_witness_denials::SourceWitnessDenialScenario {
            pool: &pool,
            tenant,
            session: &instructor_session,
            instructor,
            course,
            assignment,
            assignment_revision,
            original_execution_job: original_job.as_uuid(),
            attempt: attempt.as_uuid(),
            submission: accepted.submission.as_uuid(),
        },
    )
    .await;

    // Revocation is a current-authority boundary even when the historical
    // recovery record and its public learner reference remain durable.
    sqlx::query(
        "UPDATE public.course_member SET status='revoked', revoked_at=transaction_timestamp() \
         WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3 AND role='instructor'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(instructor.as_uuid())
    .execute(&pool)
    .await
    .expect("revoke fixture Instructor membership");
    assert!(
        instructor_broker.list("question", None).await.is_empty(),
        "revoked Instructor receives concealed operation list"
    );
}
