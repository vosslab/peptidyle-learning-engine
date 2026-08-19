#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 oracle for course-term storage, constraints, and RLS.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    CourseListScope, CourseRecord, CreateCourseCommand, PageRequest, PageSize, Store, StoreError,
    TenantContext,
};
use question_model::{CourseId, CourseTerm, TenantId, UserId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

async fn rejected_course_insert(
    pool: &PgPool,
    tenant: TenantId,
    start_date: Option<&str>,
    end_date: Option<&str>,
    time_zone: Option<&str>,
) -> sqlx::Error {
    let mut transaction = pool.begin().await.expect("begin term constraint probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("constraint probe assumes the application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("constraint probe sets its tenant context");
    let error = sqlx::query(
        "INSERT INTO course (tenant_id, course_id, title, term_start_date, term_end_date, \
         time_zone) VALUES ($1, $2, 'Rejected term', $3::text::date, $4::text::date, $5)",
    )
    .bind(tenant.as_uuid())
    .bind(id())
    .bind(start_date)
    .bind(end_date)
    .bind(time_zone)
    .execute(&mut *transaction)
    .await
    .expect_err("invalid course term must be rejected by PostgreSQL");
    transaction
        .rollback()
        .await
        .expect("aborted constraint probe rolls back");
    error
}

fn database_code(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map(|code| code.into_owned())
}

fn constraint(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .and_then(|error| error.constraint())
}

#[tokio::test]
#[ignore = "requires a fresh disposable PostgreSQL 17 database with the full migration chain"]
async fn postgres_course_terms_round_trip_enforce_constraints_and_remain_tenant_isolated() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("full migrated application schema is compatible");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x42; 32]);
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let course = CourseId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let term = CourseTerm::from_parts("0001-01-01", "9999-12-31", "Pacific/Kiritimati")
        .expect("supported application date and IANA boundaries");
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "PostgreSQL term boundary course".to_string(),
                    term: term.clone(),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("valid boundary course term persists");
    let stored = store
        .get_course(context, course)
        .await
        .expect("course read succeeds")
        .expect("course remains");
    assert_eq!(stored.term, term);
    let listed = store
        .list_courses(
            context,
            CourseListScope::Member(instructor),
            PageRequest::first(PageSize::new(10).expect("bounded course page")),
        )
        .await
        .expect("course list succeeds");
    assert_eq!(listed.items.len(), 1);
    assert_eq!(listed.items[0].term, stored.term);
    assert_eq!(
        store
            .get_course(
                TenantContext::from_authenticated_session(foreign_tenant),
                course,
            )
            .await,
        Ok(None),
    );

    for (start_date, end_date, time_zone) in [
        (None, Some("2026-12-18"), Some("America/Chicago")),
        (Some("2026-08-24"), None, Some("America/Chicago")),
        (Some("2026-08-24"), Some("2026-12-18"), None),
    ] {
        let error = rejected_course_insert(&pool, tenant, start_date, end_date, time_zone).await;
        assert_eq!(database_code(&error).as_deref(), Some("23502"));
    }
    let reversed = rejected_course_insert(
        &pool,
        tenant,
        Some("2026-12-19"),
        Some("2026-12-18"),
        Some("America/Chicago"),
    )
    .await;
    assert_eq!(constraint(&reversed), Some("course_term_order_check"));
    let out_of_range = rejected_course_insert(
        &pool,
        tenant,
        Some("9999-12-31"),
        Some("10000-01-01"),
        Some("America/Chicago"),
    )
    .await;
    assert_eq!(
        constraint(&out_of_range),
        Some("course_term_end_date_bounds_check")
    );
    let overlong_zone = "A".repeat(256);
    for invalid_zone in [
        "",
        "\t",
        " America/Chicago",
        "America/Chicago ",
        "America/\nChicago",
        &overlong_zone,
    ] {
        let error = rejected_course_insert(
            &pool,
            tenant,
            Some("2026-08-24"),
            Some("2026-12-18"),
            Some(invalid_zone),
        )
        .await;
        assert_eq!(constraint(&error), Some("course_time_zone_shape_check"));
    }

    let force_rls: bool = sqlx::query_scalar(
        "SELECT relforcerowsecurity FROM pg_class WHERE oid = 'public.course'::regclass",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect forced course RLS");
    assert!(force_rls);
    let mut foreign = pool.begin().await.expect("begin foreign RLS probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *foreign)
        .await
        .expect("foreign probe assumes application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(foreign_tenant.to_string())
        .execute(&mut *foreign)
        .await
        .expect("foreign probe sets tenant context");
    let visible: i64 = sqlx::query("SELECT count(*) AS count FROM course WHERE course_id = $1")
        .bind(course.as_uuid())
        .fetch_one(&mut *foreign)
        .await
        .expect("foreign course query is authorized but filtered")
        .try_get("count")
        .expect("count column");
    assert_eq!(visible, 0);
    foreign.rollback().await.expect("foreign probe rolls back");

    let corrupt_course = CourseId::from_uuid(id());
    sqlx::query(
        "INSERT INTO course (tenant_id, course_id, title, term_start_date, term_end_date, \
         time_zone) VALUES ($1, $2, 'Corrupt zone fixture', DATE '2026-08-24', \
         DATE '2026-12-18', 'Mars/Olympus')",
    )
    .bind(tenant.as_uuid())
    .bind(corrupt_course.as_uuid())
    .execute(&pool)
    .await
    .expect("database owner can manufacture shape-valid corrupt IANA evidence");
    assert!(matches!(
        store.get_course(context, corrupt_course).await,
        Err(StoreError::Unavailable(_))
    ));
}
