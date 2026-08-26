//! Fresh PostgreSQL 17 runtime evidence for Base Course completion receipts.

use super::load_acceptance_runtime;
use std::str::FromStr;
use std::time::Duration;

use base_course_installation::{
    BaseCourseAction, BaseCourseInstallError, BaseCourseInstallStateOutput,
};
use learning_data_access::postgres::apply_migrations;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool, Postgres};

use super::{
    Participants, ProductDatabase, fresh, install_request, prepare_request,
    reset_disposable_course_capability_memberships,
};

const BARRIER_CLASS_ID: i32 = 720_418;
const BARRIER_OBJECT_ID: i32 = 1_818;
const STORAGE_RECEIPT_SHA256: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
// PostgreSQL roles are cluster-global. This gate serializes only their disposable provisioning;
// each case still migrates and exercises a different child database.
static COMPLETION_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct CompletionFixture {
    admin: PgPool,
    database: String,
    pool: PgPool,
    product: ProductDatabase,
    participants: Participants,
    generation: uuid::Uuid,
    storage_receipt_json: String,
}

impl CompletionFixture {
    async fn installing_with_full_product_graph() -> Self {
        let runtime = load_acceptance_runtime();
        let url = runtime.admin_url().expose();
        let admin = super::admin_pool(url).await;
        reset_disposable_course_capability_memberships(&admin).await;
        let database = format!("ple_t4_completion_{:x}", fresh().as_u128());
        assert!(
            database.len() < 64,
            "generated child database name is valid"
        );
        sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database}")))
            .execute(&admin)
            .await
            .expect("create isolated completion-evidence child database");
        let options = PgConnectOptions::from_str(url)
            .expect("PostgreSQL test URL")
            .database(&database);
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await
            .expect("connect isolated completion-evidence child database");
        apply_migrations(&pool)
            .await
            .expect("apply full schema to isolated completion-evidence child database");
        let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int4")
            .fetch_one(&pool)
            .await
            .expect("PostgreSQL version");
        assert!(
            (170_000..180_000).contains(&version),
            "completion evidence requires PostgreSQL 17, found {version}"
        );
        let product = ProductDatabase::provision(&admin, url, &database).await;
        let participants = Participants::fresh();
        let installer = product.installer_pool();
        let store = product.store();
        let prepared =
            base_course_installation::install(&installer, &store, prepare_request(participants))
                .await
                .expect("prepare an installing Base Course generation");
        assert_eq!(prepared.action(), BaseCourseAction::Prepared);
        install_receipt_failure_trigger(&pool).await;
        let failed_completion = base_course_installation::install(
            &installer,
            &store,
            install_request(participants, prepared.storage_receipt_json()),
        )
        .await
        .expect_err("test-owned receipt trigger prevents only terminal completion");
        assert!(
            matches!(
                failed_completion,
                BaseCourseInstallError::Persistence { .. }
            ),
            "the injected receipt failure remains an ordinary persistence failure: {failed_completion}"
        );
        drop_receipt_trigger(&pool).await;
        drop(store);
        drop(installer);

        let state: (String, i64, i64, i64) = sqlx::query_as(
            "SELECT state, \
             (SELECT count(*) FROM public.course), \
             (SELECT count(*) FROM public.question_attempt), \
             (SELECT count(*) FROM public.live_demo_install_completion_receipt) \
             FROM public.live_demo_install_state WHERE singleton",
        )
        .fetch_one(&pool)
        .await
        .expect("full graph remains observable to the child-database owner");
        assert_eq!(state.0, "installing");
        assert_eq!(state.1, 2, "the full product created both courses");
        assert_eq!(state.2, 2, "the full product created both learner attempts");
        assert_eq!(state.3, 0, "injected receipt failure is atomic");

        Self {
            admin,
            database,
            pool,
            product,
            participants,
            generation: prepared.installation_generation(),
            storage_receipt_json: prepared.storage_receipt_json().to_string(),
        }
    }

    async fn cleanup(self) {
        self.pool.close().await;
        let _ =
            sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
                .bind(&self.database)
                .execute(&self.admin)
                .await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {}",
            self.database
        )))
        .execute(&self.admin)
        .await
        .expect("drop isolated completion-evidence child database");
    }
}

async fn install_receipt_failure_trigger(pool: &PgPool) {
    sqlx::raw_sql(
        "CREATE FUNCTION public.ple_t4_completion_receipt_fail() RETURNS trigger \
         LANGUAGE plpgsql AS $$ BEGIN \
         RAISE EXCEPTION 'test-owned Base Course completion receipt failure'; END $$; \
         CREATE TRIGGER ple_t4_completion_receipt_fail \
         BEFORE INSERT ON public.live_demo_install_completion_receipt \
         FOR EACH ROW EXECUTE FUNCTION public.ple_t4_completion_receipt_fail()",
    )
    .execute(pool)
    .await
    .expect("install test-owned completion receipt failure trigger");
}

async fn drop_receipt_trigger(pool: &PgPool) {
    sqlx::raw_sql(
        "DROP TRIGGER IF EXISTS ple_t4_completion_receipt_fail \
         ON public.live_demo_install_completion_receipt; \
         DROP FUNCTION IF EXISTS public.ple_t4_completion_receipt_fail()",
    )
    .execute(pool)
    .await
    .expect("remove test-owned completion receipt failure trigger");
}

async fn install_barrier_trigger(pool: &PgPool) {
    sqlx::raw_sql(
        "CREATE SEQUENCE public.ple_t4_completion_barrier_sequence; \
         CREATE FUNCTION public.ple_t4_completion_receipt_barrier() RETURNS trigger \
         LANGUAGE plpgsql SECURITY DEFINER \
         SET search_path TO 'pg_catalog','public',pg_temp AS $$ BEGIN \
         PERFORM nextval('public.ple_t4_completion_barrier_sequence'); \
         PERFORM pg_advisory_lock(720418,1818); \
         PERFORM pg_advisory_unlock(720418,1818); \
         RETURN NEW; END $$; \
         REVOKE ALL ON FUNCTION public.ple_t4_completion_receipt_barrier() FROM PUBLIC; \
         GRANT EXECUTE ON FUNCTION public.ple_t4_completion_receipt_barrier() \
         TO ple_base_course_install_broker; \
         CREATE TRIGGER ple_t4_completion_receipt_barrier \
         BEFORE INSERT ON public.live_demo_install_completion_receipt \
         FOR EACH ROW EXECUTE FUNCTION public.ple_t4_completion_receipt_barrier()",
    )
    .execute(pool)
    .await
    .expect("install test-owned completion receipt barrier");
    let catalog: (bool, Vec<String>, bool, bool) = sqlx::query_as(
        "SELECT procedure.prosecdef, procedure.proconfig, \
         has_function_privilege('ple_base_course_install_broker',procedure.oid,'EXECUTE'), \
         has_function_privilege('ple_app',procedure.oid,'EXECUTE') \
         FROM pg_proc AS procedure \
         WHERE procedure.oid='public.ple_t4_completion_receipt_barrier()'::regprocedure",
    )
    .fetch_one(pool)
    .await
    .expect("test-owned barrier function capability catalog");
    assert!(
        catalog.0,
        "barrier sequence uses only its sealed test helper"
    );
    assert_eq!(
        catalog.1,
        vec!["search_path=pg_catalog, public, pg_temp"],
        "barrier helper uses a fixed trusted search path"
    );
    assert!(
        catalog.2,
        "receipt insert broker invokes the barrier helper"
    );
    assert!(
        !catalog.3,
        "application role cannot invoke the barrier helper"
    );
}

async fn wait_for_barrier(pool: &PgPool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_locks \
                 WHERE locktype='advisory' AND classid=$1 AND objid=$2 AND NOT granted)",
            )
            .bind(BARRIER_CLASS_ID)
            .bind(BARRIER_OBJECT_ID)
            .fetch_one(pool)
            .await
            .expect("inspect bounded completion barrier state");
            if waiting {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("completion must reach the receipt barrier after verifier reads");
}

async fn release_barrier(connection: &mut sqlx::pool::PoolConnection<Postgres>) {
    sqlx::query("SELECT pg_advisory_unlock($1,$2)")
        .bind(BARRIER_CLASS_ID)
        .bind(BARRIER_OBJECT_ID)
        .execute(&mut **connection)
        .await
        .expect("release test-owned completion receipt barrier");
}

async fn run_serializable_cycle(
    fixture: &CompletionFixture,
    replacement_theme: &str,
) -> Result<base_course_installation::BaseCourseInstallOutput, BaseCourseInstallError> {
    install_barrier_trigger(&fixture.pool).await;
    let mut barrier_holder = fixture
        .pool
        .acquire()
        .await
        .expect("acquire receipt barrier holder connection");
    sqlx::query("SELECT pg_advisory_lock($1,$2)")
        .bind(BARRIER_CLASS_ID)
        .bind(BARRIER_OBJECT_ID)
        .execute(&mut *barrier_holder)
        .await
        .expect("hold test-owned completion receipt barrier");

    let installer = fixture.product.installer_pool();
    let store = fixture.product.store();
    let participants = fixture.participants;
    let receipt = fixture.storage_receipt_json.clone();
    let completion = tokio::spawn(async move {
        base_course_installation::install(
            &installer,
            &store,
            install_request(participants, &receipt),
        )
        .await
    });
    wait_for_barrier(&fixture.pool).await;

    let mut contender = fixture
        .pool
        .begin()
        .await
        .expect("begin serializable contender transaction");
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *contender)
        .await
        .expect("serializable contender isolation");
    let state: String =
        sqlx::query_scalar("SELECT state FROM public.live_demo_install_state WHERE singleton")
            .fetch_one(&mut *contender)
            .await
            .expect("contender reads installing marker before edit");
    assert_eq!(state, "installing");
    sqlx::query("UPDATE public.course_appearance SET theme_id=$1")
        .bind(replacement_theme)
        .execute(&mut *contender)
        .await
        .expect("contender changes a completion-verifier-read row");
    contender
        .commit()
        .await
        .expect("contender commits the rw-dependency edge");
    release_barrier(&mut barrier_holder).await;
    let result = completion.await.expect("completion task join");
    drop(barrier_holder);
    result
}

#[derive(Clone, Copy)]
enum DirectRole {
    Application,
    Installer,
}

impl DirectRole {
    fn set_statement(self) -> &'static str {
        match self {
            Self::Application => "SET LOCAL ROLE ple_app",
            Self::Installer => "SET LOCAL ROLE ple_base_course_installer",
        }
    }
}

fn assert_permission_denied<T>(result: Result<T, sqlx::Error>, label: &str) {
    let error = match result {
        Ok(_) => panic!("{label} unexpectedly succeeded"),
        Err(error) => error,
    };
    assert_eq!(
        error
            .as_database_error()
            .and_then(|database| database.code()),
        Some("42501".into()),
        "{label} is denied without yielding protected rows: {error}"
    );
}

async fn assert_direct_role_denials(pool: &PgPool, role: Option<DirectRole>, login_name: &str) {
    for (label, statement) in [
        (
            "completion receipt read",
            "SELECT 1 FROM public.live_demo_install_completion_receipt",
        ),
        (
            "completion receipt write",
            "DELETE FROM public.live_demo_install_completion_receipt WHERE false",
        ),
        (
            "protected learner graph read",
            "SELECT 1 FROM public.question_attempt",
        ),
        (
            "protected learner graph write",
            "UPDATE public.question_attempt SET attempt_status=attempt_status WHERE false",
        ),
    ] {
        let mut transaction = pool.begin().await.expect("direct role transaction");
        if let Some(role) = role {
            sqlx::query(role.set_statement())
                .execute(&mut *transaction)
                .await
                .expect("assume direct application or installer role");
        }
        let result = if label.ends_with("read") {
            sqlx::query(statement)
                .fetch_optional(&mut *transaction)
                .await
                .map(|_| ())
        } else {
            sqlx::query(statement)
                .execute(&mut *transaction)
                .await
                .map(|_| ())
        };
        assert_permission_denied(result, &format!("{login_name} {label}"));
        transaction
            .rollback()
            .await
            .expect("discard denied direct role attempt");
    }
}

async fn assert_read_committed_refusal(fixture: &CompletionFixture) {
    let direct_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&fixture.product.installer_url)
        .await
        .expect("direct installer LOGIN for READ COMMITTED oracle");
    let mut transaction = direct_pool
        .begin()
        .await
        .expect("READ COMMITTED completion transaction");
    sqlx::query("SET LOCAL ROLE ple_base_course_installer")
        .execute(&mut *transaction)
        .await
        .expect("installer capability role");
    sqlx::query("SELECT public.ple_base_course_install_acquire_lock_v1()")
        .execute(&mut *transaction)
        .await
        .expect("installer advisory lock");
    assert_permission_denied(
        sqlx::query(
            "SELECT * FROM public.ple_base_course_install_complete_v2($1,$2,'base-course-v1','[]'::jsonb,$3)",
        )
        .bind(fixture.participants.tenant.as_uuid())
        .bind(fixture.generation)
        .bind(STORAGE_RECEIPT_SHA256)
        .execute(&mut *transaction)
        .await,
        "READ COMMITTED direct completion",
    );
    transaction
        .rollback()
        .await
        .expect("discard rejected READ COMMITTED completion");
    direct_pool.close().await;
    let unchanged: (String, i64) = sqlx::query_as(
        "SELECT state,(SELECT count(*) FROM public.live_demo_install_completion_receipt) \
         FROM public.live_demo_install_state WHERE singleton",
    )
    .fetch_one(&fixture.pool)
    .await
    .expect("READ COMMITTED atomicity evidence");
    assert_eq!(unchanged, ("installing".to_string(), 0));
}

async fn assert_grade_scheme_refusal(
    fixture: &CompletionFixture,
    course_title: &'static str,
    unexpected_revision: i64,
    label: &'static str,
) {
    let mut mutation = fixture
        .pool
        .begin()
        .await
        .expect("begin isolated grade-scheme mutation");
    sqlx::query("SAVEPOINT grade_scheme_completion_refusal")
        .execute(&mut *mutation)
        .await
        .expect("grade-scheme refusal savepoint");
    // The product fences direct roster writes. This child-only negative oracle uses the
    // database owner's transaction-local replica mode to model an already-committed corrupt row.
    sqlx::query("SET LOCAL session_replication_role='replica'")
        .execute(&mut *mutation)
        .await
        .expect("test-owned grade-scheme drift bypasses only child-database write fencing");
    let changed = sqlx::query(
        "UPDATE public.course_grade_scheme SET revision=$1 \
         WHERE course_id=(SELECT course_id FROM public.course WHERE title=$2)",
    )
    .bind(unexpected_revision)
    .bind(course_title)
    .execute(&mut *mutation)
    .await
    .expect("install deliberate isolated grade-scheme revision drift");
    assert_eq!(
        changed.rows_affected(),
        1,
        "{label} fixture changes one scheme"
    );
    sqlx::query("RELEASE SAVEPOINT grade_scheme_completion_refusal")
        .execute(&mut *mutation)
        .await
        .expect("grade-scheme refusal savepoint release");
    mutation
        .commit()
        .await
        .expect("commit isolated grade-scheme revision drift");

    let direct_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&fixture.product.installer_url)
        .await
        .expect("direct installer LOGIN for serializable completion refusal");
    let mut completion = direct_pool
        .begin()
        .await
        .expect("serializable grade-scheme completion transaction");
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *completion)
        .await
        .expect("serializable completion isolation");
    sqlx::query("SET LOCAL ROLE ple_base_course_installer")
        .execute(&mut *completion)
        .await
        .expect("installer role for grade-scheme completion refusal");
    sqlx::query("SELECT public.ple_base_course_install_acquire_lock_v1()")
        .execute(&mut *completion)
        .await
        .expect("installer advisory lock for grade-scheme completion refusal");
    let witness: (Option<String>, Option<serde_json::Value>, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT * FROM public.ple_base_course_install_complete_v2($1,$2,'base-course-v1','[]'::jsonb,$3)",
        )
        .bind(fixture.participants.tenant.as_uuid())
        .bind(fixture.generation)
        .bind(STORAGE_RECEIPT_SHA256)
        .fetch_one(&mut *completion)
        .await
        .expect("typed grade-scheme completion refusal witness");
    assert_eq!(
        witness.0.as_deref(),
        Some("completion_aggregate_incomplete")
    );
    assert!(
        witness.1.is_none(),
        "{label} refusal exposes no receipt JSON"
    );
    assert!(
        witness.2.is_none(),
        "{label} refusal exposes no receipt text"
    );
    assert!(
        witness.3.is_none(),
        "{label} refusal exposes no receipt digest"
    );
    completion
        .commit()
        .await
        .expect("commit typed grade-scheme refusal without lifecycle mutation");
    direct_pool.close().await;

    let atomicity: (String, i64) = sqlx::query_as(
        "SELECT state,(SELECT count(*) FROM public.live_demo_install_completion_receipt) \
         FROM public.live_demo_install_state WHERE singleton",
    )
    .fetch_one(&fixture.pool)
    .await
    .expect("grade-scheme completion refusal atomicity");
    assert_eq!(atomicity, ("installing".to_string(), 0), "{label}");
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn completion_refuses_base_course_scheme_revision_one_without_a_receipt() {
    let _guard = COMPLETION_TEST_LOCK.lock().await;
    let fixture = CompletionFixture::installing_with_full_product_graph().await;
    assert_grade_scheme_refusal(
        &fixture,
        "Biochemistry: Protein Structure and Function",
        1,
        "Base Course scheme revision 1",
    )
    .await;
    fixture.cleanup().await;
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn completion_refuses_practice_course_scheme_revision_two_without_a_receipt() {
    let _guard = COMPLETION_TEST_LOCK.lock().await;
    let fixture = CompletionFixture::installing_with_full_product_graph().await;
    assert_grade_scheme_refusal(
        &fixture,
        "Genetics Practice Course",
        2,
        "Genetics Practice scheme revision 2",
    )
    .await;
    fixture.cleanup().await;
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn completion_receipt_is_atomic_immutable_and_closed_to_runtime_logins() {
    let _guard = COMPLETION_TEST_LOCK.lock().await;
    let fixture = CompletionFixture::installing_with_full_product_graph().await;
    assert_read_committed_refusal(&fixture).await;
    let installer = fixture.product.installer_pool();
    let store = fixture.product.store();
    let completed = base_course_installation::install(
        &installer,
        &store,
        install_request(fixture.participants, &fixture.storage_receipt_json),
    )
    .await
    .expect("the exact installing graph completes after test trigger removal");
    assert_eq!(completed.action(), BaseCourseAction::Resumed);
    assert_eq!(
        completed.install_state(),
        BaseCourseInstallStateOutput::Complete
    );
    drop(store);
    drop(installer);

    let receipt: (
        String,
        String,
        String,
        uuid::Uuid,
        uuid::Uuid,
        String,
        String,
        String,
    ) = sqlx::query_as(
        "SELECT receipt.canonical_receipt::text, receipt.receipt_sha256, \
         encode(digest(convert_to(receipt.canonical_receipt::text,'UTF8'),'sha256'),'hex'), \
         receipt.installation_generation, receipt.tenant_id, receipt.recipe_sha256, \
         state.completion_receipt_sha256, recipe.recipe_sha256 \
         FROM public.live_demo_install_completion_receipt AS receipt \
         JOIN public.live_demo_install_state AS state ON state.singleton \
         JOIN public.live_demo_install_recipe AS recipe ON recipe.singleton \
         WHERE receipt.installation_generation=state.installation_generation",
    )
    .fetch_one(&fixture.pool)
    .await
    .expect("single immutable completion receipt");
    assert_eq!(
        receipt.1, receipt.2,
        "database digest matches canonical JSON text"
    );
    assert_eq!(
        receipt.1, receipt.6,
        "marker names the immutable receipt digest"
    );
    assert_eq!(receipt.3, fixture.generation);
    assert_eq!(receipt.4, fixture.participants.tenant.as_uuid());
    assert_eq!(
        receipt.5, receipt.7,
        "receipt binds the immutable recipe digest"
    );
    assert!(
        receipt.1.len() == 64
            && receipt.1.bytes().all(|byte| {
                byte.is_ascii_digit() || (byte.is_ascii_lowercase() && byte.is_ascii_hexdigit())
            }),
        "completion digest is exactly lowercase hexadecimal"
    );
    assert_eq!(
        receipt.0,
        sqlx::query_scalar::<_, String>(
            "SELECT canonical_receipt::text FROM public.live_demo_install_completion_receipt \
             WHERE installation_generation=$1",
        )
        .bind(fixture.generation)
        .fetch_one(&fixture.pool)
        .await
        .expect("receipt has one immutable canonical JSON serialization")
    );

    let application_direct = PgPoolOptions::new()
        .max_connections(1)
        .connect(&fixture.product.application_url)
        .await
        .expect("direct application LOGIN connection");
    let installer_direct = PgPoolOptions::new()
        .max_connections(1)
        .connect(&fixture.product.installer_url)
        .await
        .expect("direct installer LOGIN connection");
    assert_direct_role_denials(&application_direct, None, "application LOGIN").await;
    assert_direct_role_denials(
        &application_direct,
        Some(DirectRole::Application),
        "application LOGIN/ple_app",
    )
    .await;
    assert_direct_role_denials(&installer_direct, None, "installer LOGIN").await;
    assert_direct_role_denials(
        &installer_direct,
        Some(DirectRole::Installer),
        "installer LOGIN/ple_base_course_installer",
    )
    .await;
    application_direct.close().await;
    installer_direct.close().await;

    sqlx::query("UPDATE public.course SET title='Retained ordinary edit' WHERE tenant_id=$1")
        .bind(fixture.participants.tenant.as_uuid())
        .execute(&fixture.pool)
        .await
        .expect("ordinary post-completion edit");
    let retained_installer = fixture.product.installer_pool();
    let retained_store = fixture.product.store();
    let retained = base_course_installation::install(
        &retained_installer,
        &retained_store,
        prepare_request(fixture.participants),
    )
    .await
    .expect("retained call leaves ordinary post-completion edit untouched");
    assert_eq!(retained.action(), BaseCourseAction::Retained);
    drop(retained_store);
    drop(retained_installer);
    let receipt_after_edit: (String, i64, i64) = sqlx::query_as(
        "SELECT (SELECT canonical_receipt::text FROM public.live_demo_install_completion_receipt), \
         (SELECT count(*) FROM public.live_demo_install_completion_receipt), \
         (SELECT count(*) FROM public.course WHERE title='Retained ordinary edit')",
    )
    .fetch_one(&fixture.pool)
    .await
    .expect("retained edit and immutable receipt evidence");
    assert_eq!(receipt_after_edit.0, receipt.0);
    assert_eq!(receipt_after_edit.1, 1);
    assert_eq!(receipt_after_edit.2, 2);
    fixture.cleanup().await;
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn serializable_completion_retries_the_full_verifier_for_invalid_and_valid_conflicts() {
    let _guard = COMPLETION_TEST_LOCK.lock().await;
    let invalid = CompletionFixture::installing_with_full_product_graph().await;
    let invalid_result = run_serializable_cycle(&invalid, "forest").await;
    match invalid_result {
        Err(BaseCourseInstallError::Persistence {
            operation,
            source: learning_data_access::StoreError::InvalidRecord(message),
        }) => {
            assert_eq!(operation, "completing the Base Course lifecycle state");
            assert_eq!(
                message,
                "Base Course completion aggregate does not exactly match the versioned recipe"
            );
        }
        other => panic!(
            "invalid committed graph must produce the safe typed completion refusal after retry: {other:?}"
        ),
    }
    let invalid_atomicity: (String, i64, i64) = sqlx::query_as(
        "SELECT state, \
         (SELECT count(*) FROM public.live_demo_install_completion_receipt), \
         (SELECT last_value FROM public.ple_t4_completion_barrier_sequence)
         FROM public.live_demo_install_state WHERE singleton",
    )
    .fetch_one(&invalid.pool)
    .await
    .expect("invalid serializable retry atomicity evidence");
    assert_eq!(invalid_atomicity.0, "installing");
    assert_eq!(invalid_atomicity.1, 0);
    assert_eq!(
        invalid_atomicity.2, 1,
        "the retry re-verifies the newly committed invalid graph before receipt insertion"
    );
    invalid.cleanup().await;

    let valid = CompletionFixture::installing_with_full_product_graph().await;
    let valid_result = run_serializable_cycle(&valid, "grass")
        .await
        .expect("same-final-valid committed conflict retries in a fresh transaction");
    assert_eq!(valid_result.action(), BaseCourseAction::Resumed);
    let valid_atomicity: (String, i64, i64) = sqlx::query_as(
        "SELECT state, \
         (SELECT count(*) FROM public.live_demo_install_completion_receipt), \
         (SELECT last_value FROM public.ple_t4_completion_barrier_sequence) \
         FROM public.live_demo_install_state WHERE singleton",
    )
    .fetch_one(&valid.pool)
    .await
    .expect("valid serializable retry evidence");
    assert_eq!(valid_atomicity.0, "complete");
    assert_eq!(
        valid_atomicity.1, 1,
        "one receipt survives the retried completion"
    );
    assert_eq!(
        valid_atomicity.2, 2,
        "the completion query receives 40001, then Rust opens a fresh transaction and retries"
    );
    valid.cleanup().await;
}
