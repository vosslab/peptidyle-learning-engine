//! Browser-safe student-response format validation (WP-C6, MOD-GRD boundary).
//!
//! This module can inspect Question Response Formats and Student input, but it has
//! no answer key and makes no correctness decision. The server repeats the
//! same validation before calling the server-only `grading` crate.

use std::collections::BTreeSet;

use question_model::answer::ResponseSelectionRule;
use question_model::presentation::{
    PresentedHotspotRegion, PresentedResponseItemContent, PresentedTextEntrySlot,
    QuestionPresentationResponseFormat,
};
use question_model::response::{
    HotspotRegion, QuestionResponseFormat, ResponseItemReference, StudentResponse, TextEntrySlot,
};
use serde::{Deserialize, Serialize};

/// One reason a student response cannot be submitted in its current form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StudentResponseFormatIssue {
    /// The response kind does not match the Question Response Format.
    ResponseKindMismatch,
    /// A numeric response is NaN or infinite.
    NumericNotFinite,
    /// The number of selected response items violates the declared Response Selection Rule.
    SelectionCount {
        /// Response Selection Rule declared by the question.
        expected: ResponseSelectionRule,
        /// Number of selected Response Item References in the submitted response.
        actual: u64,
    },
    /// A selected Response Item Reference appears more than once in one response.
    DuplicateChoice {
        /// Repeated Response Item Reference.
        choice: ResponseItemReference,
    },
    /// A submitted Response Item Reference does not occur in the Question Revision.
    UnknownChoice {
        /// Unrecognized Response Item Reference.
        choice: ResponseItemReference,
    },
    /// Short text exceeds the question's character limit.
    TextTooLong {
        /// Maximum allowed Unicode scalar values.
        max_length: u32,
        /// Submitted Unicode scalar values.
        actual_length: u64,
    },
    /// A multi-blank response does not name every declared slot exactly once.
    BlankSlotsMismatch,
    /// A matching response does not name every prompt exactly once.
    MatchingPromptsMismatch,
    /// A matching response repeats a choice where the definition requires a permutation.
    DuplicateMatchChoice {
        /// Reused Matching Choice reference.
        choice: ResponseItemReference,
    },
    /// A matching response names a choice absent from the definition.
    UnknownMatchChoice {
        /// Unrecognized Matching Choice reference.
        choice: ResponseItemReference,
    },
    /// An ordering response is not an exact permutation of the defined items.
    OrderingItemsMismatch,
    /// A Hotspot Region appears more than once in one response.
    DuplicateHotspotRegion {
        /// Repeated Hotspot Region reference.
        region: ResponseItemReference,
    },
    /// A Student Hotspot Selection names a region absent from the Question Response Format.
    UnknownHotspotRegion {
        /// Unrecognized Hotspot Region reference.
        region: ResponseItemReference,
    },
}

/// Complete browser-safe format verdict for one student response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentResponseFormatCheck {
    /// Every format problem found, in stable validation order.
    pub violations: Vec<StudentResponseFormatIssue>,
}

impl StudentResponseFormatCheck {
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
    definition: &QuestionResponseFormat,
    response: &StudentResponse,
) -> StudentResponseFormatCheck {
    let mut violations = Vec::new();

    match (definition, response) {
        (QuestionResponseFormat::Numeric { .. }, StudentResponse::Numeric { value }) => {
            if !value.is_finite() {
                violations.push(StudentResponseFormatIssue::NumericNotFinite);
            }
        }
        (
            QuestionResponseFormat::MultipleChoice { choices, selection },
            StudentResponse::MultipleChoice { selected },
        ) => validate_selection(choices, *selection, selected, &mut violations),
        (
            QuestionResponseFormat::ShortText { max_length, .. },
            StudentResponse::ShortText { text },
        ) => {
            let actual_length = count(text.chars());
            if actual_length > u64::from(*max_length) {
                violations.push(StudentResponseFormatIssue::TextTooLong {
                    max_length: *max_length,
                    actual_length,
                });
            }
        }
        (
            QuestionResponseFormat::MultiBlank { blanks },
            StudentResponse::MultiBlank { answers },
        ) => {
            validate_multi_blank(blanks, answers, &mut violations);
        }
        (
            QuestionResponseFormat::Matching { prompts, choices },
            StudentResponse::Matching { matches },
        ) => validate_matching(prompts, choices, matches, &mut violations),
        (QuestionResponseFormat::Ordering { items }, StudentResponse::Ordering { order }) => {
            let expected: BTreeSet<ResponseItemReference> =
                items.iter().map(|item| item.id.clone()).collect();
            let actual: BTreeSet<ResponseItemReference> = order.iter().cloned().collect();
            if expected.len() != items.len()
                || actual.len() != order.len()
                || order.len() != items.len()
                || actual != expected
            {
                violations.push(StudentResponseFormatIssue::OrderingItemsMismatch);
            }
        }
        (
            QuestionResponseFormat::Hotspot {
                regions, selection, ..
            },
            StudentResponse::Hotspot { selections },
        ) => validate_hotspot(regions, *selection, selections, &mut violations),
        (QuestionResponseFormat::ExternalTool {}, StudentResponse::ExternalTool {}) => {}
        _ => violations.push(StudentResponseFormatIssue::ResponseKindMismatch),
    }

    StudentResponseFormatCheck { violations }
}

/// Validates a student response against the answer-free schema frozen with an
/// issued presentation.
///
/// This is the server-side authority for a presentation-bearing attempt: it
/// lets a first submission reject malformed input without asking a mutable
/// Question Library or renderer to rebuild the student's already-issued widget. It
/// checks only public shape, never answer material or correctness.
pub fn validate_presentation_response_format(
    schema: &QuestionPresentationResponseFormat,
    response: &StudentResponse,
) -> StudentResponseFormatCheck {
    let mut violations = Vec::new();

    match (schema, response) {
        (
            QuestionPresentationResponseFormat::Numerical { .. },
            StudentResponse::Numeric { value },
        ) => {
            if !value.is_finite() {
                violations.push(StudentResponseFormatIssue::NumericNotFinite);
            }
        }
        (
            QuestionPresentationResponseFormat::SingleChoice { choices },
            StudentResponse::MultipleChoice { selected },
        ) => {
            validate_presented_selection(choices, 1, 1, selected, &mut violations);
        }
        (
            QuestionPresentationResponseFormat::MultipleAnswer {
                choices,
                minimum,
                maximum,
            },
            StudentResponse::MultipleChoice { selected },
        ) => validate_presented_selection(choices, *minimum, *maximum, selected, &mut violations),
        (
            QuestionPresentationResponseFormat::FillIn { max_characters },
            StudentResponse::ShortText { text },
        ) => {
            validate_text_length(*max_characters, text, &mut violations);
        }
        (
            QuestionPresentationResponseFormat::MultiFillIn { blanks },
            StudentResponse::MultiBlank { answers },
        ) => {
            validate_presented_multi_blank(blanks, answers, &mut violations);
        }
        (
            QuestionPresentationResponseFormat::Matching {
                prompts,
                choices,
                reuse_choices,
            },
            StudentResponse::Matching { matches },
        ) => {
            validate_presented_matching(prompts, choices, *reuse_choices, matches, &mut violations)
        }
        (
            QuestionPresentationResponseFormat::Ordering { items },
            StudentResponse::Ordering { order },
        ) => {
            let expected: BTreeSet<ResponseItemReference> = items
                .iter()
                .map(|item| ResponseItemReference::new(item.id.as_str()))
                .collect();
            let actual: BTreeSet<ResponseItemReference> = order.iter().cloned().collect();
            if expected.len() != items.len()
                || actual.len() != order.len()
                || order.len() != items.len()
                || actual != expected
            {
                violations.push(StudentResponseFormatIssue::OrderingItemsMismatch);
            }
        }
        (
            QuestionPresentationResponseFormat::Hotspot {
                surface,
                minimum,
                maximum,
            },
            StudentResponse::Hotspot { selections },
        ) => validate_presented_hotspot(
            &surface.regions,
            *minimum,
            *maximum,
            selections,
            &mut violations,
        ),
        _ => violations.push(StudentResponseFormatIssue::ResponseKindMismatch),
    }

    StudentResponseFormatCheck { violations }
}

fn validate_presented_selection<T: PresentedResponseItemContent>(
    choices: &[T],
    minimum: u32,
    maximum: u32,
    selected: &[ResponseItemReference],
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    let actual = count(selected.iter());
    if actual < u64::from(minimum) || actual > u64::from(maximum) {
        violations.push(StudentResponseFormatIssue::SelectionCount {
            expected: response_selection_rule(minimum, maximum, choices.len()),
            actual,
        });
    }
    let available: BTreeSet<ResponseItemReference> = choices
        .iter()
        .map(|choice| ResponseItemReference::new(choice.presentation_item_id().as_str()))
        .collect();
    let mut observed = BTreeSet::new();
    for choice in selected {
        if !observed.insert(choice.clone()) {
            violations.push(StudentResponseFormatIssue::DuplicateChoice {
                choice: choice.clone(),
            });
        }
        if !available.contains(choice) {
            violations.push(StudentResponseFormatIssue::UnknownChoice {
                choice: choice.clone(),
            });
        }
    }
}

fn response_selection_rule(minimum: u32, maximum: u32, available: usize) -> ResponseSelectionRule {
    let available = u32::try_from(available).expect("supported presentation choices fit u32");
    match (minimum, maximum) {
        (1, 1) => ResponseSelectionRule::ExactlyOne,
        (minimum, maximum) if minimum == maximum => {
            ResponseSelectionRule::Exactly { count: minimum }
        }
        (0, maximum) if maximum == available => ResponseSelectionRule::AnyNumber,
        (1, maximum) if maximum == available => ResponseSelectionRule::AtLeastOne,
        // Presentation construction only emits the four source cardinalities
        // above. A checksum-valid but externally malformed schema is still
        // rejected by the numeric bounds check; this fallback preserves the
        // existing browser-safe violation shape without inventing a new wire
        // contract solely for corrupt storage.
        (minimum, _) => ResponseSelectionRule::Exactly { count: minimum },
    }
}

fn validate_text_length(
    max_length: u32,
    text: &str,
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    let actual_length = count(text.chars());
    if actual_length > u64::from(max_length) {
        violations.push(StudentResponseFormatIssue::TextTooLong {
            max_length,
            actual_length,
        });
    }
}

fn validate_presented_multi_blank(
    blanks: &[PresentedTextEntrySlot],
    answers: &[question_model::response::StudentTextEntry],
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    let expected: BTreeSet<_> = blanks
        .iter()
        .map(|blank| ResponseItemReference::new(blank.id.as_str()))
        .collect();
    let actual: BTreeSet<_> = answers.iter().map(|answer| answer.slot.clone()).collect();
    if expected.len() != blanks.len()
        || actual.len() != answers.len()
        || answers.len() != blanks.len()
        || actual != expected
    {
        violations.push(StudentResponseFormatIssue::BlankSlotsMismatch);
        return;
    }
    for answer in answers {
        let blank = blanks
            .iter()
            .find(|blank| blank.id.as_str() == answer.slot.as_str())
            .expect("validated blank slot set is exact");
        validate_text_length(blank.max_characters, &answer.text, violations);
    }
}

fn validate_presented_matching<
    Prompt: PresentedResponseItemContent,
    Choice: PresentedResponseItemContent,
>(
    prompts: &[Prompt],
    choices: &[Choice],
    reuse_choices: bool,
    matches: &[question_model::response::StudentMatch],
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    let expected_prompts: BTreeSet<_> = prompts
        .iter()
        .map(|prompt| ResponseItemReference::new(prompt.presentation_item_id().as_str()))
        .collect();
    let actual_prompts: BTreeSet<_> = matches.iter().map(|pair| pair.prompt.clone()).collect();
    if expected_prompts.len() != prompts.len()
        || actual_prompts.len() != matches.len()
        || matches.len() != prompts.len()
        || actual_prompts != expected_prompts
    {
        violations.push(StudentResponseFormatIssue::MatchingPromptsMismatch);
    }
    let available_choices: BTreeSet<_> = choices
        .iter()
        .map(|choice| ResponseItemReference::new(choice.presentation_item_id().as_str()))
        .collect();
    let mut observed = BTreeSet::new();
    for pair in matches {
        if !available_choices.contains(&pair.choice) {
            violations.push(StudentResponseFormatIssue::UnknownMatchChoice {
                choice: pair.choice.clone(),
            });
        }
        if !reuse_choices && !observed.insert(pair.choice.clone()) {
            violations.push(StudentResponseFormatIssue::DuplicateMatchChoice {
                choice: pair.choice.clone(),
            });
        }
    }
}

fn validate_presented_hotspot(
    regions: &[PresentedHotspotRegion],
    minimum: u32,
    maximum: u32,
    selections: &[question_model::response::StudentHotspotSelection],
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    let actual = count(selections.iter());
    if actual < u64::from(minimum) || actual > u64::from(maximum) {
        violations.push(StudentResponseFormatIssue::SelectionCount {
            expected: response_selection_rule(minimum, maximum, regions.len()),
            actual,
        });
    }
    let available: BTreeSet<_> = regions
        .iter()
        .map(|region| ResponseItemReference::new(region.id.as_str()))
        .collect();
    validate_hotspot_region_references(selections, &available, violations);
}

fn validate_multi_blank(
    blanks: &[TextEntrySlot],
    answers: &[question_model::response::StudentTextEntry],
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    let expected: BTreeSet<_> = blanks.iter().map(|blank| blank.id.clone()).collect();
    let actual: BTreeSet<_> = answers.iter().map(|answer| answer.slot.clone()).collect();
    if expected.len() != blanks.len()
        || actual.len() != answers.len()
        || answers.len() != blanks.len()
        || actual != expected
    {
        violations.push(StudentResponseFormatIssue::BlankSlotsMismatch);
        return;
    }
    for answer in answers {
        let blank = blanks
            .iter()
            .find(|blank| blank.id == answer.slot)
            .expect("validated slot set is exact");
        let actual_length = count(answer.text.chars());
        if actual_length > u64::from(blank.max_length) {
            violations.push(StudentResponseFormatIssue::TextTooLong {
                max_length: blank.max_length,
                actual_length,
            });
        }
    }
}

fn validate_matching(
    prompts: &[question_model::response::MatchingPrompt],
    choices: &[question_model::response::MatchingChoice],
    matches: &[question_model::response::StudentMatch],
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    let expected_prompts: BTreeSet<_> = prompts.iter().map(|prompt| prompt.id.clone()).collect();
    let actual_prompts: BTreeSet<_> = matches.iter().map(|pair| pair.prompt.clone()).collect();
    if expected_prompts.len() != prompts.len()
        || actual_prompts.len() != matches.len()
        || matches.len() != prompts.len()
        || actual_prompts != expected_prompts
    {
        violations.push(StudentResponseFormatIssue::MatchingPromptsMismatch);
    }
    let available_choices: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
    let mut observed = BTreeSet::new();
    for pair in matches {
        if !available_choices.contains(&pair.choice) {
            violations.push(StudentResponseFormatIssue::UnknownMatchChoice {
                choice: pair.choice.clone(),
            });
        }
        if !observed.insert(pair.choice.clone()) {
            violations.push(StudentResponseFormatIssue::DuplicateMatchChoice {
                choice: pair.choice.clone(),
            });
        }
    }
}

fn validate_hotspot(
    regions: &[HotspotRegion],
    selection: ResponseSelectionRule,
    selections: &[question_model::response::StudentHotspotSelection],
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    validate_selection_count(selection, selections.len(), violations);
    let available: BTreeSet<_> = regions.iter().map(|region| region.id.clone()).collect();
    validate_hotspot_region_references(selections, &available, violations);
}

fn validate_hotspot_region_references(
    selections: &[question_model::response::StudentHotspotSelection],
    available: &BTreeSet<ResponseItemReference>,
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    let mut observed = BTreeSet::new();
    for selection in selections {
        if !available.contains(&selection.region) {
            violations.push(StudentResponseFormatIssue::UnknownHotspotRegion {
                region: selection.region.clone(),
            });
        }
        if !observed.insert(selection.region.clone()) {
            violations.push(StudentResponseFormatIssue::DuplicateHotspotRegion {
                region: selection.region.clone(),
            });
        }
    }
}

fn validate_selection_count(
    selection: ResponseSelectionRule,
    actual: usize,
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    let valid = match selection {
        ResponseSelectionRule::ExactlyOne => actual == 1,
        ResponseSelectionRule::Exactly { count } => actual == count as usize,
        ResponseSelectionRule::AnyNumber => true,
        ResponseSelectionRule::AtLeastOne => actual >= 1,
    };
    if !valid {
        violations.push(StudentResponseFormatIssue::SelectionCount {
            expected: selection,
            actual: actual as u64,
        });
    }
}

/// Validates multiple-choice cardinality and identifier membership.
fn validate_selection(
    choices: &[question_model::response::QuestionChoice],
    selection: ResponseSelectionRule,
    selected: &[ResponseItemReference],
    violations: &mut Vec<StudentResponseFormatIssue>,
) {
    let actual = count(selected.iter());
    let cardinality_matches = match selection {
        ResponseSelectionRule::ExactlyOne => actual == 1,
        ResponseSelectionRule::Exactly { count } => actual == u64::from(count),
        ResponseSelectionRule::AnyNumber => true,
        ResponseSelectionRule::AtLeastOne => actual >= 1,
    };
    if !cardinality_matches {
        violations.push(StudentResponseFormatIssue::SelectionCount {
            expected: selection,
            actual,
        });
    }

    let available: BTreeSet<ResponseItemReference> =
        choices.iter().map(|choice| choice.id.clone()).collect();
    let mut observed = BTreeSet::new();
    for choice in selected {
        if !observed.insert(choice.clone()) {
            violations.push(StudentResponseFormatIssue::DuplicateChoice {
                choice: choice.clone(),
            });
        }
        if !available.contains(choice) {
            violations.push(StudentResponseFormatIssue::UnknownChoice {
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
    use question_model::answer::{NumericResponseTolerance, TextResponseMatchRule};
    use question_model::response::{
        HotspotRegion, MatchingChoice, MatchingPrompt, OrderingItem, QuestionChoice,
        StudentHotspotSelection, StudentMatch, StudentTextEntry, TextEntrySlot,
    };

    fn question_choice(id: &str) -> QuestionChoice {
        QuestionChoice {
            id: ResponseItemReference::new(id),
            body: Vec::new(),
        }
    }

    fn matching_prompt(id: &str) -> MatchingPrompt {
        MatchingPrompt {
            id: ResponseItemReference::new(id),
            body: Vec::new(),
        }
    }

    fn matching_choice(id: &str) -> MatchingChoice {
        MatchingChoice {
            id: ResponseItemReference::new(id),
            body: Vec::new(),
        }
    }

    fn ordering_item(id: &str) -> OrderingItem {
        OrderingItem {
            id: ResponseItemReference::new(id),
            body: Vec::new(),
        }
    }

    #[test]
    fn a_kind_mismatch_stops_before_answer_adjacent_checks() {
        let definition = QuestionResponseFormat::Numeric {
            tolerance: NumericResponseTolerance::Exact,
            unit: None,
        };
        let response = StudentResponse::ShortText {
            text: "12".to_string(),
        };

        assert_eq!(
            validate_response_format(&definition, &response).violations,
            vec![StudentResponseFormatIssue::ResponseKindMismatch]
        );
    }

    #[test]
    fn numeric_input_must_be_finite() {
        let definition = QuestionResponseFormat::Numeric {
            tolerance: NumericResponseTolerance::Absolute { epsilon: 0.1 },
            unit: None,
        };

        assert_eq!(
            validate_response_format(&definition, &StudentResponse::Numeric { value: f64::NAN })
                .violations,
            vec![StudentResponseFormatIssue::NumericNotFinite]
        );
    }

    #[test]
    fn selection_reports_count_duplicates_and_unknown_ids() {
        let definition = QuestionResponseFormat::MultipleChoice {
            choices: vec![question_choice("a"), question_choice("b")],
            selection: ResponseSelectionRule::ExactlyOne,
        };
        let response = StudentResponse::MultipleChoice {
            selected: vec![
                ResponseItemReference::new("b"),
                ResponseItemReference::new("b"),
                ResponseItemReference::new("z"),
            ],
        };

        assert_eq!(
            validate_response_format(&definition, &response).violations,
            vec![
                StudentResponseFormatIssue::SelectionCount {
                    expected: ResponseSelectionRule::ExactlyOne,
                    actual: 3,
                },
                StudentResponseFormatIssue::DuplicateChoice {
                    choice: ResponseItemReference::new("b"),
                },
                StudentResponseFormatIssue::UnknownChoice {
                    choice: ResponseItemReference::new("z"),
                },
            ]
        );
    }

    #[test]
    fn short_text_counts_characters_instead_of_utf8_bytes() {
        let definition = QuestionResponseFormat::ShortText {
            match_mode: TextResponseMatchRule::Normalized,
            max_length: 2,
        };
        let response = StudentResponse::ShortText {
            text: "\u{03b1}\u{03b2}".to_string(),
        };

        assert!(validate_response_format(&definition, &response).is_valid());
    }

    #[test]
    fn violation_json_uses_the_browser_camel_case_contract() {
        let report = StudentResponseFormatCheck {
            violations: vec![StudentResponseFormatIssue::TextTooLong {
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
        let definition = QuestionResponseFormat::Ordering {
            items: vec![ordering_item("first"), ordering_item("second")],
        };
        let response = StudentResponse::Ordering {
            order: vec![
                ResponseItemReference::new("first"),
                ResponseItemReference::new("first"),
            ],
        };

        assert_eq!(
            validate_response_format(&definition, &response).violations,
            vec![StudentResponseFormatIssue::OrderingItemsMismatch]
        );
    }

    #[test]
    fn compound_flat_responses_refuse_stale_slots_pairs_and_regions() {
        let multi_blank = QuestionResponseFormat::MultiBlank {
            blanks: vec![
                TextEntrySlot {
                    id: ResponseItemReference::new("first"),
                    label: Vec::new(),
                    match_mode: TextResponseMatchRule::Normalized,
                    max_length: 4,
                },
                TextEntrySlot {
                    id: ResponseItemReference::new("second"),
                    label: Vec::new(),
                    match_mode: TextResponseMatchRule::Exact,
                    max_length: 4,
                },
            ],
        };
        assert_eq!(
            validate_response_format(
                &multi_blank,
                &StudentResponse::MultiBlank {
                    answers: vec![StudentTextEntry {
                        slot: ResponseItemReference::new("first"),
                        text: "value".to_string(),
                    }],
                },
            )
            .violations,
            vec![StudentResponseFormatIssue::BlankSlotsMismatch]
        );

        let matching = QuestionResponseFormat::Matching {
            prompts: vec![matching_prompt("dna"), matching_prompt("rna")],
            choices: vec![matching_choice("deoxy"), matching_choice("ribose")],
        };
        assert_eq!(
            validate_response_format(
                &matching,
                &StudentResponse::Matching {
                    matches: vec![
                        StudentMatch {
                            prompt: ResponseItemReference::new("dna"),
                            choice: ResponseItemReference::new("deoxy"),
                        },
                        StudentMatch {
                            prompt: ResponseItemReference::new("dna"),
                            choice: ResponseItemReference::new("deoxy"),
                        },
                    ],
                },
            )
            .violations,
            vec![
                StudentResponseFormatIssue::MatchingPromptsMismatch,
                StudentResponseFormatIssue::DuplicateMatchChoice {
                    choice: ResponseItemReference::new("deoxy"),
                },
            ]
        );

        let hotspot = QuestionResponseFormat::Hotspot {
            surface: question_model::envelope::QuestionAssetReference {
                asset: question_model::QuestionAssetId::from_uuid(uuid::Uuid::from_u128(1)),
                checksum: "a".repeat(64),
            },
            description: "A diagram".to_string(),
            regions: vec![HotspotRegion {
                id: ResponseItemReference::new("target"),
                label: Vec::new(),
                x: 1_000,
                y: 1_000,
                width: 2_000,
                height: 2_000,
            }],
            selection: ResponseSelectionRule::ExactlyOne,
        };
        assert_eq!(
            validate_response_format(
                &hotspot,
                &StudentResponse::Hotspot {
                    selections: vec![StudentHotspotSelection {
                        region: ResponseItemReference::new("unknown"),
                    }],
                },
            )
            .violations,
            vec![StudentResponseFormatIssue::UnknownHotspotRegion {
                region: ResponseItemReference::new("unknown"),
            }]
        );
    }

    #[test]
    fn external_tool_accepts_only_its_marker_response() {
        let external = QuestionResponseFormat::ExternalTool {};
        assert!(validate_response_format(&external, &StudentResponse::ExternalTool {}).is_valid());

        for response in [
            StudentResponse::Numeric { value: 1.0 },
            StudentResponse::MultipleChoice { selected: vec![] },
            StudentResponse::ShortText {
                text: String::new(),
            },
            StudentResponse::Ordering { order: vec![] },
        ] {
            assert_eq!(
                validate_response_format(&external, &response).violations,
                vec![StudentResponseFormatIssue::ResponseKindMismatch]
            );
        }

        let numeric = QuestionResponseFormat::Numeric {
            tolerance: NumericResponseTolerance::Exact,
            unit: None,
        };
        assert_eq!(
            validate_response_format(&numeric, &StudentResponse::ExternalTool {}).violations,
            vec![StudentResponseFormatIssue::ResponseKindMismatch]
        );
    }
}
