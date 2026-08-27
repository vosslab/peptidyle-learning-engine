//! Strict browser contracts for the Instructor assignment workspace.
//!
//! These types describe request intent and publication readiness only. The
//! server resolves question references, course-local times, and authority
//! before it changes the authoritative assignment aggregate.

use serde::{Deserialize, Serialize};

use crate::{
    AssignmentDeadlineBehavior, AssignmentDeliveryState, AssignmentLandingPresentation,
    AssignmentLifecycle, AssignmentScoringMode, AssignmentSelectionGroup, Capability,
    CourseGroupReference, IanaTimeZone, InstructorAssignmentTeachingSettingsLocal,
    LateSubmissionPolicy, LearnerDisclosurePolicy, PointValue, QuestionId, RunPolicies,
    SelectionOrdering, VariationPolicy,
};

/// Browser request to create a persisted, incomplete assignment draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAssignmentDraftRequest {
    /// Human-facing title for the new assignment.
    pub title: String,
}

/// Browser request that replaces the Questions-owned assignment content slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceAssignmentContentRequest {
    /// Human-facing title, owned by the Questions workspace.
    pub title: String,
    /// Ordered fixed questions and selection groups for future runs.
    pub entries: Vec<AssignmentEntryRequest>,
}

/// Browser request that replaces the Policies-owned assignment slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceAssignmentPoliciesRequest {
    /// Explicit learner audience.
    pub audience: AssignmentAudienceRequest,
    /// Learner-facing disclosure timing.
    pub disclosure_policy: LearnerDisclosurePolicy,
    /// Completion, grade, practice, and variation policy.
    pub policies: RunPolicies,
    /// Course-local teaching settings resolved by the server before storage.
    pub teaching_settings: InstructorAssignmentTeachingSettingsLocal,
}

/// Browser-safe audience locator. The server resolves group references under
/// the exact course and Instructor authority before constructing the internal
/// [`crate::AssignmentAudience`] used by persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AssignmentAudienceRequest {
    /// Every currently entitled course member may receive the assignment.
    CourseWide,
    /// Any member of one of the supplied course-local groups may receive it.
    AnyOfGroups { groups: Vec<CourseGroupReference> },
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
    /// A course-local teaching setting needs the supplied correction.
    TeachingSettings {
        correction: crate::AssignmentTeachingSettingsValidationFailure,
    },
    /// The selected learner audience cannot be resolved for this course.
    Audience {
        reason: AssignmentAudienceValidationReason,
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
    /// The selected lifecycle needs a publishable definition.
    PublicationReadiness {
        blocking_issues: Vec<AssignmentPublicationBlockingIssue>,
    },
}

/// Closed reason an explicitly group-scoped audience cannot be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentAudienceValidationReason {
    GroupRequired,
    GroupUnavailable,
    GroupsMustBeDistinct,
}

/// Closed reason a combined assignment policy cannot be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentPolicyConfigurationReason {
    SelectedProblemVariantsWithSelectionGroups,
}

/// Closed structural-content refusal for a question definition with issued
/// learner work. Ordinary `409` responses still cover retryable aggregate
/// conflicts; this body identifies the durable recovery path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentContentIssuedWorkConflict {
    pub kind: AssignmentContentIssuedWorkConflictKind,
}

/// Closed semantic reason for a structural Questions save refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentContentIssuedWorkConflictKind {
    IssuedLearnerWork,
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
    /// One fixed question at a visible future-run position.
    Fixed {
        question_id: QuestionId,
        position: u32,
        points_possible: PointValue,
        delivery_state: AssignmentDeliveryState,
        scoring_mode: AssignmentScoringMode,
    },
    /// A random draw from a server-resolved pool of immutable questions.
    SelectionGroup {
        candidate_question_ids: Vec<QuestionId>,
        position: u32,
        draw_count: u32,
        points_per_item: PointValue,
        ordering: SelectionOrdering,
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
pub struct AssignmentPublicationReadiness {
    /// Closed, actionable blockers in deterministic order.
    pub blocking_issues: Vec<AssignmentPublicationBlockingIssue>,
}

/// Answer-free, non-mutating learner landing projection for an Instructor's
/// stable-identity Student view.  It deliberately omits assignment, item,
/// question, run, and attempt identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorStudentView {
    /// Learner-facing assignment title.
    pub title: String,
    /// Learner-facing instructions.
    pub instructions: crate::AssignmentInstructions,
    /// Course scheduling zone used to present the delivery facts.
    pub time_zone: IanaTimeZone,
    /// Server-derived base delivery facts, without learner progress or actions.
    pub delivery: InstructorStudentViewDelivery,
    /// Number of questions a learner receives in one run; derived by the server.
    pub questions_per_run: u32,
    /// Learner-visible variation policy.
    pub variation: VariationPolicy,
    /// Learner-visible disclosure schedule.
    pub disclosure_policy: LearnerDisclosurePolicy,
}

/// Instructor-base delivery facts for stable-identity Student view. These
/// facts describe assignment policy, never a particular learner's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorStudentViewDelivery {
    pub available_at: Option<crate::ActivityTimestamp>,
    pub due_at: Option<crate::ActivityTimestamp>,
    pub closes_at: Option<crate::ActivityTimestamp>,
    pub time_limit_seconds: Option<u32>,
    pub attempt_limit: Option<u32>,
    pub late_submission: LateSubmissionPolicy,
    pub deadline_behavior: AssignmentDeadlineBehavior,
}

impl InstructorStudentView {
    /// Adds the Instructor Student-view delivery envelope to the shared
    /// answer-free assignment landing presentation.
    pub fn from_landing(
        landing: AssignmentLandingPresentation,
        delivery: InstructorStudentViewDelivery,
    ) -> Self {
        Self {
            title: landing.title,
            instructions: landing.instructions,
            time_zone: landing.time_zone,
            delivery,
            questions_per_run: landing.questions_per_run,
            variation: landing.variation,
            disclosure_policy: landing.disclosure_policy,
        }
    }
}

impl AssignmentPublicationReadiness {
    /// Derives readiness from the current definition without mutating it.
    pub fn from_definition(
        items: &[crate::AssignmentItem],
        selection_groups: &[AssignmentSelectionGroup],
    ) -> Self {
        let has_active_fixed_item = items
            .iter()
            .any(|item| item.delivery_state == AssignmentDeliveryState::Active);
        let has_deliverable_selection_group = selection_groups.iter().any(|group| {
            group.draw_count > 0
                && group
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.delivery_state == AssignmentDeliveryState::Active)
                    .count()
                    >= usize::try_from(group.draw_count).unwrap_or(usize::MAX)
        });
        let blocking_issues = (!has_active_fixed_item && !has_deliverable_selection_group)
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
    /// deliverable position.
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
        let readiness = AssignmentPublicationReadiness::from_definition(&[], &[]);

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

        assert_eq!(request.title, "Protein folding");
    }

    #[test]
    fn policies_validation_failure_is_a_closed_browser_contract() {
        let failure = AssignmentPoliciesValidationFailure {
            error: AssignmentPoliciesValidationFailureCode::AssignmentPoliciesInvalid,
            issues: vec![
                AssignmentPoliciesValidationIssue::Audience {
                    reason: AssignmentAudienceValidationReason::GroupRequired,
                },
                AssignmentPoliciesValidationIssue::Configuration {
                    reason:
                        AssignmentPolicyConfigurationReason::SelectedProblemVariantsWithSelectionGroups,
                },
                AssignmentPoliciesValidationIssue::PublicationReadiness {
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
                    {"kind": "audience", "reason": "groupRequired"},
                    {
                        "kind": "configuration",
                        "reason": "selectedProblemVariantsWithSelectionGroups"
                    },
                    {
                        "kind": "publicationReadiness",
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
    fn issued_work_content_conflict_is_a_closed_browser_contract() {
        let conflict = AssignmentContentIssuedWorkConflict {
            kind: AssignmentContentIssuedWorkConflictKind::IssuedLearnerWork,
        };

        let value = serde_json::to_value(conflict).expect("issued-work conflict serializes");
        assert_eq!(value, serde_json::json!({ "kind": "issuedLearnerWork" }));
        assert!(serde_json::from_value::<AssignmentContentIssuedWorkConflict>(value).is_ok());
        assert!(
            serde_json::from_value::<AssignmentContentIssuedWorkConflict>(
                serde_json::json!({ "kind": "issuedLearnerWork", "extra": true })
            )
            .is_err()
        );
    }

    #[test]
    fn content_and_policy_requests_use_closed_camel_case_contracts() {
        let content = serde_json::from_str::<ReplaceAssignmentContentRequest>(
            r#"{"title":"Protein folding","entries":[]}"#,
        );
        assert!(content.is_ok());
        assert!(
            serde_json::from_str::<ReplaceAssignmentContentRequest>(
                r#"{"title":"Protein folding","entries":[],"extra":true}"#,
            )
            .is_err()
        );

        let policy = ReplaceAssignmentPoliciesRequest {
            audience: AssignmentAudienceRequest::CourseWide,
            disclosure_policy: LearnerDisclosurePolicy::default(),
            policies: RunPolicies {
                completion: crate::CompletionRequirement::AnswerAll,
                grade: crate::GradePolicy::Highest,
                continued_practice: crate::ContinuedPractice::Unlimited,
                variation: VariationPolicy::NewSeeds,
            },
            teaching_settings: InstructorAssignmentTeachingSettingsLocal::new(
                "America/Chicago".parse().expect("IANA zone"),
                AssignmentLifecycle::Draft,
                crate::AssignmentInstructions::default(),
                None,
                None,
                None,
                None,
                None,
                crate::LateSubmissionPolicy::Accept,
                crate::AssignmentDeadlineBehavior::AutoSubmit,
            )
            .expect("draft settings"),
        };
        let mut value = serde_json::to_value(&policy).expect("policy request serialization");
        let record = value.as_object().expect("policy request object");
        assert!(record.contains_key("disclosurePolicy"));
        assert!(record.contains_key("teachingSettings"));
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
