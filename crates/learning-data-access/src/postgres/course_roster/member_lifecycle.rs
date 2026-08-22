//! PostgreSQL learner identity and active course-roster member projections.

use question_model::{CourseId, StudentId, TenantId, UserId};
use sqlx::types::Uuid;
use sqlx::{Postgres, Transaction};

use super::super::{course_roster_decode::decode_member, map_sqlx_error};
use crate::{CourseRosterContact, CourseRosterMember, StoreError};

pub(super) async fn upsert_course_member_record(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
    student: StudentId,
    display_name: &str,
    roster_contact: Option<&CourseRosterContact>,
) -> Result<CourseRosterMember, StoreError> {
    let display_name = crate::validated_account_display_name(display_name)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    upsert_student_membership(
        transaction,
        tenant,
        course,
        user,
        student,
        &display_name,
        roster_contact.map(|contact| contact.email.normalized()),
        roster_contact.map(|contact| contact.email.delivery()),
        roster_contact.map(|contact| contact.roster_id.as_str()),
    )
    .await
}

pub(super) async fn resolve_learner(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    user: UserId,
) -> Result<StudentId, StoreError> {
    let student = StudentId::from_uuid(random_uuid("student ID")?);
    let resolved: Uuid = sqlx::query_scalar(
        "INSERT INTO tenant_learner_identity (tenant_id, user_id, student_id) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (tenant_id, user_id) DO UPDATE SET user_id = EXCLUDED.user_id \
         RETURNING student_id",
    )
    .bind(tenant.as_uuid())
    .bind(user.as_uuid())
    .bind(student.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(StudentId::from_uuid(resolved))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn upsert_claimed_member(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
    student: StudentId,
    display_name: &str,
    normalized_email: &str,
    delivery_email: &str,
    roster_id: &str,
) -> Result<CourseRosterMember, StoreError> {
    let display_name = crate::validated_account_display_name(display_name)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    upsert_student_membership(
        transaction,
        tenant,
        course,
        user,
        student,
        &display_name,
        Some(normalized_email),
        Some(delivery_email),
        Some(roster_id),
    )
    .await
}

pub(super) async fn load_member_by_user(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> Result<CourseRosterMember, StoreError> {
    load_member_by_user_optional(transaction, tenant, course, user)
        .await?
        .ok_or(StoreError::NotFound)
}

pub(super) async fn load_member_by_user_optional(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> Result<Option<CourseRosterMember>, StoreError> {
    let row = sqlx::query(
        "SELECT membership.course_membership_id AS record_id, membership.user_id, \
                membership.student_id, profile.display_name, \
                profile.roster_email_normalized AS normalized_email, \
                profile.roster_email_delivery AS delivery_email, membership.roster_id, membership.status, \
                floor(extract(epoch FROM membership.joined_at) * 1000)::bigint AS created_at_millis, \
                floor(extract(epoch FROM membership.revoked_at) * 1000)::bigint AS revoked_at_millis \
         FROM course_member membership \
         JOIN course_roster_profile profile \
           ON profile.tenant_id = membership.tenant_id AND profile.course_id = membership.course_id \
          AND profile.course_membership_id = membership.course_membership_id \
         WHERE membership.tenant_id = $1 AND membership.course_id = $2 AND membership.user_id = $3 \
           AND membership.status = 'active'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(user.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    row.as_ref()
        .map(|row| decode_member(row, tenant, course))
        .transpose()
}

#[allow(clippy::too_many_arguments)]
async fn upsert_student_membership(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
    student: StudentId,
    display_name: &str,
    normalized_email: Option<&str>,
    delivery_email: Option<&str>,
    roster_id: Option<&str>,
) -> Result<CourseRosterMember, StoreError> {
    let membership_id = random_uuid("course membership ID")?;
    let membership: Uuid = sqlx::query_scalar(
        "INSERT INTO course_member \
         (tenant_id, course_id, course_membership_id, user_id, role, student_id, roster_id, status, joined_at) \
         VALUES ($1, $2, $3, $4, 'student', $5, $6, 'active', transaction_timestamp()) \
         RETURNING course_membership_id",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(membership_id)
    .bind(user.as_uuid())
    .bind(student.as_uuid())
    .bind(roster_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "INSERT INTO course_roster_profile \
         (tenant_id, course_id, course_membership_id, display_name, roster_email_normalized, \
          roster_email_delivery) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(membership)
    .bind(display_name)
    .bind(normalized_email)
    .bind(delivery_email)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    load_member_by_user(transaction, tenant, course, user).await
}

fn random_uuid(label: &str) -> Result<Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("{label} randomness unavailable: {error}"))
    })
}
