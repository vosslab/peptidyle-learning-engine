//! Server-owned Question feedback, Question hints, and browser-safe feedback.
//!
//! [`QuestionFeedback`] is deliberately not serializable. It belongs only to a
//! trusted Question backend and, once persistence lands, to Question-owned
//! private storage. In contrast, [`StudentFeedback`] is the small, policy-redacted
//! response DTO. Keeping the two shapes separate makes it impossible for the
//! TypeScript generator to accidentally add private teaching material to the
//! public question model.

use serde::{Deserialize, Serialize};

use crate::envelope::ContentBlock;

/// Trusted Question-attached feedback selected after automatic grading.
///
/// This is intentionally neither `Debug`, `Serialize`, nor `Deserialize`: it
/// must not reach logs or become a browser DTO merely because the root model
/// is scanned for public wire types. Store implementations will own their
/// private persistence representation in the next work package.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct QuestionFeedback {
    /// Feedback authored for the selected response item or items.
    pub choice_feedback: Option<Vec<ContentBlock>>,
    /// Feedback authored for a correct automatically graded response.
    pub correct_feedback: Option<Vec<ContentBlock>>,
    /// Feedback authored for an incorrect automatically graded response.
    pub incorrect_feedback: Option<Vec<ContentBlock>>,
    /// A rendered, accessible correct-response explanation.
    pub correct_response: Option<Vec<ContentBlock>>,
    /// A rationale that may contain answer-bearing instructional content.
    pub rationale: Option<Vec<ContentBlock>>,
}

/// Trusted Question-attached instructional support requested before a response.
///
/// A Question Hint is separate from [`QuestionFeedback`]: it is selected before
/// grading and therefore cannot be an outcome, selected-choice response, or
/// released Student feedback field.
#[derive(Clone, PartialEq, Eq)]
pub struct QuestionHint {
    content: Vec<ContentBlock>,
}

impl QuestionHint {
    /// Creates one non-empty, server-owned Question Hint.
    pub fn new(content: Vec<ContentBlock>) -> Option<Self> {
        (!content.is_empty()).then_some(Self { content })
    }

    /// Returns the rendered instructional content for the authorized hint path.
    pub fn content(&self) -> &[ContentBlock] {
        &self.content
    }
}

/// Browser-safe feedback after the server has applied its disclosure policy.
///
/// Absent fields are omitted from JSON, rather than sent as hidden `null`
/// values. This lets strict client decoders prove that a policy did not merely
/// hide restricted material in the interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentFeedback {
    /// Whether the server judged the submitted response correct.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correctness: Option<bool>,
    /// Points awarded, where the policy permits score disclosure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_earned: Option<f64>,
    /// Points available, where the policy permits score disclosure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_possible: Option<f64>,
    /// Released feedback for the selected response item or items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choice_feedback: Option<Vec<ContentBlock>>,
    /// Released feedback for a correct automatically graded response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct_feedback: Option<Vec<ContentBlock>>,
    /// Released feedback for an incorrect automatically graded response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incorrect_feedback: Option<Vec<ContentBlock>>,
    /// A server-sanitized correct-response explanation, not a raw answer key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct_response: Option<Vec<ContentBlock>>,
    /// A server-sanitized teaching rationale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<Vec<ContentBlock>>,
}

impl StudentFeedback {
    /// Makes the empty public projection used only for an unlocked ungraded
    /// backend response. Graded paths populate the permitted result fields.
    pub fn empty() -> Self {
        Self {
            correctness: None,
            points_earned: None,
            points_possible: None,
            choice_feedback: None,
            correct_feedback: None,
            incorrect_feedback: None,
            correct_response: None,
            rationale: None,
        }
    }
}

/// Closed score-only feedback for an audited Instructor Student-work read.
///
/// This type deliberately cannot carry Question Hints, rationale, correct
/// responses, or any other instructional material.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentResponseInspectionFeedback {
    /// Current correctness verdict when disclosure permits it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correctness: Option<bool>,
    /// Current earned points when disclosure permits it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_earned: Option<f64>,
    /// Current possible points when disclosure permits it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_possible: Option<f64>,
}

impl StudentResponseInspectionFeedback {
    /// Returns the no-score/no-correctness inspection projection.
    pub const fn empty() -> Self {
        Self {
            correctness: None,
            points_earned: None,
            points_possible: None,
        }
    }
}

impl std::fmt::Debug for StudentResponseInspectionFeedback {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudentResponseInspectionFeedback")
            .field("score", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undisclosed_fields_are_omitted_not_null() {
        let json = serde_json::to_value(StudentFeedback::empty())
            .expect("the public feedback DTO should serialize");
        assert_eq!(json, serde_json::json!({}));
    }

    #[test]
    fn inspected_score_feedback_debug_is_redacted() {
        let feedback = StudentResponseInspectionFeedback {
            correctness: Some(true),
            points_earned: Some(17.25),
            points_possible: Some(20.0),
        };
        let rendered = format!("{feedback:?}");
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains("17.25"));
        assert!(!rendered.contains("true"));
    }
}
