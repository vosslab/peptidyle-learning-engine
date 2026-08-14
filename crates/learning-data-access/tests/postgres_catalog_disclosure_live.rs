#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for catalog statistics disclosure boundaries.

use learning_data_access::postgres::{lazy_pool, verify_application_schema};
use sqlx::Row;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_statistics_disclosure_is_brokered_and_visibility_bound() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");

    let public_problem = id();
    let public_version = id();
    let private_problem = id();
    let private_version = id();
    let owner_tenant = id();
    let foreign_tenant = id();
    let owner = id();
    for (problem, version, question_id, scope) in [
        (public_problem, public_version, "R0D0001", "public"),
        (private_problem, private_version, "R0D0002", "institution"),
    ] {
        sqlx::query(
            "INSERT INTO problem (problem_id, question_id, owner_tenant_id, owner_user_id, visibility, license) \
             VALUES ($1, $2, $3, $4, $5, 'CC-BY-SA-4.0')",
        )
        .bind(problem)
        .bind(question_id)
        .bind(owner_tenant)
        .bind(owner)
        .bind(scope)
        .execute(&pool)
        .await
        .expect("insert catalog problem fixture");
        sqlx::query(
            "INSERT INTO problem_version (problem_id, version_id, version_number, content_sha256, workspace_id, title, publication_scope, lifecycle, authors) \
             VALUES ($1, $2, 1, repeat('a', 64), $3, 'Disclosure fixture', $4, 'published', '[\"PLE\"]'::jsonb)",
        )
        .bind(problem)
        .bind(version)
        .bind(id())
        .bind(scope)
        .execute(&pool)
        .await
        .expect("publish catalog fixture projection");
    }

    let mut broker = pool
        .begin()
        .await
        .expect("begin statistics broker transaction");
    sqlx::query("SET LOCAL ROLE ple_statistics_broker")
        .execute(&mut *broker)
        .await
        .expect("assume statistics broker role");
    for (problem, version) in [
        (public_problem, public_version),
        (private_problem, private_version),
    ] {
        sqlx::query(
            "INSERT INTO question_statistics_aggregate (problem_id, version_id, cohort_size, score_sum, attempts_sum, duration_histogram_version, duration_histogram, scored_cohort_size, score_mean, rest_score_mean, score_m2, rest_score_m2, score_rest_co_moment) \
             VALUES ($1, $2, 5, 3, 5, 1, ARRAY[5,0,0,0,0,0,0,0,0,0]::bigint[], 5, 0.6, 0.6, 0, 0, 0)",
        )
        .bind(problem)
        .bind(version)
        .execute(&mut *broker)
        .await
        .expect("statistics broker crosses first disclosure threshold");
    }
    broker.commit().await.expect("commit broker disclosure");

    let disclosure_sequence: i64 = sqlx::query_scalar(
        "SELECT disclosed_sequence FROM catalog_statistics_disclosure WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(public_problem)
    .bind(public_version)
    .fetch_one(&pool)
    .await
    .expect("first threshold crossing creates disclosure");
    sqlx::query("UPDATE question_statistics_aggregate SET cohort_size = cohort_size WHERE problem_id = $1 AND version_id = $2")
        .bind(public_problem)
        .bind(public_version)
        .execute(&pool)
        .await
        .expect("later threshold-preserving statistics update");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT disclosed_sequence FROM catalog_statistics_disclosure WHERE problem_id = $1 AND version_id = $2",
        )
        .bind(public_problem)
        .bind(public_version)
        .fetch_one(&pool)
        .await
        .expect("first disclosure is retained"),
        disclosure_sequence
    );

    let mut foreign = pool
        .begin()
        .await
        .expect("begin foreign application transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *foreign)
        .await
        .expect("assume application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(foreign_tenant.to_string())
        .execute(&mut *foreign)
        .await
        .expect("set foreign tenant context");
    let visible: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM catalog_statistics_disclosure WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(private_problem)
    .bind(private_version)
    .fetch_one(&mut *foreign)
    .await
    .expect("private disclosure query is safely filtered");
    assert_eq!(
        visible, 0,
        "foreign tenant cannot enumerate private disclosure"
    );
    foreign
        .rollback()
        .await
        .expect("rollback foreign visibility probe");

    let privileges = sqlx::query(
        "SELECT has_function_privilege('ple_app', 'public.ple_record_catalog_statistics_disclosure()', 'EXECUTE') AS app_exec, \
                has_table_privilege('ple_statistics_broker', 'public.catalog_statistics_disclosure', 'INSERT') AS broker_insert, \
                has_table_privilege('ple_statistics_broker', 'public.catalog_statistics_disclosure', 'SELECT') AS broker_select, \
                has_sequence_privilege('ple_statistics_broker', 'public.catalog_search_publication_sequence', 'USAGE') AS broker_usage, \
                has_sequence_privilege('ple_statistics_broker', 'public.catalog_search_publication_sequence', 'SELECT') AS broker_select_sequence, \
                (SELECT relforcerowsecurity FROM pg_class WHERE oid = 'public.catalog_statistics_disclosure'::regclass) AS forced_rls",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect broker privilege boundary");
    assert!(!privileges.get::<bool, _>("app_exec"));
    assert!(privileges.get::<bool, _>("broker_insert"));
    assert!(!privileges.get::<bool, _>("broker_select"));
    assert!(privileges.get::<bool, _>("broker_usage"));
    assert!(!privileges.get::<bool, _>("broker_select_sequence"));
    assert!(privileges.get::<bool, _>("forced_rls"));
}
