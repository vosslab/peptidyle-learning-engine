#![cfg(feature = "postgres")]

//! Disposable upgrade oracle for the T2 teaching-operations migration.
//!
//! Graphify omitted 46 SQL files and has no node for migration 1807, so this
//! test reads the physical PostgreSQL catalog and exercises the migration
//! boundary directly instead of inferring SQL behavior from Rust edges.

use std::fs;
use std::str::FromStr;

use sqlx::AssertSqlSafe;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

const T2: i64 = 2_026_081_807;

fn fresh() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("fixture randomness");
    Uuid::from_bytes(bytes)
}

fn migrations_through(version: i64) -> std::path::PathBuf {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/migrations");
    let target = std::env::temp_dir().join(format!("ple-t2-migrations-{}", fresh()));
    fs::create_dir_all(&target).expect("temporary migrations directory");
    for entry in fs::read_dir(source).expect("migration directory") {
        let entry = entry.expect("migration entry");
        let name = entry.file_name();
        let text = name.to_string_lossy();
        let Some(prefix) = text.split('_').next() else {
            continue;
        };
        if prefix.parse::<i64>().is_ok_and(|found| found <= version) {
            fs::copy(entry.path(), target.join(name)).expect("copy immutable migration");
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

async fn create_course<'e, E>(executor: E, tenant: Uuid, course: Uuid)
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query(
        "INSERT INTO public.course (tenant_id,course_id,title,term_start_date,term_end_date, \
         time_zone) \
         VALUES ($1,$2,'T2 upgrade fixture','2026-08-24','2026-12-18','America/Chicago')",
    )
    .bind(tenant)
    .bind(course)
    .execute(executor)
    .await
    .expect("pre-T2 course");
}

async fn seed_groups(pool: &sqlx::PgPool, tenant: Uuid, course: Uuid) {
    for purpose in ["section", "lab", "cohort", "accommodation", "work"] {
        sqlx::query(
            "INSERT INTO public.course_group (tenant_id,course_id,course_group_id,title,purpose) \
             VALUES ($1,$2,$3,$4,$4)",
        )
        .bind(tenant)
        .bind(course)
        .bind(fresh())
        .bind(purpose)
        .execute(pool)
        .await
        .expect("pre-T2 typed group");
    }
}

async fn scalar(pool: &sqlx::PgPool, query: &'static str) -> i64 {
    sqlx::query_scalar(query)
        .fetch_one(pool)
        .await
        .expect("catalog scalar")
}

async fn assert_existing_data_gate(pool: &sqlx::PgPool, migrations: &std::path::Path) {
    let tenant = fresh();
    let course = fresh();
    let mut fixture = pool.begin().await.expect("pre-T2 fixture transaction");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *fixture)
        .await
        .expect("set pre-T2 fixture tenant");
    create_course(&mut *fixture, tenant, course).await;
    let group = fresh();
    sqlx::query(
        "INSERT INTO public.course_group (tenant_id,course_id,course_group_id,title,purpose) \
         VALUES ($1,$2,$3,'Invalid work modifier','work')",
    )
    .bind(tenant)
    .bind(course)
    .bind(group)
    .execute(&mut *fixture)
    .await
    .expect("pre-T2 work group");
    let assignment = fresh();
    sqlx::query(
        "INSERT INTO public.assignment (tenant_id,assignment_id,course_id,audience_kind,title, \
         score_disclosure,per_item_correctness_disclosure,feedback_text_disclosure, \
         solution_disclosure, \
         class_statistics_disclosure) VALUES ($1,$2,$3,'course_wide','Invalid legacy modifier', \
         'after_submit','after_submit','after_submit','after_submit','never')",
    )
    .bind(tenant)
    .bind(assignment)
    .bind(course)
    .execute(&mut *fixture)
    .await
    .expect("pre-T2 assignment");
    sqlx::query(
        "INSERT INTO public.assignment_group_schedule_offset \
         (tenant_id,assignment_id,course_id,course_group_id,schedule_offset_seconds) \
         VALUES ($1,$2,$3,$4,60)",
    )
    .bind(tenant)
    .bind(assignment)
    .bind(course)
    .bind(group)
    .execute(&mut *fixture)
    .await
    .expect("pre-T2 invalid modifier remains representable");
    fixture.commit().await.expect("commit pre-T2 fixture");
    let error = sqlx::migrate::Migrator::new(migrations.to_path_buf())
        .await
        .expect("T2 migrator")
        .run(pool)
        .await
        .expect_err("T2 refuses invalid existing modifier references");
    assert!(error.to_string().contains("group-purpose cutover"));
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 upgrade database"]
async fn teaching_operations_upgrade_schema_oracle() {
    let runtime = load_acceptance_runtime();
    let url = runtime.admin_url().expose().to_owned();
    let admin = admin_pool(&url).await;
    let database = format!("ple_t2_upgrade_{:x}", fresh().as_u128());
    assert!(database.starts_with("ple_t2_upgrade_") && database.len() < 64);
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database}")))
        .execute(&admin)
        .await
        .expect("create generated database");
    let cleanup_name = database.clone();
    let gate_admin = admin.clone();
    let result = tokio::spawn(async move {
        let options = PgConnectOptions::from_str(&url)
            .expect("acceptance URL")
            .database(&database);
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .expect("upgrade database connection");
        let before = migrations_through(T2 - 1);
        sqlx::migrate::Migrator::new(before.clone())
            .await
            .expect("pre-T2 migrator")
            .run(&pool)
            .await
            .expect("migrate through 1806");
        let tenant = fresh();
        let course = fresh();
        create_course(&pool, tenant, course).await;
        seed_groups(&pool, tenant, course).await;
        let full = migrations_through(T2);
        sqlx::migrate::Migrator::new(full.clone())
            .await
            .expect("T2 migrator")
            .run(&pool)
            .await
            .expect("apply 1807 exactly once");

        assert_eq!(
            scalar(
                &pool,
                "SELECT count(*) FROM public._sqlx_migrations \
                 WHERE success AND version=2026081807",
            )
            .await,
            1
        );
        assert_eq!(
            scalar(
                &pool,
                "SELECT count(DISTINCT checksum) FROM public._sqlx_migrations \
                 WHERE success AND version=2026081807",
            )
            .await,
            1
        );
        let policies: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT purpose,multiple_membership,revision \
             FROM public.course_group_membership_policy \
             WHERE tenant_id=$1 AND course_id=$2 ORDER BY purpose",
        )
        .bind(tenant)
        .bind(course)
        .fetch_all(&pool)
        .await
        .expect("existing course policies");
        assert_eq!(
            policies,
            vec![
                ("accommodation".into(), "allow".into(), 1),
                ("cohort".into(), "allow".into(), 1),
                ("lab".into(), "allow".into(), 1),
                ("section".into(), "warn".into(), 1),
                ("work".into(), "allow".into(), 1),
            ]
        );
        let later = fresh();
        create_course(&pool, tenant, later).await;
        let future_policies: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT purpose,multiple_membership,revision \
             FROM public.course_group_membership_policy \
             WHERE tenant_id=$1 AND course_id=$2 ORDER BY purpose",
        )
        .bind(tenant)
        .bind(later)
        .fetch_all(&pool)
        .await
        .expect("future course policies");
        assert_eq!(
            future_policies, policies,
            "future courses receive the same defaults"
        );

        macro_rules! assert_catalog_count {
            ($query:expr, $expected:expr) => {
                assert_eq!(scalar(&pool, $query).await, $expected);
            };
        }
        assert_catalog_count!(
            "SELECT count(*) FROM pg_attribute \
             WHERE attrelid='public.instructor_approval'::regclass \
             AND attname='tenant_id' AND NOT attisdropped",
            0
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_policy \
             WHERE polrelid='public.instructor_approval'::regclass",
            0
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_constraint \
             WHERE conrelid='public.instructor_approval'::regclass AND contype='c' \
             AND ((conname='instructor_approval_chronology_check' \
                   AND pg_get_constraintdef(oid) LIKE '%revoked_at >= approved_at%') \
                  OR (conname='instructor_approval_revision_check' \
                      AND pg_get_constraintdef(oid) LIKE '%revision > 0%'))",
            2
        );
        assert_catalog_count!(
            "SELECT count(*) FROM information_schema.columns WHERE table_schema='public' \
             AND table_name='course_instructor_invitation' AND column_name ILIKE '%email%'",
            0
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_constraint \
             WHERE conrelid='public.course_instructor_invitation'::regclass AND contype='c' \
             AND ((conname='course_instructor_invitation_expiry_check' \
                   AND pg_get_constraintdef(oid) LIKE '%30 days%') \
                  OR (conname='course_instructor_invitation_lifecycle_check' \
                      AND pg_get_constraintdef(oid) LIKE '%accepted%' \
                      AND pg_get_constraintdef(oid) LIKE '%expired%'))",
            2
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_index \
             WHERE indrelid='public.course_instructor_invitation'::regclass AND indisunique \
             AND pg_get_expr(indpred,indrelid)='(status = ''pending''::text)'",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_constraint \
             WHERE conrelid='public.course_instructor_invitation'::regclass AND contype='f' \
             AND confdeltype='r'",
            2
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_constraint \
             WHERE conname IN ('assignment_group_schedule_offset_group_fk', \
             'assignment_group_accommodation_group_fk') AND confdeltype='r'",
            2
        );
        assert_catalog_count!(
            "WITH expected(tgname, tgfoid) AS (VALUES \
             ('assignment_audience_group_purpose_check', \
              'public.ple_validate_assignment_group_reference_purpose()'::regprocedure), \
             ('assignment_group_schedule_offset_purpose_check', \
              'public.ple_validate_assignment_group_reference_purpose()'::regprocedure), \
             ('assignment_group_accommodation_purpose_check', \
              'public.ple_validate_assignment_group_reference_purpose()'::regprocedure), \
             ('course_group_purpose_reference_check', \
              'public.ple_validate_course_group_purpose_references()'::regprocedure)) \
             SELECT count(*) FROM expected \
             JOIN pg_trigger ON pg_trigger.tgname=expected.tgname \
                            AND pg_trigger.tgfoid=expected.tgfoid \
             WHERE pg_trigger.tgconstraint <> 0 AND pg_trigger.tgdeferrable \
               AND pg_trigger.tginitdeferred",
            4
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_class \
             WHERE oid IN ('public.course_group_membership_policy'::regclass, \
             'public.course_instructor_invitation'::regclass) \
             AND relrowsecurity AND relforcerowsecurity",
            2
        );
        assert_catalog_count!(
            "WITH expected(polrelid, polname) AS (VALUES \
             ('public.course_group_membership_policy'::regclass, \
              'course_group_membership_policy_app'::name), \
             ('public.course_instructor_invitation'::regclass, \
              'course_instructor_invitation_app'::name)) \
             SELECT count(*) FROM expected \
             JOIN pg_policy ON pg_policy.polrelid=expected.polrelid \
                           AND pg_policy.polname=expected.polname \
             WHERE pg_policy.polroles @> ARRAY['ple_app'::regrole::oid]",
            2
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_roles WHERE rolname='ple_teaching_authority_broker' \
             AND NOT rolcanlogin AND NOT rolbypassrls AND NOT rolsuper",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_policy \
             WHERE polrelid='public.ple_account'::regclass \
               AND polname='ple_account_teaching_authority_broker' \
               AND polcmd='r' \
               AND polroles = ARRAY['ple_teaching_authority_broker'::regrole::oid] \
               AND pg_get_expr(polqual, polrelid) = 'true' \
               AND polwithcheck IS NULL",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_attribute \
             WHERE attrelid='public.ple_account'::regclass \
               AND attname='user_id' \
               AND has_column_privilege( \
                   'ple_teaching_authority_broker', \
                   'public.ple_account', \
                   'user_id', \
                   'SELECT' \
               ) \
               AND NOT has_column_privilege( \
                   'ple_teaching_authority_broker', \
                   'public.ple_account', \
                   'normalized_email', \
                   'SELECT' \
               ) \
               AND NOT has_column_privilege( \
                   'ple_teaching_authority_broker', \
                   'public.ple_account', \
                   'delivery_email', \
                   'SELECT' \
               ) \
               AND has_column_privilege( \
                   'ple_teaching_authority_broker', \
                   'public.ple_account', \
                   'public_id', \
                   'SELECT' \
               ) \
               AND has_column_privilege( \
                   'ple_teaching_authority_broker', \
                   'public.ple_account', \
                   'display_name', \
                   'SELECT' \
               )",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_attribute AS attribute \
             CROSS JOIN LATERAL aclexplode(attribute.attacl) AS acl_entry \
             WHERE attribute.attrelid='public.ple_account'::regclass \
               AND attribute.attname='user_id' \
               AND acl_entry.grantee='ple_teaching_authority_broker'::regrole \
               AND acl_entry.privilege_type='SELECT'",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_constraint \
             WHERE conrelid='public.instructor_approval'::regclass \
               AND confrelid='public.ple_account'::regclass \
               AND contype='f' AND confdeltype='a'",
            2
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc WHERE proname IN ('ple_instructor_approval_eligible', \
             'ple_target_session_subject','ple_own_account_reference', \
             'ple_sysadmin_account_reference','ple_approved_account_reference', \
             'ple_lock_instructor_approval_eligibility', \
             'ple_sysadmin_instructor_approval', \
             'ple_sysadmin_revoke_instructor_approval') AND prosecdef \
             AND proowner='ple_teaching_authority_broker'::regrole \
             AND proconfig @> ARRAY['search_path=pg_catalog, public']",
            8
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc \
             WHERE proname='ple_course_instructor_roster_revision' AND prosecdef \
             AND proowner='ple_teaching_authority_broker'::regrole \
             AND proconfig @> ARRAY['search_path=pg_catalog, public']",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc \
             WHERE oid='public.ple_course_co_instructor_target_search(\
                 uuid,uuid,text,integer,integer)'::regprocedure \
               AND prosecdef AND proowner='ple_teaching_authority_broker'::regrole \
               AND proconfig @> ARRAY['search_path=pg_catalog, public'] \
               AND NOT has_function_privilege('public', oid, 'EXECUTE') \
               AND has_function_privilege('ple_app', oid, 'EXECUTE')",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc \
             WHERE oid='public.ple_course_active_student_membership_reference_list(\
                 uuid,uuid,integer,integer)'::regprocedure \
               AND prosecdef AND proowner='ple_teaching_authority_broker'::regrole \
               AND proconfig @> ARRAY['search_path=pg_catalog, public'] \
               AND NOT has_function_privilege('public', oid, 'EXECUTE') \
               AND has_function_privilege('ple_app', oid, 'EXECUTE')",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc \
             WHERE oid='public.ple_course_active_student_membership_reference(\
                 uuid,uuid,uuid)'::regprocedure \
               AND prosecdef AND proowner='ple_teaching_authority_broker'::regrole \
               AND proconfig @> ARRAY['search_path=pg_catalog, public'] \
               AND NOT has_function_privilege('public', oid, 'EXECUTE') \
               AND has_function_privilege('ple_app', oid, 'EXECUTE')",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_policy \
             WHERE polrelid='public.course_instructor_invitation'::regclass \
               AND polname='course_instructor_invitation_teaching_authority_broker' \
               AND polcmd='r' \
               AND polroles=ARRAY['ple_teaching_authority_broker'::regrole::oid]",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_class AS sequence WHERE sequence.oid IN ( \
                'public.ple_account_public_id_seq'::regclass, \
                'public.course_member_public_id_seq'::regclass, \
                'public.course_instructor_invitation_public_id_seq'::regclass \
             ) AND relkind='S'",
            3
        );
        assert_catalog_count!(
            "SELECT count(*) WHERE has_sequence_privilege( \
                'ple_auth', 'public.ple_account_public_id_seq', 'USAGE,SELECT') \
             AND NOT has_sequence_privilege( \
                'ple_app', 'public.ple_account_public_id_seq', 'USAGE') \
             AND has_sequence_privilege( \
                'ple_app', 'public.course_member_public_id_seq', 'USAGE,SELECT') \
             AND has_sequence_privilege( \
                'ple_app', 'public.course_instructor_invitation_public_id_seq', 'USAGE,SELECT') \
             AND NOT has_sequence_privilege( \
                'ple_teaching_authority_broker', \
                'public.ple_account_public_id_seq', 'USAGE')",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc \
             WHERE proname='ple_instructor_approval_eligible' AND provolatile='s'",
            1
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc \
             WHERE proname IN ('ple_lock_instructor_approval_eligibility', \
             'ple_sysadmin_instructor_approval','ple_sysadmin_revoke_instructor_approval') \
             AND provolatile='v'",
            3
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc \
             CROSS JOIN LATERAL aclexplode(COALESCE(proacl, acldefault('f', proowner))) \
             WHERE proname IN ('ple_instructor_approval_eligible', 'ple_target_session_subject', \
             'ple_lock_instructor_approval_eligibility','ple_sysadmin_instructor_approval', \
             'ple_sysadmin_revoke_instructor_approval') \
             AND grantee=0 AND privilege_type='EXECUTE'",
            0
        );
        assert_catalog_count!(
            "SELECT count(*) FROM information_schema.role_table_grants WHERE grantee='ple_app' \
             AND table_schema='public' AND table_name='instructor_approval'",
            0
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc WHERE proname IN ('ple_sysadmin_instructor_approval', \
             'ple_sysadmin_revoke_instructor_approval') \
             AND pg_get_functiondef(oid) LIKE '%ERRCODE = ''55000''%'",
            2
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc \
             WHERE proname IN ('ple_sysadmin_instructor_approval', \
             'ple_sysadmin_revoke_instructor_approval') \
             AND pg_get_functiondef(oid) LIKE '%AS approval%approval.user_id%'
             AND pg_get_functiondef(oid) LIKE '%revision = approval.revision + 1%'",
            2
        );
        assert_catalog_count!(
            "SELECT count(*) FROM pg_proc WHERE proname='ple_guard_final_active_course_instructor' \
             AND pg_get_functiondef(oid) LIKE '%FROM public.course%' \
             AND pg_get_functiondef(oid) LIKE '%FOR UPDATE%'",
            1
        );

        // A second generated database makes the migration's pre-existing-data
        // refusal observable without contaminating the successful upgrade case.
        let gate_name = format!("ple_t2_gate_{:x}", fresh().as_u128());
        sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {gate_name}")))
            .execute(&gate_admin)
            .await
            .expect("gate database");
        let gate_url = url.clone();
        let gate_task_name = gate_name.clone();
        let gate_migrations = full.clone();
        let gate_result = tokio::spawn(async move {
            let gate_options = PgConnectOptions::from_str(&gate_url)
                .expect("URL")
                .database(&gate_task_name);
            let gate_pool = PgPoolOptions::new()
                .max_connections(2)
                .connect_with(gate_options)
                .await
                .expect("gate pool");
            let gate_before = migrations_through(T2 - 1);
            sqlx::migrate::Migrator::new(gate_before.clone())
                .await
                .expect("gate pre migrator")
                .run(&gate_pool)
                .await
                .expect("gate through 1806");
            assert_existing_data_gate(&gate_pool, &gate_migrations).await;
            gate_pool.close().await;
            fs::remove_dir_all(gate_before).expect("remove gate migrations");
            Ok::<(), String>(())
        })
        .await;
        let _ =
            sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
                .bind(&gate_name)
                .execute(&gate_admin)
                .await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {gate_name}"
        )))
        .execute(&gate_admin)
        .await
        .expect("drop gate database");
        gate_result
            .expect("existing-data gate task")
            .expect("existing-data gate");
        fs::remove_dir_all(before).expect("remove pre migrations");
        fs::remove_dir_all(full).expect("remove full migrations");
    })
    .await;
    let _ = sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
        .bind(&cleanup_name)
        .execute(&admin)
        .await;
    let dropped = sqlx::query(AssertSqlSafe(format!(
        "DROP DATABASE IF EXISTS {cleanup_name}"
    )))
    .execute(&admin)
    .await;
    assert!(
        dropped.is_ok(),
        "cleanup drops only the generated T2 database"
    );
    result.expect("upgrade task");
}
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
