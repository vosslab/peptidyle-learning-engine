//! Checked witnesses for closed learner-invitation aggregate capabilities.

use question_model::{CourseId, TenantId};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::super::{
    course_roster_decode::{decode_invitation, decode_member},
    map_sqlx_error,
};
use crate::{
    ClaimCourseInvitation, ClaimedCourseMembership, CourseInvitation, CourseInvitationId,
    CourseMemberStatus, CreateCourseInvitation, RevokeCourseInvitation, RosterRevision,
    SessionTokenHash, StoreError,
};

pub(super) async fn create(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    session: SessionTokenHash,
    command: &CreateCourseInvitation,
) -> Result<CourseInvitation, StoreError> {
    let row = sqlx::query(
        "SELECT invitation_id, normalized_email, delivery_email, roster_id, invited_by, status, \
                claimed_user_id, created_at_millis, expires_at_millis, roster_revision \
         FROM public.ple_create_course_invitation_v1($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(tenant.as_uuid())
    .bind(session.to_string())
    .bind(command.course.as_uuid())
    .bind(CourseInvitationId::generate()?.as_uuid())
    .bind(command.token_hash.as_bytes().to_vec())
    .bind(command.email.normalized())
    .bind(command.email.delivery())
    .bind(command.roster_id.as_str())
    .bind(command.idempotency_key.as_str())
    .bind(i64::from(command.lifetime.as_seconds()))
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let revision =
        RosterRevision::from_stored(row.try_get("roster_revision").map_err(map_sqlx_error)?)?;
    if revision.value() == 0 {
        return Err(StoreError::Unavailable(
            "invitation capability returned an invalid revision".to_string(),
        ));
    }
    let invitation = decode_invitation(&row, tenant, command.course)?;
    if invitation.email != command.email
        || invitation.roster_id != command.roster_id
        || invitation.invited_by.as_uuid().is_nil()
        || invitation.status != crate::CourseInvitationStatus::Pending
    {
        return Err(StoreError::Unavailable(
            "invitation capability returned an invalid create witness".to_string(),
        ));
    }
    Ok(invitation)
}

pub(super) async fn claim(
    transaction: &mut Transaction<'_, Postgres>,
    command: &ClaimCourseInvitation,
) -> Result<ClaimedCourseMembership, StoreError> {
    let row = sqlx::query(
        "SELECT tenant_id, course_id, invitation_id, claimed_user_id, student_id, \
                record_id, user_id, member_role, status, roster_id, created_at_millis, \
                revoked_at_millis, display_name, normalized_email, delivery_email, \
                invitation_status, invitation_claimed_user_id, replayed, delivery_state, \
                delivery_outcome_code, delivery_terminal_at_millis, delivery_accepted_at_millis, \
                roster_revision \
         FROM public.ple_claim_course_invitation_v1($1,$2,$3,$4,$5)",
    )
    .bind(command.token_hash.as_bytes().to_vec())
    .bind(command.user.as_uuid())
    .bind(command.verified_email.normalized())
    .bind(command.verified_email.delivery())
    .bind(&command.display_name)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let invalid = || {
        StoreError::Unavailable(
            "invitation claim returned an invalid authority witness".to_string(),
        )
    };
    let tenant_uuid = row
        .try_get::<Option<Uuid>, _>("tenant_id")
        .map_err(map_sqlx_error)?
        .ok_or_else(invalid)?;
    let course_uuid = row
        .try_get::<Option<Uuid>, _>("course_id")
        .map_err(map_sqlx_error)?
        .ok_or_else(invalid)?;
    let invitation = row
        .try_get::<Option<Uuid>, _>("invitation_id")
        .map_err(map_sqlx_error)?
        .ok_or_else(invalid)?;
    let claimed_user = row
        .try_get::<Option<Uuid>, _>("claimed_user_id")
        .map_err(map_sqlx_error)?
        .ok_or_else(invalid)?;
    let invitation_claimed_user = row
        .try_get::<Option<Uuid>, _>("invitation_claimed_user_id")
        .map_err(map_sqlx_error)?
        .ok_or_else(invalid)?;
    let replayed = row
        .try_get::<Option<bool>, _>("replayed")
        .map_err(map_sqlx_error)?
        .ok_or_else(invalid)?;
    let delivery_state = row
        .try_get::<Option<String>, _>("delivery_state")
        .map_err(map_sqlx_error)?
        .ok_or_else(invalid)?;
    let delivery_outcome = row
        .try_get::<Option<String>, _>("delivery_outcome_code")
        .map_err(map_sqlx_error)?;
    let delivery_terminal_at = row
        .try_get::<Option<i64>, _>("delivery_terminal_at_millis")
        .map_err(map_sqlx_error)?;
    let delivery_accepted_at = row
        .try_get::<Option<i64>, _>("delivery_accepted_at_millis")
        .map_err(map_sqlx_error)?;
    let invitation_status = row
        .try_get::<Option<String>, _>("invitation_status")
        .map_err(map_sqlx_error)?
        .ok_or_else(invalid)?;
    let member_role = row
        .try_get::<Option<String>, _>("member_role")
        .map_err(map_sqlx_error)?
        .ok_or_else(invalid)?;
    let revoked_at = row
        .try_get::<Option<i64>, _>("revoked_at_millis")
        .map_err(map_sqlx_error)?;
    let revision =
        RosterRevision::from_stored(row.try_get("roster_revision").map_err(map_sqlx_error)?)?;
    let tenant = TenantId::from_uuid(tenant_uuid);
    let course = CourseId::from_uuid(course_uuid);
    let member = decode_member(&row, tenant, course)?;
    if tenant_uuid.is_nil()
        || course_uuid.is_nil()
        || invitation.is_nil()
        || claimed_user != command.user.as_uuid()
        || invitation_claimed_user != command.user.as_uuid()
        || member.user != command.user
        || member.student.as_uuid().is_nil()
        || member.id.as_uuid().is_nil()
        || member_role != "student"
        || member.status != CourseMemberStatus::Active
        || revoked_at.is_some()
        || member.display_name != command.display_name
        || member.roster_email.as_ref() != Some(&command.verified_email)
        || member.roster_id.is_none()
        || invitation_status != "claimed"
        || !valid_terminal_delivery(
            &delivery_state,
            delivery_outcome.as_deref(),
            delivery_terminal_at,
            delivery_accepted_at,
        )
        || revision.value() == 0
        || (!replayed && revision.value() < 2)
    {
        return Err(invalid());
    }
    Ok(ClaimedCourseMembership {
        tenant,
        course,
        member,
        roster_revision: revision,
    })
}

fn valid_terminal_delivery(
    state: &str,
    outcome: Option<&str>,
    terminal_at: Option<i64>,
    accepted_at: Option<i64>,
) -> bool {
    match state {
        "cancelled" => {
            outcome == Some("cancelled") && terminal_at.is_some() && accepted_at.is_none()
        }
        "accepted_by_provider" => {
            outcome == Some("accepted") && terminal_at.is_some() && accepted_at.is_some()
        }
        "permanent_failed" => {
            outcome == Some("permanent_failure") && terminal_at.is_some() && accepted_at.is_none()
        }
        "ambiguous" => {
            outcome == Some("ambiguous_transport") && terminal_at.is_some() && accepted_at.is_none()
        }
        _ => false,
    }
}

pub(super) async fn revoke(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    session: SessionTokenHash,
    command: RevokeCourseInvitation,
) -> Result<RosterRevision, StoreError> {
    let row = sqlx::query(
        "SELECT tenant_id, actor_id, course_id, invitation_id, was_revoked, roster_revision \
         FROM public.ple_revoke_course_invitation_v1($1,$2,$3,$4,$5)",
    )
    .bind(tenant.as_uuid())
    .bind(session.to_string())
    .bind(command.course.as_uuid())
    .bind(command.invitation.as_uuid())
    .bind(i64::try_from(command.expected_revision.value()).map_err(|_| StoreError::Conflict)?)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let actual_tenant: Uuid = row.try_get("tenant_id").map_err(map_sqlx_error)?;
    let actual_course: Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
    let invitation: Uuid = row.try_get("invitation_id").map_err(map_sqlx_error)?;
    let actor = row
        .try_get::<Option<Uuid>, _>("actor_id")
        .map_err(map_sqlx_error)?;
    let replay = row
        .try_get::<Option<bool>, _>("was_revoked")
        .map_err(map_sqlx_error)?;
    let revision =
        RosterRevision::from_stored(row.try_get("roster_revision").map_err(map_sqlx_error)?)?;
    let required = if replay == Some(true) {
        command.expected_revision
    } else {
        command
            .expected_revision
            .next()
            .map_err(|_| StoreError::Conflict)?
    };
    if actual_tenant != tenant.as_uuid()
        || actual_course != command.course.as_uuid()
        || invitation != command.invitation.as_uuid()
        || actor.is_none()
        || replay.is_none()
        || revision != required
    {
        return Err(StoreError::Unavailable(
            "invitation revocation returned an invalid authority witness".to_string(),
        ));
    }
    Ok(revision)
}

pub(super) async fn claimed_membership(
    transaction: &mut Transaction<'_, Postgres>,
    command: &ClaimCourseInvitation,
) -> Result<ClaimedCourseMembership, StoreError> {
    claim(transaction, command).await
}

#[cfg(test)]
mod tests {
    use super::valid_terminal_delivery;

    #[test]
    fn terminal_delivery_witness_preserves_each_truthful_terminal_state() {
        for (state, outcome, terminal_at, accepted_at) in [
            ("cancelled", Some("cancelled"), Some(1), None),
            ("accepted_by_provider", Some("accepted"), Some(2), Some(3)),
            ("permanent_failed", Some("permanent_failure"), Some(3), None),
            ("ambiguous", Some("ambiguous_transport"), Some(4), None),
        ] {
            assert!(valid_terminal_delivery(
                state,
                outcome,
                terminal_at,
                accepted_at
            ));
        }
    }

    #[test]
    fn terminal_delivery_witness_rejects_false_cancellation_shapes() {
        assert!(!valid_terminal_delivery(
            "accepted_by_provider",
            Some("accepted"),
            Some(1),
            None
        ));
        assert!(!valid_terminal_delivery(
            "accepted_by_provider",
            Some("accepted"),
            None,
            Some(2)
        ));
        assert!(!valid_terminal_delivery(
            "permanent_failed",
            Some("permanent_failure"),
            Some(1),
            Some(2)
        ));
        assert!(!valid_terminal_delivery("pending", None, None, None));
    }
}
