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
            lock_course(&mut transaction, tenant, command.course).await?;
            let inviter =
                require_direct_instructor(&mut transaction, tenant, command.course, command.actor)
                    .await?;
            require_account(&mut transaction, command.target).await?;
            require_eligible(&mut transaction, command.target).await?;
            let invitation_id = random_uuid("co-instructor invitation ID")?;
            let row = sqlx::query(concat!(
                "INSERT INTO course_instructor_invitation (tenant_id, course_id, invitation_id, ",
                "target_user_id, invited_by_membership_id) VALUES ($1, $2, $3, $4, $5) ",
                "ON CONFLICT (tenant_id, course_id, target_user_id) WHERE status = 'pending' ",
                "DO NOTHING RETURNING invitation_id, course_id, target_user_id, ",
                "invited_by_membership_id, floor(extract(epoch FROM created_at) * 1000)::bigint ",
                "AS created_at_millis, floor(extract(epoch FROM expires_at) * 1000)::bigint ",
                "AS expires_at_millis, floor(extract(epoch FROM accepted_at) * 1000)::bigint ",
                "AS accepted_at_millis, floor(extract(epoch FROM declined_at) * 1000)::bigint ",
                "AS declined_at_millis, floor(extract(epoch FROM revoked_at) * 1000)::bigint ",
                "AS revoked_at_millis, status, revision",
            ))
            .bind(tenant.as_uuid())
            .bind(command.course.as_uuid())
            .bind(invitation_id)
            .bind(command.target.as_uuid())
            .bind(inviter.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            let row = match row {
                Some(row) => row,
                None => sqlx::query(concat!(
                    "SELECT invitation_id, course_id, target_user_id, invited_by_membership_id, ",
                    "floor(extract(epoch FROM created_at)*1000)::bigint AS created_at_millis, ",
                    "floor(extract(epoch FROM expires_at)*1000)::bigint AS expires_at_millis, ",
                    "floor(extract(epoch FROM accepted_at)*1000)::bigint AS accepted_at_millis, ",
                    "floor(extract(epoch FROM declined_at)*1000)::bigint AS declined_at_millis, ",
                    "floor(extract(epoch FROM revoked_at)*1000)::bigint AS revoked_at_millis, ",
                    "status, revision FROM course_instructor_invitation WHERE tenant_id = $1 ",
                    "AND course_id = $2 AND target_user_id = $3 AND status = 'pending' FOR UPDATE",
                ))
                .bind(tenant.as_uuid())
                .bind(command.course.as_uuid())
                .bind(command.target.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?,
            };
            let stored = decode_invitation(&row)?;
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
            let course: Option<Uuid> = sqlx::query_scalar(concat!(
                "SELECT course_id FROM course_instructor_invitation WHERE tenant_id=$1 ",
                "AND invitation_id=$2 AND target_user_id=$3",
            ))
            .bind(tenant.as_uuid())
            .bind(command.invitation.as_uuid())
            .bind(command.actor.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            let course = CourseId::from_uuid(course.ok_or(StoreError::NotFound)?);
            // T2's shared course row serializes membership changes.  The roster
            // row follows it, then the invitation and target membership lock.
            lock_course(&mut transaction, tenant, course).await?;
            roster_revision(&mut transaction, tenant, course, true).await?;
            let row = sqlx::query(concat!(
                "SELECT invitation_id, course_id, target_user_id, invited_by_membership_id, ",
                "floor(extract(epoch FROM created_at)*1000)::bigint AS created_at_millis, ",
                "floor(extract(epoch FROM expires_at)*1000)::bigint AS expires_at_millis, ",
                "floor(extract(epoch FROM accepted_at)*1000)::bigint AS accepted_at_millis, ",
                "floor(extract(epoch FROM declined_at)*1000)::bigint AS declined_at_millis, ",
                "floor(extract(epoch FROM revoked_at)*1000)::bigint AS revoked_at_millis, ",
                "status, revision FROM course_instructor_invitation WHERE tenant_id = $1 ",
                "AND invitation_id = $2 FOR UPDATE",
            ))
            .bind(tenant.as_uuid())
            .bind(command.invitation.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
            let stored = decode_invitation(&row)?;
            if command.actor != stored.invitation.target {
                return Err(StoreError::NotFound);
            }
            if stored.invitation.accepted_at.is_some() {
                let view = accepted_view(&mut transaction, tenant, command.invitation).await?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(view);
            }
            if stored.revision != command.expected_revision {
                return Err(StoreError::Conflict);
            }
            if stored.invitation.declined_at.is_some()
                || stored.invitation.revoked_at.is_some()
                || is_expired(&stored.invitation, &mut transaction).await?
            {
                return Err(StoreError::Conflict);
            }
            require_locked_eligibility(&mut transaction, command.actor).await?;
            let existing: Option<Uuid> = sqlx::query_scalar(concat!(
                "SELECT course_membership_id FROM course_member WHERE tenant_id = $1 ",
                "AND course_id = $2 AND user_id = $3 AND status = 'active' FOR UPDATE",
            ))
            .bind(tenant.as_uuid())
            .bind(stored.invitation.course.as_uuid())
            .bind(command.actor.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            let membership = match existing {
                Some(id) => {
                    let role: String = sqlx::query_scalar(concat!(
                        "SELECT role FROM course_member WHERE tenant_id=$1 AND course_id=$2 ",
                        "AND course_membership_id=$3",
                    ))
                    .bind(tenant.as_uuid())
                    .bind(stored.invitation.course.as_uuid())
                    .bind(id)
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    if role != "instructor" {
                        return Err(StoreError::Conflict);
                    }
                    id
                }
                None => {
                    let id = random_uuid("course membership ID")?;
                    sqlx::query(concat!(
                        "INSERT INTO course_member (tenant_id, course_id, course_membership_id, ",
                        "user_id, role, student_id, status, joined_at) VALUES ",
                        "($1,$2,$3,$4,'instructor',NULL,'active',transaction_timestamp())",
                    ))
                    .bind(tenant.as_uuid())
                    .bind(stored.invitation.course.as_uuid())
                    .bind(id)
                    .bind(command.actor.as_uuid())
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    id
                }
            };
            let changed = sqlx::query(concat!(
                "UPDATE course_instructor_invitation SET status='accepted', ",
                "accepted_at=transaction_timestamp(), accepted_membership_id=$3, ",
                "revision=revision+1 ",
                "WHERE tenant_id=$1 AND invitation_id=$2 AND status='pending' AND revision=$4",
            ))
            .bind(tenant.as_uuid())
            .bind(command.invitation.as_uuid())
            .bind(membership)
            .bind(command.expected_revision.as_i64())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .rows_affected();
            if changed != 1 {
                return Err(StoreError::Conflict);
            }
            let revision =
                bump_roster(&mut transaction, tenant, stored.invitation.course, None).await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(DirectInstructorMembershipView {
                membership: CourseMembershipId::from_uuid(membership),
                course: stored.invitation.course,
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
        target_terminal(self, context, command, "declined").await
    }

    async fn revoke_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: RevokeCoInstructorInvitation,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        lock_course(&mut transaction, context.tenant_id(), command.course).await?;
        require_direct_instructor(
            &mut transaction,
            context.tenant_id(),
            command.course,
            command.actor,
        )
        .await?;
        let invitation_course: Option<Uuid> = sqlx::query_scalar(
            "SELECT course_id FROM course_instructor_invitation \
             WHERE tenant_id=$1 AND invitation_id=$2 FOR UPDATE",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(command.invitation.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if invitation_course.is_none() {
            return Err(StoreError::NotFound);
        }
        let changed = sqlx::query(concat!(
            "UPDATE course_instructor_invitation SET status='revoked', ",
            "revoked_at=transaction_timestamp(), revision=revision+1 WHERE tenant_id=$1 ",
            "AND course_id=$2 AND invitation_id=$3 AND status='pending' ",
            "AND expires_at > transaction_timestamp() AND revision=$4",
        ))
        .bind(context.tenant_id().as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.invitation.as_uuid())
        .bind(command.expected_revision.as_i64())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .rows_affected();
        if changed != 1 {
            return Err(StoreError::Conflict);
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
            lock_course(&mut transaction, tenant, command.course).await?;
            require_direct_instructor(&mut transaction, tenant, command.course, command.actor)
                .await?;
            // Validate first without a row lock so missing/wrong-role targets
            // win over stale roster revisions. The final lock remains after the
            // roster lock, preserving the global course -> roster -> membership order.
            require_active_direct_instructor_membership(
                &mut transaction,
                tenant,
                command.course,
                command.membership,
                false,
            )
            .await?;
            let current = roster_revision(&mut transaction, tenant, command.course, true).await?;
            if current != command.expected_roster_revision {
                return Err(StoreError::Conflict);
            }
            require_active_direct_instructor_membership(
                &mut transaction,
                tenant,
                command.course,
                command.membership,
                true,
            )
            .await?;
            let other_active_instructors: i64 = sqlx::query_scalar(concat!(
                "SELECT count(*) FROM course_member WHERE tenant_id=$1 AND course_id=$2 ",
                "AND course_membership_id <> $3 AND role='instructor' AND status='active'",
            ))
            .bind(tenant.as_uuid())
            .bind(command.course.as_uuid())
            .bind(command.membership.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !can_remove_direct_instructor(other_active_instructors) {
                return Err(StoreError::Conflict);
            }
            let changed = sqlx::query(concat!(
                "UPDATE course_member SET status='revoked', revoked_at=transaction_timestamp() ",
                "WHERE tenant_id=$1 AND course_id=$2 AND course_membership_id=$3 ",
                "AND role='instructor' AND status='active'",
            ))
            .bind(tenant.as_uuid())
            .bind(command.course.as_uuid())
            .bind(command.membership.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .rows_affected();
            if changed != 1 {
                return Err(StoreError::NotFound);
            }
            bump_roster(&mut transaction, tenant, command.course, Some(current)).await?;
            transaction.commit().await.map_err(map_sqlx_error)
        })
        .await
    }
}

async fn target_terminal(
    store: &PostgresStore,
    context: TenantContext,
    command: RespondToCoInstructorInvitation,
    status: &str,
) -> Result<(), StoreError> {
    let mut transaction = store.begin_tenant(context).await?;
    let target: Option<Uuid> = sqlx::query_scalar(
        "SELECT target_user_id FROM course_instructor_invitation \
         WHERE tenant_id=$1 AND invitation_id=$2 FOR UPDATE",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(command.invitation.as_uuid())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;
    match target {
        None => return Err(StoreError::NotFound),
        Some(target) if target != command.actor.as_uuid() => return Err(StoreError::NotFound),
        Some(_) => {}
    }
    let changed = sqlx::query(concat!(
        "UPDATE course_instructor_invitation SET status=$3, declined_at=transaction_timestamp(), ",
        "revision=revision+1 WHERE tenant_id=$1 AND invitation_id=$2 AND target_user_id=$4 ",
        "AND status='pending' AND expires_at > transaction_timestamp() AND revision=$5",
    ))
    .bind(context.tenant_id().as_uuid())
    .bind(command.invitation.as_uuid())
    .bind(status)
    .bind(command.actor.as_uuid())
    .bind(command.expected_revision.as_i64())
    .execute(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?
    .rows_affected();
    if changed != 1 {
        return Err(StoreError::Conflict);
    }
    transaction.commit().await.map_err(map_sqlx_error)
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
async fn require_account(
    transaction: &mut Transaction<'_, Postgres>,
    user: UserId,
) -> Result<(), StoreError> {
    let exists: bool =
        sqlx::query_scalar("SELECT public.ple_instructor_approval_target_exists($1)")
            .bind(user.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    exists.then_some(()).ok_or(StoreError::NotFound)
}
async fn require_eligible(
    transaction: &mut Transaction<'_, Postgres>,
    user: UserId,
) -> Result<(), StoreError> {
    let eligible: bool = sqlx::query_scalar("SELECT public.ple_instructor_approval_eligible($1)")
        .bind(user.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    eligible.then_some(()).ok_or(StoreError::Forbidden)
}
async fn require_locked_eligibility(
    transaction: &mut Transaction<'_, Postgres>,
    user: UserId,
) -> Result<(), StoreError> {
    let eligible: bool =
        sqlx::query_scalar("SELECT public.ple_lock_instructor_approval_eligibility($1)")
            .bind(user.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    eligible.then_some(()).ok_or(StoreError::Forbidden)
}

fn can_remove_direct_instructor(other_active_instructors: i64) -> bool {
    other_active_instructors > 0
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

async fn require_active_direct_instructor_membership(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    membership: CourseMembershipId,
    lock: bool,
) -> Result<(), StoreError> {
    let query = if lock {
        concat!(
            "SELECT course_membership_id FROM course_member WHERE tenant_id=$1 AND course_id=$2 ",
            "AND course_membership_id=$3 AND role='instructor' AND status='active' FOR UPDATE",
        )
    } else {
        concat!(
            "SELECT course_membership_id FROM course_member WHERE tenant_id=$1 AND course_id=$2 ",
            "AND course_membership_id=$3 AND role='instructor' AND status='active'",
        )
    };
    let id: Option<Uuid> = sqlx::query_scalar(query)
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(membership.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    id.map(|_| ()).ok_or(StoreError::NotFound)
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
async fn lock_course(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<(), StoreError> {
    let found: Option<Uuid> = sqlx::query_scalar(
        "SELECT course_id FROM course WHERE tenant_id=$1 AND course_id=$2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    found.map(|_| ()).ok_or(StoreError::NotFound)
}
async fn roster_revision(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    lock: bool,
) -> Result<RosterRevision, StoreError> {
    let value: Option<i64> = if lock {
        sqlx::query_scalar(concat!(
            "SELECT revision FROM course_roster_state WHERE tenant_id=$1 ",
            "AND course_id=$2 FOR UPDATE",
        ))
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
    } else {
        sqlx::query_scalar(
            "SELECT revision FROM course_roster_state WHERE tenant_id=$1 AND course_id=$2",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
    };
    RosterRevision::from_stored(value.ok_or(StoreError::NotFound)?)
}
async fn bump_roster(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    expected: Option<RosterRevision>,
) -> Result<RosterRevision, StoreError> {
    let expected = expected
        .map(|value| i64::try_from(value.value()).map_err(|_| StoreError::Conflict))
        .transpose()?;
    let value: Option<i64> = sqlx::query_scalar(concat!(
        "UPDATE course_roster_state SET revision=revision+1, updated_at=transaction_timestamp() ",
        "WHERE tenant_id=$1 AND course_id=$2 AND ($3::bigint IS NULL OR revision=$3) ",
        "RETURNING revision",
    ))
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(expected)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    RosterRevision::from_stored(value.ok_or(StoreError::Conflict)?)
}
async fn accepted_view(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    invitation: CoInstructorInvitationId,
) -> Result<DirectInstructorMembershipView, StoreError> {
    let row = sqlx::query(concat!(
        "SELECT invitation.course_id, invitation.target_user_id, ",
        "invitation.accepted_membership_id, roster.revision ",
        "FROM course_instructor_invitation invitation JOIN course_roster_state roster ",
        "ON roster.tenant_id=invitation.tenant_id AND roster.course_id=invitation.course_id ",
        "WHERE invitation.tenant_id=$1 AND invitation.invitation_id=$2 ",
        "AND invitation.status='accepted'",
    ))
    .bind(tenant.as_uuid())
    .bind(invitation.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::Conflict)?;
    Ok(DirectInstructorMembershipView {
        membership: CourseMembershipId::from_uuid(
            row.try_get("accepted_membership_id")
                .map_err(map_sqlx_error)?,
        ),
        course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        user: UserId::from_uuid(row.try_get("target_user_id").map_err(map_sqlx_error)?),
        roster_revision: RosterRevision::from_stored(
            row.try_get("revision").map_err(map_sqlx_error)?,
        )?,
    })
}
async fn is_expired(
    invitation: &CoInstructorInvitation,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<bool, StoreError> {
    let now: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(now >= invitation.expires_at.as_unix_millis())
}
fn random_uuid(label: &str) -> Result<Uuid, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!("{label} randomness unavailable: {error}"))
    })?;
    Ok(Uuid::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::{can_remove_direct_instructor, require_row, stored_status_matches_terminals};
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

    #[test]
    fn final_direct_instructor_removal_conflicts_before_the_database_trigger() {
        assert!(!can_remove_direct_instructor(0));
        assert!(can_remove_direct_instructor(1));
    }
}
