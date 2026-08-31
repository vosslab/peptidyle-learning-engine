//! Response types: what a student submits, and what shape it must take
//! (WP-C1).
//!
//! [`QuestionResponseFormat`] and [`StudentResponse`] are parallel enums. Each
//! definition variant has exactly one response variant that fits it.
//! `domain::validation` checks the pairing and the variant-specific structural
//! rules identically on the server and in the browser.

use serde::{Deserialize, Serialize};

use crate::answer::{NumericResponseTolerance, ResponseSelectionRule, TextResponseMatchRule};
use crate::envelope::{AssetRef, ContentBlock};

/// The educational interaction a Question assesses.
///
/// Question Type is independent of Question Format, Question Backend, and the
/// browser control used to collect a Student Response. In particular, an
/// external tool or file-upload control still declares the educational type it
/// serves rather than becoming a type itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionType {
    /// MC: exactly one selected Question Choice.
    MultipleChoice,
    /// MA: one or more selected Question Choices.
    MultipleAnswer,
    /// FIB: one short text entry.
    FillInBlank,
    /// MULTI-FIB: several named short text entries.
    MultipleFillInBlank,
    /// NUM: one numeric entry.
    Numeric,
    /// MATCH: Matching Prompts paired to Matching Choices.
    Matching,
    /// ORDER: an ordered sequence of response items.
    Ordering,
    /// HOTSPOT: points or regions selected on an image-backed surface.
    Hotspot,
}

impl QuestionType {
    /// Every educational Question Type supported by this release.
    pub const ALL: [Self; 8] = [
        Self::MultipleChoice,
        Self::MultipleAnswer,
        Self::FillInBlank,
        Self::MultipleFillInBlank,
        Self::Numeric,
        Self::Matching,
        Self::Ordering,
        Self::Hotspot,
    ];
}

/// The browser interaction used to collect a Student Response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionResponseControl {
    ChoiceSelection,
    TextEntry,
    Matching,
    Ordering,
    Hotspot,
    FileUpload,
    ExternalTool,
}

/// Identifies one response item within a Question Response Format.
///
/// Response Item References are opaque strings assigned by the authoring
/// backend. Grading compares the exact reference rather than displayed labels,
/// so presentation ordering leaves a submitted response meaningful.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResponseItemReference(String);

impl ResponseItemReference {
    /// Wraps a backend-assigned identifier.
    pub fn new(value: impl Into<String>) -> Self {
        ResponseItemReference(value.into())
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
    pub id: ResponseItemReference,
    /// What the student sees, in render order.
    pub body: Vec<ContentBlock>,
}

/// One named text-entry slot in a multi-blank question.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextEntrySlot {
    /// Stable semantic slot identifier.
    pub id: ResponseItemReference,
    /// Student-visible label or surrounding prompt fragment.
    pub label: Vec<ContentBlock>,
    /// How the server compares this slot's text.
    pub match_mode: TextResponseMatchRule,
    /// Longest accepted response, in characters.
    pub max_length: u32,
}

/// One student-supplied value for a named text-entry slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentTextEntry {
    /// Slot being answered.
    pub slot: ResponseItemReference,
    /// Student text before server-owned normalization.
    pub text: String,
}

/// One prompt-to-choice association in a matching response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentMatch {
    /// Prompt being matched.
    pub prompt: ResponseItemReference,
    /// Choice assigned to that prompt.
    pub choice: ResponseItemReference,
}

/// A scale-independent point on a hotspot surface.
///
/// Coordinates use the inclusive integer range 0 through 10,000 rather than
/// browser pixels. This keeps submissions stable across zoom, responsive
/// layout, and image density.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentHotspotPoint {
    /// Horizontal normalized coordinate.
    pub x: u16,
    /// Vertical normalized coordinate.
    pub y: u16,
}

/// One public candidate region and its accessible student label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HotspotRegion {
    /// Stable semantic region identifier. Correctness remains server-only.
    pub id: ResponseItemReference,
    /// Nonvisual alternative used by the keyboard-first response control.
    pub label: Vec<ContentBlock>,
    /// Left edge in normalized coordinates.
    pub x: u16,
    /// Top edge in normalized coordinates.
    pub y: u16,
    /// Width in normalized coordinates.
    pub width: u16,
    /// Height in normalized coordinates.
    pub height: u16,
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
pub enum QuestionResponseFormat {
    /// A number, compared within a tolerance.
    Numeric {
        /// How close the response must be.
        tolerance: NumericResponseTolerance,
        /// Expected unit, shown to the student, for example `mL`.
        unit: Option<String>,
    },
    /// A selection from a fixed list.
    MultipleChoice {
        /// The choices, in authoring order.
        choices: Vec<ChoiceOption>,
        /// How many may be selected.
        selection: ResponseSelectionRule,
    },
    /// A short free-text answer.
    ShortText {
        /// How the text is compared.
        match_mode: TextResponseMatchRule,
        /// Longest accepted response, in characters.
        max_length: u32,
    },
    /// Several independently identified short-text entries.
    MultiBlank {
        /// Slots in student presentation order.
        blanks: Vec<TextEntrySlot>,
    },
    /// A set of prompt-to-choice associations.
    Matching {
        /// Prompts in student presentation order.
        prompts: Vec<ChoiceOption>,
        /// Available choices in student presentation order.
        choices: Vec<ChoiceOption>,
    },
    /// An arrangement of items into the correct order.
    Ordering {
        /// The items to arrange, in their presented order.
        items: Vec<ChoiceOption>,
    },
    /// One or more points selected on an image-backed surface.
    Hotspot {
        /// Immutable image used as the coordinate surface.
        surface: AssetRef,
        /// Text alternative describing the whole surface.
        description: String,
        /// Public candidate regions; the correct region set remains private.
        regions: Vec<HotspotRegion>,
        /// Number of points the student must select.
        selection: ResponseSelectionRule,
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

impl QuestionResponseFormat {
    /// Returns the browser control required to collect this response shape.
    pub const fn control(&self) -> QuestionResponseControl {
        match self {
            Self::Numeric { .. } | Self::ShortText { .. } | Self::MultiBlank { .. } => {
                QuestionResponseControl::TextEntry
            }
            Self::MultipleChoice { .. } => QuestionResponseControl::ChoiceSelection,
            Self::Matching { .. } => QuestionResponseControl::Matching,
            Self::Ordering { .. } => QuestionResponseControl::Ordering,
            Self::Hotspot { .. } => QuestionResponseControl::Hotspot,
            Self::FileUpload { .. } => QuestionResponseControl::FileUpload,
            Self::ExternalTool {} => QuestionResponseControl::ExternalTool,
        }
    }

    /// Whether this response shape can collect work for the declared Question Type.
    ///
    /// File Upload and External Tool are controls, so they remain compatible with
    /// the separately declared educational Question Type.
    pub const fn supports_question_type(&self, question_type: QuestionType) -> bool {
        match self {
            Self::Numeric { .. } => matches!(question_type, QuestionType::Numeric),
            Self::MultipleChoice { selection, .. } => match selection {
                ResponseSelectionRule::ExactlyOne => {
                    matches!(question_type, QuestionType::MultipleChoice)
                }
                _ => matches!(question_type, QuestionType::MultipleAnswer),
            },
            Self::ShortText { .. } => matches!(question_type, QuestionType::FillInBlank),
            Self::MultiBlank { .. } => matches!(question_type, QuestionType::MultipleFillInBlank),
            Self::Matching { .. } => matches!(question_type, QuestionType::Matching),
            Self::Ordering { .. } => matches!(question_type, QuestionType::Ordering),
            Self::Hotspot { .. } => matches!(question_type, QuestionType::Hotspot),
            Self::FileUpload { .. } | Self::ExternalTool {} => true,
        }
    }
}

/// What a student submitted.
///
/// Each variant matches one [`QuestionResponseFormat`] variant. Pairing and format
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
        selected: Vec<ResponseItemReference>,
    },
    /// Free text, as typed, before normalization.
    ShortText {
        /// The text the student entered.
        text: String,
    },
    /// Values supplied for named blanks.
    MultiBlank {
        /// Answers in issued slot order.
        answers: Vec<StudentTextEntry>,
    },
    /// Prompt-to-choice associations.
    Matching {
        /// Matches in issued prompt order.
        matches: Vec<StudentMatch>,
    },
    /// Items in the order the student arranged them.
    Ordering {
        /// Response Item References, first to last.
        order: Vec<ResponseItemReference>,
    },
    /// Normalized points selected on a hotspot surface.
    Hotspot {
        /// Points in student selection order.
        points: Vec<StudentHotspotPoint>,
    },
    /// A reference to an uploaded object in the `student-records` bucket.
    FileUpload {
        /// Storage key of the uploaded object.
        object_key: String,
    },
    /// The student used the ordinary submission action for an external tool.
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
            selected: vec![ResponseItemReference::new("b"), ResponseItemReference::new("d")],
        };
        let json = serde_json::to_string(&response).expect("serialization should succeed");
        let restored: StudentResponse =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(restored, response);
    }

    #[test]
    fn external_tool_markers_round_trip_as_kind_only() {
        let definition = QuestionResponseFormat::ExternalTool {};
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
            serde_json::from_value::<QuestionResponseFormat>(
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
            assert!(serde_json::from_value::<QuestionResponseFormat>(value.clone()).is_err());
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
