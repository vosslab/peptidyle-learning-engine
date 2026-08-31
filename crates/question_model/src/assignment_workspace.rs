//! Strict browser contracts for the Instructor assignment workspace.
//!
//! These types describe request intent and publication readiness only. The
//! server resolves question references, course-local times, and authority
//! before it changes the authoritative assignment aggregate.

use serde::{Deserialize, Serialize};

use crate::curriculum_adoption::AssignmentRevisionReference;
use crate::{
    AssignmentActivityRules, AssignmentDeadlineRule, AssignmentEntry, AssignmentEntryAvailability,
    AssignmentEntryScoringRule, AssignmentLifecycle, AssignmentOverview, AssignmentPointValue,
    AssignmentTitle, Capability, CourseTimeZone, InstructorAssignmentRevisionDefinitionLocal,
    LateWorkRule, QuestionId, QuestionPoolCandidateAvailability, QuestionPoolSelectionRule,
    QuestionVariationRule, StudentFeedbackReleaseRule,
};

/// Browser request to create a persisted, incomplete assignment draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAssignmentDraftRequest {
    /// Human-facing title for the new assignment.
    pub title: AssignmentTitle,
}

/// Browser request that replaces the Questions-owned assignment content slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceAssignmentContentRequest {
    /// Exact immutable draft revision the Instructor reviewed before editing.
    pub base_revision: AssignmentRevisionReference,
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
    /// Exact immutable draft revision the Instructor reviewed before editing.
    pub base_revision: AssignmentRevisionReference,
    /// Public Question ID resolved to an assignable immutable publication by
    /// the authenticated server.
    pub question_id: QuestionId,
}

/// Browser request that replaces the Policies-owned assignment slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceAssignmentPoliciesRequest {
    /// Exact immutable draft revision the Instructor reviewed before editing.
    pub base_revision: AssignmentRevisionReference,
    /// Student-facing disclosure timing.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    /// Completion, grade, practice, and Question Variation Rule.
    pub policies: AssignmentActivityRules,
    /// Course-local Assignment Revision Definition resolved by the server before storage.
    pub assignment_revision_definition: InstructorAssignmentRevisionDefinitionLocal,
}

/// Browser-safe refusal returned when the Policies workspace cannot save its
/// complete aggregate update. Once the server can build a valid teaching-state
/// candidate, it returns every independently determinable correction in stable
/// order before persistence changes the assignment revision. A malformed
/// teaching state or illegal lifecycle transition is returned alone because it
/// prevents constructing that candidate.
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
    /// A course-local Assignment Revision Definition needs the supplied correction.
    AssignmentRevisionDefinition {
        correction: crate::AssignmentRevisionDefinitionValidationFailure,
    },
    /// The combined policy configuration is not available.
    Configuration {
        reason: AssignmentPolicyConfigurationReason,
    },
    /// A selected question backend cannot satisfy one required capability.
    Capability {
        title: String,
        question_id: QuestionId,
        capability: Capability,
    },
    /// The exact draft assignment revision has publication blockers.
    DraftRevisionPublicationReadiness {
        blocking_issues: Vec<AssignmentPublicationBlockingIssue>,
    },
}

/// Closed reason a combined assignment policy cannot be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentPolicyConfigurationReason {
    SelectedQuestionVariantsWithQuestionPools,
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
    /// A random draw from a server-resolved pool of immutable questions.
    QuestionPool {
        candidate_question_ids: Vec<QuestionId>,
        availability: AssignmentEntryAvailability,
        scoring_rule: AssignmentEntryScoringRule,
        draw_count: u32,
        points_per_item: AssignmentPointValue,
        selection_rule: QuestionPoolSelectionRule,
    },
}

/// One closed reason that prevents publishing the current assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AssignmentPublicationBlockingIssue {
    /// Questions owns the missing active-deliverable correction.
    QuestionsRequired,
}

/// Server-derived publication readiness for the current assignment definition.
///
/// An empty issue list means the definition has the currently known minimum
/// conditions for publication. This is intentionally a projection rather than
/// a second persisted state, so future closed blockers can extend it without
/// changing the assignment aggregate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftAssignmentRevisionPublicationReadiness {
    /// Closed, actionable blockers in deterministic order.
    pub blocking_issues: Vec<AssignmentPublicationBlockingIssue>,
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
            question_variation_rule: landing.question_variation_rule,
            student_feedback_release_rule: landing.student_feedback_release_rule,
        }
    }
}

impl DraftAssignmentRevisionPublicationReadiness {
    /// Derives readiness from the current definition without mutating it.
    pub fn from_entries(entries: &[AssignmentEntry]) -> Self {
        let has_available_fixed_question = entries.iter().any(|entry| {
            matches!(entry, AssignmentEntry::FixedQuestion(question)
                if question.availability == AssignmentEntryAvailability::Available)
        });
        let has_deliverable_question_pool = entries.iter().any(|entry| match entry {
            AssignmentEntry::QuestionPool(pool) => {
                pool.availability == AssignmentEntryAvailability::Available
                    && pool.draw_count > 0
                    && pool
                        .candidates
                        .iter()
                        .filter(|candidate| {
                            candidate.availability == QuestionPoolCandidateAvailability::Available
                        })
                        .count()
                        >= usize::try_from(pool.draw_count).unwrap_or(usize::MAX)
            }
            AssignmentEntry::FixedQuestion(_) => false,
        });
        let blocking_issues = (!has_available_fixed_question && !has_deliverable_question_pool)
            .then_some(AssignmentPublicationBlockingIssue::QuestionsRequired)
            .into_iter()
            .collect();
        Self { blocking_issues }
    }

    /// Returns whether no current publication blocker remains.
    pub fn is_ready(&self) -> bool {
        self.blocking_issues.is_empty()
    }

    /// Returns whether this lifecycle is permitted for this definition.
    ///
    /// A new or archived definition may be empty. Closed assignments retain a
    /// historical definition, and Published assignments require an active
    /// deliverable entry.
    pub fn permits_lifecycle(&self, lifecycle: AssignmentLifecycle, has_definition: bool) -> bool {
        match lifecycle {
            AssignmentLifecycle::Draft | AssignmentLifecycle::Archived => true,
            AssignmentLifecycle::Published => self.is_ready(),
            AssignmentLifecycle::Closed => has_definition,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_definition_names_the_questions_blocker() {
        let readiness = DraftAssignmentRevisionPublicationReadiness::from_entries(&[]);

        assert_eq!(
            readiness.blocking_issues,
            vec![AssignmentPublicationBlockingIssue::QuestionsRequired]
        );
        assert!(readiness.permits_lifecycle(AssignmentLifecycle::Draft, false));
        assert!(readiness.permits_lifecycle(AssignmentLifecycle::Archived, false));
        assert!(!readiness.permits_lifecycle(AssignmentLifecycle::Closed, false));
        assert!(!readiness.permits_lifecycle(AssignmentLifecycle::Published, false));
    }

    #[test]
    fn browser_requests_reject_unknown_members() {
        let result = serde_json::from_str::<CreateAssignmentDraftRequest>(
            r#"{"title":"Protein folding","ignored":true}"#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn browser_draft_request_decodes_title() {
        let request =
            serde_json::from_str::<CreateAssignmentDraftRequest>(r#"{"title":"Protein folding"}"#)
                .expect("strict draft request");

        assert_eq!(request.title.as_str(), "Protein folding");
    }

    #[test]
    fn policies_validation_failure_is_a_closed_browser_contract() {
        let failure = AssignmentPoliciesValidationFailure {
            error: AssignmentPoliciesValidationFailureCode::AssignmentPoliciesInvalid,
            issues: vec![
                AssignmentPoliciesValidationIssue::Configuration {
                    reason:
                        AssignmentPolicyConfigurationReason::SelectedQuestionVariantsWithQuestionPools,
                },
                AssignmentPoliciesValidationIssue::DraftRevisionPublicationReadiness {
                    blocking_issues: vec![AssignmentPublicationBlockingIssue::QuestionsRequired],
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
                        "kind": "configuration",
                        "reason": "selectedQuestionVariantsWithQuestionPools"
                    },
                    {
                        "kind": "draftRevisionPublicationReadiness",
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
            r#"{"baseRevision":{"assignment":"A-1","revision_number":"1"},"title":"Protein folding","entries":[{"kind":"questionPool","candidateQuestionIds":["7K3-M9QP"],"availability":"available","scoringRule":"normal","drawCount":1,"pointsPerItem":"1","selectionRule":{"algorithm":"v1","ordering":"candidateOrder"}}]}"#,
        );
        assert!(content.is_ok());
        assert!(
            serde_json::from_str::<ReplaceAssignmentContentRequest>(
                r#"{"title":"Protein folding","entries":[{"kind":"questionPool","candidateQuestionIds":["7K3-M9QP"],"drawCount":1,"pointsPerItem":"1","ordering":"candidateOrder"}]}"#,
            )
            .is_err()
        );

        let policy = ReplaceAssignmentPoliciesRequest {
            base_revision: serde_json::from_str(r#"{"assignment":"A-1","revision_number":"1"}"#)
                .expect("exact Assignment Revision Reference"),
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            policies: AssignmentActivityRules {
                assignment_completion_rule: crate::AssignmentCompletionRule::AnswerAll,
                assignment_attempt_grade_rule: crate::AssignmentAttemptGradeRule::Highest,
                assignment_attempt_continuation_rule:
                    crate::AssignmentAttemptContinuationRule::Unlimited,
                question_variation_rule: QuestionVariationRule::ReuseQuestionsWithNewSeeds,
                ..AssignmentActivityRules::default()
            },
            assignment_revision_definition: InstructorAssignmentRevisionDefinitionLocal::new(
                "America/Chicago".parse().expect("IANA zone"),
                AssignmentLifecycle::Draft,
                crate::AssignmentInstructions::default(),
                None,
                None,
                None,
                None,
                None,
                crate::LateWorkRule::Accept,
                crate::AssignmentDeadlineRule::AutoSubmit,
            )
            .expect("draft settings"),
        };
        let mut value = serde_json::to_value(&policy).expect("policy request serialization");
        let record = value.as_object().expect("policy request object");
        assert!(record.contains_key("baseRevision"));
        assert!(record.contains_key("studentFeedbackReleaseRule"));
        assert!(record.contains_key("assignmentRevisionDefinition"));
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
