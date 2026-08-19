//! Browser-safe course, assignment, and course-access projections.

use serde::{Deserialize, Serialize};

use crate::{
    AssignmentDeliveryState, AssignmentId, AssignmentItemId, AssignmentReference,
    AssignmentScoringMode, AssignmentSelectionGroupId, BackendCapabilities, CourseId,
    CourseReference, EnrollmentId, PointValue, QuestionBackend, QuestionId, RunPolicies,
    SelectionOrdering, StudentAssignmentSummary, StudentId, TenantId,
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

/// Course information sufficient for the signed-in landing page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSummary {
    /// Durable course identity.
    pub id: CourseId,
    /// Stable typed locator used in application navigation.
    pub reference: CourseReference,
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Human-facing course or section title.
    pub title: String,
    /// Required inclusive term bounds and authoritative scheduling zone.
    pub term: crate::CourseTerm,
    /// Signed-in user's authority for this course.
    pub role: CourseMembershipRole,
}

/// Browser-safe assignment definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentItemSummary {
    /// Server-minted identity for this editable assignment slot.
    pub id: AssignmentItemId,
    /// Sole browser-visible locator for the immutable published question.
    pub question_id: QuestionId,
    /// Safe catalog label shown while editing this assignment.
    pub title: String,
    /// Adapter family selected for the item.
    pub backend: QuestionBackend,
    /// Capabilities declared for the published question.
    pub capabilities: BackendCapabilities,
    /// Zero-based position used for future runs.
    pub position: u32,
    /// Current assignment-authored points.
    pub points_possible: PointValue,
    /// Whether future runs may receive the item.
    pub delivery_state: AssignmentDeliveryState,
    /// Current-only scoring treatment.
    pub scoring_mode: AssignmentScoringMode,
}

/// Browser-safe candidate in one random-selection group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentSelectionCandidateSummary {
    /// Server-minted identity for this editable selection candidate.
    pub id: AssignmentItemId,
    /// Sole browser-visible locator for the immutable published question.
    pub question_id: QuestionId,
    /// Safe catalog label shown while editing this assignment.
    pub title: String,
    /// Adapter family selected for the candidate.
    pub backend: QuestionBackend,
    /// Capabilities declared for the published question.
    pub capabilities: BackendCapabilities,
    /// Zero-based authored order within this selection group.
    pub position: u32,
    /// Whether future runs may select this candidate.
    pub delivery_state: AssignmentDeliveryState,
}

/// Browser-safe random-selection definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentSelectionGroupSummary {
    /// Stable group identity.
    pub id: AssignmentSelectionGroupId,
    /// Position of this group among fixed items and other groups.
    pub position: u32,
    /// Number of active candidates selected for each future run.
    pub draw_count: u32,
    /// Uniform current points for each selected candidate.
    pub points_per_item: PointValue,
    /// Output ordering after selection.
    pub ordering: SelectionOrdering,
    /// Stable algorithm version needed to reproduce selection.
    pub algorithm_version: u16,
    /// Browser-safe current candidate set.
    pub candidates: Vec<AssignmentSelectionCandidateSummary>,
}

/// Browser-safe assignment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentSummary {
    /// Durable assignment identity.
    pub id: AssignmentId,
    /// Stable typed locator used in application navigation.
    pub reference: AssignmentReference,
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Course that owns this assignment.
    pub course_id: CourseId,
    /// Human-facing assignment title.
    pub title: String,
    /// Ordered stable fixed items in the current assignment definition.
    pub items: Vec<AssignmentItemSummary>,
    /// Current random-selection groups with pinned immutable candidates.
    pub selection_groups: Vec<AssignmentSelectionGroupSummary>,
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
    /// Human-facing learner name from the protected course roster.
    pub learner_name: String,
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
    use crate::{CompletionRequirement, ContinuedPractice, GradePolicy, VariationPolicy};
    use uuid::Uuid;

    #[test]
    fn rust_names_serialize_as_lower_camel_course_contracts() {
        let assignment = AssignmentSummary {
            id: AssignmentId::from_uuid(Uuid::from_u128(1)),
            reference: crate::AssignmentReference::new(1).expect("valid reference"),
            tenant: TenantId::from_uuid(Uuid::from_u128(2)),
            course_id: CourseId::from_uuid(Uuid::from_u128(3)),
            title: "Peptide bonds".to_string(),
            items: vec![AssignmentItemSummary {
                id: crate::AssignmentItemId::from_uuid(Uuid::from_u128(4)),
                question_id: "7K3-M9QX".parse().expect("fixture Question ID parses"),
                title: "Peptide bonds".to_string(),
                backend: crate::QuestionBackend::Native,
                capabilities: crate::BackendCapabilities::none(),
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
        let item = &value["items"][0];
        assert_eq!(item["questionId"], "7K3-M9QX");
        assert!(item.get("reference").is_none());
    }

    #[test]
    fn gradebook_summary_row_keeps_the_projection_nested() {
        let row = GradebookSummaryRow {
            tenant: TenantId::from_uuid(Uuid::from_u128(1)),
            course_id: CourseId::from_uuid(Uuid::from_u128(2)),
            enrollment_id: EnrollmentId::from_uuid(Uuid::from_u128(3)),
            student_id: StudentId::from_uuid(Uuid::from_u128(4)),
            learner_name: "Ada Learner".to_string(),
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
        assert_eq!(
            value.get("learnerName").and_then(|name| name.as_str()),
            Some("Ada Learner")
        );
        assert!(value.get("summary").is_some());
        assert!(value.get("bestScore").is_none());
    }
}
