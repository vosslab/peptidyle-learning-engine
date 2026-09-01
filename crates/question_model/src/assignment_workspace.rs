//! Strict browser contracts for the Instructor assignment workspace.
//!
//! These types describe request intent and publication validation only. The
//! server resolves question references, course-local times, and authority
//! before it changes the authoritative assignment aggregate.

use serde::{Deserialize, Serialize};

use crate::curriculum_adoption::AssignmentRevisionReference;
use crate::{
    AssignmentActivityRules, AssignmentDeadlineRule, AssignmentEditNumber, AssignmentEntry,
    AssignmentEntryAvailability, AssignmentEntryScoringRule, AssignmentOverview,
    AssignmentPointValue, AssignmentStatus, AssignmentTitle, Capability, CourseTimeZone,
    InstructorAssignmentWorkingCopyDefinitionLocal, LateWorkRule, QuestionId,
    QuestionPoolItemAvailability, QuestionPoolReuseRule, QuestionPoolSelectionRule,
    QuestionVariationRule, StudentFeedbackReleaseRule,
};

/// Browser request to create one stable Assignment and its first working copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAssignmentRequest {
    /// Human-facing title for the new assignment.
    pub title: AssignmentTitle,
}

/// Browser request that replaces the Questions-owned assignment content slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceAssignmentContentRequest {
    /// Exact Assignment Working Copy edit number reviewed before editing.
    pub base_edit_number: AssignmentEditNumber,
    /// Human-facing title, owned by the Questions workspace.
    pub title: AssignmentTitle,
    /// Ordered fixed questions and Question Pools for future Assignment Attempts.
    pub entries: Vec<AssignmentEntryRequest>,
}

/// Browser request to replace the immutable publication of one existing fixed
/// assignment slot. The server derives the slot, assignment, course, Account,
/// and publication version; this request names only the public Question ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceAssignmentFixedItemRequest {
    /// Exact Assignment Working Copy edit number reviewed before editing.
    pub base_edit_number: AssignmentEditNumber,
    /// Public Question ID resolved to an assignable immutable publication by
    /// the authenticated server.
    pub question_id: QuestionId,
}

/// Browser request that replaces the Policies-owned assignment slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceAssignmentPoliciesRequest {
    /// Exact Assignment Working Copy edit number reviewed before editing.
    pub base_edit_number: AssignmentEditNumber,
    /// Student-facing disclosure timing.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    /// Completion, grade, practice, and Question Variation Rule.
    pub policies: AssignmentActivityRules,
    /// Course-local Assignment Working Copy definition resolved by the server before storage.
    pub assignment_working_copy_definition: InstructorAssignmentWorkingCopyDefinitionLocal,
}

/// Browser-safe refusal returned when the Policies workspace cannot save its
/// complete aggregate update. Once the server can build a valid teaching-state
/// proposed teaching state, it returns every independently determinable correction in stable
/// order before persistence replaces the Assignment Working Copy. A malformed
/// teaching state is returned alone because it
/// prevents constructing that proposed teaching state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentPoliciesValidationFailure {
    pub error: AssignmentPoliciesValidationFailureCode,
    pub issues: Vec<AssignmentPoliciesValidationIssue>,
}

/// Closed discriminator for a Policies workspace validation refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentPoliciesValidationFailureCode {
    AssignmentPoliciesInvalid,
}

/// One browser-safe Policies correction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AssignmentPoliciesValidationIssue {
    /// A course-local Assignment Working Copy definition needs the supplied correction.
    AssignmentWorkingCopyDefinition {
        correction: crate::AssignmentWorkingCopyDefinitionValidationFailure,
    },
    /// A selected question backend cannot satisfy one required capability.
    Capability {
        title: String,
        question_id: QuestionId,
        capability: Capability,
    },
    /// The exact Assignment Working Copy has release blockers.
    AssignmentReleaseRequirements {
        blocking_issues: Vec<AssignmentReleaseIssue>,
    },
}

/// Closed structural-content refusal that requires a successor Draft Assignment
/// Revision. Ordinary `409` responses still cover retryable aggregate conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuccessorAssignmentRevisionRequired {
    /// Immutable revision whose existing Student work must remain unchanged.
    pub base_revision: AssignmentRevisionReference,
}

/// One ordered browser content entry. The server resolves every `question_id`
/// to an immutable publication before it builds a Store command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AssignmentEntryRequest {
    /// One fixed question at its place in the ordered future-run definition.
    FixedQuestion {
        question_id: QuestionId,
        points_possible: AssignmentPointValue,
        availability: AssignmentEntryAvailability,
        scoring_rule: AssignmentEntryScoringRule,
    },
    /// A server-resolved selection from a pool of immutable questions.
    QuestionPool {
        question_ids: Vec<QuestionId>,
        availability: AssignmentEntryAvailability,
        scoring_rule: AssignmentEntryScoringRule,
        selection_count: u32,
        points_per_item: AssignmentPointValue,
        selection_rule: QuestionPoolSelectionRule,
    },
}

/// One closed reason that prevents releasing the current Assignment Working Copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AssignmentReleaseIssue {
    /// Questions owns the missing active-deliverable correction.
    QuestionsRequired,
}

/// Server-derived release validation for the current Assignment Working Copy.
///
/// An empty issue list means the definition has the currently known minimum
/// conditions for publication. This is intentionally a projection rather than
/// a second persisted state, so future closed blockers can extend it without
/// changing the assignment aggregate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentReleaseValidation {
    /// Closed, actionable blockers in deterministic order.
    pub blocking_issues: Vec<AssignmentReleaseIssue>,
}

/// Answer-free, non-mutating student landing projection for an Instructor's
/// stable-identity Student view.  It deliberately omits assignment, item,
/// question, run, and attempt identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorStudentView {
    /// Student-facing assignment title.
    pub title: AssignmentTitle,
    /// Student-facing instructions.
    pub instructions: crate::AssignmentInstructions,
    /// Course scheduling zone used to present the delivery facts.
    pub time_zone: CourseTimeZone,
    /// Server-derived base delivery facts, without student progress or actions.
    pub delivery: InstructorStudentViewDelivery,
    /// Number of questions a student receives in one run; derived by the server.
    pub questions_per_run: u32,
    /// Student-visible Question Pool Reuse Rule.
    pub question_pool_reuse_rule: QuestionPoolReuseRule,
    /// Student-visible Question Variation Rule.
    pub question_variation_rule: QuestionVariationRule,
    /// Student-visible disclosure schedule.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
}

/// Instructor-base delivery facts for stable-identity Student view. These
/// facts describe assignment policy, never a particular student's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorStudentViewDelivery {
    pub available_at: Option<crate::ActivityTimestamp>,
    pub due_at: Option<crate::ActivityTimestamp>,
    pub closes_at: Option<crate::ActivityTimestamp>,
    pub assignment_attempt_time_limit_seconds: Option<u32>,
    pub attempt_limit: Option<u32>,
    pub late_work_rule: LateWorkRule,
    pub assignment_deadline_rule: AssignmentDeadlineRule,
}

impl InstructorStudentView {
    /// Adds the Instructor Student-view delivery envelope to the shared
    /// answer-free assignment landing presentation.
    pub fn from_landing(
        landing: AssignmentOverview,
        delivery: InstructorStudentViewDelivery,
    ) -> Self {
        Self {
            title: landing.title,
            instructions: landing.instructions,
            time_zone: landing.time_zone,
            delivery,
            questions_per_run: landing.questions_per_run,
            question_pool_reuse_rule: landing.question_pool_reuse_rule,
            question_variation_rule: landing.question_variation_rule,
            student_feedback_release_rule: landing.student_feedback_release_rule,
        }
    }
}

impl AssignmentReleaseValidation {
    /// Derives readiness from the current definition without mutating it.
    pub fn from_entries(entries: &[AssignmentEntry]) -> Self {
        let has_available_fixed_question = entries.iter().any(|entry| {
            matches!(entry, AssignmentEntry::FixedQuestion(question)
                if question.availability == AssignmentEntryAvailability::Available)
        });
        let has_deliverable_question_pool = entries.iter().any(|entry| match entry {
            AssignmentEntry::QuestionPool(pool) => {
                pool.availability == AssignmentEntryAvailability::Available
                    && pool.selection_count > 0
                    && pool
                        .items
                        .iter()
                        .filter(|entry| {
                            entry.availability == QuestionPoolItemAvailability::Available
                        })
                        .count()
                        >= usize::try_from(pool.selection_count).unwrap_or(usize::MAX)
            }
            AssignmentEntry::FixedQuestion(_) => false,
        });
        let blocking_issues = (!has_available_fixed_question && !has_deliverable_question_pool)
            .then_some(AssignmentReleaseIssue::QuestionsRequired)
            .into_iter()
            .collect();
        Self { blocking_issues }
    }

    /// Returns whether no current release blocker remains.
    pub fn is_ready(&self) -> bool {
        self.blocking_issues.is_empty()
    }

    /// Returns whether this stable Assignment status is valid for the current
    /// Working Copy and its released teaching history.
    pub fn permits_status(&self, status: AssignmentStatus, has_revision: bool) -> bool {
        match status {
            AssignmentStatus::Unreleased | AssignmentStatus::Archived => true,
            AssignmentStatus::Released => self.is_ready(),
            AssignmentStatus::Closed => has_revision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_definition_names_the_questions_blocker() {
        let readiness = AssignmentReleaseValidation::from_entries(&[]);

        assert_eq!(
            readiness.blocking_issues,
            vec![AssignmentReleaseIssue::QuestionsRequired]
        );
        assert!(readiness.permits_status(AssignmentStatus::Unreleased, false));
        assert!(readiness.permits_status(AssignmentStatus::Archived, false));
        assert!(!readiness.permits_status(AssignmentStatus::Closed, false));
        assert!(!readiness.permits_status(AssignmentStatus::Released, false));
    }

    #[test]
    fn browser_requests_reject_unknown_members() {
        let result = serde_json::from_str::<CreateAssignmentRequest>(
            r#"{"title":"Protein folding","ignored":true}"#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn create_assignment_request_decodes_title() {
        let request =
            serde_json::from_str::<CreateAssignmentRequest>(r#"{"title":"Protein folding"}"#)
                .expect("strict create request");

        assert_eq!(request.title.as_str(), "Protein folding");
    }

    #[test]
    fn policies_validation_failure_is_a_closed_browser_contract() {
        let failure = AssignmentPoliciesValidationFailure {
            error: AssignmentPoliciesValidationFailureCode::AssignmentPoliciesInvalid,
            issues: vec![
                AssignmentPoliciesValidationIssue::AssignmentReleaseRequirements {
                    blocking_issues: vec![AssignmentReleaseIssue::QuestionsRequired],
                },
            ],
        };

        let value = serde_json::to_value(&failure).expect("policy validation serializes");
        assert_eq!(
            value,
            serde_json::json!({
                "error": "assignmentPoliciesInvalid",
                "issues": [
                    {
                        "kind": "assignmentReleaseRequirements",
                        "blockingIssues": [{"kind": "questionsRequired"}]
                    }
                ]
            })
        );
        let mut unknown = value;
        unknown["unexpected"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<AssignmentPoliciesValidationFailure>(unknown).is_err());
    }

    #[test]
    fn successor_assignment_revision_requirement_is_a_closed_browser_contract() {
        let requirement = SuccessorAssignmentRevisionRequired {
            base_revision: serde_json::from_str(r#"{"assignment":"A-1","revision_number":"1"}"#)
                .expect("exact Assignment Revision Reference"),
        };

        let value = serde_json::to_value(requirement).expect("successor requirement serializes");
        assert_eq!(
            value,
            serde_json::json!({ "baseRevision": { "assignment": "A-1", "revision_number": "1" } })
        );
        assert!(serde_json::from_value::<SuccessorAssignmentRevisionRequired>(value).is_ok());
        assert!(
            serde_json::from_value::<SuccessorAssignmentRevisionRequired>(
                serde_json::json!({ "baseRevision": { "assignment": "A-1", "revision_number": "1" }, "extra": true })
            )
            .is_err()
        );
    }

    #[test]
    fn content_and_policy_requests_use_closed_camel_case_contracts() {
        let content = serde_json::from_str::<ReplaceAssignmentContentRequest>(
            r#"{"baseEditNumber":"1","title":"Protein folding","entries":[{"kind":"questionPool","questionIds":["7K3-M9QP"],"availability":"available","scoringRule":"normal","selectionCount":1,"pointsPerItem":"1","selectionRule":{"selectedQuestionOrder":"questionPoolOrder"}}]}"#,
        );
        assert!(content.is_ok());
        assert!(
            serde_json::from_str::<ReplaceAssignmentContentRequest>(
                r#"{"title":"Protein folding","entries":[{"kind":"questionPool","questionIds":["7K3-M9QP"],"selectionCount":1,"pointsPerItem":"1","selectedQuestionOrder":"questionPoolOrder"}]}"#,
            )
            .is_err()
        );

        let policy = ReplaceAssignmentPoliciesRequest {
            base_edit_number: "1".parse().expect("edit number"),
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            policies: AssignmentActivityRules {
                assignment_completion_rule: crate::AssignmentCompletionRule::AnswerAll,
                assignment_attempt_grade_rule: crate::AssignmentAttemptGradeRule::Highest,
                assignment_attempt_continuation_rule:
                    crate::AssignmentAttemptContinuationRule::Unlimited,
                question_pool_reuse_rule: crate::QuestionPoolReuseRule::ReuseSelection,
                question_variation_rule: QuestionVariationRule::NewVariation,
                ..AssignmentActivityRules::default()
            },
            assignment_working_copy_definition:
                InstructorAssignmentWorkingCopyDefinitionLocal::new(
                    "America/Chicago".parse().expect("IANA zone"),
                    crate::AssignmentInstructions::default(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    crate::LateWorkRule::Accept,
                    crate::AssignmentDeadlineRule::AutoSubmit,
                )
                .expect("working copy settings"),
        };
        let mut value = serde_json::to_value(&policy).expect("policy request serialization");
        let record = value.as_object().expect("policy request object");
        assert!(record.contains_key("baseEditNumber"));
        assert!(record.contains_key("studentFeedbackReleaseRule"));
        assert!(record.contains_key("assignmentWorkingCopyDefinition"));
        assert_eq!(
            serde_json::from_value::<ReplaceAssignmentPoliciesRequest>(value.clone())
                .expect("policy request roundtrip"),
            policy
        );
        value
            .as_object_mut()
            .expect("policy request object")
            .insert("extra".to_string(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<ReplaceAssignmentPoliciesRequest>(value).is_err());
    }
}
