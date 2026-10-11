//! Strict browser contracts for the Instructor assessment workspace.
//!
//! These types describe request intent and publication validation only. The
//! server resolves Question Revision Tuples, Account-local times, and authority
//! before it changes the authoritative current Assessment.

use serde::{Deserialize, Serialize};

use crate::{
    AssessmentActivityRules, AssessmentEditNumber, AssessmentEntry, AssessmentEntryAvailability,
    AssessmentEntryScoringRule, AssessmentPointValue, AssessmentStatus, AssessmentTitle,
    Capability, InstructorAssessmentAuthoredContentLocal, PublishedQuestionId,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionPoolId, StudentFeedbackReleaseRule,
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
        question_id: PublishedQuestionId,
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
        question_id: PublishedQuestionId,
        points_possible: AssessmentPointValue,
        availability: AssessmentEntryAvailability,
        scoring_rule: AssessmentEntryScoringRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// A server-resolved selection from a pool of immutable questions.
    QuestionPool {
        /// Public ID of an existing reusable Question Pool.
        question_pool_id: QuestionPoolId,
        availability: AssessmentEntryAvailability,
        scoring_rule: AssessmentEntryScoringRule,
        selection_count: std::num::NonZeroU32,
        points_per_item: AssessmentPointValue,
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
                    // Before release, persistence checks the current referenced Pool
                    // contains enough Published Questions for this selection count.
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
