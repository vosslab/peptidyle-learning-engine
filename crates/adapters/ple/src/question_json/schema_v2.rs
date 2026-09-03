//! Closed schema-version-2 source shapes for all supported PLE Question JSON Question Types.

use std::collections::HashSet;

use grading::AnswerKey;
use question_model::answer::{
    NumericResponseTolerance, ResponseSelectionRule, TextResponseMatchRule,
};
use question_model::generation::QuestionVariationRule;
use question_model::question_citation::QuestionCitation;
use question_model::question_license::QuestionLicense;
use question_model::question_tag::Tag;
use question_model::response::{
    HotspotRegion, MatchingChoice, MatchingPrompt, OrderingItem, QuestionChoice,
    QuestionResponseFormat, QuestionType, ResponseItemReference, TextEntrySlot,
};
use question_model::{
    DraftQuestionContent, QuestionAssetId, QuestionBackend, QuestionFormat, QuestionGradingRule,
    QuestionHint, QuestionMetadata, WorkspaceId,
};
use question_model::{QuestionAssetReference, QuestionContentBlock};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    CompiledPleQuestionJson, MAX_CHOICE_TEXT_CHARS, MAX_CHOICES, MAX_FEEDBACK_CHARS,
    MAX_METADATA_TEXT_CHARS, MAX_PROMPT_CHARS, MAX_TAG_CHARS, PLE_QUESTION_JSON_FORMAT_NAME,
    PleQuestionJsonAttemptLimit, PleQuestionJsonAttemptTimeLimit, PleQuestionJsonChoice,
    PleQuestionJsonError, PleQuestionJsonOutcomeFeedback, PleQuestionJsonPrivateGrading, invalid,
    markdown_blocks, validate_bounded_text, validate_choice_id, validate_markdown,
    validate_metadata_text, validate_optional_feedback, validate_optional_hint,
};

const PLE_QUESTION_JSON_SCHEMA_VERSION: u32 = 2;
const MAX_BLANKS: usize = 50;
const MAX_TEXT_RESPONSE_CHARS: u32 = 16_384;

/// Version 2 keeps common metadata outside a closed, type-specific response object.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PleQuestionJsonDocumentBody {
    format: String,
    version: u32,
    title: String,
    question_description: String,
    prompt: String,
    response: PleQuestionJsonResponse,
    #[serde(default)]
    feedback: PleQuestionJsonOutcomeFeedback,
    #[serde(default)]
    question_hint: Option<String>,
    points: f64,
    question_attempt_limit: PleQuestionJsonAttemptLimit,
    question_attempt_time_limit: PleQuestionJsonAttemptTimeLimit,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    question_license: Option<QuestionLicense>,
    #[serde(default)]
    question_citation: Option<QuestionCitation>,
    language: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum PleQuestionJsonResponse {
    SingleChoice {
        choices: Vec<PleQuestionJsonChoice>,
        correct_choice: String,
    },
    MultipleAnswer {
        choices: Vec<PleQuestionJsonChoice>,
        correct_choices: Vec<String>,
    },
    FillIn {
        answers: Vec<String>,
        match_mode: PleQuestionJsonTextResponseMatchRule,
        max_length: u32,
    },
    MultiFillIn {
        blanks: Vec<PleQuestionJsonBlank>,
    },
    Numeric {
        answer: f64,
        tolerance: PleQuestionJsonNumericResponseTolerance,
        #[serde(default)]
        unit: Option<String>,
    },
    Matching {
        prompts: Vec<PleQuestionJsonMatchingPrompt>,
        choices: Vec<PleQuestionJsonMatchingChoice>,
        matches: Vec<PleQuestionJsonMatch>,
    },
    Ordering {
        items: Vec<PleQuestionJsonOrderingItem>,
        correct_order: Vec<String>,
    },
    Hotspot {
        surface: PleQuestionJsonHotspotSurface,
        regions: Vec<PleQuestionJsonHotspotRegion>,
        correct_regions: Vec<String>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum PleQuestionJsonTextResponseMatchRule {
    Exact,
    CaseInsensitive,
    Normalized,
}

impl From<PleQuestionJsonTextResponseMatchRule> for TextResponseMatchRule {
    fn from(value: PleQuestionJsonTextResponseMatchRule) -> Self {
        match value {
            PleQuestionJsonTextResponseMatchRule::Exact => Self::Exact,
            PleQuestionJsonTextResponseMatchRule::CaseInsensitive => Self::CaseInsensitive,
            PleQuestionJsonTextResponseMatchRule::Normalized => Self::Normalized,
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
enum PleQuestionJsonNumericResponseTolerance {
    Exact,
    Absolute { epsilon: f64 },
    Relative { fraction: f64 },
    SignificantFigures { digits: u8 },
}

impl From<&PleQuestionJsonNumericResponseTolerance> for NumericResponseTolerance {
    fn from(value: &PleQuestionJsonNumericResponseTolerance) -> Self {
        match value {
            PleQuestionJsonNumericResponseTolerance::Exact => Self::Exact,
            PleQuestionJsonNumericResponseTolerance::Absolute { epsilon } => {
                Self::Absolute { epsilon: *epsilon }
            }
            PleQuestionJsonNumericResponseTolerance::Relative { fraction } => Self::Relative {
                fraction: *fraction,
            },
            PleQuestionJsonNumericResponseTolerance::SignificantFigures { digits } => {
                Self::SignificantFigures { digits: *digits }
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonMatchingPrompt {
    id: String,
    text: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonMatchingChoice {
    id: String,
    text: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonOrderingItem {
    id: String,
    text: String,
}

trait PleQuestionJsonResponseMember {
    fn id(&self) -> &str;
    fn text(&self) -> &str;
}

macro_rules! response_member {
    ($type:ident) => {
        impl PleQuestionJsonResponseMember for $type {
            fn id(&self) -> &str {
                &self.id
            }

            fn text(&self) -> &str {
                &self.text
            }
        }
    };
}

response_member!(PleQuestionJsonMatchingPrompt);
response_member!(PleQuestionJsonMatchingChoice);
response_member!(PleQuestionJsonOrderingItem);

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonBlank {
    id: String,
    label: String,
    answers: Vec<String>,
    match_mode: PleQuestionJsonTextResponseMatchRule,
    max_length: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonMatch {
    prompt: String,
    choice: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonHotspotSurface {
    asset: String,
    checksum: String,
    description: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonHotspotRegion {
    id: String,
    label: String,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl PleQuestionJsonDocumentBody {
    pub(super) fn with_hotspot_surface_asset(
        &self,
        asset: QuestionAssetId,
    ) -> Result<Self, PleQuestionJsonError> {
        let mut published = self.clone();
        let PleQuestionJsonResponse::Hotspot { surface, .. } = &mut published.response else {
            return invalid("PLE Question JSON source has no hotspot surface to retarget");
        };
        surface.asset = asset.to_string();
        published.validate()?;
        Ok(published)
    }

    pub(super) fn imported_single_choice(
        title: String,
        question_description: String,
        prompt: String,
        choices: Vec<PleQuestionJsonChoice>,
        correct_choice: String,
        points: f64,
    ) -> Self {
        Self {
            format: PLE_QUESTION_JSON_FORMAT_NAME.to_string(),
            version: PLE_QUESTION_JSON_SCHEMA_VERSION,
            title,
            question_description,
            prompt,
            response: PleQuestionJsonResponse::SingleChoice {
                choices,
                correct_choice,
            },
            feedback: PleQuestionJsonOutcomeFeedback::default(),
            question_hint: None,
            points,
            question_attempt_limit: PleQuestionJsonAttemptLimit { max_attempts: None },
            question_attempt_time_limit: PleQuestionJsonAttemptTimeLimit::Unlimited,
            tags: Vec::new(),
            question_license: None,
            question_citation: None,
            language: "en-US".to_string(),
        }
    }

    pub(super) fn validate(&self) -> Result<(), PleQuestionJsonError> {
        if self.format != PLE_QUESTION_JSON_FORMAT_NAME {
            return Err(PleQuestionJsonError::UnsupportedFormat);
        }
        if self.version != PLE_QUESTION_JSON_SCHEMA_VERSION {
            return Err(PleQuestionJsonError::UnsupportedVersion(self.version));
        }
        question_model::validate_question_title(&self.title)
            .map_err(PleQuestionJsonError::InvalidTitle)?;
        if let Err(error) =
            question_model::validate_question_description(&self.question_description)
        {
            return invalid(&error.to_string());
        }
        validate_markdown("prompt", &self.prompt, MAX_PROMPT_CHARS)?;
        validate_optional_feedback(self.feedback.correct.as_deref())?;
        validate_optional_feedback(self.feedback.incorrect.as_deref())?;
        validate_optional_hint(self.question_hint.as_deref())?;
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
        self.validate_response()
    }

    fn validate_response(&self) -> Result<(), PleQuestionJsonError> {
        match &self.response {
            PleQuestionJsonResponse::SingleChoice {
                choices,
                correct_choice,
            } => validate_choice_question(choices, std::slice::from_ref(correct_choice), true),
            PleQuestionJsonResponse::MultipleAnswer {
                choices,
                correct_choices,
            } => validate_choice_question(choices, correct_choices, false),
            PleQuestionJsonResponse::FillIn {
                answers,
                max_length,
                ..
            } => validate_answers(answers, *max_length),
            PleQuestionJsonResponse::MultiFillIn { blanks } => validate_blanks(blanks),
            PleQuestionJsonResponse::Numeric {
                answer,
                tolerance,
                unit,
            } => validate_numeric(*answer, tolerance, unit.as_deref()),
            PleQuestionJsonResponse::Matching {
                prompts,
                choices,
                matches,
            } => validate_matching(prompts, choices, matches),
            PleQuestionJsonResponse::Ordering {
                items,
                correct_order,
            } => validate_ordering(items, correct_order),
            PleQuestionJsonResponse::Hotspot {
                surface,
                regions,
                correct_regions,
            } => validate_hotspot(surface, regions, correct_regions),
        }
    }

    pub(super) fn compile(
        &self,
        workspace: WorkspaceId,
    ) -> Result<CompiledPleQuestionJson, PleQuestionJsonError> {
        self.validate()?;
        let question_type = question_type_for(&self.response);
        let (response, answer_key, choice_feedback, prompt_suffix) =
            compile_response(&self.response)?;
        let mut prompt = markdown_blocks(&self.prompt);
        prompt.extend(prompt_suffix);
        let draft = DraftQuestionContent {
            workspace,
            question_backend: QuestionBackend::Ple,
            webwork_pg_path: None,
            qti_package_item_identifier: None,
            workspace_import_id: None,
            draft_imathas_question_backend_binding: None,
            question_format: QuestionFormat::PleQuestionJson,
            prompt,
            response,
            question_type,
            question_attempt_limit: self.question_attempt_limit.into(),
            question_attempt_time_limit: self.question_attempt_time_limit.into(),
            question_variation_rule: QuestionVariationRule::Static,
            grading: QuestionGradingRule::AllOrNothing {
                points: self.points,
            },
            metadata: QuestionMetadata {
                title: self.title.clone(),
                question_description: self.question_description.clone(),
                tags: self.tags.iter().map(Tag::new).collect(),
                question_license: self.question_license.clone(),
                question_citation: self.question_citation.clone(),
                language: self.language.clone(),
            },
        };
        let private = PleQuestionJsonPrivateGrading::new_with_key(
            &draft,
            answer_key,
            choice_feedback,
            self.feedback.correct.clone(),
            self.feedback.incorrect.clone(),
        )?;
        let question_hint = self
            .question_hint
            .as_deref()
            .map(markdown_blocks)
            .and_then(QuestionHint::new);
        Ok(CompiledPleQuestionJson {
            draft,
            private,
            question_hint,
        })
    }
}

fn question_type_for(response: &PleQuestionJsonResponse) -> QuestionType {
    match response {
        PleQuestionJsonResponse::SingleChoice { .. } => QuestionType::MultipleChoice,
        PleQuestionJsonResponse::MultipleAnswer { .. } => QuestionType::MultipleAnswer,
        PleQuestionJsonResponse::FillIn { .. } => QuestionType::FillInBlank,
        PleQuestionJsonResponse::MultiFillIn { .. } => QuestionType::MultipleFillInBlank,
        PleQuestionJsonResponse::Numeric { .. } => QuestionType::Numeric,
        PleQuestionJsonResponse::Matching { .. } => QuestionType::Matching,
        PleQuestionJsonResponse::Ordering { .. } => QuestionType::Ordering,
        PleQuestionJsonResponse::Hotspot { .. } => QuestionType::Hotspot,
    }
}

type CompiledResponse = (
    QuestionResponseFormat,
    AnswerKey,
    Vec<(ResponseItemReference, String)>,
    Vec<QuestionContentBlock>,
);

fn compile_response(
    response: &PleQuestionJsonResponse,
) -> Result<CompiledResponse, PleQuestionJsonError> {
    let compiled = match response {
        PleQuestionJsonResponse::SingleChoice {
            choices,
            correct_choice,
        } => compile_choices(
            choices,
            ResponseSelectionRule::ExactlyOne,
            std::slice::from_ref(correct_choice),
        ),
        PleQuestionJsonResponse::MultipleAnswer {
            choices,
            correct_choices,
        } => compile_choices(choices, ResponseSelectionRule::AtLeastOne, correct_choices),
        PleQuestionJsonResponse::FillIn {
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
        PleQuestionJsonResponse::MultiFillIn { blanks } => (
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
        PleQuestionJsonResponse::Numeric {
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
        PleQuestionJsonResponse::Matching {
            prompts,
            choices,
            matches,
        } => (
            QuestionResponseFormat::Matching {
                prompts: compile_matching_prompts(prompts),
                choices: compile_matching_choices(choices),
            },
            AnswerKey::Matching {
                correct: matches
                    .iter()
                    .map(|pair| {
                        (
                            ResponseItemReference::new(&pair.prompt),
                            ResponseItemReference::new(&pair.choice),
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
            AnswerKey::Ordering {
                correct: correct_order
                    .iter()
                    .map(ResponseItemReference::new)
                    .collect(),
            },
            Vec::new(),
            Vec::new(),
        ),
        PleQuestionJsonResponse::Hotspot {
            surface,
            regions,
            correct_regions,
        } => {
            let asset = QuestionAssetReference {
                asset: QuestionAssetId::from_uuid(Uuid::parse_str(&surface.asset).map_err(
                    |_| {
                        PleQuestionJsonError::InvalidDocument(
                            "hotspot asset must be a UUID".to_string(),
                        )
                    },
                )?),
                checksum: surface.checksum.clone(),
            };
            (
                QuestionResponseFormat::Hotspot {
                    surface: asset.clone(),
                    description: surface.description.clone(),
                    regions: regions.iter().map(compile_region).collect(),
                    // Correct-region cardinality is private Answer Key data.
                    // The public format requires a nonempty selection without
                    // disclosing how many regions the answer key contains.
                    selection: ResponseSelectionRule::AtLeastOne,
                },
                AnswerKey::Hotspot {
                    correct: correct_regions
                        .iter()
                        .map(ResponseItemReference::new)
                        .collect(),
                },
                Vec::new(),
                vec![QuestionContentBlock::Image {
                    asset,
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
) -> CompiledResponse {
    (
        QuestionResponseFormat::MultipleChoice {
            choices: choices
                .iter()
                .map(|choice| QuestionChoice {
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

fn compile_matching_prompts(items: &[PleQuestionJsonMatchingPrompt]) -> Vec<MatchingPrompt> {
    items
        .iter()
        .map(|item| MatchingPrompt {
            id: ResponseItemReference::new(&item.id),
            body: markdown_blocks(&item.text),
        })
        .collect()
}

fn compile_matching_choices(items: &[PleQuestionJsonMatchingChoice]) -> Vec<MatchingChoice> {
    items
        .iter()
        .map(|item| MatchingChoice {
            id: ResponseItemReference::new(&item.id),
            body: markdown_blocks(&item.text),
        })
        .collect()
}

fn compile_ordering_items(items: &[PleQuestionJsonOrderingItem]) -> Vec<OrderingItem> {
    items
        .iter()
        .map(|item| OrderingItem {
            id: ResponseItemReference::new(&item.id),
            body: markdown_blocks(&item.text),
        })
        .collect()
}

fn compile_region(region: &PleQuestionJsonHotspotRegion) -> HotspotRegion {
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

fn validate_answers(answers: &[String], max_length: u32) -> Result<(), PleQuestionJsonError> {
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

fn validate_blanks(blanks: &[PleQuestionJsonBlank]) -> Result<(), PleQuestionJsonError> {
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

fn validate_matching(
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

fn validate_ordering(
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

fn validate_hotspot(
    surface: &PleQuestionJsonHotspotSurface,
    regions: &[PleQuestionJsonHotspotRegion],
    correct: &[String],
) -> Result<(), PleQuestionJsonError> {
    Uuid::parse_str(&surface.asset).map_err(|_| {
        PleQuestionJsonError::InvalidDocument("hotspot asset must be a UUID".to_string())
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
