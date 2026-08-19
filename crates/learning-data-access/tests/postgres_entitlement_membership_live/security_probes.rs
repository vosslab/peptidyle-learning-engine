//! PostgreSQL-only least-privilege probes for the entitlement live oracle.

use question_model::{CourseGroupId, CourseId, EnrollmentId, TenantId};
use sqlx::PgPool;

pub(super) async fn denied_write(pool: &PgPool, tenant: TenantId, sql: &'static str) {
    let mut transaction = pool.begin().await.expect("begin least-privilege probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("assume application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("scope application role to fixture tenant");
    let error = sqlx::query(sql)
        .execute(&mut *transaction)
        .await
        .expect_err("application role must not mutate immutable entitlement evidence");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("42501"),
        "least-privilege denial must come from PostgreSQL grants"
    );
    transaction
        .rollback()
        .await
        .expect("rollback denied write probe");
}

pub(super) async fn denied_scope_append(
    pool: &PgPool,
    tenant: TenantId,
    enrollment: EnrollmentId,
    course: CourseId,
    group: CourseGroupId,
) {
    let mut transaction = pool.begin().await.expect("begin sealed-scope probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("assume application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("scope application role to fixture tenant");
    let error = sqlx::query(
        "INSERT INTO enrollment_applicable_policy_scope_receipt \
         (tenant_id, enrollment_id, course_id, course_group_id, course_group_purpose) \
         VALUES ($1, $2, $3, $4, 'accommodation')",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.as_uuid())
    .bind(course.as_uuid())
    .bind(group.as_uuid())
    .execute(&mut *transaction)
    .await
    .expect_err("a completed entitlement receipt cannot acquire a later scope");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("55000"),
        "the receipt-set seal, not a shape mismatch, refuses the append"
    );
    transaction
        .rollback()
        .await
        .expect("rollback sealed-scope probe");
}
