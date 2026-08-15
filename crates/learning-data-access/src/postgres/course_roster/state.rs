//! Locked course-roster state primitives shared by roster workflows.

use question_model::{CourseId, TenantId, UserId};
use sqlx::{Postgres, Row, Transaction};

use super::{enrollment, map_sqlx_error};
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

pub(in crate::postgres) async fn reconcile_new_assignment(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &crate::AssignmentRecord,
) -> Result<(), StoreError> {
    let rows = sqlx::query(
        "SELECT user_id, student_id FROM course_roster_member \
         WHERE tenant_id = $1 AND course_id = $2 AND status = 'active' ORDER BY user_id",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    for row in rows {
        enrollment::insert_missing_enrollment(
            transaction,
            assignment.tenant,
            assignment.id,
            UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_error)?),
            question_model::StudentId::from_uuid(
                row.try_get("student_id").map_err(map_sqlx_error)?,
            ),
        )
        .await?;
    }
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
