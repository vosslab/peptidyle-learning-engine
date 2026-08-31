//! Server-owned Question teaching content and browser-safe Student Feedback.
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
}

/// Trusted display-ready accepted response for one exact Question Variation.
///
/// The trusted Question Backend derives this from the Answer Key without
/// allowing answer-key facts to cross into the browser contract.
#[derive(Clone, PartialEq, Eq)]
pub struct QuestionAnswer {
    content: Vec<ContentBlock>,
}

impl QuestionAnswer {
    /// Creates one non-empty display-ready Question Answer.
    pub fn new(content: Vec<ContentBlock>) -> Option<Self> {
        (!content.is_empty()).then_some(Self { content })
    }

    /// Returns the display-ready content for an authorized release projection.
    pub fn content(&self) -> &[ContentBlock] {
        &self.content
    }
}

/// Trusted display-ready explanation of how or why one Question Answer is reached.
#[derive(Clone, PartialEq, Eq)]
pub struct QuestionAnswerExplanation {
    content: Vec<ContentBlock>,
}

impl QuestionAnswerExplanation {
    /// Creates one non-empty Question Answer Explanation.
    pub fn new(content: Vec<ContentBlock>) -> Option<Self> {
        (!content.is_empty()).then_some(Self { content })
    }

    /// Returns the display-ready content for an authorized release projection.
    pub fn content(&self) -> &[ContentBlock] {
        &self.content
    }
}

/// Complete trusted post-grading teaching output for one exact Question Variation.
///
/// Named fields keep outcome-selected Question Feedback, the accepted Question
/// Answer, and its optional Question Answer Explanation independently owned.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct QuestionPostGradingContent {
    /// Teaching content selected by the Student's graded response.
    pub question_feedback: QuestionFeedback,
    /// Display-ready accepted response derived from the Answer Key.
    pub question_answer: Option<QuestionAnswer>,
    /// Optional explanation of the Question Answer.
    pub question_answer_explanation: Option<QuestionAnswerExplanation>,
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
    /// Server-sanitized display-ready accepted response, never an Answer Key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question_answer: Option<Vec<ContentBlock>>,
    /// Server-sanitized explanation of the released Question Answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question_answer_explanation: Option<Vec<ContentBlock>>,
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
            question_answer: None,
            question_answer_explanation: None,
        }
    }
}

/// Closed score-only feedback for an audited Instructor Student-work read.
///
/// This type deliberately cannot carry Question Hints, Question Answer,
/// Question Answer Explanation, or any other instructional material.
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
