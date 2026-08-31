//! Browser-safe course, assignment, and course-access projections.

use serde::{Deserialize, Serialize};

use crate::{
    ActivityTimestamp, AssignmentProgressRecord, AssignmentDeadlineBehavior,
    AssignmentDeliveryState, AssignmentId, AssignmentInstructions, AssignmentItemId,
    AssignmentReference, AssignmentScoringMode, AssignmentSelectionGroupId, BackendCapabilities,
    CourseId, CourseReference, IanaTimeZone, LateSubmissionPolicy, PointValue, QuestionBackend,
    QuestionId, AssignmentActivityRules, ScoringStatus, SelectionOrdering, StudentDisclosurePolicy,
    StudentRecordId, VariationPolicy,
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
    /// Course that owns this assignment.
    pub course_id: CourseId,
    /// Human-facing assignment title.
    pub title: String,
    /// Ordered stable fixed items in the current assignment definition.
    pub items: Vec<AssignmentItemSummary>,
    /// Current random-selection groups with pinned immutable candidates.
    pub selection_groups: Vec<AssignmentSelectionGroupSummary>,
    /// Assignment-owned student-facing disclosure schedule.
    pub disclosure_policy: StudentDisclosurePolicy,
    /// Four independent run policies.
    pub policies: AssignmentActivityRules,
}

/// Canonical answer-free facts shown at the start of an assignment.
///
/// The ordinary student detail and an Instructor's stable-identity Student
/// view use distinct envelopes, but they describe the same landing material.
/// Routes build this projection once from the authoritative definition and
/// then use their role-appropriate envelope constructors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentLandingPresentation {
    /// Student-facing assignment title.
    pub title: String,
    /// Student-facing instructions.
    pub instructions: AssignmentInstructions,
    /// Course scheduling zone used to present delivery facts.
    pub time_zone: IanaTimeZone,
    /// Number of active questions a student receives in one run.
    pub questions_per_run: u32,
    /// Student-visible variation policy.
    pub variation: VariationPolicy,
    /// Student-visible disclosure schedule.
    pub disclosure_policy: StudentDisclosurePolicy,
}

/// Student-safe assignment definition.
///
/// This projection deliberately omits course identities, run and
/// disclosure policy, and other server authority inputs. Student routes use
/// it instead of [`AssignmentSummary`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StudentAssignmentLandingSummary {
    /// Durable assignment identity scoped by the authenticated route.
    pub id: AssignmentId,
    /// Stable typed locator used in application navigation.
    pub reference: AssignmentReference,
    /// Human-facing assignment title.
    pub title: String,
}

/// Whether the Student's currently accepted work is late under resolved policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentLateStatus {
    /// The work is on time, or no due instant applies.
    OnTime,
    /// Work after due remains accepted without a late mark.
    AcceptedLate,
    /// Work after due remains accepted and is marked late.
    MarkedLate,
}

/// Server-resolved Student delivery limits for one authorized detail response.
///
/// These values are projections of the effective policy after group and
/// individual adjustments. They are not editable base-policy authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StudentAssignmentDelivery {
    /// Resolved first instant at which the assignment may be opened.
    pub available_at: Option<ActivityTimestamp>,
    /// Resolved ordinary due instant.
    pub due_at: Option<ActivityTimestamp>,
    /// Resolved hard instant after which new work closes.
    pub closes_at: Option<ActivityTimestamp>,
    /// Resolved whole-run time limit when one applies.
    pub time_limit_seconds: Option<std::num::NonZeroU32>,
    /// Resolved maximum number of runs when one applies.
    pub attempt_limit: Option<std::num::NonZeroU32>,
    /// Resolved treatment of work after the ordinary due instant.
    pub late_submission: LateSubmissionPolicy,
    /// Server behavior at the resolved effective deadline.
    pub deadline_behavior: AssignmentDeadlineBehavior,
    /// Server-owned late condition for the Student's present work.
    pub late_status: StudentLateStatus,
}

/// Student-safe assignment material for the dedicated detail route.
///
/// Paginated Student list rows deliberately omit this potentially large
/// material. The server admits this detail only after the same effective
/// policy gate used to issue a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StudentAssignmentDetail {
    /// Durable assignment identity scoped by the authenticated route.
    pub id: AssignmentId,
    /// Stable typed locator used in application navigation.
    pub reference: AssignmentReference,
    /// Human-facing assignment title.
    pub title: String,
    /// Validated Student-facing plain-text instructions.
    pub instructions: AssignmentInstructions,
    /// Authoritative IANA zone for displaying the server-resolved instants.
    pub time_zone: IanaTimeZone,
    /// Server-resolved delivery limits for this Student.
    pub delivery: StudentAssignmentDelivery,
    /// Ordered stable fixed items in the current assignment definition.
    pub items: Vec<AssignmentItemSummary>,
    /// Current random-selection groups with pinned immutable candidates.
    pub selection_groups: Vec<AssignmentSelectionGroupSummary>,
}

impl From<AssignmentSummary> for StudentAssignmentLandingSummary {
    fn from(assignment: AssignmentSummary) -> Self {
        Self {
            id: assignment.id,
            reference: assignment.reference,
            title: assignment.title,
        }
    }
}

impl StudentAssignmentDetail {
    /// Adds Student identity, resolved delivery, and the question-definition
    /// envelope to the shared answer-free landing presentation.
    pub fn from_landing(
        assignment: AssignmentSummary,
        landing: AssignmentLandingPresentation,
        delivery: StudentAssignmentDelivery,
    ) -> Self {
        Self {
            id: assignment.id,
            reference: assignment.reference,
            title: landing.title,
            instructions: landing.instructions,
            time_zone: landing.time_zone,
            delivery,
            items: assignment.items,
            selection_groups: assignment.selection_groups,
        }
    }
}

/// One compact gradebook row for a Student Record and Assignment.
///
/// The row comes only from the course-owned assignment, Student Record, and
/// `AssignmentProgressRecord` projection. It carries no Assignment Attempt
/// history, so continued practice cannot make the default gradebook slower.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GradebookSummaryRow {
    /// Course whose instructor requested this bounded page.
    pub course_id: CourseId,
    /// Course-owned Student Record represented by this row.
    pub student_record_id: StudentRecordId,
    /// Human-facing Student name from the protected course roster.
    pub student_name: String,
    /// Assignment whose grade policy selected the current score.
    pub assignment_id: AssignmentId,
    /// Human-facing assignment title from the assignment record.
    pub assignment_title: String,
    /// Transactionally maintained compact activity and score projection.
    pub summary: AssignmentProgressRecord,
    /// Current visibility and freshness of assignment scores.
    pub scoring_status: ScoringStatus,
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
            disclosure_policy: StudentDisclosurePolicy::default(),
            policies: AssignmentActivityRules {
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
        assert!(value.get("lifecycle").is_none());
        assert!(value.get("instructions").is_none());

        let student = StudentAssignmentLandingSummary::from(AssignmentSummary {
            id: AssignmentId::from_uuid(Uuid::from_u128(1)),
            reference: crate::AssignmentReference::new(1).expect("valid reference"),
            course_id: CourseId::from_uuid(Uuid::from_u128(3)),
            title: "Peptide bonds".to_string(),
            items: Vec::new(),
            selection_groups: Vec::new(),
            disclosure_policy: StudentDisclosurePolicy::default(),
            policies: AssignmentActivityRules {
                completion: CompletionRequirement::AllCorrect,
                grade: GradePolicy::Highest,
                continued_practice: ContinuedPractice::Unlimited,
                variation: VariationPolicy::NewSeeds,
            },
        });
        let student_value = serde_json::to_value(student).expect("Student serializes");
        assert!(student_value.get("instructions").is_none());
        assert_eq!(student_value["title"], "Peptide bonds");
    }

    #[test]
    fn student_detail_owns_large_material_and_server_resolved_delivery() {
        let assignment = AssignmentSummary {
            id: AssignmentId::from_uuid(Uuid::from_u128(1)),
            reference: crate::AssignmentReference::new(1).expect("valid reference"),
            course_id: CourseId::from_uuid(Uuid::from_u128(3)),
            title: "Peptide bonds".to_string(),
            items: Vec::new(),
            selection_groups: Vec::new(),
            disclosure_policy: StudentDisclosurePolicy::default(),
            policies: AssignmentActivityRules {
                completion: CompletionRequirement::AllCorrect,
                grade: GradePolicy::Highest,
                continued_practice: ContinuedPractice::Unlimited,
                variation: VariationPolicy::NewSeeds,
            },
        };
        let detail = StudentAssignmentDetail::from_landing(
            assignment,
            AssignmentLandingPresentation {
                title: "Peptide bonds".to_string(),
                instructions: AssignmentInstructions::try_new("Read the legend.".to_string())
                    .expect("valid instructions"),
                time_zone: IanaTimeZone::parse("America/Chicago").expect("known zone"),
                questions_per_run: 0,
                variation: VariationPolicy::NewSeeds,
                disclosure_policy: StudentDisclosurePolicy::default(),
            },
            StudentAssignmentDelivery {
                available_at: Some(ActivityTimestamp::from_unix_millis(1_000)),
                due_at: Some(ActivityTimestamp::from_unix_millis(2_000)),
                closes_at: Some(ActivityTimestamp::from_unix_millis(3_000)),
                time_limit_seconds: None,
                attempt_limit: None,
                late_submission: LateSubmissionPolicy::MarkLate,
                deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
                late_status: StudentLateStatus::MarkedLate,
            },
        );
        let value = serde_json::to_value(&detail).expect("detail serializes");
        assert_eq!(value["instructions"], "Read the legend.");
        assert_eq!(value["time_zone"], "America/Chicago");
        assert_eq!(value["delivery"]["late_status"], "marked_late");
        assert!(
            serde_json::from_value::<StudentAssignmentDetail>(serde_json::json!({
                "id": detail.id,
                "reference": detail.reference,
                "title": detail.title,
                "instructions": "Read the legend.",
                "time_zone": "America/Chicago",
                "delivery": value["delivery"],
                "items": [],
                "selection_groups": [],
                "unexpected": true
            }))
            .is_err()
        );
    }

    #[test]
    fn gradebook_summary_row_keeps_the_projection_nested() {
        let row = GradebookSummaryRow {
            course_id: CourseId::from_uuid(Uuid::from_u128(2)),
            student_record_id: StudentRecordId::from_uuid(Uuid::from_u128(3)),
            student_name: "Ada Student".to_string(),
            assignment_id: AssignmentId::from_uuid(Uuid::from_u128(5)),
            assignment_title: "Peptide bonds".to_string(),
            summary: AssignmentProgressRecord::empty(
                StudentRecordId::from_uuid(Uuid::from_u128(3)),
                AssignmentId::from_uuid(Uuid::from_u128(5)),
            ),
            scoring_status: crate::ScoringStatus::Current,
        };

        let value = serde_json::to_value(row).expect("gradebook row should serialize");
        assert!(value.get("course_id").is_some());
        assert!(value.get("assignment_title").is_some());
        assert_eq!(
            value.get("student_name").and_then(|name| name.as_str()),
            Some("Ada Student")
        );
        assert!(value.get("summary").is_some());
        assert_eq!(value["scoring_status"], "current");
        assert!(value.get("best_score").is_none());
    }
}
