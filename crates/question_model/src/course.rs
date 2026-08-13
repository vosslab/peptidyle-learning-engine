//! Browser-safe course, assignment, and course-access projections.

use serde::{Deserialize, Serialize};

use crate::{
    AssignmentId, AssignmentItem, AssignmentSelectionGroup, CourseId, EnrollmentId, RunPolicies,
    StudentAssignmentSummary, StudentId, TenantId, UserId,
};

/// Relationship that may be persisted on one direct course membership.
///
/// This scopes a Student or Instructor to one course. It is not another
/// inventory of human user roles, and Sysadmin is never a membership value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseMembershipRole {
    /// Works assignments and views only personal educational records.
    Student,
    /// Manages this course and its assignments.
    Instructor,
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
    pub role: CourseMembershipRole,
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
    /// Ordered stable fixed items in the current assignment definition.
    pub items: Vec<AssignmentItem>,
    /// Current random-selection groups with pinned immutable candidates.
    pub selection_groups: Vec<AssignmentSelectionGroup>,
    /// Four independent run policies.
    pub policies: RunPolicies,
}

/// One compact gradebook row for a course assignment enrollment.
///
/// The row comes only from the tenant-owned assignment, enrollment, and
/// `StudentAssignmentSummary` projection. It carries no run or attempt
/// history, so continued practice cannot make the default gradebook slower.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradebookSummaryRow {
    /// RLS boundary carried directly on this educational record projection.
    pub tenant: TenantId,
    /// Course whose instructor requested this bounded page.
    pub course_id: CourseId,
    /// Tenant-owned enrollment represented by this row.
    pub enrollment_id: EnrollmentId,
    /// Stable learner identity used by course records.
    pub student_id: StudentId,
    /// Assignment whose grade policy selected the current score.
    pub assignment_id: AssignmentId,
    /// Human-facing assignment title from the assignment record.
    pub assignment_title: String,
    /// Transactionally maintained compact activity and score projection.
    pub summary: StudentAssignmentSummary,
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
            items: vec![AssignmentItem {
                id: crate::AssignmentItemId::from_uuid(Uuid::from_u128(4)),
                reference: crate::ProblemVersionRef {
                    problem: ProblemId::from_uuid(Uuid::from_u128(5)),
                    version: VersionId::from_uuid(Uuid::from_u128(6)),
                },
                position: 0,
                points_possible: crate::PointValue::from_whole(1),
                delivery_state: crate::AssignmentDeliveryState::Active,
                scoring_mode: crate::AssignmentScoringMode::Normal,
            }],
            selection_groups: Vec::new(),
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

    #[test]
    fn gradebook_summary_row_keeps_the_projection_nested() {
        let row = GradebookSummaryRow {
            tenant: TenantId::from_uuid(Uuid::from_u128(1)),
            course_id: CourseId::from_uuid(Uuid::from_u128(2)),
            enrollment_id: EnrollmentId::from_uuid(Uuid::from_u128(3)),
            student_id: StudentId::from_uuid(Uuid::from_u128(4)),
            assignment_id: AssignmentId::from_uuid(Uuid::from_u128(5)),
            assignment_title: "Peptide bonds".to_string(),
            summary: StudentAssignmentSummary::empty(
                TenantId::from_uuid(Uuid::from_u128(1)),
                EnrollmentId::from_uuid(Uuid::from_u128(3)),
            ),
        };

        let value = serde_json::to_value(row).expect("gradebook row should serialize");
        assert!(value.get("courseId").is_some());
        assert!(value.get("assignmentTitle").is_some());
        assert!(value.get("summary").is_some());
        assert!(value.get("bestScore").is_none());
    }
}
