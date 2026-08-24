#![cfg(feature = "postgres")]

#[path = "conformance.rs"]
mod conformance;

// Disposable PostgreSQL proof for the WP-R2 immutable-publication boundary.

use learning_data_access::QuestionIdCodec;
use learning_data_access::postgres::{
    PostgresStore, apply_migrations, lazy_pool, migration_status, verify_application_schema,
};
use sqlx::Row;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

#[tokio::test]
#[ignore = "requires the disposable WP-R2 PostgreSQL acceptance database"]
async fn postgres_wp_r2_persistence_rls_and_no_drift() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid disposable PostgreSQL URL");

    apply_migrations(&pool)
        .await
        .expect("embedded migrations apply to a fresh database");
    let first_status = migration_status(&pool)
        .await
        .expect("migration status reads after application");
    apply_migrations(&pool)
        .await
        .expect("embedded migrations converge when applied again");
    assert_eq!(
        migration_status(&pool)
            .await
            .expect("migration status reads after convergence"),
        first_status,
        "the current embedded migration epoch remains verified after replay"
    );
    verify_application_schema(&pool)
        .await
        .expect("restricted application role verifies the migrated schema");

    let mut question_id_secret = [0_u8; 32];
    getrandom::fill(&mut question_id_secret).expect("live Question ID secret randomness");
    let question_ids = QuestionIdCodec::from_server_secret(question_id_secret);
    let store = PostgresStore::with_question_id_secret(pool.clone(), question_id_secret);
    conformance::exercise_durable_publication_assignment_contract(&store).await;

    let tenant_a = id();
    let tenant_b = id();
    let course_a = id();
    let course_b = id();
    sqlx::query(
        "INSERT INTO course (tenant_id, course_id, title, term_start_date, term_end_date, \
         time_zone) VALUES ($1, $2, $3, DATE '2026-08-24', DATE '2026-12-18', \
         'America/Chicago'), ($4, $5, $6, DATE '2026-08-24', DATE '2026-12-18', \
         'America/Chicago')",
    )
    .bind(tenant_a)
    .bind(course_a)
    .bind("Tenant A live course")
    .bind(tenant_b)
    .bind(course_b)
    .bind("Tenant B live course")
    .execute(&pool)
    .await
    .expect("database owner creates isolated records before the RLS probe");

    let mut foreign = pool.begin().await.expect("foreign RLS transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *foreign)
        .await
        .expect("actual restricted application role is selectable");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant_b.to_string())
        .execute(&mut *foreign)
        .await
        .expect("foreign tenant context sets only in this transaction");
    let visible: i64 = sqlx::query_scalar("SELECT count(*) FROM course WHERE course_id = $1")
        .bind(course_a)
        .fetch_one(&mut *foreign)
        .await
        .expect("restricted foreign read executes");
    assert_eq!(visible, 0, "forced RLS conceals tenant A from tenant B");
    let mutation_error = sqlx::query("UPDATE course SET title = $1 WHERE course_id = $2")
        .bind("foreign mutation")
        .bind(course_a)
        .execute(&mut *foreign)
        .await
        .expect_err("the application role has no direct course mutation capability");
    assert_eq!(
        mutation_error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("42501"),
        "foreign tenant mutation is denied at the role boundary"
    );
    foreign
        .rollback()
        .await
        .expect("foreign probe rolls back after the expected permission denial");

    let row = sqlx::query("SELECT title FROM course WHERE tenant_id = $1 AND course_id = $2")
        .bind(tenant_a)
        .bind(course_a)
        .fetch_one(&pool)
        .await
        .expect("owner reads its record after rejected foreign mutation");
    assert_eq!(
        row.try_get::<String, _>("title").expect("course title"),
        "Tenant A live course",
        "foreign refusal leaves the target record unchanged"
    );

    // The current schema makes VersionId globally unique. This database-level
    // probe uses fresh values and demonstrates that a new ProblemId cannot
    // convert an existing immutable publication version into a valid duplicate.
    let problem_a = id();
    let problem_b = id();
    let version = id();
    let question_a = question_ids
        .issue()
        .expect("live schema fixture Question ID")
        .compact()
        .to_string();
    let question_b = question_ids
        .issue()
        .expect("second live schema fixture Question ID")
        .compact()
        .to_string();
    sqlx::query(
        "INSERT INTO problem (problem_id, owner_tenant_id, owner_user_id, visibility, license, question_id) \
         VALUES ($1, $2, $3, 'public', 'CC BY-SA', $4)",
    )
    .bind(problem_a)
    .bind(tenant_a)
    .bind(id())
    .bind(question_a)
    .execute(&pool)
    .await
    .expect("first immutable problem persists");
    sqlx::query(
        "INSERT INTO problem (problem_id, owner_tenant_id, owner_user_id, visibility, license, question_id) \
         VALUES ($1, $2, $3, 'public', 'CC BY-SA', $4)",
    )
    .bind(problem_b)
    .bind(tenant_a)
    .bind(id())
    .bind(question_b)
    .execute(&pool)
    .await
    .expect("second immutable problem persists");
    sqlx::query(
        "INSERT INTO problem_version \
         (problem_id, version_id, content_sha256, workspace_id, title, author_ids, public_byline) \
         VALUES ($1, $2, repeat('a', 64), $3, 'First immutable question', \
                 jsonb_build_array($4::text), ARRAY['WP-R2 fixture'])",
    )
    .bind(problem_a)
    .bind(version)
    .bind(id())
    .bind(id())
    .execute(&pool)
    .await
    .expect("first immutable version persists");
    let duplicate = sqlx::query(
        "INSERT INTO problem_version \
         (problem_id, version_id, content_sha256, workspace_id, title, author_ids, public_byline) \
         VALUES ($1, $2, repeat('b', 64), $3, 'Duplicate immutable question', \
                 jsonb_build_array($4::text), ARRAY['WP-R2 fixture'])",
    )
    .bind(problem_b)
    .bind(version)
    .bind(id())
    .bind(id())
    .execute(&pool)
    .await;
    assert!(duplicate.is_err(), "globally reused VersionId is refused");

    // Store-level publication/replacement/run snapshot cases are exercised by
    // the focused conformance suites; this live owner additionally proves the
    // database authority those commands depend on: migration convergence,
    // actual restricted role, forced RLS, global version uniqueness, and
    // direct application-role mutation refusal.
}
