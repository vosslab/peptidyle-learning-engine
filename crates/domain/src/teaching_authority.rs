//! Pure WP-PROF-T2 validation for group warnings and co-instructor invitations.
//!
//! This module is deliberately separate from S5 entitlement. It validates
//! teaching-operation facts supplied by a Store transaction but never grants
//! learner entitlement, calculates effective policy, reads a clock, or writes
//! a direct membership.

use question_model::{
    ActivityTimestamp, CoInstructorInvitation, CoInstructorInvitationState,
    CourseGroupPurposePolicy, CourseId, InstructorApproval, MultipleMembershipDisposition, UserId,
};

/// Thirty calendar days expressed in the shared Unix-millisecond representation.
pub const CO_INSTRUCTOR_INVITATION_LIFETIME_MILLIS: i64 = 30 * 24 * 60 * 60 * 1_000;

/// Evaluates an informational multiple-membership result after a valid write.
pub const fn evaluate_multiple_membership(
    policy: CourseGroupPurposePolicy,
    resulting_membership_count: usize,
) -> MultipleMembershipDisposition {
    if resulting_membership_count > 1
        && matches!(
            policy.multiple_membership,
            question_model::MultipleMembershipPolicy::Warn
        )
    {
        MultipleMembershipDisposition::AllowedWithWarning
    } else {
        MultipleMembershipDisposition::Allowed
    }
}

/// Course-local direct Instructor membership used only to project teaching authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectInstructorMembership {
    pub course: CourseId,
    pub user: UserId,
    pub active: bool,
}

/// Explicit course authority result; global approval is intentionally absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructorAuthority {
    DirectCourseInstructor,
    NoDirectCourseMembership,
}

/// Projects teaching authority only from active, exact-course direct membership.
///
/// `InstructorApproval` is not an input by design: it is eligibility to be
/// invited, never a platform role or a source of course authority.
pub fn evaluate_course_instructor_authority(
    membership: Option<DirectInstructorMembership>,
    course: CourseId,
    user: UserId,
) -> InstructorAuthority {
    match membership {
        Some(membership)
            if membership.active && membership.course == course && membership.user == user =>
        {
            InstructorAuthority::DirectCourseInstructor
        }
        _ => InstructorAuthority::NoDirectCourseMembership,
    }
}

/// Errors from deterministic invitation lifecycle validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoInstructorInvitationError {
    ExpiryDoesNotMatchThirtyDays,
    TimestampOverflow,
    InvalidTerminalTimestamps,
    InvalidApprovalRecord,
    InvitationExpired,
    InvitationAlreadyAccepted,
    InvitationDeclined,
    InvitationRevoked,
    WrongTarget,
    TargetApprovalRequired,
}

/// The direct membership fact a Store must atomically create after acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoInstructorInvitationAcceptance {
    pub course: CourseId,
    pub target: UserId,
    pub accepted_at: ActivityTimestamp,
}

/// Reads the closed invitation state using only caller-supplied authoritative time.
pub fn invitation_state(
    invitation: &CoInstructorInvitation,
    now: ActivityTimestamp,
) -> Result<CoInstructorInvitationState, CoInstructorInvitationError> {
    validate_invitation_record(invitation, now)?;
    match (
        invitation.accepted_at,
        invitation.declined_at,
        invitation.revoked_at,
    ) {
        (Some(_), None, None) => return Ok(CoInstructorInvitationState::Accepted),
        (None, Some(_), None) => return Ok(CoInstructorInvitationState::Declined),
        (None, None, Some(_)) => return Ok(CoInstructorInvitationState::Revoked),
        (None, None, None) if now >= invitation.expires_at => {
            return Ok(CoInstructorInvitationState::Expired);
        }
        (None, None, None) => {}
        _ => return Err(CoInstructorInvitationError::InvalidTerminalTimestamps),
    }
    Ok(CoInstructorInvitationState::Pending)
}

/// Rechecks approval and produces the required ordinary direct-membership write.
pub fn accept_co_instructor_invitation(
    invitation: &CoInstructorInvitation,
    accepting_user: UserId,
    current_approval: Option<InstructorApproval>,
    now: ActivityTimestamp,
) -> Result<CoInstructorInvitationAcceptance, CoInstructorInvitationError> {
    match invitation_state(invitation, now)? {
        CoInstructorInvitationState::Pending => {}
        CoInstructorInvitationState::Expired => {
            return Err(CoInstructorInvitationError::InvitationExpired);
        }
        CoInstructorInvitationState::Accepted => {
            return Err(CoInstructorInvitationError::InvitationAlreadyAccepted);
        }
        CoInstructorInvitationState::Declined => {
            return Err(CoInstructorInvitationError::InvitationDeclined);
        }
        CoInstructorInvitationState::Revoked => {
            return Err(CoInstructorInvitationError::InvitationRevoked);
        }
    }
    if accepting_user != invitation.target {
        return Err(CoInstructorInvitationError::WrongTarget);
    }
    let Some(approval) = current_approval else {
        return Err(CoInstructorInvitationError::TargetApprovalRequired);
    };
    validate_instructor_approval(&approval, now)?;
    if approval.user != invitation.target || approval.revoked_at.is_some() {
        return Err(CoInstructorInvitationError::TargetApprovalRequired);
    }
    Ok(CoInstructorInvitationAcceptance {
        course: invitation.course,
        target: invitation.target,
        accepted_at: now,
    })
}

/// Error returned when a command would leave a course without an Instructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructorMembershipRemovalError {
    FinalActiveInstructor,
}

/// Refuses removal when it would leave fewer than one active Instructor.
pub const fn refuse_final_instructor_removal(
    active_instructor_count: u32,
) -> Result<(), InstructorMembershipRemovalError> {
    if active_instructor_count <= 1 {
        Err(InstructorMembershipRemovalError::FinalActiveInstructor)
    } else {
        Ok(())
    }
}

/// Validates an operator approval record against caller-supplied authoritative time.
///
/// A valid revoked record is still ineligible for invitation acceptance; this
/// function validates audit chronology only and does not project eligibility.
pub fn validate_instructor_approval(
    approval: &InstructorApproval,
    now: ActivityTimestamp,
) -> Result<(), CoInstructorInvitationError> {
    if approval.approved_at > now
        || approval
            .revoked_at
            .is_some_and(|revoked_at| revoked_at < approval.approved_at || revoked_at > now)
    {
        return Err(CoInstructorInvitationError::InvalidApprovalRecord);
    }
    Ok(())
}

fn validate_invitation_record(
    invitation: &CoInstructorInvitation,
    now: ActivityTimestamp,
) -> Result<(), CoInstructorInvitationError> {
    let expected_expiry = invitation
        .created_at
        .as_unix_millis()
        .checked_add(CO_INSTRUCTOR_INVITATION_LIFETIME_MILLIS)
        .ok_or(CoInstructorInvitationError::TimestampOverflow)?;
    if invitation.expires_at.as_unix_millis() != expected_expiry {
        return Err(CoInstructorInvitationError::ExpiryDoesNotMatchThirtyDays);
    }
    let terminal_count = [
        invitation.accepted_at,
        invitation.declined_at,
        invitation.revoked_at,
    ]
    .into_iter()
    .flatten()
    .count();
    if terminal_count > 1 {
        return Err(CoInstructorInvitationError::InvalidTerminalTimestamps);
    }
    match (
        invitation.accepted_at,
        invitation.declined_at,
        invitation.revoked_at,
    ) {
        (Some(terminal_at), None, None)
        | (None, Some(terminal_at), None)
        | (None, None, Some(terminal_at))
            if terminal_at < invitation.created_at
                || terminal_at >= invitation.expires_at
                || terminal_at > now =>
        {
            return Err(CoInstructorInvitationError::InvalidTerminalTimestamps);
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use question_model::{
        CoInstructorInvitationId, CourseGroupPurpose, CourseGroupPurposePolicy, CourseMembershipId,
        MultipleMembershipPolicy,
    };
    use uuid::Uuid;

    use super::*;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn stamp(value: i64) -> ActivityTimestamp {
        ActivityTimestamp::from_unix_millis(value)
    }

    fn invitation() -> CoInstructorInvitation {
        CoInstructorInvitation {
            id: CoInstructorInvitationId::from_uuid(id(1)),
            course: CourseId::from_uuid(id(2)),
            invited_by: CourseMembershipId::from_uuid(id(4)),
            target: UserId::from_uuid(id(3)),
            created_at: stamp(1_000),
            expires_at: stamp(1_000 + CO_INSTRUCTOR_INVITATION_LIFETIME_MILLIS),
            accepted_at: None,
            declined_at: None,
            revoked_at: None,
        }
    }

    #[test]
    fn every_group_purpose_has_its_closed_default() {
        assert_eq!(
            CourseGroupPurposePolicy::default_for_purpose(CourseGroupPurpose::Section)
                .multiple_membership,
            MultipleMembershipPolicy::Warn
        );
        for purpose in [
            CourseGroupPurpose::Lab,
            CourseGroupPurpose::Cohort,
            CourseGroupPurpose::Accommodation,
            CourseGroupPurpose::Work,
        ] {
            assert_eq!(
                CourseGroupPurposePolicy::default_for_purpose(purpose).multiple_membership,
                MultipleMembershipPolicy::Allow
            );
        }
    }

    #[test]
    fn multiple_memberships_warn_but_remain_allowed() {
        let outcome = evaluate_multiple_membership(
            CourseGroupPurposePolicy::default_for_purpose(CourseGroupPurpose::Section),
            2,
        );
        assert_eq!(outcome, MultipleMembershipDisposition::AllowedWithWarning);
        assert!(outcome.permits_write());
        assert_eq!(
            evaluate_multiple_membership(
                CourseGroupPurposePolicy::default_for_purpose(CourseGroupPurpose::Lab),
                2,
            ),
            MultipleMembershipDisposition::Allowed
        );
    }

    #[test]
    fn approval_is_not_course_authority() {
        let approval = InstructorApproval {
            user: UserId::from_uuid(id(3)),
            approved_by: UserId::from_uuid(id(9)),
            approved_at: stamp(10),
            revoked_at: None,
        };
        assert_eq!(approval.user, UserId::from_uuid(id(3)));
        assert_eq!(
            evaluate_course_instructor_authority(None, CourseId::from_uuid(id(2)), approval.user,),
            InstructorAuthority::NoDirectCourseMembership
        );
    }

    #[test]
    fn invitation_transitions_are_target_bound_and_thirty_days() {
        let pending = invitation();
        assert_eq!(
            invitation_state(&pending, stamp(1_001)),
            Ok(CoInstructorInvitationState::Pending)
        );
        assert_eq!(
            invitation_state(&pending, pending.expires_at),
            Ok(CoInstructorInvitationState::Expired)
        );

        let accepted = CoInstructorInvitation {
            accepted_at: Some(stamp(1_010)),
            ..pending.clone()
        };
        assert_eq!(
            invitation_state(&accepted, stamp(1_011)),
            Ok(CoInstructorInvitationState::Accepted)
        );
        let revoked = CoInstructorInvitation {
            revoked_at: Some(stamp(1_010)),
            ..pending
        };
        assert_eq!(
            invitation_state(&revoked, stamp(1_011)),
            Ok(CoInstructorInvitationState::Revoked)
        );
        let declined = CoInstructorInvitation {
            declined_at: Some(stamp(1_010)),
            ..pending
        };
        assert_eq!(
            invitation_state(&declined, stamp(1_011)),
            Ok(CoInstructorInvitationState::Declined)
        );
    }

    #[test]
    fn invitation_refuses_invalid_lifetime_and_terminal_records() {
        let invitation = invitation();
        let invalid_lifetime = CoInstructorInvitation {
            expires_at: stamp(invitation.expires_at.as_unix_millis() - 1),
            ..invitation.clone()
        };
        assert_eq!(
            invitation_state(&invalid_lifetime, stamp(1_001)),
            Err(CoInstructorInvitationError::ExpiryDoesNotMatchThirtyDays)
        );
        let conflicting_terminal = CoInstructorInvitation {
            accepted_at: Some(stamp(1_010)),
            declined_at: None,
            revoked_at: Some(stamp(1_011)),
            ..invitation
        };
        assert_eq!(
            invitation_state(&conflicting_terminal, stamp(1_012)),
            Err(CoInstructorInvitationError::InvalidTerminalTimestamps)
        );
    }

    #[test]
    fn invitation_terminal_timestamp_must_not_be_later_than_now() {
        let now = stamp(1_010);
        for (terminal, expected_state) in [
            (
                CoInstructorInvitation {
                    accepted_at: Some(now),
                    ..invitation()
                },
                CoInstructorInvitationState::Accepted,
            ),
            (
                CoInstructorInvitation {
                    declined_at: Some(now),
                    ..invitation()
                },
                CoInstructorInvitationState::Declined,
            ),
            (
                CoInstructorInvitation {
                    revoked_at: Some(now),
                    ..invitation()
                },
                CoInstructorInvitationState::Revoked,
            ),
        ] {
            assert_eq!(invitation_state(&terminal, now), Ok(expected_state));
        }
        for terminal in [
            CoInstructorInvitation {
                accepted_at: Some(stamp(1_011)),
                ..invitation()
            },
            CoInstructorInvitation {
                declined_at: Some(stamp(1_011)),
                ..invitation()
            },
            CoInstructorInvitation {
                revoked_at: Some(stamp(1_011)),
                ..invitation()
            },
        ] {
            assert_eq!(
                invitation_state(&terminal, now),
                Err(CoInstructorInvitationError::InvalidTerminalTimestamps)
            );
        }
    }

    #[test]
    fn approval_record_chronology_is_checked_at_caller_time() {
        let now = stamp(1_001);
        let active = approval(UserId::from_uuid(id(3)));
        assert_eq!(validate_instructor_approval(&active, now), Ok(()));

        let revoked = InstructorApproval {
            revoked_at: Some(stamp(950)),
            ..active
        };
        assert_eq!(validate_instructor_approval(&revoked, now), Ok(()));

        for invalid in [
            InstructorApproval {
                revoked_at: Some(stamp(899)),
                ..active
            },
            InstructorApproval {
                approved_at: stamp(1_002),
                ..active
            },
            InstructorApproval {
                revoked_at: Some(stamp(1_002)),
                ..active
            },
        ] {
            assert_eq!(
                validate_instructor_approval(&invalid, now),
                Err(CoInstructorInvitationError::InvalidApprovalRecord)
            );
        }
    }

    #[test]
    fn acceptance_rechecks_current_approval_before_direct_membership_write() {
        let invitation = invitation();
        let now = stamp(1_001);
        assert_eq!(
            accept_co_instructor_invitation(&invitation, invitation.target, None, now),
            Err(CoInstructorInvitationError::TargetApprovalRequired)
        );
        assert_eq!(
            accept_co_instructor_invitation(
                &invitation,
                UserId::from_uuid(id(4)),
                Some(approval(invitation.target)),
                now,
            ),
            Err(CoInstructorInvitationError::WrongTarget)
        );
        assert_eq!(
            accept_co_instructor_invitation(
                &invitation,
                invitation.target,
                Some(approval(invitation.target)),
                now,
            ),
            Ok(CoInstructorInvitationAcceptance {
                course: invitation.course,
                target: invitation.target,
                accepted_at: now,
            })
        );
        let revoked_approval = InstructorApproval {
            revoked_at: Some(stamp(950)),
            ..approval(invitation.target)
        };
        assert_eq!(
            accept_co_instructor_invitation(
                &invitation,
                invitation.target,
                Some(revoked_approval),
                now,
            ),
            Err(CoInstructorInvitationError::TargetApprovalRequired)
        );
        let future_approval = InstructorApproval {
            approved_at: stamp(1_002),
            ..approval(invitation.target)
        };
        assert_eq!(
            accept_co_instructor_invitation(
                &invitation,
                invitation.target,
                Some(future_approval),
                now,
            ),
            Err(CoInstructorInvitationError::InvalidApprovalRecord)
        );
        let future_revocation = InstructorApproval {
            revoked_at: Some(stamp(1_002)),
            ..approval(invitation.target)
        };
        assert_eq!(
            accept_co_instructor_invitation(
                &invitation,
                invitation.target,
                Some(future_revocation),
                now,
            ),
            Err(CoInstructorInvitationError::InvalidApprovalRecord)
        );
    }

    #[test]
    fn final_instructor_removal_is_refused() {
        assert_eq!(
            refuse_final_instructor_removal(0),
            Err(InstructorMembershipRemovalError::FinalActiveInstructor)
        );
        assert_eq!(
            refuse_final_instructor_removal(1),
            Err(InstructorMembershipRemovalError::FinalActiveInstructor)
        );
        assert_eq!(refuse_final_instructor_removal(2), Ok(()));
    }

    fn approval(user: UserId) -> InstructorApproval {
        InstructorApproval {
            user,
            approved_by: UserId::from_uuid(id(9)),
            approved_at: stamp(900),
            revoked_at: None,
        }
    }
}
