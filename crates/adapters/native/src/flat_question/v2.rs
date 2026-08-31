//! Closed version 2 source shapes for all supported PLE flat-question types.

use std::collections::HashSet;

use grading::AnswerKey;
use question_model::answer::{
    NumericResponseTolerance, ResponseSelectionRule, TextResponseMatchRule,
};
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::generation::RandomizationDefinition;
use question_model::response::{
    ResponseItemReference, ChoiceOption, HotspotRegion, QuestionResponseFormat, QuestionType, TextEntrySlot,
};
use question_model::taxonomy::{License, Tag, TaxonomyTerm};
use question_model::{
    AssetId, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, QuestionFormat,
    QuestionMetadata, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    CompiledFlatQuestion, FORMAT_NAME, FlatChoice, FlatLicense, FlatOutcomeFeedback,
    FlatQuestionAttemptLimit, FlatQuestionAttemptTimeLimit, FlatQuestionError, FlatQuestionPrivate,
    FlatTaxonomyTerm, MAX_CHOICE_TEXT_CHARS, MAX_CHOICES, MAX_FEEDBACK_CHARS,
    MAX_METADATA_TEXT_CHARS, MAX_PROMPT_CHARS, MAX_TAG_CHARS, invalid, markdown_blocks,
    validate_bounded_text, validate_choice_id, validate_markdown, validate_metadata_text,
    validate_optional_feedback,
};

const FORMAT_VERSION_V2: u32 = 2;
const MAX_BLANKS: usize = 50;
const MAX_TEXT_RESPONSE_CHARS: u32 = 16_384;

/// Version 2 keeps common metadata outside a closed, type-specific response object.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FlatQuestionV2 {
    format: String,
    version: u32,
    title: String,
    prompt: String,
    response: FlatResponseV2,
    #[serde(default)]
    feedback: FlatOutcomeFeedback,
    points: f64,
    question_attempt_limit: FlatQuestionAttemptLimit,
    question_attempt_time_limit: FlatQuestionAttemptTimeLimit,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    taxonomy: Vec<FlatTaxonomyTerm>,
    license: FlatLicense,
    language: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum FlatResponseV2 {
    SingleChoice {
        choices: Vec<FlatChoice>,
        correct_choice: String,
    },
    MultipleAnswer {
        choices: Vec<FlatChoice>,
        correct_choices: Vec<String>,
    },
    FillIn {
        answers: Vec<String>,
        match_mode: FlatTextResponseMatchRule,
        max_length: u32,
    },
    MultiFillIn {
        blanks: Vec<FlatBlank>,
    },
    Numeric {
        answer: f64,
        tolerance: FlatNumericResponseTolerance,
        #[serde(default)]
        unit: Option<String>,
    },
    Matching {
        prompts: Vec<FlatItem>,
        choices: Vec<FlatItem>,
        matches: Vec<FlatMatch>,
    },
    Ordering {
        items: Vec<FlatItem>,
        correct_order: Vec<String>,
    },
    Hotspot {
        surface: FlatHotspotSurface,
        regions: Vec<FlatHotspotRegion>,
        correct_regions: Vec<String>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FlatTextResponseMatchRule {
    Exact,
    CaseInsensitive,
    Normalized,
}

impl From<FlatTextResponseMatchRule> for TextResponseMatchRule {
    fn from(value: FlatTextResponseMatchRule) -> Self {
        match value {
            FlatTextResponseMatchRule::Exact => Self::Exact,
            FlatTextResponseMatchRule::CaseInsensitive => Self::CaseInsensitive,
            FlatTextResponseMatchRule::Normalized => Self::Normalized,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum FlatNumericResponseTolerance {
    Exact,
    Absolute { epsilon: f64 },
    Relative { fraction: f64 },
    SignificantFigures { digits: u8 },
}

impl From<&FlatNumericResponseTolerance> for NumericResponseTolerance {
    fn from(value: &FlatNumericResponseTolerance) -> Self {
        match value {
            FlatNumericResponseTolerance::Exact => Self::Exact,
            FlatNumericResponseTolerance::Absolute { epsilon } => {
                Self::Absolute { epsilon: *epsilon }
            }
            FlatNumericResponseTolerance::Relative { fraction } => Self::Relative {
                fraction: *fraction,
            },
            FlatNumericResponseTolerance::SignificantFigures { digits } => {
                Self::SignificantFigures { digits: *digits }
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatItem {
    id: String,
    text: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatBlank {
    id: String,
    label: String,
    answers: Vec<String>,
    match_mode: FlatTextResponseMatchRule,
    max_length: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatMatch {
    prompt: String,
    choice: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatHotspotSurface {
    asset: String,
    checksum: String,
    description: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatHotspotRegion {
    id: String,
    label: String,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl FlatQuestionV2 {
    pub(super) fn with_hotspot_surface_asset(
        &self,
        asset: AssetId,
    ) -> Result<Self, FlatQuestionError> {
        let mut published = self.clone();
        let FlatResponseV2::Hotspot { surface, .. } = &mut published.response else {
            return invalid("flat-question source has no hotspot surface to retarget");
        };
        surface.asset = asset.to_string();
        published.validate()?;
        Ok(published)
    }

    pub(super) fn imported_single_choice(
        title: String,
        prompt: String,
        choices: Vec<FlatChoice>,
        correct_choice: String,
        points: f64,
    ) -> Self {
        Self {
            format: FORMAT_NAME.to_string(),
            version: FORMAT_VERSION_V2,
            title,
            prompt,
            response: FlatResponseV2::SingleChoice {
                choices,
                correct_choice,
            },
            feedback: FlatOutcomeFeedback::default(),
            points,
            question_attempt_limit: FlatQuestionAttemptLimit { max_attempts: None },
            question_attempt_time_limit: FlatQuestionAttemptTimeLimit::Unlimited,
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: FlatLicense::AllRightsReserved,
            language: "en-US".to_string(),
        }
    }

    pub(super) fn validate(&self) -> Result<(), FlatQuestionError> {
        if self.format != FORMAT_NAME {
            return Err(FlatQuestionError::UnsupportedFormat);
        }
        if self.version != FORMAT_VERSION_V2 {
            return Err(FlatQuestionError::UnsupportedVersion(self.version));
        }
        question_model::validate_question_title(&self.title)
            .map_err(FlatQuestionError::InvalidTitle)?;
        validate_markdown("prompt", &self.prompt, MAX_PROMPT_CHARS)?;
        validate_optional_feedback(self.feedback.correct.as_deref())?;
        validate_optional_feedback(self.feedback.incorrect.as_deref())?;
        if !self.points.is_finite() || self.points < 0.0 {
            return invalid("points must be finite and nonnegative");
        }
        if self.question_attempt_limit.max_attempts == Some(0) {
            return invalid("maxAttempts must be positive or null");
        }
        validate_metadata_text("language", &self.language)?;
        for tag in &self.tags {
            validate_bounded_text("tag", tag, MAX_TAG_CHARS)?;
        }
        for term in &self.taxonomy {
            validate_metadata_text("taxonomy scheme", &term.scheme)?;
            validate_metadata_text("taxonomy code", &term.code)?;
            validate_metadata_text("taxonomy label", &term.label)?;
        }
        if let FlatLicense::Other { spdx } = &self.license {
            validate_metadata_text("SPDX license", spdx)?;
        }
        self.validate_response()
    }

    fn validate_response(&self) -> Result<(), FlatQuestionError> {
        match &self.response {
            FlatResponseV2::SingleChoice {
                choices,
                correct_choice,
            } => validate_choice_question(choices, std::slice::from_ref(correct_choice), true),
            FlatResponseV2::MultipleAnswer {
                choices,
                correct_choices,
            } => validate_choice_question(choices, correct_choices, false),
            FlatResponseV2::FillIn {
                answers,
                max_length,
                ..
            } => validate_answers(answers, *max_length),
            FlatResponseV2::MultiFillIn { blanks } => validate_blanks(blanks),
            FlatResponseV2::Numeric {
                answer,
                tolerance,
                unit,
            } => validate_numeric(*answer, tolerance, unit.as_deref()),
            FlatResponseV2::Matching {
                prompts,
                choices,
                matches,
            } => validate_matching(prompts, choices, matches),
            FlatResponseV2::Ordering {
                items,
                correct_order,
            } => validate_ordering(items, correct_order),
            FlatResponseV2::Hotspot {
                surface,
                regions,
                correct_regions,
            } => validate_hotspot(surface, regions, correct_regions),
        }
    }

    pub(super) fn compile(
        &self,
        workspace: WorkspaceId,
    ) -> Result<CompiledFlatQuestion, FlatQuestionError> {
        self.validate()?;
        let question_type = question_type_for(&self.response);
        let (response, answer_key, choice_feedback, prompt_suffix) =
            compile_response(&self.response)?;
        let mut prompt = markdown_blocks(&self.prompt);
        prompt.extend(prompt_suffix);
        let draft = DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native,
            question_format: QuestionFormat::PleFlatQuestionV2,
            prompt,
            response,
            question_type,
            question_attempt_limit: self.question_attempt_limit.into(),
            question_attempt_time_limit: self.question_attempt_time_limit.into(),
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing {
                points: self.points,
            },
            metadata: QuestionMetadata {
                title: self.title.clone(),
                tags: self.tags.iter().map(Tag::new).collect(),
                taxonomy: self.taxonomy.iter().map(TaxonomyTerm::from).collect(),
                license: License::from(&self.license),
                language: self.language.clone(),
            },
        };
        let private = FlatQuestionPrivate::new_with_key(
            &draft,
            answer_key,
            choice_feedback,
            self.feedback.correct.clone(),
            self.feedback.incorrect.clone(),
        )?;
        Ok(CompiledFlatQuestion { draft, private })
    }
}

fn question_type_for(response: &FlatResponseV2) -> QuestionType {
    match response {
        FlatResponseV2::SingleChoice { .. } => QuestionType::MultipleChoice,
        FlatResponseV2::MultipleAnswer { .. } => QuestionType::MultipleAnswer,
        FlatResponseV2::FillIn { .. } => QuestionType::FillInBlank,
        FlatResponseV2::MultiFillIn { .. } => QuestionType::MultipleFillInBlank,
        FlatResponseV2::Numeric { .. } => QuestionType::Numeric,
        FlatResponseV2::Matching { .. } => QuestionType::Matching,
        FlatResponseV2::Ordering { .. } => QuestionType::Ordering,
        FlatResponseV2::Hotspot { .. } => QuestionType::Hotspot,
    }
}

type CompiledResponse = (
    QuestionResponseFormat,
    AnswerKey,
    Vec<(ResponseItemReference, String)>,
    Vec<ContentBlock>,
);

fn compile_response(response: &FlatResponseV2) -> Result<CompiledResponse, FlatQuestionError> {
    let compiled = match response {
        FlatResponseV2::SingleChoice {
            choices,
            correct_choice,
        } => compile_choices(
            choices,
            ResponseSelectionRule::ExactlyOne,
            std::slice::from_ref(correct_choice),
        ),
        FlatResponseV2::MultipleAnswer {
            choices,
            correct_choices,
        } => compile_choices(choices, ResponseSelectionRule::AtLeastOne, correct_choices),
        FlatResponseV2::FillIn {
            answers,
            match_mode,
            max_length,
        } => (
            QuestionResponseFormat::ShortText {
                match_mode: (*match_mode).into(),
                max_length: *max_length,
            },
            AnswerKey::ShortText {
                accepted: answers.clone(),
            },
            Vec::new(),
            Vec::new(),
        ),
        FlatResponseV2::MultiFillIn { blanks } => (
            QuestionResponseFormat::MultiBlank {
                blanks: blanks
                    .iter()
                    .map(|blank| TextEntrySlot {
                        id: ResponseItemReference::new(&blank.id),
                        label: markdown_blocks(&blank.label),
                        match_mode: blank.match_mode.into(),
                        max_length: blank.max_length,
                    })
                    .collect(),
            },
            AnswerKey::MultiBlank {
                accepted: blanks
                    .iter()
                    .map(|blank| (ResponseItemReference::new(&blank.id), blank.answers.clone()))
                    .collect(),
            },
            Vec::new(),
            Vec::new(),
        ),
        FlatResponseV2::Numeric {
            answer,
            tolerance,
            unit,
        } => (
            QuestionResponseFormat::Numeric {
                tolerance: tolerance.into(),
                unit: unit.clone(),
            },
            AnswerKey::Numeric { expected: *answer },
            Vec::new(),
            Vec::new(),
        ),
        FlatResponseV2::Matching {
            prompts,
            choices,
            matches,
        } => (
            QuestionResponseFormat::Matching {
                prompts: compile_items(prompts),
                choices: compile_items(choices),
            },
            AnswerKey::Matching {
                correct: matches
                    .iter()
                    .map(|pair| (ResponseItemReference::new(&pair.prompt), ResponseItemReference::new(&pair.choice)))
                    .collect(),
            },
            Vec::new(),
            Vec::new(),
        ),
        FlatResponseV2::Ordering {
            items,
            correct_order,
        } => (
            QuestionResponseFormat::Ordering {
                items: compile_items(items),
            },
            AnswerKey::Ordering {
                correct: correct_order.iter().map(ResponseItemReference::new).collect(),
            },
            Vec::new(),
            Vec::new(),
        ),
        FlatResponseV2::Hotspot {
            surface,
            regions,
            correct_regions,
        } => {
            let asset = AssetRef {
                asset: AssetId::from_uuid(Uuid::parse_str(&surface.asset).map_err(|_| {
                    FlatQuestionError::InvalidDocument("hotspot asset must be a UUID".to_string())
                })?),
                checksum: surface.checksum.clone(),
            };
            (
                QuestionResponseFormat::Hotspot {
                    surface: asset.clone(),
                    description: surface.description.clone(),
                    regions: regions.iter().map(compile_region).collect(),
                    // Correct-region cardinality is private grading material.
                    // The public format requires a nonempty selection without
                    // disclosing how many regions the answer key contains.
                    selection: ResponseSelectionRule::AtLeastOne,
                },
                AnswerKey::Hotspot {
                    correct: correct_regions.iter().map(ResponseItemReference::new).collect(),
                },
                Vec::new(),
                vec![ContentBlock::Image {
                    asset,
                    description: surface.description.clone(),
                }],
            )
        }
    };
    Ok(compiled)
}

fn compile_choices(
    choices: &[FlatChoice],
    selection: ResponseSelectionRule,
    correct: &[String],
) -> CompiledResponse {
    (
        QuestionResponseFormat::MultipleChoice {
            choices: choices
                .iter()
                .map(|choice| ChoiceOption {
                    id: ResponseItemReference::new(&choice.id),
                    body: markdown_blocks(&choice.text),
                })
                .collect(),
            selection,
        },
        AnswerKey::MultipleChoice {
            correct: correct.iter().map(ResponseItemReference::new).collect(),
        },
        choices
            .iter()
            .filter_map(|choice| {
                choice
                    .feedback
                    .as_ref()
                    .map(|feedback| (ResponseItemReference::new(&choice.id), feedback.clone()))
            })
            .collect(),
        Vec::new(),
    )
}

fn compile_items(items: &[FlatItem]) -> Vec<ChoiceOption> {
    items
        .iter()
        .map(|item| ChoiceOption {
            id: ResponseItemReference::new(&item.id),
            body: markdown_blocks(&item.text),
        })
        .collect()
}

fn compile_region(region: &FlatHotspotRegion) -> HotspotRegion {
    HotspotRegion {
        id: ResponseItemReference::new(&region.id),
        label: markdown_blocks(&region.label),
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
    }
}

fn validate_choice_question(
    choices: &[FlatChoice],
    correct: &[String],
    exactly_one: bool,
) -> Result<(), FlatQuestionError> {
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

fn validate_answers(answers: &[String], max_length: u32) -> Result<(), FlatQuestionError> {
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

fn validate_blanks(blanks: &[FlatBlank]) -> Result<(), FlatQuestionError> {
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

fn validate_numeric(
    answer: f64,
    tolerance: &FlatNumericResponseTolerance,
    unit: Option<&str>,
) -> Result<(), FlatQuestionError> {
    if !answer.is_finite() {
        return invalid("numeric answer must be finite");
    }
    match tolerance {
        FlatNumericResponseTolerance::Exact => {}
        FlatNumericResponseTolerance::Absolute { epsilon } => {
            validate_nonnegative_finite("absolute epsilon", *epsilon)?;
        }
        FlatNumericResponseTolerance::Relative { fraction } => {
            validate_nonnegative_finite("relative fraction", *fraction)?;
        }
        FlatNumericResponseTolerance::SignificantFigures { digits } if *digits == 0 => {
            return invalid("significant figures must be at least one");
        }
        FlatNumericResponseTolerance::SignificantFigures { .. } => {}
    }
    if let Some(unit) = unit {
        validate_bounded_text("numeric unit", unit, MAX_METADATA_TEXT_CHARS)?;
    }
    Ok(())
}

fn validate_nonnegative_finite(name: &str, value: f64) -> Result<(), FlatQuestionError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        invalid(&format!("{name} must be finite and nonnegative"))
    }
}

fn validate_items(
    name: &str,
    items: &[FlatItem],
    minimum: usize,
) -> Result<HashSet<String>, FlatQuestionError> {
    if items.len() < minimum || items.len() > MAX_CHOICES {
        return invalid(&format!("{name} count is outside the supported range"));
    }
    let mut ids = HashSet::new();
    for item in items {
        validate_choice_id(&item.id)?;
        if !ids.insert(item.id.clone()) {
            return invalid(&format!("{name} identifiers must be unique"));
        }
        validate_markdown(name, &item.text, MAX_CHOICE_TEXT_CHARS)?;
    }
    Ok(ids)
}

fn validate_matching(
    prompts: &[FlatItem],
    choices: &[FlatItem],
    matches: &[FlatMatch],
) -> Result<(), FlatQuestionError> {
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

fn validate_ordering(items: &[FlatItem], order: &[String]) -> Result<(), FlatQuestionError> {
    let ids = validate_items("ordering item", items, 3)?;
    let order_ids: HashSet<_> = order.iter().cloned().collect();
    if order.len() != items.len() || order_ids.len() != order.len() || order_ids != ids {
        return invalid("correctOrder must contain every ordering item exactly once");
    }
    Ok(())
}

fn validate_hotspot(
    surface: &FlatHotspotSurface,
    regions: &[FlatHotspotRegion],
    correct: &[String],
) -> Result<(), FlatQuestionError> {
    Uuid::parse_str(&surface.asset).map_err(|_| {
        FlatQuestionError::InvalidDocument("hotspot asset must be a UUID".to_string())
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
        return invalid("hotspot questions require candidate and correct regions");
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
            return invalid("hotspot candidate regions must not overlap");
        }
    }
    let correct_ids: HashSet<_> = correct.iter().map(String::as_str).collect();
    if correct_ids.len() != correct.len() || !correct_ids.is_subset(&ids) {
        return invalid("correctRegions must be unique available regions");
    }
    Ok(())
}

fn regions_overlap(left: &FlatHotspotRegion, right: &FlatHotspotRegion) -> bool {
    let left_right = u32::from(left.x) + u32::from(left.width);
    let left_bottom = u32::from(left.y) + u32::from(left.height);
    let right_right = u32::from(right.x) + u32::from(right.width);
    let right_bottom = u32::from(right.y) + u32::from(right.height);
    u32::from(left.x) <= right_right
        && u32::from(right.x) <= left_right
        && u32::from(left.y) <= right_bottom
        && u32::from(right.y) <= left_bottom
}
