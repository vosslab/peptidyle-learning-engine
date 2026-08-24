#![cfg(feature = "postgres")]

//! Disposable Store-level conformance oracle for the rehearsal aggregate.
//!
//! Run this ignored target only against a fresh database named by
//! `PLE_TEST_DATABASE_URL`.  Its fixture uses ordinary Store course,
//! membership, publication, and assignment operations; direct SQL is limited
//! to migration verification, database-owned timestamps, and non-rehearsal
//! side-effect counts.

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

use domain::effective_assignment_policy::BaseAssignmentPolicy;
use learning_data_access::postgres::{PostgresStore, apply_migrations, lazy_pool};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseRecord, CourseRosterStore, CreateAssignmentCommand,
    CreateCourseCommand, DraftRecord, NavigationReferenceStore,
    PutAssignmentTeachingSettingsCommand, ReadRehearsalRouteCommand, RehearsalIdempotencyKey,
    RehearsalOperationDigest, RehearsalStore, StartRehearsalRouteCommand, Store, StoreError,
    TenantContext, UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::{
    ActivityTimestamp, AssignmentAudience, AssignmentDeliveryState, AssignmentId,
    AssignmentInstructions, AssignmentItem, AssignmentItemId, AssignmentLifecycle,
    AssignmentScoringMode, BackendCapabilities, Capability, CourseId, CourseLocalDateTime,
    CourseTerm, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, IanaTimeZone,
    LateSubmissionPolicy, PointValue, PreviewSelectedMoment, PreviewSyntheticGroupReferences,
    ProblemId, ProblemVersionRef, PublicationScope, QuestionMetadata, QuestionSource,
    RehearsalLifecycle, RehearsalSubjectStart, RehearsalSyntheticSubjectRequest,
    ResponseDefinition, SyntheticPreviewModifiers, TeachingAttemptLimitFieldPatch,
    TeachingLimitFieldPatch, TeachingOperationRevision, TeachingTimeFieldPatch, TenantId, UserId,
    VersionId, WorkspaceId,
};
use sqlx::PgPool;
use std::num::NonZeroU32;
use uuid::Uuid;

pub fn id() -> Uuid {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).expect("fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

/// Ordinary, committed source and frozen rehearsal used by post-start SQL
/// oracles.  The fixture deliberately crosses the same Store boundary as the
/// instructor route: it never invokes a rehearsal SQL start capability.
#[allow(dead_code)] // Raw SQL fault oracles consume these only after Store start commits.
pub struct StartedFixture {
    pub pool: PgPool,
    pub store: PostgresStore,
    pub context: TenantContext,
    pub tenant: TenantId,
    pub actor: UserId,
    pub course: CourseId,
    pub assignment_id: AssignmentId,
    pub assignment: question_model::AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub rehearsal: question_model::RehearsalReference,
    /// Test-private persistence identity. It is obtained only after the Store
    /// has committed the public route start.
    pub run_id: Uuid,
}

/// Creates a normal four-item published assignment and starts its rehearsal
/// through [`RehearsalStore::start_rehearsal_from_route`].
pub async fn started_fixture() -> StartedFixture {
    started_fixture_with_timing(TimingPolicy::Untimed, Some(300)).await
}

/// Creates the same ordinary Store-owned fixture with an authored question
/// timing policy and an optional assignment-wide subject limit.  Connected
/// timing oracles use this rather than inserting rehearsal rows directly.
pub async fn started_fixture_with_timing(
    timing_policy: TimingPolicy,
    subject_time_limit_seconds: Option<u32>,
) -> StartedFixture {
    let url = std::env::var("PLE_TEST_DATABASE_URL").expect("disposable database URL");
    let pool = lazy_pool(&url).expect("PostgreSQL URL");
    apply_migrations(&pool).await.expect("full migration epoch");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x54; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let actor = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "PostgreSQL rehearsal post-start fixture".into(),
                    term: CourseTerm::from_parts("2026-01-01", "2026-12-31", "America/Chicago")
                        .expect("term"),
                },
                authority: sysadmin_course_creation_authority(&store, tenant, course, actor).await,
            },
        )
        .await
        .expect("course");
    store
        .upsert_course_member(
            context,
            actor,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "Ordinary learner".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("ordinary learner");
    let publications = [
        publish_with_timing(&store, context, tenant, actor, timing_policy).await,
        publish_with_timing(&store, context, tenant, actor, timing_policy).await,
        publish_with_timing(&store, context, tenant, actor, timing_policy).await,
        publish_with_timing(&store, context, tenant, actor, timing_policy).await,
    ];
    let assignment_id = AssignmentId::from_uuid(id());
    let policy = BaseAssignmentPolicy {
        available_at: Some(ActivityTimestamp::from_unix_millis(1_787_580_000_000)),
        due_at: None,
        closes_at: None,
        time_limit_seconds: subject_time_limit_seconds
            .map(|seconds| NonZeroU32::new(seconds).expect("positive subject limit")),
        attempt_limit: Some(NonZeroU32::new(2).expect("limit")),
        late_submission: LateSubmissionPolicy::Accept,
        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
    };
    let instructions =
        AssignmentInstructions::try_new("Work through the problem.".into()).expect("instructions");
    let created = store
        .create_assignment(
            context,
            CreateAssignmentCommand {
                actor,
                base_policy: policy,
                assignment: AssignmentRecord {
                    id: assignment_id,
                    tenant,
                    course_id: course,
                    title: "Rehearsal post-start assignment".into(),
                    lifecycle: AssignmentLifecycle::Draft,
                    instructions: instructions.clone(),
                    audience: AssignmentAudience::CourseWide,
                    items: publications
                        .into_iter()
                        .enumerate()
                        .map(|(position, reference)| AssignmentItem {
                            id: AssignmentItemId::from_uuid(id()),
                            reference,
                            position: u32::try_from(position).expect("fixture position"),
                            points_possible: PointValue::from_whole(1),
                            delivery_state: AssignmentDeliveryState::Active,
                            scoring_mode: AssignmentScoringMode::Normal,
                        })
                        .collect(),
                    selection_groups: vec![],
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
            },
        )
        .await
        .expect("draft assignment");
    let published = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor,
                course,
                assignment: assignment_id,
                expected_revision: created.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: AssignmentLifecycle::Published,
                    instructions,
                    base_policy: policy,
                },
            },
        )
        .await
        .expect("published assignment");
    let assignment = store
        .assignment_reference(context, actor, assignment_id)
        .await
        .expect("reference query")
        .expect("published reference");
    let revision = TeachingOperationRevision::new(published.revision.value()).expect("revision");
    let started = store
        .start_rehearsal_from_route(
            context,
            StartRehearsalRouteCommand {
                actor,
                course,
                assignment,
                expected_revision: revision,
                subject: synthetic_subject("2026-08-25T09:00:00.000"),
                start_new_after_completion: false,
                idempotency_key: RehearsalIdempotencyKey::new("postgres-post-start".into())
                    .expect("idempotency key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([0x61; 32]),
            },
        )
        .await
        .expect("Store-owned route rehearsal start")
        .receipt;
    let run_id: Uuid = sqlx::query_scalar(
        "SELECT rehearsal_run_id FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_reference=$2",
    )
    .bind(tenant.as_uuid())
    .bind(i64::from(started.rehearsal.number()))
    .fetch_one(&pool)
    .await
    .expect("committed rehearsal internal identity");
    StartedFixture {
        pool,
        store,
        context,
        tenant,
        actor,
        course,
        assignment_id,
        assignment,
        revision,
        rehearsal: started.rehearsal,
        run_id,
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

fn synthetic_subject(moment: &str) -> RehearsalSubjectStart {
    RehearsalSubjectStart::Synthetic {
        request: RehearsalSyntheticSubjectRequest {
            selected_moment: PreviewSelectedMoment {
                value: CourseLocalDateTime::parse(moment).expect("moment"),
                time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
            },
            groups: PreviewSyntheticGroupReferences::try_from(Vec::new()).expect("groups"),
            modifiers: SyntheticPreviewModifiers {
                mode: question_model::PolicyModificationModeView::ExtendOnly,
                patch: question_model::PolicyPatchView {
                    available_at: TeachingTimeFieldPatch::Inherit,
                    due_at: TeachingTimeFieldPatch::Inherit,
                    closes_at: TeachingTimeFieldPatch::Inherit,
                    time_limit_seconds: TeachingLimitFieldPatch::Inherit,
                    attempt_limit: TeachingAttemptLimitFieldPatch::Inherit,
                },
            },
        },
    }
}

async fn publish(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
) -> ProblemVersionRef {
    publish_with_timing(store, context, tenant, instructor, TimingPolicy::Untimed).await
}

async fn publish_with_timing(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
    timing_policy: TimingPolicy,
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
                family: "rehearsal_store_live".into(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Store rehearsal fixture".into(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Exact,
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Store rehearsal fixture".into(),
                tags: vec![],
                taxonomy: vec![],
                license: question_model::taxonomy::License::CcBy,
                language: "en-US".into(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("draft");
    store
        .publish_draft(
            context,
            instructor,
            learning_data_access::PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "rehearsal_store_live".into(),
                },
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".into()).expect("byline"),
                ])
                .expect("byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("published question");
    reference
}

async fn ordinary_counts(pool: &PgPool, tenant: TenantId) -> [i64; 7] {
    async fn count(pool: &PgPool, tenant: TenantId, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql)
            .bind(tenant.as_uuid())
            .fetch_one(pool)
            .await
            .expect("ordinary effect count")
    }
    [
        count(
            pool,
            tenant,
            "SELECT count(*) FROM enrollment WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM assignment_run WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM question_attempt WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM submission WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM course_grade_scheme WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM course_item_analysis_current WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM worker_job WHERE tenant_id=$1",
        )
        .await,
    ]
}

async fn assert_application_cannot_update_rehearsal_rows(pool: &PgPool) {
    let can_update: bool = sqlx::query_scalar(
        "SELECT has_table_privilege('ple_app', 'public.rehearsal_run', 'UPDATE')",
    )
    .fetch_one(pool)
    .await
    .expect("application rehearsal-row privilege inventory");
    assert!(
        !can_update,
        "the Store verifies broker-prelocked rehearsals without application UPDATE authority"
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL for a disposable migrated PostgreSQL database"]
async fn postgres_rehearsal_store_live_conformance() {
    let url = std::env::var("PLE_TEST_DATABASE_URL").expect("disposable database URL");
    let pool = lazy_pool(&url).expect("PostgreSQL URL");
    apply_migrations(&pool).await.expect("full migration epoch");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x54; 32]);
    assert_application_cannot_update_rehearsal_rows(&pool).await;
    let tenant = TenantId::from_uuid(id());
    let foreign = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id());
    let outsider = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "PostgreSQL rehearsal Store".into(),
                    term: CourseTerm::from_parts("2026-01-01", "2026-12-31", "America/Chicago")
                        .expect("term"),
                },
                authority: sysadmin_course_creation_authority(&store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("course");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "Ordinary learner".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("ordinary learner");
    let publication = publish(&store, context, tenant, instructor).await;
    let second_publication = publish(&store, context, tenant, instructor).await;
    let assignment_id = AssignmentId::from_uuid(id());
    let policy = BaseAssignmentPolicy {
        available_at: Some(ActivityTimestamp::from_unix_millis(1_787_580_000_000)),
        due_at: None,
        closes_at: None,
        time_limit_seconds: Some(NonZeroU32::new(300).expect("limit")),
        attempt_limit: Some(NonZeroU32::new(2).expect("limit")),
        late_submission: LateSubmissionPolicy::Accept,
        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
    };
    let instructions =
        AssignmentInstructions::try_new("Work through the problem.".into()).expect("instructions");
    let created = store
        .create_assignment(
            context,
            CreateAssignmentCommand {
                actor: instructor,
                base_policy: policy,
                assignment: AssignmentRecord {
                    id: assignment_id,
                    tenant,
                    course_id: course,
                    title: "Rehearsal Store assignment".into(),
                    lifecycle: AssignmentLifecycle::Draft,
                    instructions: instructions.clone(),
                    audience: AssignmentAudience::CourseWide,
                    items: vec![
                        AssignmentItem {
                            id: AssignmentItemId::from_uuid(id()),
                            reference: publication,
                            position: 0,
                            points_possible: PointValue::from_whole(1),
                            delivery_state: AssignmentDeliveryState::Active,
                            scoring_mode: AssignmentScoringMode::Normal,
                        },
                        AssignmentItem {
                            id: AssignmentItemId::from_uuid(id()),
                            reference: second_publication,
                            position: 1,
                            points_possible: PointValue::from_whole(1),
                            delivery_state: AssignmentDeliveryState::Active,
                            scoring_mode: AssignmentScoringMode::Normal,
                        },
                    ],
                    selection_groups: vec![],
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
            },
        )
        .await
        .expect("draft assignment");
    let published = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment: assignment_id,
                expected_revision: created.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: AssignmentLifecycle::Published,
                    instructions,
                    base_policy: policy,
                },
            },
        )
        .await
        .expect("published assignment");
    let assignment = store
        .assignment_reference(context, instructor, assignment_id)
        .await
        .expect("reference query")
        .expect("published reference");
    let revision = TeachingOperationRevision::new(published.revision.value()).expect("revision");
    let before = ordinary_counts(&pool, tenant).await;
    let route_start = StartRehearsalRouteCommand {
        actor: instructor,
        course,
        assignment,
        expected_revision: revision,
        subject: synthetic_subject("2026-08-25T09:00:00.000"),
        start_new_after_completion: false,
        idempotency_key: RehearsalIdempotencyKey::new("postgres-route-start".into())
            .expect("idempotency key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x61; 32]),
    };
    let first_result = store
        .start_rehearsal_from_route(context, route_start.clone())
        .await
        .expect("create active rehearsal");
    assert!(!first_result.replayed);
    let first = first_result.receipt;
    assert_eq!(first.lifecycle, RehearsalLifecycle::Active);
    let replay = store
        .start_rehearsal_from_route(context, route_start.clone())
        .await
        .expect("route replay");
    assert!(replay.replayed);
    assert_eq!(
        replay.receipt, first,
        "idempotency replays the durable receipt"
    );
    assert_eq!(
        store
            .read_rehearsal_from_route(
                context,
                ReadRehearsalRouteCommand {
                    actor: instructor,
                    course,
                    assignment,
                    rehearsal: first.rehearsal,
                },
            )
            .await
            .expect("rehydrate"),
        first,
        "Store rehydrates the exact durable receipt"
    );
    assert_eq!(
        store
            .read_rehearsal_from_route(
                TenantContext::from_authenticated_session(foreign),
                ReadRehearsalRouteCommand {
                    actor: instructor,
                    course,
                    assignment,
                    rehearsal: first.rehearsal,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "foreign tenant cannot distinguish rehearsal existence"
    );
    assert_eq!(
        store
            .read_rehearsal_from_route(
                context,
                ReadRehearsalRouteCommand {
                    actor: outsider,
                    course,
                    assignment,
                    rehearsal: first.rehearsal,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "foreign actor cannot inspect the aggregate"
    );
    let material_rows: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT \
            (SELECT count(*) FROM rehearsal_frozen_material_set WHERE tenant_id=$1), \
            (SELECT count(*) FROM rehearsal_frozen_source_snapshot WHERE tenant_id=$1), \
            (SELECT count(*) FROM rehearsal_frozen_private_execution WHERE tenant_id=$1), \
            (SELECT count(*) FROM rehearsal_start_freeze_source_binding WHERE tenant_id=$1)",
    )
    .bind(tenant.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("complete frozen material inventory");
    assert_eq!(
        material_rows,
        (1, 2, 2, 2),
        "the ordinary two-item assignment becomes one complete immutable material set"
    );
    let material_header: (i32, i32, i64) = sqlx::query_as(
        "SELECT expected_item_count, frozen_item_count, assignment_revision
           FROM rehearsal_frozen_material_set
          WHERE tenant_id=$1
            AND rehearsal_run_id=(
                SELECT rehearsal_run_id
                  FROM rehearsal_run
                 WHERE tenant_id=$1 AND rehearsal_reference=$2
            )",
    )
    .bind(tenant.as_uuid())
    .bind(i64::from(first.rehearsal.number()))
    .fetch_one(&pool)
    .await
    .expect("exact frozen material header");
    assert_eq!(
        material_header,
        (
            2,
            2,
            i64::try_from(revision.value()).expect("database revision")
        ),
        "the immutable header commits the exact ordinary assignment inventory and revision"
    );
    assert_eq!(
        ordinary_counts(&pool, tenant).await,
        before,
        "route rehearsal start has no learner-work side effects"
    );
}
