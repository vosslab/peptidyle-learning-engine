//! Browser-safe course, assignment, and course-access projections.

use serde::{Deserialize, Serialize};

use crate::{
    ActivityTimestamp, AssignmentActivityRules, AssignmentDeadlineRule,
    AssignmentEntryAvailability, AssignmentEntryId, AssignmentEntryScoringRule, AssignmentId,
    AssignmentInstructions, AssignmentPointValue, AssignmentProgressRecord, AssignmentReference,
    AssignmentScoringState, AssignmentTitle, CourseId, CourseInstanceReference, CourseTimeZone,
    LateWorkRule, QuestionBackend, QuestionBackendCapabilities, QuestionId,
    QuestionPoolItemAvailability, QuestionPoolItemId, QuestionPoolSelectionRule,
    QuestionVariationRule, StudentFeedbackReleaseRule, StudentRecordId,
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
    pub reference: CourseInstanceReference,
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
pub struct FixedQuestionAssignmentEntrySummary {
    /// Server-minted identity for this editable assignment slot.
    pub id: AssignmentEntryId,
    /// Sole browser-visible locator for the immutable published question.
    pub question_id: QuestionId,
    /// Safe Question Library label shown while editing this assignment.
    pub title: String,
    /// Question Backend selected for the item.
    pub backend: QuestionBackend,
    /// Capabilities declared for the published question.
    pub capabilities: QuestionBackendCapabilities,
    /// Current assignment-authored points.
    pub points_possible: AssignmentPointValue,
    /// Whether future Assignment Attempts may receive this Assignment Entry.
    pub availability: AssignmentEntryAvailability,
    /// Current-only scoring treatment.
    pub scoring_rule: AssignmentEntryScoringRule,
}

/// Browser-safe Question Pool Item in one Question Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolItemSummary {
    /// Server-minted identity for this editable Question Pool Item.
    pub id: QuestionPoolItemId,
    /// Sole browser-visible locator for the immutable published question.
    pub question_id: QuestionId,
    /// Safe Question Library label shown while editing this assignment.
    pub title: String,
    /// Question Backend selected for this Question Pool Item.
    pub backend: QuestionBackend,
    /// Capabilities declared for the published question.
    pub capabilities: QuestionBackendCapabilities,
    /// Whether future Question Pool Selections may select this Question Pool Item.
    pub availability: QuestionPoolItemAvailability,
}

/// Browser-safe Question Pool Assignment Entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolAssignmentEntrySummary {
    /// Stable Assignment Entry identity.
    pub id: AssignmentEntryId,
    /// Whether future Assignment Attempts may receive this Assignment Entry.
    pub availability: AssignmentEntryAvailability,
    /// Current-only scoring rule applied to every selected Question Pool Item.
    pub scoring_rule: AssignmentEntryScoringRule,
    /// Number of available entries selected for each future Assignment Attempt.
    pub selection_count: u32,
    /// Uniform current points for each selected Question Pool Item.
    pub points_per_item: AssignmentPointValue,
    /// Complete reviewed selection behavior.
    pub selection_rule: QuestionPoolSelectionRule,
    /// Browser-safe current Question Pool Items.
    pub items: Vec<QuestionPoolItemSummary>,
}

/// Browser-safe Assignment Entry in authored delivery order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssignmentEntrySummary {
    /// One exact fixed Question.
    FixedQuestion(FixedQuestionAssignmentEntrySummary),
    /// One deterministic Question Pool.
    QuestionPool(QuestionPoolAssignmentEntrySummary),
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
    pub title: AssignmentTitle,
    /// Ordered complete Assignment Entry definition.
    pub entries: Vec<AssignmentEntrySummary>,
    /// Assignment-owned student-facing disclosure schedule.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    /// Nine independent Assignment Activity Rules.
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
pub struct AssignmentOverview {
    /// Student-facing assignment title.
    pub title: AssignmentTitle,
    /// Student-facing instructions.
    pub instructions: AssignmentInstructions,
    /// Course scheduling zone used to present delivery facts.
    pub time_zone: CourseTimeZone,
    /// Number of active questions a student receives in one run.
    pub questions_per_run: u32,
    /// Student-visible Question Pool Reuse Rule.
    pub question_pool_reuse_rule: crate::QuestionPoolReuseRule,
    /// Student-visible Question Variation Rule.
    pub question_variation_rule: QuestionVariationRule,
    /// Student-visible disclosure schedule.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
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
    pub title: AssignmentTitle,
}

/// Whether the Student's currently accepted work is late under resolved policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentLateWorkStatus {
    /// The work is on time, or no due instant applies.
    OnTime,
    /// Work after due remains accepted without a late mark.
    AcceptedLate,
    /// Work after due remains accepted and is marked late.
    MarkedLate,
}

/// Server-resolved Student delivery limits for one authorized detail response.
///
/// These values are projections of the effective Course policy after direct
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
    pub assignment_attempt_time_limit_seconds: Option<std::num::NonZeroU32>,
    /// Resolved maximum number of runs when one applies.
    pub attempt_limit: Option<std::num::NonZeroU32>,
    /// Resolved treatment of work after the ordinary due instant.
    pub late_work_rule: LateWorkRule,
    /// Server behavior at the resolved effective deadline.
    pub assignment_deadline_rule: AssignmentDeadlineRule,
    /// Server-owned late condition for the Student's present work.
    pub late_status: StudentLateWorkStatus,
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
    pub title: AssignmentTitle,
    /// Validated Student-facing plain-text instructions.
    pub instructions: AssignmentInstructions,
    /// Authoritative IANA zone for displaying the server-resolved instants.
    pub time_zone: CourseTimeZone,
    /// Server-resolved delivery limits for this Student.
    pub delivery: StudentAssignmentDelivery,
    /// Ordered complete Assignment Entry definition.
    pub entries: Vec<AssignmentEntrySummary>,
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
        landing: AssignmentOverview,
        delivery: StudentAssignmentDelivery,
    ) -> Self {
        Self {
            id: assignment.id,
            reference: assignment.reference,
            title: landing.title,
            instructions: landing.instructions,
            time_zone: landing.time_zone,
            delivery,
            entries: assignment.entries,
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
    pub assignment_title: AssignmentTitle,
    /// Transactionally maintained compact activity and score projection.
    pub summary: AssignmentProgressRecord,
    /// Current visibility and freshness of assignment scores.
    pub assignment_scoring_state: AssignmentScoringState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssignmentAttemptContinuationRule, AssignmentAttemptGradeRule, AssignmentCompletionRule,
        QuestionVariationRule,
    };
    use uuid::Uuid;

    fn assignment_title(value: &str) -> AssignmentTitle {
        AssignmentTitle::try_new(value.to_string()).expect("valid Assignment Title fixture")
    }

    #[test]
    fn rust_names_serialize_as_lower_camel_course_contracts() {
        let assignment = AssignmentSummary {
            id: AssignmentId::from_uuid(Uuid::from_u128(1)),
            reference: crate::AssignmentReference::new(1).expect("valid reference"),
            course_id: CourseId::from_uuid(Uuid::from_u128(3)),
            title: assignment_title("Peptide bonds"),
            entries: vec![AssignmentEntrySummary::FixedQuestion(
                FixedQuestionAssignmentEntrySummary {
                    id: crate::AssignmentEntryId::from_uuid(Uuid::from_u128(4)),
                    question_id: "7K3-M9QX".parse().expect("fixture Question ID parses"),
                    title: "Peptide bonds".to_string(),
                    backend: crate::QuestionBackend::Ple,
                    capabilities: crate::QuestionBackendCapabilities::none(),
                    points_possible: crate::AssignmentPointValue::from_whole(1),
                    availability: crate::AssignmentEntryAvailability::Available,
                    scoring_rule: crate::AssignmentEntryScoringRule::Normal,
                },
            )],
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            policies: AssignmentActivityRules {
                assignment_completion_rule: AssignmentCompletionRule::AllCorrect,
                assignment_attempt_grade_rule: AssignmentAttemptGradeRule::Highest,
                assignment_attempt_continuation_rule: AssignmentAttemptContinuationRule::Unlimited,
                question_pool_reuse_rule: crate::QuestionPoolReuseRule::ReuseSelection,
                question_variation_rule: QuestionVariationRule::NewVariation,
                ..AssignmentActivityRules::default()
            },
        };

        let value = serde_json::to_value(assignment).expect("assignment should serialize");
        assert!(value.get("courseId").is_some());
        assert!(value.get("course_id").is_none());
        let item = &value["entries"][0];
        assert_eq!(item["questionId"], "7K3-M9QX");
        assert!(item.get("reference").is_none());
        assert!(value.get("lifecycle").is_none());
        assert!(value.get("instructions").is_none());

        let student = StudentAssignmentLandingSummary::from(AssignmentSummary {
            id: AssignmentId::from_uuid(Uuid::from_u128(1)),
            reference: crate::AssignmentReference::new(1).expect("valid reference"),
            course_id: CourseId::from_uuid(Uuid::from_u128(3)),
            title: assignment_title("Peptide bonds"),
            entries: Vec::new(),
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            policies: AssignmentActivityRules {
                assignment_completion_rule: AssignmentCompletionRule::AllCorrect,
                assignment_attempt_grade_rule: AssignmentAttemptGradeRule::Highest,
                assignment_attempt_continuation_rule: AssignmentAttemptContinuationRule::Unlimited,
                question_pool_reuse_rule: crate::QuestionPoolReuseRule::ReuseSelection,
                question_variation_rule: QuestionVariationRule::NewVariation,
                ..AssignmentActivityRules::default()
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
            title: assignment_title("Peptide bonds"),
            entries: Vec::new(),
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            policies: AssignmentActivityRules {
                assignment_completion_rule: AssignmentCompletionRule::AllCorrect,
                assignment_attempt_grade_rule: AssignmentAttemptGradeRule::Highest,
                assignment_attempt_continuation_rule: AssignmentAttemptContinuationRule::Unlimited,
                question_pool_reuse_rule: crate::QuestionPoolReuseRule::ReuseSelection,
                question_variation_rule: QuestionVariationRule::NewVariation,
                ..AssignmentActivityRules::default()
            },
        };
        let detail = StudentAssignmentDetail::from_landing(
            assignment,
            AssignmentOverview {
                title: assignment_title("Peptide bonds"),
                instructions: AssignmentInstructions::try_new("Read the legend.".to_string())
                    .expect("valid instructions"),
                time_zone: CourseTimeZone::parse("America/Chicago").expect("known zone"),
                questions_per_run: 0,
                question_pool_reuse_rule: crate::QuestionPoolReuseRule::ReuseSelection,
                question_variation_rule: QuestionVariationRule::NewVariation,
                student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            },
            StudentAssignmentDelivery {
                available_at: Some(ActivityTimestamp::from_unix_millis(1_000)),
                due_at: Some(ActivityTimestamp::from_unix_millis(2_000)),
                closes_at: Some(ActivityTimestamp::from_unix_millis(3_000)),
                assignment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::MarkLate,
                assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
                late_status: StudentLateWorkStatus::MarkedLate,
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
                "entries": [],
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
            assignment_title: assignment_title("Peptide bonds"),
            summary: AssignmentProgressRecord::empty(
                StudentRecordId::from_uuid(Uuid::from_u128(3)),
                AssignmentId::from_uuid(Uuid::from_u128(5)),
            ),
            assignment_scoring_state: crate::AssignmentScoringState::Current,
        };

        let value = serde_json::to_value(row).expect("gradebook row should serialize");
        assert!(value.get("course_id").is_some());
        assert!(value.get("assignment_title").is_some());
        assert_eq!(
            value.get("student_name").and_then(|name| name.as_str()),
            Some("Ada Student")
        );
        assert!(value.get("summary").is_some());
        assert_eq!(value["assignment_scoring_state"], "current");
        assert!(value.get("best_score").is_none());
    }
}
