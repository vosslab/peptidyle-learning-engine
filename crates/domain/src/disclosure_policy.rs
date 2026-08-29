//! Pure Student disclosure evaluation.
//!
//! The assignment-policy resolver has already resolved the Student's effective
//! assignment window and access verdict. This module only
//! consumes that verdict and an authoritative supplied timestamp. It neither
//! reconstructs access decisions nor records a feedback-release receipt.

use question_model::{
    ActivityTimestamp, AttemptResult, DisclosedFeedback, FeedbackContent,
    InspectedStudentScoreFeedbackV1, ScoringStatus, StudentDisclosurePolicy,
    StudentDisclosureTiming,
};

use crate::effective_assignment_policy::{EffectiveAssignmentPolicy, EffectivePolicyDecision};

/// The five independently evaluated Student-facing disclosure fields.
///
/// A caller uses these booleans to omit protected fields from a projection;
/// this type contains no protected content itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudentDisclosureDecision {
    pub score: bool,
    pub per_item_correctness: bool,
    pub feedback_text: bool,
    pub solution: bool,
    pub class_statistics: bool,
}

/// Removes score-dependent feedback fields while an assignment score is not current.
///
/// A recalculating or failed aggregate must not expose an older numeric score or
/// correctness verdict beside its current status.
pub fn score_current_disclosure(
    mut decision: StudentDisclosureDecision,
    scoring_status: ScoringStatus,
) -> StudentDisclosureDecision {
    if !matches!(scoring_status, ScoringStatus::Current) {
        decision.score = false;
        decision.per_item_correctness = false;
    }
    decision
}

/// Projects exactly the independently disclosed Student feedback fields.
///
/// `None` means no feedback field is currently visible. Hidden fields are
/// omitted rather than represented by null or a protected-content marker.
pub fn project_disclosed_feedback(
    decision: StudentDisclosureDecision,
    result: Option<AttemptResult>,
    content: &FeedbackContent,
) -> Option<DisclosedFeedback> {
    let mut disclosed = DisclosedFeedback::empty();
    if let Some(result) = result {
        if decision.per_item_correctness {
            disclosed.correctness = Some(result.correct);
        }
        if decision.score {
            disclosed.points_earned = Some(result.points_earned);
            disclosed.points_possible = Some(result.points_possible);
        }
    }
    if decision.feedback_text {
        disclosed.hint = content.hint.clone();
        disclosed.rationale = content.rationale.clone();
    }
    if decision.solution {
        disclosed.correct_response = content.correct_response.clone();
    }
    (decision.per_item_correctness || decision.score || decision.feedback_text || decision.solution)
        .then_some(disclosed)
}

/// Projects feedback for an Instructor inspecting one Student's submitted work.
///
/// Inspection can show only the current score and correctness permitted by
/// assignment disclosure. Hint, rationale, solution, and correct-response
/// content have no representation in this detail capability.
pub fn project_inspected_student_score_feedback(
    decision: StudentDisclosureDecision,
    scoring_status: ScoringStatus,
    result: Option<AttemptResult>,
) -> InspectedStudentScoreFeedbackV1 {
    let decision = score_current_disclosure(decision, scoring_status);
    let mut feedback = InspectedStudentScoreFeedbackV1::empty();
    if let Some(result) = result {
        if decision.per_item_correctness {
            feedback.correctness = Some(result.correct);
        }
        if decision.score {
            feedback.points_earned = Some(result.points_earned);
            feedback.points_possible = Some(result.points_possible);
        }
    }
    feedback
}

/// Evaluates Student disclosure from one already-resolved assignment policy verdict.
///
/// A denied assignment-policy verdict produces no Student decision. For allowed verdicts,
/// every field is evaluated independently using the supplied server timestamp
/// and the resolved due/close times, if present. A submission timestamp is
/// evidence that the current Student submitted; it is not read from storage.
pub fn evaluate_student_disclosure(
    disclosure: StudentDisclosurePolicy,
    effective_policy: &EffectivePolicyDecision,
    now: ActivityTimestamp,
    submitted_at: Option<ActivityTimestamp>,
) -> Option<StudentDisclosureDecision> {
    let EffectivePolicyDecision::Allowed { policy, .. } = effective_policy else {
        return None;
    };

    Some(evaluate_allowed_student_disclosure(
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
/// [`evaluate_student_disclosure`]; this helper does not authorize access.
pub fn evaluate_allowed_student_disclosure(
    policy: &EffectiveAssignmentPolicy,
    disclosure: StudentDisclosurePolicy,
    now: ActivityTimestamp,
    submitted_at: Option<ActivityTimestamp>,
) -> StudentDisclosureDecision {
    StudentDisclosureDecision {
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
    timing: StudentDisclosureTiming,
    now: ActivityTimestamp,
    submitted_at: Option<ActivityTimestamp>,
    due_at: Option<ActivityTimestamp>,
    closes_at: Option<ActivityTimestamp>,
) -> bool {
    match timing {
        StudentDisclosureTiming::DuringAttempt => true,
        StudentDisclosureTiming::AfterSubmit => submitted_at.is_some(),
        StudentDisclosureTiming::AfterDue => due_at.is_some_and(|due_at| now >= due_at),
        StudentDisclosureTiming::AfterClose => closes_at.is_some_and(|closes_at| now >= closes_at),
        StudentDisclosureTiming::Never => false,
    }
}

#[cfg(test)]
mod tests;
