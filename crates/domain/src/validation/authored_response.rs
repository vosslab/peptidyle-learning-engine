//! Validates responses against authored Question Response Formats.

use std::collections::BTreeSet;

use question_model::answer::ResponseSelectionRule;
use question_model::response::{
    HotspotRegion, QuestionResponseFormat, ResponseItemId, StudentResponse, TextEntrySlot,
};

use super::{
    StudentResponseFormatCheck, StudentResponseFormatIssue, count, validate_hotspot_region_ids,
    validate_text_length,
};

/// Validates student-controlled response structure without consulting a key.
///
/// The server runs this function before grading. The browser calls the same
/// implementation through `wasm_bridge` for immediate format feedback.
pub fn validate_response_format(
    response_format: &QuestionResponseFormat,
    response: &StudentResponse,
) -> StudentResponseFormatCheck {
    let mut issues = Vec::new();

    match (response_format, response) {
        (QuestionResponseFormat::Numeric { .. }, StudentResponse::Numeric { value }) => {
            if !value.is_finite() {
                issues.push(StudentResponseFormatIssue::NumericNotFinite);
            }
        }
        (
            QuestionResponseFormat::MultipleChoice { choices, selection },
            StudentResponse::MultipleChoice { selected },
        ) => validate_selection(choices, *selection, selected, &mut issues),
        (
            QuestionResponseFormat::ShortText { max_length, .. },
            StudentResponse::ShortText { text },
        ) => {
            validate_text_length(*max_length, text, &mut issues);
        }
        (
            QuestionResponseFormat::MultiBlank { blanks },
            StudentResponse::MultiBlank { answers },
        ) => {
            validate_multi_blank(blanks, answers, &mut issues);
        }
        (
            QuestionResponseFormat::Matching { prompts, choices },
            StudentResponse::Matching { matches },
        ) => validate_matching(prompts, choices, matches, &mut issues),
        (QuestionResponseFormat::Ordering { items }, StudentResponse::Ordering { order }) => {
            let expected: BTreeSet<ResponseItemId> =
                items.iter().map(|item| item.id.clone()).collect();
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
            QuestionResponseFormat::Hotspot {
                regions, selection, ..
            },
            StudentResponse::Hotspot { selections },
        ) => validate_hotspot(regions, *selection, selections, &mut issues),
        (
            QuestionResponseFormat::ImathasQuestionBackend {},
            StudentResponse::ImathasQuestionBackend {},
        ) => {}
        (QuestionResponseFormat::BackendOwned {}, StudentResponse::BackendOwned { .. }) => {}
        _ => issues.push(StudentResponseFormatIssue::ResponseKindMismatch),
    }

    StudentResponseFormatCheck { issues }
}

fn validate_multi_blank(
    blanks: &[TextEntrySlot],
    answers: &[question_model::response::StudentTextEntry],
    issues: &mut Vec<StudentResponseFormatIssue>,
) {
    let expected: BTreeSet<_> = blanks.iter().map(|blank| blank.id.clone()).collect();
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
            .find(|blank| blank.id == answer.slot)
            .expect("validated response slots exist in the Question");
        validate_text_length(blank.max_length, &answer.text, issues);
    }
}

fn validate_matching(
    prompts: &[question_model::response::MatchingPrompt],
    choices: &[question_model::response::MatchingChoice],
    matches: &[question_model::response::StudentMatch],
    issues: &mut Vec<StudentResponseFormatIssue>,
) {
    let expected_prompts: BTreeSet<_> = prompts.iter().map(|prompt| prompt.id.clone()).collect();
    let actual_prompts: BTreeSet<_> = matches.iter().map(|pair| pair.prompt.clone()).collect();
    if expected_prompts.len() != prompts.len()
        || actual_prompts.len() != matches.len()
        || matches.len() > prompts.len()
        || !actual_prompts.is_subset(&expected_prompts)
    {
        issues.push(StudentResponseFormatIssue::MatchingPromptsMismatch);
    }
    let available_choices: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
    let mut observed = BTreeSet::new();
    for pair in matches {
        if !available_choices.contains(&pair.choice) {
            issues.push(StudentResponseFormatIssue::UnknownMatchChoice {
                choice: pair.choice.clone(),
            });
        }
        if !observed.insert(pair.choice.clone()) {
            issues.push(StudentResponseFormatIssue::DuplicateMatchChoice {
                choice: pair.choice.clone(),
            });
        }
    }
}

fn validate_hotspot(
    regions: &[HotspotRegion],
    selection: ResponseSelectionRule,
    selections: &[question_model::response::StudentHotspotSelection],
    issues: &mut Vec<StudentResponseFormatIssue>,
) {
    validate_selection_count(selection, selections.len(), issues);
    let available: BTreeSet<_> = regions.iter().map(|region| region.id.clone()).collect();
    validate_hotspot_region_ids(selections, &available, issues);
}

fn validate_selection_count(
    selection: ResponseSelectionRule,
    actual: usize,
    issues: &mut Vec<StudentResponseFormatIssue>,
) {
    let valid = match selection {
        ResponseSelectionRule::ExactlyOne => actual == 1,
        ResponseSelectionRule::Exactly { count } => actual == count as usize,
        ResponseSelectionRule::AnyNumber => true,
        ResponseSelectionRule::AtLeastOne => actual >= 1,
    };
    if !valid {
        issues.push(StudentResponseFormatIssue::SelectionCount {
            expected: selection,
            actual: actual as u64,
        });
    }
}

fn validate_selection(
    choices: &[question_model::response::QuestionChoice],
    selection: ResponseSelectionRule,
    selected: &[ResponseItemId],
    issues: &mut Vec<StudentResponseFormatIssue>,
) {
    let actual = count(selected.iter());
    let cardinality_matches = match selection {
        ResponseSelectionRule::ExactlyOne => actual == 1,
        ResponseSelectionRule::Exactly { count } => actual == u64::from(count),
        ResponseSelectionRule::AnyNumber => true,
        ResponseSelectionRule::AtLeastOne => actual >= 1,
    };
    if !cardinality_matches {
        issues.push(StudentResponseFormatIssue::SelectionCount {
            expected: selection,
            actual,
        });
    }

    let available: BTreeSet<ResponseItemId> =
        choices.iter().map(|choice| choice.id.clone()).collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::answer::NumericResponseTolerance;

    #[test]
    fn numeric_input_must_be_finite() {
        let response_format = QuestionResponseFormat::Numeric {
            tolerance: NumericResponseTolerance::Absolute { epsilon: 0.1 },
            unit: None,
        };
        assert_eq!(
            validate_response_format(
                &response_format,
                &StudentResponse::Numeric { value: f64::NAN }
            )
            .issues,
            vec![StudentResponseFormatIssue::NumericNotFinite]
        );
    }
}
