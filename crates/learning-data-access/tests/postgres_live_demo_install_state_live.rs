#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for the canonical Base Course installation product.

use std::fs;
use std::str::FromStr;

use base_course_installation::{
    BaseCourseInstallError, BaseCourseInstallPhase, BaseCourseInstallRequest,
    BaseCourseParticipants,
};
use learning_data_access::StoreError;
use learning_data_access::postgres::{
    BaseCourseInstallerPool, PostgresStore, ProductionLoginProfile, apply_migrations,
    local_base_course_application_pool, local_base_course_installer_pool, local_development_pool,
};
use question_model::{TenantId, UserId};
use sqlx::AssertSqlSafe;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

#[path = "postgres_live_demo_install_state_live/account_convergence.rs"]
mod account_convergence;
#[path = "postgres_live_demo_install_state_live/completion_evidence.rs"]
mod completion_evidence;
#[path = "postgres_live_demo_install_state_live/product_lifecycle.rs"]
mod product_lifecycle;

const LIVE_DEMO_INSTALL_STATE_MIGRATION: i64 = 2_026_081_808;
const INSTALLER_LOGIN: &str = "ple_base_course_installer_login";
const INSTALLER_PASSWORD: &str = "install-state-installer-fixture";
const APPLICATION_LOGIN: &str = "ple_base_course_app_login";
const APPLICATION_PASSWORD: &str = "install-state-application-fixture";
const API_LOGIN: &str = "ple_api_login";
const API_PASSWORD: &str = "install-state-api-fixture";

#[derive(Clone, Copy)]
struct Participants {
    tenant: TenantId,
    instructor: UserId,
    mary: UserId,
    jack: UserId,
    avery: UserId,
    morgan: UserId,
}

impl Participants {
    fn fresh() -> Self {
        Self {
            tenant: TenantId::from_uuid(fresh()),
            instructor: UserId::from_uuid(fresh()),
            mary: UserId::from_uuid(fresh()),
            jack: UserId::from_uuid(fresh()),
            avery: UserId::from_uuid(fresh()),
            morgan: UserId::from_uuid(fresh()),
        }
    }

    fn product(self) -> BaseCourseParticipants {
        BaseCourseParticipants::try_new(
            self.tenant,
            self.instructor,
            self.mary,
            self.jack,
            self.avery,
            self.morgan,
        )
        .expect("five distinct Base Course participants")
    }

    fn user_ids(self) -> [Uuid; 5] {
        [
            self.instructor.as_uuid(),
            self.mary.as_uuid(),
            self.jack.as_uuid(),
            self.avery.as_uuid(),
            self.morgan.as_uuid(),
        ]
    }
}

struct ProductDatabase {
    installer_url: String,
    application_url: String,
    api_url: String,
}

impl ProductDatabase {
    async fn provision(admin: &sqlx::PgPool, admin_url: &str, database: &str) -> Self {
        // The fixed disposable logins reproduce the two child-only production authorities.
        sqlx::raw_sql(
            "DO $$ BEGIN \
             IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname='ple_base_course_installer_login') \
             THEN CREATE ROLE ple_base_course_installer_login LOGIN; END IF; \
             IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname='ple_base_course_app_login') \
             THEN CREATE ROLE ple_base_course_app_login LOGIN; END IF; \
             IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname='ple_api_login') \
             THEN CREATE ROLE ple_api_login LOGIN; END IF; END $$; \
             ALTER ROLE ple_base_course_installer_login LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE \
             NOINHERIT NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 1 \
             PASSWORD 'install-state-installer-fixture'; \
             ALTER ROLE ple_base_course_app_login LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE \
             NOINHERIT NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 1 \
             PASSWORD 'install-state-application-fixture'; \
             ALTER ROLE ple_api_login LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE \
             NOINHERIT NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 1 \
             PASSWORD 'install-state-api-fixture'; \
             REVOKE ple_app FROM ple_base_course_installer_login; \
             REVOKE ple_base_course_installer FROM ple_base_course_app_login; \
             GRANT ple_base_course_installer TO ple_base_course_installer_login \
             WITH INHERIT FALSE, SET TRUE, ADMIN FALSE; \
             GRANT ple_app TO ple_base_course_app_login \
             WITH INHERIT FALSE, SET TRUE, ADMIN FALSE; \
             GRANT ple_app TO ple_api_login \
             WITH INHERIT FALSE, SET TRUE, ADMIN FALSE; \
             GRANT ple_auth TO ple_api_login \
             WITH INHERIT FALSE, SET TRUE, ADMIN FALSE",
        )
        .execute(admin)
        .await
        .expect("provision exact disposable Base Course logins");
        Self {
            installer_url: login_url(admin_url, database, INSTALLER_LOGIN, INSTALLER_PASSWORD),
            application_url: login_url(
                admin_url,
                database,
                APPLICATION_LOGIN,
                APPLICATION_PASSWORD,
            ),
            api_url: login_url(admin_url, database, API_LOGIN, API_PASSWORD),
        }
    }

    fn installer_pool(&self) -> BaseCourseInstallerPool {
        local_base_course_installer_pool(&self.installer_url)
            .expect("attested local Base Course installer pool")
    }

    fn store(&self) -> PostgresStore {
        let pool = local_base_course_application_pool(&self.application_url)
            .expect("attested local Base Course application pool");
        PostgresStore::with_question_id_secret(pool, [0x42; 32])
    }

    fn api_store(&self) -> PostgresStore {
        let pool = local_development_pool(&self.api_url, ProductionLoginProfile::Api)
            .expect("attested local API pool");
        PostgresStore::with_question_id_secret(pool, [0x42; 32])
    }
}

async fn reset_disposable_course_capability_memberships(admin: &sqlx::PgPool) {
    // Migrations run before disposable child-login provisioning, as they do in production.
    sqlx::raw_sql(
        "DO $$ DECLARE membership record; BEGIN \
         FOR membership IN \
             SELECT parent.rolname AS parent_name, member.rolname AS member_name \
             FROM pg_auth_members AS grant_map \
             JOIN pg_roles AS parent ON parent.oid=grant_map.roleid \
             JOIN pg_roles AS member ON member.oid=grant_map.member \
             WHERE member.rolname IN (\
                 'ple_base_course_installer_login','ple_base_course_app_login','ple_api_login',\
                 'ple_course_creation_broker','ple_base_course_installer',\
                 'ple_base_course_install_broker','ple_base_course_freshness_broker',\
                 'ple_base_course_completion_verification_broker',\
                 'ple_course_roster_mutator_broker'\
             ) OR parent.rolname IN (\
                 'ple_base_course_installer_login','ple_base_course_app_login','ple_api_login',\
                 'ple_course_creation_broker','ple_base_course_installer',\
                 'ple_base_course_install_broker','ple_base_course_freshness_broker',\
                 'ple_base_course_completion_verification_broker',\
                 'ple_course_roster_mutator_broker'\
             ) \
         LOOP EXECUTE format(\
             'REVOKE %I FROM %I', membership.parent_name, membership.member_name\
         ); END LOOP; END $$",
    )
    .execute(admin)
    .await
    .expect("reset disposable Base Course login memberships before migrations");
}

fn login_url(admin_url: &str, database: &str, login: &str, password: &str) -> String {
    let options = PgConnectOptions::from_str(admin_url).expect("disposable PostgreSQL URL");
    let host = options.get_host();
    assert!(
        host.is_ascii() && !host.chars().any(char::is_whitespace),
        "the disposable live oracle requires a TCP host"
    );
    assert!(
        database
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "the generated database name is URL-safe"
    );
    let rendered_host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    format!(
        "postgres://{login}:{password}@{rendered_host}:{}/{database}",
        options.get_port()
    )
}

fn prepare_request(participants: Participants) -> BaseCourseInstallRequest {
    BaseCourseInstallRequest::new(participants.product(), BaseCourseInstallPhase::Prepare)
}

fn install_request(
    participants: Participants,
    storage_receipt_json: &str,
) -> BaseCourseInstallRequest {
    BaseCourseInstallRequest::new(
        participants.product(),
        BaseCourseInstallPhase::Install {
            storage_receipt_json: storage_receipt_json.to_string(),
        },
    )
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FreshnessCase {
    Fresh,
    EmailChallenge,
    RateLimit,
    UserlessWebauthnCeremony,
    ConsumedQuestionNamespace,
    ProblemCollection,
    OrphanRecipe,
}

impl FreshnessCase {
    fn label(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::EmailChallenge => "email_challenge",
            Self::RateLimit => "rate_limit",
            Self::UserlessWebauthnCeremony => "userless_webauthn",
            Self::ConsumedQuestionNamespace => "consumed_namespace",
            Self::ProblemCollection => "problem_collection",
            Self::OrphanRecipe => "orphan_recipe",
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
            Self::ProblemCollection => Some(
                "INSERT INTO public.problem_collection (collection_id,owner_tenant_id,owner_user_id,title,visibility) VALUES ('10000000-0000-4000-8000-000000000003','10000000-0000-4000-8000-000000000004','10000000-0000-4000-8000-000000000005','Freshness catalog oracle','private')",
            ),
            Self::OrphanRecipe => None,
        }
    }

    fn expected_error_fragment(self) -> Option<&'static str> {
        match self {
            Self::Fresh => None,
            Self::EmailChallenge => Some("public.email_authentication_challenge"),
            Self::RateLimit => Some("public.authentication_rate_limit"),
            Self::UserlessWebauthnCeremony => Some("public.webauthn_ceremony"),
            Self::ConsumedQuestionNamespace => Some("unconsumed question ID namespace"),
            Self::ProblemCollection => Some("public.problem_collection"),
            Self::OrphanRecipe => Some("public.live_demo_install_recipe"),
        }
    }

    async fn seed_after_migrations(self, pool: &sqlx::PgPool) {
        if self == Self::OrphanRecipe {
            sqlx::query(
                "INSERT INTO public.live_demo_install_recipe \
                 (singleton,installation_generation,tenant_id,baseline_version,recipe,recipe_sha256) \
                 VALUES (true,'10000000-0000-4000-8000-000000000006', \
                 '10000000-0000-4000-8000-000000000007','base-course-v1','{}'::jsonb,repeat('a',64))",
            )
            .execute(pool)
            .await
            .expect("seed orphan Base Course recipe");
        }
    }
}

async fn run_freshness_case(admin: &sqlx::PgPool, url: &str, case: FreshnessCase) {
    reset_disposable_course_capability_memberships(admin).await;
    let database = format!("ple_ld1_{}_{:x}", case.label(), fresh().as_u128());
    assert!(database.len() < 64);
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database}")))
        .execute(admin)
        .await
        .expect("create generated database");
    let options = PgConnectOptions::from_str(url)
        .expect("acceptance URL")
        .database(&database);
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
    apply_migrations(&pool)
        .await
        .expect("ordinary application schema upgrades through the current product migration");
    case.seed_after_migrations(&pool).await;
    let ledger_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public._sqlx_migrations \
         WHERE success AND version = 2026081808",
    )
    .fetch_one(&pool)
    .await
    .expect("LD1 migration ledger row");
    assert_eq!(ledger_count, 1);

    let product = ProductDatabase::provision(admin, url, &database).await;
    let installer = product.installer_pool();
    let store = product.store();
    let prepared = base_course_installation::install(
        &installer,
        &store,
        prepare_request(Participants::fresh()),
    )
    .await;
    if let Some(expected_fragment) = case.expected_error_fragment() {
        let error = prepared.expect_err("populated unmarked application state refuses prepare");
        assert!(matches!(
            &error,
            BaseCourseInstallError::Persistence {
                source: StoreError::InvalidRecord(_),
                ..
            }
        ));
        let message = error.to_string();
        assert!(message.contains("live-demo baseline requires"), "{error}");
        assert!(message.contains(expected_fragment), "{error}");
        let marker_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public.live_demo_install_state WHERE singleton",
        )
        .fetch_one(&pool)
        .await
        .expect("rejected marker lookup");
        assert_eq!(marker_count, 0, "rejected prepare leaves no marker");
        let recipe_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM public.live_demo_install_recipe")
                .fetch_one(&pool)
                .await
                .expect("rejected recipe lookup");
        assert_eq!(
            recipe_count,
            if case == FreshnessCase::OrphanRecipe {
                1
            } else {
                0
            },
            "rejected prepare creates no recipe"
        );
    } else {
        let output = prepared.expect("fresh application state accepts Base Course prepare");
        assert_eq!(
            output.install_state(),
            base_course_installation::BaseCourseInstallStateOutput::Installing
        );
        let evidence: (i64, i64) = sqlx::query_as(
            "SELECT count(*),count(*) FILTER(WHERE state.installation_generation=recipe.installation_generation \
             AND state.tenant_id=recipe.tenant_id AND state.baseline_version=recipe.baseline_version \
             AND recipe.recipe_sha256 ~ '^[0-9a-f]{64}$') FROM public.live_demo_install_state AS state \
             JOIN public.live_demo_install_recipe AS recipe ON recipe.singleton=state.singleton",
        )
        .fetch_one(&pool)
        .await
        .expect("fresh prepare marker and recipe evidence");
        assert_eq!(evidence, (1, 1));
    }

    drop(store);
    drop(installer);
    pool.close().await;
    fs::remove_dir_all(before).expect("remove pre-LD1 migrations");
    let _ = sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
        .bind(&database)
        .execute(admin)
        .await;
    sqlx::query(AssertSqlSafe(format!("DROP DATABASE IF EXISTS {database}")))
        .execute(admin)
        .await
        .expect("drop generated database");
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL 17 database with CREATEDB"]
async fn live_demo_migration_upgrades_ordinary_data_and_prepare_requires_fresh_state() {
    let runtime = load_acceptance_runtime();
    let url = runtime.admin_url().expose();
    let admin = admin_pool(url).await;
    for case in [
        FreshnessCase::Fresh,
        FreshnessCase::EmailChallenge,
        FreshnessCase::RateLimit,
        FreshnessCase::UserlessWebauthnCeremony,
        FreshnessCase::ConsumedQuestionNamespace,
        FreshnessCase::ProblemCollection,
        FreshnessCase::OrphanRecipe,
    ] {
        run_freshness_case(&admin, url, case).await;
    }
}
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
