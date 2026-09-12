//! Immutable Assignment Attempt evidence and the policy provenance that explains it.

use serde::{Deserialize, Serialize};

use super::{AccommodationId, AssignmentAttemptId, AssignmentId, StudentRecordId, Timestamp};
use crate::{
    AssignmentActivityRules, AssignmentAttemptReference, AssignmentInstructions, AssignmentTitle,
    BaseAssignmentPolicy, StudentFeedbackReleaseRule,
};

/// Authoritative completion state of one Assignment Attempt.
///
/// Successor availability is deliberately separate: an Assignment Attempt can have no next
/// attempt because it completed or because it exhausted its attempt policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentAttemptCompletion {
    /// The Assignment Attempt has not satisfied its assignment completion requirement.
    InProgress,
    /// The Assignment Attempt has satisfied its assignment completion requirement.
    Completed,
}

/// Effective Assignment facts retained when an Assignment Attempt starts.
///
/// Resume, submission, grading, history, disclosure, and statistics consume
/// this record with issued-question evidence. They do not reconstruct past
/// meaning from a later mutable Assignment save.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentAttemptEvidence {
    pub title: AssignmentTitle,
    pub instructions: AssignmentInstructions,
    pub base_policy: BaseAssignmentPolicy,
    pub activity_rules: AssignmentActivityRules,
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    pub effective_policy_sources: AssignmentAttemptPolicySources,
}

/// The exact current policy record that supplied a retained effective value.
///
/// Accommodation adjustments currently apply only to Assignment timing and
/// limits. The storage boundary verifies that this identifier belongs to the
/// Attempt's Student Record and Assignment; the domain model deliberately has
/// no broad source category that could claim an unrelated override.
/// This keeps the combined retained evidence contextually consistent (ASVS
/// 2.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssignmentAttemptPolicySource {
    Assignment,
    Accommodation { accommodation: AccommodationId },
}

/// Qualified sources for the only retained effective values that current
/// Student Accommodation adjustments can change.
///
/// Schedule covers available, due, and close instants. The Assignment policy
/// remains the source for all activity, feedback, ordering, reuse, variation,
/// completion, and late-work rules because no current adjustment can change
/// those facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentAttemptPolicySources {
    pub schedule: AssignmentAttemptPolicySource,
    pub assignment_attempt_time_limit: AssignmentAttemptPolicySource,
    pub attempt_limit: AssignmentAttemptPolicySource,
}

impl Default for AssignmentAttemptPolicySources {
    fn default() -> Self {
        Self {
            schedule: AssignmentAttemptPolicySource::Assignment,
            assignment_attempt_time_limit: AssignmentAttemptPolicySource::Assignment,
            attempt_limit: AssignmentAttemptPolicySource::Assignment,
        }
    }
}

/// One pass through an assignment.
///
/// There is deliberately no stored `complete` boolean. The domain derives
/// within-Assignment-Attempt completion from current question states, then records the
/// resulting completion timestamp and score as one transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentAttempt {
    /// Durable Assignment Attempt identity.
    pub id: AssignmentAttemptId,
    /// Stable Assignment Attempt Reference used in application navigation.
    pub reference: AssignmentAttemptReference,
    /// Student Record that owns this Assignment Attempt.
    pub student_record: StudentRecordId,
    /// Assignment that this Student Record attempts.
    pub assignment: AssignmentId,
    /// Effective assignment facts frozen for this Student Work occurrence.
    pub evidence: AssignmentAttemptEvidence,
    /// One-based attempt number for this Student Record and Assignment.
    pub attempt_number: u32,
    /// Server time at which the Assignment Attempt began.
    pub started_at: Timestamp,
    /// Server time at which derived completion was recorded, if complete.
    pub completed_at: Option<Timestamp>,
    /// Score fraction recorded on completion, if complete.
    pub score: Option<f64>,
}

/// The policy-selected course result for one Student Record and Assignment.
///
/// This record owns selected-score pointers only. Immutable Student Work remains
/// under Assignment Attempts and their Issued Questions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssignmentGrade {
    /// Student Record whose course result this is.
    pub student_record: StudentRecordId,
    /// Assignment whose policy selected this result.
    pub assignment: AssignmentId,
    /// First time an Assignment Attempt satisfied completion.
    pub first_completed_at: Option<Timestamp>,
    /// Assignment Attempt currently selected by the grade rule.
    pub current_assignment_attempt: Option<AssignmentAttemptId>,
    /// Score earned by the Assignment Attempt currently selected by the grade rule.
    pub current_score: Option<f64>,
    /// Highest-scoring completed Assignment Attempt.
    pub best_assignment_attempt: Option<AssignmentAttemptId>,
    /// Score earned by the highest-scoring completed Assignment Attempt.
    pub best_score: Option<f64>,
    /// Most recently completed Assignment Attempt.
    pub latest_assignment_attempt: Option<AssignmentAttemptId>,
    /// Score earned by the most recently completed Assignment Attempt.
    pub latest_score: Option<f64>,
}

impl AssignmentAttempt {
    /// Returns the completion state recorded by the authoritative Assignment Attempt.
    pub fn completion(&self) -> AssignmentAttemptCompletion {
        if self.completed_at.is_some() {
            AssignmentAttemptCompletion::Completed
        } else {
            AssignmentAttemptCompletion::InProgress
        }
    }

    /// Returns the frozen Question Pool selection policy for this Attempt.
    pub fn question_pool_reuse_rule(&self) -> crate::QuestionPoolReuseRule {
        self.evidence.activity_rules.question_pool_reuse_rule
    }

    /// Returns the frozen Question Variation policy for this Attempt.
    pub fn question_variation_rule(&self) -> crate::AssignmentQuestionVariationRule {
        self.evidence.activity_rules.question_variation_rule
    }
}
