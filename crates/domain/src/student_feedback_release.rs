//! Pure Student Feedback Release evaluation.
//!
//! The assignment-policy resolver has already resolved the Student's effective
//! assignment window and access verdict. This module only
//! consumes that verdict and an authoritative supplied timestamp. It neither
//! reconstructs access decisions nor records a feedback-release receipt.

use question_model::{
    AssignmentScoringState, GradingResult, QuestionPostGradingContent, StudentFeedback,
    StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming, StudentResponseInspectionFeedback,
    Timestamp,
};

use crate::effective_assignment_policy::{AssignmentAccessDecision, EffectiveAssignmentPolicy};

/// The six independently evaluated Student Feedback Release fields.
///
/// A caller uses these booleans to omit protected fields from a projection;
/// this type contains no protected content itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudentFeedbackReleaseDecision {
    pub score: bool,
    pub per_item_correctness: bool,
    pub feedback_text: bool,
    pub question_answer: bool,
    pub question_answer_explanation: bool,
    pub class_statistics: bool,
}

/// Removes score-dependent feedback fields while an assignment score is not current.
///
/// A recalculating or failed aggregate must not expose an older numeric score or
/// correctness verdict beside its current status.
pub fn score_current_student_feedback_release(
    mut decision: StudentFeedbackReleaseDecision,
    assignment_scoring_state: AssignmentScoringState,
) -> StudentFeedbackReleaseDecision {
    if !matches!(assignment_scoring_state, AssignmentScoringState::Current) {
        decision.score = false;
        decision.per_item_correctness = false;
    }
    decision
}

/// Projects exactly the independently disclosed Student feedback fields.
///
/// `None` means no feedback field is currently visible. Hidden fields are
/// omitted rather than represented by null or a protected-content marker.
pub fn project_student_feedback(
    decision: StudentFeedbackReleaseDecision,
    result: Option<GradingResult>,
    content: &QuestionPostGradingContent,
) -> Option<StudentFeedback> {
    let mut disclosed = StudentFeedback::empty();
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
        disclosed.choice_feedback = content.question_feedback.choice_feedback.clone();
        disclosed.correct_feedback = content.question_feedback.correct_feedback.clone();
        disclosed.incorrect_feedback = content.question_feedback.incorrect_feedback.clone();
    }
    if decision.question_answer {
        disclosed.question_answer = content
            .question_answer
            .as_ref()
            .map(|question_answer| question_answer.content().to_vec());
    }
    if decision.question_answer_explanation {
        disclosed.question_answer_explanation = content
            .question_answer_explanation
            .as_ref()
            .map(|explanation| explanation.content().to_vec());
    }
    (decision.per_item_correctness
        || decision.score
        || decision.feedback_text
        || decision.question_answer
        || decision.question_answer_explanation)
        .then_some(disclosed)
}

/// Projects feedback for an Instructor inspecting one Student's submitted work.
///
/// Inspection can show only the current score and correctness permitted by
/// assignment disclosure. Question Hint, Question Answer, and Question Answer
/// Explanation have no representation in this detail capability.
pub fn project_student_response_inspection_feedback(
    decision: StudentFeedbackReleaseDecision,
    assignment_scoring_state: AssignmentScoringState,
    result: Option<GradingResult>,
) -> StudentResponseInspectionFeedback {
    let decision = score_current_student_feedback_release(decision, assignment_scoring_state);
    let mut feedback = StudentResponseInspectionFeedback::empty();
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

/// Evaluates Student Feedback Release from one already-resolved assignment policy verdict.
///
/// A denied assignment-policy verdict produces no Student decision. For allowed verdicts,
/// every field is evaluated independently using the supplied server timestamp
/// and the resolved due/close times, if present. A submission timestamp is
/// evidence that the current Student submitted; it is not read from storage.
pub fn evaluate_student_feedback_release(
    rule: StudentFeedbackReleaseRule,
    effective_policy: &AssignmentAccessDecision,
    now: Timestamp,
    submitted_at: Option<Timestamp>,
) -> Option<StudentFeedbackReleaseDecision> {
    let AssignmentAccessDecision::Allowed { policy, .. } = effective_policy else {
        return None;
    };

    Some(evaluate_allowed_student_feedback_release(
        policy,
        rule,
        now,
        submitted_at,
    ))
}

/// Evaluates Student Feedback Release from an already-authorized effective assignment policy.
///
/// Store receipt projections may retain the resolved policy without its S3
/// gate verdict. Callers that still have the verdict should prefer
/// [`evaluate_student_feedback_release`]; this helper does not authorize access.
pub fn evaluate_allowed_student_feedback_release(
    policy: &EffectiveAssignmentPolicy,
    rule: StudentFeedbackReleaseRule,
    now: Timestamp,
    submitted_at: Option<Timestamp>,
) -> StudentFeedbackReleaseDecision {
    StudentFeedbackReleaseDecision {
        score: timing_released(
            rule.score,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        per_item_correctness: timing_released(
            rule.per_item_correctness,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        feedback_text: timing_released(
            rule.feedback_text,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        question_answer: timing_released(
            rule.question_answer,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        question_answer_explanation: timing_released(
            rule.question_answer_explanation,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        class_statistics: timing_released(
            rule.class_statistics,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
    }
}

fn timing_released(
    timing: StudentFeedbackReleaseTiming,
    now: Timestamp,
    submitted_at: Option<Timestamp>,
    due_at: Option<Timestamp>,
    closes_at: Option<Timestamp>,
) -> bool {
    match timing {
        StudentFeedbackReleaseTiming::DuringAttempt => true,
        StudentFeedbackReleaseTiming::AfterSubmit => submitted_at.is_some(),
        StudentFeedbackReleaseTiming::AfterDue => due_at.is_some_and(|due_at| now >= due_at),
        StudentFeedbackReleaseTiming::AfterClose => {
            closes_at.is_some_and(|closes_at| now >= closes_at)
        }
        StudentFeedbackReleaseTiming::Never => false,
    }
}

#[cfg(test)]
mod tests;
