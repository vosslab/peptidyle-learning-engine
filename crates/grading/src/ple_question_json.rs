//! Server-only integrity contract for PLE's static Question JSON format.
//!
//! The authoring parser deliberately lives in `adapter_ple`; this module
//! owns every rule whose failure could change correctness or disclose answers.

use std::collections::HashSet;

use domain::validation::StudentResponseFormatIssue;
use question_model::response::{
    QuestionResponseFormat, QuestionType, ResponseItemReference, StudentResponse,
};
use question_model::{
    QuestionAnswer, QuestionAnswerExplanation, QuestionEvaluation, QuestionFeedback,
    QuestionTitleError,
};
use serde::{Deserialize, Serialize};

use crate::AnswerKey;

#[path = "ple_question_json_validate.rs"]
mod ple_question_json_validate;
mod recorded_teaching;

use ple_question_json_validate::{
    evaluate_response, invalid, is_hex_sha256, markdown_blocks, question_answer_blocks,
    selectable_ids, validate_choice_id, validate_feedback, validate_key_against_response,
    validate_optional_feedback, validate_response_for_type,
};

pub use recorded_teaching::PleQuestionJsonRecordedTeachingContent;

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
    InvalidDocument(String),
    InvalidQuestionTitle(QuestionTitleError),
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
            Self::InvalidDocument(message) => {
                write!(formatter, "invalid PLE Question JSON document: {message}")
            }
            Self::InvalidQuestionTitle(error) => error.fmt(formatter),
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
            question_feedback: self.question_feedback_for(response, result.correct())?,
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
        correct: bool,
    ) -> Result<QuestionFeedback, PleQuestionJsonError> {
        let mut feedback = self.selected_choice_feedback_for(response)?;
        let (correct_feedback, incorrect_feedback) = if correct {
            (self.outcome_feedback.correct.as_deref(), None)
        } else {
            (None, self.outcome_feedback.incorrect.as_deref())
        };
        feedback.correct_feedback = correct_feedback.map(markdown_blocks);
        feedback.incorrect_feedback = incorrect_feedback.map(markdown_blocks);
        Ok(feedback)
    }

    fn selected_choice_feedback_for(
        &self,
        response: &StudentResponse,
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
        Ok(QuestionFeedback {
            choice_feedback: (!choice_feedback.is_empty()).then_some(choice_feedback),
            correct_feedback: None,
            incorrect_feedback: None,
        })
    }

    fn question_answer_for(
        &self,
        response_format: &QuestionResponseFormat,
    ) -> Result<QuestionAnswer, PleQuestionJsonError> {
        let question_answer =
            QuestionAnswer::new(question_answer_blocks(response_format, &self.answer_key)?)
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
