//! Shape, key, and evaluation helpers for PLE Question JSON Private Grading.

use std::collections::{BTreeSet, HashSet};

use question_model::QuestionContentBlock;
use question_model::QuestionEvaluation;
use question_model::answer::ResponseSelectionRule;
use question_model::response::{
    QuestionResponseFormat, QuestionType, ResponseItemId, StudentResponse,
};

use crate::AnswerKey;

use super::{
    MAX_CHOICE_ID_BYTES, MAX_CHOICES, MAX_FEEDBACK_CHARS, PleQuestionJsonError,
    PleQuestionJsonGradingError,
};

pub(super) fn validate_response_for_type(
    question_type: QuestionType,
    response: &QuestionResponseFormat,
) -> Result<(), PleQuestionJsonError> {
    match (question_type, response) {
        (
            QuestionType::MultipleChoice,
            QuestionResponseFormat::MultipleChoice { choices, selection },
        ) if *selection == ResponseSelectionRule::ExactlyOne => validate_options(choices, 2),
        (
            QuestionType::MultipleAnswer,
            QuestionResponseFormat::MultipleChoice { choices, selection },
        ) if *selection == ResponseSelectionRule::AtLeastOne => validate_options(choices, 2),
        (QuestionType::FillInBlank, QuestionResponseFormat::ShortText { max_length, .. })
            if *max_length > 0 =>
        {
            Ok(())
        }
        (QuestionType::MultipleFillInBlank, QuestionResponseFormat::MultiBlank { blanks })
            if !blanks.is_empty() && blanks.len() <= 50 =>
        {
            let mut ids = HashSet::new();
            for blank in blanks {
                validate_choice_id(blank.id.as_str())?;
                if blank.max_length == 0 || !ids.insert(blank.id.as_str()) {
                    return invalid(
                        "PLE Question JSON multi-blank slots must be unique and nonempty",
                    );
                }
            }
            Ok(())
        }
        (QuestionType::Numeric, QuestionResponseFormat::Numeric { tolerance, .. }) => {
            validate_numeric_tolerance(tolerance)
        }
        (QuestionType::Matching, QuestionResponseFormat::Matching { prompts, choices })
            if prompts.len() >= 2 && prompts.len() <= choices.len() =>
        {
            validate_options(prompts, 2)?;
            validate_options(choices, 2)
        }
        (QuestionType::Ordering, QuestionResponseFormat::Ordering { items }) => {
            validate_options(items, 3)
        }
        (
            QuestionType::Hotspot,
            QuestionResponseFormat::Hotspot {
                surface,
                description,
                regions,
                selection,
            },
        ) if !regions.is_empty() => {
            if description.trim().is_empty()
                || !is_hex_sha256(&surface.checksum)
                || matches!(selection, ResponseSelectionRule::AnyNumber)
            {
                return invalid("PLE Question JSON hotspot surface or selection is invalid");
            }
            let mut ids = HashSet::new();
            for region in regions {
                validate_choice_id(region.id.as_str())?;
                if !ids.insert(region.id.as_str())
                    || region.label.is_empty()
                    || region.width == 0
                    || region.height == 0
                    || u32::from(region.x) + u32::from(region.width) > 10_000
                    || u32::from(region.y) + u32::from(region.height) > 10_000
                {
                    return invalid("PLE Question JSON hotspot region is invalid");
                }
            }
            Ok(())
        }
        _ => invalid("PLE Question JSON Type and Question Response Format do not agree"),
    }
}

/// Evaluates one structurally valid PLE response without a generic Question
/// container or Assessment scoring rule. PLE Question JSON evaluates each
/// valid response all-or-nothing, returning normalized credit of zero or one.
pub(super) fn evaluate_response(
    response_format: &QuestionResponseFormat,
    response: &StudentResponse,
    key: &AnswerKey,
) -> Result<QuestionEvaluation, PleQuestionJsonError> {
    let check = domain::validation::validate_response_format(response_format, response);
    if !check.is_valid() {
        return Err(PleQuestionJsonError::Grading(
            PleQuestionJsonGradingError::InvalidResponse(check.issues),
        ));
    }
    let correct = match (response_format, response, key) {
        (
            QuestionResponseFormat::Numeric { tolerance, .. },
            StudentResponse::Numeric { value },
            AnswerKey::Numeric { expected },
        ) => numeric_is_correct(*value, *expected, tolerance)?,
        (
            QuestionResponseFormat::MultipleChoice { choices, .. },
            StudentResponse::MultipleChoice { selected },
            AnswerKey::MultipleChoice { correct },
        ) => {
            let available: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
            if !correct.is_subset(&available) {
                return Err(invalid_grading(
                    "multiple-choice key names an unavailable choice",
                ));
            }
            selected.iter().cloned().collect::<BTreeSet<_>>() == *correct
        }
        (
            QuestionResponseFormat::ShortText { match_mode, .. },
            StudentResponse::ShortText { text },
            AnswerKey::ShortText { accepted },
        ) => accepted
            .iter()
            .any(|value| text_matches(text, value, *match_mode)),
        (
            QuestionResponseFormat::MultiBlank { blanks },
            StudentResponse::MultiBlank { answers },
            AnswerKey::MultiBlank { accepted },
        ) => {
            if accepted.len() != blanks.len()
                || blanks.iter().any(|blank| !accepted.contains_key(&blank.id))
            {
                return Err(invalid_grading(
                    "multi-blank key must name every available slot exactly once",
                ));
            }
            answers.iter().all(|answer| {
                let blank = blanks
                    .iter()
                    .find(|blank| blank.id == answer.slot)
                    .expect("format validation proved the slot set");
                accepted.get(&answer.slot).is_some_and(|values| {
                    values
                        .iter()
                        .any(|value| text_matches(&answer.text, value, blank.match_mode))
                })
            })
        }
        (
            QuestionResponseFormat::Matching { prompts, choices },
            StudentResponse::Matching { matches },
            AnswerKey::Matching { correct },
        ) => {
            let prompts: BTreeSet<_> = prompts.iter().map(|prompt| prompt.id.clone()).collect();
            let choices: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
            if correct.len() != prompts.len()
                || correct.keys().cloned().collect::<BTreeSet<_>>() != prompts
                || correct.values().any(|choice| !choices.contains(choice))
            {
                return Err(invalid_grading(
                    "matching key must bind every prompt to an available choice",
                ));
            }
            matches
                .iter()
                .all(|pair| correct.get(&pair.prompt) == Some(&pair.choice))
        }
        (
            QuestionResponseFormat::Ordering { items },
            StudentResponse::Ordering { order },
            AnswerKey::Ordering { correct },
        ) => {
            let available: BTreeSet<_> = items.iter().map(|item| item.id.clone()).collect();
            let keyed: BTreeSet<_> = correct.iter().cloned().collect();
            if keyed.len() != correct.len() || keyed != available {
                return Err(invalid_grading(
                    "ordering key must contain every available item exactly once",
                ));
            }
            order == correct
        }
        (
            QuestionResponseFormat::Hotspot { regions, .. },
            StudentResponse::Hotspot { selections },
            AnswerKey::Hotspot { correct },
        ) => {
            let available: BTreeSet<_> = regions.iter().map(|region| region.id.clone()).collect();
            if !correct.is_subset(&available) {
                return Err(invalid_grading("hotspot key names an unavailable region"));
            }
            selections
                .iter()
                .map(|selection| selection.region.clone())
                .collect::<BTreeSet<_>>()
                == *correct
        }
        _ => {
            return Err(PleQuestionJsonError::Grading(
                PleQuestionJsonGradingError::KindMismatch,
            ));
        }
    };
    QuestionEvaluation::new(correct, f64::from(correct)).map_err(|error| {
        PleQuestionJsonError::Grading(PleQuestionJsonGradingError::InvalidSource(
            error.to_string(),
        ))
    })
}

fn invalid_grading(message: &str) -> PleQuestionJsonError {
    PleQuestionJsonError::Grading(PleQuestionJsonGradingError::InvalidSource(
        message.to_string(),
    ))
}

fn numeric_is_correct(
    actual: f64,
    expected: f64,
    tolerance: &question_model::answer::NumericResponseTolerance,
) -> Result<bool, PleQuestionJsonError> {
    if !expected.is_finite() {
        return Err(invalid_grading("numeric key must be finite"));
    }
    match tolerance {
        question_model::answer::NumericResponseTolerance::Exact => Ok(actual == expected),
        question_model::answer::NumericResponseTolerance::Absolute { epsilon } => {
            finite_nonnegative("absolute epsilon", *epsilon)?;
            Ok((actual - expected).abs() <= *epsilon)
        }
        question_model::answer::NumericResponseTolerance::Relative { fraction } => {
            finite_nonnegative("relative fraction", *fraction)?;
            Ok((actual - expected).abs() <= expected.abs() * *fraction)
        }
        question_model::answer::NumericResponseTolerance::SignificantFigures { digits } => {
            if *digits == 0 {
                return Err(invalid_grading("significant figures must be at least one"));
            }
            Ok(round_significant(actual, *digits) == round_significant(expected, *digits))
        }
    }
}

fn finite_nonnegative(name: &str, value: f64) -> Result<(), PleQuestionJsonError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(invalid_grading(&format!(
            "{name} must be finite and nonnegative"
        )))
    }
}

fn round_significant(value: f64, digits: u8) -> f64 {
    if value == 0.0 {
        return 0.0;
    }
    let scale = 10_f64.powi(digits as i32 - 1 - value.abs().log10().floor() as i32);
    (value * scale).round() / scale
}

fn text_matches(
    actual: &str,
    expected: &str,
    rule: question_model::answer::TextResponseMatchRule,
) -> bool {
    match rule {
        question_model::answer::TextResponseMatchRule::Exact => actual == expected,
        question_model::answer::TextResponseMatchRule::CaseInsensitive => {
            actual.eq_ignore_ascii_case(expected)
        }
        question_model::answer::TextResponseMatchRule::Normalized => {
            normalize_text(actual) == normalize_text(expected)
        }
    }
}

fn normalize_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

trait SelectableResponseItem {
    fn id(&self) -> &question_model::response::ResponseItemId;
    fn body(&self) -> &[QuestionContentBlock];
}

impl SelectableResponseItem for question_model::response::QuestionChoice {
    fn id(&self) -> &question_model::response::ResponseItemId {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

impl SelectableResponseItem for question_model::response::MatchingPrompt {
    fn id(&self) -> &question_model::response::ResponseItemId {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

impl SelectableResponseItem for question_model::response::MatchingChoice {
    fn id(&self) -> &question_model::response::ResponseItemId {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

impl SelectableResponseItem for question_model::response::OrderingItem {
    fn id(&self) -> &question_model::response::ResponseItemId {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

fn validate_options<T: SelectableResponseItem>(
    choices: &[T],
    minimum: usize,
) -> Result<(), PleQuestionJsonError> {
    if choices.len() < minimum || choices.len() > MAX_CHOICES {
        return invalid("PLE Question JSON selectable item count is outside the supported range");
    }
    let mut identifiers = HashSet::new();
    for choice in choices {
        validate_choice_id(choice.id().as_str())?;
        if choice.body().is_empty() || !identifiers.insert(choice.id().as_str()) {
            return invalid(
                "PLE Question JSON selectable item identifiers and bodies must be valid",
            );
        }
    }
    Ok(())
}

fn validate_numeric_tolerance(
    tolerance: &question_model::answer::NumericResponseTolerance,
) -> Result<(), PleQuestionJsonError> {
    match tolerance {
        question_model::answer::NumericResponseTolerance::Exact => Ok(()),
        question_model::answer::NumericResponseTolerance::Absolute { epsilon } => {
            validate_nonnegative_finite("absolute epsilon", *epsilon)
        }
        question_model::answer::NumericResponseTolerance::Relative { fraction } => {
            validate_nonnegative_finite("relative fraction", *fraction)
        }
        question_model::answer::NumericResponseTolerance::SignificantFigures { digits }
            if *digits > 0 =>
        {
            Ok(())
        }
        question_model::answer::NumericResponseTolerance::SignificantFigures { .. } => {
            invalid("significant figures must be at least one")
        }
    }
}

fn validate_nonnegative_finite(name: &str, value: f64) -> Result<(), PleQuestionJsonError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        invalid(&format!("{name} must be finite and nonnegative"))
    }
}

pub(super) fn validate_key_against_response(
    response: &QuestionResponseFormat,
    key: &AnswerKey,
) -> Result<(), PleQuestionJsonError> {
    match (response, key) {
        (QuestionResponseFormat::Numeric { .. }, AnswerKey::Numeric { expected })
            if expected.is_finite() =>
        {
            Ok(())
        }
        (
            QuestionResponseFormat::MultipleChoice { choices, .. },
            AnswerKey::MultipleChoice { correct },
        ) => {
            let available: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
            if correct.is_empty() || !correct.is_subset(&available) {
                return Err(PleQuestionJsonError::PublicContentChecksumMismatch);
            }
            Ok(())
        }
        (QuestionResponseFormat::ShortText { .. }, AnswerKey::ShortText { accepted })
            if !accepted.is_empty() =>
        {
            Ok(())
        }
        (QuestionResponseFormat::MultiBlank { blanks }, AnswerKey::MultiBlank { accepted }) => {
            let available: BTreeSet<_> = blanks.iter().map(|blank| blank.id.clone()).collect();
            if accepted.len() != available.len()
                || accepted.keys().cloned().collect::<BTreeSet<_>>() != available
                || accepted.values().any(Vec::is_empty)
            {
                return Err(PleQuestionJsonError::PublicContentChecksumMismatch);
            }
            Ok(())
        }
        (
            QuestionResponseFormat::Matching { prompts, choices },
            AnswerKey::Matching { correct },
        ) => {
            let prompt_ids: BTreeSet<_> = prompts.iter().map(|prompt| prompt.id.clone()).collect();
            let choice_ids: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
            let correct_choices: BTreeSet<_> = correct.values().cloned().collect();
            if correct.keys().cloned().collect::<BTreeSet<_>>() != prompt_ids
                || correct_choices.len() != correct.len()
                || !correct_choices.is_subset(&choice_ids)
            {
                return Err(PleQuestionJsonError::PublicContentChecksumMismatch);
            }
            Ok(())
        }
        (QuestionResponseFormat::Ordering { items }, AnswerKey::Ordering { correct }) => {
            let available: BTreeSet<_> = items.iter().map(|item| item.id.clone()).collect();
            let keyed: BTreeSet<_> = correct.iter().cloned().collect();
            if keyed.len() != correct.len() || keyed != available {
                return Err(PleQuestionJsonError::PublicContentChecksumMismatch);
            }
            Ok(())
        }
        (QuestionResponseFormat::Hotspot { regions, .. }, AnswerKey::Hotspot { correct }) => {
            let available: BTreeSet<_> = regions.iter().map(|region| region.id.clone()).collect();
            if correct.is_empty() || !correct.is_subset(&available) {
                return Err(PleQuestionJsonError::PublicContentChecksumMismatch);
            }
            Ok(())
        }
        _ => Err(PleQuestionJsonError::PublicContentChecksumMismatch),
    }
}

pub(super) fn selectable_ids(response: &QuestionResponseFormat) -> BTreeSet<ResponseItemId> {
    match response {
        QuestionResponseFormat::MultipleChoice { choices, .. } => {
            choices.iter().map(|choice| choice.id.clone()).collect()
        }
        QuestionResponseFormat::Matching { choices, .. } => {
            choices.iter().map(|choice| choice.id.clone()).collect()
        }
        QuestionResponseFormat::Ordering { items } => {
            items.iter().map(|item| item.id.clone()).collect()
        }
        QuestionResponseFormat::Hotspot { regions, .. } => {
            regions.iter().map(|region| region.id.clone()).collect()
        }
        QuestionResponseFormat::Numeric { .. }
        | QuestionResponseFormat::ShortText { .. }
        | QuestionResponseFormat::MultiBlank { .. }
        | QuestionResponseFormat::ImathasQuestionBackend {}
        | QuestionResponseFormat::BackendOwned {} => BTreeSet::new(),
    }
}

pub(super) fn question_answer_blocks(
    response: &QuestionResponseFormat,
    key: &AnswerKey,
) -> Result<Vec<QuestionContentBlock>, PleQuestionJsonError> {
    validate_key_against_response(response, key)?;
    let blocks = match (response, key) {
        (
            QuestionResponseFormat::MultipleChoice { choices, .. },
            AnswerKey::MultipleChoice { correct },
        ) => choices
            .iter()
            .filter(|choice| correct.contains(&choice.id))
            .flat_map(|choice| choice.body.clone())
            .collect(),
        (QuestionResponseFormat::ShortText { .. }, AnswerKey::ShortText { accepted }) => {
            markdown_blocks(&accepted.join("; "))
        }
        (QuestionResponseFormat::Numeric { unit, .. }, AnswerKey::Numeric { expected }) => {
            markdown_blocks(&format!(
                "{expected}{}",
                unit.as_deref()
                    .map_or(String::new(), |unit| format!(" {unit}"))
            ))
        }
        (QuestionResponseFormat::MultiBlank { blanks }, AnswerKey::MultiBlank { accepted }) => {
            vec![QuestionContentBlock::Table {
                headers: vec!["Blank".to_string(), "Accepted response".to_string()],
                rows: blanks
                    .iter()
                    .map(|blank| vec![blocks_text(&blank.label), accepted[&blank.id].join("; ")])
                    .collect(),
                description: "Accepted responses for each blank".to_string(),
            }]
        }
        (
            QuestionResponseFormat::Matching { prompts, choices },
            AnswerKey::Matching { correct },
        ) => {
            vec![QuestionContentBlock::Table {
                headers: vec!["Prompt".to_string(), "Match".to_string()],
                rows: prompts
                    .iter()
                    .map(|prompt| {
                        let choice_id = &correct[&prompt.id];
                        let choice = choices
                            .iter()
                            .find(|choice| &choice.id == choice_id)
                            .expect("validated matching key names an available choice");
                        vec![blocks_text(&prompt.body), blocks_text(&choice.body)]
                    })
                    .collect(),
                description: "Correct prompt and choice matches".to_string(),
            }]
        }
        (QuestionResponseFormat::Ordering { items }, AnswerKey::Ordering { correct }) => correct
            .iter()
            .flat_map(|id| {
                items
                    .iter()
                    .find(|item| &item.id == id)
                    .expect("validated ordering key names an available item")
                    .body
                    .clone()
            })
            .collect(),
        (QuestionResponseFormat::Hotspot { regions, .. }, AnswerKey::Hotspot { correct }) => {
            regions
                .iter()
                .filter(|region| correct.contains(&region.id))
                .flat_map(|region| region.label.clone())
                .collect()
        }
        _ => return Err(PleQuestionJsonError::PublicContentChecksumMismatch),
    };
    Ok(blocks)
}

fn blocks_text(blocks: &[QuestionContentBlock]) -> String {
    blocks
        .iter()
        .map(|block| match block {
            QuestionContentBlock::Text { markdown } => markdown.as_str(),
            QuestionContentBlock::Math { description, .. }
            | QuestionContentBlock::Image { description, .. }
            | QuestionContentBlock::Table { description, .. } => description.as_str(),
            QuestionContentBlock::Code { source, .. } => source.as_str(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn markdown_blocks(markdown: &str) -> Vec<QuestionContentBlock> {
    vec![QuestionContentBlock::Text {
        markdown: markdown.to_string(),
    }]
}
pub(super) fn validate_choice_id(value: &str) -> Result<(), PleQuestionJsonError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_CHOICE_ID_BYTES
        || !bytes[0].is_ascii_lowercase()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(byte))
    {
        return invalid(
            "choice IDs must start with a lowercase letter and contain only lowercase ASCII letters, digits, underscores, or hyphens",
        );
    }
    Ok(())
}
pub(super) fn validate_feedback(value: &str) -> Result<(), PleQuestionJsonError> {
    validate_bounded_text("feedback", value, MAX_FEEDBACK_CHARS)
}
pub(super) fn validate_optional_feedback(value: Option<&str>) -> Result<(), PleQuestionJsonError> {
    if let Some(value) = value {
        validate_feedback(value)?;
    }
    Ok(())
}
fn validate_bounded_text(
    name: &str,
    value: &str,
    maximum_chars: usize,
) -> Result<(), PleQuestionJsonError> {
    if value.trim().is_empty() {
        return invalid(&format!("{name} must not be blank"));
    }
    if value.chars().count() > maximum_chars {
        return invalid(&format!("{name} exceeds {maximum_chars} characters"));
    }
    Ok(())
}
pub(super) fn is_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}
pub(super) fn invalid<T>(message: &str) -> Result<T, PleQuestionJsonError> {
    Err(PleQuestionJsonError::InvalidDocument(message.to_string()))
}
