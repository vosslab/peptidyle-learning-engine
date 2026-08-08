//! Response types: what a student submits, and what shape it must take
//! (WP-C1).
//!
//! [`ResponseDefinition`] and [`StudentResponse`] are parallel enums. Each
//! definition variant has exactly one response variant that fits it.
//! `domain::validation` checks the pairing and the variant-specific structural
//! rules identically on the server and in the browser.

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
    rename_all_fields = "camelCase",
    deny_unknown_fields
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
    /// A server-brokered external learning tool.
    ///
    /// This marker deliberately contains no provider, launch, answer, score,
    /// token, or completion data. The server owns the later provider exchange.
    ExternalTool {},
}

/// What a student submitted.
///
/// Each variant matches one [`ResponseDefinition`] variant. Pairing and format
/// checks live in `crates/domain`, where browser and server use the same code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
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
    /// The learner used the ordinary submission action for an external tool.
    ///
    /// It is intentionally a marker only; browser-supplied provider material
    /// can never enter the generic submission record through this variant.
    ExternalTool {},
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn external_tool_markers_round_trip_as_kind_only() {
        let definition = ResponseDefinition::ExternalTool {};
        let response = StudentResponse::ExternalTool {};

        assert_eq!(
            serde_json::to_value(&definition).unwrap(),
            serde_json::json!({"kind": "externalTool"})
        );
        assert_eq!(
            serde_json::to_value(&response).unwrap(),
            serde_json::json!({"kind": "externalTool"})
        );
        assert_eq!(
            serde_json::from_value::<ResponseDefinition>(
                serde_json::json!({"kind": "externalTool"})
            )
            .unwrap(),
            definition
        );
        assert_eq!(
            serde_json::from_value::<StudentResponse>(serde_json::json!({"kind": "externalTool"}))
                .unwrap(),
            response
        );
    }

    #[test]
    fn response_enums_reject_unknown_fields() {
        for extra in [
            "score",
            "correct",
            "result",
            "provider",
            "token",
            "launchUrl",
        ] {
            let value = serde_json::json!({"kind": "externalTool", extra: true});
            assert!(serde_json::from_value::<ResponseDefinition>(value.clone()).is_err());
            assert!(serde_json::from_value::<StudentResponse>(value).is_err());
        }

        assert!(
            serde_json::from_value::<StudentResponse>(serde_json::json!({
                "kind": "numeric", "value": 1.0, "score": 1
            }))
            .is_err()
        );
    }
}
