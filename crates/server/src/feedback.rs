//! Allowlist-only learner feedback projection.
//!
//! The data-access layer supplies an already-authorized, current S4 decision.
//! This module never consults catalog question policy, releases, or clocks.

use domain::disclosure_policy::LearnerDisclosureDecision;
use question_model::{AttemptResult, DisclosedFeedback, FeedbackContent};

/// Projects exactly the independently disclosed learner fields.
///
/// `None` means that no feedback field is currently visible. Hidden fields
/// are omitted rather than represented by null or a secret-bearing marker.
pub fn project_feedback(
    decision: LearnerDisclosureDecision,
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

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::envelope::ContentBlock;

    fn decision() -> LearnerDisclosureDecision {
        LearnerDisclosureDecision {
            score: false,
            per_item_correctness: false,
            feedback_text: false,
            solution: false,
            class_statistics: true,
        }
    }

    fn content() -> FeedbackContent {
        FeedbackContent {
            hint: Some(vec![ContentBlock::Text {
                markdown: "Hint".to_string(),
            }]),
            rationale: Some(vec![ContentBlock::Text {
                markdown: "Why".to_string(),
            }]),
            correct_response: Some(vec![ContentBlock::Text {
                markdown: "Answer".to_string(),
            }]),
        }
    }

    fn result() -> AttemptResult {
        AttemptResult {
            correct: false,
            points_earned: 0.0,
            points_possible: 2.0,
        }
    }

    #[test]
    fn fields_are_independently_allowlisted() {
        let mut allowed = decision();
        allowed.per_item_correctness = true;
        let projected = project_feedback(allowed, Some(result()), &content()).expect("visible");
        assert_eq!(projected.correctness, Some(false));
        assert_eq!(projected.points_earned, None);
        assert_eq!(projected.hint, None);
        assert_eq!(projected.correct_response, None);

        allowed = decision();
        allowed.feedback_text = true;
        let projected = project_feedback(allowed, Some(result()), &content()).expect("visible");
        assert!(projected.hint.is_some());
        assert!(projected.rationale.is_some());
        assert_eq!(projected.correctness, None);
        assert_eq!(projected.correct_response, None);

        allowed = decision();
        allowed.solution = true;
        let projected = project_feedback(allowed, Some(result()), &content()).expect("visible");
        assert!(projected.correct_response.is_some());
        assert_eq!(projected.hint, None);
    }

    #[test]
    fn class_statistics_has_no_feedback_projection() {
        assert!(project_feedback(decision(), Some(result()), &content()).is_none());
    }
}
