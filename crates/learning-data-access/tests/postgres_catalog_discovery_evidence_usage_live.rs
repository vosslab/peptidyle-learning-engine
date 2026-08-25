#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for WP-PROF-D1 evidence and usage authority.

use std::fs;
use std::str::FromStr;

use learning_data_access::postgres::{lazy_pool, verify_application_schema};
use sqlx::AssertSqlSafe;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use fixture::{Fixture, Observation};

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn migration_copy(maximum_version: Option<i64>) -> std::path::PathBuf {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/migrations");
    let destination = std::env::temp_dir().join(format!("ple-d1-migrations-{}", id()));
    fs::create_dir_all(&destination).expect("temporary D1 migration directory");
    for entry in fs::read_dir(source).expect("migration directory") {
        let entry = entry.expect("migration entry");
        let name = entry.file_name();
        let version = name
            .to_string_lossy()
            .split('_')
            .next()
            .and_then(|value| value.parse::<i64>().ok())
            .expect("migration filename begins with a numeric version");
        if maximum_version.is_some_and(|maximum| version > maximum) {
            continue;
        }
        fs::copy(entry.path(), destination.join(name)).expect("copy D1 migration input");
    }
    destination
}

async fn migration_admin_pool(url: &str) -> PgPool {
    let options = PgConnectOptions::from_str(url)
        .expect("acceptance PostgreSQL URL")
        .database("postgres");
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .expect("D1 migration admin connection")
}

type ClosedBrokerRole = (String, bool, bool, bool, bool, bool, bool, bool);

async fn record(
    pool: &PgPool,
    fixture: &Fixture,
    observation: Observation,
    attempt: Uuid,
) -> Result<bool, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(observation.tenant.to_string())
        .execute(&mut *transaction)
        .await?;
    let recorded = sqlx::query_scalar(
        "SELECT ple_record_question_statistics( \
            $1,$2,$3,$4,$5,$6,0.5::double precision,1,12,NULL,$7)",
    )
    .bind(observation.tenant)
    .bind(observation.enrollment)
    .bind(observation.run)
    .bind(attempt)
    .bind(fixture.problem)
    .bind(fixture.version)
    .bind(vec![5_u8; 32])
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(recorded)
}

async fn catalog_brokers_are_exactly_closed(pool: &PgPool) {
    let roles: Vec<ClosedBrokerRole> = sqlx::query_as(
        "SELECT rolname,rolcanlogin,rolsuper,rolcreatedb,rolcreaterole,rolinherit, \
                rolreplication,rolbypassrls \
           FROM pg_roles \
          WHERE rolname IN ('ple_statistics_broker','ple_catalog_usage_broker') \
          ORDER BY rolname",
    )
    .fetch_all(pool)
    .await
    .expect("catalog broker role catalog");
    assert_eq!(
        roles,
        vec![
            (
                "ple_catalog_usage_broker".to_string(),
                false,
                false,
                false,
                false,
                false,
                false,
                false,
            ),
            (
                "ple_statistics_broker".to_string(),
                false,
                false,
                false,
                false,
                false,
                false,
                false,
            ),
        ],
        "both catalog brokers have exactly the closed role flags"
    );
    let memberships: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_auth_members \
          WHERE member IN ('ple_statistics_broker'::regrole,'ple_catalog_usage_broker'::regrole) \
             OR roleid IN ('ple_statistics_broker'::regrole,'ple_catalog_usage_broker'::regrole)",
    )
    .fetch_one(pool)
    .await
    .expect("catalog broker membership graph");
    assert_eq!(memberships, 0, "catalog brokers have no membership edges");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_broker_role_sealing_repairs_pre_d1_epoch_drift() {
    let runtime = load_acceptance_runtime();
    let url = runtime.admin_url().expose().to_owned();
    let admin = migration_admin_pool(&url).await;
    let database = format!("ple_d1_roles_{:x}", id().as_u128());
    assert!(
        database.len() < 64,
        "generated database identifier is bounded"
    );
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database}")))
        .execute(&admin)
        .await
        .expect("create isolated D1 migration database");
    let cleanup_database = database.clone();
    let result = tokio::spawn(async move {
        let options = PgConnectOptions::from_str(&url)
            .expect("acceptance PostgreSQL URL")
            .database(&database);
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .expect("isolated D1 migration database connection");
        let pre_d1 = migration_copy(Some(2026081826));
        sqlx::migrate::Migrator::new(pre_d1.clone())
            .await
            .expect("pre-D1 migration source")
            .run(&pool)
            .await
            .expect("migrate through 1826");
        sqlx::raw_sql(
            "DO $$ BEGIN \
                 IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname='ple_catalog_usage_broker') \
                 THEN CREATE ROLE ple_catalog_usage_broker; END IF; END $$; \
             ALTER ROLE ple_catalog_usage_broker LOGIN SUPERUSER CREATEROLE CREATEDB \
                 INHERIT REPLICATION BYPASSRLS; \
             ALTER ROLE ple_statistics_broker LOGIN SUPERUSER CREATEROLE CREATEDB \
                 INHERIT REPLICATION BYPASSRLS; \
             GRANT ple_app TO ple_statistics_broker; \
             GRANT ple_catalog_usage_broker TO ple_app;",
        )
        .execute(&pool)
        .await
        .expect("inject pre-D1 broker role and bidirectional membership drift");
        let full = migration_copy(None);
        sqlx::migrate::Migrator::new(full.clone())
            .await
            .expect("full D1 migration source")
            .run(&pool)
            .await
            .expect("1827 and 1828 repair the pre-D1 broker drift exactly once");
        catalog_brokers_are_exactly_closed(&pool).await;
        let applied: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public._sqlx_migrations \
             WHERE success AND version IN (2026081827,2026081828)",
        )
        .fetch_one(&pool)
        .await
        .expect("D1 migration ledger rows");
        assert_eq!(applied, 2, "both canonical D1 migrations apply once");
        pool.close().await;
        fs::remove_dir_all(pre_d1).expect("remove pre-D1 migration copy");
        fs::remove_dir_all(full).expect("remove full D1 migration copy");
    })
    .await;
    let _ = sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
        .bind(&cleanup_database)
        .execute(&admin)
        .await;
    sqlx::raw_sql(
        "ALTER ROLE ple_statistics_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE \
             NOINHERIT NOREPLICATION NOBYPASSRLS; \
         ALTER ROLE ple_catalog_usage_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE \
             NOINHERIT NOREPLICATION NOBYPASSRLS; \
         REVOKE ple_app FROM ple_statistics_broker; \
         REVOKE ple_catalog_usage_broker FROM ple_app;",
    )
    .execute(&admin)
    .await
    .expect("restore shared PostgreSQL broker posture after D1 fixture");
    sqlx::query(AssertSqlSafe(format!(
        "DROP DATABASE IF EXISTS {cleanup_database}"
    )))
    .execute(&admin)
    .await
    .expect("drop isolated D1 migration database");
    result.expect("pre-D1 drift fixture task");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_discovery_evidence_and_usage_are_validity_and_actor_bound() {
    let runtime = load_acceptance_runtime();
    let pool = lazy_pool(runtime.admin_url().expose()).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let fixture = fixture::seed(&pool).await;

    for invalid in [fixture.ineligible_attempt, fixture.later_attempt] {
        let error = record(&pool, &fixture, fixture.observations[0], invalid)
            .await
            .expect_err("noncanonical attempt is rejected");
        assert_eq!(
            error
                .as_database_error()
                .and_then(|value| value.code())
                .as_deref(),
            Some("22023")
        );
    }

    for observation in fixture.observations.iter().take(5).copied() {
        assert!(
            record(&pool, &fixture, observation, observation.attempt)
                .await
                .expect("record same-course contribution")
        );
    }
    let suppressed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM catalog_discovery_evidence_revision \
         WHERE problem_id=$1 AND version_id=$2",
    )
    .bind(fixture.problem)
    .bind(fixture.version)
    .fetch_one(&pool)
    .await
    .expect("count suppressed revisions");
    assert_eq!(suppressed, 0, "one course never crosses disclosure");

    let duplicate_cross_course = fixture.observations[5];
    assert!(
        record(
            &pool,
            &fixture,
            duplicate_cross_course,
            duplicate_cross_course.attempt
        )
        .await
        .expect("record duplicate learner in another course")
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM catalog_discovery_course_fingerprint_receipt \
             WHERE problem_id=$1 AND version_id=$2",
        )
        .bind(fixture.problem)
        .bind(fixture.version)
        .fetch_one(&pool)
        .await
        .expect("duplicate learner does not advance course breadth"),
        1
    );
    let cross_course = fixture.observations[6];
    assert!(
        record(&pool, &fixture, cross_course, cross_course.attempt)
            .await
            .expect("record cross-course contribution")
    );
    let first = sqlx::query(
        "SELECT evidence_sequence,course_count,first_attempt_count,formula_version, \
                discrimination_index,quality_signal::text AS quality \
         FROM catalog_discovery_evidence_revision \
         WHERE problem_id=$1 AND version_id=$2",
    )
    .bind(fixture.problem)
    .bind(fixture.version)
    .fetch_one(&pool)
    .await
    .expect("cross-course revision");
    let boundary: i64 = first.get("evidence_sequence");
    assert_eq!(first.get::<i64, _>("course_count"), 2);
    assert_eq!(first.get::<i64, _>("first_attempt_count"), 6);
    assert_eq!(first.get::<i16, _>("formula_version"), 1);
    assert!(
        first
            .get::<Option<f64>, _>("discrimination_index")
            .is_none()
    );
    assert_ne!(first.get::<String, _>("quality"), "0.000000");
    assert!(
        !record(&pool, &fixture, cross_course, cross_course.attempt)
            .await
            .expect("exact receipt replay")
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM catalog_discovery_evidence_revision \
             WHERE problem_id=$1 AND version_id=$2",
        )
        .bind(fixture.problem)
        .bind(fixture.version)
        .fetch_one(&pool)
        .await
        .expect("replay revision count"),
        1
    );
    let later = fixture.observations[7];
    assert!(
        record(&pool, &fixture, later, later.attempt)
            .await
            .expect("record later valid contribution")
    );
    let cohort_before_other_tenant: i64 = sqlx::query_scalar(
        "SELECT cohort_size FROM question_statistics_aggregate \
         WHERE problem_id=$1 AND version_id=$2",
    )
    .bind(fixture.problem)
    .bind(fixture.version)
    .fetch_one(&pool)
    .await
    .expect("read cohort before tenant-local identity witness");
    let other_tenant = fixture.observations[8];
    assert!(
        record(&pool, &fixture, other_tenant, other_tenant.attempt)
            .await
            .expect("same logical learner in another tenant contributes independently")
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT cohort_size FROM question_statistics_aggregate \
             WHERE problem_id=$1 AND version_id=$2",
        )
        .bind(fixture.problem)
        .bind(fixture.version)
        .fetch_one(&pool)
        .await
        .expect("read tenant-local independent cohort"),
        cohort_before_other_tenant + 1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM question_statistics_contribution_receipt \
             WHERE problem_id=$1 AND version_id=$2 \
               AND contribution_disposition='duplicateLearner'",
        )
        .bind(fixture.problem)
        .bind(fixture.version)
        .fetch_one(&pool)
        .await
        .expect("count accepted duplicate-learner audit receipts"),
        1
    );
    let learner_fingerprints: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM catalog_discovery_learner_fingerprint_receipt \
         WHERE problem_id=$1 AND version_id=$2",
    )
    .bind(fixture.problem)
    .bind(fixture.version)
    .fetch_one(&pool)
    .await
    .expect("count independent anonymous learners");
    assert_eq!(learner_fingerprints, 8);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM catalog_discovery_course_fingerprint_receipt \
             WHERE problem_id=$1 AND version_id=$2",
        )
        .bind(fixture.problem)
        .bind(fixture.version)
        .fetch_one(&pool)
        .await
        .expect("count independently witnessed tenant-local courses"),
        3
    );
    sqlx::query(
        "DELETE FROM question_statistics_contribution_receipt \
         WHERE tenant_id=$1 AND enrollment_id=$2 AND problem_id=$3 AND version_id=$4",
    )
    .bind(fixture.tenant)
    .bind(fixture.observations[0].enrollment)
    .bind(fixture.problem)
    .bind(fixture.version)
    .execute(&pool)
    .await
    .expect("simulate retention of identity-bearing audit receipt");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM catalog_discovery_learner_fingerprint_receipt \
             WHERE problem_id=$1 AND version_id=$2",
        )
        .bind(fixture.problem)
        .bind(fixture.version)
        .fetch_one(&pool)
        .await
        .expect("anonymous learner evidence survives identity retention"),
        learner_fingerprints
    );
    let aggregate_before_replay: i64 = sqlx::query_scalar(
        "SELECT cohort_size FROM question_statistics_aggregate \
         WHERE problem_id=$1 AND version_id=$2",
    )
    .bind(fixture.problem)
    .bind(fixture.version)
    .fetch_one(&pool)
    .await
    .expect("read aggregate before retained-identity replay");
    assert!(
        record(
            &pool,
            &fixture,
            fixture.observations[0],
            fixture.observations[0].attempt
        )
        .await
        .expect("accept retained-identity replay as duplicate learner")
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT cohort_size FROM question_statistics_aggregate \
             WHERE problem_id=$1 AND version_id=$2",
        )
        .bind(fixture.problem)
        .bind(fixture.version)
        .fetch_one(&pool)
        .await
        .expect("read aggregate after retained-identity replay"),
        aggregate_before_replay
    );

    let mut app = pool.begin().await.expect("begin app evidence read");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *app)
        .await
        .expect("assume app role");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *app)
        .await
        .expect("set app tenant");
    let as_of: i64 = sqlx::query_scalar(
        "SELECT first_attempt_count FROM ple_catalog_discovery_evidence_at($1,$2,$3)",
    )
    .bind(fixture.problem)
    .bind(fixture.version)
    .bind(boundary)
    .fetch_one(&mut *app)
    .await
    .expect("read cursor-bound evidence");
    let latest: i64 = sqlx::query_scalar(
        "SELECT first_attempt_count FROM ple_catalog_discovery_evidence_at($1,$2,$3)",
    )
    .bind(fixture.problem)
    .bind(fixture.version)
    .bind(i64::MAX)
    .fetch_one(&mut *app)
    .await
    .expect("read latest evidence");
    assert_eq!((as_of, latest), (6, 8));
    app.rollback().await.expect("rollback evidence read");

    let privileges = sqlx::query(
        "SELECT has_table_privilege('ple_app', \
                    'public.catalog_discovery_course_fingerprint_receipt','SELECT') AS app_private, \
                has_function_privilege('public', \
                    'public.ple_record_question_statistics(uuid,uuid,uuid,uuid,uuid,uuid,double precision,bigint,bigint,double precision,bytea)'::regprocedure, \
                    'EXECUTE') AS public_record, \
                (SELECT rolcanlogin OR rolinherit OR rolbypassrls OR rolsuper \
                   FROM pg_roles WHERE rolname='ple_catalog_usage_broker') AS unsafe_broker",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect evidence ACL");
    assert!(!privileges.get::<bool, _>("app_private"));
    assert!(!privileges.get::<bool, _>("public_record"));
    assert!(!privileges.get::<bool, _>("unsafe_broker"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM information_schema.columns \
             WHERE table_schema='public' \
               AND table_name='catalog_discovery_learner_fingerprint_receipt' \
               AND column_name IN ('tenant_id','student_id','user_id','enrollment_id')",
        )
        .fetch_one(&pool)
        .await
        .expect("inspect anonymous learner receipt shape"),
        0
    );

    let mut usage = pool.begin().await.expect("begin actor usage read");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *usage)
        .await
        .expect("assume app usage role");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *usage)
        .await
        .expect("set usage tenant");
    sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
        .bind(&fixture.actor_session)
        .execute(&mut *usage)
        .await
        .expect("present actor session");
    let summary = sqlx::query("SELECT * FROM ple_instructor_catalog_usage_summary($1,$2,$3)")
        .bind(fixture.tenant)
        .bind(&fixture.actor_session)
        .bind(fixture.question_id)
        .fetch_one(&mut *usage)
        .await
        .expect("read usage summary");
    assert_eq!(summary.get::<i64, _>("institution_course_count"), 2);
    assert_eq!(summary.get::<i64, _>("institution_assignment_count"), 2);
    assert_eq!(summary.get::<i64, _>("own_course_count"), 1);
    assert_eq!(summary.get::<i64, _>("own_assignment_count"), 1);
    sqlx::query("SAVEPOINT unapproved_catalog_actor")
        .execute(&mut *usage)
        .await
        .expect("isolate expected unapproved Instructor refusal");
    sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
        .bind(&fixture.unapproved_instructor_session)
        .execute(&mut *usage)
        .await
        .expect("present unapproved Instructor session");
    let unapproved_error =
        sqlx::query("SELECT * FROM ple_begin_instructor_catalog_usage_snapshot($1,$2,300,5000)")
            .bind(fixture.tenant)
            .bind(&fixture.unapproved_instructor_session)
            .fetch_all(&mut *usage)
            .await
            .expect_err("unapproved Instructor cannot begin a catalog usage snapshot");
    assert_eq!(
        unapproved_error
            .as_database_error()
            .and_then(|value| value.code())
            .as_deref(),
        Some("42501")
    );
    sqlx::query("ROLLBACK TO SAVEPOINT unapproved_catalog_actor")
        .execute(&mut *usage)
        .await
        .expect("restore transaction after expected catalog refusal");
    sqlx::query("RELEASE SAVEPOINT unapproved_catalog_actor")
        .execute(&mut *usage)
        .await
        .expect("release expected-refusal savepoint");
    sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
        .bind(&fixture.sysadmin_session)
        .execute(&mut *usage)
        .await
        .expect("present Morgan Sysadmin session");
    let sysadmin_summary =
        sqlx::query("SELECT * FROM ple_instructor_catalog_usage_summary($1,$2,$3)")
            .bind(fixture.tenant)
            .bind(&fixture.sysadmin_session)
            .bind(fixture.question_id)
            .fetch_one(&mut *usage)
            .await
            .expect("Morgan reads aggregate catalog usage without Instructor membership");
    assert_eq!(
        sysadmin_summary.get::<i64, _>("institution_course_count"),
        2
    );
    assert_eq!(
        sysadmin_summary.get::<i64, _>("institution_assignment_count"),
        2
    );
    assert_eq!(sysadmin_summary.get::<i64, _>("own_course_count"), 0);
    assert_eq!(sysadmin_summary.get::<i64, _>("own_assignment_count"), 0);
    let sysadmin_named_courses: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_instructor_catalog_course_usage($1,$2,$3,NULL,20)",
    )
    .bind(fixture.tenant)
    .bind(&fixture.sysadmin_session)
    .bind(fixture.question_id)
    .fetch_one(&mut *usage)
    .await
    .expect("Morgan receives no ambient course names");
    assert_eq!(sysadmin_named_courses, 0);
    let sysadmin_snapshot = sqlx::query(
        "SELECT row_count FROM ple_begin_instructor_catalog_usage_snapshot($1,$2,300,5000)",
    )
    .bind(fixture.tenant)
    .bind(&fixture.sysadmin_session)
    .fetch_one(&mut *usage)
    .await
    .expect("Morgan begins an empty own-course usage snapshot");
    assert_eq!(sysadmin_snapshot.get::<i32, _>("row_count"), 0);
    sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
        .bind(&fixture.actor_session)
        .execute(&mut *usage)
        .await
        .expect("restore Instructor session for own-course detail");
    let courses =
        sqlx::query("SELECT * FROM ple_instructor_catalog_course_usage($1,$2,$3,NULL,20)")
            .bind(fixture.tenant)
            .bind(&fixture.actor_session)
            .bind(fixture.question_id)
            .fetch_all(&mut *usage)
            .await
            .expect("read actor-owned course rows");
    assert_eq!(courses.len(), 1);
    assert_eq!(
        courses[0].get::<String, _>("course_title"),
        "Actor-owned D1 course"
    );
    assert_eq!(courses[0].get::<i64, _>("assignment_count"), 1);
    assert_ne!(
        courses[0].get::<String, _>("course_title"),
        "Foreign instructor secret course"
    );
    let replacement_usage: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_instructor_catalog_course_usage($1,$2,'D1A0002',NULL,20)",
    )
    .bind(fixture.tenant)
    .bind(&fixture.actor_session)
    .fetch_one(&mut *usage)
    .await
    .expect("read exact replacement usage");
    assert_eq!(replacement_usage, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM catalog_discovery_evidence_revision \
             WHERE problem_id=$1 AND version_id=$2",
        )
        .bind(fixture.replacement_problem)
        .bind(fixture.replacement_version)
        .fetch_one(&mut *usage)
        .await
        .expect("replacement evidence remains separate"),
        0
    );
    snapshot_cases::run(&pool, &fixture, usage).await;

    let mut index_transaction = pool.begin().await.expect("begin reverse index proof");
    for (query, index) in [
        (
            "EXPLAIN (COSTS OFF, FORMAT JSON) SELECT assignment_id FROM assignment_item WHERE problem_id=$1 AND version_id=$2 AND delivery_state='active'",
            "assignment_item_active_publication_usage_idx",
        ),
        (
            "EXPLAIN (COSTS OFF, FORMAT JSON) SELECT assignment_id FROM assignment_selection_candidate WHERE problem_id=$1 AND version_id=$2 AND delivery_state='active'",
            "assignment_selection_candidate_active_publication_usage_idx",
        ),
    ] {
        sqlx::query("SET LOCAL enable_seqscan=off")
            .execute(&mut *index_transaction)
            .await
            .expect("prefer reverse index for capability proof");
        let plan: serde_json::Value = sqlx::query_scalar(query)
            .bind(fixture.problem)
            .bind(fixture.version)
            .fetch_one(&mut *index_transaction)
            .await
            .expect("explain publication reverse lookup");
        assert!(
            plan.to_string().contains(index),
            "publication reverse lookup uses {index} when a selective index path is requested"
        );
    }
    index_transaction
        .rollback()
        .await
        .expect("rollback reverse index proof");

    assert_ne!(fixture.actor_course, fixture.foreign_course);
    assert_ne!(fixture.actor, Uuid::nil());
    assert_ne!(fixture.sysadmin, Uuid::nil());
}

#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
#[path = "postgres_catalog_discovery_evidence_usage_live/fixture.rs"]
mod fixture;
#[path = "postgres_catalog_discovery_evidence_usage_live/snapshot_cases.rs"]
mod snapshot_cases;
