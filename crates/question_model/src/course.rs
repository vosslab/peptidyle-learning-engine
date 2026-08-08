//! Browser-safe course, assignment, and course-access projections.

use serde::{Deserialize, Serialize};

use crate::{AssignmentId, CourseId, ProblemVersionRef, RunPolicies, TenantId, UserId};

/// A person's course-specific authorization role.
///
/// This is deliberately separate from the coarse roles carried by a login
/// session: being an instructor somewhere in a tenant does not grant access
/// to every course in that tenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseRole {
    /// Works assignments and views only personal educational records.
    Student,
    /// Manages this course and its assignments.
    Instructor,
    /// Tenant administrator viewing the course through global authority.
    Administrator,
}

/// Authority that may be persisted on one direct course membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseMembershipRole {
    /// Works assignments and views only personal educational records.
    Student,
    /// Manages this course and its assignments.
    Instructor,
}

impl From<CourseMembershipRole> for CourseRole {
    fn from(role: CourseMembershipRole) -> Self {
        match role {
            CourseMembershipRole::Student => Self::Student,
            CourseMembershipRole::Instructor => Self::Instructor,
        }
    }
}

/// One authenticated user's direct membership in a course.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseMembership {
    /// Authenticated person who may enter the course.
    pub user: UserId,
    /// Course-local authority granted to that person.
    pub role: CourseMembershipRole,
}

/// Course information sufficient for the signed-in landing page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSummary {
    /// Durable course identity.
    pub id: CourseId,
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Human-facing course or section title.
    pub title: String,
    /// Signed-in user's authority for this course.
    pub role: CourseRole,
}

/// Browser-safe assignment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentSummary {
    /// Durable assignment identity.
    pub id: AssignmentId,
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Course that owns this assignment.
    pub course_id: CourseId,
    /// Human-facing assignment title.
    pub title: String,
    /// Ordered exact immutable problem versions selected for the assignment.
    pub problems: Vec<ProblemVersionRef>,
    /// Four independent run policies.
    pub policies: RunPolicies,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CompletionRequirement, ContinuedPractice, GradePolicy, ProblemId, VariationPolicy,
        VersionId,
    };
    use uuid::Uuid;

    #[test]
    fn rust_names_serialize_as_lower_camel_course_contracts() {
        let assignment = AssignmentSummary {
            id: AssignmentId::from_uuid(Uuid::from_u128(1)),
            tenant: TenantId::from_uuid(Uuid::from_u128(2)),
            course_id: CourseId::from_uuid(Uuid::from_u128(3)),
            title: "Peptide bonds".to_string(),
            problems: vec![ProblemVersionRef {
                problem: ProblemId::from_uuid(Uuid::from_u128(4)),
                version: VersionId::from_uuid(Uuid::from_u128(5)),
            }],
            policies: RunPolicies {
                completion: CompletionRequirement::AllCorrect,
                grade: GradePolicy::Highest,
                continued_practice: ContinuedPractice::Unlimited,
                variation: VariationPolicy::NewSeeds,
            },
        };

        let value = serde_json::to_value(assignment).expect("assignment should serialize");
        assert!(value.get("courseId").is_some());
        assert!(value.get("course_id").is_none());
    }
}
