//! Server-owned teaching feedback and its browser-safe projection.
//!
//! [`FeedbackContent`] is deliberately not serializable. It belongs only to a
//! trusted backend and, once persistence lands, to tenant-owned private
//! storage. In contrast, [`DisclosedFeedback`] is the small, policy-redacted
//! response DTO. Keeping the two shapes separate makes it impossible for the
//! TypeScript generator to accidentally add private teaching material to the
//! public question model.

use serde::{Deserialize, Serialize};

use crate::envelope::ContentBlock;

/// Trusted, sanitized teaching material held behind the server boundary.
///
/// This is intentionally neither `Debug`, `Serialize`, nor `Deserialize`: it
/// must not reach logs or become a browser DTO merely because the root model
/// is scanned for public wire types. Store implementations will own their
/// private persistence representation in the next work package.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct FeedbackContent {
    /// A teaching hint that does not reveal the answer by itself.
    pub hint: Option<Vec<ContentBlock>>,
    /// A rendered, accessible correct-response explanation, never an answer key.
    pub correct_response: Option<Vec<ContentBlock>>,
    /// A rendered explanation of why the response is correct.
    pub rationale: Option<Vec<ContentBlock>>,
}

/// Browser-safe feedback after the server has applied its disclosure policy.
///
/// Absent fields are omitted from JSON, rather than sent as hidden `null`
/// values. This lets strict client decoders prove that a policy did not merely
/// hide restricted material in the interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisclosedFeedback {
    /// Whether the server judged the submitted response correct.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correctness: Option<bool>,
    /// Points awarded, where the policy permits score disclosure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_earned: Option<f64>,
    /// Points available, where the policy permits score disclosure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_possible: Option<f64>,
    /// A server-sanitized hint, rendered as accessible content blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<Vec<ContentBlock>>,
    /// A server-sanitized correct-response explanation, not a raw answer key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct_response: Option<Vec<ContentBlock>>,
    /// A server-sanitized teaching rationale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<Vec<ContentBlock>>,
}

impl DisclosedFeedback {
    /// Makes the empty public projection used only for an unlocked ungraded
    /// backend response. Graded paths populate the permitted result fields.
    pub fn empty() -> Self {
        Self {
            correctness: None,
            points_earned: None,
            points_possible: None,
            hint: None,
            correct_response: None,
            rationale: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undisclosed_fields_are_omitted_not_null() {
        let json = serde_json::to_value(DisclosedFeedback::empty())
            .expect("the public feedback DTO should serialize");
        assert_eq!(json, serde_json::json!({}));
    }
}
