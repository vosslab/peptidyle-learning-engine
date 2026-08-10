//! PostgreSQL course-roster persistence and atomic invitation claim.

use std::collections::BTreeSet;

use async_trait::async_trait;
use question_model::{CourseId, CourseMembershipRole, StudentId, TenantId, UserId};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::course_roster_decode::*;
use super::{PostgresStore, map_sqlx_error, page_from_keyed_records};
use crate::{
    ClaimCourseInvitation, ClaimedCourseMembership, CommitCourseRosterImport,
    CommittedCourseRosterImport, CourseEnrollmentPolicy, CourseInvitation, CourseInvitationId,
    CourseMemberId, CourseRosterImportPreview, CourseRosterMember, CourseRosterPage,
    CourseRosterStore, CreateCourseInvitation, PageRequest, ReplaceCourseEnrollmentPolicy,
    RevokeCourseInvitation, RevokeCourseMember, RosterRevision, SessionTokenHash,
    StageCourseRosterImport, StoreError, TenantContext,
};

#[path = "course_roster/enrollment.rs"]
mod enrollment;
#[path = "course_roster/import.rs"]
mod import;

#[async_trait]
impl CourseRosterStore for PostgresStore {
    async fn list_course_roster(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        page: PageRequest,
    ) -> Result<CourseRosterPage, StoreError> {
        let mut transaction = self.begin_tenant_snapshot(context).await?;
        require_course(&mut transaction, context.tenant_id(), course).await?;
        require_manager(&mut transaction, session, course).await?;
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
                 SELECT 'm:' || member.course_member_id::text AS stable_key, \
                        'member'::text AS record_kind, member.course_member_id AS record_id, \
                        member.user_id, member.student_id, member.display_name, \
                        member.roster_email_normalized AS normalized_email, \
                        member.roster_email_delivery AS delivery_email, member.roster_id, \
                        member.status, NULL::uuid AS invited_by, NULL::uuid AS claimed_user_id, \
                        member.joined_at AS created_at, NULL::timestamptz AS expires_at, \
                        member.revoked_at \
                   FROM course_roster_member member \
                  WHERE member.tenant_id = $1 AND member.course_id = $2 \
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

    async fn create_course_invitation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CreateCourseInvitation,
    ) -> Result<CourseInvitation, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        require_manager(&mut transaction, session, command.course).await?;
        lock_course_roster_cross_product(&mut transaction, tenant, command.course).await?;
        let actor = require_manager(&mut transaction, session, command.course).await?;
        let policy = load_policy(&mut transaction, tenant, command.course, true).await?;
        if !policy.validates(&command.email) {
            return Err(StoreError::InvalidRecord(
                "invitation email domain is not permitted".to_string(),
            ));
        }
        sqlx::query(
            "UPDATE course_invitation SET status = 'expired' \
             WHERE tenant_id = $1 AND course_id = $2 AND status = 'pending' \
               AND expires_at <= transaction_timestamp()",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if let Some(row) = sqlx::query(
            "SELECT invitation_id, token_hash, normalized_email, delivery_email, roster_id, \
                    invited_by, status, claimed_user_id, \
                    floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM course_invitation \
             WHERE tenant_id = $1 AND course_id = $2 AND idempotency_key = $3 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.idempotency_key.as_str())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        {
            let stored_hash: Vec<u8> = row.try_get("token_hash").map_err(map_sqlx_error)?;
            let invitation = decode_invitation(&row, tenant, command.course)?;
            if stored_hash == command.token_hash.as_bytes()
                && invitation.email == command.email
                && invitation.roster_id == command.roster_id
            {
                if invitation.status != crate::CourseInvitationStatus::Pending {
                    return Err(StoreError::Conflict);
                }
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(invitation);
            }
            return Err(StoreError::Conflict);
        }
        let invitation_id = CourseInvitationId::generate()?;
        let row = sqlx::query(
            "INSERT INTO course_invitation \
             (tenant_id, course_id, invitation_id, token_hash, normalized_email, \
              delivery_email, roster_id, invited_by, idempotency_key, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, \
                     transaction_timestamp() + ($10::bigint * interval '1 second')) \
             RETURNING invitation_id, normalized_email, delivery_email, roster_id, invited_by, \
                       status, claimed_user_id, \
                       floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                       floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .bind(invitation_id.as_uuid())
        .bind(command.token_hash.as_bytes().to_vec())
        .bind(command.email.normalized())
        .bind(command.email.delivery())
        .bind(command.roster_id.as_str())
        .bind(actor.as_uuid())
        .bind(command.idempotency_key.as_str())
        .bind(i64::from(command.lifetime.as_seconds()))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        bump_revision(&mut transaction, tenant, command.course, None).await?;
        let invitation = decode_invitation(&row, tenant, command.course)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(invitation)
    }

    async fn replace_course_enrollment_policy(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceCourseEnrollmentPolicy,
    ) -> Result<CourseEnrollmentPolicy, StoreError> {
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
        require_manager(&mut transaction, session, command.course).await?;
        lock_course_roster_cross_product(&mut transaction, tenant, command.course).await?;
        require_manager(&mut transaction, session, command.course).await?;
        let current = load_policy(&mut transaction, tenant, command.course, true).await?;
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        if current.allowed_domains == candidate.allowed_domains
            && current.signup_posture == candidate.signup_posture
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(current);
        }
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

    async fn claim_course_invitation(
        &self,
        command: ClaimCourseInvitation,
    ) -> Result<ClaimedCourseMembership, StoreError> {
        let mut transaction = self.begin_app().await?;
        let row = sqlx::query(
            "SELECT tenant_id, course_id, invitation_id, normalized_email, delivery_email, \
                    roster_id, status, claimed_user_id, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM public.ple_claim_course_invitation_context($1)",
        )
        .bind(command.token_hash.as_bytes().to_vec())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let tenant = TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?);
        let course = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
        let invitation_id =
            CourseInvitationId::from_uuid(row.try_get("invitation_id").map_err(map_sqlx_error)?);
        let normalized: String = row.try_get("normalized_email").map_err(map_sqlx_error)?;
        if normalized != command.verified_email.normalized() {
            return Err(StoreError::Forbidden);
        }
        let status: String = row.try_get("status").map_err(map_sqlx_error)?;
        let claimed_user: Option<Uuid> = row.try_get("claimed_user_id").map_err(map_sqlx_error)?;
        if status == "claimed" {
            if claimed_user.map(UserId::from_uuid) != Some(command.user) {
                return Err(StoreError::Conflict);
            }
            let member =
                load_member_by_user(&mut transaction, tenant, course, command.user).await?;
            let policy = load_policy(&mut transaction, tenant, course, false).await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ClaimedCourseMembership {
                tenant,
                course,
                member,
                roster_revision: policy.revision,
            });
        }
        let expires_at: i64 = row.try_get("expires_at_millis").map_err(map_sqlx_error)?;
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint",
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if status != "pending" || expires_at <= now {
            return Err(StoreError::NotFound);
        }
        let roster_id: String = row.try_get("roster_id").map_err(map_sqlx_error)?;
        let delivery: String = row.try_get("delivery_email").map_err(map_sqlx_error)?;
        let student = resolve_learner(&mut transaction, tenant, command.user).await?;
        let member = upsert_claimed_member(
            &mut transaction,
            tenant,
            course,
            command.user,
            student,
            &command.display_name,
            &normalized,
            &delivery,
            &roster_id,
        )
        .await?;
        ensure_student_membership(&mut transaction, tenant, course, command.user).await?;
        reconcile_member_assignments(&mut transaction, tenant, course, command.user, student)
            .await?;
        let updated = sqlx::query(
            "UPDATE course_invitation SET status = 'claimed', claimed_user_id = $4, \
                    claimed_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND course_id = $2 AND invitation_id = $3 \
                   AND status = 'pending'",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(invitation_id.as_uuid())
        .bind(command.user.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        let roster_revision = bump_revision(&mut transaction, tenant, course, None).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(ClaimedCourseMembership {
            tenant,
            course,
            member,
            roster_revision,
        })
    }

    async fn revoke_course_member(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: RevokeCourseMember,
    ) -> Result<RosterRevision, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        require_manager(&mut transaction, session, command.course).await?;
        lock_course_roster_cross_product(&mut transaction, tenant, command.course).await?;
        require_manager(&mut transaction, session, command.course).await?;
        let current = load_policy(&mut transaction, tenant, command.course, true).await?;
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let row = sqlx::query(
            "SELECT user_id, status FROM course_roster_member \
             WHERE tenant_id = $1 AND course_id = $2 AND course_member_id = $3 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.member.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let user: Uuid = row.try_get("user_id").map_err(map_sqlx_error)?;
        let status: String = row.try_get("status").map_err(map_sqlx_error)?;
        if status == "revoked" {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(current.revision);
        }
        sqlx::query(
            "UPDATE course_roster_member SET status = 'revoked', \
                    revoked_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND course_id = $2 AND course_member_id = $3",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.member.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "DELETE FROM course_member \
             WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 AND role = 'student'",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .bind(user)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let revision = bump_revision(
            &mut transaction,
            tenant,
            command.course,
            Some(command.expected_revision),
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(revision)
    }

    async fn revoke_course_invitation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: RevokeCourseInvitation,
    ) -> Result<RosterRevision, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        require_manager(&mut transaction, session, command.course).await?;
        lock_course_roster_cross_product(&mut transaction, tenant, command.course).await?;
        require_manager(&mut transaction, session, command.course).await?;
        let current = load_policy(&mut transaction, tenant, command.course, true).await?;
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let status: String = sqlx::query_scalar(
            "SELECT status FROM course_invitation \
             WHERE tenant_id = $1 AND course_id = $2 AND invitation_id = $3 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.invitation.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        match status.as_str() {
            "pending" => {}
            "revoked" => {
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(current.revision);
            }
            "claimed" | "expired" => return Err(StoreError::Conflict),
            _ => {
                return Err(StoreError::Unavailable(
                    "stored invitation status is invalid".to_string(),
                ));
            }
        }
        let updated = sqlx::query(
            "UPDATE course_invitation SET status = 'revoked' \
             WHERE tenant_id = $1 AND course_id = $2 AND invitation_id = $3 \
               AND status = 'pending'",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.invitation.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        let revision = bump_revision(
            &mut transaction,
            tenant,
            command.course,
            Some(command.expected_revision),
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(revision)
    }

    async fn stage_course_roster_import(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: StageCourseRosterImport,
    ) -> Result<CourseRosterImportPreview, StoreError> {
        import::stage(self, context, session, command).await
    }

    async fn commit_course_roster_import(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CommitCourseRosterImport,
    ) -> Result<CommittedCourseRosterImport, StoreError> {
        import::commit(self, context, session, command).await
    }
}

pub(super) async fn ensure_roster_state(
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

pub(super) async fn reconcile_new_assignment(
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
            StudentId::from_uuid(row.try_get("student_id").map_err(map_sqlx_error)?),
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn lock_course_roster_cross_product(
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

pub(super) async fn reconcile_legacy_course_members(
    transaction: &mut Transaction<'_, Postgres>,
    course: &crate::CourseRecord,
) -> Result<(), StoreError> {
    let students = course
        .members
        .iter()
        .filter(|membership| membership.role == CourseMembershipRole::Student)
        .map(|membership| membership.user)
        .collect::<BTreeSet<_>>();
    for user in &students {
        let student = resolve_learner(transaction, course.tenant, *user).await?;
        sqlx::query(
            "INSERT INTO course_roster_member \
             (tenant_id, course_id, course_member_id, user_id, student_id, display_name, \
              source, status, joined_at) \
             VALUES ($1, $2, $3, $4, $5, 'Legacy learner', 'legacy', 'active', \
                     transaction_timestamp()) \
             ON CONFLICT (tenant_id, course_id, user_id) DO UPDATE SET \
                 student_id = EXCLUDED.student_id, \
                 status = CASE WHEN course_roster_member.source = 'legacy' \
                               THEN 'active' ELSE course_roster_member.status END, \
                 revoked_at = CASE WHEN course_roster_member.source = 'legacy' \
                                   THEN NULL ELSE course_roster_member.revoked_at END",
        )
        .bind(course.tenant.as_uuid())
        .bind(course.id.as_uuid())
        .bind(CourseMemberId::generate()?.as_uuid())
        .bind(user.as_uuid())
        .bind(student.as_uuid())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    let student_ids = students
        .iter()
        .map(|user| user.as_uuid())
        .collect::<Vec<_>>();
    sqlx::query(
        "UPDATE course_roster_member SET status = 'revoked', \
                revoked_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND course_id = $2 AND status = 'active' \
           AND NOT (user_id = ANY($3::uuid[]))",
    )
    .bind(course.tenant.as_uuid())
    .bind(course.id.as_uuid())
    .bind(student_ids)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

async fn reconcile_member_assignments(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
    student: StudentId,
) -> Result<(), StoreError> {
    let assignments: Vec<Uuid> = sqlx::query_scalar(
        "SELECT assignment_id FROM assignment \
         WHERE tenant_id = $1 AND course_id = $2 ORDER BY assignment_id",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    for assignment in assignments {
        enrollment::insert_missing_enrollment(
            transaction,
            tenant,
            question_model::AssignmentId::from_uuid(assignment),
            user,
            student,
        )
        .await?;
    }
    Ok(())
}

async fn resolve_learner(
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
async fn upsert_claimed_member(
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
    let member = load_member_by_user_optional(transaction, tenant, course, user).await?;
    let member_id = member
        .as_ref()
        .map_or_else(CourseMemberId::generate, |member| Ok(member.id))?;
    let row = sqlx::query(
        "INSERT INTO course_roster_member \
         (tenant_id, course_id, course_member_id, user_id, student_id, display_name, \
          roster_email_normalized, roster_email_delivery, roster_id, source, status, joined_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'invitation', 'active', \
                 transaction_timestamp()) \
         ON CONFLICT (tenant_id, course_id, user_id) DO UPDATE SET \
             student_id = EXCLUDED.student_id, display_name = EXCLUDED.display_name, \
             roster_email_normalized = EXCLUDED.roster_email_normalized, \
             roster_email_delivery = EXCLUDED.roster_email_delivery, roster_id = EXCLUDED.roster_id, \
             source = 'invitation', status = 'active', revoked_at = NULL \
         RETURNING course_member_id, user_id, student_id, display_name, \
                   roster_email_normalized AS normalized_email, \
                   roster_email_delivery AS delivery_email, roster_id, status, \
                   floor(extract(epoch FROM joined_at) * 1000)::bigint AS created_at_millis, \
                   floor(extract(epoch FROM revoked_at) * 1000)::bigint AS revoked_at_millis",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(member_id.as_uuid())
    .bind(user.as_uuid())
    .bind(student.as_uuid())
    .bind(display_name)
    .bind(normalized_email)
    .bind(delivery_email)
    .bind(roster_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    decode_member(&row, tenant, course)
}

async fn ensure_student_membership(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> Result<(), StoreError> {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM course_member \
         WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(user.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    match role.as_deref() {
        Some("student") => Ok(()),
        Some(_) => Err(StoreError::Conflict),
        None => {
            sqlx::query(
                "INSERT INTO course_member (tenant_id, course_id, user_id, role) \
                 VALUES ($1, $2, $3, 'student')",
            )
            .bind(tenant.as_uuid())
            .bind(course.as_uuid())
            .bind(user.as_uuid())
            .execute(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            Ok(())
        }
    }
}

async fn require_course(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<(), StoreError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM course \
         WHERE tenant_id = $1 AND course_id = $2 \
         AND public.ple_course_records_accessible(tenant_id, course_id))",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    exists.then_some(()).ok_or(StoreError::NotFound)
}

pub(super) async fn require_manager(
    transaction: &mut Transaction<'_, Postgres>,
    session: SessionTokenHash,
    course: CourseId,
) -> Result<UserId, StoreError> {
    let actor: Option<Uuid> =
        sqlx::query_scalar("SELECT public.ple_course_roster_actor($1, $2, true)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    if let Some(actor) = actor {
        return Ok(UserId::from_uuid(actor));
    }
    let course_visible: Option<Uuid> =
        sqlx::query_scalar("SELECT public.ple_course_roster_actor($1, $2, false)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    if course_visible.is_some() {
        Err(StoreError::Forbidden)
    } else {
        Err(StoreError::NotFound)
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

async fn load_member_by_user(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> Result<CourseRosterMember, StoreError> {
    load_member_by_user_optional(transaction, tenant, course, user)
        .await?
        .ok_or(StoreError::NotFound)
}

async fn load_member_by_user_optional(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> Result<Option<CourseRosterMember>, StoreError> {
    let row = sqlx::query(
        "SELECT course_member_id, user_id, student_id, display_name, \
                roster_email_normalized AS normalized_email, \
                roster_email_delivery AS delivery_email, roster_id, status, \
                floor(extract(epoch FROM joined_at) * 1000)::bigint AS created_at_millis, \
                floor(extract(epoch FROM revoked_at) * 1000)::bigint AS revoked_at_millis \
         FROM course_roster_member WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3",
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

fn random_uuid(label: &str) -> Result<Uuid, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!("{label} randomness unavailable: {error}"))
    })?;
    Ok(Uuid::from_bytes(bytes))
}
