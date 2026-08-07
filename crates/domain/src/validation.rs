//! Browser-safe student-response format validation (WP-C6, MOD-GRD boundary).
//!
//! This module can inspect response definitions and student input, but it has
//! no answer key and makes no correctness decision. The server repeats the
//! same validation before calling the server-only `grading` crate.

use std::collections::BTreeSet;

use question_model::answer::SelectionCardinality;
use question_model::response::{ChoiceId, ResponseDefinition, StudentResponse};
use serde::{Deserialize, Serialize};

/// One reason a student response cannot be submitted in its current form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ResponseFormatViolation {
    /// The response kind does not match the question's response definition.
    ResponseKindMismatch,
    /// A numeric response is NaN or infinite.
    NumericNotFinite,
    /// The number of selected choices violates the declared cardinality.
    SelectionCount {
        /// Cardinality declared by the question.
        expected: SelectionCardinality,
        /// Number of choice IDs in the submitted response.
        actual: u64,
    },
    /// A choice ID appears more than once in one response.
    DuplicateChoice {
        /// Repeated choice identifier.
        choice: ChoiceId,
    },
    /// A submitted choice ID does not occur in the question definition.
    UnknownChoice {
        /// Unrecognized choice identifier.
        choice: ChoiceId,
    },
    /// Short text exceeds the question's character limit.
    TextTooLong {
        /// Maximum allowed Unicode scalar values.
        max_length: u32,
        /// Submitted Unicode scalar values.
        actual_length: u64,
    },
    /// An ordering response is not an exact permutation of the defined items.
    OrderingItemsMismatch,
    /// A file-upload response does not contain its server-issued object key.
    MissingUploadReference,
}

/// Complete browser-safe format verdict for one student response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseFormatReport {
    /// Every format problem found, in stable validation order.
    pub violations: Vec<ResponseFormatViolation>,
}

impl ResponseFormatReport {
    /// Whether the response is structurally ready for server submission.
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Validates student-controlled response structure without consulting a key.
///
/// The server runs this function before grading. The browser calls the same
/// implementation through `wasm_bridge` for immediate format feedback.
pub fn validate_response_format(
    definition: &ResponseDefinition,
    response: &StudentResponse,
) -> ResponseFormatReport {
    let mut violations = Vec::new();

    match (definition, response) {
        (ResponseDefinition::Numeric { .. }, StudentResponse::Numeric { value }) => {
            if !value.is_finite() {
                violations.push(ResponseFormatViolation::NumericNotFinite);
            }
        }
        (
            ResponseDefinition::MultipleChoice { choices, selection },
            StudentResponse::MultipleChoice { selected },
        ) => validate_selection(choices, *selection, selected, &mut violations),
        (ResponseDefinition::ShortText { max_length, .. }, StudentResponse::ShortText { text }) => {
            let actual_length = count(text.chars());
            if actual_length > u64::from(*max_length) {
                violations.push(ResponseFormatViolation::TextTooLong {
                    max_length: *max_length,
                    actual_length,
                });
            }
        }
        (ResponseDefinition::Ordering { items }, StudentResponse::Ordering { order }) => {
            let expected: BTreeSet<ChoiceId> = items.iter().map(|item| item.id.clone()).collect();
            let actual: BTreeSet<ChoiceId> = order.iter().cloned().collect();
            if expected.len() != items.len()
                || actual.len() != order.len()
                || order.len() != items.len()
                || actual != expected
            {
                violations.push(ResponseFormatViolation::OrderingItemsMismatch);
            }
        }
        (ResponseDefinition::FileUpload { .. }, StudentResponse::FileUpload { object_key }) => {
            if object_key.trim().is_empty() {
                violations.push(ResponseFormatViolation::MissingUploadReference);
            }
        }
        _ => violations.push(ResponseFormatViolation::ResponseKindMismatch),
    }

    ResponseFormatReport { violations }
}

/// Validates multiple-choice cardinality and identifier membership.
fn validate_selection(
    choices: &[question_model::response::ChoiceOption],
    selection: SelectionCardinality,
    selected: &[ChoiceId],
    violations: &mut Vec<ResponseFormatViolation>,
) {
    let actual = count(selected.iter());
    let cardinality_matches = match selection {
        SelectionCardinality::ExactlyOne => actual == 1,
        SelectionCardinality::Exactly { count } => actual == u64::from(count),
        SelectionCardinality::AnyNumber => true,
        SelectionCardinality::AtLeastOne => actual >= 1,
    };
    if !cardinality_matches {
        violations.push(ResponseFormatViolation::SelectionCount {
            expected: selection,
            actual,
        });
    }

    let available: BTreeSet<ChoiceId> = choices.iter().map(|choice| choice.id.clone()).collect();
    let mut observed = BTreeSet::new();
    for choice in selected {
        if !observed.insert(choice.clone()) {
            violations.push(ResponseFormatViolation::DuplicateChoice {
                choice: choice.clone(),
            });
        }
        if !available.contains(choice) {
            violations.push(ResponseFormatViolation::UnknownChoice {
                choice: choice.clone(),
            });
        }
    }
}

/// Converts an iterator count without target-width differences in the report.
fn count(values: impl Iterator) -> u64 {
    u64::try_from(values.count()).expect("supported usize values fit u64")
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::answer::{NumericTolerance, TextMatchMode};
    use question_model::response::ChoiceOption;

    fn choice(id: &str) -> ChoiceOption {
        ChoiceOption {
            id: ChoiceId::new(id),
            body: Vec::new(),
        }
    }

    #[test]
    fn a_kind_mismatch_stops_before_answer_adjacent_checks() {
        let definition = ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Exact,
            unit: None,
        };
        let response = StudentResponse::ShortText {
            text: "12".to_string(),
        };

        assert_eq!(
            validate_response_format(&definition, &response).violations,
            vec![ResponseFormatViolation::ResponseKindMismatch]
        );
    }

    #[test]
    fn numeric_input_must_be_finite() {
        let definition = ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Absolute { epsilon: 0.1 },
            unit: None,
        };

        assert_eq!(
            validate_response_format(&definition, &StudentResponse::Numeric { value: f64::NAN })
                .violations,
            vec![ResponseFormatViolation::NumericNotFinite]
        );
    }

    #[test]
    fn selection_reports_count_duplicates_and_unknown_ids() {
        let definition = ResponseDefinition::MultipleChoice {
            choices: vec![choice("a"), choice("b")],
            selection: SelectionCardinality::ExactlyOne,
        };
        let response = StudentResponse::MultipleChoice {
            selected: vec![ChoiceId::new("b"), ChoiceId::new("b"), ChoiceId::new("z")],
        };

        assert_eq!(
            validate_response_format(&definition, &response).violations,
            vec![
                ResponseFormatViolation::SelectionCount {
                    expected: SelectionCardinality::ExactlyOne,
                    actual: 3,
                },
                ResponseFormatViolation::DuplicateChoice {
                    choice: ChoiceId::new("b"),
                },
                ResponseFormatViolation::UnknownChoice {
                    choice: ChoiceId::new("z"),
                },
            ]
        );
    }

    #[test]
    fn short_text_counts_characters_instead_of_utf8_bytes() {
        let definition = ResponseDefinition::ShortText {
            match_mode: TextMatchMode::Normalized,
            max_length: 2,
        };
        let response = StudentResponse::ShortText {
            text: "\u{03b1}\u{03b2}".to_string(),
        };

        assert!(validate_response_format(&definition, &response).is_valid());
    }

    #[test]
    fn violation_json_uses_the_browser_camel_case_contract() {
        let report = ResponseFormatReport {
            violations: vec![ResponseFormatViolation::TextTooLong {
                max_length: 2,
                actual_length: 3,
            }],
        };

        assert_eq!(
            serde_json::to_value(report).expect("format report should serialize"),
            serde_json::json!({
                "violations": [{
                    "kind": "textTooLong",
                    "maxLength": 2,
                    "actualLength": 3
                }]
            })
        );
    }

    #[test]
    fn ordering_requires_each_defined_item_exactly_once() {
        let definition = ResponseDefinition::Ordering {
            items: vec![choice("first"), choice("second")],
        };
        let response = StudentResponse::Ordering {
            order: vec![ChoiceId::new("first"), ChoiceId::new("first")],
        };

        assert_eq!(
            validate_response_format(&definition, &response).violations,
            vec![ResponseFormatViolation::OrderingItemsMismatch]
        );
    }

    #[test]
    fn file_upload_requires_a_server_issued_object_reference() {
        let definition = ResponseDefinition::FileUpload {
            max_bytes: 10,
            accepted_extensions: vec!["pdf".to_string()],
        };
        let response = StudentResponse::FileUpload {
            object_key: "  ".to_string(),
        };

        assert_eq!(
            validate_response_format(&definition, &response).violations,
            vec![ResponseFormatViolation::MissingUploadReference]
        );
    }
}
