//! In-memory atomic course roster reference implementation.

use std::collections::BTreeSet;

use async_trait::async_trait;
use question_model::{
    AssignmentEnrollment, CourseMembership, CourseMembershipRole, EnrollmentId,
    StudentAssignmentSummary, StudentId, UserRole,
};

use super::invitation_delivery::{cancel_for_invitation, create_pending};
use super::sessions::active_subject;
use super::{MemoryStore, State, page_records, require_course_records_accessible};
use crate::{
    ClaimCourseInvitation, ClaimedCourseMembership, CommitCourseRosterImport,
    CommittedCourseRosterImport, CourseEnrollmentPolicy, CourseInvitation, CourseInvitationId,
    CourseInvitationStatus, CourseMemberId, CourseMemberStatus, CourseRosterEntry,
    CourseRosterImportPreview, CourseRosterPage, CourseRosterStore, CourseRosterSupportAction,
    CourseRosterSupportAudit, CourseSignupPosture, CreateCourseInvitation,
    ReplaceCourseEnrollmentPolicy, RevokeCourseInvitation, RevokeCourseMember, RosterRevision,
    SessionTokenHash, StageCourseRosterImport, StoreError, TenantContext, UpsertCourseMember,
};

#[path = "course_roster/import.rs"]
pub(super) mod import;
pub(super) use import::delivery_provenance;

#[derive(Debug, Clone)]
pub(super) struct StoredCourseInvitation {
    pub(super) record: CourseInvitation,
}

#[async_trait]
impl CourseRosterStore for MemoryStore {
    async fn list_course_roster(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: question_model::CourseId,
        page: crate::PageRequest,
    ) -> Result<CourseRosterPage, StoreError> {
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, context.tenant_id(), course)?;
        require_roster_authority(
            &mut state,
            context,
            session,
            course,
            CourseRosterSupportAction::ListRoster,
        )?;
        let mut records = state
            .roster_members
            .iter()
            .filter(|((tenant, record_course, _), _)| {
                *tenant == context.tenant_id() && *record_course == course
            })
            .map(|((_, _, member), record)| {
                (
                    format!("member:{}", member.as_uuid()),
                    CourseRosterEntry::Member(record.clone()),
                )
            })
            .chain(
                state
                    .course_invitations
                    .iter()
                    .filter(|((tenant, record_course, _), _)| {
                        *tenant == context.tenant_id() && *record_course == course
                    })
                    .filter(|(_, stored)| {
                        public_invitation(&state, stored).status == CourseInvitationStatus::Pending
                    })
                    .map(|((_, _, invitation), stored)| {
                        (
                            format!("invitation:{}", invitation.as_uuid()),
                            CourseRosterEntry::Invitation(public_invitation(&state, stored)),
                        )
                    }),
            )
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(CourseRosterPage {
            entries: page_records(records, &page),
            policy: roster_policy(&state, context.tenant_id(), course),
        })
    }

    async fn create_course_invitation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CreateCourseInvitation,
    ) -> Result<CourseInvitation, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, tenant, command.course)?;
        let actor = require_roster_authority(
            &mut state,
            context,
            session,
            command.course,
            CourseRosterSupportAction::CreateInvitation,
        )?;
        let policy = roster_policy(&state, tenant, command.course);
        if !policy.validates(&command.email) {
            return Err(StoreError::InvalidRecord(
                "invitation email domain is not permitted".to_string(),
            ));
        }
        let receipt_key = (tenant, command.course, command.idempotency_key.clone());
        if let Some((invitation, token_hash)) = state.invitation_idempotency.get(&receipt_key) {
            let stored = state
                .course_invitations
                .get(&(tenant, command.course, *invitation))
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "invitation idempotency receipt is inconsistent".to_string(),
                    )
                })?;
            if *token_hash == command.token_hash
                && stored.record.email == command.email
                && stored.record.roster_id == command.roster_id
            {
                let invitation = public_invitation(&state, stored);
                return (invitation.status == CourseInvitationStatus::Pending)
                    .then_some(invitation)
                    .ok_or(StoreError::Conflict);
            }
            return Err(StoreError::Conflict);
        }
        if state.invitation_by_hash.contains_key(&command.token_hash)
            || roster_id_in_use(&state, tenant, command.course, &command.roster_id)
            || invitation_email_in_use(&state, tenant, command.course, &command.email)
        {
            return Err(StoreError::AlreadyExists);
        }
        let invitation = CourseInvitation {
            id: CourseInvitationId::generate()?,
            tenant,
            course: command.course,
            email: command.email,
            roster_id: command.roster_id,
            invited_by: actor,
            status: CourseInvitationStatus::Pending,
            created_at: state.authoritative_time,
            expires_at: timestamp_after_seconds(
                state.authoritative_time,
                command.lifetime.as_seconds(),
            )?,
            claimed_by: None,
        };
        let stored = StoredCourseInvitation {
            record: invitation.clone(),
        };
        state
            .invitation_by_hash
            .insert(command.token_hash, (tenant, command.course, invitation.id));
        state
            .invitation_idempotency
            .insert(receipt_key, (invitation.id, command.token_hash));
        state
            .course_invitations
            .insert((tenant, command.course, invitation.id), stored);
        create_pending(&mut state, tenant, command.course, invitation.id)?;
        bump_roster_revision(&mut state, tenant, command.course, None)?;
        Ok(invitation)
    }

    async fn replace_course_enrollment_policy(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceCourseEnrollmentPolicy,
    ) -> Result<CourseEnrollmentPolicy, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, tenant, command.course)?;
        require_roster_authority(
            &mut state,
            context,
            session,
            command.course,
            CourseRosterSupportAction::ReplaceEnrollmentPolicy,
        )?;
        let current = roster_policy(&state, tenant, command.course);
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let candidate = CourseEnrollmentPolicy {
            course: command.course,
            allowed_domains: command.allowed_domains,
            signup_posture: command.signup_posture,
            revision: current.revision,
        };
        candidate
            .validate_shape()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        if current.allowed_domains == candidate.allowed_domains
            && current.signup_posture == candidate.signup_posture
        {
            return Ok(current);
        }
        let updated = CourseEnrollmentPolicy {
            revision: current.revision.next()?,
            ..candidate
        };
        state
            .roster_policies
            .insert((tenant, command.course), updated.clone());
        Ok(updated)
    }

    async fn claim_course_invitation(
        &self,
        command: ClaimCourseInvitation,
    ) -> Result<ClaimedCourseMembership, StoreError> {
        let mut state = self.write_state()?;
        let (tenant, course, invitation_id) = state
            .invitation_by_hash
            .get(&command.token_hash)
            .copied()
            .ok_or(StoreError::NotFound)?;
        require_course_records_accessible(&state, tenant, course)?;
        let stored = state
            .course_invitations
            .get(&(tenant, course, invitation_id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if stored.record.email.normalized() != command.verified_email.normalized() {
            return Err(StoreError::Forbidden);
        }
        if stored.record.expires_at <= state.authoritative_time {
            if let Some(expired) =
                state
                    .course_invitations
                    .get_mut(&(tenant, course, invitation_id))
            {
                expired.record.status = CourseInvitationStatus::Expired;
            }
            cancel_for_invitation(&mut state, tenant, course, invitation_id);
            return Err(StoreError::NotFound);
        }
        if stored.record.status == CourseInvitationStatus::Claimed {
            if stored.record.claimed_by != Some(command.user) {
                return Err(StoreError::Conflict);
            }
            return claimed_existing_member(&state, tenant, course, command.user);
        }
        if stored.record.status != CourseInvitationStatus::Pending {
            return Err(StoreError::NotFound);
        }

        let snapshot = state.clone();
        let result = claim_locked(&mut state, tenant, course, invitation_id, stored, command);
        if result.is_err() {
            *state = snapshot;
        }
        result
    }

    async fn upsert_course_member(
        &self,
        context: TenantContext,
        command: UpsertCourseMember,
    ) -> Result<ClaimedCourseMembership, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, tenant, command.course)?;
        let rollback = state.clone();
        let result = upsert_member_locked(&mut state, tenant, command);
        if result.is_err() {
            *state = rollback;
        }
        result
    }

    async fn revoke_course_member(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: RevokeCourseMember,
    ) -> Result<RosterRevision, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, tenant, command.course)?;
        require_roster_authority(
            &mut state,
            context,
            session,
            command.course,
            CourseRosterSupportAction::RevokeMember,
        )?;
        let current = roster_policy(&state, tenant, command.course);
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let key = (tenant, command.course, command.member);
        let now = state.authoritative_time;
        let user = {
            let member = state
                .roster_members
                .get_mut(&key)
                .ok_or(StoreError::NotFound)?;
            if member.status == CourseMemberStatus::Revoked {
                return Ok(current.revision);
            }
            member.status = CourseMemberStatus::Revoked;
            member.revoked_at = Some(now);
            member.user
        };
        let course_record = state
            .courses
            .get_mut(&(tenant, command.course))
            .ok_or(StoreError::NotFound)?;
        course_record
            .members
            .retain(|membership| membership.user != user);
        for record in state
            .course_groups
            .values_mut()
            .filter(|record| record.tenant == tenant && record.course == command.course)
        {
            record.members.retain(|member| *member != user);
        }
        bump_roster_revision(
            &mut state,
            tenant,
            command.course,
            Some(command.expected_revision),
        )
    }

    async fn revoke_course_invitation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: RevokeCourseInvitation,
    ) -> Result<RosterRevision, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, tenant, command.course)?;
        require_roster_authority(
            &mut state,
            context,
            session,
            command.course,
            CourseRosterSupportAction::RevokeInvitation,
        )?;
        let current = roster_policy(&state, tenant, command.course);
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let invitation = state
            .course_invitations
            .get_mut(&(tenant, command.course, command.invitation))
            .ok_or(StoreError::NotFound)?;
        match invitation.record.status {
            CourseInvitationStatus::Pending => {
                invitation.record.status = CourseInvitationStatus::Revoked;
            }
            CourseInvitationStatus::Revoked => return Ok(current.revision),
            CourseInvitationStatus::Claimed | CourseInvitationStatus::Expired => {
                return Err(StoreError::Conflict);
            }
        }
        cancel_for_invitation(&mut state, tenant, command.course, command.invitation);
        bump_roster_revision(
            &mut state,
            tenant,
            command.course,
            Some(command.expected_revision),
        )
    }

    async fn stage_course_roster_import(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: StageCourseRosterImport,
    ) -> Result<CourseRosterImportPreview, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, tenant, command.course)?;
        let actor = require_roster_authority(
            &mut state,
            context,
            session,
            command.course,
            CourseRosterSupportAction::StageImport,
        )?;
        import::stage(&mut state, tenant, actor, command)
    }

    async fn commit_course_roster_import(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CommitCourseRosterImport,
    ) -> Result<CommittedCourseRosterImport, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, tenant, command.course)?;
        let actor = require_roster_authority(
            &mut state,
            context,
            session,
            command.course,
            CourseRosterSupportAction::CommitImport,
        )?;
        let rollback = state.clone();
        let result = import::commit(&mut state, tenant, actor, command);
        if result.is_err() {
            *state = rollback;
        }
        result
    }
}

fn claim_locked(
    state: &mut State,
    tenant: question_model::TenantId,
    course: question_model::CourseId,
    invitation_id: CourseInvitationId,
    stored: StoredCourseInvitation,
    command: ClaimCourseInvitation,
) -> Result<ClaimedCourseMembership, StoreError> {
    if let Some(existing_member) = state
        .roster_member_by_roster_id
        .get(&(tenant, course, stored.record.roster_id.clone()))
        .copied()
        && state
            .roster_members
            .get(&(tenant, course, existing_member))
            .is_some_and(|member| member.user != command.user)
    {
        return Err(StoreError::Conflict);
    }
    let student = learner_identity(state, tenant, command.user)?;
    let member_id = match state
        .roster_member_by_user
        .get(&(tenant, course, command.user))
        .copied()
    {
        Some(existing) => existing,
        None => CourseMemberId::generate()?,
    };
    let member_key = (tenant, course, member_id);
    let joined_at = state
        .roster_members
        .get(&member_key)
        .map_or(state.authoritative_time, |member| member.joined_at);
    let member = crate::CourseRosterMember {
        id: member_id,
        tenant,
        course,
        user: command.user,
        student,
        display_name: crate::validated_account_display_name(&command.display_name)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
        roster_email: Some(stored.record.email.clone()),
        roster_id: Some(stored.record.roster_id.clone()),
        status: CourseMemberStatus::Active,
        joined_at,
        revoked_at: None,
    };
    state
        .roster_member_by_user
        .insert((tenant, course, command.user), member_id);
    state.roster_member_by_roster_id.insert(
        (
            tenant,
            course,
            member
                .roster_id
                .clone()
                .expect("a claimed invitation always carries a roster identifier"),
        ),
        member_id,
    );
    state.roster_members.insert(member_key, member.clone());

    let course_record = state
        .courses
        .get_mut(&(tenant, course))
        .ok_or(StoreError::NotFound)?;
    if course_record.role_for(command.user).is_none() {
        course_record.members.push(CourseMembership {
            user: command.user,
            role: CourseMembershipRole::Student,
        });
    } else if course_record.role_for(command.user) != Some(CourseMembershipRole::Student) {
        return Err(StoreError::Conflict);
    }
    reconcile_member_assignments(state, tenant, course, command.user, student)?;
    let invitation = state
        .course_invitations
        .get_mut(&(tenant, course, invitation_id))
        .ok_or(StoreError::NotFound)?;
    invitation.record.status = CourseInvitationStatus::Claimed;
    invitation.record.claimed_by = Some(command.user);
    cancel_for_invitation(state, tenant, course, invitation_id);
    let roster_revision = bump_roster_revision(state, tenant, course, None)?;
    Ok(ClaimedCourseMembership {
        tenant,
        course,
        member,
        roster_revision,
    })
}

fn upsert_member_locked(
    state: &mut State,
    tenant: question_model::TenantId,
    command: UpsertCourseMember,
) -> Result<ClaimedCourseMembership, StoreError> {
    let student = learner_identity(state, tenant, command.user)?;
    if let Some(existing_member_id) = state
        .roster_member_by_user
        .get(&(tenant, command.course, command.user))
        .copied()
    {
        let existing = state
            .roster_members
            .get(&(tenant, command.course, existing_member_id))
            .cloned()
            .ok_or(StoreError::Unavailable(
                "local roster member index is inconsistent".to_string(),
            ))?;
        if existing.status == CourseMemberStatus::Active {
            ensure_student_membership(state, tenant, command.course, command.user)?;
            reconcile_member_assignments(state, tenant, command.course, command.user, student)?;
            return Ok(ClaimedCourseMembership {
                tenant,
                course: command.course,
                member: existing,
                roster_revision: roster_policy(state, tenant, command.course).revision,
            });
        }
    }
    let member_id = match state
        .roster_member_by_user
        .get(&(tenant, command.course, command.user))
        .copied()
    {
        Some(existing) => existing,
        None => CourseMemberId::generate()?,
    };
    let member_key = (tenant, command.course, member_id);
    let joined_at = state
        .roster_members
        .get(&member_key)
        .map_or(state.authoritative_time, |member| member.joined_at);
    let member = crate::CourseRosterMember {
        id: member_id,
        tenant,
        course: command.course,
        user: command.user,
        student,
        display_name: crate::validated_account_display_name(&command.display_name)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
        roster_email: command
            .roster_contact
            .as_ref()
            .map(|contact| contact.email.clone()),
        roster_id: command
            .roster_contact
            .as_ref()
            .map(|contact| contact.roster_id.clone()),
        status: CourseMemberStatus::Active,
        joined_at,
        revoked_at: None,
    };
    state
        .roster_member_by_user
        .insert((tenant, command.course, command.user), member_id);
    if let Some(roster_id) = member.roster_id.clone()
        && let Some(existing) = state
            .roster_member_by_roster_id
            .insert((tenant, command.course, roster_id), member_id)
        && existing != member_id
    {
        return Err(StoreError::Conflict);
    }
    state.roster_members.insert(member_key, member.clone());
    ensure_student_membership(state, tenant, command.course, command.user)?;
    reconcile_member_assignments(state, tenant, command.course, command.user, student)?;
    let roster_revision = bump_roster_revision(state, tenant, command.course, None)?;
    Ok(ClaimedCourseMembership {
        tenant,
        course: command.course,
        member,
        roster_revision,
    })
}

fn ensure_student_membership(
    state: &mut State,
    tenant: question_model::TenantId,
    course: question_model::CourseId,
    user: question_model::UserId,
) -> Result<(), StoreError> {
    let course_record = state
        .courses
        .get_mut(&(tenant, course))
        .ok_or(StoreError::NotFound)?;
    if course_record.role_for(user).is_none() {
        course_record.members.push(CourseMembership {
            user,
            role: CourseMembershipRole::Student,
        });
    } else if course_record.role_for(user) != Some(CourseMembershipRole::Student) {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn learner_identity(
    state: &mut State,
    tenant: question_model::TenantId,
    user: question_model::UserId,
) -> Result<StudentId, StoreError> {
    if let Some(student) = state.learner_by_user.get(&(tenant, user)).copied() {
        return Ok(student);
    }
    let student = random_student_id()?;
    if !state.learner_by_student.insert((tenant, student, user)) {
        return Err(StoreError::Conflict);
    }
    state.learner_by_user.insert((tenant, user), student);
    Ok(student)
}

fn reconcile_member_assignments(
    state: &mut State,
    tenant: question_model::TenantId,
    course: question_model::CourseId,
    user: question_model::UserId,
    student: StudentId,
) -> Result<(), StoreError> {
    let assignments = state
        .assignments
        .values()
        .filter(|assignment| assignment.tenant == tenant && assignment.course_id == course)
        .map(|assignment| assignment.id)
        .collect::<BTreeSet<_>>();
    for assignment in assignments {
        if state.enrollments.values().any(|enrollment| {
            enrollment.tenant == tenant
                && enrollment.assignment == assignment
                && enrollment.user == user
        }) {
            continue;
        }
        if state.enrollments.values().any(|enrollment| {
            enrollment.tenant == tenant
                && enrollment.assignment == assignment
                && enrollment.student == student
        }) {
            return Err(StoreError::Conflict);
        }
        let enrollment = AssignmentEnrollment {
            id: random_enrollment_id()?,
            tenant,
            assignment,
            user,
            student,
            first_completed_at: None,
            current_grade_run: None,
            best_grade_run: None,
        };
        state.summaries.insert(
            (tenant, enrollment.id),
            StudentAssignmentSummary::empty(tenant, enrollment.id),
        );
        state
            .enrollments
            .insert((tenant, enrollment.id), enrollment);
    }
    Ok(())
}

pub(super) fn reconcile_new_assignment(
    state: &mut State,
    assignment: &crate::AssignmentRecord,
) -> Result<(), StoreError> {
    let members = state
        .roster_members
        .values()
        .filter(|member| {
            member.tenant == assignment.tenant
                && member.course == assignment.course_id
                && member.status == CourseMemberStatus::Active
        })
        .map(|member| (member.user, member.student))
        .collect::<Vec<_>>();
    for (user, student) in members {
        reconcile_member_assignments(
            state,
            assignment.tenant,
            assignment.course_id,
            user,
            student,
        )?;
    }
    Ok(())
}

pub(super) fn require_course_instructor(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
    course: question_model::CourseId,
) -> Result<question_model::UserId, StoreError> {
    let subject = active_subject(state, context, session).ok_or(StoreError::NotFound)?;
    let record = state
        .courses
        .get(&(context.tenant_id(), course))
        .ok_or(StoreError::NotFound)?;
    match record.role_for(subject.user()) {
        Some(CourseMembershipRole::Instructor) => Ok(subject.user()),
        Some(CourseMembershipRole::Student) => Err(StoreError::Forbidden),
        None => Err(StoreError::NotFound),
    }
}

pub(super) fn require_roster_authority(
    state: &mut State,
    context: TenantContext,
    session: SessionTokenHash,
    course: question_model::CourseId,
    action: CourseRosterSupportAction,
) -> Result<question_model::UserId, StoreError> {
    let subject = active_subject(state, context, session).ok_or(StoreError::NotFound)?;
    let actor = subject.user();
    let is_sysadmin = subject.roles().contains(&UserRole::Sysadmin);
    let record = state
        .courses
        .get(&(context.tenant_id(), course))
        .ok_or(StoreError::NotFound)?;
    if record.role_for(actor) == Some(CourseMembershipRole::Instructor) {
        return Ok(actor);
    }
    if !is_sysadmin {
        return match record.role_for(actor) {
            Some(CourseMembershipRole::Student) => Err(StoreError::Forbidden),
            None => Err(StoreError::NotFound),
            Some(CourseMembershipRole::Instructor) => {
                unreachable!("direct Instructor returned above")
            }
        };
    }
    state.roster_support_audits.push(CourseRosterSupportAudit {
        tenant: context.tenant_id(),
        course,
        actor,
        action,
        occurred_at: state.authoritative_time,
    });
    Ok(actor)
}

/// Authorizes an already-owned coarse projection without creating a second
/// Sysadmin support audit for the preceding mutation.
pub(super) fn require_roster_read_authority(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
    course: question_model::CourseId,
) -> Result<(), StoreError> {
    let subject = active_subject(state, context, session).ok_or(StoreError::NotFound)?;
    let record = state
        .courses
        .get(&(context.tenant_id(), course))
        .ok_or(StoreError::NotFound)?;
    if record.role_for(subject.user()) == Some(CourseMembershipRole::Instructor)
        || subject.roles().contains(&UserRole::Sysadmin)
    {
        Ok(())
    } else if record.role_for(subject.user()).is_some() {
        Err(StoreError::Forbidden)
    } else {
        Err(StoreError::NotFound)
    }
}

fn roster_policy(
    state: &State,
    tenant: question_model::TenantId,
    course: question_model::CourseId,
) -> CourseEnrollmentPolicy {
    state
        .roster_policies
        .get(&(tenant, course))
        .cloned()
        .unwrap_or(CourseEnrollmentPolicy {
            course,
            allowed_domains: BTreeSet::new(),
            signup_posture: CourseSignupPosture::InvitationOnly,
            revision: RosterRevision::INITIAL,
        })
}

fn bump_roster_revision(
    state: &mut State,
    tenant: question_model::TenantId,
    course: question_model::CourseId,
    expected: Option<RosterRevision>,
) -> Result<RosterRevision, StoreError> {
    let mut policy = roster_policy(state, tenant, course);
    if expected.is_some_and(|expected| expected != policy.revision) {
        return Err(StoreError::Conflict);
    }
    policy.revision = policy.revision.next()?;
    let revision = policy.revision;
    state.roster_policies.insert((tenant, course), policy);
    Ok(revision)
}

fn roster_id_in_use(
    state: &State,
    tenant: question_model::TenantId,
    course: question_model::CourseId,
    roster_id: &crate::CourseRosterId,
) -> bool {
    state
        .roster_member_by_roster_id
        .contains_key(&(tenant, course, roster_id.clone()))
        || state.course_invitations.values().any(|invitation| {
            invitation.record.tenant == tenant
                && invitation.record.course == course
                && invitation.record.roster_id == *roster_id
                && invitation.record.status == CourseInvitationStatus::Pending
                && invitation.record.expires_at > state.authoritative_time
        })
}

fn invitation_email_in_use(
    state: &State,
    tenant: question_model::TenantId,
    course: question_model::CourseId,
    email: &crate::AuthenticationEmail,
) -> bool {
    state.course_invitations.values().any(|invitation| {
        invitation.record.tenant == tenant
            && invitation.record.course == course
            && invitation.record.email.normalized() == email.normalized()
            && invitation.record.status == CourseInvitationStatus::Pending
            && invitation.record.expires_at > state.authoritative_time
    })
}

fn public_invitation(state: &State, stored: &StoredCourseInvitation) -> CourseInvitation {
    let mut invitation = stored.record.clone();
    if invitation.status == CourseInvitationStatus::Pending
        && invitation.expires_at <= state.authoritative_time
    {
        invitation.status = CourseInvitationStatus::Expired;
    }
    invitation
}

fn claimed_existing_member(
    state: &State,
    tenant: question_model::TenantId,
    course: question_model::CourseId,
    user: question_model::UserId,
) -> Result<ClaimedCourseMembership, StoreError> {
    let member_id = state
        .roster_member_by_user
        .get(&(tenant, course, user))
        .copied()
        .ok_or(StoreError::NotFound)?;
    let member = state
        .roster_members
        .get(&(tenant, course, member_id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    Ok(ClaimedCourseMembership {
        tenant,
        course,
        member,
        roster_revision: roster_policy(state, tenant, course).revision,
    })
}

fn timestamp_after_seconds(
    timestamp: question_model::ActivityTimestamp,
    seconds: u32,
) -> Result<question_model::ActivityTimestamp, StoreError> {
    let millis = i64::from(seconds)
        .checked_mul(1_000)
        .and_then(|value| timestamp.as_unix_millis().checked_add(value))
        .ok_or_else(|| StoreError::InvalidRecord("invitation expiry overflow".to_string()))?;
    Ok(question_model::ActivityTimestamp::from_unix_millis(millis))
}

fn random_student_id() -> Result<StudentId, StoreError> {
    random_uuid("student ID").map(StudentId::from_uuid)
}

fn random_enrollment_id() -> Result<EnrollmentId, StoreError> {
    random_uuid("enrollment ID").map(EnrollmentId::from_uuid)
}

fn random_uuid(label: &str) -> Result<uuid::Uuid, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!("{label} randomness unavailable: {error}"))
    })?;
    Ok(uuid::Uuid::from_bytes(bytes))
}
