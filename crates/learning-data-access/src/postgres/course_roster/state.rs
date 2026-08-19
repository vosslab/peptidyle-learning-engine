//! Locked course-roster state primitives shared by roster workflows.

use question_model::{CourseId, TenantId};
use sqlx::{Postgres, Transaction};

use super::map_sqlx_error;
use crate::StoreError;

pub(in crate::postgres) async fn ensure_roster_state(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO course_roster_state (tenant_id, course_id) VALUES ($1, $2) \
         ON CONFLICT (tenant_id, course_id) DO NOTHING",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

pub(in crate::postgres) async fn lock_course_roster_cross_product(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<(), StoreError> {
    sqlx::query("SELECT 1 FROM course WHERE tenant_id = $1 AND course_id = $2 FOR UPDATE")
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
    Ok(())
}
