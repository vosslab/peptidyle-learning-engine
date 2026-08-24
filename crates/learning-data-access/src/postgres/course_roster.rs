use async_trait::async_trait;
use question_model::{CourseId, TenantId, UserId};
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeSet;

use super::course_roster_decode::*;
use super::{PostgresStore, map_sqlx_error, page_from_keyed_records, retry_transaction};
use crate::{
    ClaimCourseInvitation, ClaimedCourseMembership, CommitCourseRosterImport,
    CommittedCourseRosterImport, CourseEnrollmentPolicy, CourseInvitation, CourseInvitationId,
    CourseRosterImportPreview, CourseRosterPage, CourseRosterStore, CourseRosterSupportAction,
    CreateCourseInvitation, PageRequest, ReplaceCourseEnrollmentPolicy, RevokeCourseInvitation,
    RevokeCourseMember, RosterRevision, SessionTokenHash, StageCourseRosterImport, StoreError,
    TenantContext, UpsertCourseMember,
};

#[path = "course_roster/authority.rs"]
mod authority;
#[path = "course_roster/import.rs"]
mod import;
#[path = "course_roster/invitation_capability.rs"]
mod invitation_capability;
#[path = "course_roster/member_revoke_capability.rs"]
mod member_revoke_capability;
#[path = "course_roster/member_upsert_capability.rs"]
mod member_upsert_capability;
#[path = "course_roster/state.rs"]
mod state;

use authority::require_course;
pub(super) use authority::{
    precheck_course_roster_authority, require_audited_course_roster_actor,
    require_course_instructor,
};
pub(super) use state::{ensure_roster_state, lock_course_roster_cross_product};

#[async_trait]
impl CourseRosterStore for PostgresStore {
    async fn list_course_roster(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        page: PageRequest,
    ) -> Result<CourseRosterPage, StoreError> {
        retry_transaction(|| {
            let page = page.clone();
            async move {
        let mut transaction = self.begin_tenant_writable_snapshot(context).await?;
        require_course(&mut transaction, context.tenant_id(), course).await?;
        require_audited_course_roster_actor(
            &mut transaction,
            session,
            course,
            CourseRosterSupportAction::ListRoster,
        )
        .await?;
        let policy = load_policy(&mut transaction, context.tenant_id(), course, false).await?;
        let after = page
            .after
            .as_ref()
            .map(|cursor| cursor.as_str().to_string());
        let rows = sqlx::query(
            "SELECT stable_key, record_kind, record_id, user_id, student_id, display_name, \
                    normalized_email, delivery_email, roster_id, status, invited_by, \
                    claimed_user_id, \
                    floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis, \
                    floor(extract(epoch FROM revoked_at) * 1000)::bigint AS revoked_at_millis \
             FROM ( \
                 SELECT 'm:' || membership.course_membership_id::text AS stable_key, \
                        'member'::text AS record_kind, membership.course_membership_id AS record_id, \
                        membership.user_id, membership.student_id, profile.display_name, \
                        profile.roster_email_normalized AS normalized_email, \
                        profile.roster_email_delivery AS delivery_email, membership.roster_id, membership.status, \
                        NULL::uuid AS invited_by, NULL::uuid AS claimed_user_id, \
                        membership.joined_at AS created_at, NULL::timestamptz AS expires_at, \
                        membership.revoked_at \
                   FROM course_member membership \
                   JOIN course_roster_profile profile \
                     ON profile.tenant_id = membership.tenant_id \
                    AND profile.course_id = membership.course_id \
                    AND profile.course_membership_id = membership.course_membership_id \
                  WHERE membership.tenant_id = $1 AND membership.course_id = $2 \
                    AND membership.role = 'student' \
                 UNION ALL \
                 SELECT 'i:' || invitation.invitation_id::text AS stable_key, \
                        'invitation'::text AS record_kind, invitation.invitation_id AS record_id, \
                        NULL::uuid AS user_id, NULL::uuid AS student_id, \
                        NULL::text AS display_name, invitation.normalized_email, \
                        invitation.delivery_email, invitation.roster_id, \
                        CASE WHEN invitation.status = 'pending' \
                                  AND invitation.expires_at <= transaction_timestamp() \
                             THEN 'expired' ELSE invitation.status END AS status, \
                        invitation.invited_by, invitation.claimed_user_id, invitation.created_at, \
                        invitation.expires_at, NULL::timestamptz AS revoked_at \
                   FROM course_invitation invitation \
                  WHERE invitation.tenant_id = $1 AND invitation.course_id = $2 \
                    AND invitation.status = 'pending' \
                    AND invitation.expires_at > transaction_timestamp() \
             ) roster \
             WHERE ($3::text IS NULL OR stable_key > $3) ORDER BY stable_key LIMIT $4",
        )
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
                    row.try_get("stable_key").map_err(map_sqlx_error)?,
                    decode_roster_entry(row, context.tenant_id(), course)?,
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let entries = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseRosterPage { entries, policy })
            }
        })
        .await
    }

    async fn create_course_invitation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CreateCourseInvitation,
    ) -> Result<CourseInvitation, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                let tenant = context.tenant_id();
                let mut transaction = self.begin_tenant(context).await?;
                let invitation =
                    invitation_capability::create(&mut transaction, tenant, session, &command)
                        .await?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(invitation)
            }
        })
        .await
    }

    async fn replace_course_enrollment_policy(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceCourseEnrollmentPolicy,
    ) -> Result<CourseEnrollmentPolicy, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                let tenant = context.tenant_id();
                let candidate = CourseEnrollmentPolicy {
                    course: command.course,
                    allowed_domains: command.allowed_domains,
                    signup_posture: command.signup_posture,
                    revision: command.expected_revision,
                };
                candidate
                    .validate_shape()
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
                let mut transaction = self.begin_tenant(context).await?;
                precheck_course_roster_authority(&mut transaction, session, command.course).await?;
                lock_course_roster_cross_product(&mut transaction, tenant, command.course).await?;
                let current = load_policy(&mut transaction, tenant, command.course, true).await?;
                if current.revision != command.expected_revision {
                    return Err(StoreError::Conflict);
                }
                if current.allowed_domains == candidate.allowed_domains
                    && current.signup_posture == candidate.signup_posture
                {
                    require_audited_course_roster_actor(
                        &mut transaction,
                        session,
                        command.course,
                        CourseRosterSupportAction::ReplaceEnrollmentPolicy,
                    )
                    .await?;
                    transaction.commit().await.map_err(map_sqlx_error)?;
                    return Ok(current);
                }
                require_audited_course_roster_actor(
                    &mut transaction,
                    session,
                    command.course,
                    CourseRosterSupportAction::ReplaceEnrollmentPolicy,
                )
                .await?;
                sqlx::query(
            "DELETE FROM course_allowed_email_domain WHERE tenant_id = $1 AND course_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
                for rule in &candidate.allowed_domains {
                    sqlx::query(
                        "INSERT INTO course_allowed_email_domain \
                 (tenant_id, course_id, normalized_domain, include_subdomains) \
                 VALUES ($1, $2, $3, $4)",
                    )
                    .bind(tenant.as_uuid())
                    .bind(command.course.as_uuid())
                    .bind(rule.domain.as_str())
                    .bind(rule.include_subdomains)
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                }
                let revision = bump_revision(
                    &mut transaction,
                    tenant,
                    command.course,
                    Some(command.expected_revision),
                )
                .await?;
                sqlx::query(
                    "UPDATE course_roster_state SET signup_posture = $3 \
             WHERE tenant_id = $1 AND course_id = $2",
                )
                .bind(tenant.as_uuid())
                .bind(command.course.as_uuid())
                .bind(posture_name(candidate.signup_posture))
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(CourseEnrollmentPolicy {
                    revision,
                    ..candidate
                })
            }
        })
        .await
    }

    async fn claim_course_invitation(
        &self,
        command: ClaimCourseInvitation,
    ) -> Result<ClaimedCourseMembership, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                let mut transaction = self.begin_app().await?;
                let membership =
                    invitation_capability::claimed_membership(&mut transaction, &command).await?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(membership)
            }
        })
        .await
    }

    async fn upsert_course_member(
        &self,
        context: TenantContext,
        actor: UserId,
        command: UpsertCourseMember,
    ) -> Result<ClaimedCourseMembership, StoreError> {
        let candidate_student = member_upsert_capability::candidate_student_id()?;
        let candidate_membership = member_upsert_capability::candidate_membership_id()?;
        retry_transaction(|| {
            let command = command.clone();
            async move {
                let tenant = context.tenant_id();
                let mut transaction = self.begin_tenant(context).await?;
                let result = member_upsert_capability::upsert_course_student(
                    &mut transaction,
                    tenant,
                    actor,
                    &command,
                    candidate_student,
                    candidate_membership,
                )
                .await?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(result)
            }
        })
        .await
    }

    async fn revoke_course_member(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: RevokeCourseMember,
    ) -> Result<RosterRevision, StoreError> {
        retry_transaction(|| async move {
            let tenant = context.tenant_id();
            let mut transaction = self.begin_tenant(context).await?;
            let actor =
                precheck_course_roster_authority(&mut transaction, session, command.course).await?;
            lock_course_roster_cross_product(&mut transaction, tenant, command.course).await?;
            let revision = member_revoke_capability::revoke_course_student(
                &mut transaction,
                tenant,
                actor,
                session,
                command,
            )
            .await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(revision)
        })
        .await
    }

    async fn revoke_course_invitation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: RevokeCourseInvitation,
    ) -> Result<RosterRevision, StoreError> {
        retry_transaction(|| async move {
            let tenant = context.tenant_id();
            let mut transaction = self.begin_tenant(context).await?;
            let revision =
                invitation_capability::revoke(&mut transaction, tenant, session, command).await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(revision)
        })
        .await
    }

    async fn stage_course_roster_import(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: StageCourseRosterImport,
    ) -> Result<CourseRosterImportPreview, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move { import::stage(self, context, session, command).await }
        })
        .await
    }

    async fn commit_course_roster_import(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CommitCourseRosterImport,
    ) -> Result<CommittedCourseRosterImport, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move { import::commit(self, context, session, command).await }
        })
        .await
    }
}

async fn load_policy(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    lock: bool,
) -> Result<CourseEnrollmentPolicy, StoreError> {
    if lock {
        ensure_roster_state(transaction, tenant, course).await?;
    }
    let query = if lock {
        "SELECT revision, signup_posture FROM course_roster_state \
         WHERE tenant_id = $1 AND course_id = $2 FOR UPDATE"
    } else {
        "SELECT revision, signup_posture FROM course_roster_state \
         WHERE tenant_id = $1 AND course_id = $2"
    };
    let row = sqlx::query(query)
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    let domains = sqlx::query(
        "SELECT normalized_domain, include_subdomains FROM course_allowed_email_domain \
         WHERE tenant_id = $1 AND course_id = $2 ORDER BY normalized_domain",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .iter()
    .map(|row| {
        Ok(crate::AllowedEmailDomain {
            domain: crate::EmailDomain::parse(
                &row.try_get::<String, _>("normalized_domain")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|error| StoreError::Unavailable(error.to_string()))?,
            include_subdomains: row.try_get("include_subdomains").map_err(map_sqlx_error)?,
        })
    })
    .collect::<Result<BTreeSet<_>, StoreError>>()?;
    Ok(CourseEnrollmentPolicy {
        course,
        allowed_domains: domains,
        signup_posture: decode_posture(
            &row.try_get::<String, _>("signup_posture")
                .map_err(map_sqlx_error)?,
        )?,
        revision: RosterRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)?,
    })
}

async fn bump_revision(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    expected: Option<RosterRevision>,
) -> Result<RosterRevision, StoreError> {
    let expected = expected
        .map(|revision| i64::try_from(revision.value()).map_err(|_| StoreError::Conflict))
        .transpose()?;
    let row = sqlx::query(
        "UPDATE course_roster_state SET revision = revision + 1, \
                updated_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND course_id = $2 \
           AND ($3::bigint IS NULL OR revision = $3) RETURNING revision",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(expected)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::Conflict)?;
    RosterRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)
}
