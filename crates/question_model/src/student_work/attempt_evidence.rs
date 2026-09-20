//! Immutable Assessment Attempt evidence and the policy provenance that explains it.

use serde::{Deserialize, Serialize};

use super::{AccommodationId, AssessmentAttemptId, AssessmentId, StudentRecordId, Timestamp};
use crate::{
    AssessmentActivityRules, AssessmentInstructions, AssessmentTitle, BaseAssessmentPolicy,
    StudentFeedbackReleaseRule,
};

/// Authoritative completion state of one Assessment Attempt.
///
/// Successor availability is deliberately separate: an Assessment Attempt can have no next
/// attempt because it completed or because it exhausted its attempt policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentAttemptCompletion {
    /// The Assessment Attempt has not been submitted.
    InProgress,
    /// The Assessment Attempt was submitted by the Student or automatically at expiry.
    Completed,
}

/// Effective Assessment facts retained when an Assessment Attempt starts.
///
/// Resume, submission, grading, history, disclosure, and statistics consume
/// this record with issued-question evidence. They do not reconstruct past
/// meaning from a later mutable Assessment save.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentAttemptEvidence {
    pub title: AssessmentTitle,
    pub instructions: AssessmentInstructions,
    pub base_policy: BaseAssessmentPolicy,
    pub activity_rules: AssessmentActivityRules,
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    pub effective_policy_sources: AssessmentAttemptPolicySources,
}

/// The exact current policy record that supplied a retained effective value.
///
/// Accommodation adjustments currently apply only to Assessment timing and
/// limits. The storage boundary verifies that this identifier belongs to the
/// Attempt's Student Record and Assessment; the domain model deliberately has
/// no broad source category that could claim an unrelated override.
/// This keeps the combined retained evidence contextually consistent (ASVS
/// 2.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssessmentAttemptPolicySource {
    Assessment,
    Accommodation { accommodation: AccommodationId },
}

/// Qualified sources for the only retained effective values that current
/// Student Accommodation adjustments can change.
///
/// Schedule covers available, due, and close instants. The Assessment policy
/// remains the source for all activity, feedback, ordering, variation,
/// and late-work rules because no current adjustment can change
/// those facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentAttemptPolicySources {
    pub schedule: AssessmentAttemptPolicySource,
    pub assessment_attempt_time_limit: AssessmentAttemptPolicySource,
    pub attempt_limit: AssessmentAttemptPolicySource,
}

impl Default for AssessmentAttemptPolicySources {
    fn default() -> Self {
        Self {
            schedule: AssessmentAttemptPolicySource::Assessment,
            assessment_attempt_time_limit: AssessmentAttemptPolicySource::Assessment,
            attempt_limit: AssessmentAttemptPolicySource::Assessment,
        }
    }
}

/// One pass through an assessment.
///
/// There is deliberately no stored `complete` boolean. Submission records the
/// terminal timestamp independently of correctness, score, and grading.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentAttempt {
    /// Durable Assessment Attempt identity.
    pub id: AssessmentAttemptId,
    /// Student Record that owns this Assessment Attempt.
    pub student_record: StudentRecordId,
    /// Assessment that this Student Record attempts.
    pub assessment_id: AssessmentId,
    /// Effective assessment facts frozen for this Student Work occurrence.
    pub evidence: AssessmentAttemptEvidence,
    /// One-based attempt number for this Student Record and Assessment.
    pub attempt_number: u32,
    /// Server time at which the Assessment Attempt began.
    pub started_at: Timestamp,
    /// Server time at which Student or expiry submission occurred, if submitted.
    pub submitted_at: Option<Timestamp>,
    /// Score fraction recorded after grading, when available.
    pub score: Option<f64>,
}

/// The policy-selected course result for one Student Record and Assessment.
///
/// This record owns selected-score pointers only. Immutable Student Work remains
/// under Assessment Attempts and their Issued Questions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssessmentGrade {
    /// Student Record whose course result this is.
    pub student_record: StudentRecordId,
    /// Assessment whose policy selected this result.
    pub assessment_id: AssessmentId,
    /// First time an Assessment Attempt satisfied completion.
    pub first_completed_at: Option<Timestamp>,
    /// Assessment Attempt currently selected by the grade rule.
    pub current_assessment_attempt: Option<AssessmentAttemptId>,
    /// Score earned by the Assessment Attempt currently selected by the grade rule.
    pub current_score: Option<f64>,
    /// Highest-scoring completed Assessment Attempt.
    pub best_assessment_attempt: Option<AssessmentAttemptId>,
    /// Score earned by the highest-scoring completed Assessment Attempt.
    pub best_score: Option<f64>,
    /// Most recently completed Assessment Attempt.
    pub latest_assessment_attempt: Option<AssessmentAttemptId>,
    /// Score earned by the most recently completed Assessment Attempt.
    pub latest_score: Option<f64>,
}

impl AssessmentAttempt {
    /// Returns the completion state recorded by the authoritative Assessment Attempt.
    pub fn completion(&self) -> AssessmentAttemptCompletion {
        if self.submitted_at.is_some() {
            AssessmentAttemptCompletion::Completed
        } else {
            AssessmentAttemptCompletion::InProgress
        }
    }

    /// Returns the frozen Question Variation policy for this Attempt.
    pub fn question_variation_rule(&self) -> crate::AssessmentQuestionVariationRule {
        self.evidence.activity_rules.question_variation_rule
    }
}
