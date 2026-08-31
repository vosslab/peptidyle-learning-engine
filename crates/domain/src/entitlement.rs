//! Pure entitlement evaluation over normalized facts.

use question_model::{
    AccountId, AssignmentId, CourseId, CourseMembershipId, MaterializationBasis, StudentRecordId,
};

/// Why current authority is absent. Reasons are internal and never a Student DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitlementDenial {
    CourseNotFound,
    AssignmentNotFound,
    AssignmentOutsideCourse,
    StudentNotActiveCourse,
}

/// A successful authority decision. Its fields remain private so only this
/// evaluator can mint a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitlementGrant {
    course: CourseId,
    assignment: AssignmentId,
    student_account: AccountId,
    student_record: StudentRecordId,
    membership: CourseMembershipId,
    basis: MaterializationBasis,
}

impl EntitlementGrant {
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
    pub fn basis(&self) -> MaterializationBasis {
        self.basis
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntitlementDecision {
    Granted(EntitlementGrant),
    Denied(EntitlementDenial),
}

/// Identity-free normalized facts for a synthetic T3 preview subject.
///
/// Neither this type nor its resulting grant can represent a Student,
/// membership, or persisted Student Record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticPreviewEntitlementFacts {
    course: CourseId,
    assignment: AssignmentId,
}

impl SyntheticPreviewEntitlementFacts {
    pub fn new(course: CourseId, assignment: AssignmentId) -> Self {
        Self { course, assignment }
    }
}

/// S5-minted synthetic preview authority. Its fields deliberately remain
/// private so only this module can approve synthetic preview authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticPreviewEntitlementGrant {
    course: CourseId,
    assignment: AssignmentId,
    basis: MaterializationBasis,
}

impl SyntheticPreviewEntitlementGrant {
    pub fn course(&self) -> CourseId {
        self.course
    }
    pub fn assignment(&self) -> AssignmentId {
        self.assignment
    }
    pub fn basis(&self) -> MaterializationBasis {
        self.basis
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntheticPreviewEntitlementDecision {
    Granted(SyntheticPreviewEntitlementGrant),
    Denied(EntitlementDenial),
}

/// All normalized facts the Store must load under its transaction boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitlementFacts {
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

/// Decides current assignment authority. This intentionally contains no
/// lifecycle, scheduling, late-work, disclosure, or receipt logic.
pub fn evaluate_assignment_entitlement(facts: EntitlementFacts) -> EntitlementDecision {
    let Some(membership) = facts.membership else {
        return EntitlementDecision::Denied(EntitlementDenial::StudentNotActiveCourse);
    };
    EntitlementDecision::Granted(EntitlementGrant {
        course: facts.course,
        assignment: facts.assignment,
        student_account: facts.student_account,
        student_record: membership.student_record,
        membership: membership.id,
        basis: MaterializationBasis::ActiveStudentCourseMembership,
    })
}

/// Decides synthetic preview authority without a Student identity or a
/// Student authority token.
pub fn evaluate_synthetic_preview_entitlement(
    facts: SyntheticPreviewEntitlementFacts,
) -> SyntheticPreviewEntitlementDecision {
    SyntheticPreviewEntitlementDecision::Granted(SyntheticPreviewEntitlementGrant {
        course: facts.course,
        assignment: facts.assignment,
        basis: MaterializationBasis::ActiveStudentCourseMembership,
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
        let decision = evaluate_assignment_entitlement(EntitlementFacts {
            course: CourseId::from_uuid(id(2)),
            assignment: AssignmentId::from_uuid(id(3)),
            student_account: AccountId::from_uuid(id(4)),
            membership: Some(ActiveStudentMembership {
                id: CourseMembershipId::from_uuid(id(5)),
                student_record: StudentRecordId::from_uuid(id(6)),
            }),
        });
        assert!(matches!(decision, EntitlementDecision::Granted(_)));
    }

    #[test]
    fn direct_access_has_the_active_membership_basis() {
        let decision = evaluate_assignment_entitlement(EntitlementFacts {
            course: CourseId::from_uuid(id(2)),
            assignment: AssignmentId::from_uuid(id(3)),
            student_account: AccountId::from_uuid(id(4)),
            membership: Some(ActiveStudentMembership {
                id: CourseMembershipId::from_uuid(id(5)),
                student_record: StudentRecordId::from_uuid(id(6)),
            }),
        });
        let EntitlementDecision::Granted(grant) = decision else {
            panic!("active membership should grant assignment authority");
        };
        assert_eq!(
            grant.basis(),
            MaterializationBasis::ActiveStudentCourseMembership
        );
    }

    #[test]
    fn synthetic_preview_grants_direct_assignment_access() {
        let decision =
            evaluate_synthetic_preview_entitlement(SyntheticPreviewEntitlementFacts::new(
                CourseId::from_uuid(id(2)),
                AssignmentId::from_uuid(id(3)),
            ));
        assert!(matches!(
            decision,
            SyntheticPreviewEntitlementDecision::Granted(_)
        ));
    }
}
