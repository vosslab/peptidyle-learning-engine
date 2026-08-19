//! Pure entitlement evaluation over normalized facts.

use question_model::{
    AssignmentAudience, AssignmentId, CourseGroupId, CourseGroupPurpose, CourseId,
    CourseMembershipId, GroupPurposeCapabilities, MaterializationBasis, StudentId, TenantId,
    UserId,
};

/// Evaluator-approved policy scopes. Only this module can create one from
/// normalized current group facts; consumers can only inspect it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicablePolicyScopes(Vec<(CourseGroupId, CourseGroupPurpose)>);

impl ApplicablePolicyScopes {
    fn from_current_memberships(mut scopes: Vec<(CourseGroupId, CourseGroupPurpose)>) -> Self {
        scopes.retain(|(_, purpose)| {
            let capabilities = GroupPurposeCapabilities::for_purpose(*purpose);
            capabilities.schedule_scope || capabilities.accommodation_scope
        });
        scopes.sort_unstable();
        scopes.dedup();
        Self(scopes)
    }

    pub fn contains(&self, group: CourseGroupId, purpose: CourseGroupPurpose) -> bool {
        self.0.binary_search(&(group, purpose)).is_ok()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &(CourseGroupId, CourseGroupPurpose)> {
        self.0.iter()
    }
}

/// Why current authority is absent. Reasons are internal and never a learner DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitlementDenial {
    CourseNotFound,
    AssignmentNotFound,
    AssignmentOutsideCourse,
    LearnerNotActiveCourseStudent,
    AudienceExcludesLearner,
}

/// A successful authority decision. Its fields remain private so only this
/// evaluator can mint a grant or evaluator-approved scopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitlementGrant {
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    learner: UserId,
    student: StudentId,
    membership: CourseMembershipId,
    basis: MaterializationBasis,
    applicable_policy_scopes: ApplicablePolicyScopes,
}

impl EntitlementGrant {
    pub fn tenant(&self) -> TenantId {
        self.tenant
    }
    pub fn course(&self) -> CourseId {
        self.course
    }
    pub fn assignment(&self) -> AssignmentId {
        self.assignment
    }
    pub fn learner(&self) -> UserId {
        self.learner
    }
    pub fn student(&self) -> StudentId {
        self.student
    }
    pub fn membership(&self) -> CourseMembershipId {
        self.membership
    }
    pub fn basis(&self) -> MaterializationBasis {
        self.basis
    }
    pub fn applicable_policy_scopes(&self) -> &ApplicablePolicyScopes {
        &self.applicable_policy_scopes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntitlementDecision {
    Granted(EntitlementGrant),
    Denied(EntitlementDenial),
}

/// All normalized facts the Store must load under its transaction boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitlementFacts {
    pub tenant: TenantId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub learner: UserId,
    pub membership: Option<ActiveStudentMembership>,
    pub audience: AssignmentAudience,
    pub current_groups: Vec<(CourseGroupId, CourseGroupPurpose)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveStudentMembership {
    pub id: CourseMembershipId,
    pub student: StudentId,
}

/// Decides current assignment authority. This intentionally contains no
/// lifecycle, scheduling, late-work, disclosure, or receipt logic.
pub fn evaluate_assignment_entitlement(facts: EntitlementFacts) -> EntitlementDecision {
    let Some(membership) = facts.membership else {
        return EntitlementDecision::Denied(EntitlementDenial::LearnerNotActiveCourseStudent);
    };
    let scopes = ApplicablePolicyScopes::from_current_memberships(facts.current_groups.clone());
    let basis = match facts.audience {
        AssignmentAudience::CourseWide => Some(MaterializationBasis::CourseWide),
        AssignmentAudience::AnyOfGroups(groups) => groups
            .iter()
            .find_map(|audience_group| {
                facts.current_groups.iter().find_map(|(group, purpose)| {
                    (*group == audience_group
                        && GroupPurposeCapabilities::for_purpose(*purpose).assignment_audience)
                        .then_some((*group, *purpose))
                })
            })
            .map(|(group, purpose)| MaterializationBasis::GroupAudience { group, purpose }),
    };
    match basis {
        Some(basis) => EntitlementDecision::Granted(EntitlementGrant {
            tenant: facts.tenant,
            course: facts.course,
            assignment: facts.assignment,
            learner: facts.learner,
            student: membership.student,
            membership: membership.id,
            basis,
            applicable_policy_scopes: scopes,
        }),
        None => EntitlementDecision::Denied(EntitlementDenial::AudienceExcludesLearner),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{CourseGroupId, CourseMembershipId};
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    #[test]
    fn group_audience_is_or_and_work_never_becomes_a_policy_scope() {
        let decision = evaluate_assignment_entitlement(EntitlementFacts {
            tenant: TenantId::from_uuid(id(1)),
            course: CourseId::from_uuid(id(2)),
            assignment: AssignmentId::from_uuid(id(3)),
            learner: UserId::from_uuid(id(4)),
            membership: Some(ActiveStudentMembership {
                id: CourseMembershipId::from_uuid(id(5)),
                student: StudentId::from_uuid(id(6)),
            }),
            audience: AssignmentAudience::any_of_groups(vec![CourseGroupId::from_uuid(id(7))])
                .expect("nonempty audience"),
            current_groups: vec![(CourseGroupId::from_uuid(id(7)), CourseGroupPurpose::Work)],
        });
        assert_eq!(
            decision,
            EntitlementDecision::Denied(EntitlementDenial::AudienceExcludesLearner)
        );
    }

    #[test]
    fn section_audience_grants_and_scopes_are_deduplicated() {
        let section = CourseGroupId::from_uuid(id(7));
        let decision = evaluate_assignment_entitlement(EntitlementFacts {
            tenant: TenantId::from_uuid(id(1)),
            course: CourseId::from_uuid(id(2)),
            assignment: AssignmentId::from_uuid(id(3)),
            learner: UserId::from_uuid(id(4)),
            membership: Some(ActiveStudentMembership {
                id: CourseMembershipId::from_uuid(id(5)),
                student: StudentId::from_uuid(id(6)),
            }),
            audience: AssignmentAudience::any_of_groups(vec![
                section,
                CourseGroupId::from_uuid(id(8)),
            ])
            .expect("nonempty audience"),
            current_groups: vec![
                (section, CourseGroupPurpose::Section),
                (section, CourseGroupPurpose::Section),
                (
                    CourseGroupId::from_uuid(id(9)),
                    CourseGroupPurpose::Accommodation,
                ),
            ],
        });
        let EntitlementDecision::Granted(grant) = decision else {
            panic!("section membership should grant assignment authority");
        };
        assert_eq!(
            grant.basis(),
            MaterializationBasis::GroupAudience {
                group: section,
                purpose: CourseGroupPurpose::Section,
            }
        );
        assert_eq!(grant.applicable_policy_scopes().iter().len(), 2);
    }
}
