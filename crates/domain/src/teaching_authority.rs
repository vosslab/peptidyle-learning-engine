//! Pure WP-INST-T2 validation for Instructor Course Invitations.
//!
//! This module is deliberately separate from S5 entitlement. It validates
//! teaching-operation facts supplied by a Store transaction but never grants
//! student entitlement, calculates effective policy, reads a clock, or writes
//! a direct membership.

use question_model::{
    AccountId, ActivityTimestamp, CourseId, CourseInvitation, CourseInvitationEventKind,
    CourseInvitationState, CourseMembershipId, CourseMembershipRole, InstructorApprovalEvent,
    InstructorApprovalEventKind, StudentRecordId,
};

/// Thirty calendar days expressed in the shared Unix-millisecond representation.
pub const COURSE_INVITATION_LIFETIME_MILLIS: i64 = 30 * 24 * 60 * 60 * 1_000;

/// Current direct Instructor-membership facts for one exact course.
///
/// The Store supplies this projection after locking the durable membership
/// record. It deliberately contains no creator distinction: the course creator
/// and every accepted Teaching Team Member use the same exact membership relation.
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
    approval: Option<InstructorApprovalEvent>,
    instructor_account: AccountId,
    now: ActivityTimestamp,
) -> bool {
    match approval {
        Some(approval) => {
            approval.account == instructor_account
                && approval.kind == InstructorApprovalEventKind::Approved
                && approval.occurred_at <= now
        }
        None => false,
    }
}

/// Returns whether an account is a current Instructor for exactly `course`.
///
/// This is the canonical pure predicate for all course-Instructor operations:
/// current global approval and an active direct Instructor membership are both
/// required. Neither a creator flag nor a Teaching Team Member distinction exists.
pub fn current_course_instructor(
    approval: Option<InstructorApprovalEvent>,
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
    pub student_record: StudentRecordId,
    pub role: CourseMembershipRole,
    pub active: bool,
}

/// Returns whether one account currently owns the exact Student record in an
/// exact course.
///
/// Callers must supply the membership projection from the same protected
/// transaction as the educational-record access. A foreign course, account,
/// Student Record, membership episode, revoked episode, or non-Student role
/// fails closed.
pub fn student_owns_course_record(
    membership: Option<StudentCourseMembership>,
    student_account: AccountId,
    course: CourseId,
    student_record: StudentRecordId,
) -> bool {
    matches!(
        membership,
        Some(membership)
            if membership.active
                && membership.role == CourseMembershipRole::Student
                && membership.course == course
                && membership.student_account == student_account
                && membership.student_record == student_record
    )
}

/// Classifies the exact course-Instructor predicate for callers that need a
/// denial reason without turning a role label into authority.
pub fn evaluate_course_instructor_authority(
    approval: Option<InstructorApprovalEvent>,
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
pub enum CourseInvitationError {
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
pub struct CourseInvitationAcceptance {
    pub course: CourseId,
    pub target: AccountId,
    pub accepted_at: ActivityTimestamp,
}

/// Reads the closed invitation state using only caller-supplied authoritative time.
pub fn invitation_state(
    invitation: &CourseInvitation,
    now: ActivityTimestamp,
) -> Result<CourseInvitationState, CourseInvitationError> {
    validate_invitation_record(invitation, now)?;
    match invitation.terminal_event.map(|event| event.kind) {
        Some(CourseInvitationEventKind::Accepted) => {
            return Ok(CourseInvitationState::Accepted);
        }
        Some(CourseInvitationEventKind::Declined) => {
            return Ok(CourseInvitationState::Declined);
        }
        Some(CourseInvitationEventKind::Revoked) => {
            return Ok(CourseInvitationState::Revoked);
        }
        None if now >= invitation.expires_at => {
            return Ok(CourseInvitationState::Expired);
        }
        None => {}
    }
    Ok(CourseInvitationState::Pending)
}

/// Rechecks approval and produces the required ordinary direct-membership write.
pub fn accept_course_invitation(
    invitation: &CourseInvitation,
    accepting_account: AccountId,
    current_approval: Option<InstructorApprovalEvent>,
    now: ActivityTimestamp,
) -> Result<CourseInvitationAcceptance, CourseInvitationError> {
    match invitation_state(invitation, now)? {
        CourseInvitationState::Pending => {}
        CourseInvitationState::Expired => {
            return Err(CourseInvitationError::InvitationExpired);
        }
        CourseInvitationState::Accepted => {
            return Err(CourseInvitationError::InvitationAlreadyAccepted);
        }
        CourseInvitationState::Declined => {
            return Err(CourseInvitationError::InvitationDeclined);
        }
        CourseInvitationState::Revoked => {
            return Err(CourseInvitationError::InvitationRevoked);
        }
    }
    if accepting_account != invitation.target {
        return Err(CourseInvitationError::WrongTarget);
    }
    let Some(approval) = current_approval else {
        return Err(CourseInvitationError::TargetApprovalRequired);
    };
    validate_instructor_approval(&approval, now)?;
    if approval.account != invitation.target
        || approval.kind != InstructorApprovalEventKind::Approved
    {
        return Err(CourseInvitationError::TargetApprovalRequired);
    }
    Ok(CourseInvitationAcceptance {
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
    approval: &InstructorApprovalEvent,
    now: ActivityTimestamp,
) -> Result<(), CourseInvitationError> {
    if approval.occurred_at > now {
        return Err(CourseInvitationError::InvalidApprovalRecord);
    }
    Ok(())
}

fn validate_invitation_record(
    invitation: &CourseInvitation,
    now: ActivityTimestamp,
) -> Result<(), CourseInvitationError> {
    let expected_expiry = invitation
        .created_at
        .as_unix_millis()
        .checked_add(COURSE_INVITATION_LIFETIME_MILLIS)
        .ok_or(CourseInvitationError::TimestampOverflow)?;
    if invitation.expires_at.as_unix_millis() != expected_expiry {
        return Err(CourseInvitationError::ExpiryDoesNotMatchThirtyDays);
    }
    if let Some(event) = invitation.terminal_event
        && (event.invitation != invitation.id
            || event.occurred_at < invitation.created_at
            || event.occurred_at >= invitation.expires_at
            || event.occurred_at > now
            || (matches!(
                event.kind,
                CourseInvitationEventKind::Accepted | CourseInvitationEventKind::Declined
            ) && event.performed_by != invitation.target))
    {
        return Err(CourseInvitationError::InvalidTerminalTimestamps);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use question_model::{CourseInvitationEvent, CourseInvitationId, CourseMembershipId};
    use uuid::Uuid;

    use super::*;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn stamp(value: i64) -> ActivityTimestamp {
        ActivityTimestamp::from_unix_millis(value)
    }

    fn invitation() -> CourseInvitation {
        CourseInvitation {
            id: CourseInvitationId::from_uuid(id(1)),
            course: CourseId::from_uuid(id(2)),
            invited_by: CourseMembershipId::from_uuid(id(4)),
            membership_role: CourseMembershipRole::Instructor,
            target: AccountId::from_uuid(id(3)),
            created_at: stamp(1_000),
            expires_at: stamp(1_000 + COURSE_INVITATION_LIFETIME_MILLIS),
            terminal_event: None,
        }
    }

    fn terminal_event(
        invitation: &CourseInvitation,
        kind: CourseInvitationEventKind,
        performed_by: AccountId,
        occurred_at: ActivityTimestamp,
    ) -> CourseInvitationEvent {
        CourseInvitationEvent {
            invitation: invitation.id,
            kind,
            performed_by,
            occurred_at,
        }
    }

    #[test]
    fn current_course_instructor_requires_active_approval_and_membership() {
        let instructor_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        let approval = InstructorApprovalEvent {
            account: instructor_account,
            authorized_by: AccountId::from_uuid(id(9)),
            kind: InstructorApprovalEventKind::Approved,
            occurred_at: stamp(10),
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
            Some(InstructorApprovalEvent {
                kind: InstructorApprovalEventKind::Revoked,
                occurred_at: stamp(1_001),
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
            membership.student_record,
        ));
        assert!(!student_owns_course_record(
            Some(membership),
            student_account,
            course,
            StudentRecordId::from_uuid(id(7)),
        ));
    }

    #[test]
    fn renewed_student_membership_reuses_the_stable_course_record() {
        let student_account = AccountId::from_uuid(id(3));
        let course = CourseId::from_uuid(id(2));
        let membership = student_membership(student_account, course);
        assert!(student_owns_course_record(
            Some(StudentCourseMembership {
                membership: CourseMembershipId::from_uuid(id(7)),
                ..membership
            }),
            student_account,
            course,
            membership.student_record,
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
            membership.student_record,
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
            membership.student_record,
        ));
    }

    #[test]
    fn invitation_transitions_are_target_bound_and_thirty_days() {
        let pending = invitation();
        assert_eq!(
            invitation_state(&pending, stamp(1_001)),
            Ok(CourseInvitationState::Pending)
        );
        assert_eq!(
            invitation_state(&pending, pending.expires_at),
            Ok(CourseInvitationState::Expired)
        );

        let accepted = CourseInvitation {
            terminal_event: Some(terminal_event(
                &pending,
                CourseInvitationEventKind::Accepted,
                pending.target,
                stamp(1_010),
            )),
            ..pending.clone()
        };
        assert_eq!(
            invitation_state(&accepted, stamp(1_011)),
            Ok(CourseInvitationState::Accepted)
        );
        let revoked = CourseInvitation {
            terminal_event: Some(terminal_event(
                &pending,
                CourseInvitationEventKind::Revoked,
                AccountId::from_uuid(id(9)),
                stamp(1_010),
            )),
            ..pending.clone()
        };
        assert_eq!(
            invitation_state(&revoked, stamp(1_011)),
            Ok(CourseInvitationState::Revoked)
        );
        let declined = CourseInvitation {
            terminal_event: Some(terminal_event(
                &pending,
                CourseInvitationEventKind::Declined,
                pending.target,
                stamp(1_010),
            )),
            ..pending
        };
        assert_eq!(
            invitation_state(&declined, stamp(1_011)),
            Ok(CourseInvitationState::Declined)
        );
    }

    #[test]
    fn invitation_refuses_invalid_lifetime_and_terminal_records() {
        let invitation = invitation();
        let invalid_lifetime = CourseInvitation {
            expires_at: stamp(invitation.expires_at.as_unix_millis() - 1),
            ..invitation.clone()
        };
        assert_eq!(
            invitation_state(&invalid_lifetime, stamp(1_001)),
            Err(CourseInvitationError::ExpiryDoesNotMatchThirtyDays)
        );
        let invalid_terminal = CourseInvitation {
            terminal_event: Some(CourseInvitationEvent {
                invitation: CourseInvitationId::from_uuid(id(99)),
                kind: CourseInvitationEventKind::Accepted,
                performed_by: invitation.target,
                occurred_at: stamp(1_010),
            }),
            ..invitation
        };
        assert_eq!(
            invitation_state(&invalid_terminal, stamp(1_012)),
            Err(CourseInvitationError::InvalidTerminalTimestamps)
        );
    }

    #[test]
    fn invitation_terminal_timestamp_must_not_be_later_than_now() {
        let now = stamp(1_010);
        for (kind, expected_state) in [
            (
                CourseInvitationEventKind::Accepted,
                CourseInvitationState::Accepted,
            ),
            (
                CourseInvitationEventKind::Declined,
                CourseInvitationState::Declined,
            ),
            (
                CourseInvitationEventKind::Revoked,
                CourseInvitationState::Revoked,
            ),
        ] {
            let pending = invitation();
            let terminal = CourseInvitation {
                terminal_event: Some(terminal_event(
                    &pending,
                    kind,
                    if matches!(kind, CourseInvitationEventKind::Revoked) {
                        AccountId::from_uuid(id(9))
                    } else {
                        pending.target
                    },
                    now,
                )),
                ..pending
            };
            assert_eq!(invitation_state(&terminal, now), Ok(expected_state));
        }
        for kind in [
            CourseInvitationEventKind::Accepted,
            CourseInvitationEventKind::Declined,
            CourseInvitationEventKind::Revoked,
        ] {
            let pending = invitation();
            let terminal = CourseInvitation {
                terminal_event: Some(terminal_event(
                    &pending,
                    kind,
                    if matches!(kind, CourseInvitationEventKind::Revoked) {
                        AccountId::from_uuid(id(9))
                    } else {
                        pending.target
                    },
                    stamp(1_011),
                )),
                ..pending
            };
            assert_eq!(
                invitation_state(&terminal, now),
                Err(CourseInvitationError::InvalidTerminalTimestamps)
            );
        }
    }

    #[test]
    fn approval_record_chronology_is_checked_at_caller_time() {
        let now = stamp(1_001);
        let active = approval(AccountId::from_uuid(id(3)));
        assert_eq!(validate_instructor_approval(&active, now), Ok(()));

        let revoked = InstructorApprovalEvent {
            kind: InstructorApprovalEventKind::Revoked,
            occurred_at: stamp(950),
            ..active
        };
        assert_eq!(validate_instructor_approval(&revoked, now), Ok(()));

        let invalid = InstructorApprovalEvent {
            occurred_at: stamp(1_002),
            ..active
        };
        assert_eq!(
            validate_instructor_approval(&invalid, now),
            Err(CourseInvitationError::InvalidApprovalRecord)
        );
    }

    #[test]
    fn acceptance_rechecks_current_approval_before_direct_membership_write() {
        let invitation = invitation();
        let now = stamp(1_001);
        assert_eq!(
            accept_course_invitation(&invitation, invitation.target, None, now),
            Err(CourseInvitationError::TargetApprovalRequired)
        );
        assert_eq!(
            accept_course_invitation(
                &invitation,
                AccountId::from_uuid(id(4)),
                Some(approval(invitation.target)),
                now,
            ),
            Err(CourseInvitationError::WrongTarget)
        );
        assert_eq!(
            accept_course_invitation(
                &invitation,
                invitation.target,
                Some(approval(invitation.target)),
                now,
            ),
            Ok(CourseInvitationAcceptance {
                course: invitation.course,
                target: invitation.target,
                accepted_at: now,
            })
        );
        let revoked_approval = InstructorApprovalEvent {
            kind: InstructorApprovalEventKind::Revoked,
            occurred_at: stamp(950),
            ..approval(invitation.target)
        };
        assert_eq!(
            accept_course_invitation(&invitation, invitation.target, Some(revoked_approval), now,),
            Err(CourseInvitationError::TargetApprovalRequired)
        );
        let future_approval = InstructorApprovalEvent {
            occurred_at: stamp(1_002),
            ..approval(invitation.target)
        };
        assert_eq!(
            accept_course_invitation(&invitation, invitation.target, Some(future_approval), now,),
            Err(CourseInvitationError::InvalidApprovalRecord)
        );
        let future_revocation = InstructorApprovalEvent {
            kind: InstructorApprovalEventKind::Revoked,
            occurred_at: stamp(1_002),
            ..approval(invitation.target)
        };
        assert_eq!(
            accept_course_invitation(&invitation, invitation.target, Some(future_revocation), now,),
            Err(CourseInvitationError::InvalidApprovalRecord)
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

    fn approval(account: AccountId) -> InstructorApprovalEvent {
        InstructorApprovalEvent {
            account,
            authorized_by: AccountId::from_uuid(id(9)),
            kind: InstructorApprovalEventKind::Approved,
            occurred_at: stamp(900),
        }
    }

    fn student_membership(student_account: AccountId, course: CourseId) -> StudentCourseMembership {
        StudentCourseMembership {
            membership: CourseMembershipId::from_uuid(id(6)),
            course,
            student_account,
            student_record: StudentRecordId::from_uuid(id(5)),
            role: CourseMembershipRole::Student,
            active: true,
        }
    }
}
