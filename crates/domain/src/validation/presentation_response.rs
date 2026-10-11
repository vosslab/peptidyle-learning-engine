//! Validates responses against the answer-free schema frozen at issuance.

use std::collections::BTreeSet;

use question_model::answer::ResponseSelectionRule;
use question_model::presentation::{
    PresentedHotspotRegion, PresentedResponseItemContent, PresentedTextEntrySlot,
    QuestionPresentationResponseFormat,
};
use question_model::response::{ResponseItemId, StudentResponse};

use super::{
    StudentResponseFormatCheck, StudentResponseFormatIssue, count, validate_hotspot_region_ids,
    validate_text_length,
};

/// Validates a student response against the answer-free schema frozen with an
/// issued presentation.
///
/// This is the server-side authority for a presentation-bearing attempt: it
/// lets a first submission reject malformed input without asking a mutable
/// Question Library or renderer to rebuild the student's already-issued Question Response Control. It
/// checks only public shape, never Answer Key facts or correctness.
pub fn validate_presentation_response_format(
    schema: &QuestionPresentationResponseFormat,
    response: &StudentResponse,
) -> StudentResponseFormatCheck {
    let mut issues = Vec::new();

    match (schema, response) {
        (
            QuestionPresentationResponseFormat::Numerical { .. },
            StudentResponse::Numeric { value },
        ) => {
            if !value.is_finite() {
                issues.push(StudentResponseFormatIssue::NumericNotFinite);
            }
        }
        (
            QuestionPresentationResponseFormat::SingleChoice { choices },
            StudentResponse::MultipleChoice { selected },
        ) => {
            validate_presented_selection(choices, 1, 1, selected, &mut issues);
        }
        (
            QuestionPresentationResponseFormat::MultipleAnswer {
                choices,
                minimum,
                maximum,
            },
            StudentResponse::MultipleChoice { selected },
        ) => validate_presented_selection(choices, *minimum, *maximum, selected, &mut issues),
        (
            QuestionPresentationResponseFormat::FillIn { max_characters },
            StudentResponse::ShortText { text },
        ) => {
            validate_text_length(*max_characters, text, &mut issues);
        }
        (
            QuestionPresentationResponseFormat::MultiFillIn { blanks },
            StudentResponse::MultiBlank { answers },
        ) => {
            validate_presented_multi_blank(blanks, answers, &mut issues);
        }
        (
            QuestionPresentationResponseFormat::Matching {
                prompts,
                choices,
                reuse_choices,
            },
            StudentResponse::Matching { matches },
        ) => validate_presented_matching(prompts, choices, *reuse_choices, matches, &mut issues),
        (
            QuestionPresentationResponseFormat::Ordering { items },
            StudentResponse::Ordering { order },
        ) => {
            let expected: BTreeSet<ResponseItemId> = items
                .iter()
                .map(|item| ResponseItemId::new(item.id.as_str()))
                .collect();
            let actual: BTreeSet<ResponseItemId> = order.iter().cloned().collect();
            if expected.len() != items.len()
                || actual.len() != order.len()
                || order.len() != items.len()
                || actual != expected
            {
                issues.push(StudentResponseFormatIssue::OrderingItemsMismatch);
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
            &mut issues,
        ),
        (
            QuestionPresentationResponseFormat::ImathasQuestionBackend {},
            StudentResponse::ImathasQuestionBackend {},
        ) => {}
        (
            QuestionPresentationResponseFormat::BackendOwned {},
            StudentResponse::BackendOwned { .. },
        ) => {}
        _ => issues.push(StudentResponseFormatIssue::ResponseKindMismatch),
    }

    StudentResponseFormatCheck { issues }
}

fn validate_presented_selection<T: PresentedResponseItemContent>(
    choices: &[T],
    minimum: u32,
    maximum: u32,
    selected: &[ResponseItemId],
    issues: &mut Vec<StudentResponseFormatIssue>,
) {
    let actual = count(selected.iter());
    if actual < u64::from(minimum) || actual > u64::from(maximum) {
        issues.push(StudentResponseFormatIssue::SelectionCount {
            expected: response_selection_rule(minimum, maximum, choices.len()),
            actual,
        });
    }
    let available: BTreeSet<ResponseItemId> = choices
        .iter()
        .map(|choice| ResponseItemId::new(choice.presentation_item_id().as_str()))
        .collect();
    let mut observed = BTreeSet::new();
    for choice in selected {
        if !observed.insert(choice.clone()) {
            issues.push(StudentResponseFormatIssue::DuplicateChoice {
                choice: choice.clone(),
            });
        }
        if !available.contains(choice) {
            issues.push(StudentResponseFormatIssue::UnknownChoice {
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
        // existing browser-safe issue shape without inventing a new wire
        // contract solely for corrupt storage.
        (minimum, _) => ResponseSelectionRule::Exactly { count: minimum },
    }
}

fn validate_presented_multi_blank(
    blanks: &[PresentedTextEntrySlot],
    answers: &[question_model::response::StudentTextEntry],
    issues: &mut Vec<StudentResponseFormatIssue>,
) {
    let expected: BTreeSet<_> = blanks
        .iter()
        .map(|blank| ResponseItemId::new(blank.id.as_str()))
        .collect();
    let actual: BTreeSet<_> = answers.iter().map(|answer| answer.slot.clone()).collect();
    if expected.len() != blanks.len()
        || actual.len() != answers.len()
        || !actual.is_subset(&expected)
    {
        issues.push(StudentResponseFormatIssue::BlankSlotsMismatch);
        return;
    }
    for answer in answers {
        let blank = blanks
            .iter()
            .find(|blank| blank.id.as_str() == answer.slot.as_str())
            .expect("validated response slots exist in the Question");
        validate_text_length(blank.max_characters, &answer.text, issues);
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
    issues: &mut Vec<StudentResponseFormatIssue>,
) {
    let expected_prompts: BTreeSet<_> = prompts
        .iter()
        .map(|prompt| ResponseItemId::new(prompt.presentation_item_id().as_str()))
        .collect();
    let actual_prompts: BTreeSet<_> = matches.iter().map(|pair| pair.prompt.clone()).collect();
    if expected_prompts.len() != prompts.len()
        || actual_prompts.len() != matches.len()
        || matches.len() > prompts.len()
        || !actual_prompts.is_subset(&expected_prompts)
    {
        issues.push(StudentResponseFormatIssue::MatchingPromptsMismatch);
    }
    let available_choices: BTreeSet<_> = choices
        .iter()
        .map(|choice| ResponseItemId::new(choice.presentation_item_id().as_str()))
        .collect();
    let mut observed = BTreeSet::new();
    for pair in matches {
        if !available_choices.contains(&pair.choice) {
            issues.push(StudentResponseFormatIssue::UnknownMatchChoice {
                choice: pair.choice.clone(),
            });
        }
        if !reuse_choices && !observed.insert(pair.choice.clone()) {
            issues.push(StudentResponseFormatIssue::DuplicateMatchChoice {
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
    issues: &mut Vec<StudentResponseFormatIssue>,
) {
    let actual = count(selections.iter());
    if actual < u64::from(minimum) || actual > u64::from(maximum) {
        issues.push(StudentResponseFormatIssue::SelectionCount {
            expected: response_selection_rule(minimum, maximum, regions.len()),
            actual,
        });
    }
    let available: BTreeSet<_> = regions
        .iter()
        .map(|region| ResponseItemId::new(region.id.as_str()))
        .collect();
    validate_hotspot_region_ids(selections, &available, issues);
}
