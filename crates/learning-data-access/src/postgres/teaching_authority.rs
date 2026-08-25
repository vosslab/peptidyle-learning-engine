//! PostgreSQL T2 operator-eligibility and target-bound co-instructor flows.

use async_trait::async_trait;
use question_model::{
    ActivityTimestamp, CoInstructorInvitation, CoInstructorInvitationId, CourseId,
    CourseMembershipId, InstructorApproval, TenantId, UserId,
};
use sqlx::postgres::PgRow;
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::{PostgresStore, map_sqlx_error, page_from_keyed_records, retry_transaction};
use crate::{
    ApproveInstructorAccount, CoInstructorInvitationRevision, CreateCoInstructorInvitation,
    DirectInstructorMembershipView, InstructorApprovalRevision, Page, PageRequest,
    RemoveDirectInstructorMembership, RespondToCoInstructorInvitation,
    RevokeCoInstructorInvitation, RevokeInstructorApproval, RosterRevision, SessionTokenHash,
    StoreError, StoredCoInstructorInvitation, StoredInstructorApproval, TeachingAuthorityStore,
    TenantContext,
};

fn map_invitation_mutator_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("23505")
        && database_error.message() == "co-instructor invitation revision conflicts"
    {
        return StoreError::Conflict;
    }
    map_sqlx_error(error)
}

#[async_trait]
impl TeachingAuthorityStore for PostgresStore {
    async fn approve_instructor_account(
        &self,
        context: TenantContext,
        command: ApproveInstructorAccount,
    ) -> Result<StoredInstructorApproval, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(concat!(
            "SELECT user_id, approved_by, floor(extract(epoch FROM approved_at) * 1000)::bigint ",
            "AS approved_at_millis, floor(extract(epoch FROM revoked_at) * 1000)::bigint ",
            "AS revoked_at_millis, revision FROM public.ple_sysadmin_instructor_approval",
            "($1, $2, $3)",
        ))
        .bind(command.session.to_string())
        .bind(command.target.as_uuid())
        .bind(
            command
                .expected_revision
                .map(InstructorApprovalRevision::as_i64),
        )
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let row = require_row(row)?;
        let stored = decode_approval(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(stored)
    }

    async fn revoke_instructor_approval(
        &self,
        context: TenantContext,
        command: RevokeInstructorApproval,
    ) -> Result<StoredInstructorApproval, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(concat!(
            "SELECT user_id, approved_by, floor(extract(epoch FROM approved_at) * 1000)::bigint ",
            "AS approved_at_millis, floor(extract(epoch FROM revoked_at) * 1000)::bigint ",
            "AS revoked_at_millis, revision FROM public.ple_sysadmin_revoke_instructor_approval",
            "($1, $2, $3)",
        ))
        .bind(command.session.to_string())
        .bind(command.target.as_uuid())
        .bind(command.expected_revision.as_i64())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let row = require_row(row)?;
        let stored = decode_approval(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(stored)
    }

    async fn create_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: CreateCoInstructorInvitation,
    ) -> Result<StoredCoInstructorInvitation, StoreError> {
        retry_transaction(|| async {
            let tenant = context.tenant_id();
            let mut transaction = self.begin_tenant(context).await?;
            let row = sqlx::query(
                "SELECT * FROM public.ple_create_co_instructor_invitation_v1($1,$2,$3,$4)",
            )
            .bind(tenant.as_uuid())
            .bind(command.session.to_string())
            .bind(command.course.as_uuid())
            .bind(command.target.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
            validate_invitation_capability_identity(
                &row,
                tenant,
                command.actor,
                command.course,
                None,
            )?;
            let stored = decode_invitation(&row)?;
            if stored.invitation.target != command.target
                || stored.invitation.invited_by.as_uuid().is_nil()
            {
                return Err(StoreError::Unavailable(
                    "co-instructor creation capability witness is invalid".to_string(),
                ));
            }
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(stored)
        })
        .await
    }

    async fn list_course_co_instructor_invitations(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<StoredCoInstructorInvitation>, StoreError> {
        let mut transaction = self.begin_tenant_snapshot(context).await?;
        require_direct_instructor(&mut transaction, context.tenant_id(), course, actor).await?;
        let after = page
            .after
            .as_ref()
            .map(|cursor| cursor.as_str().to_string());
        let rows = sqlx::query(concat!(
            "SELECT invitation_id, course_id, target_user_id, invited_by_membership_id, ",
            "floor(extract(epoch FROM created_at)*1000)::bigint AS created_at_millis, ",
            "floor(extract(epoch FROM expires_at)*1000)::bigint AS expires_at_millis, ",
            "floor(extract(epoch FROM accepted_at)*1000)::bigint AS accepted_at_millis, ",
            "floor(extract(epoch FROM declined_at)*1000)::bigint AS declined_at_millis, ",
            "floor(extract(epoch FROM revoked_at)*1000)::bigint AS revoked_at_millis, ",
            "CASE WHEN status = 'pending' AND expires_at <= transaction_timestamp() THEN ",
            "'expired' ELSE status END AS status, revision FROM course_instructor_invitation ",
            "WHERE tenant_id = $1 AND course_id = $2 AND ($3::text IS NULL OR ",
            "invitation_id::text > $3) ORDER BY invitation_id LIMIT $4",
        ))
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(after)
        .bind(i64::from(page.size.get()) + 1)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = rows
            .iter()
            .map(|row| {
                Ok((
                    row.try_get::<Uuid, _>("invitation_id")
                        .map_err(map_sqlx_error)?
                        .to_string(),
                    decode_invitation(row)?,
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn list_pending_co_instructor_invitations(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<StoredCoInstructorInvitation>, StoreError> {
        let mut transaction = self.begin_tenant_snapshot(context).await?;
        let target = session_subject(&mut transaction, context.tenant_id(), session).await?;
        let after = page
            .after
            .as_ref()
            .map(|cursor| cursor.as_str().to_string());
        let rows = sqlx::query(concat!(
            "SELECT invitation_id, course_id, target_user_id, invited_by_membership_id, ",
            "floor(extract(epoch FROM created_at)*1000)::bigint AS created_at_millis, ",
            "floor(extract(epoch FROM expires_at)*1000)::bigint AS expires_at_millis, ",
            "floor(extract(epoch FROM accepted_at)*1000)::bigint AS accepted_at_millis, ",
            "floor(extract(epoch FROM declined_at)*1000)::bigint AS declined_at_millis, ",
            "floor(extract(epoch FROM revoked_at)*1000)::bigint AS revoked_at_millis, status, ",
            "revision FROM course_instructor_invitation WHERE tenant_id = $1 ",
            "AND target_user_id = $2 ",
            "AND status = 'pending' AND expires_at > transaction_timestamp() AND ",
            "($3::text IS NULL OR invitation_id::text > $3) ORDER BY invitation_id LIMIT $4",
        ))
        .bind(context.tenant_id().as_uuid())
        .bind(target.as_uuid())
        .bind(after)
        .bind(i64::from(page.size.get()) + 1)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = rows
            .iter()
            .map(|row| {
                Ok((
                    row.try_get::<Uuid, _>("invitation_id")
                        .map_err(map_sqlx_error)?
                        .to_string(),
                    decode_invitation(row)?,
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn accept_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: RespondToCoInstructorInvitation,
    ) -> Result<DirectInstructorMembershipView, StoreError> {
        retry_transaction(|| async {
            let tenant = context.tenant_id();
            let mut transaction = self.begin_tenant(context).await?;
            let row = sqlx::query(
                "SELECT tenant_id, actor_id, course_id, course_membership_id, roster_revision \
                 FROM public.ple_accept_co_instructor_invitation_v1($1,$2,$3,$4)",
            )
            .bind(tenant.as_uuid())
            .bind(command.session.to_string())
            .bind(command.invitation.as_uuid())
            .bind(command.expected_revision.as_i64())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
            let returned_tenant =
                TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?);
            let returned_actor =
                UserId::from_uuid(row.try_get("actor_id").map_err(map_sqlx_error)?);
            let course = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
            let membership = CourseMembershipId::from_uuid(
                row.try_get("course_membership_id")
                    .map_err(map_sqlx_error)?,
            );
            let revision = RosterRevision::from_stored(
                row.try_get("roster_revision").map_err(map_sqlx_error)?,
            )?;
            if returned_tenant != tenant || returned_actor != command.actor {
                return Err(StoreError::Unavailable(
                    "co-instructor acceptance capability witness is invalid".to_string(),
                ));
            }
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(DirectInstructorMembershipView {
                membership,
                course,
                user: command.actor,
                roster_revision: revision,
            })
        })
        .await
    }

    async fn decline_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: RespondToCoInstructorInvitation,
    ) -> Result<(), StoreError> {
        target_terminal(self, context, command).await
    }

    async fn revoke_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: RevokeCoInstructorInvitation,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT * FROM public.ple_revoke_co_instructor_invitation_v1($1,$2,$3,$4,$5)",
        )
        .bind(tenant.as_uuid())
        .bind(command.session.to_string())
        .bind(command.course.as_uuid())
        .bind(command.invitation.as_uuid())
        .bind(command.expected_revision.as_i64())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_invitation_mutator_error)?
        .ok_or(StoreError::NotFound)?;
        validate_invitation_capability_identity(
            &row,
            tenant,
            command.actor,
            command.course,
            Some(command.invitation),
        )?;
        let revision: i64 = row.try_get("revision").map_err(map_sqlx_error)?;
        if revision != command.expected_revision.as_i64().saturating_add(1) {
            return Err(StoreError::Unavailable(
                "co-instructor revocation capability witness is invalid".to_string(),
            ));
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn remove_direct_instructor_membership(
        &self,
        context: TenantContext,
        command: RemoveDirectInstructorMembership,
    ) -> Result<(), StoreError> {
        retry_transaction(|| async {
            let tenant = context.tenant_id();
            let mut transaction = self.begin_tenant(context).await?;
            let expected_roster_revision = i64::try_from(command.expected_roster_revision.value())
                .map_err(|_| StoreError::Conflict)?;
            let row = sqlx::query(
                "SELECT * FROM public.ple_remove_direct_instructor_membership_v1($1,$2,$3,$4,$5)",
            )
            .bind(tenant.as_uuid())
            .bind(command.session.to_string())
            .bind(command.course.as_uuid())
            .bind(command.membership.as_uuid())
            .bind(expected_roster_revision)
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            let returned_tenant: Uuid = row.try_get("tenant_id").map_err(map_sqlx_error)?;
            let returned_actor: Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
            let returned_course: Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
            let returned_membership: Uuid = row
                .try_get("course_membership_id")
                .map_err(map_sqlx_error)?;
            let returned_revision: i64 = row.try_get("roster_revision").map_err(map_sqlx_error)?;
            if returned_tenant != tenant.as_uuid()
                || returned_actor != command.actor.as_uuid()
                || returned_course != command.course.as_uuid()
                || returned_membership != command.membership.as_uuid()
                || RosterRevision::from_stored(returned_revision)?
                    != command.expected_roster_revision.next()?
            {
                return Err(StoreError::Conflict);
            }
            transaction.commit().await.map_err(map_sqlx_error)
        })
        .await
    }
}

async fn target_terminal(
    store: &PostgresStore,
    context: TenantContext,
    command: RespondToCoInstructorInvitation,
) -> Result<(), StoreError> {
    let tenant = context.tenant_id();
    let mut transaction = store.begin_tenant(context).await?;
    let row =
        sqlx::query("SELECT * FROM public.ple_decline_co_instructor_invitation_v1($1,$2,$3,$4)")
            .bind(tenant.as_uuid())
            .bind(command.session.to_string())
            .bind(command.invitation.as_uuid())
            .bind(command.expected_revision.as_i64())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_invitation_mutator_error)?
            .ok_or(StoreError::NotFound)?;
    let course = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
    validate_invitation_capability_identity(
        &row,
        tenant,
        command.actor,
        course,
        Some(command.invitation),
    )?;
    let revision: i64 = row.try_get("revision").map_err(map_sqlx_error)?;
    if revision != command.expected_revision.as_i64().saturating_add(1) {
        return Err(StoreError::Unavailable(
            "co-instructor decline capability witness is invalid".to_string(),
        ));
    }
    transaction.commit().await.map_err(map_sqlx_error)
}

fn validate_invitation_capability_identity(
    row: &PgRow,
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    invitation: Option<CoInstructorInvitationId>,
) -> Result<(), StoreError> {
    let returned_tenant = TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?);
    let returned_actor = UserId::from_uuid(row.try_get("actor_id").map_err(map_sqlx_error)?);
    let returned_course = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
    let returned_invitation =
        CoInstructorInvitationId::from_uuid(row.try_get("invitation_id").map_err(map_sqlx_error)?);
    if returned_tenant != tenant
        || returned_actor != actor
        || returned_course != course
        || returned_invitation.as_uuid().is_nil()
        || invitation.is_some_and(|expected| expected != returned_invitation)
    {
        return Err(StoreError::Unavailable(
            "co-instructor invitation capability witness is invalid".to_string(),
        ));
    }
    Ok(())
}

fn decode_approval(row: &PgRow) -> Result<StoredInstructorApproval, StoreError> {
    Ok(StoredInstructorApproval {
        approval: InstructorApproval {
            user: UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_error)?),
            approved_by: UserId::from_uuid(row.try_get("approved_by").map_err(map_sqlx_error)?),
            approved_at: stamp(row, "approved_at_millis")?,
            revoked_at: optional_stamp(row, "revoked_at_millis")?,
        },
        revision: InstructorApprovalRevision::try_from_i64(
            row.try_get("revision").map_err(map_sqlx_error)?,
        )?,
    })
}

fn require_row<T>(row: Option<T>) -> Result<T, StoreError> {
    row.ok_or(StoreError::NotFound)
}

fn decode_invitation(row: &PgRow) -> Result<StoredCoInstructorInvitation, StoreError> {
    let status: String = row.try_get("status").map_err(map_sqlx_error)?;
    let invitation = CoInstructorInvitation {
        id: CoInstructorInvitationId::from_uuid(
            row.try_get("invitation_id").map_err(map_sqlx_error)?,
        ),
        course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        target: UserId::from_uuid(row.try_get("target_user_id").map_err(map_sqlx_error)?),
        invited_by: CourseMembershipId::from_uuid(
            row.try_get("invited_by_membership_id")
                .map_err(map_sqlx_error)?,
        ),
        created_at: stamp(row, "created_at_millis")?,
        expires_at: stamp(row, "expires_at_millis")?,
        accepted_at: optional_stamp(row, "accepted_at_millis")?,
        declined_at: optional_stamp(row, "declined_at_millis")?,
        revoked_at: optional_stamp(row, "revoked_at_millis")?,
    };
    if !stored_status_matches_terminals(
        &status,
        invitation.accepted_at.is_some(),
        invitation.declined_at.is_some(),
        invitation.revoked_at.is_some(),
    ) {
        return Err(StoreError::InvalidRecord(
            "stored co-instructor invitation lifecycle is invalid".to_string(),
        ));
    }
    Ok(StoredCoInstructorInvitation {
        invitation,
        revision: CoInstructorInvitationRevision::try_from_i64(
            row.try_get("revision").map_err(map_sqlx_error)?,
        )?,
    })
}

fn stored_status_matches_terminals(
    status: &str,
    accepted: bool,
    declined: bool,
    revoked: bool,
) -> bool {
    matches!(
        (status, accepted, declined, revoked),
        ("pending" | "expired", false, false, false)
            | ("accepted", true, false, false)
            | ("declined", false, true, false)
            | ("revoked", false, false, true)
    )
}
fn stamp(row: &PgRow, column: &str) -> Result<ActivityTimestamp, StoreError> {
    Ok(ActivityTimestamp::from_unix_millis(
        row.try_get(column).map_err(map_sqlx_error)?,
    ))
}
fn optional_stamp(row: &PgRow, column: &str) -> Result<Option<ActivityTimestamp>, StoreError> {
    Ok(row
        .try_get::<Option<i64>, _>(column)
        .map_err(map_sqlx_error)?
        .map(ActivityTimestamp::from_unix_millis))
}
async fn require_direct_instructor(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> Result<CourseMembershipId, StoreError> {
    let id: Option<Uuid> = sqlx::query_scalar(concat!(
        "SELECT course_membership_id FROM course_member WHERE tenant_id=$1 AND course_id=$2 ",
        "AND user_id=$3 AND role='instructor' AND status='active'",
    ))
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(user.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    id.map(CourseMembershipId::from_uuid)
        .ok_or(StoreError::NotFound)
}

async fn session_subject(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    session: SessionTokenHash,
) -> Result<UserId, StoreError> {
    sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
        .bind(session.to_string())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    let user: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM public.ple_target_session_subject($1, $2)")
            .bind(session.to_string())
            .bind(tenant.as_uuid())
            .fetch_optional(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    user.map(UserId::from_uuid).ok_or(StoreError::NotFound)
}
#[cfg(test)]
mod tests {
    use super::{require_row, stored_status_matches_terminals};
    use crate::StoreError;

    #[test]
    fn zero_row_sysadmin_functions_map_to_not_found() {
        assert!(matches!(require_row::<()>(None), Err(StoreError::NotFound)));
    }

    #[test]
    fn stored_invitation_status_accepts_only_closed_lifecycle_shapes() {
        for (status, accepted, declined, revoked) in [
            ("pending", false, false, false),
            ("expired", false, false, false),
            ("accepted", true, false, false),
            ("declined", false, true, false),
            ("revoked", false, false, true),
        ] {
            assert!(stored_status_matches_terminals(
                status, accepted, declined, revoked
            ));
        }
        assert!(!stored_status_matches_terminals(
            "pending", true, false, false
        ));
        assert!(!stored_status_matches_terminals(
            "other", false, false, false
        ));
    }
}
