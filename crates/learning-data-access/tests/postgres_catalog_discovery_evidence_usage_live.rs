#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for WP-PROF-D1 evidence and usage authority.

use std::fs;
use std::str::FromStr;

use learning_data_access::postgres::{lazy_pool, verify_application_schema};
use sqlx::AssertSqlSafe;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Row};
use uuid::Uuid;

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

struct Fixture {
    tenant: Uuid,
    actor: Uuid,
    sysadmin: Uuid,
    question_id: &'static str,
    problem: Uuid,
    version: Uuid,
    replacement_problem: Uuid,
    replacement_version: Uuid,
    actor_course: Uuid,
    foreign_course: Uuid,
    actor_session: String,
    sysadmin_session: String,
    unapproved_instructor_session: String,
    foreign_session: String,
    observations: Vec<Observation>,
    ineligible_attempt: Uuid,
    later_attempt: Uuid,
}

#[derive(Clone, Copy)]
struct Observation {
    tenant: Uuid,
    enrollment: Uuid,
    run: Uuid,
    attempt: Uuid,
}

type ClosedBrokerRole = (String, bool, bool, bool, bool, bool, bool, bool);

async fn seed(pool: &PgPool) -> Fixture {
    let tenant = id();
    let tenant_two = id();
    let actor = id();
    let sysadmin = id();
    let unapproved_instructor = id();
    let foreign_actor = id();
    let problem = id();
    let version = id();
    let replacement_problem = id();
    let replacement_version = id();
    let actor_course = id();
    let foreign_course = id();
    let actor_assignment = id();
    let foreign_assignment = id();
    let tenant_two_course = id();
    let tenant_two_assignment = id();
    let actor_session = "a".repeat(64);
    let sysadmin_session = "c".repeat(64);
    let unapproved_instructor_session = "d".repeat(64);
    let foreign_session = "b".repeat(64);

    let mut catalog = pool.begin().await.expect("begin catalog fixture");
    for (user, name) in [(actor, "D1 Actor"), (foreign_actor, "D1 Foreign")] {
        let email = format!("d1-{user}@example.test");
        sqlx::query(
            "INSERT INTO ple_account (user_id,normalized_email,delivery_email,display_name) \
             VALUES ($1,$2,$2,$3)",
        )
        .bind(user)
        .bind(&email)
        .bind(name)
        .execute(&mut *catalog)
        .await
        .expect("insert instructor account");
        sqlx::query(
            "INSERT INTO instructor_approval \
             (user_id,approved_by,approved_at,revision) \
             VALUES ($1,$1,transaction_timestamp(),1)",
        )
        .bind(user)
        .execute(&mut *catalog)
        .await
        .expect("approve instructor");
    }
    let sysadmin_email = format!("d1-{sysadmin}@example.test");
    sqlx::query(
        "INSERT INTO ple_account (user_id,normalized_email,delivery_email,display_name) \
         VALUES ($1,$2,$2,'Morgan Sysadmin')",
    )
    .bind(sysadmin)
    .bind(&sysadmin_email)
    .execute(&mut *catalog)
    .await
    .expect("insert Morgan Sysadmin account");
    let unapproved_email = format!("d1-{unapproved_instructor}@example.test");
    sqlx::query(
        "INSERT INTO ple_account (user_id,normalized_email,delivery_email,display_name) \
         VALUES ($1,$2,$2,'D1 Unapproved Instructor')",
    )
    .bind(unapproved_instructor)
    .bind(&unapproved_email)
    .execute(&mut *catalog)
    .await
    .expect("insert unapproved Instructor account");
    sqlx::query(
        "INSERT INTO problem \
         (problem_id,owner_tenant_id,owner_user_id,visibility,license,lifecycle,question_id) \
         VALUES ($1,$2,$3,'public','CC-BY-4.0','published','D1A0001')",
    )
    .bind(problem)
    .bind(tenant)
    .bind(actor)
    .execute(&mut *catalog)
    .await
    .expect("insert discovery problem");
    sqlx::query(
        "INSERT INTO problem_version \
         (problem_id,version_id,content_sha256,workspace_id,title,lifecycle,backend, \
          publication_scope,author_ids,public_byline,response_family) \
         VALUES ($1,$2,$3,$4,'D1 evidence oracle','published','native','public', \
                 jsonb_build_array($5::text),ARRAY['D1 Oracle'],'multipleChoice')",
    )
    .bind(problem)
    .bind(version)
    .bind("d".repeat(64))
    .bind(id())
    .bind(actor)
    .execute(&mut *catalog)
    .await
    .expect("publish discovery problem");
    sqlx::query(
        "INSERT INTO problem_version_payload(problem_id,version_id,payload,payload_sha256) \
         VALUES ($1,$2, \
           '{\"question\":{\"response\":{\"kind\":\"multipleChoice\"}}}'::jsonb,$3)",
    )
    .bind(problem)
    .bind(version)
    .bind("d".repeat(64))
    .execute(&mut *catalog)
    .await
    .expect("insert immutable response-family source");
    sqlx::query(
        "INSERT INTO problem \
         (problem_id,owner_tenant_id,owner_user_id,visibility,license,lifecycle,question_id) \
         VALUES ($1,$2,$3,'public','CC-BY-4.0','published','D1A0002')",
    )
    .bind(replacement_problem)
    .bind(tenant)
    .bind(actor)
    .execute(&mut *catalog)
    .await
    .expect("insert replacement identity");
    sqlx::query(
        "INSERT INTO problem_version \
         (problem_id,version_id,content_sha256,workspace_id,title,lifecycle,backend, \
          publication_scope,author_ids,public_byline,response_family, \
          derived_from_problem_id,derived_from_version_id) \
         VALUES ($1,$2,$3,$4,'D1 explicit replacement','published','native','public', \
                 jsonb_build_array($5::text),ARRAY['D1 Oracle'],'multipleChoice',$6,$7)",
    )
    .bind(replacement_problem)
    .bind(replacement_version)
    .bind("e".repeat(64))
    .bind(id())
    .bind(actor)
    .bind(problem)
    .bind(version)
    .execute(&mut *catalog)
    .await
    .expect("publish explicitly linked replacement version");
    sqlx::query(
        "INSERT INTO problem_version_payload(problem_id,version_id,payload,payload_sha256) \
         VALUES ($1,$2,'{\"question\":{\"response\":{\"kind\":\"multipleChoice\"}}}'::jsonb,$3)",
    )
    .bind(replacement_problem)
    .bind(replacement_version)
    .bind("e".repeat(64))
    .execute(&mut *catalog)
    .await
    .expect("insert replacement immutable payload");
    catalog.commit().await.expect("commit catalog fixture");

    let mut fixture = pool.begin().await.expect("begin activity fixture");
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *fixture)
        .await
        .expect("disable unrelated fixture triggers");
    for (course_tenant, course, title) in [
        (tenant, actor_course, "Actor-owned D1 course"),
        (tenant, foreign_course, "Foreign instructor secret course"),
        (
            tenant_two,
            tenant_two_course,
            "Second tenant evidence course",
        ),
    ] {
        sqlx::query(
            "INSERT INTO course \
             (tenant_id,course_id,title,term_start_date,term_end_date,time_zone) \
             VALUES ($1,$2,$3,DATE '2026-08-01',DATE '2026-12-31','America/Chicago')",
        )
        .bind(course_tenant)
        .bind(course)
        .bind(title)
        .execute(&mut *fixture)
        .await
        .expect("insert course");
    }
    for (course, user) in [
        (actor_course, actor),
        (actor_course, foreign_actor),
        (foreign_course, foreign_actor),
    ] {
        sqlx::query(
            "INSERT INTO course_member \
             (tenant_id,course_id,course_membership_id,user_id,role,status,joined_at) \
             VALUES ($1,$2,$3,$4,'instructor','active',transaction_timestamp())",
        )
        .bind(tenant)
        .bind(course)
        .bind(id())
        .bind(user)
        .execute(&mut *fixture)
        .await
        .expect("insert instructor membership");
    }
    for (session, user, name, roles) in [
        (&actor_session, actor, "D1 Actor", r#"["instructor"]"#),
        (
            &sysadmin_session,
            sysadmin,
            "Morgan Sysadmin",
            r#"["sysadmin"]"#,
        ),
        (
            &unapproved_instructor_session,
            unapproved_instructor,
            "D1 Unapproved Instructor",
            r#"["instructor"]"#,
        ),
        (
            &foreign_session,
            foreign_actor,
            "D1 Foreign",
            r#"["instructor"]"#,
        ),
    ] {
        sqlx::query(
            "INSERT INTO auth_session \
             (session_hash,tenant_id,user_id,display_name,roles,expires_at) \
             VALUES ($1,$2,$3,$4,$5::jsonb, \
                     transaction_timestamp() + interval '1 hour')",
        )
        .bind(session)
        .bind(tenant)
        .bind(user)
        .bind(name)
        .bind(roles)
        .execute(&mut *fixture)
        .await
        .expect("insert catalog actor session");
    }
    for (assignment_tenant, assignment, course, title) in [
        (tenant, actor_assignment, actor_course, "Actor assignment"),
        (
            tenant,
            foreign_assignment,
            foreign_course,
            "Foreign assignment",
        ),
        (
            tenant_two,
            tenant_two_assignment,
            tenant_two_course,
            "Second tenant assignment",
        ),
    ] {
        sqlx::query(
            "INSERT INTO assignment \
             (tenant_id,assignment_id,course_id,title,lifecycle,audience_kind, \
              score_disclosure,per_item_correctness_disclosure,feedback_text_disclosure, \
              solution_disclosure,class_statistics_disclosure) \
             VALUES ($1,$2,$3,$4,'published','course_wide','after_submit', \
                     'after_submit','after_submit','after_submit','never')",
        )
        .bind(assignment_tenant)
        .bind(assignment)
        .bind(course)
        .bind(title)
        .execute(&mut *fixture)
        .await
        .expect("insert assignment");
        sqlx::query(
            "INSERT INTO assignment_item \
             (tenant_id,assignment_id,assignment_item_id,position,problem_id,version_id, \
              points_possible,delivery_state,scoring_mode) \
             VALUES ($1,$2,$3,0,$4,$5,1,'active','normal')",
        )
        .bind(assignment_tenant)
        .bind(assignment)
        .bind(id())
        .bind(problem)
        .bind(version)
        .execute(&mut *fixture)
        .await
        .expect("insert current fixed usage");
    }
    let selection_group = id();
    sqlx::query(
        "INSERT INTO assignment_selection_group \
         (tenant_id,assignment_id,selection_group_id,position,draw_count,points_per_item, \
          ordering_policy,algorithm_version) \
         VALUES ($1,$2,$3,1,1,1,'candidate_order',1)",
    )
    .bind(tenant)
    .bind(actor_assignment)
    .bind(selection_group)
    .execute(&mut *fixture)
    .await
    .expect("insert selection group");
    sqlx::query(
        "INSERT INTO assignment_selection_candidate \
         (tenant_id,assignment_id,selection_group_id,candidate_id,position,problem_id, \
          version_id,delivery_state) VALUES ($1,$2,$3,$4,0,$5,$6,'active')",
    )
    .bind(tenant)
    .bind(actor_assignment)
    .bind(selection_group)
    .bind(id())
    .bind(problem)
    .bind(version)
    .execute(&mut *fixture)
    .await
    .expect("insert current pool usage");
    sqlx::query(
        "INSERT INTO assignment_item \
         (tenant_id,assignment_id,assignment_item_id,position,problem_id,version_id, \
          points_possible,delivery_state,scoring_mode) \
         VALUES ($1,$2,$3,2,$4,$5,1,'active','normal')",
    )
    .bind(tenant)
    .bind(actor_assignment)
    .bind(id())
    .bind(replacement_problem)
    .bind(replacement_version)
    .execute(&mut *fixture)
    .await
    .expect("insert separate replacement usage");
    let mut observations = Vec::new();
    let mut ineligible_attempt = Uuid::nil();
    let mut later_attempt = Uuid::nil();
    let repeated_student = id();
    for index in 0..9 {
        let observation_tenant = if index == 8 { tenant_two } else { tenant };
        let course = if index < 5 {
            actor_course
        } else if index == 8 {
            tenant_two_course
        } else {
            foreign_course
        };
        let assignment = if index < 5 {
            actor_assignment
        } else if index == 8 {
            tenant_two_assignment
        } else {
            foreign_assignment
        };
        let enrollment = id();
        let run = id();
        let first_attempt = id();
        let student = if matches!(index, 0 | 5 | 8) {
            repeated_student
        } else {
            id()
        };
        sqlx::query(
            "INSERT INTO enrollment \
             (tenant_id,enrollment_id,assignment_id,student_id,user_id,course_id, \
              course_membership_id,materialized_at,materialization_purpose, \
              materialized_by_user_id,evaluator_version) \
             VALUES ($1,$2,$3,$4,$4,$5,$6,transaction_timestamp(), \
                     'instructor_issue',$7,1)",
        )
        .bind(observation_tenant)
        .bind(enrollment)
        .bind(assignment)
        .bind(student)
        .bind(course)
        .bind(id())
        .bind(if index < 5 { actor } else { foreign_actor })
        .execute(&mut *fixture)
        .await
        .expect("insert enrollment");
        sqlx::query(
            "INSERT INTO assignment_run \
             (tenant_id,run_id,enrollment_id,run_number,started_at,completed_at,payload,payload_sha256) \
             VALUES ($1,$2,$3,1,transaction_timestamp() - interval '2 minutes', \
                     transaction_timestamp(),'{\"mode\":\"assigned\"}'::jsonb,$4)",
        )
        .bind(observation_tenant)
        .bind(run)
        .bind(enrollment)
        .bind("1".repeat(64))
        .execute(&mut *fixture)
        .await
        .expect("insert completed run");

        let canonical_position = if index == 0 { 1 } else { 0 };
        if index == 0 {
            sqlx::query(
                "INSERT INTO assignment_run_item \
                 (tenant_id,run_id,assignment_item_id,source_position,issued_position, \
                  problem_id,version_id,delivery_status,statistics_eligible) \
                 VALUES ($1,$2,$3,0,0,$4,$5,'submitted',false)",
            )
            .bind(observation_tenant)
            .bind(run)
            .bind(id())
            .bind(problem)
            .bind(version)
            .execute(&mut *fixture)
            .await
            .expect("insert ineligible duplicate position");
        }
        sqlx::query(
            "INSERT INTO assignment_run_item \
             (tenant_id,run_id,assignment_item_id,source_position,issued_position, \
              problem_id,version_id,delivery_status,statistics_eligible) \
             VALUES ($1,$2,$3,0,$4,$5,$6,'submitted',true)",
        )
        .bind(observation_tenant)
        .bind(run)
        .bind(id())
        .bind(canonical_position)
        .bind(problem)
        .bind(version)
        .execute(&mut *fixture)
        .await
        .expect("insert eligible issued position");

        let mut attempts = vec![(first_attempt, canonical_position, 1_i64)];
        if index == 0 {
            ineligible_attempt = id();
            later_attempt = id();
            attempts.push((ineligible_attempt, 0, 1));
            attempts.push((later_attempt, canonical_position, 2));
        }
        for (attempt, position, minute) in attempts {
            sqlx::query(
                "INSERT INTO question_attempt \
                 (tenant_id,attempt_id,run_id,problem_id,version_id,occurred_at,payload, \
                  payload_sha256,attempt_status,submitted_at,assignment_position,course_id, \
                  presentation_capability,issued_question_snapshot_payload, \
                  issued_question_snapshot_payload_sha256,authored_timing_grace_seconds) \
                 VALUES ($1,$2,$3,$4,$5,transaction_timestamp() - interval '3 minutes' \
                         + $6 * interval '1 second','{}'::jsonb,$7,'submitted', \
                         transaction_timestamp() - interval '2 minutes' \
                         + $6 * interval '1 second',$8,$9,'not_applicable', \
                         '{\"schemaVersion\":1,\"question\":{},\"familyWitness\":{\"family\":\"native\",\"physicalAssetBindings\":[]}}'::jsonb, \
                         $10,0)",
            )
            .bind(observation_tenant)
            .bind(attempt)
            .bind(run)
            .bind(problem)
            .bind(version)
            .bind(minute)
            .bind("2".repeat(64))
            .bind(position)
            .bind(course)
            .bind("3".repeat(64))
            .execute(&mut *fixture)
            .await
            .expect("insert scored attempt");
            sqlx::query(
                "INSERT INTO submission_idempotency \
                 (tenant_id,attempt_id,idempotency_key,request_sha256,submitted_at,payload, \
                  payload_sha256,course_id,request_contract_version) \
                 VALUES ($1,$2,$3,$4,transaction_timestamp(),'{}'::jsonb,$5,$6,1)",
            )
            .bind(observation_tenant)
            .bind(attempt)
            .bind(format!("d1-{attempt}"))
            .bind("5".repeat(64))
            .bind("6".repeat(64))
            .bind(course)
            .execute(&mut *fixture)
            .await
            .expect("insert accepted-submission receipt");
            sqlx::query(
                "INSERT INTO submission_evaluation \
                 (tenant_id,attempt_id,submission_id,credit_fraction,correct,grading_status, \
                  payload,payload_sha256,course_id) \
                 VALUES ($1,$2,$2,0.5,false,'graded','{}'::jsonb,$3,$4)",
            )
            .bind(observation_tenant)
            .bind(attempt)
            .bind("4".repeat(64))
            .bind(course)
            .execute(&mut *fixture)
            .await
            .expect("insert scored evaluation");
        }
        observations.push(Observation {
            tenant: observation_tenant,
            enrollment,
            run,
            attempt: first_attempt,
        });
    }
    fixture.commit().await.expect("commit activity fixture");
    Fixture {
        tenant,
        actor,
        sysadmin,
        question_id: "D1A0001",
        problem,
        version,
        replacement_problem,
        replacement_version,
        actor_course,
        foreign_course,
        actor_session,
        sysadmin_session,
        unapproved_instructor_session,
        foreign_session,
        observations,
        ineligible_attempt,
        later_attempt,
    }
}

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
    let fixture = seed(&pool).await;

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
#[path = "postgres_catalog_discovery_evidence_usage_live/snapshot_cases.rs"]
mod snapshot_cases;
