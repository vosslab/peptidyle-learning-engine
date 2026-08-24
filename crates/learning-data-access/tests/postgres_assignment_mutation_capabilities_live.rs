#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for the actor-authorized assignment mutators.
//!
//! The canonical Store prepares assignment mutations through the broker.
//! This migration-level suite proves that the prepared authority may complete each authorized mutation,
//! while stale, foreign, malformed, and direct-DML requests cannot.

use learning_data_access::postgres::{
    PostgresStore, apply_migrations, lazy_pool, migration_status,
};
use learning_data_access::{AssignmentRecord, CreateAssignmentCommand, Store, TenantContext};
use question_model::run_policy::{
    CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies, VariationPolicy,
};
use question_model::{
    AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentInstructions,
    AssignmentItem, AssignmentItemId, AssignmentLifecycle, AssignmentScoringMode, CourseId,
    PointValue, ProblemId, ProblemVersionRef, TenantId, UserId, VersionId,
};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

async fn pool() -> PgPool {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL names a disposable PostgreSQL 17 database");
    let pool = lazy_pool(&url).expect("valid disposable PostgreSQL URL");
    apply_migrations(&pool)
        .await
        .expect("full migration epoch applies");
    apply_migrations(&pool)
        .await
        .expect("migration epoch converges");
    let status = migration_status(&pool)
        .await
        .expect("migration status is readable");
    assert!(status.is_compatible(), "migration epoch is complete");
    assert!(
        status
            .entries()
            .iter()
            .any(|entry| entry.version() == 2026081814),
        "the complete authority migration is present"
    );
    pool
}

struct Source {
    tenant: Uuid,
    course: Uuid,
    actor: Uuid,
    backup_instructor: Uuid,
    problem: Uuid,
    version: Uuid,
    group: Uuid,
    accommodation_group: Uuid,
    student: Uuid,
}

async fn source(pool: &PgPool) -> Source {
    let source = Source {
        tenant: id(),
        course: id(),
        actor: id(),
        backup_instructor: id(),
        problem: id(),
        version: id(),
        group: id(),
        accommodation_group: id(),
        student: id(),
    };
    let mut tx = pool.begin().await.expect("begin owner fixture");
    let question_id = id().simple().to_string()[..7].to_ascii_uppercase();
    sqlx::query("SELECT set_config('ple.tenant_id', $1, false)")
        .bind(source.tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("fixture tenant");
    sqlx::query("INSERT INTO course (tenant_id,course_id,title,term_start_date,term_end_date,time_zone) VALUES ($1,$2,'Assignment authority oracle',DATE '2026-08-24',DATE '2026-12-18','America/Chicago')")
        .bind(source.tenant).bind(source.course).execute(&mut *tx).await.expect("course");
    sqlx::query(
        "INSERT INTO tenant_learner_identity (tenant_id,user_id,student_id) VALUES ($1,$2,$3)",
    )
    .bind(source.tenant)
    .bind(source.student)
    .bind(source.student)
    .execute(&mut *tx)
    .await
    .expect("tenant learner identity");
    for (user, membership, role) in [
        (source.actor, id(), "instructor"),
        (source.backup_instructor, id(), "instructor"),
        (source.student, id(), "student"),
    ] {
        sqlx::query("INSERT INTO course_member (tenant_id,course_id,user_id,role,course_membership_id,student_id,status,joined_at) VALUES ($1,$2,$3,$4,$5,$6,'active',transaction_timestamp())")
            .bind(source.tenant).bind(source.course).bind(user).bind(role).bind(membership)
            .bind((role == "student").then_some(user)).execute(&mut *tx).await.expect("membership");
    }
    sqlx::query("INSERT INTO course_group (tenant_id,course_id,course_group_id,title,purpose) VALUES ($1,$2,$3,'Section A','section')")
        .bind(source.tenant).bind(source.course).bind(source.group).execute(&mut *tx).await.expect("group");
    sqlx::query("INSERT INTO course_group (tenant_id,course_id,course_group_id,title,purpose) VALUES ($1,$2,$3,'Extended-time accommodation','accommodation')")
        .bind(source.tenant).bind(source.course).bind(source.accommodation_group).execute(&mut *tx).await.expect("accommodation group");
    sqlx::query("INSERT INTO problem (problem_id,owner_tenant_id,owner_user_id,visibility,license,lifecycle,question_id) VALUES ($1,$2,$3,'institution','CC-BY','published',$4)")
        .bind(source.problem).bind(source.tenant).bind(source.actor).bind(question_id).execute(&mut *tx).await.expect("problem");
    sqlx::query("INSERT INTO problem_version (problem_id,version_id,content_sha256,workspace_id,title,lifecycle,backend,publication_scope,author_ids,public_byline) VALUES ($1,$2,$3,$4,'Authority problem','published','native','institution',jsonb_build_array($5::text),ARRAY['Oracle author'])")
        .bind(source.problem).bind(source.version).bind("a".repeat(64)).bind(id()).bind(source.actor).execute(&mut *tx).await.expect("published version");
    sqlx::query(
        "INSERT INTO catalog_tenant_grant (tenant_id,problem_id,version_id) VALUES ($1,$2,$3)",
    )
    .bind(source.tenant)
    .bind(source.problem)
    .bind(source.version)
    .execute(&mut *tx)
    .await
    .expect("catalog grant");
    tx.commit().await.expect("commit fixture");
    source
}

async fn app<'a>(pool: &'a PgPool, tenant: Uuid) -> Transaction<'a, Postgres> {
    let mut tx = pool.begin().await.expect("begin application transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("application role");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("application tenant");
    tx
}

fn payload(
    source: &Source,
    assignment: Uuid,
    item: Uuid,
    candidate: Uuid,
    selection: Uuid,
    title: &str,
    points: &str,
) -> Value {
    json!({
        "schemaVersion": 1, "title": title, "lifecycle": "published", "instructions": "Use the displayed model.",
        "policies": {"completion":{"kind":"answerAll"},"grade":"highest","continuedPractice":{"kind":"unlimited"},"variation":"newSeeds"},
        "disclosurePolicy": {"score":"afterSubmit","perItemCorrectness":"afterSubmit","feedbackText":"afterSubmit","solution":"afterClose","classStatistics":"never"},
        "audience": {"kind":"anyOfGroups","groups":[source.group]},
        "basePolicy": {"availableAt":1787590800000_i64,"dueAt":1787677200000_i64,"closesAt":1787763600000_i64,"lateSubmission":"markLate","deadlineBehavior":"autoSubmit","timeLimitSeconds":3600,"attemptLimit":2},
        "entries": [
            {"kind":"fixed","id":item,"position":0,"problemId":source.problem,"versionId":source.version,"pointsPossible":points,"deliveryState":"active","scoringMode":"normal"},
            {"kind":"selectionGroup","id":selection,"position":1,"drawCount":1,"pointsPerItem":"2.0","ordering":"candidateOrder","algorithmVersion":1,"candidates":[{"id":candidate,"position":0,"problemId":source.problem,"versionId":source.version,"deliveryState":"active"}]}
        ],
        "assignmentIdForFixtureOnly": assignment
    })
}

fn valid_payload(
    source: &Source,
    item: Uuid,
    candidate: Uuid,
    selection: Uuid,
    title: &str,
    points: &str,
) -> Value {
    let mut value = payload(source, id(), item, candidate, selection, title, points);
    value
        .as_object_mut()
        .expect("object")
        .remove("assignmentIdForFixtureOnly");
    value
}

async fn revision(tx: &mut Transaction<'_, Postgres>, source: &Source, assignment: Uuid) -> i64 {
    sqlx::query_scalar("SELECT revision FROM assignment WHERE tenant_id=$1 AND assignment_id=$2")
        .bind(source.tenant)
        .bind(assignment)
        .fetch_one(&mut **tx)
        .await
        .expect("assignment revision")
}

async fn prepared(
    tx: &mut Transaction<'_, Postgres>,
    source: &Source,
    assignment: Uuid,
    expected: i64,
) {
    let found: i64 =
        sqlx::query_scalar("SELECT public.ple_prepare_assignment_mutation($1,$2,$3,$4,$5)")
            .bind(source.tenant)
            .bind(source.actor)
            .bind(source.course)
            .bind(assignment)
            .bind(expected)
            .fetch_one(&mut **tx)
            .await
            .expect("prepare assignment mutation");
    assert_eq!(
        found, expected,
        "verified zero-run witness preserves revision"
    );
}

async fn prepared_creation(tx: &mut Transaction<'_, Postgres>, source: &Source, assignment: Uuid) {
    let returned: (Uuid, Uuid, Uuid, Uuid, String, String, String) = sqlx::query_as(
        "SELECT tenant_id,actor_id,course_id,assignment_id, \
                term_start_date::text,term_end_date::text,time_zone \
           FROM public.ple_prepare_assignment_creation_v1($1,$2,$3,$4)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(assignment)
    .fetch_one(&mut **tx)
    .await
    .expect("prepare assignment creation");
    assert_eq!(
        returned,
        (
            source.tenant,
            source.actor,
            source.course,
            assignment,
            "2026-08-24".to_string(),
            "2026-12-18".to_string(),
            "America/Chicago".to_string(),
        ),
        "prepare returns only exact bindings and the locked course term"
    );
}

struct CreationPrepareRequest {
    context_tenant: Uuid,
    tenant: Uuid,
    actor: Uuid,
    course: Uuid,
    assignment: Uuid,
}

async fn refused_creation_prepare(
    pool: &PgPool,
    request: CreationPrepareRequest,
    expected_refusal: &str,
) {
    let mut tx = app(pool, request.context_tenant).await;
    let denied =
        sqlx::query("SELECT * FROM public.ple_prepare_assignment_creation_v1($1,$2,$3,$4)")
            .bind(request.tenant)
            .bind(request.actor)
            .bind(request.course)
            .bind(request.assignment)
            .fetch_one(&mut *tx)
            .await;
    assert!(denied.is_err(), "{expected_refusal}");
    tx.rollback()
        .await
        .expect("rollback refused creation prepare");
}

async fn active_attempt_witness(
    tx: &mut Transaction<'_, Postgres>,
    source: &Source,
    assignment: Uuid,
    revision: i64,
) -> (String, i64, i64, Vec<Uuid>) {
    sqlx::query_as(
        "SELECT * FROM public.ple_prepare_assignment_active_attempt_reresolution($1,$2,$3,$4,$5)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(assignment)
    .bind(revision)
    .fetch_one(&mut **tx)
    .await
    .expect("prepare active-attempt re-resolution")
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and a disposable PostgreSQL 17 database"]
async fn definition_create_replace_and_focused_capabilities_are_authorized_atomic_and_revisioned() {
    let pool = pool().await;
    let source = source(&pool).await;
    let assignment = id();
    let item = id();
    let group = id();
    let candidate = id();
    let mut tx = app(&pool, source.tenant).await;
    prepared_creation(&mut tx, &source, assignment).await;
    let initial = valid_payload(&source, item, candidate, group, "Mechanism mutation", "4.0");
    let created: (Uuid, i64, i64, String) = sqlx::query_as(
        "SELECT * FROM public.ple_create_assignment_definition_v1($1,$2,$3,$4,$5,$6)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(assignment)
    .bind(initial)
    .bind(Option::<Uuid>::None)
    .bind(Option::<i32>::None)
    .fetch_one(&mut *tx)
    .await
    .expect("authorized complete creation");
    assert_eq!(created.0, assignment);
    assert_eq!(created.1, 1);
    assert_eq!(created.2, 1);
    assert_eq!(created.3, "current");
    let scheme_before: i64 = sqlx::query_scalar(
        "SELECT revision FROM course_grade_scheme WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(source.tenant)
    .bind(source.course)
    .fetch_one(&mut *tx)
    .await
    .expect("course scheme created with course");
    assert_eq!(
        scheme_before, 2,
        "creation revises the course grade scheme exactly once"
    );
    tx.commit()
        .await
        .expect("commit complete definition creation");
    let mut tx = app(&pool, source.tenant).await;
    prepared(&mut tx, &source, assignment, 1).await;
    let replaced: (i64, i64, String) = sqlx::query_as(
        "SELECT * FROM public.ple_replace_assignment_definition_v1($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(assignment)
    .bind(1_i64)
    .bind(valid_payload(
        &source,
        item,
        candidate,
        group,
        "Mechanism mutation revised",
        "5.0",
    ))
    .bind(Option::<Uuid>::None)
    .bind(Option::<i32>::None)
    .fetch_one(&mut *tx)
    .await
    .expect("identity-preserving complete replacement");
    assert_eq!(
        replaced,
        (2, 2, "current".to_owned()),
        "score-affecting replacement increments generation without scores"
    );
    let empty = active_attempt_witness(&mut tx, &source, assignment, 2).await;
    assert_eq!(empty.0, "published");
    assert_eq!(empty.1, 2);
    assert_eq!(empty.2, 0, "zero active attempts are witnessed exactly");
    assert!(
        empty.3.is_empty(),
        "zero active attempts have no opaque identifiers"
    );
    let preserved: (Uuid, Uuid, Uuid) = sqlx::query_as("SELECT assignment_item_id,(SELECT selection_group_id FROM assignment_selection_group WHERE tenant_id=$1 AND assignment_id=$2), (SELECT candidate_id FROM assignment_selection_candidate WHERE tenant_id=$1 AND selection_group_id=$3) FROM assignment_item WHERE tenant_id=$1 AND assignment_id=$2")
        .bind(source.tenant).bind(assignment).bind(group).fetch_one(&mut *tx).await.expect("server identities");
    assert_eq!(preserved, (item, group, candidate));

    let mut expected = 2;
    let offset = 3600_i32;
    let settings = json!({"lifecycle":"published","instructions":"Updated instructions","basePolicy":{"availableAt":1787590800000_i64,"dueAt":1787677200000_i64,"closesAt":1787763600000_i64,"lateSubmission":"markLate","deadlineBehavior":"autoSubmit","timeLimitSeconds":3600,"attemptLimit":2}});
    let accommodation = json!({"overrideKind":"explicit_override","dueMode":"unrestricted","timeLimitMode":"unlimited","attemptLimitMode":"unlimited"});
    let exception = id();
    macro_rules! mutate { ($sql:literal $(, $value:expr )* $(,)?) => {{ prepared(&mut tx, &source, assignment, expected).await; let next: i64 = sqlx::query_scalar($sql).bind(source.tenant).bind(source.actor).bind(source.course).bind(assignment).bind(expected) $(.bind($value))* .fetch_one(&mut *tx).await.expect("authorized focused mutator"); expected += 1; assert_eq!(next, expected); }}; }
    mutate!(
        "SELECT public.ple_put_assignment_teaching_settings($1,$2,$3,$4,$5,$6)",
        settings
    );
    mutate!(
        "SELECT public.ple_put_assignment_group_schedule_offset($1,$2,$3,$4,$5,$6,$7)",
        source.group,
        offset
    );
    mutate!(
        "SELECT public.ple_delete_assignment_group_schedule_offset($1,$2,$3,$4,$5,$6)",
        source.group
    );
    mutate!(
        "SELECT public.ple_put_assignment_group_accommodation($1,$2,$3,$4,$5,$6,$7)",
        source.accommodation_group,
        accommodation.clone()
    );
    mutate!(
        "SELECT public.ple_delete_assignment_group_accommodation($1,$2,$3,$4,$5,$6)",
        source.accommodation_group
    );
    mutate!(
        "SELECT public.ple_put_assignment_individual_exception($1,$2,$3,$4,$5,$6,$7,$8)",
        exception,
        source.student,
        accommodation
    );
    mutate!(
        "SELECT public.ple_delete_assignment_individual_exception($1,$2,$3,$4,$5,$6)",
        source.student
    );
    let inserted_item = id();
    mutate!(
        "SELECT public.ple_replace_assignment_fixed_item($1,$2,$3,$4,$5,$6,$7,$8)",
        item,
        source.problem,
        source.version
    );
    mutate!(
        "SELECT public.ple_add_assignment_fixed_item($1,$2,$3,$4,$5,$6,$7,$8,$9,$10::numeric,$11,$12)",
        inserted_item,
        2_i32,
        source.problem,
        source.version,
        3.0_f64,
        "active",
        "normal"
    );
    mutate!(
        "SELECT public.ple_remove_assignment_fixed_item($1,$2,$3,$4,$5,$6)",
        inserted_item
    );
    assert_eq!(
        revision(&mut tx, &source, assignment).await,
        expected,
        "every capability has one compare-and-swap revision"
    );
    tx.commit().await.expect("commit authorized mutations");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and a disposable PostgreSQL 17 database"]
async fn assignment_capabilities_refuse_stale_foreign_malformed_and_direct_dml_without_partial_state()
 {
    let pool = pool().await;
    let source = source(&pool).await;
    let assignment = id();
    let item = id();
    let group = id();
    let candidate = id();
    let mut owner = app(&pool, source.tenant).await;
    sqlx::query("SELECT * FROM public.ple_create_assignment_definition_v1($1,$2,$3,$4,$5,$6)")
        .bind(source.tenant)
        .bind(source.actor)
        .bind(source.course)
        .bind(assignment)
        .bind(valid_payload(
            &source, item, candidate, group, "Atomic", "4.0",
        ))
        .bind(Option::<Uuid>::None)
        .bind(Option::<i32>::None)
        .execute(&mut *owner)
        .await
        .expect("create mutation target");
    owner.commit().await.expect("commit target");
    refused_creation_prepare(
        &pool,
        CreationPrepareRequest {
            context_tenant: source.tenant,
            tenant: source.tenant,
            actor: id(),
            course: source.course,
            assignment: id(),
        },
        "nonmember cannot prepare assignment creation",
    )
    .await;
    refused_creation_prepare(
        &pool,
        CreationPrepareRequest {
            context_tenant: source.tenant,
            tenant: source.tenant,
            actor: source.student,
            course: source.course,
            assignment: id(),
        },
        "Student cannot prepare assignment creation",
    )
    .await;
    refused_creation_prepare(
        &pool,
        CreationPrepareRequest {
            context_tenant: source.tenant,
            tenant: id(),
            actor: source.actor,
            course: source.course,
            assignment: id(),
        },
        "foreign tenant binding cannot prepare assignment creation",
    )
    .await;
    refused_creation_prepare(
        &pool,
        CreationPrepareRequest {
            context_tenant: source.tenant,
            tenant: source.tenant,
            actor: source.actor,
            course: source.course,
            assignment,
        },
        "duplicate assignment identity cannot be prepared",
    )
    .await;
    let mut stale = app(&pool, source.tenant).await;
    let denied = sqlx::query_scalar::<_, i64>(
        "SELECT public.ple_put_assignment_group_schedule_offset($1,$2,$3,$4,$5,$6,60)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(assignment)
    .bind(99_i64)
    .bind(source.group)
    .fetch_one(&mut *stale)
    .await;
    assert!(denied.is_err(), "stale compare-and-swap is refused");
    stale.rollback().await.expect("rollback stale");
    let mut foreign = app(&pool, source.tenant).await;
    let denied = sqlx::query_scalar::<_, i64>(
        "SELECT public.ple_put_assignment_group_schedule_offset($1,$2,$3,$4,$5,$6,60)",
    )
    .bind(source.tenant)
    .bind(id())
    .bind(source.course)
    .bind(assignment)
    .bind(1_i64)
    .bind(source.group)
    .fetch_one(&mut *foreign)
    .await;
    assert!(
        denied.is_err(),
        "an actor without direct instructor membership is refused"
    );
    foreign.rollback().await.expect("rollback foreign test");
    let mut lock_attacker = app(&pool, source.tenant).await;
    let denied = sqlx::query(
        "SELECT assignment_id FROM assignment WHERE tenant_id=$1 AND assignment_id=$2 FOR UPDATE",
    )
    .bind(source.tenant)
    .bind(assignment)
    .fetch_all(&mut *lock_attacker)
    .await;
    assert!(
        denied.is_err(),
        "ple_app cannot lock assignment rows directly for update"
    );
    lock_attacker
        .rollback()
        .await
        .expect("rollback denied update lock");
    let mut course_lock_attacker = app(&pool, source.tenant).await;
    let denied = sqlx::query(
        "SELECT course_id FROM course WHERE tenant_id=$1 AND course_id=$2 FOR KEY SHARE",
    )
    .bind(source.tenant)
    .bind(source.course)
    .fetch_all(&mut *course_lock_attacker)
    .await;
    assert!(
        denied.is_err(),
        "ple_app cannot regain direct locked course reads"
    );
    course_lock_attacker
        .rollback()
        .await
        .expect("rollback denied course lock");
    let mut private_helper_attacker = app(&pool, source.tenant).await;
    let denied =
        sqlx::query("SELECT public.ple_assignment_mutator_require_create_editor($1,$2,$3,$4)")
            .bind(source.tenant)
            .bind(source.actor)
            .bind(source.course)
            .bind(id())
            .execute(&mut *private_helper_attacker)
            .await;
    assert!(
        denied.is_err(),
        "ple_app cannot execute the private creation authorization helper"
    );
    private_helper_attacker
        .rollback()
        .await
        .expect("rollback denied private helper");
    let mut share_attacker = app(&pool, source.tenant).await;
    let denied = sqlx::query(
        "SELECT assignment_id FROM assignment WHERE tenant_id=$1 AND assignment_id=$2 FOR SHARE",
    )
    .bind(source.tenant)
    .bind(assignment)
    .fetch_all(&mut *share_attacker)
    .await;
    assert!(
        denied.is_err(),
        "ple_app cannot lock assignment rows directly for share"
    );
    share_attacker
        .rollback()
        .await
        .expect("rollback denied share lock");
    let mut direct = app(&pool, source.tenant).await;
    let denied =
        sqlx::query("UPDATE assignment SET title='bypass' WHERE tenant_id=$1 AND assignment_id=$2")
            .bind(source.tenant)
            .bind(assignment)
            .execute(&mut *direct)
            .await;
    assert!(denied.is_err(), "ple_app has no direct assignment DML");
    direct.rollback().await.expect("rollback direct DML");
    let old_signature: Option<String> = sqlx::query_scalar("SELECT to_regprocedure('public.ple_replace_assignment_fixed_item(uuid,uuid,uuid,bigint,uuid,uuid,uuid)')::text")
        .fetch_one(&pool).await.expect("catalog signature probe");
    assert!(
        old_signature.is_none(),
        "actorless capability signature has been retired"
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and a disposable PostgreSQL 17 database"]
async fn creation_prepare_catalog_and_membership_lock_preserve_least_authority() {
    let pool = pool().await;
    let source = source(&pool).await;
    let assignment = id();

    let catalog: (String, bool, String, String) = sqlx::query_as(
        "SELECT owner.rolname,p.prosecdef, \
                array_to_string(p.proconfig,','),pg_get_function_result(p.oid) \
           FROM pg_proc p \
           JOIN pg_namespace n ON n.oid=p.pronamespace \
           JOIN pg_roles owner ON owner.oid=p.proowner \
          WHERE n.nspname='public' \
            AND p.oid='public.ple_prepare_assignment_creation_v1(uuid,uuid,uuid,uuid)'::regprocedure",
    )
    .fetch_one(&pool)
    .await
    .expect("creation prepare catalog row");
    assert_eq!(catalog.0, "ple_assignment_mutator_broker");
    assert!(catalog.1, "creation prepare is security definer");
    assert_eq!(
        catalog.2, "search_path=pg_catalog, public, pg_temp",
        "creation prepare has the exact fixed search path"
    );
    assert_eq!(
        catalog.3,
        "TABLE(tenant_id uuid, actor_id uuid, course_id uuid, assignment_id uuid, term_start_date date, term_end_date date, time_zone text)"
    );

    let privileges: (bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT \
           has_function_privilege('ple_app','public.ple_prepare_assignment_creation_v1(uuid,uuid,uuid,uuid)','EXECUTE'), \
           has_function_privilege('public','public.ple_prepare_assignment_creation_v1(uuid,uuid,uuid,uuid)','EXECUTE'), \
           has_function_privilege('ple_app','public.ple_assignment_mutator_require_create_editor(uuid,uuid,uuid,uuid)','EXECUTE'), \
           has_table_privilege('ple_app','public.course','UPDATE'), \
           has_table_privilege('ple_assignment_mutator_broker','public.course','SELECT')",
    )
    .fetch_one(&pool)
    .await
    .expect("creation prepare privilege matrix");
    assert_eq!(
        privileges,
        (true, false, false, false, true),
        "only the public witness exposes the broker-owned preparation seam"
    );

    let store_assignment = AssignmentId::from_uuid(id());
    let store = PostgresStore::new(pool.clone());
    let stored = store
        .create_assignment(
            TenantContext::from_authenticated_session(TenantId::from_uuid(source.tenant)),
            CreateAssignmentCommand {
                actor: UserId::from_uuid(source.actor),
                assignment: AssignmentRecord {
                    id: store_assignment,
                    tenant: TenantId::from_uuid(source.tenant),
                    course_id: CourseId::from_uuid(source.course),
                    title: "Ordinary prepared creation".to_string(),
                    lifecycle: AssignmentLifecycle::Draft,
                    instructions: AssignmentInstructions::default(),
                    audience: AssignmentAudience::CourseWide,
                    items: vec![AssignmentItem {
                        id: AssignmentItemId::from_uuid(id()),
                        reference: ProblemVersionRef {
                            problem: ProblemId::from_uuid(source.problem),
                            version: VersionId::from_uuid(source.version),
                        },
                        position: 0,
                        points_possible: PointValue::from_whole(1),
                        delivery_state: AssignmentDeliveryState::Active,
                        scoring_mode: AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AnswerAll,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
                base_policy: question_model::BaseAssignmentPolicy::default(),
            },
        )
        .await
        .expect("ordinary Postgres Store uses broker-prepared assignment creation");
    assert_eq!(stored.record.id, store_assignment);
    assert_eq!(stored.record.lifecycle, AssignmentLifecycle::Draft);

    let mut creator = app(&pool, source.tenant).await;
    prepared_creation(&mut creator, &source, assignment).await;

    let mut revoker = pool.begin().await.expect("begin membership revocation");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(source.tenant.to_string())
        .execute(&mut *revoker)
        .await
        .expect("revoker tenant");
    let blocked = sqlx::query(
        "SELECT course_membership_id FROM course_member \
          WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3 FOR UPDATE NOWAIT",
    )
    .bind(source.tenant)
    .bind(source.course)
    .bind(source.actor)
    .fetch_one(&mut *revoker)
    .await;
    assert!(
        blocked.is_err(),
        "the preparation witness retains the direct-Instructor membership lock"
    );
    revoker
        .rollback()
        .await
        .expect("rollback blocked revocation");

    let item = id();
    let candidate = id();
    let group = id();
    sqlx::query("SELECT * FROM public.ple_create_assignment_definition_v1($1,$2,$3,$4,$5,$6)")
        .bind(source.tenant)
        .bind(source.actor)
        .bind(source.course)
        .bind(assignment)
        .bind(valid_payload(
            &source,
            item,
            candidate,
            group,
            "Locked creation",
            "4.0",
        ))
        .bind(Option::<Uuid>::None)
        .bind(Option::<i32>::None)
        .execute(&mut *creator)
        .await
        .expect("prepared assignment creates atomically");
    creator.commit().await.expect("commit prepared creation");

    let mut revoke = pool.begin().await.expect("begin post-creation revocation");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(source.tenant.to_string())
        .execute(&mut *revoke)
        .await
        .expect("post-creation revoker tenant");
    sqlx::query(
        "UPDATE course_member SET status='revoked',revoked_at=transaction_timestamp() \
          WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3",
    )
    .bind(source.tenant)
    .bind(source.course)
    .bind(source.actor)
    .execute(&mut *revoke)
    .await
    .expect("revoke after creation commit");
    revoke.commit().await.expect("commit revocation");

    let refused_assignment = id();
    let mut refused = app(&pool, source.tenant).await;
    let denied =
        sqlx::query("SELECT * FROM public.ple_prepare_assignment_creation_v1($1,$2,$3,$4)")
            .bind(source.tenant)
            .bind(source.actor)
            .bind(source.course)
            .bind(refused_assignment)
            .fetch_one(&mut *refused)
            .await;
    assert!(
        denied.is_err(),
        "revoked Instructor cannot prepare creation"
    );
    refused
        .rollback()
        .await
        .expect("rollback denied preparation");
    let refused_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM assignment WHERE tenant_id=$1 AND assignment_id=$2)",
    )
    .bind(source.tenant)
    .bind(refused_assignment)
    .fetch_one(&pool)
    .await
    .expect("refused assignment absence");
    assert!(
        !refused_exists,
        "denied preparation leaves no partial assignment"
    );
}
