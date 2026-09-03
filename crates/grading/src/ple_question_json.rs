//! Server-only integrity contract for PLE's static Question JSON format.
//!
//! The authoring parser deliberately lives in `adapter_ple`; this module
//! owns every rule whose failure could change correctness or disclose answers.

use std::collections::{BTreeSet, HashSet};

use domain::validation::StudentResponseFormatIssue;
use question_model::QuestionContentBlock;
use question_model::answer::ResponseSelectionRule;
use question_model::response::{
    QuestionResponseFormat, QuestionType, ResponseItemReference, StudentResponse,
};
use question_model::{
    QuestionAnswer, QuestionAnswerExplanation, QuestionEvaluation, QuestionFeedback,
    QuestionTitleError,
};
use serde::{Deserialize, Serialize};

use crate::AnswerKey;

#[derive(Debug, Clone, PartialEq)]
pub enum PleQuestionJsonGradingError {
    InvalidResponse(Vec<StudentResponseFormatIssue>),
    KindMismatch,
    InvalidSource(String),
}

impl std::fmt::Display for PleQuestionJsonGradingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidResponse(_) => {
                formatter.write_str("response does not match the PLE Question JSON format")
            }
            Self::KindMismatch => {
                formatter.write_str("PLE Question JSON answer key does not match response format")
            }
            Self::InvalidSource(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for PleQuestionJsonGradingError {}

/// Upper bound shared by persisted PLE Question JSON Private Grading and source adapters.
pub const MAX_PLE_QUESTION_JSON_BYTES: usize = 256 * 1024;
const PRIVATE_SCHEMA_VERSION: u32 = 2;
const MAX_CHOICES: usize = 100;
const MAX_CHOICE_ID_BYTES: usize = 64;
const MAX_FEEDBACK_CHARS: usize = 16_384;

/// Stable errors for PLE Question JSON parsing, validation, persistence, and grading.
///
/// The PLE Question Backend re-exports this type to retain its established public API.
#[derive(Debug, Clone, PartialEq)]
pub enum PleQuestionJsonError {
    TooLarge,
    MalformedJson(String),
    UnsupportedFormat,
    UnsupportedVersion(u32),
    InvalidDocument(String),
    InvalidTitle(QuestionTitleError),
    PublicContentChecksumMismatch,
    Grading(PleQuestionJsonGradingError),
    Encoding(String),
}

impl std::fmt::Display for PleQuestionJsonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(
                formatter,
                "PLE Question JSON exceeds {MAX_PLE_QUESTION_JSON_BYTES} bytes"
            ),
            Self::MalformedJson(message) => {
                write!(formatter, "invalid PLE Question JSON: {message}")
            }
            Self::UnsupportedFormat => formatter.write_str("unsupported PLE Question JSON format"),
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "unsupported PLE Question JSON schema version {version}"
            ),
            Self::InvalidDocument(message) => {
                write!(formatter, "invalid PLE Question JSON document: {message}")
            }
            Self::InvalidTitle(error) => error.fmt(formatter),
            Self::PublicContentChecksumMismatch => formatter.write_str(
                "PLE Question JSON Private Grading does not match the PLE Question JSON public content",
            ),
            Self::Grading(error) => error.fmt(formatter),
            Self::Encoding(message) => {
                write!(formatter, "PLE Question JSON encoding failed: {message}")
            }
        }
    }
}

impl std::error::Error for PleQuestionJsonError {}

/// Server-only Answer Key and Question Feedback bound to one exact public
/// Question payload by its PLE Question JSON Public Content Checksum.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PleQuestionJsonPrivateGrading {
    schema_version: u32,
    public_content_checksum: String,
    answer_key: AnswerKey,
    choice_feedback: Vec<PleQuestionJsonChoiceFeedback>,
    outcome_feedback: PleQuestionJsonOutcomeFeedback,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonChoiceFeedback {
    choice: String,
    markdown: String,
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonOutcomeFeedback {
    #[serde(default)]
    correct: Option<String>,
    #[serde(default)]
    incorrect: Option<String>,
}

/// Grading Result plus selected Question Feedback, Question Answer, and Question
/// Answer Explanation from one trusted evaluation.
pub struct PleQuestionJsonEvaluation {
    pub evaluation: QuestionEvaluation,
    pub question_feedback: QuestionFeedback,
    pub question_answer: Option<QuestionAnswer>,
    pub question_answer_explanation: Option<QuestionAnswerExplanation>,
}

impl PleQuestionJsonPrivateGrading {
    /// Builds PLE Question JSON Private Grading for one of the closed v2 PLE Question JSON Types.
    pub fn new_with_key(
        source_checksum: String,
        question_type: QuestionType,
        response_format: &QuestionResponseFormat,
        answer_key: AnswerKey,
        choice_feedback: Vec<(ResponseItemReference, String)>,
        correct_feedback: Option<String>,
        incorrect_feedback: Option<String>,
    ) -> Result<Self, PleQuestionJsonError> {
        validate_ple_question_json_shape(question_type, response_format)?;
        if !is_hex_sha256(&source_checksum) {
            return invalid("source checksum must be a 64-character lowercase SHA-256 checksum");
        }
        let available = selectable_ids(response_format);
        let mut feedback_ids = HashSet::new();
        let mut feedback = Vec::with_capacity(choice_feedback.len());
        for (choice, markdown) in choice_feedback {
            if !available.contains(&choice) || !feedback_ids.insert(choice.clone()) {
                return invalid("choice feedback targets must be unique available choices");
            }
            validate_feedback(&markdown)?;
            feedback.push(PleQuestionJsonChoiceFeedback {
                choice: choice.as_str().to_string(),
                markdown,
            });
        }
        validate_optional_feedback(correct_feedback.as_deref())?;
        validate_optional_feedback(incorrect_feedback.as_deref())?;
        validate_key_against_response(response_format, &answer_key)?;
        Ok(Self {
            schema_version: PRIVATE_SCHEMA_VERSION,
            public_content_checksum: source_checksum,
            answer_key,
            choice_feedback: feedback,
            outcome_feedback: PleQuestionJsonOutcomeFeedback {
                correct: correct_feedback,
                incorrect: incorrect_feedback,
            },
        })
    }

    pub fn public_content_checksum(&self) -> &str {
        &self.public_content_checksum
    }

    /// Rebinds the unchanged server-only Answer Key and Question Feedback to
    /// the exact PLE Question JSON public content emitted during publication.
    ///
    /// Publication uses this only when a private HOTSPOT workspace asset is
    /// assigned its fresh version-scoped Question Library asset identity. The
    /// Answer Key and Question Feedback remain byte-for-byte unchanged; the
    /// PLE Question JSON Public Content Checksum changes because that
    /// browser-safe asset identifier is part of the public content.
    pub fn rebind_to_source(
        &self,
        source_checksum: String,
        question_type: QuestionType,
        response_format: &QuestionResponseFormat,
    ) -> Result<Self, PleQuestionJsonError> {
        self.validate_private_shape()?;
        // The caller has already validated these private Question records against the
        // staged draft. Publication may now change only the version-scoped
        // HOTSPOT asset ID, so validate every semantic key/feedback relation
        // against the new PLE Question JSON public content without requiring the old
        // PLE Question JSON Public Content Checksum.
        validate_ple_question_json_shape(question_type, response_format)?;
        validate_key_against_response(response_format, &self.answer_key)?;
        self.validate_feedback_targets(response_format)?;
        Ok(Self {
            schema_version: self.schema_version,
            public_content_checksum: source_checksum,
            answer_key: self.answer_key.clone(),
            choice_feedback: self.choice_feedback.clone(),
            outcome_feedback: self.outcome_feedback.clone(),
        })
    }

    /// Validates this PLE Question JSON Private Grading against its editable public draft.
    ///
    /// Publication uses this seam before durable identifiers exist. It proves
    /// that the Answer Key and Question Feedback describe this exact public payload without
    /// fabricating a generic published-question wrapper.
    pub fn validate_for_source(
        &self,
        source_checksum: &str,
        question_type: QuestionType,
        response_format: &QuestionResponseFormat,
    ) -> Result<(), PleQuestionJsonError> {
        validate_ple_question_json_shape(question_type, response_format)?;
        if source_checksum != self.public_content_checksum {
            return Err(PleQuestionJsonError::PublicContentChecksumMismatch);
        }
        self.validate_private_shape()?;
        validate_key_against_response(response_format, &self.answer_key)?;
        self.validate_feedback_targets(response_format)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, PleQuestionJsonError> {
        if bytes.len() > MAX_PLE_QUESTION_JSON_BYTES {
            return Err(PleQuestionJsonError::TooLarge);
        }
        let value: Self = serde_json::from_slice(bytes)
            .map_err(|error| PleQuestionJsonError::MalformedJson(error.to_string()))?;
        if value.canonical_bytes()? != bytes {
            return Err(PleQuestionJsonError::MalformedJson(
                "PLE Question JSON Private Grading is not canonical".to_string(),
            ));
        }
        value.validate_private_shape()?;
        Ok(value)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, PleQuestionJsonError> {
        serde_json::to_vec(self).map_err(|error| PleQuestionJsonError::Encoding(error.to_string()))
    }

    pub fn evaluate(
        &self,
        source_checksum: &str,
        question_type: QuestionType,
        response_format: &QuestionResponseFormat,
        response: &StudentResponse,
    ) -> Result<PleQuestionJsonEvaluation, PleQuestionJsonError> {
        self.validate_for_source(source_checksum, question_type, response_format)?;
        if source_checksum != self.public_content_checksum {
            return Err(PleQuestionJsonError::PublicContentChecksumMismatch);
        }
        let result = evaluate_response(response_format, response, &self.answer_key)?;
        Ok(PleQuestionJsonEvaluation {
            evaluation: result,
            question_feedback: self.question_feedback_for(response, result)?,
            question_answer: Some(self.question_answer_for(response_format)?),
            question_answer_explanation: None,
        })
    }

    /// Verifies this PLE Question JSON Private Grading against one exact immutable published
    /// PLE Question JSON public content before an issuance capability retains it for later grade.
    pub fn validate_for_source_document(
        &self,
        source_checksum: &str,
        question_type: QuestionType,
        response_format: &QuestionResponseFormat,
    ) -> Result<(), PleQuestionJsonError> {
        validate_ple_question_json_shape(question_type, response_format)?;
        if source_checksum != self.public_content_checksum {
            return Err(PleQuestionJsonError::PublicContentChecksumMismatch);
        }
        self.validate_private_shape()?;
        validate_key_against_response(response_format, &self.answer_key)?;
        self.validate_feedback_targets(response_format)
    }

    fn validate_private_shape(&self) -> Result<(), PleQuestionJsonError> {
        if self.schema_version != PRIVATE_SCHEMA_VERSION {
            return Err(PleQuestionJsonError::UnsupportedVersion(
                self.schema_version,
            ));
        }
        if !is_hex_sha256(&self.public_content_checksum) {
            return invalid(
                "publicContentChecksum must be a 64-character lowercase SHA-256 checksum",
            );
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

    fn validate_feedback_targets(
        &self,
        response: &QuestionResponseFormat,
    ) -> Result<(), PleQuestionJsonError> {
        let available = selectable_ids(response);
        if self
            .choice_feedback
            .iter()
            .any(|feedback| !available.contains(&ResponseItemReference::new(&feedback.choice)))
        {
            return Err(PleQuestionJsonError::PublicContentChecksumMismatch);
        }
        Ok(())
    }

    fn question_feedback_for(
        &self,
        response: &StudentResponse,
        result: QuestionEvaluation,
    ) -> Result<QuestionFeedback, PleQuestionJsonError> {
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
        let (correct_feedback, incorrect_feedback) = if result.correct() {
            (self.outcome_feedback.correct.as_deref(), None)
        } else {
            (None, self.outcome_feedback.incorrect.as_deref())
        };
        Ok(QuestionFeedback {
            choice_feedback: (!choice_feedback.is_empty()).then_some(choice_feedback),
            correct_feedback: correct_feedback.map(markdown_blocks),
            incorrect_feedback: incorrect_feedback.map(markdown_blocks),
        })
    }

    fn question_answer_for(
        &self,
        response_format: &QuestionResponseFormat,
    ) -> Result<QuestionAnswer, PleQuestionJsonError> {
        let question_answer =
            QuestionAnswer::new(correct_response_blocks(response_format, &self.answer_key)?)
                .ok_or(PleQuestionJsonError::PublicContentChecksumMismatch)?;
        Ok(question_answer)
    }
}

/// Validates the closed PLE Question JSON type and response contract.
pub fn validate_ple_question_json_shape(
    question_type: QuestionType,
    response: &QuestionResponseFormat,
) -> Result<(), PleQuestionJsonError> {
    validate_response_for_type(question_type, response)?;
    Ok(())
}

fn validate_response_for_type(
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
/// container or Assignment scoring rule. PLE Question JSON v3 evaluates each
/// valid response all-or-nothing, returning normalized credit of zero or one.
fn evaluate_response(
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

fn validate_key_against_response(
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
        | QuestionResponseFormat::ImathasQuestionBackend {} => BTreeSet::new(),
    }
}

fn correct_response_blocks(
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

fn markdown_blocks(markdown: &str) -> Vec<QuestionContentBlock> {
    vec![QuestionContentBlock::Text {
        markdown: markdown.to_string(),
    }]
}
fn validate_choice_id(value: &str) -> Result<(), PleQuestionJsonError> {
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
fn validate_feedback(value: &str) -> Result<(), PleQuestionJsonError> {
    validate_bounded_text("feedback", value, MAX_FEEDBACK_CHARS)
}
fn validate_optional_feedback(value: Option<&str>) -> Result<(), PleQuestionJsonError> {
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
fn is_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}
fn invalid<T>(message: &str) -> Result<T, PleQuestionJsonError> {
    Err(PleQuestionJsonError::InvalidDocument(message.to_string()))
}
