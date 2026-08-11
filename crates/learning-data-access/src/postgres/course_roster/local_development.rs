//! PostgreSQL-only local-development roster activation.

use question_model::{CourseId, StudentId, TenantId, UserId};
use sqlx::{Postgres, Transaction};

use super::super::map_sqlx_error;
use super::{
    CourseMemberId, CourseRosterMember, StoreError, decode_member, load_member_by_user_optional,
};

pub(super) async fn upsert_local_development_member(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
    student: StudentId,
    display_name: &str,
) -> Result<CourseRosterMember, StoreError> {
    let display_name = crate::validated_account_display_name(display_name)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let member = load_member_by_user_optional(transaction, tenant, course, user).await?;
    let member_id = member
        .as_ref()
        .map_or_else(CourseMemberId::generate, |member| Ok(member.id))?;
    let row = sqlx::query(
        "INSERT INTO course_roster_member \
         (tenant_id, course_id, course_member_id, user_id, student_id, display_name, \
          source, status, joined_at) \
         VALUES ($1, $2, $3, $4, $5, $6, 'local_development', 'active', \
                 transaction_timestamp()) \
         ON CONFLICT (tenant_id, course_id, user_id) DO UPDATE SET \
             student_id = EXCLUDED.student_id, display_name = EXCLUDED.display_name, \
             roster_email_normalized = NULL, roster_email_delivery = NULL, roster_id = NULL, \
             source = 'local_development', status = 'active', revoked_at = NULL \
         RETURNING course_member_id, user_id, student_id, display_name, \
                   roster_email_normalized AS normalized_email, \
                   roster_email_delivery AS delivery_email, roster_id, source, status, \
                   floor(extract(epoch FROM joined_at) * 1000)::bigint AS created_at_millis, \
                   floor(extract(epoch FROM revoked_at) * 1000)::bigint AS revoked_at_millis",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(member_id.as_uuid())
    .bind(user.as_uuid())
    .bind(student.as_uuid())
    .bind(display_name)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    decode_member(&row, tenant, course)
}
