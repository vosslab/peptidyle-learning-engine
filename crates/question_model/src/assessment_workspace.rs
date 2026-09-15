//! Strict browser contracts for the Instructor assessment workspace.
//!
//! These types describe request intent and publication validation only. The
//! server resolves question references, Account-local times, and authority
//! before it changes the authoritative current Assessment.

use serde::{Deserialize, Serialize};

use crate::{
    AssessmentActivityRules, AssessmentEditNumber, AssessmentEntry, AssessmentEntryAvailability,
    AssessmentEntryScoringRule, AssessmentPointValue, AssessmentStatus, AssessmentTitle,
    Capability, InstructorAssessmentAuthoredContentLocal, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionId, QuestionPoolSelectionRule, StudentFeedbackReleaseRule,
};

/// Browser request to create one stable Assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAssessmentRequest {
    /// Human-facing title for the new assessment.
    pub title: AssessmentTitle,
}

/// Browser request that replaces the Questions-owned assessment content slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceAssessmentContentRequest {
    /// Exact Assessment edit number reviewed before editing.
    pub base_edit_number: AssessmentEditNumber,
    /// Human-facing title, owned by the Questions workspace.
    pub title: AssessmentTitle,
    /// Ordered fixed questions and Question Pools for future Assessment Attempts.
    pub entries: Vec<AssessmentEntryRequest>,
}

/// Browser request that replaces the Policies-owned assessment slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceAssessmentPoliciesRequest {
    /// Exact Assessment edit number reviewed before editing.
    pub base_edit_number: AssessmentEditNumber,
    /// Student-facing disclosure timing.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    /// Completion, grade, practice, and Question Variation Rule.
    pub policies: AssessmentActivityRules,
    /// Account-local Assessment-authored content resolved by the server before storage.
    pub assessment_authored_content: InstructorAssessmentAuthoredContentLocal,
}

/// Browser-safe refusal returned when the Policies workspace cannot save its
/// complete aggregate update. Once the server can build a valid teaching-state
/// proposed teaching state, it returns every independently determinable correction in stable
/// order before persistence replaces the Assessment. A malformed
/// teaching state is returned alone because it
/// prevents constructing that proposed teaching state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentPoliciesValidationFailure {
    pub error: AssessmentPoliciesValidationFailureCode,
    pub issues: Vec<AssessmentPoliciesValidationIssue>,
}

/// Closed discriminator for a Policies workspace validation refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentPoliciesValidationFailureCode {
    AssessmentPoliciesInvalid,
}

/// One browser-safe Policies correction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AssessmentPoliciesValidationIssue {
    /// Account-local Assessment-authored content needs the supplied correction.
    AssessmentAuthoredContent {
        correction: crate::AssessmentAuthoredContentValidationFailure,
    },
    /// A selected question backend cannot satisfy one required capability.
    Capability {
        question_title: String,
        question_id: QuestionId,
        capability: Capability,
    },
    /// The exact Assessment has release blockers.
    AssessmentReleaseRequirements {
        blocking_issues: Vec<AssessmentReleaseIssue>,
    },
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
pub enum AssessmentEntryRequest {
    /// One fixed Question in an Assessment's ordered content for future Assessment Attempts.
    FixedQuestion {
        question_id: QuestionId,
        points_possible: AssessmentPointValue,
        availability: AssessmentEntryAvailability,
        scoring_rule: AssessmentEntryScoringRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// A server-resolved selection from a pool of immutable questions.
    QuestionPool {
        /// Public Pool identity. The server resolves its source Revision and mints the
        /// Assessment-owned fork; the browser supplies neither source pins nor IDs.
        question_pool_id: QuestionId,
        availability: AssessmentEntryAvailability,
        scoring_rule: AssessmentEntryScoringRule,
        selection_count: std::num::NonZeroU32,
        points_per_item: AssessmentPointValue,
        selection_rule: QuestionPoolSelectionRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
}

/// One closed reason that prevents releasing the current Assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AssessmentReleaseIssue {
    /// No active deliverable Question or Question Pool is present.
    QuestionsRequired,
}

/// Server-derived release validation for the current Assessment.
///
/// An empty issue list means the Assessment Content has the currently known minimum
/// conditions for publication. This Assessment Release Validation is intentionally derived rather than
/// a second persisted state, so future closed blockers can extend it without
/// changing the current Assessment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentReleaseValidation {
    /// Closed, actionable blockers in deterministic order.
    pub blocking_issues: Vec<AssessmentReleaseIssue>,
}

impl AssessmentReleaseValidation {
    /// Derives readiness from current Assessment Content without mutating it.
    pub fn from_entries(entries: &[AssessmentEntry]) -> Self {
        let has_available_fixed_question = entries.iter().any(|entry| {
            matches!(entry, AssessmentEntry::FixedQuestion(question)
                if question.availability == AssessmentEntryAvailability::Available)
        });
        let has_deliverable_question_pool = entries.iter().any(|entry| match entry {
            AssessmentEntry::QuestionPool(pool) => {
                pool.availability == AssessmentEntryAvailability::Available
                    // The fork Revision's members are immutable Pool-owned data. The
                    // persistence boundary verifies that it can satisfy this entry's
                    // positive selection count before release.
                    && pool.selection_count.get() > 0
            }
            AssessmentEntry::FixedQuestion(_) => false,
        });
        let blocking_issues = (!has_available_fixed_question && !has_deliverable_question_pool)
            .then_some(AssessmentReleaseIssue::QuestionsRequired)
            .into_iter()
            .collect();
        Self { blocking_issues }
    }

    /// Returns whether no current release blocker remains.
    pub fn is_ready(&self) -> bool {
        self.blocking_issues.is_empty()
    }

    /// Returns whether this stable Assessment status is valid for the current
    /// Assessment and its released teaching history.
    pub fn permits_status(&self, status: AssessmentStatus, has_released_history: bool) -> bool {
        match status {
            AssessmentStatus::Unreleased | AssessmentStatus::Archived => true,
            AssessmentStatus::Released => self.is_ready(),
            AssessmentStatus::Closed => has_released_history,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AssessmentQuestionVariationRule;

    #[test]
    fn empty_assessment_content_names_the_questions_blocker() {
        let readiness = AssessmentReleaseValidation::from_entries(&[]);

        assert_eq!(
            readiness.blocking_issues,
            vec![AssessmentReleaseIssue::QuestionsRequired]
        );
        assert!(readiness.permits_status(AssessmentStatus::Unreleased, false));
        assert!(readiness.permits_status(AssessmentStatus::Archived, false));
        assert!(!readiness.permits_status(AssessmentStatus::Closed, false));
        assert!(!readiness.permits_status(AssessmentStatus::Released, false));
    }

    #[test]
    fn browser_requests_reject_unknown_members() {
        let result = serde_json::from_str::<CreateAssessmentRequest>(
            r#"{"title":"Protein folding","ignored":true}"#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn create_assessment_request_decodes_title() {
        let request =
            serde_json::from_str::<CreateAssessmentRequest>(r#"{"title":"Protein folding"}"#)
                .expect("strict create request");

        assert_eq!(request.title.as_str(), "Protein folding");
    }

    #[test]
    fn policies_validation_failure_is_a_closed_browser_contract() {
        let failure = AssessmentPoliciesValidationFailure {
            error: AssessmentPoliciesValidationFailureCode::AssessmentPoliciesInvalid,
            issues: vec![
                AssessmentPoliciesValidationIssue::AssessmentReleaseRequirements {
                    blocking_issues: vec![AssessmentReleaseIssue::QuestionsRequired],
                },
            ],
        };

        let value = serde_json::to_value(&failure).expect("policy validation serializes");
        assert_eq!(
            value,
            serde_json::json!({
                "error": "assessmentPoliciesInvalid",
                "issues": [
                    {
                        "kind": "assessmentReleaseRequirements",
                        "blockingIssues": [{"kind": "questionsRequired"}]
                    }
                ]
            })
        );
        let mut unknown = value;
        unknown["unexpected"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<AssessmentPoliciesValidationFailure>(unknown).is_err());
    }

    #[test]
    fn content_and_policy_requests_use_closed_camel_case_contracts() {
        let content = serde_json::from_str::<ReplaceAssessmentContentRequest>(
            r#"{"baseEditNumber":"1","title":"Protein folding","entries":[{"kind":"questionPool","questionPoolId":"7K3M-X9QP","availability":"available","scoringRule":"normal","selectionCount":1,"pointsPerItem":"1","selectionRule":{"selectedQuestionOrder":"questionPoolOrder"},"questionAttemptLimit":{"maxAttempts":null},"questionAttemptTimeLimit":{"kind":"unlimited"}}]}"#,
        );
        assert!(content.is_ok());
        assert!(
            serde_json::from_str::<ReplaceAssessmentContentRequest>(
                r#"{"title":"Protein folding","entries":[{"kind":"questionPool","questionIds":["7K3M-X9QP"],"selectionCount":1,"pointsPerItem":"1","selectedQuestionOrder":"questionPoolOrder"}]}"#,
            )
            .is_err()
        );

        let policy = ReplaceAssessmentPoliciesRequest {
            base_edit_number: "1".parse().expect("edit number"),
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            policies: AssessmentActivityRules {
                assessment_completion_rule: crate::AssessmentCompletionRule::AnswerAll,
                assessment_attempt_grade_rule: crate::AssessmentAttemptGradeRule::Highest,
                assessment_attempt_continuation_rule:
                    crate::AssessmentAttemptContinuationRule::Unlimited,
                question_pool_reuse_rule: crate::QuestionPoolReuseRule::ReuseSelection,
                question_variation_rule: AssessmentQuestionVariationRule::NewVariation,
                ..AssessmentActivityRules::default()
            },
            assessment_authored_content: InstructorAssessmentAuthoredContentLocal::new(
                crate::AssessmentInstructions::default(),
                None,
                None,
                None,
                None,
                None,
                crate::LateWorkRule::Accept,
            )
            .expect("Assessment settings"),
        };
        let mut value = serde_json::to_value(&policy).expect("policy request serialization");
        let record = value.as_object().expect("policy request object");
        assert!(record.contains_key("baseEditNumber"));
        assert!(record.contains_key("studentFeedbackReleaseRule"));
        assert!(record.contains_key("assessmentAuthoredContent"));
        assert_eq!(
            serde_json::from_value::<ReplaceAssessmentPoliciesRequest>(value.clone())
                .expect("policy request roundtrip"),
            policy
        );
        value
            .as_object_mut()
            .expect("policy request object")
            .insert("extra".to_string(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<ReplaceAssessmentPoliciesRequest>(value).is_err());
    }
}
