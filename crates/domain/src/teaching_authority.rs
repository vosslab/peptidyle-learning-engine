//! Pure WP-INST-T2 validation for group warnings and co-instructor invitations.
//!
//! This module is deliberately separate from S5 entitlement. It validates
//! teaching-operation facts supplied by a Store transaction but never grants
//! learner entitlement, calculates effective policy, reads a clock, or writes
//! a direct membership.

use question_model::{
    AccountId, ActivityTimestamp, CoInstructorInvitation, CoInstructorInvitationState,
    CourseGroupPurposePolicy, CourseId, CourseMembershipId, CourseMembershipRole,
    InstructorApproval, MultipleMembershipDisposition, StudentId,
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

/// Current direct Instructor-membership facts for one exact course.
///
/// The Store supplies this projection after locking the durable membership
/// record. It deliberately contains no creator distinction: the course creator
/// and every accepted co-Instructor use the same exact membership relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectInstructorMembership {
    pub course: CourseId,
    pub instructor_account: AccountId,
    pub active: bool,
}

/// Explicit result of a current course-Instructor authorization evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructorAuthority {
    CurrentCourseInstructor,
    ApprovalRequired,
    NoDirectCourseMembership,
}

/// Returns whether an account currently has global Instructor capability.
///
/// A supplied timestamp keeps the predicate deterministic and lets the
/// transaction owner provide its authoritative clock. A malformed, future, or
/// revoked approval fails closed.
pub fn approved_instructor(
    approval: Option<InstructorApproval>,
    instructor_account: AccountId,
    now: ActivityTimestamp,
) -> bool {
    match approval {
        Some(approval) => {
            approval.account == instructor_account
                && approval.approved_at <= now
                && approval.revoked_at.is_none()
        }
        None => false,
    }
}

/// Returns whether an account is a current Instructor for exactly `course`.
///
/// This is the canonical pure predicate for all course-Instructor operations:
/// current global approval and an active direct Instructor membership are both
/// required. Neither a creator flag nor a co-Instructor distinction exists.
pub fn current_course_instructor(
    approval: Option<InstructorApproval>,
    membership: Option<DirectInstructorMembership>,
    instructor_account: AccountId,
    course: CourseId,
    now: ActivityTimestamp,
) -> bool {
    approved_instructor(approval, instructor_account, now)
        && matches!(
            membership,
            Some(membership)
                if membership.active
                    && membership.course == course
                    && membership.instructor_account == instructor_account
        )
}

/// Current Student-membership facts that bind a global account to one exact
/// Student educational-record identity in one course.
///
/// The membership episode remains distinct from the global account and from
/// the Student identity, so revocation and re-enrollment cannot rewrite past
/// evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudentCourseMembership {
    pub membership: CourseMembershipId,
    pub course: CourseId,
    pub student_account: AccountId,
    pub student: StudentId,
    pub role: CourseMembershipRole,
    pub active: bool,
}

/// Returns whether one account currently owns the exact Student record in an
/// exact course.
///
/// Callers must supply the membership projection from the same protected
/// transaction as the educational-record access. A foreign course, account,
/// Student identity, membership episode, revoked episode, or non-Student role
/// fails closed.
pub fn student_owns_course_record(
    membership: Option<StudentCourseMembership>,
    student_account: AccountId,
    course: CourseId,
    record_membership: CourseMembershipId,
    student: StudentId,
) -> bool {
    matches!(
        membership,
        Some(membership)
            if membership.active
                && membership.role == CourseMembershipRole::Student
                && membership.course == course
                && membership.student_account == student_account
                && membership.membership == record_membership
                && membership.student == student
    )
}

/// Classifies the exact course-Instructor predicate for callers that need a
/// denial reason without turning a role label into authority.
pub fn evaluate_course_instructor_authority(
    approval: Option<InstructorApproval>,
    membership: Option<DirectInstructorMembership>,
    course: CourseId,
    instructor_account: AccountId,
    now: ActivityTimestamp,
) -> InstructorAuthority {
    if !approved_instructor(approval, instructor_account, now) {
        InstructorAuthority::ApprovalRequired
    } else if current_course_instructor(approval, membership, instructor_account, course, now) {
        InstructorAuthority::CurrentCourseInstructor
    } else {
        InstructorAuthority::NoDirectCourseMembership
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
    pub target: AccountId,
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
    accepting_account: AccountId,
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
    if accepting_account != invitation.target {
        return Err(CoInstructorInvitationError::WrongTarget);
    }
    let Some(approval) = current_approval else {
        return Err(CoInstructorInvitationError::TargetApprovalRequired);
    };
    validate_instructor_approval(&approval, now)?;
    if approval.account != invitation.target || approval.revoked_at.is_some() {
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
            target: AccountId::from_uuid(id(3)),
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
    fn current_course_instructor_requires_active_approval_and_membership() {
        let instructor_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        let approval = InstructorApproval {
            account: instructor_account,
            approved_by: AccountId::from_uuid(id(9)),
            approved_at: stamp(10),
            revoked_at: None,
        };
        let membership = DirectInstructorMembership {
            course,
            instructor_account,
            active: true,
        };
        assert_eq!(
            evaluate_course_instructor_authority(
                Some(approval),
                Some(membership),
                course,
                instructor_account,
                stamp(11),
            ),
            InstructorAuthority::CurrentCourseInstructor
        );
    }

    #[test]
    fn approval_withdrawal_revokes_course_instructor_authority() {
        let instructor_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        let approval = approval(instructor_account);
        assert!(!current_course_instructor(
            Some(InstructorApproval {
                revoked_at: Some(stamp(1_001)),
                ..approval
            }),
            Some(DirectInstructorMembership {
                course,
                instructor_account,
                active: true,
            }),
            instructor_account,
            course,
            stamp(1_001),
        ));
    }

    #[test]
    fn foreign_course_membership_cannot_authorize_an_instructor() {
        let instructor_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        assert!(!current_course_instructor(
            Some(approval(instructor_account)),
            Some(DirectInstructorMembership {
                course: CourseId::from_uuid(id(4)),
                instructor_account,
                active: true,
            }),
            instructor_account,
            course,
            stamp(1_001),
        ));
    }

    #[test]
    fn foreign_account_membership_cannot_authorize_an_instructor() {
        let instructor_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        assert!(!current_course_instructor(
            Some(approval(instructor_account)),
            Some(DirectInstructorMembership {
                course,
                instructor_account: AccountId::from_uuid(id(4)),
                active: true,
            }),
            instructor_account,
            course,
            stamp(1_001),
        ));
    }

    #[test]
    fn revoked_membership_cannot_authorize_an_instructor() {
        let instructor_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        assert!(!current_course_instructor(
            Some(approval(instructor_account)),
            Some(DirectInstructorMembership {
                course,
                instructor_account,
                active: false,
            }),
            instructor_account,
            course,
            stamp(1_001),
        ));
    }

    #[test]
    fn exact_student_record_is_permitted_and_other_student_is_denied() {
        let student_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        let membership = student_membership(student_account, course);
        assert!(student_owns_course_record(
            Some(membership),
            student_account,
            course,
            membership.membership,
            membership.student,
        ));
        assert!(!student_owns_course_record(
            Some(membership),
            student_account,
            course,
            membership.membership,
            StudentId::from_uuid(id(7)),
        ));
    }

    #[test]
    fn mismatched_student_membership_episode_cannot_authorize_course_record_access() {
        let student_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        let membership = student_membership(student_account, course);
        assert!(!student_owns_course_record(
            Some(membership),
            student_account,
            course,
            CourseMembershipId::from_uuid(id(7)),
            membership.student,
        ));
    }

    #[test]
    fn inactive_student_membership_cannot_authorize_course_record_access() {
        let student_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        let membership = student_membership(student_account, course);
        assert!(!student_owns_course_record(
            Some(StudentCourseMembership {
                active: false,
                ..membership
            }),
            student_account,
            course,
            membership.membership,
            membership.student,
        ));
    }

    #[test]
    fn non_student_membership_cannot_authorize_course_record_access() {
        let student_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        let membership = student_membership(student_account, course);
        assert!(!student_owns_course_record(
            Some(StudentCourseMembership {
                role: CourseMembershipRole::Instructor,
                ..membership
            }),
            student_account,
            course,
            membership.membership,
            membership.student,
        ));
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
        let active = approval(AccountId::from_uuid(id(3)));
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
                AccountId::from_uuid(id(4)),
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

    fn approval(account: AccountId) -> InstructorApproval {
        InstructorApproval {
            account,
            approved_by: AccountId::from_uuid(id(9)),
            approved_at: stamp(900),
            revoked_at: None,
        }
    }

    fn student_membership(student_account: AccountId, course: CourseId) -> StudentCourseMembership {
        StudentCourseMembership {
            membership: CourseMembershipId::from_uuid(id(6)),
            course,
            student_account,
            student: StudentId::from_uuid(id(5)),
            role: CourseMembershipRole::Student,
            active: true,
        }
    }
}
