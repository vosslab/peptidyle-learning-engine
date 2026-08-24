//! PostgreSQL-specific authority probes isolated from the lifecycle workflow.

use super::*;

async fn app_transaction<'a>(pool: &'a PgPool, tenant: TenantId) -> Transaction<'a, Postgres> {
    let mut transaction = pool
        .begin()
        .await
        .expect("start application authority probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("set ple_app");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(tenant.as_uuid().to_string())
        .execute(&mut *transaction)
        .await
        .expect("set tenant context");
    transaction
}

pub(super) async fn assert_application_authority_catalog(
    pool: &PgPool,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    student: UserId,
    attempt: QuestionAttemptId,
) {
    let grants: (bool, bool, bool) = sqlx::query_as(
        "SELECT has_table_privilege('ple_app','public.question_attempt','SELECT'), \
                has_table_privilege('ple_app','public.course','UPDATE'), \
                has_function_privilege('ple_app','public.ple_prepare_attempt_work(uuid,uuid,uuid,uuid,uuid,text)','EXECUTE')",
    )
    .fetch_one(pool)
    .await
    .expect("authority catalog rows");
    assert!(
        grants.0,
        "post-broker immutable hydration retains tenant-scoped plain SELECT"
    );
    assert!(
        !grants.1,
        "ple_app has no course source-graph mutation authority"
    );
    assert!(
        grants.2,
        "ple_app receives only the execute-only 1817 preparation capability"
    );

    let mut no_tenant = pool.begin().await.expect("begin no-tenant RLS probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *no_tenant)
        .await
        .expect("set app for RLS probe");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM public.question_attempt")
        .fetch_one(&mut *no_tenant)
        .await
        .expect("RLS empty result is readable");
    assert_eq!(
        rows, 0,
        "without tenant context RLS returns no issued attempts"
    );
    no_tenant.rollback().await.expect("end no-tenant RLS probe");

    let mut application = app_transaction(pool, tenant).await;
    let prepared: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.ple_prepare_attempt_work($1,$2,$3,$4,$5,'student_self')",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(assignment.as_uuid())
    .bind(student.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_one(&mut *application)
    .await
    .expect("execute-only 1817 wrapper authorizes exact student attempt");
    assert_eq!(
        prepared, 1,
        "1817 returns exactly one typed attempt witness"
    );
    let lock = sqlx::query(
        "SELECT course_id FROM public.course WHERE tenant_id=$1 AND course_id=$2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .execute(&mut *application)
    .await
    .expect_err("ple_app cannot independently lock a source course");
    assert_eq!(
        lock.as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("42501")
    );
    application
        .rollback()
        .await
        .expect("discard application authority probe");
}
