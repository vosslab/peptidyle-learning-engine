//! Memory authorization boundary for T2 public teaching-authority locators.

use async_trait::async_trait;
use question_model::{
    AccountReference, CoInstructorInvitationReference, CoInstructorInvitationState, CourseId,
    CourseMembershipId, CourseMembershipReference, CourseMembershipRole, StudentId, UserId,
};

use super::*;
use crate::{OwnAccountReferenceView, TeachingAuthorityReferenceStore};

#[async_trait]
impl TeachingAuthorityReferenceStore for MemoryStore {
    async fn search_sysadmin_instructor_candidates(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
        request: question_model::SysadminInstructorCandidateSearchRequest,
    ) -> Result<question_model::SysadminInstructorCandidateSearchPage, StoreError> {
        let mut state = self.write_state()?;
        super::teaching_authority::require_session_sysadmin(&state, context, session)?;
        let query = request.query.as_str().to_lowercase();
        let mut records = state
            .accounts
            .iter()
            .filter(|(_, account)| account.display_name.to_lowercase().contains(&query))
            .map(|(user, account)| (*user, account.display_name.clone()))
            .collect::<Vec<_>>();
        records.sort_by_key(|(user, _)| *user);

        let mut views = Vec::with_capacity(records.len());
        for (user, display_name) in records {
            let reference =
                super::navigation_references::ensure_account_reference(&mut state, user)?;
            let approval = state.instructor_approvals.get(&user).copied();
            let (state_view, revision) = match approval {
                None => (
                    question_model::SysadminInstructorApprovalStateView::Unapproved,
                    None,
                ),
                Some(stored) if stored.approval.revoked_at.is_none() => (
                    question_model::SysadminInstructorApprovalStateView::Approved,
                    question_model::TeachingOperationRevision::new(stored.revision.as_i64() as u64),
                ),
                Some(stored) => (
                    question_model::SysadminInstructorApprovalStateView::Revoked,
                    question_model::TeachingOperationRevision::new(stored.revision.as_i64() as u64),
                ),
            };
            views.push((
                format!("{:010}", reference.number()),
                question_model::SysadminInstructorCandidateView {
                    account: question_model::TeachingAccountView {
                        reference,
                        display: question_model::TeachingDisplayLabel::try_from(display_name)
                            .map_err(|error| StoreError::InvalidRecord(error.to_owned()))?,
                    },
                    approval: question_model::SysadminInstructorApprovalView {
                        state: state_view,
                        revision,
                    },
                },
            ));
        }
        let page = super::catalog::page_records(
            views,
            &crate::PageRequest {
                after: request
                    .after
                    .map(crate::Cursor::parse)
                    .transpose()
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
                size: crate::PageSize::new(request.size.get() as u16)
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
            },
        );
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
        request: question_model::CoInstructorTargetSearchRequest,
    ) -> Result<question_model::CoInstructorTargetSearchPage, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;

        let now = state.authoritative_time;
        let query = request.query.as_str().to_lowercase();
        let excluded_instructors = state
            .course_memberships
            .iter()
            .filter(|((record_tenant, _), membership)| {
                *record_tenant == tenant
                    && membership.course == course
                    && membership.role == CourseMembershipRole::Instructor
                    && membership.status == crate::CourseMemberStatus::Active
            })
            .map(|(_, membership)| membership.user)
            .collect::<std::collections::BTreeSet<_>>();
        let pending_targets = state
            .co_instructor_invitations
            .iter()
            .filter(|((record_tenant, _), stored)| {
                *record_tenant == tenant
                    && stored.invitation.course == course
                    && domain::teaching_authority::invitation_state(&stored.invitation, now).ok()
                        == Some(CoInstructorInvitationState::Pending)
            })
            .map(|(_, stored)| stored.invitation.target)
            .collect::<std::collections::BTreeSet<_>>();
        let users = state
            .accounts
            .iter()
            .filter_map(|(user, account)| {
                let approval = state.instructor_approvals.get(user)?;
                let active = approval.approval.revoked_at.is_none()
                    && domain::teaching_authority::validate_instructor_approval(
                        &approval.approval,
                        now,
                    )
                    .is_ok();
                (active
                    && !excluded_instructors.contains(user)
                    && !pending_targets.contains(user)
                    && account.display_name.to_lowercase().contains(&query))
                .then_some(*user)
            })
            .collect::<Vec<_>>();

        let mut records = Vec::with_capacity(users.len());
        for user in users {
            let display_name = state
                .accounts
                .get(&user)
                .ok_or(StoreError::NotFound)?
                .display_name
                .clone();
            let display = question_model::TeachingDisplayLabel::try_from(display_name)
                .map_err(|error| StoreError::InvalidRecord(error.to_owned()))?;
            let approval = state
                .instructor_approvals
                .get(&user)
                .copied()
                .ok_or(StoreError::NotFound)?;
            let revision =
                question_model::TeachingOperationRevision::new(approval.revision.as_i64() as u64)
                    .ok_or_else(|| {
                    StoreError::InvalidRecord("invalid instructor approval revision".to_owned())
                })?;
            let reference =
                super::navigation_references::ensure_account_reference(&mut state, user)?;
            records.push((
                format!("{:010}", reference.number()),
                question_model::CoInstructorTargetView {
                    account: question_model::TeachingAccountView { reference, display },
                    approval: question_model::AccountApprovalView {
                        state: question_model::teaching_operations::InstructorApprovalStateView::Approved,
                        revision,
                    },
                },
            ));
        }
        let page = super::catalog::page_records(
            records,
            &crate::PageRequest {
                after: request
                    .after
                    .map(crate::Cursor::parse)
                    .transpose()
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
                size: crate::PageSize::new(request.size.get() as u16)
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
            },
        );
        Ok(question_model::CoInstructorTargetSearchPage {
            targets: page.items,
            next_cursor: page.next_cursor.map(|cursor| cursor.as_str().to_owned()),
        })
    }

    async fn own_account_reference(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
    ) -> Result<OwnAccountReferenceView, StoreError> {
        let mut state = self.write_state()?;
        let user = super::sessions::active_subject(&state, context, session)
            .ok_or(StoreError::NotFound)?
            .user();
        let account = state.accounts.get(&user).ok_or(StoreError::NotFound)?;
        let display_name = account.display_name.clone();
        let reference = super::navigation_references::ensure_account_reference(&mut state, user)?;
        Ok(OwnAccountReferenceView {
            reference,
            display_name,
        })
    }

    async fn resolve_account_reference_for_operator(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
        reference: AccountReference,
    ) -> Result<Option<UserId>, StoreError> {
        let state = self.read_state()?;
        super::teaching_authority::require_session_sysadmin(&state, context, session)?;
        Ok(state.accounts_by_reference.get(&reference).copied())
    }

    async fn resolve_approved_account_reference_for_course(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: AccountReference,
    ) -> Result<Option<UserId>, StoreError> {
        let state = self.read_state()?;
        super::teaching_authority::require_direct_instructor(
            &state,
            context.tenant_id(),
            course,
            actor,
        )?;
        let Some(user) = state.accounts_by_reference.get(&reference).copied() else {
            return Ok(None);
        };
        let approved = state.instructor_approvals.get(&user).is_some_and(|stored| {
            stored.approval.revoked_at.is_none()
                && domain::teaching_authority::validate_instructor_approval(
                    &stored.approval,
                    state.authoritative_time,
                )
                .is_ok()
        });
        Ok(approved.then_some(user))
    }

    async fn list_course_co_instructor_invitation_reference_views(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: crate::PageRequest,
    ) -> Result<crate::Page<crate::CourseCoInstructorInvitationReferenceView>, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        let now = state.authoritative_time;
        let invitations = state
            .co_instructor_invitations
            .iter()
            .filter(|((record_tenant, _), stored)| {
                *record_tenant == tenant && stored.invitation.course == course
            })
            .map(|((_, id), stored)| (*id, stored.clone()))
            .collect::<Vec<_>>();
        let mut records = Vec::with_capacity(invitations.len());
        for (id, stored) in invitations {
            let approval = state
                .instructor_approvals
                .get(&stored.invitation.target)
                .copied()
                .ok_or(StoreError::NotFound)?;
            let target_display_name = state
                .accounts
                .get(&stored.invitation.target)
                .ok_or(StoreError::NotFound)?
                .display_name
                .clone();
            let target = super::navigation_references::ensure_account_reference(
                &mut state,
                stored.invitation.target,
            )?;
            let reference =
                super::navigation_references::ensure_co_instructor_invitation_reference(
                    &mut state, tenant, id,
                )?;
            records.push((
                format!("{:010}", reference.number()),
                crate::CourseCoInstructorInvitationReferenceView {
                    reference,
                    target,
                    target_display_name,
                    target_approval_state: if approval.approval.revoked_at.is_none() {
                        question_model::teaching_operations::InstructorApprovalStateView::Approved
                    } else {
                        question_model::teaching_operations::InstructorApprovalStateView::Revoked
                    },
                    target_approval_revision: approval.revision,
                    state: domain::teaching_authority::invitation_state(&stored.invitation, now)
                        .map_err(|error| StoreError::InvalidRecord(format!("{error:?}")))?,
                    created_at: stored.invitation.created_at,
                    expires_at: stored.invitation.expires_at,
                    revision: stored.revision,
                },
            ));
        }
        Ok(super::catalog::page_records(records, &page))
    }

    async fn list_pending_co_instructor_invitation_reference_views(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
        page: crate::PageRequest,
    ) -> Result<crate::Page<crate::PendingCoInstructorInvitationReferenceView>, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let target = super::sessions::active_subject(&state, context, session)
            .ok_or(StoreError::NotFound)?
            .user();
        let now = state.authoritative_time;
        let invitations = state
            .co_instructor_invitations
            .iter()
            .filter(|((record_tenant, _), stored)| {
                *record_tenant == tenant
                    && stored.invitation.target == target
                    && domain::teaching_authority::invitation_state(&stored.invitation, now).ok()
                        == Some(CoInstructorInvitationState::Pending)
            })
            .map(|((_, id), stored)| (*id, stored.clone()))
            .collect::<Vec<_>>();
        let mut records = Vec::with_capacity(invitations.len());
        for (id, stored) in invitations {
            let course_title = state
                .courses
                .get(&(tenant, stored.invitation.course))
                .ok_or(StoreError::NotFound)?
                .title
                .clone();
            let reference =
                super::navigation_references::ensure_co_instructor_invitation_reference(
                    &mut state, tenant, id,
                )?;
            records.push((
                format!("{:010}", reference.number()),
                crate::PendingCoInstructorInvitationReferenceView {
                    reference,
                    course_title,
                    expires_at: stored.invitation.expires_at,
                    revision: stored.revision,
                },
            ));
        }
        Ok(super::catalog::page_records(records, &page))
    }

    async fn list_course_membership_reference_views(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: crate::PageRequest,
    ) -> Result<crate::Page<crate::CourseMembershipReferenceView>, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        let memberships = state
            .course_memberships
            .iter()
            .filter(|((record_tenant, _), membership)| {
                *record_tenant == tenant && membership.course == course
            })
            .map(|((_, id), membership)| (*id, membership.clone()))
            .collect::<Vec<_>>();
        membership_reference_page(&mut state, tenant, memberships, &page)
    }

    async fn list_course_active_student_membership_reference_views(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: crate::PageRequest,
    ) -> Result<crate::Page<crate::CourseMembershipReferenceView>, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        let memberships = state
            .course_memberships
            .iter()
            .filter(|((record_tenant, _), membership)| {
                *record_tenant == tenant
                    && membership.course == course
                    && membership.role == CourseMembershipRole::Student
                    && membership.status == crate::CourseMemberStatus::Active
            })
            .map(|((_, id), membership)| (*id, membership.clone()))
            .collect::<Vec<_>>();
        membership_reference_page(&mut state, tenant, memberships, &page)
    }

    async fn list_course_instructor_membership_reference_views(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: crate::PageRequest,
    ) -> Result<crate::CourseInstructorMembershipReferencePage, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        let roster_revision = super::course_roster::roster_policy(&state, tenant, course).revision;
        let memberships = state
            .course_memberships
            .iter()
            .filter(|((record_tenant, _), membership)| {
                *record_tenant == tenant
                    && membership.course == course
                    && membership.role == CourseMembershipRole::Instructor
                    && membership.status == crate::CourseMemberStatus::Active
            })
            .map(|((_, id), membership)| (*id, membership.clone()))
            .collect::<Vec<_>>();
        let mut records = Vec::with_capacity(memberships.len());
        for (id, membership) in memberships {
            let user = membership.user;
            let account_display_name = state
                .accounts
                .get(&user)
                .ok_or(StoreError::NotFound)?
                .display_name
                .clone();
            let membership = super::navigation_references::ensure_course_membership_reference(
                &mut state, tenant, id,
            )?;
            let account = super::navigation_references::ensure_account_reference(&mut state, user)?;
            records.push((
                format!("{:010}", membership.number()),
                crate::CourseInstructorMembershipReferenceView {
                    membership,
                    account,
                    account_display_name,
                },
            ));
        }
        Ok(crate::CourseInstructorMembershipReferencePage {
            page: super::catalog::page_records(records, &page),
            roster_revision,
        })
    }

    async fn list_course_group_membership_reference_views(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        group: question_model::CourseGroupReference,
        page: crate::PageRequest,
    ) -> Result<crate::Page<crate::CourseMembershipReferenceView>, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        let group = state
            .course_groups_by_reference
            .get(&(tenant, group))
            .copied()
            .and_then(|id| state.course_groups.get(&(tenant, id)))
            .filter(|record| record.course == course)
            .ok_or(StoreError::NotFound)?;
        let memberships = group
            .members
            .iter()
            .filter_map(|id| {
                state
                    .course_memberships
                    .get(&(tenant, *id))
                    .map(|record| (*id, record.clone()))
            })
            .collect::<Vec<_>>();
        membership_reference_page(&mut state, tenant, memberships, &page)
    }

    async fn course_membership_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        membership: CourseMembershipId,
    ) -> Result<Option<CourseMembershipReference>, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        let matches_course = state
            .course_memberships
            .get(&(tenant, membership))
            .is_some_and(|record| record.course == course);
        matches_course
            .then(|| {
                super::navigation_references::ensure_course_membership_reference(
                    &mut state, tenant, membership,
                )
            })
            .transpose()
    }

    async fn resolve_course_membership_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: CourseMembershipReference,
    ) -> Result<Option<CourseMembershipId>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        Ok(state
            .course_memberships_by_reference
            .get(&(tenant, reference))
            .copied()
            .filter(|membership| {
                state
                    .course_memberships
                    .get(&(tenant, *membership))
                    .is_some_and(|record| record.course == course)
            }))
    }

    async fn resolve_active_student_target_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: CourseMembershipReference,
    ) -> Result<Option<crate::InstructorStudentTargetView>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        Ok(state
            .course_memberships_by_reference
            .get(&(tenant, reference))
            .and_then(|membership| state.course_memberships.get(&(tenant, *membership)))
            .filter(|record| {
                record.course == course
                    && record.role == CourseMembershipRole::Student
                    && record.status == crate::CourseMemberStatus::Active
            })
            .and_then(|record| {
                record
                    .student
                    .map(|student| crate::InstructorStudentTargetView {
                        course: record.course,
                        membership: record.id,
                        user: record.user,
                        student,
                    })
            }))
    }

    async fn active_student_membership_reference_view(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        student: StudentId,
    ) -> Result<Option<crate::CourseMembershipReferenceView>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        let Some((membership, record)) =
            state
                .course_memberships
                .iter()
                .find(|((record_tenant, _), record)| {
                    *record_tenant == tenant
                        && record.course == course
                        && record.student == Some(student)
                        && record.role == CourseMembershipRole::Student
                        && record.status == crate::CourseMemberStatus::Active
                })
        else {
            return Ok(None);
        };
        let reference = state
            .course_membership_references
            .get(&(tenant, membership.1))
            .copied()
            .ok_or(StoreError::NotFound)?;
        let display_name = state
            .accounts
            .get(&record.user)
            .map(|account| account.display_name.clone())
            .or_else(|| {
                state
                    .roster_profiles
                    .get(&(tenant, course, membership.1))
                    .map(|profile| profile.display_name.clone())
            })
            .ok_or(StoreError::NotFound)?;
        Ok(Some(crate::CourseMembershipReferenceView {
            reference,
            display_name,
            role: record.role,
            status: record.status,
        }))
    }

    async fn co_instructor_invitation_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        invitation: question_model::CoInstructorInvitationId,
    ) -> Result<Option<CoInstructorInvitationReference>, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        let matches_course = state
            .co_instructor_invitations
            .get(&(tenant, invitation))
            .is_some_and(|stored| stored.invitation.course == course);
        matches_course
            .then(|| {
                super::navigation_references::ensure_co_instructor_invitation_reference(
                    &mut state, tenant, invitation,
                )
            })
            .transpose()
    }

    async fn resolve_pending_co_instructor_invitation_reference(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
        reference: CoInstructorInvitationReference,
    ) -> Result<Option<question_model::CoInstructorInvitationId>, StoreError> {
        let state = self.read_state()?;
        let target = super::sessions::active_subject(&state, context, session)
            .ok_or(StoreError::NotFound)?
            .user();
        let tenant = context.tenant_id();
        Ok(state
            .co_instructor_invitations_by_reference
            .get(&(tenant, reference))
            .copied()
            .filter(|invitation| {
                state
                    .co_instructor_invitations
                    .get(&(tenant, *invitation))
                    .is_some_and(|stored| {
                        stored.invitation.target == target
                            && domain::teaching_authority::invitation_state(
                                &stored.invitation,
                                state.authoritative_time,
                            )
                            .ok()
                                == Some(CoInstructorInvitationState::Pending)
                    })
            }))
    }

    async fn resolve_pending_course_co_instructor_invitation_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: CoInstructorInvitationReference,
    ) -> Result<Option<question_model::CoInstructorInvitationId>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        super::teaching_authority::require_direct_instructor(&state, tenant, course, actor)?;
        Ok(state
            .co_instructor_invitations_by_reference
            .get(&(tenant, reference))
            .copied()
            .filter(|invitation| {
                state
                    .co_instructor_invitations
                    .get(&(tenant, *invitation))
                    .is_some_and(|stored| {
                        stored.invitation.course == course
                            && domain::teaching_authority::invitation_state(
                                &stored.invitation,
                                state.authoritative_time,
                            )
                            .ok()
                                == Some(CoInstructorInvitationState::Pending)
                    })
            }))
    }
}

fn membership_reference_page(
    state: &mut State,
    tenant: question_model::TenantId,
    memberships: Vec<(CourseMembershipId, crate::CourseMembershipRecord)>,
    page: &crate::PageRequest,
) -> Result<crate::Page<crate::CourseMembershipReferenceView>, StoreError> {
    let mut records = Vec::with_capacity(memberships.len());
    for (id, membership) in memberships {
        let display_name = state
            .accounts
            .get(&membership.user)
            .map(|account| account.display_name.clone())
            .or_else(|| {
                state
                    .roster_profiles
                    .get(&(tenant, membership.course, id))
                    .map(|profile| profile.display_name.clone())
            })
            .ok_or(StoreError::NotFound)?;
        let reference =
            super::navigation_references::ensure_course_membership_reference(state, tenant, id)?;
        records.push((
            format!("{:010}", reference.number()),
            crate::CourseMembershipReferenceView {
                reference,
                display_name,
                role: membership.role,
                status: membership.status,
            },
        ));
    }
    Ok(super::catalog::page_records(records, page))
}
