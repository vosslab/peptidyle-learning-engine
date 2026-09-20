//! Browser-safe course, assessment, and course-access projections.

use serde::{Deserialize, Serialize};

use crate::{
    AccountTimeZone, AssessmentActivityRules, AssessmentEntryAvailability, AssessmentEntryId,
    AssessmentEntryScoringRule, AssessmentGrade, AssessmentId, AssessmentInstructions,
    AssessmentPointValue, AssessmentProgressRecord, AssessmentQuestionVariationRule,
    AssessmentScoringState, AssessmentTitle, CourseInstanceId, CourseTerm, LateWorkRule,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionBackend, QuestionBackendCapabilities,
    QuestionId, QuestionPoolEditNumber, QuestionPoolSelectionRule, StudentFeedbackReleaseRule,
    StudentRecordId, Timestamp,
};

/// Relationship that may be persisted on one direct course membership.
///
/// This scopes an Account's participation in one Course Instance. It is not
/// another inventory of human Product Roles, and Sysadmin is never a membership
/// value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseMembershipRole {
    /// Works assessments and views only personal educational records.
    Student,
    /// Manages this course and its assessments.
    Instructor,
}

/// Course information sufficient for the signed-in landing page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSummary {
    /// Current Course metadata; members cannot acquire editing authority from it.
    pub classification: crate::CourseClassification,
    /// Course Instance ID (`CIXXXXXXXZ`).
    pub id: CourseInstanceId,
    /// Compact Course Instance name for constrained navigation.
    pub short_name: String,
    /// Descriptive Course Instance name for headings and breadcrumbs.
    pub long_name: String,
    /// Required inclusive term bounds.
    pub term: crate::CourseTerm,
    /// Signed-in Account's Course Membership Role for this Course Instance.
    pub role: CourseMembershipRole,
}

/// Closed Course Instance identity and teaching-period data for course routes.
///
/// This projection deliberately contains no durable Course identity. Route authorization resolves
/// the signed-in Account's membership role before constructing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseInstanceRouteSummary {
    /// Current independently selected Course classification.
    pub classification: crate::CourseClassification,
    /// Course Instance ID used in application navigation.
    pub id: CourseInstanceId,
    /// Compact Course Instance name for constrained navigation.
    pub short_name: String,
    /// Descriptive Course Instance name for headings and breadcrumbs.
    pub long_name: String,
    /// Required inclusive term bounds.
    pub term: CourseTerm,
    /// Signed-in Account's Course Membership Role for this Course Instance.
    pub role: CourseMembershipRole,
}

/// Browser-safe Assessment Content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixedQuestionAssessmentEntrySummary {
    /// Server-minted identity for this editable assessment slot.
    pub id: AssessmentEntryId,
    /// Browser-visible Question ID for the stable Published Question lineage.
    pub question_id: QuestionId,
    /// Safe Question Library label shown while editing this assessment.
    pub question_title: String,
    /// Question Backend selected for this Fixed Question Assessment Entry.
    pub backend: QuestionBackend,
    /// Capabilities declared for the published question.
    pub capabilities: QuestionBackendCapabilities,
    /// Current assessment-authored points.
    pub points_possible: AssessmentPointValue,
    /// Whether future Assessment Attempts may receive this Assessment Entry.
    pub availability: AssessmentEntryAvailability,
    /// Current-only scoring treatment.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Question Attempt retry bound frozen with this Assessment Entry.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Question Attempt timing frozen with this Assessment Entry.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// Browser-safe Question Pool Assessment Entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolAssessmentEntrySummary {
    /// Stable Assessment Entry identity.
    pub id: AssessmentEntryId,
    pub question_pool_id: QuestionId,
    /// Current-state Pool Edit Number; not a historical membership object.
    pub question_pool_edit_number: QuestionPoolEditNumber,
    /// Whether future Assessment Attempts may receive this Assessment Entry.
    pub availability: AssessmentEntryAvailability,
    /// Current-only scoring rule applied to every selected Question Pool Item.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Positive number of Pool members selected for each future Assessment Attempt.
    pub selection_count: std::num::NonZeroU32,
    /// Uniform current points for each selected Question Pool Item.
    pub points_per_item: AssessmentPointValue,
    /// Complete reviewed selection behavior.
    pub selection_rule: QuestionPoolSelectionRule,
    /// Uniform Question Attempt retry bound for every Question selected from this pool.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Uniform Question Attempt timing for every Question selected from this pool.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// Browser-safe Assessment Entry in authored delivery order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssessmentEntrySummary {
    /// One exact fixed Question.
    FixedQuestion(FixedQuestionAssessmentEntrySummary),
    /// One deterministic Question Pool.
    QuestionPool(QuestionPoolAssessmentEntrySummary),
}

/// Browser-safe Assessment Content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentSummary {
    /// Assessment ID (`AXXXXXXXZ`).
    pub id: AssessmentId,
    /// Course that owns this assessment.
    pub course_instance_id: CourseInstanceId,
    /// Human-facing assessment title.
    pub title: AssessmentTitle,
    /// Ordered complete Assessment Content Entry.
    pub entries: Vec<AssessmentEntrySummary>,
    /// Assessment-owned student-facing disclosure schedule.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    /// Six independent Assessment Activity Rules.
    pub policies: AssessmentActivityRules,
}

/// Exact answer-free Assessment Overview facts shown at the start of an assessment.
///
/// The ordinary student detail and an Instructor's stable-identity Student
/// view use distinct response records, but they describe the same Assessment Overview.
/// Routes build this Assessment Overview once from the authoritative Assessment Content and
/// then use their role-appropriate response constructors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentOverview {
    /// Student-facing assessment title.
    pub title: AssessmentTitle,
    /// Student-facing instructions.
    pub instructions: AssessmentInstructions,
    /// Number of active questions a student receives in one Assessment Attempt.
    pub questions_per_assessment_attempt: u32,
    /// Student-visible Question Variation Rule.
    pub question_variation_rule: AssessmentQuestionVariationRule,
    /// Student-visible disclosure schedule.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
}

/// Student-safe Student Assessment Landing Summary.
///
/// This Student Assessment Landing Summary deliberately omits course identities, Assessment Attempt and
/// disclosure policy, and other server authority inputs. Student routes use
/// it instead of [`AssessmentSummary`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StudentAssessmentLandingSummary {
    /// Assessment ID scoped by the authenticated route.
    pub id: AssessmentId,
    /// Human-facing assessment title.
    pub title: AssessmentTitle,
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
pub struct StudentAssessmentDelivery {
    /// Resolved first instant at which the assessment may be opened.
    pub available_at: Option<Timestamp>,
    /// Resolved ordinary due instant.
    pub due_at: Option<Timestamp>,
    /// Resolved hard instant after which new work closes.
    pub closes_at: Option<Timestamp>,
    /// Resolved whole Assessment Attempt time limit when one applies.
    pub assessment_attempt_time_limit_seconds: Option<std::num::NonZeroU32>,
    /// Resolved maximum number of Assessment Attempts when one applies.
    pub attempt_limit: Option<std::num::NonZeroU32>,
    /// Resolved treatment of work after the ordinary due instant.
    pub late_work_rule: LateWorkRule,
    /// Server-owned late condition for the Student's present work.
    pub student_late_work_status: StudentLateWorkStatus,
}

/// Student-safe detail returned by the dedicated Student Assessment Detail route.
///
/// Paginated Student list rows deliberately omit this potentially large
/// detail. The server admits this Student Assessment Detail only after the same effective
/// policy gate used to issue an Assessment Attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StudentAssessmentDetail {
    /// Assessment ID scoped by the authenticated route.
    pub id: AssessmentId,
    /// Human-facing assessment title.
    pub title: AssessmentTitle,
    /// Validated Student-facing plain-text instructions.
    pub instructions: AssessmentInstructions,
    /// Authenticated Student's IANA zone for displaying the server-resolved instants.
    pub display_time_zone: AccountTimeZone,
    /// Server-resolved delivery limits for this Student.
    pub delivery: StudentAssessmentDelivery,
    /// Ordered complete Assessment Content Entry.
    pub entries: Vec<AssessmentEntrySummary>,
}

impl From<AssessmentSummary> for StudentAssessmentLandingSummary {
    fn from(assessment: AssessmentSummary) -> Self {
        Self {
            id: assessment.id,
            title: assessment.title,
        }
    }
}

impl StudentAssessmentDetail {
    /// Adds Student identity, resolved delivery, and the Question Content
    /// Question Content to the shared answer-free landing presentation.
    pub fn from_landing(
        assessment: AssessmentSummary,
        landing: AssessmentOverview,
        delivery: StudentAssessmentDelivery,
        display_time_zone: AccountTimeZone,
    ) -> Self {
        Self {
            id: assessment.id,
            title: landing.title,
            instructions: landing.instructions,
            display_time_zone,
            delivery,
            entries: assessment.entries,
        }
    }
}

/// One compact gradebook row for a Student Record and Assessment.
///
/// The row comes only from the course-owned assessment, Student Record,
/// `AssessmentGrade`, and `AssessmentProgressRecord`. It carries no Assessment Attempt
/// history, so continued practice cannot make the default gradebook slower.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GradebookSummaryRow {
    /// Course whose instructor requested this bounded page.
    pub course_instance_id: CourseInstanceId,
    /// Course-owned Student Record represented by this row.
    pub student_record_id: StudentRecordId,
    /// Human-facing Student name from the protected course roster.
    pub student_name: String,
    /// Assessment whose grade policy selected the current score.
    pub assessment_id: AssessmentId,
    /// Human-facing assessment title from the assessment record.
    pub assessment_title: AssessmentTitle,
    /// Transactionally maintained compact Assessment Grade.
    pub assessment_grade: AssessmentGrade,
    /// Transactionally maintained compact Assessment Progress Record.
    pub assessment_progress: AssessmentProgressRecord,
    /// Current visibility and freshness of assessment scores.
    pub assessment_scoring_state: AssessmentScoringState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AssessmentQuestionVariationRule;
    use uuid::Uuid;

    fn assessment_title(value: &str) -> AssessmentTitle {
        AssessmentTitle::try_new(value.to_string()).expect("valid Assessment Title fixture")
    }

    #[test]
    fn rust_names_serialize_as_lower_camel_course_contracts() {
        let assessment = AssessmentSummary {
            id: AssessmentId::from_debug_serial(1),

            course_instance_id: CourseInstanceId::from_debug_serial(3),
            title: assessment_title("Peptide bonds"),
            entries: vec![AssessmentEntrySummary::FixedQuestion(
                FixedQuestionAssessmentEntrySummary {
                    id: crate::AssessmentEntryId::from_uuid(Uuid::from_u128(4)),
                    question_id: "7K3M-19QX".parse().expect("fixture Question ID parses"),
                    question_title: "Peptide bonds".to_string(),
                    backend: crate::QuestionBackend::Ple,
                    capabilities: crate::QuestionBackendCapabilities::none(),
                    points_possible: crate::AssessmentPointValue::from_whole(1),
                    availability: crate::AssessmentEntryAvailability::Available,
                    scoring_rule: crate::AssessmentEntryScoringRule::Normal,
                    question_attempt_limit: crate::QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: crate::QuestionAttemptTimeLimit::Unlimited,
                },
            )],
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            policies: AssessmentActivityRules {
                question_variation_rule: AssessmentQuestionVariationRule::NewVariation,
                ..AssessmentActivityRules::default()
            },
        };

        let value = serde_json::to_value(assessment).expect("assessment should serialize");
        assert!(value.get("courseInstanceId").is_some());
        assert!(value.get("courseId").is_none());
        assert!(value.get("course_id").is_none());
        let item = &value["entries"][0];
        assert_eq!(item["questionId"], "7K3M-19QX");
        assert!(item.get("reference").is_none());
        assert!(value.get("lifecycle").is_none());
        assert!(value.get("instructions").is_none());

        let student = StudentAssessmentLandingSummary::from(AssessmentSummary {
            id: AssessmentId::from_debug_serial(1),

            course_instance_id: CourseInstanceId::from_debug_serial(3),
            title: assessment_title("Peptide bonds"),
            entries: Vec::new(),
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            policies: AssessmentActivityRules {
                question_variation_rule: AssessmentQuestionVariationRule::NewVariation,
                ..AssessmentActivityRules::default()
            },
        });
        let student_value = serde_json::to_value(student).expect("Student serializes");
        assert!(student_value.get("instructions").is_none());
        assert_eq!(student_value["title"], "Peptide bonds");
    }

    #[test]
    fn student_detail_owns_instructions_and_server_resolved_delivery() {
        let assessment = AssessmentSummary {
            id: AssessmentId::from_debug_serial(1),

            course_instance_id: CourseInstanceId::from_debug_serial(3),
            title: assessment_title("Peptide bonds"),
            entries: Vec::new(),
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            policies: AssessmentActivityRules {
                question_variation_rule: AssessmentQuestionVariationRule::NewVariation,
                ..AssessmentActivityRules::default()
            },
        };
        let detail = StudentAssessmentDetail::from_landing(
            assessment,
            AssessmentOverview {
                title: assessment_title("Peptide bonds"),
                instructions: AssessmentInstructions::try_new("Read the legend.".to_string())
                    .expect("valid instructions"),
                questions_per_assessment_attempt: 0,
                question_variation_rule: AssessmentQuestionVariationRule::NewVariation,
                student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            },
            StudentAssessmentDelivery {
                available_at: Some(Timestamp::from_unix_millis(1_000)),
                due_at: Some(Timestamp::from_unix_millis(2_000)),
                closes_at: Some(Timestamp::from_unix_millis(3_000)),
                assessment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::MarkLate,
                student_late_work_status: StudentLateWorkStatus::MarkedLate,
            },
            AccountTimeZone::parse("America/New_York").expect("known zone"),
        );
        let value = serde_json::to_value(&detail).expect("detail serializes");
        assert_eq!(value["instructions"], "Read the legend.");
        assert_eq!(value["display_time_zone"], "America/New_York");
        assert_eq!(value["delivery"]["student_late_work_status"], "marked_late");
        assert!(
            serde_json::from_value::<StudentAssessmentDetail>(serde_json::json!({
                "id": detail.id,
                "title": detail.title,
                "instructions": "Read the legend.",
                "display_time_zone": "America/New_York",
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
            course_instance_id: CourseInstanceId::from_debug_serial(2),
            student_record_id: StudentRecordId::from_uuid(Uuid::from_u128(3)),
            student_name: "Ada Student".to_string(),
            assessment_id: AssessmentId::from_debug_serial(5),
            assessment_title: assessment_title("Peptide bonds"),
            assessment_grade: AssessmentGrade::empty(
                StudentRecordId::from_uuid(Uuid::from_u128(3)),
                AssessmentId::from_debug_serial(5),
            ),
            assessment_progress: AssessmentProgressRecord::empty(
                StudentRecordId::from_uuid(Uuid::from_u128(3)),
                AssessmentId::from_debug_serial(5),
            ),
            assessment_scoring_state: crate::AssessmentScoringState::Current,
        };

        let value = serde_json::to_value(row).expect("gradebook row should serialize");
        assert!(value.get("course_instance_id").is_some());
        assert!(value.get("course_id").is_none());
        assert!(value.get("assessment_title").is_some());
        assert_eq!(
            value.get("student_name").and_then(|name| name.as_str()),
            Some("Ada Student")
        );
        assert!(value.get("assessment_grade").is_some());
        assert!(value.get("assessment_progress").is_some());
        assert_eq!(value["assessment_scoring_state"], "current");
        assert!(value.get("best_score").is_none());
    }
}
