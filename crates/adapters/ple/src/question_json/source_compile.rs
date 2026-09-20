//! Compile and validate type-specific PLE Question JSON response objects.

use std::collections::HashSet;

use grading::AnswerKey;
use question_model::answer::ResponseSelectionRule;
use question_model::response::{
    HotspotRegion, MatchingChoice, MatchingPrompt, OrderingItem, QuestionChoice,
    QuestionResponseFormat, ResponseItemId, TextEntrySlot,
};
use question_model::{
    NativeChoiceOrder, QuestionContentBlock, QuestionImageAssetId, QuestionImageAssetTuple,
};
use uuid::Uuid;

use super::super::{
    MAX_CHOICE_TEXT_CHARS, MAX_CHOICES, MAX_FEEDBACK_CHARS, MAX_METADATA_TEXT_CHARS,
    PleQuestionJsonChoice, PleQuestionJsonError, invalid, markdown_blocks, validate_bounded_text,
    validate_choice_id, validate_markdown, validate_optional_feedback,
};
use super::{
    MAX_BLANKS, MAX_TEXT_RESPONSE_CHARS, PleQuestionJsonBlank, PleQuestionJsonHotspotRegion,
    PleQuestionJsonHotspotSurface, PleQuestionJsonMatch, PleQuestionJsonMatchingChoice,
    PleQuestionJsonMatchingPrompt, PleQuestionJsonNumericResponseTolerance,
    PleQuestionJsonOrderingItem, PleQuestionJsonResponse, PleQuestionJsonResponseMember,
};

type CompiledResponse = (
    QuestionResponseFormat,
    NativeChoiceOrder,
    AnswerKey,
    Vec<(ResponseItemId, String)>,
    Vec<QuestionContentBlock>,
);

pub(super) fn compile_response(
    response: &PleQuestionJsonResponse,
) -> Result<CompiledResponse, PleQuestionJsonError> {
    let compiled = match response {
        PleQuestionJsonResponse::SingleChoice {
            choices,
            correct_choice,
            randomize_choices,
        } => compile_choices(
            choices,
            ResponseSelectionRule::ExactlyOne,
            std::slice::from_ref(correct_choice),
            *randomize_choices,
        ),
        PleQuestionJsonResponse::MultipleAnswer {
            choices,
            correct_choices,
            randomize_choices,
        } => compile_choices(
            choices,
            ResponseSelectionRule::AtLeastOne,
            correct_choices,
            *randomize_choices,
        ),
        PleQuestionJsonResponse::FillIn {
            answers,
            match_mode,
            max_length,
        } => (
            QuestionResponseFormat::ShortText {
                match_mode: (*match_mode).into(),
                max_length: *max_length,
            },
            NativeChoiceOrder::Fixed,
            AnswerKey::ShortText {
                accepted: answers.clone(),
            },
            Vec::new(),
            Vec::new(),
        ),
        PleQuestionJsonResponse::MultiFillIn { blanks } => (
            QuestionResponseFormat::MultiBlank {
                blanks: blanks
                    .iter()
                    .map(|blank| TextEntrySlot {
                        id: ResponseItemId::new(&blank.id),
                        label: markdown_blocks(&blank.label),
                        match_mode: blank.match_mode.into(),
                        max_length: blank.max_length,
                    })
                    .collect(),
            },
            NativeChoiceOrder::Fixed,
            AnswerKey::MultiBlank {
                accepted: blanks
                    .iter()
                    .map(|blank| (ResponseItemId::new(&blank.id), blank.answers.clone()))
                    .collect(),
            },
            Vec::new(),
            Vec::new(),
        ),
        PleQuestionJsonResponse::Numeric {
            answer,
            tolerance,
            unit,
        } => (
            QuestionResponseFormat::Numeric {
                tolerance: tolerance.into(),
                unit: unit.clone(),
            },
            NativeChoiceOrder::Fixed,
            AnswerKey::Numeric { expected: *answer },
            Vec::new(),
            Vec::new(),
        ),
        PleQuestionJsonResponse::Matching {
            prompts,
            choices,
            matches,
        } => (
            QuestionResponseFormat::Matching {
                prompts: compile_matching_prompts(prompts),
                choices: compile_matching_choices(choices),
            },
            NativeChoiceOrder::Fixed,
            AnswerKey::Matching {
                correct: matches
                    .iter()
                    .map(|pair| {
                        (
                            ResponseItemId::new(&pair.prompt),
                            ResponseItemId::new(&pair.choice),
                        )
                    })
                    .collect(),
            },
            Vec::new(),
            Vec::new(),
        ),
        PleQuestionJsonResponse::Ordering {
            items,
            correct_order,
        } => (
            QuestionResponseFormat::Ordering {
                items: compile_ordering_items(items),
            },
            NativeChoiceOrder::Fixed,
            AnswerKey::Ordering {
                correct: correct_order.iter().map(ResponseItemId::new).collect(),
            },
            Vec::new(),
            Vec::new(),
        ),
        PleQuestionJsonResponse::Hotspot {
            surface,
            regions,
            correct_regions,
        } => {
            let question_image_asset_tuple = QuestionImageAssetTuple {
                question_image_asset_id: QuestionImageAssetId::from_uuid(
                    Uuid::parse_str(&surface.question_image_asset_id).map_err(|_| {
                        PleQuestionJsonError::InvalidDocument(
                            "hotspot question asset must be a UUID".to_string(),
                        )
                    })?,
                ),
                checksum: surface.checksum.clone(),
            };
            (
                QuestionResponseFormat::Hotspot {
                    question_image_asset_tuple: question_image_asset_tuple.clone(),
                    description: surface.description.clone(),
                    regions: regions.iter().map(compile_region).collect(),
                    // Correct-region cardinality is private Answer Key data.
                    // The public format requires a nonempty selection without
                    // disclosing how many regions the answer key contains.
                    selection: ResponseSelectionRule::AtLeastOne,
                },
                NativeChoiceOrder::Fixed,
                AnswerKey::Hotspot {
                    correct: correct_regions.iter().map(ResponseItemId::new).collect(),
                },
                Vec::new(),
                vec![QuestionContentBlock::Image {
                    question_image_asset_tuple,
                    description: surface.description.clone(),
                }],
            )
        }
    };
    Ok(compiled)
}

fn compile_choices(
    choices: &[PleQuestionJsonChoice],
    selection: ResponseSelectionRule,
    correct: &[String],
    randomize_choices: bool,
) -> CompiledResponse {
    (
        QuestionResponseFormat::MultipleChoice {
            choices: choices
                .iter()
                .map(|choice| QuestionChoice {
                    id: ResponseItemId::new(&choice.id),
                    body: markdown_blocks(&choice.text),
                })
                .collect(),
            selection,
        },
        if randomize_choices {
            NativeChoiceOrder::NonceRandomized
        } else {
            NativeChoiceOrder::Fixed
        },
        AnswerKey::MultipleChoice {
            correct: correct.iter().map(ResponseItemId::new).collect(),
        },
        choices
            .iter()
            .filter_map(|choice| {
                choice
                    .feedback
                    .as_ref()
                    .map(|feedback| (ResponseItemId::new(&choice.id), feedback.clone()))
            })
            .collect(),
        Vec::new(),
    )
}

fn compile_matching_prompts(items: &[PleQuestionJsonMatchingPrompt]) -> Vec<MatchingPrompt> {
    items
        .iter()
        .map(|item| MatchingPrompt {
            id: ResponseItemId::new(&item.id),
            body: markdown_blocks(&item.text),
        })
        .collect()
}

fn compile_matching_choices(items: &[PleQuestionJsonMatchingChoice]) -> Vec<MatchingChoice> {
    items
        .iter()
        .map(|item| MatchingChoice {
            id: ResponseItemId::new(&item.id),
            body: markdown_blocks(&item.text),
        })
        .collect()
}

fn compile_ordering_items(items: &[PleQuestionJsonOrderingItem]) -> Vec<OrderingItem> {
    items
        .iter()
        .map(|item| OrderingItem {
            id: ResponseItemId::new(&item.id),
            body: markdown_blocks(&item.text),
        })
        .collect()
}

fn compile_region(region: &PleQuestionJsonHotspotRegion) -> HotspotRegion {
    HotspotRegion {
        id: ResponseItemId::new(&region.id),
        label: markdown_blocks(&region.label),
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
    }
}

pub(super) fn validate_choice_question(
    choices: &[PleQuestionJsonChoice],
    correct: &[String],
    exactly_one: bool,
) -> Result<(), PleQuestionJsonError> {
    if !(2..=MAX_CHOICES).contains(&choices.len()) {
        return invalid("choice questions require 2 to 100 choices");
    }
    if correct.is_empty() || (exactly_one && correct.len() != 1) {
        return invalid("correct choices do not satisfy the question cardinality");
    }
    let mut ids = HashSet::new();
    for choice in choices {
        validate_choice_id(&choice.id)?;
        if !ids.insert(choice.id.as_str()) {
            return invalid("choice identifiers must be unique");
        }
        validate_markdown("choice text", &choice.text, MAX_CHOICE_TEXT_CHARS)?;
        validate_optional_feedback(choice.feedback.as_deref())?;
    }
    let correct_set: HashSet<_> = correct.iter().map(String::as_str).collect();
    if correct_set.len() != correct.len() || !correct_set.is_subset(&ids) {
        return invalid("correct choices must be unique available choices");
    }
    Ok(())
}

pub(super) fn validate_answers(
    answers: &[String],
    max_length: u32,
) -> Result<(), PleQuestionJsonError> {
    if answers.is_empty() || max_length == 0 || max_length > MAX_TEXT_RESPONSE_CHARS {
        return invalid("text answers require accepted values and a valid maxLength");
    }
    let mut unique = HashSet::new();
    for answer in answers {
        validate_bounded_text("accepted answer", answer, MAX_FEEDBACK_CHARS)?;
        if !unique.insert(answer) {
            return invalid("accepted answers must be unique");
        }
    }
    Ok(())
}

pub(super) fn validate_blanks(blanks: &[PleQuestionJsonBlank]) -> Result<(), PleQuestionJsonError> {
    if blanks.is_empty() || blanks.len() > MAX_BLANKS {
        return invalid("multi-fill questions require 1 to 50 blanks");
    }
    let mut ids = HashSet::new();
    for blank in blanks {
        validate_choice_id(&blank.id)?;
        if !ids.insert(blank.id.as_str()) {
            return invalid("blank identifiers must be unique");
        }
        validate_markdown("blank label", &blank.label, MAX_CHOICE_TEXT_CHARS)?;
        validate_answers(&blank.answers, blank.max_length)?;
    }
    Ok(())
}

pub(super) fn validate_numeric(
    answer: f64,
    tolerance: &PleQuestionJsonNumericResponseTolerance,
    unit: Option<&str>,
) -> Result<(), PleQuestionJsonError> {
    if !answer.is_finite() {
        return invalid("numeric answer must be finite");
    }
    match tolerance {
        PleQuestionJsonNumericResponseTolerance::Exact => {}
        PleQuestionJsonNumericResponseTolerance::Absolute { epsilon } => {
            validate_nonnegative_finite("absolute epsilon", *epsilon)?;
        }
        PleQuestionJsonNumericResponseTolerance::Relative { fraction } => {
            validate_nonnegative_finite("relative fraction", *fraction)?;
        }
        PleQuestionJsonNumericResponseTolerance::SignificantFigures { digits } if *digits == 0 => {
            return invalid("significant figures must be at least one");
        }
        PleQuestionJsonNumericResponseTolerance::SignificantFigures { .. } => {}
    }
    if let Some(unit) = unit {
        validate_bounded_text("numeric unit", unit, MAX_METADATA_TEXT_CHARS)?;
    }
    Ok(())
}

fn validate_nonnegative_finite(name: &str, value: f64) -> Result<(), PleQuestionJsonError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        invalid(&format!("{name} must be finite and nonnegative"))
    }
}

fn validate_items<T: PleQuestionJsonResponseMember>(
    name: &str,
    items: &[T],
    minimum: usize,
) -> Result<HashSet<String>, PleQuestionJsonError> {
    if items.len() < minimum || items.len() > MAX_CHOICES {
        return invalid(&format!("{name} count is outside the supported range"));
    }
    let mut ids = HashSet::new();
    for item in items {
        validate_choice_id(item.id())?;
        if !ids.insert(item.id().to_string()) {
            return invalid(&format!("{name} identifiers must be unique"));
        }
        validate_markdown(name, item.text(), MAX_CHOICE_TEXT_CHARS)?;
    }
    Ok(ids)
}

pub(super) fn validate_matching(
    prompts: &[PleQuestionJsonMatchingPrompt],
    choices: &[PleQuestionJsonMatchingChoice],
    matches: &[PleQuestionJsonMatch],
) -> Result<(), PleQuestionJsonError> {
    let prompt_ids = validate_items("matching prompt", prompts, 2)?;
    let choice_ids = validate_items("matching choice", choices, 2)?;
    if prompts.len() > choices.len() || matches.len() != prompts.len() {
        return invalid("matching requires one match per prompt and at least as many choices");
    }
    let mut matched_prompts = HashSet::new();
    let mut matched_choices = HashSet::new();
    for pair in matches {
        if !prompt_ids.contains(&pair.prompt)
            || !choice_ids.contains(&pair.choice)
            || !matched_prompts.insert(pair.prompt.as_str())
            || !matched_choices.insert(pair.choice.as_str())
        {
            return invalid("matching pairs must bind every prompt to one unique available choice");
        }
    }
    Ok(())
}

pub(super) fn validate_ordering(
    items: &[PleQuestionJsonOrderingItem],
    order: &[String],
) -> Result<(), PleQuestionJsonError> {
    let ids = validate_items("ordering item", items, 3)?;
    let order_ids: HashSet<_> = order.iter().cloned().collect();
    if order.len() != items.len() || order_ids.len() != order.len() || order_ids != ids {
        return invalid("correctOrder must contain every ordering item exactly once");
    }
    Ok(())
}

pub(super) fn validate_hotspot(
    surface: &PleQuestionJsonHotspotSurface,
    regions: &[PleQuestionJsonHotspotRegion],
    correct: &[String],
) -> Result<(), PleQuestionJsonError> {
    Uuid::parse_str(&surface.question_image_asset_id).map_err(|_| {
        PleQuestionJsonError::InvalidDocument("hotspot question asset must be a UUID".to_string())
    })?;
    if surface.checksum.len() != 64
        || !surface
            .checksum
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return invalid("hotspot checksum must be lowercase SHA-256 hex");
    }
    validate_bounded_text(
        "hotspot description",
        &surface.description,
        MAX_CHOICE_TEXT_CHARS,
    )?;
    if regions.is_empty() || regions.len() > MAX_CHOICES || correct.is_empty() {
        return invalid("hotspot questions require Hotspot Regions and correct regions");
    }
    let mut ids = HashSet::new();
    for region in regions {
        validate_choice_id(&region.id)?;
        if !ids.insert(region.id.as_str()) {
            return invalid("hotspot region identifiers must be unique");
        }
        validate_bounded_text("hotspot region label", &region.label, MAX_CHOICE_TEXT_CHARS)?;
        if region.width == 0
            || region.height == 0
            || u32::from(region.x) + u32::from(region.width) > 10_000
            || u32::from(region.y) + u32::from(region.height) > 10_000
        {
            return invalid("hotspot regions must be nonempty normalized rectangles");
        }
    }
    for (index, left) in regions.iter().enumerate() {
        if regions[index + 1..]
            .iter()
            .any(|right| regions_overlap(left, right))
        {
            return invalid("hotspot regions must not overlap");
        }
    }
    let correct_ids: HashSet<_> = correct.iter().map(String::as_str).collect();
    if correct_ids.len() != correct.len() || !correct_ids.is_subset(&ids) {
        return invalid("correctRegions must be unique available regions");
    }
    Ok(())
}

fn regions_overlap(
    left: &PleQuestionJsonHotspotRegion,
    right: &PleQuestionJsonHotspotRegion,
) -> bool {
    let left_right = u32::from(left.x) + u32::from(left.width);
    let left_bottom = u32::from(left.y) + u32::from(left.height);
    let right_right = u32::from(right.x) + u32::from(right.width);
    let right_bottom = u32::from(right.y) + u32::from(right.height);
    u32::from(left.x) <= right_right
        && u32::from(right.x) <= left_right
        && u32::from(left.y) <= right_bottom
        && u32::from(right.y) <= left_bottom
}
