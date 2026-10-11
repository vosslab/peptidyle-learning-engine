//! Pure validation for Instructor Course Invitations.
//!
//! This module is deliberately separate from Student Assessment Access. It validates
//! teaching-operation facts supplied by a Store transaction but never grants
//! Student Assessment Access, calculates effective policy, reads a clock, or writes
//! a direct membership.

use question_model::{
    AccountId, CourseInstanceId, CourseInvitation, CourseInvitationEventKind,
    CourseInvitationState, CourseMembershipId, CourseMembershipRole, StudentRecordId, Timestamp,
};

/// Thirty calendar days expressed in the shared Unix-millisecond representation.
pub const COURSE_INVITATION_LIFETIME_MILLIS: i64 = 30 * 24 * 60 * 60 * 1_000;

/// Current Instructor Course Membership facts for one exact course.
///
/// The Store supplies these Instructor Course Membership facts after locking
/// the durable membership record. It deliberately contains no creator distinction:
/// the course creator and every accepted Teaching Team Member use the same exact membership relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentInstructorCourseMembership {
    pub course_instance_id: CourseInstanceId,
    pub instructor_account: AccountId,
    pub active: bool,
}

/// Explicit result of a current course-Instructor authorization evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructorAuthority {
    CurrentCourseInstructor,
    NoCurrentInstructorCourseMembership,
}

/// Returns whether an account is a current Instructor for exactly `course`.
///
/// This is the shared pure predicate for all course-Instructor operations.
/// The Store establishes the Account's immutable Instructor User Role before
/// it creates an Instructor Course Membership. The Assigned Instructor remains
/// an accountability fact; it creates no authority distinction among Teaching Team Members.
pub fn current_course_instructor(
    membership: Option<CurrentInstructorCourseMembership>,
    instructor_account: AccountId,
    course_instance_id: CourseInstanceId,
) -> bool {
    matches!(
        membership,
        Some(membership)
            if membership.active
                && membership.course_instance_id == course_instance_id
                && membership.instructor_account == instructor_account
    )
}

/// Current Student-membership facts that bind a global account to one exact
/// Student educational-record identity in one course.
///
/// The membership episode remains distinct from the global account and from
/// the Student identity, so revocation and re-enrollment cannot rewrite past
/// evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudentCourseMembership {
    pub membership: CourseMembershipId,
    pub course_instance_id: CourseInstanceId,
    pub student_account: AccountId,
    pub student_record: StudentRecordId,
    pub role: CourseMembershipRole,
    pub active: bool,
}

/// Returns whether one account currently owns the exact Student record in an
/// exact course.
///
/// Callers must supply the Student Course Membership facts from the same
/// protected transaction as the educational-record access. A foreign course, account,
/// Student Record, membership episode, revoked episode, or non-Student role
/// fails closed.
pub fn student_owns_course_record(
    membership: Option<StudentCourseMembership>,
    student_account: AccountId,
    course_instance_id: CourseInstanceId,
    student_record: StudentRecordId,
) -> bool {
    matches!(
        membership,
        Some(membership)
            if membership.active
                && membership.role == CourseMembershipRole::Student
                && membership.course_instance_id == course_instance_id
                && membership.student_account == student_account
                && membership.student_record == student_record
    )
}

/// Classifies the exact course-Instructor predicate for callers that need a
/// denial reason without turning a role label into authority.
pub fn evaluate_course_instructor_authority(
    membership: Option<CurrentInstructorCourseMembership>,
    course_instance_id: CourseInstanceId,
    instructor_account: AccountId,
) -> InstructorAuthority {
    if current_course_instructor(membership, instructor_account, course_instance_id) {
        InstructorAuthority::CurrentCourseInstructor
    } else {
        InstructorAuthority::NoCurrentInstructorCourseMembership
    }
}

/// Errors from deterministic invitation lifecycle validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInvitationError {
    ExpiryDoesNotMatchThirtyDays,
    TimestampOverflow,
    InvalidTerminalTimestamps,
    InvitationExpired,
    InvitationAlreadyAccepted,
    InvitationDeclined,
    InvitationRevoked,
    WrongTarget,
}

/// The Instructor Course Membership fact a Store must atomically create after acceptance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseInvitationAcceptance {
    pub course_instance_id: CourseInstanceId,
    pub target_account_id: AccountId,
    pub accepted_at: Timestamp,
}

/// Reads the closed invitation state using only caller-supplied authoritative time.
pub fn invitation_state(
    invitation: &CourseInvitation,
    now: Timestamp,
) -> Result<CourseInvitationState, CourseInvitationError> {
    validate_invitation_record(invitation, now)?;
    match invitation.terminal_event.as_ref().map(|event| event.kind) {
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

/// Produces the required ordinary direct-membership write.
pub fn accept_course_invitation(
    invitation: &CourseInvitation,
    accepting_account: AccountId,
    now: Timestamp,
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
    if accepting_account != invitation.target_account_id {
        return Err(CourseInvitationError::WrongTarget);
    }
    Ok(CourseInvitationAcceptance {
        course_instance_id: invitation.course_instance_id.clone(),
        target_account_id: invitation.target_account_id.clone(),
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

fn validate_invitation_record(
    invitation: &CourseInvitation,
    now: Timestamp,
) -> Result<(), CourseInvitationError> {
    let expected_expiry = invitation
        .created_at
        .as_unix_millis()
        .checked_add(COURSE_INVITATION_LIFETIME_MILLIS)
        .ok_or(CourseInvitationError::TimestampOverflow)?;
    if invitation.expires_at.as_unix_millis() != expected_expiry {
        return Err(CourseInvitationError::ExpiryDoesNotMatchThirtyDays);
    }
    if let Some(ref event) = invitation.terminal_event
        && (event.course_invitation_id != invitation.course_invitation_id
            || event.occurred_at < invitation.created_at
            || event.occurred_at >= invitation.expires_at
            || event.occurred_at > now
            || (matches!(
                event.kind,
                CourseInvitationEventKind::Accepted | CourseInvitationEventKind::Declined
            ) && event.performed_by != invitation.target_account_id))
    {
        return Err(CourseInvitationError::InvalidTerminalTimestamps);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn student_membership(
        student_account: &AccountId,
        course_instance_id: &CourseInstanceId,
    ) -> StudentCourseMembership {
        StudentCourseMembership {
            membership: CourseMembershipId::from_uuid(Uuid::from_u128(6)),
            course_instance_id: course_instance_id.clone(),
            student_account: student_account.clone(),
            student_record: StudentRecordId::from_uuid(Uuid::from_u128(5)),
            role: CourseMembershipRole::Student,
            active: true,
        }
    }

    #[test]
    fn current_course_instructor_requires_exact_active_membership() {
        let account = AccountId::from_debug_serial(3);
        let course = CourseInstanceId::from_debug_serial(2);
        let membership = CurrentInstructorCourseMembership {
            course_instance_id: course.clone(),
            instructor_account: account.clone(),
            active: true,
        };
        assert!(current_course_instructor(
            Some(membership.clone()),
            account.clone(),
            course.clone(),
        ));
        assert!(!current_course_instructor(
            Some(CurrentInstructorCourseMembership {
                course_instance_id: CourseInstanceId::from_debug_serial(4),
                ..membership.clone()
            }),
            account.clone(),
            course.clone(),
        ));
        assert!(!current_course_instructor(
            Some(CurrentInstructorCourseMembership {
                instructor_account: AccountId::from_debug_serial(4),
                ..membership.clone()
            }),
            account.clone(),
            course.clone(),
        ));
        assert!(!current_course_instructor(
            Some(CurrentInstructorCourseMembership {
                active: false,
                ..membership
            }),
            account,
            course,
        ));
    }

    #[test]
    fn exact_student_record_is_permitted_and_other_student_is_denied() {
        let account = AccountId::from_debug_serial(3);
        let course = CourseInstanceId::from_debug_serial(2);
        let membership = student_membership(&account, &course);
        assert!(student_owns_course_record(
            Some(membership.clone()),
            account.clone(),
            course.clone(),
            membership.student_record,
        ));
        assert!(!student_owns_course_record(
            Some(membership),
            account,
            course,
            StudentRecordId::from_uuid(Uuid::from_u128(7)),
        ));
    }
}
