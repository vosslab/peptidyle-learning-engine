use async_trait::async_trait;
use question_model::{CourseId, TenantId, UserId};
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeSet;

use super::course_roster_decode::*;
use super::{PostgresStore, map_sqlx_error, page_from_keyed_records, retry_transaction};
use crate::{
    ClaimCourseInvitation, ClaimedCourseMembership, CommitCourseRosterImport,
    CommittedCourseRosterImport, CourseEnrollmentPolicy, CourseInvitation,
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
                let domains = serde_json::Value::Array(
                    candidate
                        .allowed_domains
                        .iter()
                        .map(|rule| {
                            serde_json::json!({
                                "domain": rule.domain.as_str(),
                                "include_subdomains": rule.include_subdomains,
                            })
                        })
                        .collect(),
                );
                let mut transaction = self.begin_tenant(context).await?;
                let row = sqlx::query(
                    "SELECT tenant_id, actor_id, course_id, roster_revision \
                     FROM public.ple_replace_course_enrollment_policy_v1($1,$2,$3,$4,$5,$6)",
                )
                .bind(tenant.as_uuid())
                .bind(session.to_string())
                .bind(command.course.as_uuid())
                .bind(
                    i64::try_from(command.expected_revision.value())
                        .map_err(|_| StoreError::Conflict)?,
                )
                .bind(posture_name(candidate.signup_posture))
                .bind(sqlx::types::Json(domains))
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)?;
                let returned_tenant: uuid::Uuid =
                    row.try_get("tenant_id").map_err(map_sqlx_error)?;
                let returned_actor: uuid::Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
                let returned_course: uuid::Uuid =
                    row.try_get("course_id").map_err(map_sqlx_error)?;
                let revision = RosterRevision::from_stored(
                    row.try_get("roster_revision").map_err(map_sqlx_error)?,
                )?;
                if returned_tenant != tenant.as_uuid()
                    || returned_course != command.course.as_uuid()
                    || returned_actor.is_nil()
                    || revision.value() < command.expected_revision.value()
                {
                    return Err(StoreError::Unavailable(
                        "enrollment policy capability returned an invalid witness".to_string(),
                    ));
                }
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
