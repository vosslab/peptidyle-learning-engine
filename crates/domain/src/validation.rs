//! Browser-safe student-response format validation (WP-C6, MOD-GRD boundary).
//!
//! This module can inspect Question Response Formats and Student input, but it has
//! no answer key and makes no correctness decision. The server repeats the
//! same validation before calling the server-only `grading` crate.

use std::collections::BTreeSet;

use question_model::answer::SelectionCardinality;
use question_model::presentation::{
    PresentedBlankV1, PresentedChoiceV1, PresentedHotspotRegionV1, IssuedQuestionResponseFormatV1,
};
use question_model::response::{
    ChoiceId, HotspotRegion, QuestionResponseFormat, StudentResponse, TextEntrySlot,
};
use serde::{Deserialize, Serialize};

/// One reason a student response cannot be submitted in its current form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ResponseFormatViolation {
    /// The response kind does not match the Question Response Format.
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
    /// A multi-blank response does not name every declared slot exactly once.
    BlankSlotsMismatch,
    /// A matching response does not name every prompt exactly once.
    MatchingPromptsMismatch,
    /// A matching response repeats a choice where the definition requires a permutation.
    DuplicateMatchChoice {
        /// Reused choice identifier.
        choice: ChoiceId,
    },
    /// A matching response names a choice absent from the definition.
    UnknownMatchChoice {
        /// Unrecognized choice identifier.
        choice: ChoiceId,
    },
    /// An ordering response is not an exact permutation of the defined items.
    OrderingItemsMismatch,
    /// A hotspot coordinate lies outside the normalized 0 through 10,000 surface.
    HotspotPointOutOfBounds,
    /// A hotspot point does not fall within exactly one public candidate region.
    HotspotPointOutsideRegion,
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
    definition: &QuestionResponseFormat,
    response: &StudentResponse,
) -> ResponseFormatReport {
    let mut violations = Vec::new();

    match (definition, response) {
        (QuestionResponseFormat::Numeric { .. }, StudentResponse::Numeric { value }) => {
            if !value.is_finite() {
                violations.push(ResponseFormatViolation::NumericNotFinite);
            }
        }
        (
            QuestionResponseFormat::MultipleChoice { choices, selection },
            StudentResponse::MultipleChoice { selected },
        ) => validate_selection(choices, *selection, selected, &mut violations),
        (QuestionResponseFormat::ShortText { max_length, .. }, StudentResponse::ShortText { text }) => {
            let actual_length = count(text.chars());
            if actual_length > u64::from(*max_length) {
                violations.push(ResponseFormatViolation::TextTooLong {
                    max_length: *max_length,
                    actual_length,
                });
            }
        }
        (QuestionResponseFormat::MultiBlank { blanks }, StudentResponse::MultiBlank { answers }) => {
            validate_multi_blank(blanks, answers, &mut violations);
        }
        (
            QuestionResponseFormat::Matching { prompts, choices },
            StudentResponse::Matching { matches },
        ) => validate_matching(prompts, choices, matches, &mut violations),
        (QuestionResponseFormat::Ordering { items }, StudentResponse::Ordering { order }) => {
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
        (
            QuestionResponseFormat::Hotspot {
                regions, selection, ..
            },
            StudentResponse::Hotspot { points },
        ) => validate_hotspot(regions, *selection, points, &mut violations),
        (QuestionResponseFormat::FileUpload { .. }, StudentResponse::FileUpload { object_key }) => {
            if object_key.trim().is_empty() {
                violations.push(ResponseFormatViolation::MissingUploadReference);
            }
        }
        (QuestionResponseFormat::ExternalTool {}, StudentResponse::ExternalTool {}) => {}
        _ => violations.push(ResponseFormatViolation::ResponseKindMismatch),
    }

    ResponseFormatReport { violations }
}

/// Validates a student response against the answer-free schema frozen with an
/// issued presentation.
///
/// This is the server-side authority for a presentation-bearing attempt: it
/// lets a first submission reject malformed input without asking a mutable
/// catalog or renderer to rebuild the student's already-issued widget. It
/// checks only public shape, never answer material or correctness.
pub fn validate_presentation_response_format(
    schema: &IssuedQuestionResponseFormatV1,
    response: &StudentResponse,
) -> ResponseFormatReport {
    let mut violations = Vec::new();

    match (schema, response) {
        (IssuedQuestionResponseFormatV1::Numerical { .. }, StudentResponse::Numeric { value }) => {
            if !value.is_finite() {
                violations.push(ResponseFormatViolation::NumericNotFinite);
            }
        }
        (
            IssuedQuestionResponseFormatV1::SingleChoice { choices },
            StudentResponse::MultipleChoice { selected },
        ) => {
            validate_presented_selection(choices, 1, 1, selected, &mut violations);
        }
        (
            IssuedQuestionResponseFormatV1::MultipleAnswer {
                choices,
                minimum,
                maximum,
            },
            StudentResponse::MultipleChoice { selected },
        ) => validate_presented_selection(choices, *minimum, *maximum, selected, &mut violations),
        (IssuedQuestionResponseFormatV1::FillIn { max_characters }, StudentResponse::ShortText { text }) => {
            validate_text_length(*max_characters, text, &mut violations);
        }
        (IssuedQuestionResponseFormatV1::MultiFillIn { blanks }, StudentResponse::MultiBlank { answers }) => {
            validate_presented_multi_blank(blanks, answers, &mut violations);
        }
        (
            IssuedQuestionResponseFormatV1::Matching {
                prompts,
                choices,
                reuse_choices,
            },
            StudentResponse::Matching { matches },
        ) => {
            validate_presented_matching(prompts, choices, *reuse_choices, matches, &mut violations)
        }
        (IssuedQuestionResponseFormatV1::Ordering { items }, StudentResponse::Ordering { order }) => {
            let expected: BTreeSet<ChoiceId> = items
                .iter()
                .map(|item| ChoiceId::new(item.id.as_str()))
                .collect();
            let actual: BTreeSet<ChoiceId> = order.iter().cloned().collect();
            if expected.len() != items.len()
                || actual.len() != order.len()
                || order.len() != items.len()
                || actual != expected
            {
                violations.push(ResponseFormatViolation::OrderingItemsMismatch);
            }
        }
        (
            IssuedQuestionResponseFormatV1::Hotspot {
                surface,
                minimum,
                maximum,
            },
            StudentResponse::Hotspot { points },
        ) => validate_presented_hotspot(
            &surface.regions,
            *minimum,
            *maximum,
            points,
            &mut violations,
        ),
        _ => violations.push(ResponseFormatViolation::ResponseKindMismatch),
    }

    ResponseFormatReport { violations }
}

fn validate_presented_selection(
    choices: &[PresentedChoiceV1],
    minimum: u32,
    maximum: u32,
    selected: &[ChoiceId],
    violations: &mut Vec<ResponseFormatViolation>,
) {
    let actual = count(selected.iter());
    if actual < u64::from(minimum) || actual > u64::from(maximum) {
        violations.push(ResponseFormatViolation::SelectionCount {
            expected: selection_cardinality(minimum, maximum, choices.len()),
            actual,
        });
    }
    let available: BTreeSet<ChoiceId> = choices
        .iter()
        .map(|choice| ChoiceId::new(choice.id.as_str()))
        .collect();
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

fn selection_cardinality(minimum: u32, maximum: u32, available: usize) -> SelectionCardinality {
    let available = u32::try_from(available).expect("supported presentation choices fit u32");
    match (minimum, maximum) {
        (1, 1) => SelectionCardinality::ExactlyOne,
        (minimum, maximum) if minimum == maximum => {
            SelectionCardinality::Exactly { count: minimum }
        }
        (0, maximum) if maximum == available => SelectionCardinality::AnyNumber,
        (1, maximum) if maximum == available => SelectionCardinality::AtLeastOne,
        // Presentation construction only emits the four source cardinalities
        // above. A checksum-valid but externally malformed schema is still
        // rejected by the numeric bounds check; this fallback preserves the
        // existing browser-safe violation shape without inventing a new wire
        // contract solely for corrupt storage.
        (minimum, _) => SelectionCardinality::Exactly { count: minimum },
    }
}

fn validate_text_length(
    max_length: u32,
    text: &str,
    violations: &mut Vec<ResponseFormatViolation>,
) {
    let actual_length = count(text.chars());
    if actual_length > u64::from(max_length) {
        violations.push(ResponseFormatViolation::TextTooLong {
            max_length,
            actual_length,
        });
    }
}

fn validate_presented_multi_blank(
    blanks: &[PresentedBlankV1],
    answers: &[question_model::response::TextEntryAnswer],
    violations: &mut Vec<ResponseFormatViolation>,
) {
    let expected: BTreeSet<_> = blanks
        .iter()
        .map(|blank| ChoiceId::new(blank.id.as_str()))
        .collect();
    let actual: BTreeSet<_> = answers.iter().map(|answer| answer.slot.clone()).collect();
    if expected.len() != blanks.len()
        || actual.len() != answers.len()
        || answers.len() != blanks.len()
        || actual != expected
    {
        violations.push(ResponseFormatViolation::BlankSlotsMismatch);
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

fn validate_presented_matching(
    prompts: &[PresentedChoiceV1],
    choices: &[PresentedChoiceV1],
    reuse_choices: bool,
    matches: &[question_model::response::MatchPair],
    violations: &mut Vec<ResponseFormatViolation>,
) {
    let expected_prompts: BTreeSet<_> = prompts
        .iter()
        .map(|prompt| ChoiceId::new(prompt.id.as_str()))
        .collect();
    let actual_prompts: BTreeSet<_> = matches.iter().map(|pair| pair.prompt.clone()).collect();
    if expected_prompts.len() != prompts.len()
        || actual_prompts.len() != matches.len()
        || matches.len() != prompts.len()
        || actual_prompts != expected_prompts
    {
        violations.push(ResponseFormatViolation::MatchingPromptsMismatch);
    }
    let available_choices: BTreeSet<_> = choices
        .iter()
        .map(|choice| ChoiceId::new(choice.id.as_str()))
        .collect();
    let mut observed = BTreeSet::new();
    for pair in matches {
        if !available_choices.contains(&pair.choice) {
            violations.push(ResponseFormatViolation::UnknownMatchChoice {
                choice: pair.choice.clone(),
            });
        }
        if !reuse_choices && !observed.insert(pair.choice.clone()) {
            violations.push(ResponseFormatViolation::DuplicateMatchChoice {
                choice: pair.choice.clone(),
            });
        }
    }
}

fn validate_presented_hotspot(
    regions: &[PresentedHotspotRegionV1],
    minimum: u32,
    maximum: u32,
    points: &[question_model::response::HotspotPoint],
    violations: &mut Vec<ResponseFormatViolation>,
) {
    let actual = count(points.iter());
    if actual < u64::from(minimum) || actual > u64::from(maximum) {
        violations.push(ResponseFormatViolation::SelectionCount {
            expected: selection_cardinality(minimum, maximum, regions.len()),
            actual,
        });
    }
    for point in points {
        if point.x > 10_000 || point.y > 10_000 {
            violations.push(ResponseFormatViolation::HotspotPointOutOfBounds);
            continue;
        }
        if regions
            .iter()
            .filter(|region| presented_region_contains(region, point.x, point.y))
            .count()
            != 1
        {
            violations.push(ResponseFormatViolation::HotspotPointOutsideRegion);
        }
    }
}

fn presented_region_contains(region: &PresentedHotspotRegionV1, x: u16, y: u16) -> bool {
    let right = u32::from(region.x) + u32::from(region.width);
    let bottom = u32::from(region.y) + u32::from(region.height);
    u32::from(x) >= u32::from(region.x)
        && u32::from(x) <= right
        && u32::from(y) >= u32::from(region.y)
        && u32::from(y) <= bottom
}

fn validate_multi_blank(
    blanks: &[TextEntrySlot],
    answers: &[question_model::response::TextEntryAnswer],
    violations: &mut Vec<ResponseFormatViolation>,
) {
    let expected: BTreeSet<_> = blanks.iter().map(|blank| blank.id.clone()).collect();
    let actual: BTreeSet<_> = answers.iter().map(|answer| answer.slot.clone()).collect();
    if expected.len() != blanks.len()
        || actual.len() != answers.len()
        || answers.len() != blanks.len()
        || actual != expected
    {
        violations.push(ResponseFormatViolation::BlankSlotsMismatch);
        return;
    }
    for answer in answers {
        let blank = blanks
            .iter()
            .find(|blank| blank.id == answer.slot)
            .expect("validated slot set is exact");
        let actual_length = count(answer.text.chars());
        if actual_length > u64::from(blank.max_length) {
            violations.push(ResponseFormatViolation::TextTooLong {
                max_length: blank.max_length,
                actual_length,
            });
        }
    }
}

fn validate_matching(
    prompts: &[question_model::response::ChoiceOption],
    choices: &[question_model::response::ChoiceOption],
    matches: &[question_model::response::MatchPair],
    violations: &mut Vec<ResponseFormatViolation>,
) {
    let expected_prompts: BTreeSet<_> = prompts.iter().map(|prompt| prompt.id.clone()).collect();
    let actual_prompts: BTreeSet<_> = matches.iter().map(|pair| pair.prompt.clone()).collect();
    if expected_prompts.len() != prompts.len()
        || actual_prompts.len() != matches.len()
        || matches.len() != prompts.len()
        || actual_prompts != expected_prompts
    {
        violations.push(ResponseFormatViolation::MatchingPromptsMismatch);
    }
    let available_choices: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
    let mut observed = BTreeSet::new();
    for pair in matches {
        if !available_choices.contains(&pair.choice) {
            violations.push(ResponseFormatViolation::UnknownMatchChoice {
                choice: pair.choice.clone(),
            });
        }
        if !observed.insert(pair.choice.clone()) {
            violations.push(ResponseFormatViolation::DuplicateMatchChoice {
                choice: pair.choice.clone(),
            });
        }
    }
}

fn validate_hotspot(
    regions: &[HotspotRegion],
    selection: SelectionCardinality,
    points: &[question_model::response::HotspotPoint],
    violations: &mut Vec<ResponseFormatViolation>,
) {
    validate_selection_count(selection, points.len(), violations);
    for point in points {
        if point.x > 10_000 || point.y > 10_000 {
            violations.push(ResponseFormatViolation::HotspotPointOutOfBounds);
            continue;
        }
        if regions
            .iter()
            .filter(|region| region_contains(region, point.x, point.y))
            .count()
            != 1
        {
            violations.push(ResponseFormatViolation::HotspotPointOutsideRegion);
        }
    }
}

fn region_contains(region: &HotspotRegion, x: u16, y: u16) -> bool {
    let right = u32::from(region.x) + u32::from(region.width);
    let bottom = u32::from(region.y) + u32::from(region.height);
    u32::from(x) >= u32::from(region.x)
        && u32::from(x) <= right
        && u32::from(y) >= u32::from(region.y)
        && u32::from(y) <= bottom
}

fn validate_selection_count(
    selection: SelectionCardinality,
    actual: usize,
    violations: &mut Vec<ResponseFormatViolation>,
) {
    let valid = match selection {
        SelectionCardinality::ExactlyOne => actual == 1,
        SelectionCardinality::Exactly { count } => actual == count as usize,
        SelectionCardinality::AnyNumber => true,
        SelectionCardinality::AtLeastOne => actual >= 1,
    };
    if !valid {
        violations.push(ResponseFormatViolation::SelectionCount {
            expected: selection,
            actual: actual as u64,
        });
    }
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
    use question_model::response::{
        ChoiceOption, HotspotPoint, HotspotRegion, MatchPair, TextEntryAnswer, TextEntrySlot,
    };

    fn choice(id: &str) -> ChoiceOption {
        ChoiceOption {
            id: ChoiceId::new(id),
            body: Vec::new(),
        }
    }

    #[test]
    fn a_kind_mismatch_stops_before_answer_adjacent_checks() {
        let definition = QuestionResponseFormat::Numeric {
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
        let definition = QuestionResponseFormat::Numeric {
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
        let definition = QuestionResponseFormat::MultipleChoice {
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
        let definition = QuestionResponseFormat::ShortText {
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
        let definition = QuestionResponseFormat::Ordering {
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
    fn compound_flat_responses_refuse_stale_slots_pairs_and_regions() {
        let multi_blank = QuestionResponseFormat::MultiBlank {
            blanks: vec![
                TextEntrySlot {
                    id: ChoiceId::new("first"),
                    label: Vec::new(),
                    match_mode: TextMatchMode::Normalized,
                    max_length: 4,
                },
                TextEntrySlot {
                    id: ChoiceId::new("second"),
                    label: Vec::new(),
                    match_mode: TextMatchMode::Exact,
                    max_length: 4,
                },
            ],
        };
        assert_eq!(
            validate_response_format(
                &multi_blank,
                &StudentResponse::MultiBlank {
                    answers: vec![TextEntryAnswer {
                        slot: ChoiceId::new("first"),
                        text: "value".to_string(),
                    }],
                },
            )
            .violations,
            vec![ResponseFormatViolation::BlankSlotsMismatch]
        );

        let matching = QuestionResponseFormat::Matching {
            prompts: vec![choice("dna"), choice("rna")],
            choices: vec![choice("deoxy"), choice("ribose")],
        };
        assert_eq!(
            validate_response_format(
                &matching,
                &StudentResponse::Matching {
                    matches: vec![
                        MatchPair {
                            prompt: ChoiceId::new("dna"),
                            choice: ChoiceId::new("deoxy"),
                        },
                        MatchPair {
                            prompt: ChoiceId::new("dna"),
                            choice: ChoiceId::new("deoxy"),
                        },
                    ],
                },
            )
            .violations,
            vec![
                ResponseFormatViolation::MatchingPromptsMismatch,
                ResponseFormatViolation::DuplicateMatchChoice {
                    choice: ChoiceId::new("deoxy"),
                },
            ]
        );

        let hotspot = QuestionResponseFormat::Hotspot {
            surface: question_model::envelope::AssetRef {
                asset: question_model::AssetId::from_uuid(uuid::Uuid::from_u128(1)),
                checksum: "a".repeat(64),
            },
            description: "A diagram".to_string(),
            regions: vec![HotspotRegion {
                id: ChoiceId::new("target"),
                label: Vec::new(),
                x: 1_000,
                y: 1_000,
                width: 2_000,
                height: 2_000,
            }],
            selection: SelectionCardinality::ExactlyOne,
        };
        assert_eq!(
            validate_response_format(
                &hotspot,
                &StudentResponse::Hotspot {
                    points: vec![HotspotPoint { x: 9_000, y: 9_000 }],
                },
            )
            .violations,
            vec![ResponseFormatViolation::HotspotPointOutsideRegion]
        );
    }

    #[test]
    fn file_upload_requires_a_server_issued_object_reference() {
        let definition = QuestionResponseFormat::FileUpload {
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
            StudentResponse::FileUpload {
                object_key: "object".to_string(),
            },
        ] {
            assert_eq!(
                validate_response_format(&external, &response).violations,
                vec![ResponseFormatViolation::ResponseKindMismatch]
            );
        }

        let numeric = QuestionResponseFormat::Numeric {
            tolerance: NumericTolerance::Exact,
            unit: None,
        };
        assert_eq!(
            validate_response_format(&numeric, &StudentResponse::ExternalTool {}).violations,
            vec![ResponseFormatViolation::ResponseKindMismatch]
        );
    }
}
