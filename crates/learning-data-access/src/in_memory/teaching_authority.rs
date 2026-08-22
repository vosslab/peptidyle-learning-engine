//! Atomic Memory implementation of T2 operator approvals and co-instructor invitations.

use super::*;
use crate::{
    ApproveInstructorAccount, CoInstructorInvitationRevision, CreateCoInstructorInvitation,
    DirectInstructorMembershipView, InstructorApprovalRevision, RemoveDirectInstructorMembership,
    RespondToCoInstructorInvitation, RevokeCoInstructorInvitation, RevokeInstructorApproval,
    StoredCoInstructorInvitation, StoredInstructorApproval, TeachingAuthorityStore,
};
use async_trait::async_trait;
use question_model::{
    CoInstructorInvitation, CoInstructorInvitationId, CoInstructorInvitationState,
    CourseMembershipId, CourseMembershipRole, UserId, UserRole,
};

#[async_trait]
impl TeachingAuthorityStore for MemoryStore {
    async fn approve_instructor_account(
        &self,
        context: TenantContext,
        command: ApproveInstructorAccount,
    ) -> Result<StoredInstructorApproval, StoreError> {
        let mut state = self.write_state()?;
        let operator = require_session_sysadmin(&state, context, command.session)?;
        if !state.accounts.contains_key(&command.target) {
            return Err(StoreError::NotFound);
        }
        let now = state.authoritative_time;
        let stored = match state.instructor_approvals.get(&command.target).copied() {
            None if command.expected_revision.is_none() => StoredInstructorApproval {
                approval: question_model::InstructorApproval {
                    user: command.target,
                    approved_by: operator,
                    approved_at: now,
                    revoked_at: None,
                },
                revision: InstructorApprovalRevision::INITIAL,
            },
            Some(current) if command.expected_revision == Some(current.revision) => {
                StoredInstructorApproval {
                    approval: question_model::InstructorApproval {
                        user: command.target,
                        approved_by: operator,
                        approved_at: now,
                        revoked_at: None,
                    },
                    revision: current.revision.next()?,
                }
            }
            _ => return Err(StoreError::Conflict),
        };
        domain::teaching_authority::validate_instructor_approval(&stored.approval, now)
            .map_err(domain_error)?;
        state.instructor_approvals.insert(command.target, stored);
        Ok(stored)
    }

    async fn revoke_instructor_approval(
        &self,
        context: TenantContext,
        command: RevokeInstructorApproval,
    ) -> Result<StoredInstructorApproval, StoreError> {
        let mut state = self.write_state()?;
        require_session_sysadmin(&state, context, command.session)?;
        let now = state.authoritative_time;
        let current = state
            .instructor_approvals
            .get(&command.target)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current.revision != command.expected_revision || current.approval.revoked_at.is_some() {
            return Err(StoreError::Conflict);
        }
        let stored = StoredInstructorApproval {
            approval: question_model::InstructorApproval {
                revoked_at: Some(now),
                ..current.approval
            },
            revision: current.revision.next()?,
        };
        domain::teaching_authority::validate_instructor_approval(&stored.approval, now)
            .map_err(domain_error)?;
        state.instructor_approvals.insert(command.target, stored);
        Ok(stored)
    }

    async fn create_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: CreateCoInstructorInvitation,
    ) -> Result<StoredCoInstructorInvitation, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let invited_by = require_direct_instructor(&state, tenant, command.course, command.actor)?;
        if !state.accounts.contains_key(&command.target) {
            return Err(StoreError::NotFound);
        }
        let now = state.authoritative_time;
        let approval = state.instructor_approvals.get(&command.target).copied();
        require_current_approval(approval, command.target, now)?;
        for ((record_tenant, _), stored) in &state.co_instructor_invitations {
            if *record_tenant == tenant
                && stored.invitation.course == command.course
                && stored.invitation.target == command.target
                && domain::teaching_authority::invitation_state(&stored.invitation, now)
                    .map_err(domain_error)?
                    == CoInstructorInvitationState::Pending
            {
                return Ok(stored.clone());
            }
        }
        let expires_at = ActivityTimestamp::from_unix_millis(
            now.as_unix_millis()
                .checked_add(domain::teaching_authority::CO_INSTRUCTOR_INVITATION_LIFETIME_MILLIS)
                .ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "co-instructor invitation expiry overflow".to_string(),
                    )
                })?,
        );
        let invitation = CoInstructorInvitation {
            id: fresh_invitation_id()?,
            course: command.course,
            invited_by,
            target: command.target,
            created_at: now,
            expires_at,
            accepted_at: None,
            declined_at: None,
            revoked_at: None,
        };
        domain::teaching_authority::invitation_state(&invitation, now).map_err(domain_error)?;
        let stored = StoredCoInstructorInvitation {
            invitation: invitation.clone(),
            revision: CoInstructorInvitationRevision::INITIAL,
        };
        state
            .co_instructor_invitations
            .insert((tenant, invitation.id), stored.clone());
        Ok(stored)
    }

    async fn list_course_co_instructor_invitations(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<StoredCoInstructorInvitation>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        require_direct_instructor(&state, tenant, course, actor)?;
        let records = state
            .co_instructor_invitations
            .iter()
            .filter(|((record_tenant, _), stored)| {
                *record_tenant == tenant && stored.invitation.course == course
            })
            .map(|((_, id), stored)| (id.as_uuid().to_string(), stored.clone()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn list_pending_co_instructor_invitations(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<StoredCoInstructorInvitation>, StoreError> {
        let state = self.read_state()?;
        let target = super::sessions::active_subject(&state, context, session)
            .ok_or(StoreError::NotFound)?
            .user();
        let now = state.authoritative_time;
        let records = state
            .co_instructor_invitations
            .iter()
            .filter(|((tenant, _), stored)| {
                *tenant == context.tenant_id()
                    && stored.invitation.target == target
                    && domain::teaching_authority::invitation_state(&stored.invitation, now).ok()
                        == Some(CoInstructorInvitationState::Pending)
            })
            .map(|((_, id), stored)| (id.as_uuid().to_string(), stored.clone()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn accept_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: RespondToCoInstructorInvitation,
    ) -> Result<DirectInstructorMembershipView, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let snapshot = state.clone();
        let now = state.authoritative_time;
        let key = (tenant, command.invitation);
        let stored = state
            .co_instructor_invitations
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if domain::teaching_authority::invitation_state(&stored.invitation, now)
            .map_err(domain_error)?
            == CoInstructorInvitationState::Accepted
        {
            if command.actor != stored.invitation.target {
                return Err(StoreError::NotFound);
            }
            return invitation_acceptance_view(&state, tenant, command.invitation);
        }
        if stored.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let approval = state
            .instructor_approvals
            .get(&stored.invitation.target)
            .copied();
        if command.actor != stored.invitation.target {
            return Err(StoreError::NotFound);
        }
        if let Err(error) = domain::teaching_authority::accept_co_instructor_invitation(
            &stored.invitation,
            command.actor,
            approval.map(|record| record.approval),
            now,
        ) {
            return Err(domain_error(error));
        }
        let membership = match super::entitlement::active_membership_for(
            &state,
            tenant,
            stored.invitation.course,
            stored.invitation.target,
        ) {
            Some(record) if record.role == CourseMembershipRole::Instructor => record.id,
            Some(_) => return Err(StoreError::Conflict),
            None => match create_direct_instructor(
                &mut state,
                tenant,
                stored.invitation.course,
                stored.invitation.target,
            ) {
                Ok(value) => value,
                Err(error) => {
                    *state = snapshot;
                    return Err(error);
                }
            },
        };
        let mut accepted = stored;
        accepted.invitation.accepted_at = Some(now);
        accepted.revision = match accepted.revision.next() {
            Ok(value) => value,
            Err(error) => {
                *state = snapshot;
                return Err(error);
            }
        };
        let course = accepted.invitation.course;
        state.co_instructor_invitations.insert(key, accepted);
        state
            .co_instructor_invitation_acceptances
            .insert(key, membership);
        if let Err(error) =
            super::course_roster::bump_roster_revision(&mut state, tenant, course, None)
        {
            *state = snapshot;
            return Err(error);
        }
        invitation_acceptance_view(&state, tenant, command.invitation)
    }

    async fn decline_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: RespondToCoInstructorInvitation,
    ) -> Result<(), StoreError> {
        mutate_target_invitation(
            self,
            context,
            command,
            CoInstructorInvitationState::Declined,
        )
        .await
    }

    async fn revoke_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: RevokeCoInstructorInvitation,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        require_direct_instructor(&state, tenant, command.course, command.actor)?;
        let key = (tenant, command.invitation);
        let mut stored = state
            .co_instructor_invitations
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let now = state.authoritative_time;
        if stored.invitation.course != command.course
            || stored.revision != command.expected_revision
            || domain::teaching_authority::invitation_state(&stored.invitation, now)
                .map_err(domain_error)?
                != CoInstructorInvitationState::Pending
        {
            return Err(StoreError::Conflict);
        }
        stored.invitation.revoked_at = Some(now);
        stored.revision = stored.revision.next()?;
        state.co_instructor_invitations.insert(key, stored);
        Ok(())
    }

    async fn remove_direct_instructor_membership(
        &self,
        context: TenantContext,
        command: RemoveDirectInstructorMembership,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let snapshot = state.clone();
        require_direct_instructor(&state, tenant, command.course, command.actor)?;
        let key = (tenant, command.membership);
        let current = state
            .course_memberships
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if current.course != command.course || current.role != CourseMembershipRole::Instructor {
            return Err(StoreError::NotFound);
        }
        let roster_revision =
            super::course_roster::roster_policy(&state, tenant, command.course).revision;
        if roster_revision != command.expected_roster_revision {
            return Err(StoreError::Conflict);
        }
        let count = state
            .course_memberships
            .values()
            .filter(|record| {
                record.tenant == tenant
                    && record.course == command.course
                    && record.role == CourseMembershipRole::Instructor
                    && record.status == crate::CourseMemberStatus::Active
            })
            .count();
        domain::teaching_authority::refuse_final_instructor_removal(
            u32::try_from(count)
                .map_err(|_| StoreError::Unavailable("instructor count exceeds u32".to_string()))?,
        )
        .map_err(|_| StoreError::Conflict)?;
        let now = state.authoritative_time;
        let record = state
            .course_memberships
            .get_mut(&key)
            .ok_or(StoreError::NotFound)?;
        let removed_user = record.user;
        record.status = crate::CourseMemberStatus::Revoked;
        record.revoked_at = Some(now);
        state
            .active_course_membership_by_user
            .remove(&(tenant, command.course, removed_user));
        if let Err(error) = super::course_roster::bump_roster_revision(
            &mut state,
            tenant,
            command.course,
            Some(roster_revision),
        ) {
            *state = snapshot;
            return Err(error);
        }
        Ok(())
    }
}

async fn mutate_target_invitation(
    store: &MemoryStore,
    context: TenantContext,
    command: RespondToCoInstructorInvitation,
    terminal: CoInstructorInvitationState,
) -> Result<(), StoreError> {
    let tenant = context.tenant_id();
    let mut state = store.write_state()?;
    let key = (tenant, command.invitation);
    let mut stored = state
        .co_instructor_invitations
        .get(&key)
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let now = state.authoritative_time;
    if command.actor != stored.invitation.target {
        return Err(StoreError::NotFound);
    }
    if stored.revision != command.expected_revision
        || domain::teaching_authority::invitation_state(&stored.invitation, now)
            .map_err(domain_error)?
            != CoInstructorInvitationState::Pending
    {
        return Err(StoreError::Conflict);
    }
    match terminal {
        CoInstructorInvitationState::Declined => stored.invitation.declined_at = Some(now),
        _ => {
            return Err(StoreError::InvalidRecord(
                "unsupported invitation terminal state".to_string(),
            ));
        }
    }
    stored.revision = stored.revision.next()?;
    state.co_instructor_invitations.insert(key, stored);
    Ok(())
}

pub(super) fn require_session_sysadmin(
    state: &State,
    context: TenantContext,
    session: crate::SessionTokenHash,
) -> Result<UserId, StoreError> {
    let subject =
        super::sessions::active_subject(state, context, session).ok_or(StoreError::NotFound)?;
    subject
        .roles()
        .contains(&UserRole::Sysadmin)
        .then_some(subject.user())
        .ok_or(StoreError::Forbidden)
}

pub(super) fn require_direct_instructor(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> Result<CourseMembershipId, StoreError> {
    let record = super::entitlement::active_membership_for(state, tenant, course, actor)
        .filter(|record| record.role == CourseMembershipRole::Instructor)
        .ok_or(StoreError::NotFound)?;
    Ok(record.id)
}

fn require_current_approval(
    approval: Option<StoredInstructorApproval>,
    target: UserId,
    now: ActivityTimestamp,
) -> Result<(), StoreError> {
    let approval = approval.ok_or(StoreError::Forbidden)?;
    domain::teaching_authority::validate_instructor_approval(&approval.approval, now)
        .map_err(domain_error)?;
    if approval.approval.user != target || approval.approval.revoked_at.is_some() {
        return Err(StoreError::Forbidden);
    }
    Ok(())
}

fn create_direct_instructor(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> Result<CourseMembershipId, StoreError> {
    super::entitlement::create_initial_instructor_membership(state, tenant, course, user)
}

fn membership_view(
    state: &State,
    tenant: TenantId,
    membership: CourseMembershipId,
) -> Result<DirectInstructorMembershipView, StoreError> {
    let record = super::entitlement::active_membership_by_id(state, tenant, membership)
        .filter(|record| record.role == CourseMembershipRole::Instructor)
        .ok_or(StoreError::NotFound)?;
    let roster_revision =
        super::course_roster::roster_policy(state, tenant, record.course).revision;
    Ok(DirectInstructorMembershipView {
        membership,
        course: record.course,
        user: record.user,
        roster_revision,
    })
}

fn invitation_acceptance_view(
    state: &State,
    tenant: TenantId,
    invitation: CoInstructorInvitationId,
) -> Result<DirectInstructorMembershipView, StoreError> {
    let membership = state
        .co_instructor_invitation_acceptances
        .get(&(tenant, invitation))
        .copied()
        .ok_or(StoreError::Unavailable(
            "accepted invitation lacks membership receipt".to_string(),
        ))?;
    membership_view(state, tenant, membership)
}

fn fresh_invitation_id() -> Result<CoInstructorInvitationId, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!(
            "co-instructor invitation ID randomness unavailable: {error}"
        ))
    })
    .map(CoInstructorInvitationId::from_uuid)
}

fn domain_error(error: domain::teaching_authority::CoInstructorInvitationError) -> StoreError {
    StoreError::InvalidRecord(format!("invalid co-instructor authority record: {error:?}"))
}
