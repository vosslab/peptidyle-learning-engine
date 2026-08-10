//! Strict decoding for protected course-roster rows.

use question_model::{ActivityTimestamp, CourseId, StudentId, TenantId, UserId};
use sqlx::Row;
use sqlx::postgres::PgRow;
use sqlx::types::Uuid;

use super::map_sqlx_error;
use crate::{
    AuthenticationEmail, CourseInvitation, CourseInvitationId, CourseInvitationStatus,
    CourseMemberId, CourseMemberStatus, CourseRosterEntry, CourseRosterId, CourseRosterMember,
    CourseSignupPosture, StoreError,
};

pub(super) fn decode_roster_entry(
    row: &PgRow,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseRosterEntry, StoreError> {
    match row
        .try_get::<String, _>("record_kind")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "member" => Ok(CourseRosterEntry::Member(decode_member(
            row, tenant, course,
        )?)),
        "invitation" => Ok(CourseRosterEntry::Invitation(decode_invitation(
            row, tenant, course,
        )?)),
        _ => Err(StoreError::Unavailable(
            "stored roster entry kind is invalid".to_string(),
        )),
    }
}

pub(super) fn decode_member(
    row: &PgRow,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseRosterMember, StoreError> {
    let normalized: Option<String> = row.try_get("normalized_email").map_err(map_sqlx_error)?;
    let delivery: Option<String> = row.try_get("delivery_email").map_err(map_sqlx_error)?;
    let roster_email = match (normalized, delivery) {
        (None, None) => None,
        (Some(normalized), Some(delivery)) => Some(decode_email(&normalized, &delivery)?),
        _ => {
            return Err(StoreError::Unavailable(
                "stored roster email is incomplete".to_string(),
            ));
        }
    };
    Ok(CourseRosterMember {
        id: CourseMemberId::from_uuid(
            row.try_get("record_id")
                .or_else(|_| row.try_get("course_member_id"))
                .map_err(map_sqlx_error)?,
        ),
        tenant,
        course,
        user: UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_error)?),
        student: StudentId::from_uuid(row.try_get("student_id").map_err(map_sqlx_error)?),
        display_name: row.try_get("display_name").map_err(map_sqlx_error)?,
        roster_email,
        roster_id: row
            .try_get::<Option<String>, _>("roster_id")
            .map_err(map_sqlx_error)?
            .map(|value| {
                CourseRosterId::parse(&value)
                    .map_err(|error| StoreError::Unavailable(error.to_string()))
            })
            .transpose()?,
        status: decode_member_status(&row.try_get::<String, _>("status").map_err(map_sqlx_error)?)?,
        joined_at: timestamp(row, "created_at_millis")?,
        revoked_at: optional_timestamp(row, "revoked_at_millis")?,
    })
}

pub(super) fn decode_invitation(
    row: &PgRow,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseInvitation, StoreError> {
    let normalized: String = row.try_get("normalized_email").map_err(map_sqlx_error)?;
    let delivery: String = row.try_get("delivery_email").map_err(map_sqlx_error)?;
    Ok(CourseInvitation {
        id: CourseInvitationId::from_uuid(
            row.try_get("record_id")
                .or_else(|_| row.try_get("invitation_id"))
                .map_err(map_sqlx_error)?,
        ),
        tenant,
        course,
        email: decode_email(&normalized, &delivery)?,
        roster_id: CourseRosterId::parse(
            &row.try_get::<String, _>("roster_id")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|error| StoreError::Unavailable(error.to_string()))?,
        invited_by: UserId::from_uuid(row.try_get("invited_by").map_err(map_sqlx_error)?),
        status: decode_invitation_status(
            &row.try_get::<String, _>("status").map_err(map_sqlx_error)?,
        )?,
        created_at: timestamp(row, "created_at_millis")?,
        expires_at: timestamp(row, "expires_at_millis")?,
        claimed_by: row
            .try_get::<Option<Uuid>, _>("claimed_user_id")
            .map_err(map_sqlx_error)?
            .map(UserId::from_uuid),
    })
}

fn decode_email(normalized: &str, delivery: &str) -> Result<AuthenticationEmail, StoreError> {
    let email = AuthenticationEmail::parse(delivery)
        .map_err(|error| StoreError::Unavailable(error.to_string()))?;
    (email.normalized() == normalized)
        .then_some(email)
        .ok_or_else(|| StoreError::Unavailable("stored roster email mismatch".to_string()))
}

pub(super) fn posture_name(posture: CourseSignupPosture) -> &'static str {
    match posture {
        CourseSignupPosture::InvitationOnly => "invitation_only",
        CourseSignupPosture::PermittedDomains => "permitted_domains",
    }
}

pub(super) fn decode_posture(value: &str) -> Result<CourseSignupPosture, StoreError> {
    match value {
        "invitation_only" => Ok(CourseSignupPosture::InvitationOnly),
        "permitted_domains" => Ok(CourseSignupPosture::PermittedDomains),
        _ => Err(StoreError::Unavailable(
            "stored signup posture is invalid".to_string(),
        )),
    }
}

fn decode_member_status(value: &str) -> Result<CourseMemberStatus, StoreError> {
    match value {
        "active" => Ok(CourseMemberStatus::Active),
        "revoked" => Ok(CourseMemberStatus::Revoked),
        _ => Err(StoreError::Unavailable(
            "stored roster status is invalid".to_string(),
        )),
    }
}

fn decode_invitation_status(value: &str) -> Result<CourseInvitationStatus, StoreError> {
    match value {
        "pending" => Ok(CourseInvitationStatus::Pending),
        "claimed" => Ok(CourseInvitationStatus::Claimed),
        "expired" => Ok(CourseInvitationStatus::Expired),
        "revoked" => Ok(CourseInvitationStatus::Revoked),
        _ => Err(StoreError::Unavailable(
            "stored invitation status is invalid".to_string(),
        )),
    }
}

fn timestamp(row: &PgRow, column: &str) -> Result<ActivityTimestamp, StoreError> {
    Ok(ActivityTimestamp::from_unix_millis(
        row.try_get(column).map_err(map_sqlx_error)?,
    ))
}

fn optional_timestamp(row: &PgRow, column: &str) -> Result<Option<ActivityTimestamp>, StoreError> {
    Ok(row
        .try_get::<Option<i64>, _>(column)
        .map_err(map_sqlx_error)?
        .map(ActivityTimestamp::from_unix_millis))
}
