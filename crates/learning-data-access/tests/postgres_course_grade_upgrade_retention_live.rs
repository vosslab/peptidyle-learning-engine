#![cfg(feature = "postgres")]

//! Disposable lifecycle oracle for the S6 upgrade backfill and retention wrapper.
//!
//! Graphify's report explicitly omits SQL edges because `tree_sitter_sql` is
//! unavailable.  This test therefore directly exercises the two SQL lifecycle
//! boundaries rather than inferring them from the Rust call graph.

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

use std::fs;
use std::str::FromStr;

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    CourseGradebookStore, CourseRecord, CreateCourseCommand, JobClaimFilter, JobKind,
    JobLeaseDuration, JobPayload, JobStore, RetentionApiStore, RetentionStage, RetentionStore,
    RetentionWorkerCommand, RetentionWorkerStore, SessionLifetime, SessionStore, SessionSubject,
    SessionTokenHash, Store, StoreError, TenantContext,
};
use question_model::{CourseId, CourseTerm, TenantId, UserId, UserRole};
use sqlx::AssertSqlSafe;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

fn fresh() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("fixture randomness");
    Uuid::from_bytes(bytes)
}

fn disposable_url() -> String {
    std::env::var("PLE_TEST_DATABASE_URL").expect("disposable acceptance database URL")
}

fn generated_database_name() -> String {
    // Identifier is generated locally, contains only ASCII identifier bytes,
    // and can never select a caller-owned database.
    format!("ple_s6_upgrade_{:x}", fresh().as_u128())
}

async fn admin_pool(url: &str) -> sqlx::PgPool {
    let options = PgConnectOptions::from_str(url)
        .expect("acceptance PostgreSQL URL")
        .database("postgres");
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .expect("admin connection derived from disposable URL")
}

fn copied_migrations(exclude_s6: bool) -> std::path::PathBuf {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/migrations");
    let destination = std::env::temp_dir().join(format!("ple-s6-migrations-{}", fresh()));
    fs::create_dir_all(&destination).expect("temporary migration directory");
    for entry in fs::read_dir(source).expect("migration directory") {
        let entry = entry.expect("migration entry");
        let name = entry.file_name();
        if exclude_s6 && name.to_string_lossy().starts_with("2026081806_") {
            continue;
        }
        fs::copy(entry.path(), destination.join(name)).expect("copy immutable migration");
    }
    destination
}

async fn create_pre_s6_course(pool: &sqlx::PgPool, tenant: Uuid, course: Uuid) {
    // This connection is the disposable migration principal, not the app role.
    // The values satisfy the pre-S6 course shape through migration 1805.
    sqlx::query(
        "INSERT INTO public.course (tenant_id, course_id, title, term_start_date, term_end_date, time_zone) \
         VALUES ($1,$2,'Pre-S6 upgrade fixture','2026-08-24','2026-12-18','America/Chicago')",
    )
    .bind(tenant)
    .bind(course)
    .execute(pool)
    .await
    .expect("pre-S6 course persists");
}

async fn assert_upgrade_backfill() {
    let source_url = disposable_url();
    let admin = admin_pool(&source_url).await;
    let name = generated_database_name();
    assert!(name.starts_with("ple_s6_upgrade_") && name.len() < 64);
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&admin)
        .await
        .expect("create unique disposable upgrade database");
    // Run the migration body as a task so ordinary assertion failures and
    // migration panics still return here for generated-database cleanup.
    let upgrade_database = name.clone();
    let result = tokio::spawn(async move {
        let base_options = PgConnectOptions::from_str(&source_url)
            .expect("acceptance URL")
            .database(&upgrade_database);
        let upgrade_pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(base_options)
            .await
            .expect("upgrade database connection");
        let before = copied_migrations(true);
        sqlx::migrate::Migrator::new(before.clone())
            .await
            .expect("1805 migrator")
            .run(&upgrade_pool)
            .await
            .expect("migrate through 1805");
        let tenant = fresh();
        let existing = fresh();
        create_pre_s6_course(&upgrade_pool, tenant, existing).await;
        let full = copied_migrations(false);
        sqlx::migrate::Migrator::new(full.clone())
            .await
            .expect("full migrator")
            .run(&upgrade_pool)
            .await
            .expect("apply S6 migration exactly once");
        let backfilled: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public.course_grade_scheme WHERE tenant_id=$1 AND course_id=$2 \
             AND revision=1 AND mode='total_points' AND rounding='four_decimal_places_half_away_from_zero'",
        )
        .bind(tenant)
        .bind(existing)
        .fetch_one(&upgrade_pool)
        .await
        .expect("backfill count");
        assert_eq!(backfilled, 1, "preexisting course receives exactly one default scheme");
        let later = fresh();
        create_pre_s6_course(&upgrade_pool, tenant, later).await;
        let triggered: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public.course_grade_scheme WHERE tenant_id=$1 AND course_id=$2 AND revision=1",
        )
        .bind(tenant)
        .bind(later)
        .fetch_one(&upgrade_pool)
        .await
        .expect("trigger default count");
        assert_eq!(triggered, 1, "new course trigger installs one default scheme");
        let ledger: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public._sqlx_migrations WHERE success AND version=2026081806",
        )
        .fetch_one(&upgrade_pool)
        .await
        .expect("S6 ledger count");
        assert_eq!(ledger, 1, "S6 migration ledger has one successful row");
        let checksums: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT checksum) FROM public._sqlx_migrations WHERE success AND version=2026081806",
        )
        .fetch_one(&upgrade_pool)
        .await
        .expect("S6 checksum count");
        assert_eq!(checksums, 1, "S6 ledger checksum is singular");
        fs::remove_dir_all(before).expect("remove copied pre-S6 migrations");
        fs::remove_dir_all(full).expect("remove copied full migrations");
    })
    .await;
    // Terminate fixture connections before dropping its generated database.
    let _ = sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
        .bind(&name)
        .execute(&admin)
        .await;
    let dropped = sqlx::query(AssertSqlSafe(format!("DROP DATABASE IF EXISTS {name}")))
        .execute(&admin)
        .await;
    assert!(
        dropped.is_ok(),
        "ordinary cleanup drops only generated database"
    );
    result.expect("upgrade fixture task");
}

async fn session(store: &PostgresStore, tenant: TenantId, user: UserId) -> SessionTokenHash {
    let token = SessionTokenHash::compute(fresh().as_bytes());
    store
        .create_session(
            token,
            SessionSubject::new(
                tenant,
                user,
                "S6 retention fixture",
                vec![UserRole::Instructor],
            )
            .expect("fixture session"),
            SessionLifetime::from_seconds(3600).expect("fixture lifetime"),
        )
        .await
        .expect("fixture session persists");
    token
}

async fn create_course(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
) -> CourseId {
    let course = CourseId::from_uuid(fresh());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "S6 retention wrapper fixture".into(),
                    term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                        .expect("term"),
                },
                authority: sysadmin_course_creation_authority(store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("fixture course");
    course
}

async fn insert_s6_records(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    course: CourseId,
    instructor: UserId,
) {
    // This lifecycle test has no question-publication concern.  It uses the
    // app's row shape for one active course assignment, then the Store's
    // grade-export and retention APIs for the protected lifecycle boundaries.
    let mut tx = pool.begin().await.expect("app transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("app role");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("tenant");
    let category = fresh();
    let assignment = fresh();
    sqlx::query(
        "INSERT INTO assignment (tenant_id,assignment_id,course_id,audience_kind,title,gradebook_included,score_disclosure,per_item_correctness_disclosure,feedback_text_disclosure,solution_disclosure,class_statistics_disclosure) VALUES ($1,$2,$3,'course_wide','Lifecycle mapping',false,'after_submit','after_submit','after_submit','after_submit','never')",
    )
    .bind(tenant.as_uuid())
    .bind(assignment)
    .bind(course.as_uuid())
    .execute(&mut *tx)
    .await
    .expect("fixture assignment");
    sqlx::query("UPDATE course_grade_scheme SET mode='weighted_categories',revision=2 WHERE tenant_id=$1 AND course_id=$2")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).execute(&mut *tx).await.expect("weighted scheme");
    sqlx::query("INSERT INTO course_grade_category (tenant_id,course_id,category_id,position,title,weight_basis_points,drop_lowest) VALUES ($1,$2,$3,0,'Lifecycle',10000,0)")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(category).execute(&mut *tx).await.expect("category");
    sqlx::query("INSERT INTO course_grade_category_assignment (tenant_id,course_id,category_id,assignment_id,position) VALUES ($1,$2,$3,$4,0)")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(category).bind(assignment).execute(&mut *tx).await.expect("assignment mapping");
    sqlx::query("INSERT INTO course_grade_letter_band (tenant_id,course_id,letter_band_id,label,minimum_basis_points) VALUES ($1,$2,$3,'A',9000)")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(fresh()).execute(&mut *tx).await.expect("band");
    sqlx::query("INSERT INTO course_grade_export_audit (tenant_id,course_id,assignment_id,export_id,requested_by,row_count) VALUES ($1,$2,$3,$4,$5,0)")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(assignment).bind(fresh()).bind(instructor.as_uuid()).execute(&mut *tx).await.expect("legacy audit");
    tx.commit().await.expect("seed S6 lifecycle rows");
}

async fn assert_retention_wrapper() {
    let url = disposable_url();
    let pool = lazy_pool(&url).expect("pool");
    verify_application_schema(&pool)
        .await
        .expect("baseline schema");
    let store = PostgresStore::new(pool.clone());
    let tenant = TenantId::from_uuid(fresh());
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(fresh());
    let token = session(&store, tenant, instructor).await;
    let course = create_course(&store, context, tenant, instructor).await;
    insert_s6_records(&pool, tenant, course, instructor).await;
    let export = store
        .create_course_grade_export(context, token, course)
        .await
        .expect("public Store export audit");
    assert_eq!(
        export.audit.row_count, 0,
        "empty active roster has bounded export"
    );
    let record = store
        .end_course_retention(context, token, course)
        .await
        .expect("end course retention");
    let view = record.safe_view().expect("retention view");
    let scheduled = store
        .request_retention_delete_if_revision(context, token, course, view.revision)
        .await
        .expect("queue deletion");
    let claimed = store
        .claim_next_job(
            &JobClaimFilter::new([JobKind::Retention]).expect("retention-only filter"),
            JobLeaseDuration::from_seconds(300).expect("lease"),
        )
        .await
        .expect("claim job")
        .expect("retention job");
    let (stage, generation) = match claimed.payload {
        JobPayload::Retention {
            course: payload_course,
            stage,
            generation,
        } => {
            assert_eq!(payload_course, course);
            (stage, generation)
        }
        other => panic!("expected retention job, got {other:?}"),
    };
    assert_eq!(stage, RetentionStage::DeleteStudentRecords);
    let command = RetentionWorkerCommand {
        tenant,
        course,
        stage,
        generation,
        job: claimed.id,
        lease: claimed.lease_token,
    };
    let wrong = RetentionWorkerCommand {
        generation: generation + 1,
        ..command
    };
    assert_eq!(
        store.prepare_retention_work(wrong).await,
        Err(StoreError::Conflict)
    );
    let before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM course_total_export_audit WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("count before");
    assert_eq!(before, 1, "wrong generation preserves S6 rows");
    store
        .prepare_retention_work(command)
        .await
        .expect("prepare wrapper");
    assert_eq!(
        store.commit_retention_work(wrong).await,
        Err(StoreError::Conflict)
    );
    store
        .commit_retention_work(command)
        .await
        .expect("commit wrapper");
    for relation in [
        "course_grade_scheme",
        "course_grade_category",
        "course_grade_category_assignment",
        "course_grade_letter_band",
        "course_total_export_audit",
        "course_grade_export_audit",
    ] {
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT count(*) FROM public.{relation} WHERE tenant_id=$1 AND course_id=$2"
        )))
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .fetch_one(&pool)
        .await
        .expect("absence count");
        assert_eq!(count, 0, "{relation} removed through retention wrapper");
    }
    let safe_definer: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_proc WHERE proname='ple_commit_delete_retention_work' AND prosecdef AND pg_get_functiondef(oid) LIKE '%SET search_path TO ''pg_catalog'', ''public''%'",
    ).fetch_one(&pool).await.expect("function ownership probe");
    assert_eq!(
        safe_definer, 1,
        "retention wrapper remains security definer with fixed search path"
    );
    let _ = scheduled;
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_course_grade_upgrade_backfill_and_retention_wrapper_are_lifecycle_safe() {
    assert_upgrade_backfill().await;
    assert_retention_wrapper().await;
}
