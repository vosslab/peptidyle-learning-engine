#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for WP-PROF-S4 assignment disclosure policy.
//!
//! Store commands create the educational state.  SQL is limited to the
//! PostgreSQL-only promises: closed columns, forced RLS, and retention fences.

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;
use published_assignment::create_published_assignment;

use domain::disclosure_policy::evaluate_learner_disclosure;
use domain::effective_assignment_policy::{
    AuthorizationGate, BaseAssignmentPolicy, EffectivePolicyDecision,
};
use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, AssignmentUpdate, CatalogStore, CourseRecord, CourseRosterStore,
    CreateCourseCommand, DraftRecord, PutAssignmentTeachingSettingsCommand,
    ResolveEffectivePolicyCommand, Store, StoreError, TenantContext, UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentItem,
    AssignmentItemId, AssignmentScoringMode, BackendCapabilities, Capability, CourseId,
    DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, LearnerDisclosurePolicy,
    LearnerDisclosureTiming, PointValue, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionMetadata, QuestionSource, ResponseDefinition, TenantId, UserId, VersionId, WorkspaceId,
};
use sqlx::Row;
use uuid::Uuid;

const TERM_BASE_MILLIS: i64 = 1_787_590_800_000;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AnswerAll,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

fn disclosure_policy() -> LearnerDisclosurePolicy {
    LearnerDisclosurePolicy {
        score: LearnerDisclosureTiming::AfterDue,
        per_item_correctness: LearnerDisclosureTiming::AfterClose,
        feedback_text: LearnerDisclosureTiming::Never,
        solution: LearnerDisclosureTiming::Never,
        class_statistics: LearnerDisclosureTiming::Never,
    }
}

async fn publish_question(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
) -> ProblemVersionRef {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id()),
            source: DraftQuestionSource::Native {
                family: "molar_mass".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "S4 disclosure fixture".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "S4 disclosure fixture".to_string(),
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
        .expect("save disclosure fixture draft");
    store
        .publish_draft(
            context,
            instructor,
            learning_data_access::PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "molar_mass".to_string(),
                },
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid byline"),
                ])
                .expect("valid byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish disclosure fixture question");
    reference
}

fn assignment(
    tenant: TenantId,
    course: CourseId,
    assignment_id: AssignmentId,
    reference: ProblemVersionRef,
) -> AssignmentRecord {
    AssignmentRecord {
        id: assignment_id,
        tenant,
        course_id: course,
        title: "S4 disclosure live fixture".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Published,
        instructions: question_model::AssignmentInstructions::default(),
        audience: AssignmentAudience::CourseWide,
        items: vec![AssignmentItem {
            id: AssignmentItemId::from_uuid(id()),
            reference,
            position: 0,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        }],
        selection_groups: Vec::new(),
        disclosure_policy: disclosure_policy(),
        policies: policies(),
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_assignment_disclosure_policy_is_closed_revisioned_current_and_rls_bound() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("full migrated application schema is compatible");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x42; 32]);
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let course = CourseId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    let assignment_id = AssignmentId::from_uuid(id());

    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "S4 disclosure policy course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("valid fixture term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("create S4 fixture course");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "S4 learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create S4 learner membership");
    let reference = publish_question(&store, context, tenant, instructor).await;

    let created = create_published_assignment(
        &store,
        context,
        instructor,
        assignment(tenant, course, assignment_id, reference),
        BaseAssignmentPolicy::default(),
    )
    .await
    .expect("create explicit disclosure policy");
    assert_eq!(created.record.disclosure_policy, disclosure_policy());

    let row = sqlx::query(
        "SELECT score_disclosure, per_item_correctness_disclosure, feedback_text_disclosure, \
                solution_disclosure, class_statistics_disclosure \
         FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment_id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("round-trip explicit policy columns");
    assert_eq!(row.get::<String, _>("score_disclosure"), "after_due");
    assert_eq!(
        row.get::<String, _>("per_item_correctness_disclosure"),
        "after_close"
    );
    assert_eq!(row.get::<String, _>("feedback_text_disclosure"), "never");
    assert_eq!(row.get::<String, _>("solution_disclosure"), "never");
    assert_eq!(row.get::<String, _>("class_statistics_disclosure"), "never");

    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = 'assignment' \
         AND column_name IN ('score_disclosure', 'per_item_correctness_disclosure', \
             'feedback_text_disclosure', 'solution_disclosure', 'class_statistics_disclosure', \
             'feedback_disclosure') ORDER BY column_name",
    )
    .fetch_all(&pool)
    .await
    .expect("inspect disclosure columns");
    assert_eq!(
        columns.len(),
        5,
        "the retired assignment disclosure column is absent"
    );
    let legacy_columns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns \
         WHERE table_schema = 'public' AND column_name IN \
             ('feedback_disclosure', 'issued_feedback_disclosure')",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect retired disclosure columns");
    assert_eq!(legacy_columns, 0, "no legacy disclosure authority remains");
    let no_defaults: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = 'assignment' \
           AND column_name IN ('score_disclosure', 'per_item_correctness_disclosure', \
               'feedback_text_disclosure', 'solution_disclosure', 'class_statistics_disclosure') \
           AND column_default IS NULL",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect closed write columns");
    assert_eq!(
        no_defaults, 5,
        "new writes must name every disclosure field"
    );

    let changed_policy = disclosure_policy();
    let update = AssignmentUpdate {
        title: "S4 disclosure revision fixture".to_string(),
        audience: created.record.audience.clone(),
        items: created.record.items.clone(),
        selection_groups: Vec::new(),
        disclosure_policy: changed_policy,
        policies: created.record.policies,
    };
    let updated = store
        .replace_assignment(
            context,
            course,
            assignment_id,
            created.revision,
            update.clone(),
        )
        .await
        .expect("current revision updates disclosure policy");
    assert_eq!(updated.record.disclosure_policy, changed_policy);
    assert_eq!(
        store
            .replace_assignment(context, course, assignment_id, created.revision, update,)
            .await,
        Err(StoreError::Conflict),
        "stale revision cannot overwrite disclosure policy"
    );

    store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment: assignment_id,
                expected_revision: updated.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy {
                        available_at: Some(ActivityTimestamp::from_unix_millis(TERM_BASE_MILLIS)),
                        due_at: Some(ActivityTimestamp::from_unix_millis(
                            TERM_BASE_MILLIS + 1_000,
                        )),
                        closes_at: Some(ActivityTimestamp::from_unix_millis(
                            TERM_BASE_MILLIS + 2_000,
                        )),
                        time_limit_seconds: None,
                        attempt_limit: None,
                        late_submission: question_model::LateSubmissionPolicy::Accept,
                        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
                    },
                },
            },
        )
        .await
        .expect("set S3 due and close authority");
    let entitlement = store
        .evaluate_assignment_entitlement(context, learner, course, assignment_id)
        .await
        .expect("S5 entitlement for current S3 evaluation");
    let before_due = store
        .resolve_effective_policy(
            context,
            ResolveEffectivePolicyCommand {
                assignment: assignment_id,
                entitlement: entitlement.clone(),
                authorization: AuthorizationGate::Authorized,
                now: ActivityTimestamp::from_unix_millis(TERM_BASE_MILLIS + 999),
                prior_run_count: 0,
            },
        )
        .await
        .expect("resolve current S3 policy before due")
        .expect("assignment exists");
    let before = evaluate_learner_disclosure(
        updated.record.disclosure_policy,
        &before_due.decision,
        ActivityTimestamp::from_unix_millis(TERM_BASE_MILLIS + 999),
        Some(ActivityTimestamp::from_unix_millis(TERM_BASE_MILLIS + 10)),
    )
    .expect("S3/S5 allowed decision evaluates disclosure");
    assert!(!before.score && !before.per_item_correctness);

    let after_close = store
        .resolve_effective_policy(
            context,
            ResolveEffectivePolicyCommand {
                assignment: assignment_id,
                entitlement,
                authorization: AuthorizationGate::Authorized,
                now: ActivityTimestamp::from_unix_millis(TERM_BASE_MILLIS + 2_000),
                prior_run_count: 0,
            },
        )
        .await
        .expect("resolve current S3 policy after close")
        .expect("assignment exists");
    let after = evaluate_learner_disclosure(
        updated.record.disclosure_policy,
        &after_close.decision,
        ActivityTimestamp::from_unix_millis(TERM_BASE_MILLIS + 2_000),
        Some(ActivityTimestamp::from_unix_millis(TERM_BASE_MILLIS + 10)),
    )
    .expect("S3/S5 allowed decision evaluates disclosure");
    assert!(after.score && after.per_item_correctness);
    assert!(
        !after.feedback_text,
        "feedback_release is absent from the evaluator and cannot unlock withheld feedback"
    );
    assert!(!after.solution && !after.class_statistics);
    assert!(matches!(
        after_close.decision,
        EffectivePolicyDecision::Allowed { .. }
    ));

    let mut foreign = pool.begin().await.expect("begin foreign RLS probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *foreign)
        .await
        .expect("assume application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(foreign_tenant.to_string())
        .execute(&mut *foreign)
        .await
        .expect("scope foreign tenant");
    let visible: i64 =
        sqlx::query_scalar("SELECT count(*) FROM assignment WHERE assignment_id = $1")
            .bind(assignment_id.as_uuid())
            .fetch_one(&mut *foreign)
            .await
            .expect("foreign assignment query is safely filtered");
    assert_eq!(
        visible, 0,
        "forced RLS prevents cross-tenant policy enumeration"
    );
    foreign.rollback().await.expect("rollback RLS probe");

    let feedback_release_fenced: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.feedback_release'::regclass \
             AND NOT tgisinternal AND tgname = 'feedback_release_retention_fence')",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect feedback-release retention fence");
    assert!(
        feedback_release_fenced,
        "feedback-release audit evidence remains retention-fenced"
    );
    let audit_is_not_mutable: bool = sqlx::query_scalar(
        "SELECT NOT has_table_privilege('ple_app', 'public.feedback_release', 'UPDATE')",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect feedback-release audit privilege");
    assert!(
        audit_is_not_mutable,
        "feedback-release evidence is append-only audit state"
    );
}
