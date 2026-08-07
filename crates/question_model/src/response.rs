//! Response types: what a student submits, and what shape it must take
//! (WP-C1).
//!
//! [`ResponseDefinition`] and [`StudentResponse`] are parallel enums. Each
//! definition variant has exactly one response variant that fits it, so a
//! numeric response paired with a multiple-choice question is a shape mismatch
//! the server rejects before grading, and the browser catches locally without
//! issuing a request.

use serde::{Deserialize, Serialize};

use crate::answer::{NumericTolerance, SelectionCardinality, TextMatchMode};
use crate::envelope::ContentBlock;

/// Identifies one selectable choice within a question.
///
/// Choice identifiers are opaque strings assigned by the authoring backend.
/// Grading compares identifiers rather than displayed labels, so shuffling the
/// presentation order leaves a submitted response meaningful.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChoiceId(String);

impl ChoiceId {
    /// Wraps a backend-assigned identifier.
    pub fn new(value: impl Into<String>) -> Self {
        ChoiceId(value.into())
    }

    /// The identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One option a student can pick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    /// Stable identifier, used by grading.
    pub id: ChoiceId,
    /// What the student sees, in render order.
    pub body: Vec<ContentBlock>,
}

/// The shape of response a question expects.
///
/// Every variant carries the information a widget needs to render an input and
/// validate its shape locally. None of it reveals a correct answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ResponseDefinition {
    /// A number, compared within a tolerance.
    Numeric {
        /// How close the response must be.
        tolerance: NumericTolerance,
        /// Expected unit, shown to the student, for example `mL`.
        unit: Option<String>,
    },
    /// A selection from a fixed list.
    MultipleChoice {
        /// The choices, in authoring order.
        choices: Vec<ChoiceOption>,
        /// How many may be selected.
        selection: SelectionCardinality,
    },
    /// A short free-text answer.
    ShortText {
        /// How the text is compared.
        match_mode: TextMatchMode,
        /// Longest accepted response, in characters.
        max_length: u32,
    },
    /// An arrangement of items into the correct order.
    Ordering {
        /// The items to arrange, in their presented order.
        items: Vec<ChoiceOption>,
    },
    /// An uploaded file, for work done outside the browser.
    FileUpload {
        /// Largest accepted upload, in bytes.
        max_bytes: u64,
        /// Accepted extensions, lowercase and without a leading dot.
        accepted_extensions: Vec<String>,
    },
}

/// What a student submitted.
///
/// Each variant matches one [`ResponseDefinition`] variant. Pairing them is
/// checked once, in `crates/domain`, and both the browser and the server run
/// that same check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StudentResponse {
    /// A numeric entry, as typed, before tolerance is applied.
    Numeric {
        /// The value the student entered.
        value: f64,
    },
    /// Selected choices, identified rather than positional.
    MultipleChoice {
        /// Identifiers of the selected choices.
        selected: Vec<ChoiceId>,
    },
    /// Free text, as typed, before normalization.
    ShortText {
        /// The text the student entered.
        text: String,
    },
    /// Items in the order the student arranged them.
    Ordering {
        /// Choice identifiers, first to last.
        order: Vec<ChoiceId>,
    },
    /// A reference to an uploaded object in the `student-records` bucket.
    FileUpload {
        /// Storage key of the uploaded object.
        object_key: String,
    },
}

impl StudentResponse {
    /// Whether this response has the shape the definition expects.
    ///
    /// Shape agreement only. Correctness is a separate question, answered
    /// server-side by `crates/grading` where answer keys live, which is why
    /// this check is safe to run in a browser.
    ///
    /// # Examples
    ///
    /// ```
    /// use question_model::answer::TextMatchMode;
    /// use question_model::response::{ResponseDefinition, StudentResponse};
    ///
    /// let definition = ResponseDefinition::ShortText {
    ///     match_mode: TextMatchMode::Normalized,
    ///     max_length: 40,
    /// };
    /// let response = StudentResponse::ShortText { text: "mitochondria".to_string() };
    /// assert!(response.matches_shape(&definition));
    ///
    /// let wrong_shape = StudentResponse::Numeric { value: 1.0 };
    /// assert!(!wrong_shape.matches_shape(&definition));
    /// ```
    pub fn matches_shape(&self, definition: &ResponseDefinition) -> bool {
        matches!(
            (self, definition),
            (
                StudentResponse::Numeric { .. },
                ResponseDefinition::Numeric { .. }
            ) | (
                StudentResponse::MultipleChoice { .. },
                ResponseDefinition::MultipleChoice { .. }
            ) | (
                StudentResponse::ShortText { .. },
                ResponseDefinition::ShortText { .. }
            ) | (
                StudentResponse::Ordering { .. },
                ResponseDefinition::Ordering { .. }
            ) | (
                StudentResponse::FileUpload { .. },
                ResponseDefinition::FileUpload { .. }
            )
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numeric_definition() -> ResponseDefinition {
        ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Relative { fraction: 0.01 },
            unit: Some("mL".to_string()),
        }
    }

    #[test]
    fn a_matching_pair_agrees_on_shape() {
        let response = StudentResponse::Numeric { value: 12.5 };
        assert!(response.matches_shape(&numeric_definition()));
    }

    #[test]
    fn a_mismatched_pair_is_rejected_by_shape() {
        let response = StudentResponse::ShortText {
            text: "12.5".to_string(),
        };
        assert!(!response.matches_shape(&numeric_definition()));
    }

    #[test]
    fn choice_identifiers_survive_a_round_trip() {
        let response = StudentResponse::MultipleChoice {
            selected: vec![ChoiceId::new("b"), ChoiceId::new("d")],
        };
        let json = serde_json::to_string(&response).expect("serialization should succeed");
        let restored: StudentResponse =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(restored, response);
    }
}
