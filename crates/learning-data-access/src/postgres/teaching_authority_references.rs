//! PostgreSQL authorized projections for T2 public teaching-authority locators.

use async_trait::async_trait;
use question_model::teaching_operations::{
    AccountApprovalView, CoInstructorTargetSearchPage, CoInstructorTargetSearchRequest,
    CoInstructorTargetView, TeachingAccountView, TeachingDisplayLabel, TeachingOperationRevision,
};
use question_model::{
    AccountReference, CoInstructorInvitationReference, CoInstructorInvitationState, CourseId,
    CourseMembershipId, CourseMembershipReference, CourseMembershipRole, StudentId, UserId,
};
use sqlx::types::Uuid;

use super::*;
use crate::{OwnAccountReferenceView, TeachingAuthorityReferenceStore};

fn account_reference(value: i32) -> Result<AccountReference, StoreError> {
    AccountReference::new(value as u64)
        .ok_or_else(|| StoreError::Unavailable("stored account reference is invalid".to_string()))
}

fn membership_reference(value: i32) -> Result<CourseMembershipReference, StoreError> {
    CourseMembershipReference::new(value as u64).ok_or_else(|| {
        StoreError::Unavailable("stored course-membership reference is invalid".to_string())
    })
}

fn invitation_reference(value: i32) -> Result<CoInstructorInvitationReference, StoreError> {
    CoInstructorInvitationReference::new(value as u64).ok_or_else(|| {
        StoreError::Unavailable("stored co-instructor invitation reference is invalid".to_string())
    })
}

fn page_after(page: &crate::PageRequest) -> Result<Option<i32>, StoreError> {
    page.after
        .as_ref()
        .map(|cursor| {
            cursor.as_str().parse::<i32>().map_err(|_| {
                StoreError::InvalidRecord("invalid public-reference cursor".to_string())
            })
        })
        .transpose()
}

fn target_search_after(after: Option<&str>) -> Result<Option<i32>, StoreError> {
    after
        .map(|cursor| {
            cursor.parse::<i32>().map_err(|_| {
                StoreError::InvalidRecord("invalid co-instructor target cursor".to_string())
            })
        })
        .transpose()
}

fn database_page_limit(size: u32) -> Result<i32, StoreError> {
    let bounded = i32::try_from(size).map_err(|_| {
        StoreError::InvalidRecord("public-reference page size is too large".to_string())
    })?;
    bounded.checked_add(1).ok_or_else(|| {
        StoreError::InvalidRecord("public-reference page size is too large".to_string())
    })
}

fn database_public_reference(value: u32) -> Result<i32, StoreError> {
    i32::try_from(value).map_err(|_| {
        StoreError::InvalidRecord("public reference exceeds the database range".to_string())
    })
}

fn invitation_state(value: &str) -> Result<CoInstructorInvitationState, StoreError> {
    match value {
        "pending" => Ok(CoInstructorInvitationState::Pending),
        "accepted" => Ok(CoInstructorInvitationState::Accepted),
        "declined" => Ok(CoInstructorInvitationState::Declined),
        "revoked" => Ok(CoInstructorInvitationState::Revoked),
        "expired" => Ok(CoInstructorInvitationState::Expired),
        _ => Err(StoreError::Unavailable(
            "stored co-instructor invitation state is invalid".to_string(),
        )),
    }
}

fn approval_state(value: bool) -> question_model::teaching_operations::InstructorApprovalStateView {
    if value {
        question_model::teaching_operations::InstructorApprovalStateView::Approved
    } else {
        question_model::teaching_operations::InstructorApprovalStateView::Revoked
    }
}

fn membership_role(value: &str) -> Result<CourseMembershipRole, StoreError> {
    match value {
        "student" => Ok(CourseMembershipRole::Student),
        "instructor" => Ok(CourseMembershipRole::Instructor),
        _ => Err(StoreError::Unavailable(
            "stored membership role is invalid".to_string(),
        )),
    }
}

fn membership_status(value: &str) -> Result<crate::CourseMemberStatus, StoreError> {
    match value {
        "active" => Ok(crate::CourseMemberStatus::Active),
        "revoked" => Ok(crate::CourseMemberStatus::Revoked),
        _ => Err(StoreError::Unavailable(
            "stored membership status is invalid".to_string(),
        )),
    }
}

#[async_trait]
impl TeachingAuthorityReferenceStore for PostgresStore {
    async fn search_sysadmin_instructor_candidates(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
        request: question_model::SysadminInstructorCandidateSearchRequest,
    ) -> Result<question_model::SysadminInstructorCandidateSearchPage, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        set_presented_session_hash(&mut transaction, session).await?;
        let rows = sqlx::query(
            "SELECT * FROM public.ple_sysadmin_instructor_candidate_search($1, $2, $3, $4)",
        )
        .bind(session.to_string())
        .bind(request.query.as_str())
        .bind(target_search_after(request.after.as_deref())?)
        .bind(database_page_limit(request.size.get())?)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = rows
            .into_iter()
            .map(|row| {
                let reference =
                    account_reference(row.try_get("account_public_id").map_err(map_sqlx_error)?)?;
                let display = TeachingDisplayLabel::try_from(
                    row.try_get::<String, _>("account_display_name")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|error| StoreError::Unavailable(error.to_string()))?;
                let state = match row
                    .try_get::<String, _>("approval_state")
                    .map_err(map_sqlx_error)?
                    .as_str()
                {
                    "unapproved" => question_model::SysadminInstructorApprovalStateView::Unapproved,
                    "approved" => question_model::SysadminInstructorApprovalStateView::Approved,
                    "revoked" => question_model::SysadminInstructorApprovalStateView::Revoked,
                    _ => {
                        return Err(StoreError::Unavailable(
                            "stored instructor approval state is invalid".to_owned(),
                        ));
                    }
                };
                let revision = row
                    .try_get::<Option<i64>, _>("approval_revision")
                    .map_err(map_sqlx_error)?
                    .map(|value| {
                        TeachingOperationRevision::new(value as u64).ok_or_else(|| {
                            StoreError::Unavailable(
                                "stored instructor approval revision is invalid".to_owned(),
                            )
                        })
                    })
                    .transpose()?;
                Ok((
                    format!("{:010}", reference.number()),
                    question_model::SysadminInstructorCandidateView {
                        account: TeachingAccountView { reference, display },
                        approval: question_model::SysadminInstructorApprovalView {
                            state,
                            revision,
                        },
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let page = page_from_keyed_records(&mut records, request.size.get() as u16)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(question_model::SysadminInstructorCandidateSearchPage {
            candidates: page.items,
            next_cursor: page.next_cursor.map(|cursor| cursor.as_str().to_owned()),
        })
    }

    async fn search_course_co_instructor_targets(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        request: CoInstructorTargetSearchRequest,
    ) -> Result<CoInstructorTargetSearchPage, StoreError> {
        // This is one bounded SQL statement, so it needs no multi-query page
        // snapshot.  A normal tenant transaction also matches the broker's
        // runtime capability used by the physical ple_app oracle.
        let mut transaction = self.begin_tenant(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query(
            "SELECT * FROM public.ple_course_co_instructor_target_search($1, $2, $3, $4, $5)",
        )
        .bind(actor.as_uuid())
        .bind(course.as_uuid())
        .bind(request.query.as_str())
        .bind(target_search_after(request.after.as_deref())?)
        .bind(database_page_limit(request.size.get())?)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = rows
            .into_iter()
            .map(|row| {
                let reference = account_reference(
                    row.try_get("account_public_id").map_err(map_sqlx_error)?,
                )?;
                let display_name: String = row
                    .try_get("account_display_name")
                    .map_err(map_sqlx_error)?;
                let display = TeachingDisplayLabel::try_from(display_name)
                .map_err(|error| StoreError::Unavailable(error.to_string()))?;
                let revision = row.try_get::<i64, _>("approval_revision").map_err(map_sqlx_error)?;
                let revision = TeachingOperationRevision::new(revision as u64).ok_or_else(|| {
                    StoreError::Unavailable("stored instructor approval revision is invalid".to_string())
                })?;
                Ok((
                    format!("{:010}", reference.number()),
                    CoInstructorTargetView {
                        account: TeachingAccountView { reference, display },
                        approval: AccountApprovalView {
                            state: question_model::teaching_operations::InstructorApprovalStateView::Approved,
                            revision,
                        },
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let page = page_from_keyed_records(&mut records, request.size.get() as u16)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CoInstructorTargetSearchPage {
            targets: page.items,
            next_cursor: page.next_cursor.map(|cursor| cursor.as_str().to_owned()),
        })
    }

    async fn own_account_reference(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
    ) -> Result<OwnAccountReferenceView, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        set_presented_session_hash(&mut transaction, session).await?;
        let row = sqlx::query(
            "SELECT public_id, display_name FROM public.ple_own_account_reference($1, $2)",
        )
        .bind(session.to_string())
        .bind(context.tenant_id().as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let result = OwnAccountReferenceView {
            reference: account_reference(row.try_get("public_id").map_err(map_sqlx_error)?)?,
            display_name: row.try_get("display_name").map_err(map_sqlx_error)?,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_account_reference_for_operator(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
        reference: AccountReference,
    ) -> Result<Option<UserId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        set_presented_session_hash(&mut transaction, session).await?;
        let user: Option<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM public.ple_sysadmin_account_reference($1, $2)")
                .bind(session.to_string())
                .bind(database_public_reference(reference.number())?)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(user.map(UserId::from_uuid))
    }

    async fn resolve_approved_account_reference_for_course(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: AccountReference,
    ) -> Result<Option<UserId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let user: Option<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM public.ple_approved_account_reference($1)")
                .bind(database_public_reference(reference.number())?)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(user.map(UserId::from_uuid))
    }

    async fn list_course_co_instructor_invitation_reference_views(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: crate::PageRequest,
    ) -> Result<crate::Page<crate::CourseCoInstructorInvitationReferenceView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query(
            "SELECT * FROM public.ple_course_instructor_invitation_reference_list($1, $2, $3, $4)",
        )
        .bind(actor.as_uuid())
        .bind(course.as_uuid())
        .bind(page_after(&page)?)
        .bind(database_page_limit(u32::from(page.size.get()))?)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = rows
            .into_iter()
            .map(|row| {
                let reference = invitation_reference(
                    row.try_get("invitation_public_id")
                        .map_err(map_sqlx_error)?,
                )?;
                Ok((
                    format!("{:010}", reference.number()),
                    crate::CourseCoInstructorInvitationReferenceView {
                        reference,
                        target: account_reference(
                            row.try_get("target_public_id").map_err(map_sqlx_error)?,
                        )?,
                        target_display_name: row
                            .try_get("target_display_name")
                            .map_err(map_sqlx_error)?,
                        target_approval_state: approval_state(
                            row.try_get("target_is_approved").map_err(map_sqlx_error)?,
                        ),
                        target_approval_revision: crate::InstructorApprovalRevision::try_from_i64(
                            row.try_get("target_approval_revision")
                                .map_err(map_sqlx_error)?,
                        )?,
                        state: invitation_state(
                            &row.try_get::<String, _>("state").map_err(map_sqlx_error)?,
                        )?,
                        created_at: question_model::ActivityTimestamp::from_unix_millis(
                            row.try_get("created_at_millis").map_err(map_sqlx_error)?,
                        ),
                        expires_at: question_model::ActivityTimestamp::from_unix_millis(
                            row.try_get("expires_at_millis").map_err(map_sqlx_error)?,
                        ),
                        revision: crate::CoInstructorInvitationRevision::try_from_i64(
                            row.try_get("revision").map_err(map_sqlx_error)?,
                        )?,
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn list_pending_co_instructor_invitation_reference_views(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
        page: crate::PageRequest,
    ) -> Result<crate::Page<crate::PendingCoInstructorInvitationReferenceView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        set_presented_session_hash(&mut transaction, session).await?;
        let rows = sqlx::query(
            "SELECT * FROM public.ple_pending_instructor_invitation_reference_list($1, $2, $3, $4)",
        )
        .bind(session.to_string())
        .bind(context.tenant_id().as_uuid())
        .bind(page_after(&page)?)
        .bind(database_page_limit(u32::from(page.size.get()))?)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = rows
            .into_iter()
            .map(|row| {
                let reference = invitation_reference(
                    row.try_get("invitation_public_id")
                        .map_err(map_sqlx_error)?,
                )?;
                Ok((
                    format!("{:010}", reference.number()),
                    crate::PendingCoInstructorInvitationReferenceView {
                        reference,
                        course_title: row.try_get("course_title").map_err(map_sqlx_error)?,
                        expires_at: question_model::ActivityTimestamp::from_unix_millis(
                            row.try_get("expires_at_millis").map_err(map_sqlx_error)?,
                        ),
                        revision: crate::CoInstructorInvitationRevision::try_from_i64(
                            row.try_get("revision").map_err(map_sqlx_error)?,
                        )?,
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        if records.is_empty() {
            let target: Option<Uuid> =
                sqlx::query_scalar("SELECT user_id FROM public.ple_target_session_subject($1, $2)")
                    .bind(session.to_string())
                    .bind(context.tenant_id().as_uuid())
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
            if target.is_none() {
                return Err(StoreError::NotFound);
            }
        }
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn list_course_membership_reference_views(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: crate::PageRequest,
    ) -> Result<crate::Page<crate::CourseMembershipReferenceView>, StoreError> {
        list_membership_reference_views(self, context, actor, course, None, page).await
    }

    async fn list_course_active_student_membership_reference_views(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: crate::PageRequest,
    ) -> Result<crate::Page<crate::CourseMembershipReferenceView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query(
            "SELECT * FROM public.ple_course_active_student_membership_reference_list(\
             $1, $2, $3, $4)",
        )
        .bind(actor.as_uuid())
        .bind(course.as_uuid())
        .bind(page_after(&page)?)
        .bind(database_page_limit(u32::from(page.size.get()))?)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = rows
            .into_iter()
            .map(membership_reference_view_record)
            .collect::<Result<Vec<_>, StoreError>>()?;
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn list_course_instructor_membership_reference_views(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: crate::PageRequest,
    ) -> Result<crate::CourseInstructorMembershipReferencePage, StoreError> {
        let mut transaction = self.begin_tenant_writable_snapshot(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let roster_revision: Option<i64> =
            sqlx::query_scalar("SELECT public.ple_course_instructor_roster_revision($1, $2)")
                .bind(actor.as_uuid())
                .bind(course.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let roster_revision = roster_revision
            .ok_or(StoreError::NotFound)
            .and_then(crate::RosterRevision::from_stored)?;
        let rows = sqlx::query(
            "SELECT * FROM public.ple_course_instructor_membership_reference_list($1, $2, $3, $4)",
        )
        .bind(actor.as_uuid())
        .bind(course.as_uuid())
        .bind(page_after(&page)?)
        .bind(database_page_limit(u32::from(page.size.get()))?)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = rows
            .into_iter()
            .map(|row| {
                let membership = membership_reference(
                    row.try_get("membership_public_id")
                        .map_err(map_sqlx_error)?,
                )?;
                Ok((
                    format!("{:010}", membership.number()),
                    crate::CourseInstructorMembershipReferenceView {
                        membership,
                        account: account_reference(
                            row.try_get("account_public_id").map_err(map_sqlx_error)?,
                        )?,
                        account_display_name: row
                            .try_get("account_display_name")
                            .map_err(map_sqlx_error)?,
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let result = crate::CourseInstructorMembershipReferencePage {
            page: page_from_keyed_records(&mut records, page.size.get())?,
            roster_revision,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn list_course_group_membership_reference_views(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        group: question_model::CourseGroupReference,
        page: crate::PageRequest,
    ) -> Result<crate::Page<crate::CourseMembershipReferenceView>, StoreError> {
        list_membership_reference_views(
            self,
            context,
            actor,
            course,
            Some(group.number() as i32),
            page,
        )
        .await
    }

    async fn course_membership_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        membership: CourseMembershipId,
    ) -> Result<Option<CourseMembershipReference>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let value: Option<i32> = sqlx::query_scalar(
            "SELECT public_id FROM course_member WHERE tenant_id=$1 AND course_id=$2 \
             AND course_membership_id=$3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(membership.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        value.map(membership_reference).transpose()
    }

    async fn resolve_course_membership_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: CourseMembershipReference,
    ) -> Result<Option<CourseMembershipId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let membership: Option<Uuid> = sqlx::query_scalar(
            "SELECT course_membership_id FROM course_member WHERE tenant_id=$1 AND course_id=$2 \
             AND public_id=$3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(database_public_reference(reference.number())?)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(membership.map(CourseMembershipId::from_uuid))
    }

    async fn resolve_active_student_target_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: CourseMembershipReference,
    ) -> Result<Option<crate::InstructorStudentTargetView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let row = sqlx::query(
            "SELECT course_membership_id, user_id, student_id FROM course_member \
             WHERE tenant_id=$1 AND course_id=$2 AND public_id=$3 AND role='student' \
             AND status='active' AND student_id IS NOT NULL",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(database_public_reference(reference.number())?)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .map(|row| {
                Ok::<crate::InstructorStudentTargetView, StoreError>(
                    crate::InstructorStudentTargetView {
                        course,
                        membership: CourseMembershipId::from_uuid(
                            row.try_get("course_membership_id")
                                .map_err(map_sqlx_error)?,
                        ),
                        user: UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_error)?),
                        student: StudentId::from_uuid(
                            row.try_get("student_id").map_err(map_sqlx_error)?,
                        ),
                    },
                )
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn active_student_membership_reference_view(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        student: StudentId,
    ) -> Result<Option<crate::CourseMembershipReferenceView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let row = sqlx::query(
            "SELECT * FROM public.ple_course_active_student_membership_reference($1, $2, $3)",
        )
        .bind(actor.as_uuid())
        .bind(course.as_uuid())
        .bind(student.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .map(|row| {
                Ok::<crate::CourseMembershipReferenceView, StoreError>(
                    crate::CourseMembershipReferenceView {
                        reference: membership_reference(
                            row.try_get("membership_public_id")
                                .map_err(map_sqlx_error)?,
                        )?,
                        display_name: row.try_get("display_name").map_err(map_sqlx_error)?,
                        role: membership_role(
                            &row.try_get::<String, _>("role").map_err(map_sqlx_error)?,
                        )?,
                        status: membership_status(
                            &row.try_get::<String, _>("status").map_err(map_sqlx_error)?,
                        )?,
                    },
                )
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn co_instructor_invitation_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        invitation: question_model::CoInstructorInvitationId,
    ) -> Result<Option<CoInstructorInvitationReference>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let value: Option<i32> = sqlx::query_scalar(
            "SELECT public_id FROM course_instructor_invitation WHERE tenant_id=$1 \
             AND course_id=$2 \
             AND invitation_id=$3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(invitation.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        value.map(invitation_reference).transpose()
    }

    async fn resolve_pending_co_instructor_invitation_reference(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
        reference: CoInstructorInvitationReference,
    ) -> Result<Option<question_model::CoInstructorInvitationId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        set_presented_session_hash(&mut transaction, session).await?;
        let target: Option<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM public.ple_target_session_subject($1, $2)")
                .bind(session.to_string())
                .bind(context.tenant_id().as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let Some(target) = target else {
            return Err(StoreError::NotFound);
        };
        let invitation: Option<Uuid> = sqlx::query_scalar(
            "SELECT invitation_id FROM course_instructor_invitation WHERE tenant_id=$1 \
             AND public_id=$2 AND target_user_id=$3 AND status='pending' \
             AND expires_at > transaction_timestamp()",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(database_public_reference(reference.number())?)
        .bind(target)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(invitation.map(question_model::CoInstructorInvitationId::from_uuid))
    }

    async fn resolve_pending_course_co_instructor_invitation_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: CoInstructorInvitationReference,
    ) -> Result<Option<question_model::CoInstructorInvitationId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let invitation: Option<Uuid> = sqlx::query_scalar(
            "SELECT invitation_id FROM course_instructor_invitation WHERE tenant_id=$1 \
             AND course_id=$2 AND public_id=$3 AND status='pending' \
             AND expires_at > transaction_timestamp()",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(database_public_reference(reference.number())?)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(invitation.map(question_model::CoInstructorInvitationId::from_uuid))
    }
}

async fn set_presented_session_hash(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: crate::SessionTokenHash,
) -> Result<(), StoreError> {
    sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
        .bind(session.to_string())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    Ok(())
}

async fn list_membership_reference_views(
    store: &PostgresStore,
    context: TenantContext,
    actor: UserId,
    course: CourseId,
    group: Option<i32>,
    page: crate::PageRequest,
) -> Result<crate::Page<crate::CourseMembershipReferenceView>, StoreError> {
    let mut transaction = store.begin_tenant(context).await?;
    if !postgres_is_course_instructor(&mut transaction, context.tenant_id(), course, actor).await? {
        return Err(StoreError::NotFound);
    }
    let rows = sqlx::query(
        "SELECT * FROM public.ple_course_membership_reference_list($1, $2, $3, $4, $5)",
    )
    .bind(actor.as_uuid())
    .bind(course.as_uuid())
    .bind(group)
    .bind(page_after(&page)?)
    .bind(database_page_limit(u32::from(page.size.get()))?)
    .fetch_all(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut records = rows
        .into_iter()
        .map(membership_reference_view_record)
        .collect::<Result<Vec<_>, StoreError>>()?;
    let result = page_from_keyed_records(&mut records, page.size.get())?;
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}

fn membership_reference_view_record(
    row: sqlx::postgres::PgRow,
) -> Result<(String, crate::CourseMembershipReferenceView), StoreError> {
    let reference = membership_reference(
        row.try_get("membership_public_id")
            .map_err(map_sqlx_error)?,
    )?;
    Ok((
        format!("{:010}", reference.number()),
        crate::CourseMembershipReferenceView {
            reference,
            display_name: row.try_get("display_name").map_err(map_sqlx_error)?,
            role: membership_role(&row.try_get::<String, _>("role").map_err(map_sqlx_error)?)?,
            status: membership_status(
                &row.try_get::<String, _>("status").map_err(map_sqlx_error)?,
            )?,
        },
    ))
}
