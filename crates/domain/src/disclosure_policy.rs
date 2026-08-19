//! Pure learner disclosure evaluation (WP-PROF-S4).
//!
//! S3 has already resolved the learner's effective assignment window, and S5
//! has already determined whether the learner may access it. This module only
//! consumes that verdict and an authoritative supplied timestamp. It neither
//! reconstructs access decisions nor records a feedback-release receipt.

use question_model::{ActivityTimestamp, LearnerDisclosurePolicy, LearnerDisclosureTiming};

use crate::effective_assignment_policy::{EffectiveAssignmentPolicy, EffectivePolicyDecision};

/// The five independently evaluated learner-facing disclosure fields.
///
/// A caller uses these booleans to omit protected fields from a projection;
/// this type contains no protected content itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearnerDisclosureDecision {
    pub score: bool,
    pub per_item_correctness: bool,
    pub feedback_text: bool,
    pub solution: bool,
    pub class_statistics: bool,
}

/// Evaluates learner disclosure from one already-resolved S3 policy verdict.
///
/// A denied S3 verdict produces no learner decision. For allowed verdicts,
/// every field is evaluated independently using the supplied server timestamp
/// and the resolved due/close times, if present. A submission timestamp is
/// evidence that the current learner submitted; it is not read from storage.
pub fn evaluate_learner_disclosure(
    disclosure: LearnerDisclosurePolicy,
    effective_policy: &EffectivePolicyDecision,
    now: ActivityTimestamp,
    submitted_at: Option<ActivityTimestamp>,
) -> Option<LearnerDisclosureDecision> {
    let EffectivePolicyDecision::Allowed { policy, .. } = effective_policy else {
        return None;
    };

    Some(evaluate_allowed_learner_disclosure(
        policy,
        disclosure,
        now,
        submitted_at,
    ))
}

/// Evaluates disclosure from an already-authorized effective assignment policy.
///
/// Store receipt projections may retain the resolved policy without its S3
/// gate verdict. Callers that still have the verdict should prefer
/// [`evaluate_learner_disclosure`]; this helper does not authorize access.
pub fn evaluate_allowed_learner_disclosure(
    policy: &EffectiveAssignmentPolicy,
    disclosure: LearnerDisclosurePolicy,
    now: ActivityTimestamp,
    submitted_at: Option<ActivityTimestamp>,
) -> LearnerDisclosureDecision {
    LearnerDisclosureDecision {
        score: timing_released(
            disclosure.score,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        per_item_correctness: timing_released(
            disclosure.per_item_correctness,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        feedback_text: timing_released(
            disclosure.feedback_text,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        solution: timing_released(
            disclosure.solution,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        class_statistics: timing_released(
            disclosure.class_statistics,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
    }
}

fn timing_released(
    timing: LearnerDisclosureTiming,
    now: ActivityTimestamp,
    submitted_at: Option<ActivityTimestamp>,
    due_at: Option<ActivityTimestamp>,
    closes_at: Option<ActivityTimestamp>,
) -> bool {
    match timing {
        LearnerDisclosureTiming::DuringAttempt => true,
        LearnerDisclosureTiming::AfterSubmit => submitted_at.is_some(),
        LearnerDisclosureTiming::AfterDue => due_at.is_some_and(|due_at| now >= due_at),
        LearnerDisclosureTiming::AfterClose => closes_at.is_some_and(|closes_at| now >= closes_at),
        LearnerDisclosureTiming::Never => false,
    }
}

#[cfg(test)]
mod tests;
