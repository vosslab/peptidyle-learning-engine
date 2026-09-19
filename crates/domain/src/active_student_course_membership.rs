//! Pure active-membership gate for Student Assessment Access.

use question_model::{
    AccountId, AssessmentId, CourseInstanceId, CourseMembershipId, StudentRecordId,
};

/// Why the active-membership prerequisite for Assessment Access is absent.
/// Reasons are internal and never a Student DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveStudentCourseMembershipDenial {
    CourseNotFound,
    AssessmentNotFound,
    AssessmentOutsideCourse,
    StudentNotActiveCourse,
}

/// A successful authority decision. Its fields remain private so only this
/// evaluator can mint a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveStudentCourseMembershipGrant {
    course: CourseInstanceId,
    assessment: AssessmentId,
    student_account: AccountId,
    student_record: StudentRecordId,
    membership: CourseMembershipId,
}

impl ActiveStudentCourseMembershipGrant {
    pub fn course(&self) -> CourseInstanceId {
        self.course.clone()
    }
    pub fn assessment(&self) -> AssessmentId {
        self.assessment.clone()
    }
    pub fn student_account(&self) -> AccountId {
        self.student_account.clone()
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

/// Identity-free normalized facts for a Hypothetical Student View Scenario.
///
/// Neither this type nor its resulting grant can represent a Student,
/// membership, or persisted Student Record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypotheticalStudentViewScenarioAdmissionFacts {
    course: CourseInstanceId,
    assessment: AssessmentId,
}

impl HypotheticalStudentViewScenarioAdmissionFacts {
    pub fn new(course: CourseInstanceId, assessment: AssessmentId) -> Self {
        Self { course, assessment }
    }
}

/// Module-minted Hypothetical Student View Scenario admission. Its fields deliberately
/// remain private so only this module can approve scenario scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypotheticalStudentViewScenarioAdmission {
    course: CourseInstanceId,
    assessment: AssessmentId,
}

impl HypotheticalStudentViewScenarioAdmission {
    pub fn course(&self) -> CourseInstanceId {
        self.course.clone()
    }
    pub fn assessment(&self) -> AssessmentId {
        self.assessment.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypotheticalStudentViewScenarioAdmissionDenial {
    CourseNotFound,
    AssessmentNotFound,
    AssessmentOutsideCourse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HypotheticalStudentViewScenarioAdmissionDecision {
    Granted(HypotheticalStudentViewScenarioAdmission),
    Denied(HypotheticalStudentViewScenarioAdmissionDenial),
}

/// All normalized facts the Store must load under its transaction boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveStudentCourseMembershipFacts {
    pub course: CourseInstanceId,
    pub assessment: AssessmentId,
    pub student_account: AccountId,
    pub membership: Option<ActiveStudentMembership>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveStudentMembership {
    pub id: CourseMembershipId,
    pub student_record: StudentRecordId,
}

/// Evaluates the active-membership prerequisite for Assessment Access.
///
/// Effective Assessment Policy applies lifecycle, schedule, late-work, and
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
        assessment: facts.assessment,
        student_account: facts.student_account,
        student_record: membership.student_record,
        membership: membership.id,
    })
}

/// Mints scenario admission after Store-owned course and Assessment scope resolution.
///
/// The admission carries neither a Student identity nor a Student authority token.
pub fn admit_hypothetical_student_view_scenario(
    facts: HypotheticalStudentViewScenarioAdmissionFacts,
) -> HypotheticalStudentViewScenarioAdmissionDecision {
    HypotheticalStudentViewScenarioAdmissionDecision::Granted(
        HypotheticalStudentViewScenarioAdmission {
            course: facts.course,
            assessment: facts.assessment,
        },
    )
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
    fn active_student_membership_grants_direct_assessment_access() {
        let decision =
            evaluate_active_student_course_membership(ActiveStudentCourseMembershipFacts {
                course: CourseInstanceId::from_debug_serial(2),
                assessment: AssessmentId::from_debug_serial(3),
                student_account: AccountId::from_debug_serial(4),
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
                course: CourseInstanceId::from_debug_serial(2),
                assessment: AssessmentId::from_debug_serial(3),
                student_account: AccountId::from_debug_serial(4),
                membership: Some(ActiveStudentMembership {
                    id: CourseMembershipId::from_uuid(id(5)),
                    student_record: StudentRecordId::from_uuid(id(6)),
                }),
            });
        let ActiveStudentCourseMembershipDecision::Granted(grant) = decision else {
            panic!("active membership should grant assessment authority");
        };
        assert_eq!(grant.membership(), CourseMembershipId::from_uuid(id(5)));
    }

    #[test]
    fn hypothetical_student_view_scenario_grants_scope_admission() {
        let decision = admit_hypothetical_student_view_scenario(
            HypotheticalStudentViewScenarioAdmissionFacts::new(
                CourseInstanceId::from_debug_serial(2),
                AssessmentId::from_debug_serial(3),
            ),
        );
        assert!(matches!(
            decision,
            HypotheticalStudentViewScenarioAdmissionDecision::Granted(_)
        ));
    }
}
