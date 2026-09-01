//! Server-only integrity contract for PLE's static flat-question format.
//!
//! The authoring parser deliberately lives in `adapter_ple`; this module
//! owns every rule whose failure could change correctness or disclose answers.

use std::collections::{BTreeSet, HashSet};
use std::fmt::Write as _;

use question_model::answer::ResponseSelectionRule;
use question_model::assignment_activity_rules::{QuestionAttemptLimit, QuestionAttemptTimeLimit};
use question_model::envelope::QuestionContentBlock;
use question_model::generation::QuestionVariationDefinition;
use question_model::response::{
    QuestionResponseFormat, QuestionType, ResponseItemReference, StudentResponse,
};
use question_model::{
    DraftQuestionBackendLocator, DraftQuestionRevision, GradingResult, QuestionAnswer,
    QuestionBackendLocator, QuestionFeedback, QuestionFormat, QuestionGradingRule,
    QuestionMetadata, QuestionPostGradingContent, QuestionRevision, QuestionTitleError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AnswerKey, GradingError, QuestionGradingOutcome, grade};

/// Upper bound shared by persisted private material and source adapters.
pub const MAX_FLAT_QUESTION_BYTES: usize = 256 * 1024;
const PRIVATE_SCHEMA_VERSION: u32 = 2;
const MAX_CHOICES: usize = 100;
const MAX_CHOICE_ID_BYTES: usize = 64;
const MAX_FEEDBACK_CHARS: usize = 16_384;

/// Stable errors for flat question parsing, validation, persistence, and grading.
///
/// The PLE Question Backend re-exports this type to retain its established public API.
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
    pub outcome: QuestionGradingOutcome,
    pub post_grading_content: QuestionPostGradingContent,
}

impl FlatQuestionPrivate {
    /// Builds private material for one of the closed v2 flat Question Types.
    pub fn new_with_key(
        draft: &DraftQuestionRevision,
        answer_key: AnswerKey,
        choice_feedback: Vec<(ResponseItemReference, String)>,
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

    /// Rebinds the unchanged private Answer Key and Question Feedback to the exact public draft
    /// emitted during publication.
    ///
    /// Publication uses this only when a private HOTSPOT workspace asset is
    /// assigned its fresh version-scoped Question Library asset identity.  The answer
    /// key and feedback remain byte-for-byte unchanged; the public binding
    /// digest changes because that browser-safe asset identifier is part of
    /// the public Question Response Format.
    pub fn rebind_to_draft(
        &self,
        draft: &DraftQuestionRevision,
    ) -> Result<Self, FlatQuestionError> {
        self.validate_private_shape()?;
        // The caller has already validated these private Question records against the
        // staged draft. Publication may now change only the version-scoped
        // HOTSPOT asset ID, so validate every semantic key/feedback relation
        // against the new definition without requiring the old binding hash.
        validate_for_draft(draft)?;
        validate_key_against_response(&draft.response, &self.answer_key)?;
        self.validate_feedback_targets(&draft.response)?;
        Ok(Self {
            schema_version: self.schema_version,
            public_sha256: public_binding_sha256_for_draft(draft)?,
            answer_key: self.answer_key.clone(),
            choice_feedback: self.choice_feedback.clone(),
            outcome_feedback: self.outcome_feedback.clone(),
        })
    }

    /// Validates this private material against its editable public draft.
    ///
    /// Publication uses this seam before durable identifiers exist. It proves
    /// that the key and feedback describe this exact public payload without
    /// fabricating a published [`QuestionRevision`].
    pub fn validate_for_draft(
        &self,
        draft: &DraftQuestionRevision,
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
        question: &QuestionRevision,
        response: &StudentResponse,
    ) -> Result<FlatQuestionEvaluation, FlatQuestionError> {
        validate_flat_question_question(question)?;
        if public_binding_sha256_for_question(question)? != self.public_sha256 {
            return Err(FlatQuestionError::PublicBindingMismatch);
        }
        self.validate_against_question(question)?;
        let outcome = grade(question, response, Some(&self.answer_key))
            .map_err(FlatQuestionError::Grading)?;
        let QuestionGradingOutcome::Graded(result) = outcome else {
            return Err(FlatQuestionError::Grading(GradingError::InvalidDefinition(
                "flat-question grading must produce a numeric result".to_string(),
            )));
        };
        Ok(FlatQuestionEvaluation {
            outcome: QuestionGradingOutcome::Graded(result),
            post_grading_content: self.post_grading_content_for(question, response, result)?,
        })
    }

    /// Verifies this private material against one exact immutable published
    /// definition before an issuance capability retains it for later grade.
    pub fn validate_for_question(
        &self,
        question: &QuestionRevision,
    ) -> Result<(), FlatQuestionError> {
        validate_flat_question_question(question)?;
        if public_binding_sha256_for_question(question)? != self.public_sha256 {
            return Err(FlatQuestionError::PublicBindingMismatch);
        }
        self.validate_against_question(question)
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
        question: &QuestionRevision,
    ) -> Result<(), FlatQuestionError> {
        self.validate_private_shape()?;
        validate_key_against_response(&question.response, &self.answer_key)?;
        self.validate_feedback_targets(&question.response)
    }

    fn validate_feedback_targets(
        &self,
        response: &QuestionResponseFormat,
    ) -> Result<(), FlatQuestionError> {
        let available = selectable_ids(response);
        if self
            .choice_feedback
            .iter()
            .any(|feedback| !available.contains(&ResponseItemReference::new(&feedback.choice)))
        {
            return Err(FlatQuestionError::PublicBindingMismatch);
        }
        Ok(())
    }

    fn post_grading_content_for(
        &self,
        question: &QuestionRevision,
        response: &StudentResponse,
        result: GradingResult,
    ) -> Result<QuestionPostGradingContent, FlatQuestionError> {
        let mut choice_feedback = Vec::new();
        if let StudentResponse::MultipleChoice { selected } = response {
            for selected_choice in selected {
                if let Some(feedback) = self
                    .choice_feedback
                    .iter()
                    .find(|feedback| feedback.choice == selected_choice.as_str())
                {
                    choice_feedback.extend(markdown_blocks(&feedback.markdown));
                }
            }
        }
        let (correct_feedback, incorrect_feedback) = if result.correct {
            (self.outcome_feedback.correct.as_deref(), None)
        } else {
            (None, self.outcome_feedback.incorrect.as_deref())
        };
        let question_answer = QuestionAnswer::new(correct_response_blocks(
            &question.response,
            &self.answer_key,
        )?)
        .ok_or(FlatQuestionError::PublicBindingMismatch)?;
        Ok(QuestionPostGradingContent {
            question_feedback: QuestionFeedback {
                choice_feedback: (!choice_feedback.is_empty()).then_some(choice_feedback),
                correct_feedback: correct_feedback.map(markdown_blocks),
                incorrect_feedback: incorrect_feedback.map(markdown_blocks),
            },
            question_answer: Some(question_answer),
            question_answer_explanation: None,
        })
    }
}

/// Validates the public contract on a draft before it can receive private key material.
pub fn validate_for_draft(draft: &DraftQuestionRevision) -> Result<(), FlatQuestionError> {
    if !matches!(draft.backend_locator, DraftQuestionBackendLocator::Ple)
        || draft.question_format != QuestionFormat::PleFlatQuestionV2
    {
        return Err(FlatQuestionError::PublicBindingMismatch);
    }
    validate_flat_shape(
        draft.question_type,
        &draft.question_variation_definition,
        &draft.response,
        &draft.grading,
    )
}

/// Validates a closed flat Question Type after publication.
pub fn validate_flat_question_question(
    question: &QuestionRevision,
) -> Result<(), FlatQuestionError> {
    if !matches!(question.backend_locator, QuestionBackendLocator::Ple)
        || question.question_format != QuestionFormat::PleFlatQuestionV2
    {
        return Err(FlatQuestionError::PublicBindingMismatch);
    }
    validate_flat_shape(
        question.question_type,
        &question.question_variation_definition,
        &question.response,
        &question.grading,
    )
}

fn validate_flat_shape(
    question_type: QuestionType,
    question_variation_definition: &QuestionVariationDefinition,
    response: &QuestionResponseFormat,
    grading: &QuestionGradingRule,
) -> Result<(), FlatQuestionError> {
    if !matches!(
        question_variation_definition,
        QuestionVariationDefinition::Static
    ) {
        return invalid("flat questions require a static Question Variation Definition");
    }
    validate_response_for_type(question_type, response)?;
    let QuestionGradingRule::AllOrNothing { points } = grading else {
        return invalid("flat Question Type requires all-or-nothing grading");
    };
    if !points.is_finite() || *points < 0.0 {
        return invalid("points must be finite and nonnegative");
    }
    Ok(())
}

fn validate_response_for_type(
    question_type: QuestionType,
    response: &QuestionResponseFormat,
) -> Result<(), FlatQuestionError> {
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
                    return invalid("flat multi-blank slots must be unique and nonempty");
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
        _ => invalid("flat Question Type and Question Response Format do not agree"),
    }
}

trait SelectableResponseItem {
    fn id(&self) -> &question_model::response::ResponseItemReference;
    fn body(&self) -> &[QuestionContentBlock];
}

impl SelectableResponseItem for question_model::response::QuestionChoice {
    fn id(&self) -> &question_model::response::ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

impl SelectableResponseItem for question_model::response::MatchingPrompt {
    fn id(&self) -> &question_model::response::ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

impl SelectableResponseItem for question_model::response::MatchingChoice {
    fn id(&self) -> &question_model::response::ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

impl SelectableResponseItem for question_model::response::OrderingItem {
    fn id(&self) -> &question_model::response::ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

fn validate_options<T: SelectableResponseItem>(
    choices: &[T],
    minimum: usize,
) -> Result<(), FlatQuestionError> {
    if choices.len() < minimum || choices.len() > MAX_CHOICES {
        return invalid("flat selectable item count is outside the supported range");
    }
    let mut identifiers = HashSet::new();
    for choice in choices {
        validate_choice_id(choice.id().as_str())?;
        if choice.body().is_empty() || !identifiers.insert(choice.id().as_str()) {
            return invalid("flat selectable item identifiers and bodies must be valid");
        }
    }
    Ok(())
}

fn validate_numeric_tolerance(
    tolerance: &question_model::answer::NumericResponseTolerance,
) -> Result<(), FlatQuestionError> {
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

fn validate_nonnegative_finite(name: &str, value: f64) -> Result<(), FlatQuestionError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        invalid(&format!("{name} must be finite and nonnegative"))
    }
}

fn validate_key_against_response(
    response: &QuestionResponseFormat,
    key: &AnswerKey,
) -> Result<(), FlatQuestionError> {
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
                return Err(FlatQuestionError::PublicBindingMismatch);
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
                return Err(FlatQuestionError::PublicBindingMismatch);
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
                return Err(FlatQuestionError::PublicBindingMismatch);
            }
            Ok(())
        }
        (QuestionResponseFormat::Ordering { items }, AnswerKey::Ordering { correct }) => {
            let available: BTreeSet<_> = items.iter().map(|item| item.id.clone()).collect();
            let keyed: BTreeSet<_> = correct.iter().cloned().collect();
            if keyed.len() != correct.len() || keyed != available {
                return Err(FlatQuestionError::PublicBindingMismatch);
            }
            Ok(())
        }
        (QuestionResponseFormat::Hotspot { regions, .. }, AnswerKey::Hotspot { correct }) => {
            let available: BTreeSet<_> = regions.iter().map(|region| region.id.clone()).collect();
            if correct.is_empty() || !correct.is_subset(&available) {
                return Err(FlatQuestionError::PublicBindingMismatch);
            }
            Ok(())
        }
        _ => Err(FlatQuestionError::PublicBindingMismatch),
    }
}

fn selectable_ids(response: &QuestionResponseFormat) -> BTreeSet<ResponseItemReference> {
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
        | QuestionResponseFormat::ExternalTool {} => BTreeSet::new(),
    }
}

fn correct_response_blocks(
    response: &QuestionResponseFormat,
    key: &AnswerKey,
) -> Result<Vec<QuestionContentBlock>, FlatQuestionError> {
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
                description: "Correct responses for each blank".to_string(),
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
        _ => return Err(FlatQuestionError::PublicBindingMismatch),
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicBinding<'a> {
    question_format: QuestionFormat,
    question_type: QuestionType,
    prompt: &'a [QuestionContentBlock],
    response: &'a QuestionResponseFormat,
    question_attempt_limit: QuestionAttemptLimit,
    question_attempt_time_limit: QuestionAttemptTimeLimit,
    question_variation_definition: &'a QuestionVariationDefinition,
    grading: &'a QuestionGradingRule,
    metadata: &'a QuestionMetadata,
}
/// Returns the checksum that binds private Answer Key and Question Feedback to one exact
/// browser-safe PLE flat Question Revision.
pub fn public_binding_sha256_for_draft(
    draft: &DraftQuestionRevision,
) -> Result<String, FlatQuestionError> {
    if !matches!(draft.backend_locator, DraftQuestionBackendLocator::Ple)
        || draft.question_format != QuestionFormat::PleFlatQuestionV2
    {
        return Err(FlatQuestionError::PublicBindingMismatch);
    }
    public_binding_sha256(PublicBinding {
        question_format: draft.question_format,
        question_type: draft.question_type,
        prompt: &draft.prompt,
        response: &draft.response,
        question_attempt_limit: draft.question_attempt_limit,
        question_attempt_time_limit: draft.question_attempt_time_limit,
        question_variation_definition: &draft.question_variation_definition,
        grading: &draft.grading,
        metadata: &draft.metadata,
    })
}
fn public_binding_sha256_for_question(
    question: &QuestionRevision,
) -> Result<String, FlatQuestionError> {
    if !matches!(question.backend_locator, QuestionBackendLocator::Ple)
        || question.question_format != QuestionFormat::PleFlatQuestionV2
    {
        return Err(FlatQuestionError::PublicBindingMismatch);
    }
    public_binding_sha256(PublicBinding {
        question_format: question.question_format,
        question_type: question.question_type,
        prompt: &question.prompt,
        response: &question.response,
        question_attempt_limit: question.question_attempt_limit,
        question_attempt_time_limit: question.question_attempt_time_limit,
        question_variation_definition: &question.question_variation_definition,
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
fn markdown_blocks(markdown: &str) -> Vec<QuestionContentBlock> {
    vec![QuestionContentBlock::Text {
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
