//! Server-only integrity contract for PLE's static flat-question family.
//!
//! The authoring parser deliberately lives in `adapter_native`; this module
//! owns every rule whose failure could change correctness or disclose answers.

use std::collections::{BTreeSet, HashSet};
use std::fmt::Write as _;

use question_model::answer::SelectionCardinality;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::{ChoiceId, ResponseDefinition, StudentResponse};
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::{
    AttemptResult, DraftQuestionDefinition, DraftQuestionSource, FeedbackContent,
    GradingDefinition, QuestionDefinition, QuestionMetadata, QuestionSource, QuestionTitleError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AnswerKey, GradeOutcome, GradingError, grade};

/// Stable source-family identifiers persisted in the public question model.
pub const FLAT_SINGLE_CHOICE_V2_FAMILY: &str = "flat_single_choice_v2";
pub const FLAT_MULTIPLE_ANSWER_FAMILY: &str = "flat_multiple_answer_v2";
pub const FLAT_FILL_IN_FAMILY: &str = "flat_fill_in_v2";
pub const FLAT_MULTI_FILL_IN_FAMILY: &str = "flat_multi_fill_in_v2";
pub const FLAT_NUMERIC_FAMILY: &str = "flat_numeric_v2";
pub const FLAT_MATCHING_FAMILY: &str = "flat_matching_v2";
pub const FLAT_ORDERING_FAMILY: &str = "flat_ordering_v2";
pub const FLAT_HOTSPOT_FAMILY: &str = "flat_hotspot_v2";
/// Upper bound shared by persisted private material and source adapters.
pub const MAX_FLAT_QUESTION_BYTES: usize = 256 * 1024;
const PRIVATE_SCHEMA_VERSION: u32 = 2;
const MAX_CHOICES: usize = 100;
const MAX_CHOICE_ID_BYTES: usize = 64;
const MAX_FEEDBACK_CHARS: usize = 16_384;

/// Stable errors for flat question parsing, validation, persistence, and grading.
///
/// The native adapter re-exports this type to retain its established public API.
#[derive(Debug, Clone, PartialEq)]
pub enum FlatQuestionError {
    TooLarge,
    MalformedJson(String),
    UnsupportedFormat,
    UnsupportedVersion(u32),
    InvalidDocument(String),
    InvalidTitle(QuestionTitleError),
    PublicBindingMismatch,
    Grading(GradingError),
    Encoding(String),
}

impl std::fmt::Display for FlatQuestionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(
                formatter,
                "flat-question JSON exceeds {MAX_FLAT_QUESTION_BYTES} bytes"
            ),
            Self::MalformedJson(message) => {
                write!(formatter, "invalid flat-question JSON: {message}")
            }
            Self::UnsupportedFormat => formatter.write_str("unsupported flat-question format"),
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "unsupported flat-question schema version {version}"
            ),
            Self::InvalidDocument(message) => {
                write!(formatter, "invalid flat-question document: {message}")
            }
            Self::InvalidTitle(error) => error.fmt(formatter),
            Self::PublicBindingMismatch => formatter
                .write_str("private flat-question material does not match the public definition"),
            Self::Grading(error) => error.fmt(formatter),
            Self::Encoding(message) => {
                write!(formatter, "flat-question encoding failed: {message}")
            }
        }
    }
}

impl std::error::Error for FlatQuestionError {}

/// Server-only key and feedback bound to one exact public question payload.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlatQuestionPrivate {
    schema_version: u32,
    public_sha256: String,
    answer_key: AnswerKey,
    choice_feedback: Vec<FlatChoiceFeedback>,
    outcome_feedback: FlatOutcomeFeedback,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatChoiceFeedback {
    choice: String,
    markdown: String,
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatOutcomeFeedback {
    #[serde(default)]
    correct: Option<String>,
    #[serde(default)]
    incorrect: Option<String>,
}

/// Result and private teaching content from one trusted evaluation.
pub struct FlatQuestionEvaluation {
    pub outcome: GradeOutcome,
    pub feedback: FeedbackContent,
}

impl FlatQuestionPrivate {
    /// Builds private material for one of the closed v2 flat families.
    pub fn new_with_key(
        draft: &DraftQuestionDefinition,
        answer_key: AnswerKey,
        choice_feedback: Vec<(ChoiceId, String)>,
        correct_feedback: Option<String>,
        incorrect_feedback: Option<String>,
    ) -> Result<Self, FlatQuestionError> {
        validate_for_draft(draft)?;
        let available = selectable_ids(&draft.response);
        let mut feedback_ids = HashSet::new();
        let mut feedback = Vec::with_capacity(choice_feedback.len());
        for (choice, markdown) in choice_feedback {
            if !available.contains(&choice) || !feedback_ids.insert(choice.clone()) {
                return invalid("choice feedback targets must be unique available choices");
            }
            validate_feedback(&markdown)?;
            feedback.push(FlatChoiceFeedback {
                choice: choice.as_str().to_string(),
                markdown,
            });
        }
        validate_optional_feedback(correct_feedback.as_deref())?;
        validate_optional_feedback(incorrect_feedback.as_deref())?;
        validate_key_against_response(&draft.response, &answer_key)?;
        Ok(Self {
            schema_version: PRIVATE_SCHEMA_VERSION,
            public_sha256: public_binding_sha256_for_draft(draft)?,
            answer_key,
            choice_feedback: feedback,
            outcome_feedback: FlatOutcomeFeedback {
                correct: correct_feedback,
                incorrect: incorrect_feedback,
            },
        })
    }

    pub fn public_binding_sha256(&self) -> &str {
        &self.public_sha256
    }

    /// Validates this private material against its editable public draft.
    ///
    /// Publication uses this seam before durable identifiers exist. It proves
    /// that the key and feedback describe this exact public payload without
    /// fabricating a published [`QuestionDefinition`].
    pub fn validate_for_draft(
        &self,
        draft: &DraftQuestionDefinition,
    ) -> Result<(), FlatQuestionError> {
        validate_for_draft(draft)?;
        if public_binding_sha256_for_draft(draft)? != self.public_sha256 {
            return Err(FlatQuestionError::PublicBindingMismatch);
        }
        self.validate_private_shape()?;
        validate_key_against_response(&draft.response, &self.answer_key)?;
        self.validate_feedback_targets(&draft.response)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, FlatQuestionError> {
        if bytes.len() > MAX_FLAT_QUESTION_BYTES {
            return Err(FlatQuestionError::TooLarge);
        }
        let value: Self = serde_json::from_slice(bytes)
            .map_err(|error| FlatQuestionError::MalformedJson(error.to_string()))?;
        if value.canonical_bytes()? != bytes {
            return Err(FlatQuestionError::MalformedJson(
                "flat-question private material is not canonical".to_string(),
            ));
        }
        value.validate_private_shape()?;
        Ok(value)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, FlatQuestionError> {
        serde_json::to_vec(self).map_err(|error| FlatQuestionError::Encoding(error.to_string()))
    }

    pub fn evaluate(
        &self,
        question: &QuestionDefinition,
        response: &StudentResponse,
    ) -> Result<FlatQuestionEvaluation, FlatQuestionError> {
        validate_flat_question_question(question)?;
        if public_binding_sha256_for_question(question)? != self.public_sha256 {
            return Err(FlatQuestionError::PublicBindingMismatch);
        }
        self.validate_against_question(question)?;
        let outcome = grade(question, response, Some(&self.answer_key))
            .map_err(FlatQuestionError::Grading)?;
        let GradeOutcome::Graded(result) = outcome else {
            return Err(FlatQuestionError::Grading(GradingError::InvalidDefinition(
                "flat-question grading must produce a numeric result".to_string(),
            )));
        };
        Ok(FlatQuestionEvaluation {
            outcome: GradeOutcome::Graded(result),
            feedback: self.feedback_for(question, response, result)?,
        })
    }

    fn validate_private_shape(&self) -> Result<(), FlatQuestionError> {
        if self.schema_version != PRIVATE_SCHEMA_VERSION {
            return Err(FlatQuestionError::UnsupportedVersion(self.schema_version));
        }
        if !is_hex_sha256(&self.public_sha256) {
            return invalid("publicSha256 must be a 64-character lowercase hex digest");
        }
        validate_optional_feedback(self.outcome_feedback.correct.as_deref())?;
        validate_optional_feedback(self.outcome_feedback.incorrect.as_deref())?;
        let mut feedback_ids = HashSet::new();
        for feedback in &self.choice_feedback {
            validate_choice_id(&feedback.choice)?;
            if !feedback_ids.insert(feedback.choice.as_str()) {
                return invalid("choice feedback target IDs must be unique");
            }
            validate_feedback(&feedback.markdown)?;
        }
        Ok(())
    }

    fn validate_against_question(
        &self,
        question: &QuestionDefinition,
    ) -> Result<(), FlatQuestionError> {
        self.validate_private_shape()?;
        validate_key_against_response(&question.response, &self.answer_key)?;
        self.validate_feedback_targets(&question.response)
    }

    fn validate_feedback_targets(
        &self,
        response: &ResponseDefinition,
    ) -> Result<(), FlatQuestionError> {
        let available = selectable_ids(response);
        if self
            .choice_feedback
            .iter()
            .any(|feedback| !available.contains(&ChoiceId::new(&feedback.choice)))
        {
            return Err(FlatQuestionError::PublicBindingMismatch);
        }
        Ok(())
    }

    fn feedback_for(
        &self,
        question: &QuestionDefinition,
        response: &StudentResponse,
        result: AttemptResult,
    ) -> Result<FeedbackContent, FlatQuestionError> {
        let mut teaching = Vec::new();
        if let StudentResponse::MultipleChoice { selected } = response {
            for selected_choice in selected {
                if let Some(feedback) = self
                    .choice_feedback
                    .iter()
                    .find(|feedback| feedback.choice == selected_choice.as_str())
                {
                    teaching.extend(markdown_blocks(&feedback.markdown));
                }
            }
        }
        let outcome = if result.correct {
            self.outcome_feedback.correct.as_deref()
        } else {
            self.outcome_feedback.incorrect.as_deref()
        };
        if let Some(markdown) = outcome {
            teaching.extend(markdown_blocks(markdown));
        }
        Ok(FeedbackContent {
            hint: (!teaching.is_empty()).then_some(teaching),
            correct_response: Some(correct_response_blocks(
                &question.response,
                &self.answer_key,
            )?),
            rationale: None,
        })
    }
}

/// Validates the public contract on a draft before it can receive private key material.
pub fn validate_for_draft(draft: &DraftQuestionDefinition) -> Result<(), FlatQuestionError> {
    let DraftQuestionSource::Native { family } = &draft.source else {
        return Err(FlatQuestionError::PublicBindingMismatch);
    };
    if !is_flat_question_family(family) {
        return Err(FlatQuestionError::PublicBindingMismatch);
    }
    validate_flat_shape(
        family,
        &draft.randomization,
        &draft.response,
        &draft.grading,
    )
}

/// Validates any closed flat-question family after publication.
pub fn validate_flat_question_question(
    question: &QuestionDefinition,
) -> Result<(), FlatQuestionError> {
    let QuestionSource::Native { family } = &question.source else {
        return Err(FlatQuestionError::PublicBindingMismatch);
    };
    if !is_flat_question_family(family) {
        return Err(FlatQuestionError::PublicBindingMismatch);
    }
    validate_flat_shape(
        family,
        &question.randomization,
        &question.response,
        &question.grading,
    )
}

/// Whether a persisted native family belongs to the protected flat-question set.
pub fn is_flat_question_family(family: &str) -> bool {
    matches!(
        family,
        FLAT_SINGLE_CHOICE_V2_FAMILY
            | FLAT_MULTIPLE_ANSWER_FAMILY
            | FLAT_FILL_IN_FAMILY
            | FLAT_MULTI_FILL_IN_FAMILY
            | FLAT_NUMERIC_FAMILY
            | FLAT_MATCHING_FAMILY
            | FLAT_ORDERING_FAMILY
            | FLAT_HOTSPOT_FAMILY
    )
}

fn validate_flat_shape(
    family: &str,
    randomization: &RandomizationDefinition,
    response: &ResponseDefinition,
    grading: &GradingDefinition,
) -> Result<(), FlatQuestionError> {
    if !matches!(randomization, RandomizationDefinition::Static) {
        return invalid("flat family requires static randomization");
    }
    validate_response_for_family(family, response)?;
    let GradingDefinition::AllOrNothing { points } = grading else {
        return invalid("flat family requires all-or-nothing grading");
    };
    if !points.is_finite() || *points < 0.0 {
        return invalid("points must be finite and nonnegative");
    }
    Ok(())
}

fn validate_response_for_family(
    family: &str,
    response: &ResponseDefinition,
) -> Result<(), FlatQuestionError> {
    match (family, response) {
        (
            FLAT_SINGLE_CHOICE_V2_FAMILY,
            ResponseDefinition::MultipleChoice { choices, selection },
        ) if *selection == SelectionCardinality::ExactlyOne => validate_options(choices, 2),
        (
            FLAT_MULTIPLE_ANSWER_FAMILY,
            ResponseDefinition::MultipleChoice { choices, selection },
        ) if *selection == SelectionCardinality::AtLeastOne => validate_options(choices, 2),
        (FLAT_FILL_IN_FAMILY, ResponseDefinition::ShortText { max_length, .. })
            if *max_length > 0 =>
        {
            Ok(())
        }
        (FLAT_MULTI_FILL_IN_FAMILY, ResponseDefinition::MultiBlank { blanks })
            if !blanks.is_empty() && blanks.len() <= 50 =>
        {
            let mut ids = HashSet::new();
            for blank in blanks {
                validate_choice_id(blank.id.as_str())?;
                if blank.max_length == 0 || !ids.insert(blank.id.as_str()) {
                    return invalid("flat multi-blank slots must be unique and nonempty");
                }
            }
            Ok(())
        }
        (FLAT_NUMERIC_FAMILY, ResponseDefinition::Numeric { tolerance, .. }) => {
            validate_numeric_tolerance(tolerance)
        }
        (FLAT_MATCHING_FAMILY, ResponseDefinition::Matching { prompts, choices })
            if prompts.len() >= 2 && prompts.len() <= choices.len() =>
        {
            validate_options(prompts, 2)?;
            validate_options(choices, 2)
        }
        (FLAT_ORDERING_FAMILY, ResponseDefinition::Ordering { items }) => {
            validate_options(items, 3)
        }
        (
            FLAT_HOTSPOT_FAMILY,
            ResponseDefinition::Hotspot {
                surface,
                description,
                regions,
                selection,
            },
        ) if !regions.is_empty() => {
            if description.trim().is_empty()
                || !is_hex_sha256(&surface.checksum)
                || matches!(selection, SelectionCardinality::AnyNumber)
            {
                return invalid("flat hotspot surface or selection is invalid");
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
                    return invalid("flat hotspot region is invalid");
                }
            }
            Ok(())
        }
        _ => invalid("flat family and response definition do not agree"),
    }
}

fn validate_options(
    choices: &[question_model::response::ChoiceOption],
    minimum: usize,
) -> Result<(), FlatQuestionError> {
    if choices.len() < minimum || choices.len() > MAX_CHOICES {
        return invalid("flat selectable item count is outside the supported range");
    }
    let mut identifiers = HashSet::new();
    for choice in choices {
        validate_choice_id(choice.id.as_str())?;
        if choice.body.is_empty() || !identifiers.insert(choice.id.as_str()) {
            return invalid("flat selectable item identifiers and bodies must be valid");
        }
    }
    Ok(())
}

fn validate_numeric_tolerance(
    tolerance: &question_model::answer::NumericTolerance,
) -> Result<(), FlatQuestionError> {
    match tolerance {
        question_model::answer::NumericTolerance::Exact => Ok(()),
        question_model::answer::NumericTolerance::Absolute { epsilon } => {
            validate_nonnegative_finite("absolute epsilon", *epsilon)
        }
        question_model::answer::NumericTolerance::Relative { fraction } => {
            validate_nonnegative_finite("relative fraction", *fraction)
        }
        question_model::answer::NumericTolerance::SignificantFigures { digits } if *digits > 0 => {
            Ok(())
        }
        question_model::answer::NumericTolerance::SignificantFigures { .. } => {
            invalid("significant figures must be at least one")
        }
    }
}

fn validate_nonnegative_finite(name: &str, value: f64) -> Result<(), FlatQuestionError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        invalid(&format!("{name} must be finite and nonnegative"))
    }
}

fn validate_key_against_response(
    response: &ResponseDefinition,
    key: &AnswerKey,
) -> Result<(), FlatQuestionError> {
    match (response, key) {
        (ResponseDefinition::Numeric { .. }, AnswerKey::Numeric { expected })
            if expected.is_finite() =>
        {
            Ok(())
        }
        (
            ResponseDefinition::MultipleChoice { choices, .. },
            AnswerKey::MultipleChoice { correct },
        ) => {
            let available: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
            if correct.is_empty() || !correct.is_subset(&available) {
                return Err(FlatQuestionError::PublicBindingMismatch);
            }
            Ok(())
        }
        (ResponseDefinition::ShortText { .. }, AnswerKey::ShortText { accepted })
            if !accepted.is_empty() =>
        {
            Ok(())
        }
        (ResponseDefinition::MultiBlank { blanks }, AnswerKey::MultiBlank { accepted }) => {
            let available: BTreeSet<_> = blanks.iter().map(|blank| blank.id.clone()).collect();
            if accepted.len() != available.len()
                || accepted.keys().cloned().collect::<BTreeSet<_>>() != available
                || accepted.values().any(Vec::is_empty)
            {
                return Err(FlatQuestionError::PublicBindingMismatch);
            }
            Ok(())
        }
        (ResponseDefinition::Matching { prompts, choices }, AnswerKey::Matching { correct }) => {
            let prompt_ids: BTreeSet<_> = prompts.iter().map(|prompt| prompt.id.clone()).collect();
            let choice_ids: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
            let correct_choices: BTreeSet<_> = correct.values().cloned().collect();
            if correct.keys().cloned().collect::<BTreeSet<_>>() != prompt_ids
                || correct_choices.len() != correct.len()
                || !correct_choices.is_subset(&choice_ids)
            {
                return Err(FlatQuestionError::PublicBindingMismatch);
            }
            Ok(())
        }
        (ResponseDefinition::Ordering { items }, AnswerKey::Ordering { correct }) => {
            let available: BTreeSet<_> = items.iter().map(|item| item.id.clone()).collect();
            let keyed: BTreeSet<_> = correct.iter().cloned().collect();
            if keyed.len() != correct.len() || keyed != available {
                return Err(FlatQuestionError::PublicBindingMismatch);
            }
            Ok(())
        }
        (ResponseDefinition::Hotspot { regions, .. }, AnswerKey::Hotspot { correct }) => {
            let available: BTreeSet<_> = regions.iter().map(|region| region.id.clone()).collect();
            if correct.is_empty() || !correct.is_subset(&available) {
                return Err(FlatQuestionError::PublicBindingMismatch);
            }
            Ok(())
        }
        _ => Err(FlatQuestionError::PublicBindingMismatch),
    }
}

fn selectable_ids(response: &ResponseDefinition) -> BTreeSet<ChoiceId> {
    match response {
        ResponseDefinition::MultipleChoice { choices, .. } => {
            choices.iter().map(|choice| choice.id.clone()).collect()
        }
        ResponseDefinition::Matching { choices, .. } => {
            choices.iter().map(|choice| choice.id.clone()).collect()
        }
        ResponseDefinition::Ordering { items } => {
            items.iter().map(|item| item.id.clone()).collect()
        }
        ResponseDefinition::Hotspot { regions, .. } => {
            regions.iter().map(|region| region.id.clone()).collect()
        }
        ResponseDefinition::Numeric { .. }
        | ResponseDefinition::ShortText { .. }
        | ResponseDefinition::MultiBlank { .. }
        | ResponseDefinition::FileUpload { .. }
        | ResponseDefinition::ExternalTool {} => BTreeSet::new(),
    }
}

fn correct_response_blocks(
    response: &ResponseDefinition,
    key: &AnswerKey,
) -> Result<Vec<ContentBlock>, FlatQuestionError> {
    validate_key_against_response(response, key)?;
    let blocks = match (response, key) {
        (
            ResponseDefinition::MultipleChoice { choices, .. },
            AnswerKey::MultipleChoice { correct },
        ) => choices
            .iter()
            .filter(|choice| correct.contains(&choice.id))
            .flat_map(|choice| choice.body.clone())
            .collect(),
        (ResponseDefinition::ShortText { .. }, AnswerKey::ShortText { accepted }) => {
            markdown_blocks(&accepted.join("; "))
        }
        (ResponseDefinition::Numeric { unit, .. }, AnswerKey::Numeric { expected }) => {
            markdown_blocks(&format!(
                "{expected}{}",
                unit.as_deref()
                    .map_or(String::new(), |unit| format!(" {unit}"))
            ))
        }
        (ResponseDefinition::MultiBlank { blanks }, AnswerKey::MultiBlank { accepted }) => {
            vec![ContentBlock::Table {
                headers: vec!["Blank".to_string(), "Accepted response".to_string()],
                rows: blanks
                    .iter()
                    .map(|blank| vec![blocks_text(&blank.label), accepted[&blank.id].join("; ")])
                    .collect(),
                description: "Correct responses for each blank".to_string(),
            }]
        }
        (ResponseDefinition::Matching { prompts, choices }, AnswerKey::Matching { correct }) => {
            vec![ContentBlock::Table {
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
        (ResponseDefinition::Ordering { items }, AnswerKey::Ordering { correct }) => correct
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
        (ResponseDefinition::Hotspot { regions, .. }, AnswerKey::Hotspot { correct }) => regions
            .iter()
            .filter(|region| correct.contains(&region.id))
            .flat_map(|region| region.label.clone())
            .collect(),
        _ => return Err(FlatQuestionError::PublicBindingMismatch),
    };
    Ok(blocks)
}

fn blocks_text(blocks: &[ContentBlock]) -> String {
    blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { markdown } => markdown.as_str(),
            ContentBlock::Math { description, .. }
            | ContentBlock::Image { description, .. }
            | ContentBlock::Table { description, .. } => description.as_str(),
            ContentBlock::Code { source, .. } => source.as_str(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicBinding<'a> {
    source_family: &'a str,
    prompt: &'a [ContentBlock],
    response: &'a ResponseDefinition,
    attempt_policy: AttemptPolicy,
    timing_policy: TimingPolicy,
    randomization: &'a RandomizationDefinition,
    grading: &'a GradingDefinition,
    metadata: &'a QuestionMetadata,
}
fn public_binding_sha256_for_draft(
    draft: &DraftQuestionDefinition,
) -> Result<String, FlatQuestionError> {
    let DraftQuestionSource::Native { family } = &draft.source else {
        return Err(FlatQuestionError::PublicBindingMismatch);
    };
    public_binding_sha256(PublicBinding {
        source_family: family,
        prompt: &draft.prompt,
        response: &draft.response,
        attempt_policy: draft.attempt_policy,
        timing_policy: draft.timing_policy,
        randomization: &draft.randomization,
        grading: &draft.grading,
        metadata: &draft.metadata,
    })
}
fn public_binding_sha256_for_question(
    question: &QuestionDefinition,
) -> Result<String, FlatQuestionError> {
    let QuestionSource::Native { family } = &question.source else {
        return Err(FlatQuestionError::PublicBindingMismatch);
    };
    public_binding_sha256(PublicBinding {
        source_family: family,
        prompt: &question.prompt,
        response: &question.response,
        attempt_policy: question.attempt_policy,
        timing_policy: question.timing_policy,
        randomization: &question.randomization,
        grading: &question.grading,
        metadata: &question.metadata,
    })
}
fn public_binding_sha256(binding: PublicBinding<'_>) -> Result<String, FlatQuestionError> {
    serde_json::to_vec(&binding)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| FlatQuestionError::Encoding(error.to_string()))
}
fn sha256_hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}
fn markdown_blocks(markdown: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        markdown: markdown.to_string(),
    }]
}
fn validate_choice_id(value: &str) -> Result<(), FlatQuestionError> {
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
fn validate_feedback(value: &str) -> Result<(), FlatQuestionError> {
    validate_bounded_text("feedback", value, MAX_FEEDBACK_CHARS)
}
fn validate_optional_feedback(value: Option<&str>) -> Result<(), FlatQuestionError> {
    if let Some(value) = value {
        validate_feedback(value)?;
    }
    Ok(())
}
fn validate_bounded_text(
    name: &str,
    value: &str,
    maximum_chars: usize,
) -> Result<(), FlatQuestionError> {
    if value.trim().is_empty() {
        return invalid(&format!("{name} must not be blank"));
    }
    if value.chars().count() > maximum_chars {
        return invalid(&format!("{name} exceeds {maximum_chars} characters"));
    }
    Ok(())
}
fn is_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}
fn invalid<T>(message: &str) -> Result<T, FlatQuestionError> {
    Err(FlatQuestionError::InvalidDocument(message.to_string()))
}
