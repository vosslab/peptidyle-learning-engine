//! Bounded schema and disposable database owned by the issued-read oracle.

use learning_data_access::postgres::{lazy_pool, migration_status_from_directory};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::id;

// This oracle consumes learner-work preparation (1817), course creation
// (1818), response-family publication metadata (1827), the prefetch parent
// broker capability (1829), and the canonical immutable receipt aggregate
// decoded for every lifecycle state (1851). Its terminal-receipt oracle clears
// an evaluated attempt through the current support workflow, whose immutable
// scoring-invalidation binding is owned by 1865.
const ISSUED_ATTEMPT_READ_EPOCH: i64 = 2_026_081_865;

pub(super) struct DisposableDatabase {
    admin: PgPool,
    database: String,
    migration_directory: PathBuf,
    pub(super) pool: PgPool,
}

impl DisposableDatabase {
    pub(super) fn database_name(&self) -> &str {
        &self.database
    }

    pub(super) async fn provision(url: &str) -> Self {
        let admin = lazy_pool(url).expect("valid PostgreSQL administration URL");
        let database = format!("ple_t4_issued_read_{:x}", id().as_u128());
        assert!(
            database.len() < 64,
            "child database identifier fits PostgreSQL"
        );
        sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database}")))
            .execute(&admin)
            .await
            .expect("create isolated issued-read PostgreSQL database");
        let options = PgConnectOptions::from_str(url)
            .expect("PostgreSQL URL")
            .database(&database);
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await
            .expect("connect isolated issued-read PostgreSQL database");
        let migration_directory = migrations_through(ISSUED_ATTEMPT_READ_EPOCH);
        sqlx::migrate::Migrator::new(migration_directory.clone())
            .await
            .expect("bounded issued-read migration directory")
            .run(&pool)
            .await
            .expect("apply issued-read migration epoch");
        assert_issued_attempt_epoch(&pool, &migration_directory).await;
        Self {
            admin,
            database,
            migration_directory,
            pool,
        }
    }

    pub(super) async fn cleanup(self) {
        self.pool.close().await;
        sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
            .bind(&self.database)
            .execute(&self.admin)
            .await
            .expect("disconnect issued-read child database");
        sqlx::query(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {}",
            self.database
        )))
        .execute(&self.admin)
        .await
        .expect("drop isolated issued-read child database");
        fs::remove_dir_all(&self.migration_directory)
            .expect("remove isolated issued-read migration directory");
    }
}

fn migrations_through(version: i64) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/migrations");
    let target = std::env::temp_dir().join(format!("ple-issued-read-migrations-{}", id()));
    fs::create_dir_all(&target).expect("create isolated issued-read migration directory");
    for entry in fs::read_dir(source).expect("read immutable migration directory") {
        let entry = entry.expect("migration directory entry");
        let name = entry.file_name();
        let filename = name.to_string_lossy();
        let prefix = filename.split('_').next().unwrap_or_default();
        if prefix.parse::<i64>().is_ok_and(|found| found <= version) {
            fs::copy(entry.path(), target.join(name)).expect("copy bounded immutable migration");
        }
    }
    target
}

async fn assert_issued_attempt_epoch(pool: &PgPool, migrations: &Path) {
    let status = migration_status_from_directory(pool, migrations)
        .await
        .expect("read bounded migration ledger");
    assert!(
        status.is_compatible(),
        "issued-read database ledger matches its bounded migration epoch: {status:?}"
    );
    let capabilities: (bool, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT has_function_privilege('ple_app', \
                'public.ple_prepare_attempt_work(uuid,uuid,uuid,uuid,uuid,text)', 'EXECUTE'), \
                has_function_privilege('ple_app', \
                'public.ple_create_course_as_sysadmin_v1(uuid,uuid,text,date,date,text,uuid,character)', 'EXECUTE'), \
                EXISTS (SELECT 1 FROM information_schema.columns \
                         WHERE table_schema='public' AND table_name='problem_version' \
                           AND column_name='response_family'), \
                has_table_privilege('ple_learner_work_broker', \
                    'public.question_prefetch', 'SELECT'), \
                EXISTS (SELECT 1 FROM information_schema.columns \
                         WHERE table_schema='public' \
                           AND table_name='submission_receipt_snapshot' \
                           AND column_name='receipt_attempt_canonical_json'), \
                has_function_privilege('ple_app', \
                    'public.ple_bind_attempt_support_invalidation_v1(uuid,uuid,uuid)', \
                    'EXECUTE')",
    )
    .fetch_one(pool)
    .await
    .expect("read bounded issued-read capability catalog");
    assert!(capabilities.0, "1817 learner-work broker is executable");
    assert!(capabilities.1, "1818 course-creation broker is executable");
    assert!(capabilities.2, "1827 response-family metadata is available");
    assert!(capabilities.3, "1829 prefetch parent broker is available");
    assert!(
        capabilities.4,
        "1851 immutable receipt aggregate is available"
    );
    assert!(
        capabilities.5,
        "1865 support invalidation binding is available"
    );
}
