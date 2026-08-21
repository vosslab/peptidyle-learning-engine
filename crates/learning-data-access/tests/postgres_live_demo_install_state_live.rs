#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for Base Course installation coordination.

use std::fs;
use std::str::FromStr;
use std::time::Duration;

use learning_data_access::postgres::{
    BaseCourseAccountPlatformRoles, BaseCourseAccountRecipe, BaseCourseInstallState, PostgresStore,
    acquire_base_course_install_lock, apply_migrations, lazy_pool,
};
use learning_data_access::{AuthenticationEmail, LiveDemoInstallationStore};
use question_model::{TenantId, UserId, UserRole};
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, Row};
use uuid::Uuid;

const LIVE_DEMO_INSTALL_STATE_MIGRATION: i64 = 2_026_081_808;

fn tenant() -> TenantId {
    TenantId::from_uuid(Uuid::from_u128(0x6c69_7665_2d64_656d_6f2d_696e_7374_616c))
}

fn fresh() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("fixture randomness");
    Uuid::from_bytes(bytes)
}

fn migrations_through(version: i64) -> std::path::PathBuf {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/migrations");
    let target = std::env::temp_dir().join(format!("ple-ld1-migrations-{}", fresh()));
    fs::create_dir_all(&target).expect("temporary migration directory");
    for entry in fs::read_dir(source).expect("migration directory") {
        let entry = entry.expect("migration entry");
        let name = entry.file_name();
        let text = name.to_string_lossy();
        let Some(prefix) = text.split('_').next() else {
            continue;
        };
        if prefix.parse::<i64>().is_ok_and(|found| found <= version) {
            fs::copy(entry.path(), target.join(name)).expect("copy migration");
        }
    }
    target
}

async fn admin_pool(url: &str) -> sqlx::PgPool {
    let options = PgConnectOptions::from_str(url)
        .expect("disposable PostgreSQL URL")
        .database("postgres");
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .expect("admin connection")
}

#[derive(Clone, Copy, Debug)]
enum FreshnessCase {
    Fresh,
    EmailChallenge,
    RateLimit,
    UserlessWebauthnCeremony,
    ConsumedQuestionNamespace,
}

impl FreshnessCase {
    fn label(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::EmailChallenge => "email_challenge",
            Self::RateLimit => "rate_limit",
            Self::UserlessWebauthnCeremony => "userless_webauthn",
            Self::ConsumedQuestionNamespace => "consumed_namespace",
        }
    }

    fn seed_sql(self) -> Option<&'static str> {
        match self {
            Self::Fresh => None,
            Self::EmailChallenge => Some(
                "INSERT INTO public.email_authentication_challenge (\
                 challenge_id, token_hash, browser_binding_hash, normalized_email, delivery_email, \
                 purpose, purpose_user_id, expires_at, rate_limit_key_hash) VALUES (\
                 '10000000-0000-4000-8000-000000000001', decode(repeat('11',32),'hex'), \
                 decode(repeat('22',32),'hex'), 'freshness@example.edu', 'freshness@example.edu', \
                 'sign_in_or_register', NULL, transaction_timestamp() + interval '5 minutes', \
                 decode(repeat('33',32),'hex'))",
            ),
            Self::RateLimit => Some(
                "INSERT INTO public.authentication_rate_limit (\
                 limit_scope, key_hash, window_started_at, attempt_count, updated_at) VALUES (\
                 'network', decode(repeat('44',32),'hex'), transaction_timestamp(), 1, \
                 transaction_timestamp())",
            ),
            Self::UserlessWebauthnCeremony => Some(
                "INSERT INTO public.webauthn_ceremony (\
                 ceremony_id, ceremony_kind, user_id, browser_binding_hash, state, expires_at) \
                 VALUES ('10000000-0000-4000-8000-000000000002', 'authentication', NULL, \
                 decode(repeat('55',32),'hex'), '{}'::jsonb, \
                 transaction_timestamp() + interval '5 minutes')",
            ),
            Self::ConsumedQuestionNamespace => {
                Some("UPDATE public.question_id_namespace SET issued_count = 1 WHERE singleton")
            }
        }
    }

    fn expects_prepare_success(self) -> bool {
        matches!(self, Self::Fresh)
    }

    fn expected_error_fragment(self) -> Option<&'static str> {
        match self {
            Self::Fresh => None,
            Self::EmailChallenge => Some("public.email_authentication_challenge"),
            Self::RateLimit => Some("public.authentication_rate_limit"),
            Self::UserlessWebauthnCeremony => Some("public.webauthn_ceremony"),
            Self::ConsumedQuestionNamespace => Some("unconsumed question ID namespace"),
        }
    }
}

async fn run_freshness_case(admin: &sqlx::PgPool, url: &str, case: FreshnessCase) {
    let database = format!("ple_ld1_{}_{:x}", case.label(), fresh().as_u128());
    assert!(database.len() < 64);
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database}")))
        .execute(admin)
        .await
        .expect("create generated database");

    let case_database = database.clone();
    let case_url = url.to_string();
    let result = tokio::spawn(async move {
        let options = PgConnectOptions::from_str(&case_url)
            .expect("acceptance URL")
            .database(&case_database);
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .expect("freshness database connection");
        let before = migrations_through(LIVE_DEMO_INSTALL_STATE_MIGRATION - 1);
        sqlx::migrate::Migrator::new(before.clone())
            .await
            .expect("pre-LD1 migrator")
            .run(&pool)
            .await
            .expect("migrate through 1807");
        if let Some(seed_sql) = case.seed_sql() {
            sqlx::query(seed_sql)
                .execute(&pool)
                .await
                .expect("seed pre-marker freshness case");
        }

        let full = migrations_through(LIVE_DEMO_INSTALL_STATE_MIGRATION);
        sqlx::migrate::Migrator::new(full.clone())
            .await
            .expect("LD1 migrator")
            .run(&pool)
            .await
            .expect("ordinary application schema upgrades through the lifecycle migration");
        let ledger_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public._sqlx_migrations \
             WHERE success AND version = 2026081808",
        )
        .fetch_one(&pool)
        .await
        .expect("LD1 migration ledger row");
        assert_eq!(ledger_count, 1);

        let mut install = acquire_base_course_install_lock(&pool)
            .await
            .expect("installer lock after ordinary upgrade");
        let prepared = install
            .prepare(tenant(), "base-course-v1", &json!([]))
            .await;
        if case.expects_prepare_success() {
            assert!(matches!(
                prepared.expect("fresh application state accepts first Base Course prepare"),
                BaseCourseInstallState::Installing { .. }
            ));
        } else {
            let error = prepared.expect_err("populated unmarked application state refuses prepare");
            let message = error.to_string();
            assert!(
                message.contains("live-demo baseline requires"),
                "unexpected {} prepare error: {error}",
                case.label()
            );
            let expected_fragment = case
                .expected_error_fragment()
                .expect("refusal case names its rejected state");
            assert!(
                message.contains(expected_fragment),
                "{} refusal did not identify {expected_fragment}: {error}",
                case.label()
            );
            let marker_count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM public.live_demo_install_state WHERE singleton",
            )
            .fetch_one(&pool)
            .await
            .expect("rejected marker lookup");
            assert!(
                marker_count == 0,
                "rejected first prepare leaves no lifecycle marker"
            );
        }
        install.release().await.expect("installer lock releases");
        pool.close().await;
        fs::remove_dir_all(before).expect("remove pre-LD1 migrations");
        fs::remove_dir_all(full).expect("remove LD1 migrations");
    })
    .await;

    let _ = sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
        .bind(&database)
        .execute(admin)
        .await;
    sqlx::query(AssertSqlSafe(format!("DROP DATABASE IF EXISTS {database}")))
        .execute(admin)
        .await
        .expect("drop generated database");
    result.expect("freshness case task");
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL 17 database with CREATEDB"]
async fn live_demo_migration_upgrades_ordinary_data_and_prepare_requires_fresh_state() {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name a disposable PostgreSQL database");
    let admin = admin_pool(&url).await;
    for case in [
        FreshnessCase::Fresh,
        FreshnessCase::EmailChallenge,
        FreshnessCase::RateLimit,
        FreshnessCase::UserlessWebauthnCeremony,
        FreshnessCase::ConsumedQuestionNamespace,
    ] {
        run_freshness_case(&admin, &url, case).await;
    }
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL for a fresh disposable PostgreSQL database"]
async fn base_course_install_state_serializes_and_resumes_only_identical_inputs() {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name a fresh disposable database");
    let pool = lazy_pool(&url).expect("PostgreSQL URL");
    apply_migrations(&pool)
        .await
        .expect("embedded migrations apply to the disposable database");
    let capability = sqlx::query(
        "SELECT procedure.prosecdef, owner_role.rolname, \
                has_function_privilege(\
                    'ple_auth', procedure.oid, 'EXECUTE'\
                ) AS auth_execute, \
                has_table_privilege(\
                    'ple_auth', 'public.live_demo_install_state', 'SELECT'\
                ) AS auth_table_select \
         FROM pg_proc AS procedure \
         JOIN pg_roles AS owner_role ON owner_role.oid = procedure.proowner \
         WHERE procedure.proname = 'ple_completed_live_demo_installation_generation'",
    )
    .fetch_one(&pool)
    .await
    .expect("completed-generation broker catalog");
    assert!(
        capability.try_get::<bool, _>("prosecdef").expect("definer"),
        "the completed-generation read is brokered"
    );
    assert_eq!(
        capability
            .try_get::<String, _>("rolname")
            .expect("broker owner"),
        "ple_live_demo_installation_broker"
    );
    assert!(
        capability
            .try_get::<bool, _>("auth_execute")
            .expect("auth broker execute"),
        "the auth path can read only through the broker"
    );
    assert!(
        !capability
            .try_get::<bool, _>("auth_table_select")
            .expect("auth direct table select"),
        "the auth path has no direct lifecycle-table read"
    );
    sqlx::query("DELETE FROM public.live_demo_install_state")
        .execute(&pool)
        .await
        .expect("disposable oracle clears only its lifecycle marker");

    let mut first = acquire_base_course_install_lock(&pool)
        .await
        .expect("first host installer holds the lock");
    let state = first
        .prepare(tenant(), "base-course-v1", &json!([]))
        .await
        .expect("fresh install begins");
    assert!(matches!(state, BaseCourseInstallState::Installing { .. }));
    assert_eq!(
        first
            .prepare(
                TenantId::from_uuid(Uuid::from_u128(2)),
                "base-course-v1",
                &json!([]),
            )
            .await,
        Err(learning_data_access::StoreError::Conflict)
    );

    let competing_pool = pool.clone();
    let competing =
        tokio::spawn(async move { acquire_base_course_install_lock(&competing_pool).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !competing.is_finished(),
        "the second caller waits on the PostgreSQL session advisory lock"
    );

    first
        .abort()
        .await
        .expect("failed installer closes its locked session");

    let mut second = tokio::time::timeout(Duration::from_secs(5), competing)
        .await
        .expect("waiting installer resumes after the first session closes")
        .expect("waiting installer task")
        .expect("closed advisory-lock session is available to a resumer");
    assert!(matches!(
        second
            .prepare(tenant(), "base-course-v1", &json!([]))
            .await
            .expect("closed installer leaves an exact marker to resume"),
        BaseCourseInstallState::Installing { .. }
    ));
    let installation_generation = match second.read_state().await.expect("state reads") {
        Some(BaseCourseInstallState::Installing {
            installation_generation,
            ..
        }) => installation_generation,
        state => panic!("expected installing state, got {state:?}"),
    };
    second
        .mark_complete(
            tenant(),
            "base-course-v1",
            installation_generation,
            &json!([]),
            &"a".repeat(64),
        )
        .await
        .expect("matching installing marker completes");
    second.release().await.expect("second session unlocks");
    let application_store = PostgresStore::new(pool.clone());
    assert_eq!(
        application_store
            .completed_live_demo_installation_generation()
            .await
            .expect("completed generation broker read"),
        Some(installation_generation),
        "the application reads only the durable completed generation"
    );
    sqlx::query(
        "INSERT INTO public.authentication_rate_limit \
         (limit_scope, key_hash, window_started_at, attempt_count, updated_at) VALUES (\
         'network', decode(repeat('44',32),'hex'), transaction_timestamp(), 1, \
         transaction_timestamp())",
    )
    .execute(&pool)
    .await
    .expect("ordinary retained state after complete installation");
    let mut retained = acquire_base_course_install_lock(&pool)
        .await
        .expect("retained installer holds the lock");
    assert!(matches!(
        retained
            .prepare(tenant(), "base-course-v1", &json!([]))
            .await
            .expect("complete marker bypasses first-install freshness scan"),
        BaseCourseInstallState::Complete { .. }
    ));
    retained
        .release()
        .await
        .expect("retained installer unlocks");
    let retained_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.authentication_rate_limit \
         WHERE key_hash = decode(repeat('44',32),'hex')",
    )
    .fetch_one(&pool)
    .await
    .expect("retained ordinary state lookup");
    assert_eq!(retained_rows, 1);
    sqlx::query("DELETE FROM public.live_demo_install_state")
        .execute(&pool)
        .await
        .expect("disposable oracle cleanup");
    assert_eq!(
        application_store
            .completed_live_demo_installation_generation()
            .await
            .expect("missing generation broker read"),
        None,
        "an absent lifecycle marker has no claimable generation"
    );
    sqlx::query(
        "INSERT INTO public.live_demo_install_state \
         (singleton, state, baseline_version, tenant_id, installation_generation, object_manifest) \
         VALUES (true, 'installing', 'base-course-v1', $1, $2, '[]'::jsonb)",
    )
    .bind(tenant().as_uuid())
    .bind(Uuid::from_u128(9))
    .execute(&pool)
    .await
    .expect("disposable installing marker");
    assert_eq!(
        application_store
            .completed_live_demo_installation_generation()
            .await
            .expect("installing generation broker read"),
        None,
        "an installing lifecycle marker cannot authorize ownership"
    );
    sqlx::query("DELETE FROM public.live_demo_install_state")
        .execute(&pool)
        .await
        .expect("disposable installing-marker cleanup");
    sqlx::query("DELETE FROM public.authentication_rate_limit")
        .execute(&pool)
        .await
        .expect("disposable ordinary-state cleanup");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL for a fresh disposable PostgreSQL database"]
async fn base_course_installer_atomically_converges_exact_accounts() {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name a fresh disposable database");
    let pool = lazy_pool(&url).expect("PostgreSQL URL");
    apply_migrations(&pool)
        .await
        .expect("embedded migrations apply to the disposable database");

    let ordinary_user = UserId::from_uuid(fresh());
    let sysadmin_user = UserId::from_uuid(fresh());
    let ordinary_email = AuthenticationEmail::parse(&format!(
        "ordinary-{}@example.invalid",
        ordinary_user.as_uuid()
    ))
    .expect("ordinary provisioning email");
    let sysadmin_email = AuthenticationEmail::parse(&format!(
        "sysadmin-{}@example.invalid",
        sysadmin_user.as_uuid()
    ))
    .expect("Sysadmin provisioning email");
    let ordinary = BaseCourseAccountRecipe::new(
        ordinary_user,
        ordinary_email,
        "Ordinary Baseline Account",
        BaseCourseAccountPlatformRoles::None,
    )
    .expect("ordinary account recipe");
    let sysadmin = BaseCourseAccountRecipe::new(
        sysadmin_user,
        sysadmin_email,
        "Seeded Sysadmin Account",
        BaseCourseAccountPlatformRoles::Sysadmin,
    )
    .expect("Sysadmin account recipe");

    let mut install = acquire_base_course_install_lock(&pool)
        .await
        .expect("host installer holds the lock");
    install
        .provision_accounts(&[ordinary.clone(), sysadmin.clone()])
        .await
        .expect("fresh account recipes provision atomically");
    install
        .provision_accounts(&[ordinary.clone(), sysadmin.clone()])
        .await
        .expect("exact account recipes converge on replay");

    let ordinary_row: (String, String, String, sqlx::types::Json<Vec<UserRole>>) = sqlx::query_as(
        "SELECT normalized_email, delivery_email, display_name, platform_roles \
             FROM public.ple_account WHERE user_id = $1",
    )
    .bind(ordinary_user.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("ordinary account row");
    assert_eq!(
        ordinary_row,
        (
            ordinary.email().normalized().to_string(),
            ordinary.email().delivery().to_string(),
            ordinary.display_name().to_string(),
            sqlx::types::Json(Vec::new()),
        )
    );
    let sysadmin_roles: sqlx::types::Json<Vec<UserRole>> =
        sqlx::query_scalar("SELECT platform_roles FROM public.ple_account WHERE user_id = $1")
            .bind(sysadmin_user.as_uuid())
            .fetch_one(&pool)
            .await
            .expect("Sysadmin platform roles");
    assert_eq!(sysadmin_roles.0, vec![UserRole::Sysadmin]);

    let mut auth_attempt = pool.begin().await.expect("auth privilege transaction");
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut *auth_attempt)
        .await
        .expect("assume the application authentication role");
    let grant_result = sqlx::query(
        "UPDATE public.ple_account SET platform_roles = '[\"sysadmin\"]'::jsonb \
         WHERE user_id = $1",
    )
    .bind(ordinary_user.as_uuid())
    .execute(&mut *auth_attempt)
    .await;
    assert!(
        grant_result.is_err(),
        "ple_auth must remain unable to grant the Sysadmin platform role"
    );
    auth_attempt
        .rollback()
        .await
        .expect("discard the denied privilege attempt");

    sqlx::query(
        "UPDATE public.ple_account SET display_name = 'Drifted Account' WHERE user_id = $1",
    )
    .bind(ordinary_user.as_uuid())
    .execute(&pool)
    .await
    .expect("seed display-name drift");
    let rollback_user = UserId::from_uuid(fresh());
    let rollback_recipe = BaseCourseAccountRecipe::new(
        rollback_user,
        AuthenticationEmail::parse(&format!(
            "rollback-{}@example.invalid",
            rollback_user.as_uuid()
        ))
        .expect("rollback provisioning email"),
        "Must Roll Back",
        BaseCourseAccountPlatformRoles::None,
    )
    .expect("rollback account recipe");
    assert_eq!(
        install
            .provision_accounts(&[rollback_recipe, ordinary.clone()])
            .await,
        Err(learning_data_access::StoreError::Conflict)
    );
    install
        .read_state()
        .await
        .expect("the locked session completes its failed transaction rollback");
    let rolled_back: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.ple_account WHERE user_id = $1")
            .bind(rollback_user.as_uuid())
            .fetch_one(&pool)
            .await
            .expect("rolled-back account lookup");
    assert_eq!(rolled_back, 0);

    sqlx::query("UPDATE public.ple_account SET display_name = $2 WHERE user_id = $1")
        .bind(ordinary_user.as_uuid())
        .bind(ordinary.display_name())
        .execute(&pool)
        .await
        .expect("restore the exact display name");
    sqlx::query(
        "UPDATE public.ple_account SET normalized_email = 'drifted@example.invalid' \
         WHERE user_id = $1",
    )
    .bind(ordinary_user.as_uuid())
    .execute(&pool)
    .await
    .expect("seed normalized-email drift");
    assert_eq!(
        install
            .provision_accounts(std::slice::from_ref(&ordinary))
            .await,
        Err(learning_data_access::StoreError::Conflict)
    );
    install
        .read_state()
        .await
        .expect("normalized-email refusal rolls back");

    sqlx::query(
        "UPDATE public.ple_account SET normalized_email = $2, delivery_email = 'Drift@Example.invalid' \
         WHERE user_id = $1",
    )
    .bind(ordinary_user.as_uuid())
    .bind(ordinary.email().normalized())
    .execute(&pool)
    .await
    .expect("restore normalized email and seed delivery-email drift");
    assert_eq!(
        install
            .provision_accounts(std::slice::from_ref(&ordinary))
            .await,
        Err(learning_data_access::StoreError::Conflict)
    );
    install
        .read_state()
        .await
        .expect("delivery-email refusal rolls back");

    sqlx::query(
        "UPDATE public.ple_account SET delivery_email = $2, \
         platform_roles = '[\"sysadmin\"]'::jsonb WHERE user_id = $1",
    )
    .bind(ordinary_user.as_uuid())
    .bind(ordinary.email().delivery())
    .execute(&pool)
    .await
    .expect("restore delivery email and seed platform-role drift");
    assert_eq!(
        install
            .provision_accounts(std::slice::from_ref(&ordinary))
            .await,
        Err(learning_data_access::StoreError::Conflict)
    );
    install
        .read_state()
        .await
        .expect("platform-role refusal rolls back");

    sqlx::query("DELETE FROM public.ple_account WHERE user_id = ANY($1)")
        .bind(vec![ordinary_user.as_uuid(), sysadmin_user.as_uuid()])
        .execute(&pool)
        .await
        .expect("disposable account cleanup");
    install.release().await.expect("installer session unlocks");
}
