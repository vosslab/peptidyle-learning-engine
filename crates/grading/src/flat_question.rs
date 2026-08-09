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

/// Stable source-family identifier persisted in the public question model.
pub const FLAT_SINGLE_CHOICE_FAMILY: &str = "flat_single_choice_v1";
/// Upper bound shared by persisted private material and source adapters.
pub const MAX_FLAT_QUESTION_BYTES: usize = 256 * 1024;
const PRIVATE_SCHEMA_VERSION: u32 = 1;
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
    /// Builds answer-bearing material only after its exact public draft passed
    /// the family contract. Feedback identifiers are checked against choices.
    pub fn new(
        draft: &DraftQuestionDefinition,
        correct_choice: ChoiceId,
        choice_feedback: Vec<(ChoiceId, String)>,
        correct_feedback: Option<String>,
        incorrect_feedback: Option<String>,
    ) -> Result<Self, FlatQuestionError> {
        validate_for_draft(draft)?;
        let available = choices_for_draft(draft)?;
        if !available.contains(&correct_choice) {
            return invalid("correct choice must name an available choice");
        }
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
        Ok(Self {
            schema_version: PRIVATE_SCHEMA_VERSION,
            public_sha256: public_binding_sha256_for_draft(draft)?,
            answer_key: AnswerKey::MultipleChoice {
                correct: BTreeSet::from([correct_choice]),
            },
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
        self.validate_against_choices(choices_for_draft(draft)?)
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
        validate_flat_single_choice_question(question)?;
        if public_binding_sha256_for_question(question)? != self.public_sha256 {
            return Err(FlatQuestionError::PublicBindingMismatch);
        }
        self.validate_against_question(question)?;
        let outcome = grade(question, response, Some(&self.answer_key))
            .map_err(FlatQuestionError::Grading)?;
        let GradeOutcome::Graded(result) = outcome else {
            return Err(FlatQuestionError::Grading(GradingError::InvalidDefinition(
                "flat single-choice grading must produce a numeric result".to_string(),
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
        let AnswerKey::MultipleChoice { correct } = &self.answer_key else {
            return invalid("flat private material requires a multiple-choice answer key");
        };
        if correct.len() != 1 {
            return invalid("flat private material requires exactly one correct choice");
        }
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
        self.validate_against_choices(choices_for_question(question)?)
    }

    fn validate_against_choices(
        &self,
        available: BTreeSet<ChoiceId>,
    ) -> Result<(), FlatQuestionError> {
        let AnswerKey::MultipleChoice { correct } = &self.answer_key else {
            unreachable!("checked by validate_private_shape");
        };
        if !correct.is_subset(&available) {
            return Err(FlatQuestionError::PublicBindingMismatch);
        }
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
        let StudentResponse::MultipleChoice { selected } = response else {
            return invalid("flat single-choice feedback requires a choice response");
        };
        let Some(selected_choice) = selected.first() else {
            return invalid("flat single-choice feedback requires one selected choice");
        };
        let ResponseDefinition::MultipleChoice { choices, .. } = &question.response else {
            return invalid("flat single-choice public response kind changed");
        };
        let AnswerKey::MultipleChoice { correct } = &self.answer_key else {
            unreachable!("checked before grading");
        };
        let correct_choice = choices
            .iter()
            .find(|choice| correct.contains(&choice.id))
            .ok_or(FlatQuestionError::PublicBindingMismatch)?;
        let mut teaching = Vec::new();
        if let Some(feedback) = self
            .choice_feedback
            .iter()
            .find(|feedback| feedback.choice == selected_choice.as_str())
        {
            teaching.extend(markdown_blocks(&feedback.markdown));
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
            correct_response: Some(correct_choice.body.clone()),
            rationale: None,
        })
    }
}

/// Validates the public contract on a draft before it can receive private key material.
pub fn validate_for_draft(draft: &DraftQuestionDefinition) -> Result<(), FlatQuestionError> {
    let DraftQuestionSource::Native { family } = &draft.source else {
        return Err(FlatQuestionError::PublicBindingMismatch);
    };
    if family != FLAT_SINGLE_CHOICE_FAMILY {
        return Err(FlatQuestionError::PublicBindingMismatch);
    }
    validate_flat_shape(&draft.randomization, &draft.response, &draft.grading)
}

/// Alias for callers that name the flat family explicitly.
pub fn validate_flat_single_choice_draft(
    draft: &DraftQuestionDefinition,
) -> Result<(), FlatQuestionError> {
    validate_for_draft(draft)
}

/// Validates the immutable public form used by the native backend registry.
pub fn validate_flat_single_choice_question(
    question: &QuestionDefinition,
) -> Result<(), FlatQuestionError> {
    let QuestionSource::Native { family } = &question.source else {
        return Err(FlatQuestionError::PublicBindingMismatch);
    };
    if family != FLAT_SINGLE_CHOICE_FAMILY {
        return Err(FlatQuestionError::PublicBindingMismatch);
    }
    validate_flat_shape(
        &question.randomization,
        &question.response,
        &question.grading,
    )
}

fn validate_flat_shape(
    randomization: &RandomizationDefinition,
    response: &ResponseDefinition,
    grading: &GradingDefinition,
) -> Result<(), FlatQuestionError> {
    if !matches!(randomization, RandomizationDefinition::Static) {
        return invalid("flat family requires static randomization");
    }
    let ResponseDefinition::MultipleChoice { choices, selection } = response else {
        return invalid("flat family requires multiple-choice response");
    };
    if *selection != SelectionCardinality::ExactlyOne {
        return invalid("flat family requires exactly-one selection");
    }
    if !(2..=MAX_CHOICES).contains(&choices.len()) {
        return invalid("flat family requires 2 to 100 choices");
    }
    let mut identifiers = HashSet::new();
    for choice in choices {
        validate_choice_id(choice.id.as_str())?;
        if !identifiers.insert(choice.id.as_str()) {
            return invalid("choice identifiers must be unique");
        }
    }
    let GradingDefinition::AllOrNothing { points } = grading else {
        return invalid("flat family requires all-or-nothing grading");
    };
    if !points.is_finite() || *points < 0.0 {
        return invalid("points must be finite and nonnegative");
    }
    Ok(())
}

fn choices_for_draft(
    draft: &DraftQuestionDefinition,
) -> Result<BTreeSet<ChoiceId>, FlatQuestionError> {
    let ResponseDefinition::MultipleChoice { choices, .. } = &draft.response else {
        return invalid("flat family requires multiple-choice response");
    };
    Ok(choices.iter().map(|choice| choice.id.clone()).collect())
}
fn choices_for_question(
    question: &QuestionDefinition,
) -> Result<BTreeSet<ChoiceId>, FlatQuestionError> {
    let ResponseDefinition::MultipleChoice { choices, .. } = &question.response else {
        return invalid("flat family requires multiple-choice response");
    };
    Ok(choices.iter().map(|choice| choice.id.clone()).collect())
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
