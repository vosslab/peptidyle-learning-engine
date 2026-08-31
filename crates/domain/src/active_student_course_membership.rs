//! Pure active-membership gate for Student Assignment Access.

use question_model::{AccountId, AssignmentId, CourseId, CourseMembershipId, StudentRecordId};

/// Why the active-membership prerequisite for Assignment Access is absent.
/// Reasons are internal and never a Student DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveStudentCourseMembershipDenial {
    CourseNotFound,
    AssignmentNotFound,
    AssignmentOutsideCourse,
    StudentNotActiveCourse,
}

/// A successful authority decision. Its fields remain private so only this
/// evaluator can mint a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveStudentCourseMembershipGrant {
    course: CourseId,
    assignment: AssignmentId,
    student_account: AccountId,
    student_record: StudentRecordId,
    membership: CourseMembershipId,
}

impl ActiveStudentCourseMembershipGrant {
    pub fn course(&self) -> CourseId {
        self.course
    }
    pub fn assignment(&self) -> AssignmentId {
        self.assignment
    }
    pub fn student_account(&self) -> AccountId {
        self.student_account
    }
    pub fn student_record(&self) -> StudentRecordId {
        self.student_record
    }
    pub fn membership(&self) -> CourseMembershipId {
        self.membership
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveStudentCourseMembershipDecision {
    Granted(ActiveStudentCourseMembershipGrant),
    Denied(ActiveStudentCourseMembershipDenial),
}

/// Identity-free normalized facts for a synthetic T3 preview subject.
///
/// Neither this type nor its resulting grant can represent a Student,
/// membership, or persisted Student Record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticPreviewAdmissionFacts {
    course: CourseId,
    assignment: AssignmentId,
}

impl SyntheticPreviewAdmissionFacts {
    pub fn new(course: CourseId, assignment: AssignmentId) -> Self {
        Self { course, assignment }
    }
}

/// S5-minted synthetic preview authority. Its fields deliberately remain
/// private so only this module can approve synthetic preview authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticPreviewAdmission {
    course: CourseId,
    assignment: AssignmentId,
}

impl SyntheticPreviewAdmission {
    pub fn course(&self) -> CourseId {
        self.course
    }
    pub fn assignment(&self) -> AssignmentId {
        self.assignment
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntheticPreviewAdmissionDecision {
    Granted(SyntheticPreviewAdmission),
    Denied(ActiveStudentCourseMembershipDenial),
}

/// All normalized facts the Store must load under its transaction boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveStudentCourseMembershipFacts {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub student_account: AccountId,
    pub membership: Option<ActiveStudentMembership>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveStudentMembership {
    pub id: CourseMembershipId,
    pub student_record: StudentRecordId,
}

/// Evaluates the active-membership prerequisite for Assignment Access.
///
/// Effective Assignment Policy applies lifecycle, schedule, late-work, and
/// action rules after this gate succeeds.
pub fn evaluate_active_student_course_membership(
    facts: ActiveStudentCourseMembershipFacts,
) -> ActiveStudentCourseMembershipDecision {
    let Some(membership) = facts.membership else {
        return ActiveStudentCourseMembershipDecision::Denied(
            ActiveStudentCourseMembershipDenial::StudentNotActiveCourse,
        );
    };
    ActiveStudentCourseMembershipDecision::Granted(ActiveStudentCourseMembershipGrant {
        course: facts.course,
        assignment: facts.assignment,
        student_account: facts.student_account,
        student_record: membership.student_record,
        membership: membership.id,
    })
}

/// Decides synthetic preview authority without a Student identity or a
/// Student authority token.
pub fn admit_synthetic_preview(
    facts: SyntheticPreviewAdmissionFacts,
) -> SyntheticPreviewAdmissionDecision {
    SyntheticPreviewAdmissionDecision::Granted(SyntheticPreviewAdmission {
        course: facts.course,
        assignment: facts.assignment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::CourseMembershipId;
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    #[test]
    fn active_student_membership_grants_direct_assignment_access() {
        let decision =
            evaluate_active_student_course_membership(ActiveStudentCourseMembershipFacts {
                course: CourseId::from_uuid(id(2)),
                assignment: AssignmentId::from_uuid(id(3)),
                student_account: AccountId::from_uuid(id(4)),
                membership: Some(ActiveStudentMembership {
                    id: CourseMembershipId::from_uuid(id(5)),
                    student_record: StudentRecordId::from_uuid(id(6)),
                }),
            });
        assert!(matches!(
            decision,
            ActiveStudentCourseMembershipDecision::Granted(_)
        ));
    }

    #[test]
    fn direct_access_has_the_active_membership_basis() {
        let decision =
            evaluate_active_student_course_membership(ActiveStudentCourseMembershipFacts {
                course: CourseId::from_uuid(id(2)),
                assignment: AssignmentId::from_uuid(id(3)),
                student_account: AccountId::from_uuid(id(4)),
                membership: Some(ActiveStudentMembership {
                    id: CourseMembershipId::from_uuid(id(5)),
                    student_record: StudentRecordId::from_uuid(id(6)),
                }),
            });
        let ActiveStudentCourseMembershipDecision::Granted(grant) = decision else {
            panic!("active membership should grant assignment authority");
        };
        assert_eq!(grant.membership(), CourseMembershipId::from_uuid(id(5)));
    }

    #[test]
    fn synthetic_preview_grants_direct_assignment_access() {
        let decision = admit_synthetic_preview(SyntheticPreviewAdmissionFacts::new(
            CourseId::from_uuid(id(2)),
            AssignmentId::from_uuid(id(3)),
        ));
        assert!(matches!(
            decision,
            SyntheticPreviewAdmissionDecision::Granted(_)
        ));
    }
}
