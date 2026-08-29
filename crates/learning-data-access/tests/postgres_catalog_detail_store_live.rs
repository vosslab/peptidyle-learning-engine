#![cfg(feature = "postgres")]

//! Disposable PostgreSQL Store oracle for catalog-detail disclosure parity.
//!
//! The fixture creates public catalog content and course assignments through
//! production Store contracts.  It then supplies five valid first attempts to
//! the server-owned statistics capability, which is the narrow database
//! boundary that publishes anonymous discovery evidence.

use std::collections::BTreeMap;

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;
use published_assignment::create_published_assignment;

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseRecord, CourseRosterStore, CreateCourseCommand,
    DraftRecord, PublishDraftCommand, SessionLifetime, SessionStore, SessionSubject,
    SessionTokenHash, Store, TenantContext, UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentScoringMode, BackendCapabilities, Capability, CatalogDiscoveryEvidence,
    CatalogPromptProjection, CourseId, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, PointValue, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionMetadata, QuestionSource, ResponseDefinition, TenantId, UserId, UserRole, VersionId,
    WorkspaceId,
};
use sqlx::PgPool;
use uuid::Uuid;

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

fn assignment(
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    item: AssignmentItemId,
    reference: ProblemVersionRef,
    title: &str,
) -> AssignmentRecord {
    AssignmentRecord {
        id: assignment,
        tenant,
        course_id: course,
        title: title.to_string(),
        lifecycle: question_model::AssignmentLifecycle::Published,
        instructions: question_model::AssignmentInstructions::default(),
        audience: AssignmentAudience::CourseWide,
        items: vec![AssignmentItem {
            id: item,
            reference,
            position: 0,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        }],
        selection_groups: Vec::new(),
        disclosure_policy: question_model::StudentDisclosurePolicy::default(),
        policies: policies(),
    }
}

async fn approved_instructor_session(
    pool: &PgPool,
    store: &PostgresStore,
    tenant: TenantId,
    user: UserId,
    name: &str,
) -> SessionTokenHash {
    let email = format!("catalog-detail-{}@example.test", user.as_uuid().simple());
    sqlx::query(
        "INSERT INTO ple_account (user_id, normalized_email, delivery_email, display_name) \
         VALUES ($1, $2, $2, $3)",
    )
    .bind(user.as_uuid())
    .bind(&email)
    .bind(name)
    .execute(pool)
    .await
    .expect("persist approved instructor account");
    sqlx::query(
        "INSERT INTO instructor_approval (user_id, approved_by, approved_at, revision) \
         VALUES ($1, $1, transaction_timestamp(), 1)",
    )
    .bind(user.as_uuid())
    .execute(pool)
    .await
    .expect("approve live catalog-detail instructor");

    let session = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            session,
            SessionSubject::new(tenant, user, name, vec![UserRole::Instructor])
                .expect("valid approved instructor subject"),
            SessionLifetime::from_seconds(3_600).expect("positive session lifetime"),
        )
        .await
        .expect("persist approved instructor session");
    session
}

async fn create_course(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
    title: &str,
) -> CourseId {
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: title.to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("valid catalog-detail course term"),
                },
                authority: sysadmin_course_creation_authority(store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("create live catalog-detail course");
    course
}

async fn publish_question(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
    prompt_markdown: &str,
    randomization: RandomizationDefinition,
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
                markdown: prompt_markdown.to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Catalog detail evidence fixture".to_string(),
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
        .expect("save immutable catalog-detail draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "molar_mass".to_string(),
                },
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE catalog-detail fixture".to_string())
                        .expect("valid public byline"),
                ])
                .expect("valid public byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish immutable catalog-detail question");
    reference
}

fn seeded_randomization() -> RandomizationDefinition {
    RandomizationDefinition::Seeded {
        generator: GeneratorReference {
            id: "catalog-projection-fixture".to_string(),
            version: "1".to_string(),
        },
        parameters: BTreeMap::from([(
            "residue".to_string(),
            ParameterSpec::Choice {
                options: vec!["glycine".to_string()],
            },
        )]),
    }
}

struct FirstAttemptSpec {
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
    course: CourseId,
    assignment: AssignmentId,
    item: AssignmentItemId,
    reference: ProblemVersionRef,
}

async fn record_first_attempt(pool: &PgPool, store: &PostgresStore, spec: FirstAttemptSpec) {
    let FirstAttemptSpec {
        context,
        tenant,
        instructor,
        course,
        assignment,
        item,
        reference,
    } = spec;
    let learner = UserId::from_uuid(id());
    let claimed = store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "Catalog detail evidence learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create canonical catalog-detail learner membership");
    let enrollment = id();
    let run = id();
    let attempt = id();
    let mut fixture = pool.begin().await.expect("begin first-attempt fixture");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *fixture)
        .await
        .expect("set first-attempt fixture tenant context");
    sqlx::query(
        "INSERT INTO enrollment \
         (tenant_id, enrollment_id, assignment_id, student_id, user_id, course_id, \
          course_membership_id, materialized_at, materialization_purpose, \
          materialized_by_user_id, evaluator_version) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, transaction_timestamp(), \
                 'instructor_issue', $8, 1)",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment)
    .bind(assignment.as_uuid())
    .bind(claimed.member.student.as_uuid())
    .bind(claimed.member.user.as_uuid())
    .bind(course.as_uuid())
    .bind(claimed.member.id.as_uuid())
    .bind(instructor.as_uuid())
    .execute(&mut *fixture)
    .await
    .expect("insert enrolled first-attempt learner");
    sqlx::query(
        "INSERT INTO enrollment_entitlement_basis_receipt \
         (tenant_id, enrollment_id, scope_receipt_id, scope_kind, course_id, \
          course_group_id, course_group_purpose) \
         VALUES ($1, $2, $3, 'course_wide', $4, NULL, NULL)",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment)
    .bind(id())
    .bind(course.as_uuid())
    .execute(&mut *fixture)
    .await
    .expect("persist course-wide entitlement basis");
    sqlx::query(
        "UPDATE enrollment SET entitlement_receipts_sealed_at=transaction_timestamp() \
         WHERE tenant_id=$1 AND enrollment_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment)
    .execute(&mut *fixture)
    .await
    .expect("seal enrollment entitlement basis");
    sqlx::query(
        "INSERT INTO assignment_run \
         (tenant_id, run_id, enrollment_id, run_number, started_at, completed_at, payload, \
          payload_sha256) \
         VALUES ($1, $2, $3, 1, transaction_timestamp() - interval '2 minutes', \
                 transaction_timestamp(), '{\"mode\":\"assigned\"}'::jsonb, $4)",
    )
    .bind(tenant.as_uuid())
    .bind(run)
    .bind(enrollment)
    .bind("1".repeat(64))
    .execute(&mut *fixture)
    .await
    .expect("insert completed first-attempt run");
    sqlx::query(
        "INSERT INTO assignment_run_item \
         (tenant_id, run_id, assignment_item_id, source_position, issued_position, \
          problem_id, version_id, delivery_status, statistics_eligible) \
         VALUES ($1, $2, $3, 0, 0, $4, $5, 'submitted', true)",
    )
    .bind(tenant.as_uuid())
    .bind(run)
    .bind(item.as_uuid())
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .execute(&mut *fixture)
    .await
    .expect("insert eligible issued item");
    sqlx::query(
        "INSERT INTO question_attempt \
         (tenant_id, attempt_id, run_id, problem_id, version_id, occurred_at, payload, \
          payload_sha256, attempt_status, submitted_at, assignment_position, course_id, \
          presentation_capability, issued_question_snapshot_payload, \
          issued_question_snapshot_payload_sha256, authored_timing_grace_seconds) \
         VALUES ($1, $2, $3, $4, $5, transaction_timestamp() - interval '90 seconds', \
                 '{}'::jsonb, $6, 'submitted', transaction_timestamp() - interval '60 seconds', \
                 0, $7, 'not_applicable', \
                 '{\"schemaVersion\":1,\"question\":{},\"familyWitness\":{\"family\":\"native\",\"physicalAssetBindings\":[]}}'::jsonb, \
                 $8, 0)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt)
    .bind(run)
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .bind("2".repeat(64))
    .bind(course.as_uuid())
    .bind("3".repeat(64))
    .execute(&mut *fixture)
    .await
    .expect("insert submitted first attempt without answer material");
    sqlx::query(
        "INSERT INTO submission_idempotency \
         (tenant_id, attempt_id, idempotency_key, request_sha256, submitted_at, payload, \
          payload_sha256, course_id, request_contract_version) \
         VALUES ($1, $2, $3, $4, transaction_timestamp(), '{}'::jsonb, $5, $6, 1)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt)
    .bind(format!("catalog-detail-{attempt}"))
    .bind("4".repeat(64))
    .bind("5".repeat(64))
    .bind(course.as_uuid())
    .execute(&mut *fixture)
    .await
    .expect("insert accepted first-attempt receipt");
    sqlx::query(
        "INSERT INTO submission_evaluation \
         (tenant_id, attempt_id, submission_id, credit_fraction, correct, grading_status, \
          payload, payload_sha256, course_id) \
         VALUES ($1, $2, $2, 0.5, false, 'graded', '{}'::jsonb, $3, $4)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt)
    .bind("6".repeat(64))
    .bind(course.as_uuid())
    .execute(&mut *fixture)
    .await
    .expect("insert deterministic first-attempt grade");
    fixture
        .commit()
        .await
        .expect("commit first-attempt fixture");

    let mut app = pool
        .begin()
        .await
        .expect("begin statistics capability transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *app)
        .await
        .expect("assume application role for statistics");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *app)
        .await
        .expect("set statistics tenant context");
    let recorded: bool = sqlx::query_scalar(
        "SELECT ple_record_question_statistics($1, $2, $3, $4, $5, $6, \
         0.5::double precision, 1, 30, NULL, $7)",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment)
    .bind(run)
    .bind(attempt)
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .bind(vec![7_u8; 32])
    .fetch_one(&mut *app)
    .await
    .expect("record server-owned first-attempt evidence");
    assert!(
        recorded,
        "new first attempt records immutable evidence input"
    );
    app.commit()
        .await
        .expect("commit statistics capability transaction");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_detail_store_discloses_evidence_and_actor_owned_usage_only() {
    let runtime = load_acceptance_runtime();
    let pool = lazy_pool(runtime.admin_url().expose()).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0xD1; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let actor = UserId::from_uuid(id());
    let foreign_actor = UserId::from_uuid(id());
    let actor_session =
        approved_instructor_session(&pool, &store, tenant, actor, "Catalog detail actor").await;
    let foreign_session = approved_instructor_session(
        &pool,
        &store,
        tenant,
        foreign_actor,
        "Catalog detail colleague",
    )
    .await;
    let actor_course = create_course(
        &store,
        context,
        tenant,
        actor,
        "Actor-visible catalog course",
    )
    .await;
    let foreign_course = create_course(
        &store,
        context,
        tenant,
        foreign_actor,
        "Colleague-private catalog course",
    )
    .await;
    let reference = publish_question(
        &store,
        context,
        tenant,
        actor,
        "Fixed catalog prompt.",
        RandomizationDefinition::Static,
    )
    .await;
    let seeded_reference = publish_question(
        &store,
        context,
        tenant,
        actor,
        "A {{residue}} example.",
        seeded_randomization(),
    )
    .await;
    let actor_assignment = AssignmentId::from_uuid(id());
    let actor_item = AssignmentItemId::from_uuid(id());
    create_published_assignment(
        &store,
        context,
        actor,
        assignment(
            tenant,
            actor_course,
            actor_assignment,
            actor_item,
            reference,
            "Actor catalog use",
        ),
        question_model::BaseAssignmentPolicy::default(),
    )
    .await
    .expect("publish actor course catalog use");
    let foreign_assignment = AssignmentId::from_uuid(id());
    let foreign_item = AssignmentItemId::from_uuid(id());
    create_published_assignment(
        &store,
        context,
        foreign_actor,
        assignment(
            tenant,
            foreign_course,
            foreign_assignment,
            foreign_item,
            reference,
            "Colleague catalog use",
        ),
        question_model::BaseAssignmentPolicy::default(),
    )
    .await
    .expect("publish colleague course catalog use");

    for _ in 0..3 {
        record_first_attempt(
            &pool,
            &store,
            FirstAttemptSpec {
                context,
                tenant,
                instructor: actor,
                course: actor_course,
                assignment: actor_assignment,
                item: actor_item,
                reference,
            },
        )
        .await;
    }
    for _ in 0..2 {
        record_first_attempt(
            &pool,
            &store,
            FirstAttemptSpec {
                context,
                tenant,
                instructor: foreign_actor,
                course: foreign_course,
                assignment: foreign_assignment,
                item: foreign_item,
                reference,
            },
        )
        .await;
    }

    let actor_detail = store
        .get_catalog_detail(context, actor_session, reference)
        .await
        .expect("actor reads catalog detail")
        .expect("published public detail remains visible");
    assert!(matches!(
        actor_detail.evidence,
        CatalogDiscoveryEvidence::Available {
            formula_version: 1,
            observed_course_count: 2,
            independent_learner_observation_count: 5,
            difficulty_index,
            attempts_mean,
            time_median_seconds_estimate: 30,
            discrimination_index: None,
            ..
        } if difficulty_index == 0.5 && attempts_mean == 1.0
    ));
    assert_eq!(actor_detail.usage.summary.institution_course_count, 2);
    assert_eq!(actor_detail.usage.summary.institution_assignment_count, 2);
    assert_eq!(actor_detail.usage.summary.own_course_count, 1);
    assert_eq!(actor_detail.usage.summary.own_assignment_count, 1);
    assert_eq!(actor_detail.usage.own_courses.len(), 1);
    assert_eq!(
        actor_detail.usage.own_courses[0].title,
        "Actor-visible catalog course"
    );
    assert!(
        actor_detail
            .usage
            .own_courses
            .iter()
            .all(|course| course.title != "Colleague-private catalog course")
    );
    assert_eq!(
        actor_detail.prompt,
        CatalogPromptProjection::Static {
            blocks: vec![ContentBlock::Text {
                markdown: "Fixed catalog prompt.".to_string(),
            }],
        }
    );

    let foreign_detail = store
        .get_catalog_detail(context, foreign_session, reference)
        .await
        .expect("colleague reads catalog detail")
        .expect("published public detail remains visible to colleague");
    assert_eq!(foreign_detail.usage.summary.institution_course_count, 2);
    assert_eq!(foreign_detail.usage.summary.institution_assignment_count, 2);
    assert_eq!(foreign_detail.usage.summary.own_course_count, 1);
    assert_eq!(foreign_detail.usage.own_courses.len(), 1);
    assert_eq!(
        foreign_detail.usage.own_courses[0].title,
        "Colleague-private catalog course"
    );
    assert!(
        foreign_detail
            .usage
            .own_courses
            .iter()
            .all(|course| course.title != "Actor-visible catalog course")
    );

    let seeded_detail = store
        .get_catalog_detail(context, actor_session, seeded_reference)
        .await
        .expect("actor reads seeded catalog detail")
        .expect("published seeded detail remains visible");
    assert_eq!(
        seeded_detail.prompt,
        CatalogPromptProjection::GeneratedExample {
            blocks: vec![ContentBlock::Text {
                markdown: "A glycine example.".to_string(),
            }],
        }
    );
}

#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
