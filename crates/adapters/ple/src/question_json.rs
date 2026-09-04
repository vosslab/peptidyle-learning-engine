//! Strict PLE Question JSON source and compiler for static Questions.
//!
//! The closed version 3 Question Type set follows the reviewed QTI Package Maker item model.
//! Parsing produces two values:
//! a browser-safe draft and PLE Question JSON Private Grading. The latter stays
//! in this server-only adapter crate and is bound by checksum to the public
//! PLE Question JSON public content it grades.

use std::fmt::Write as _;

pub use grading::ple_question_json::{
    PleQuestionJsonError, PleQuestionJsonEvaluation, PleQuestionJsonPrivateGrading,
    validate_ple_question_json_shape,
};
use question_model::QuestionContentBlock;
use question_model::{QuestionHint, QuestionType};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Trusted QTI-profile mapping bridge for canonical PLE Question JSON source.
pub mod imported;
mod schema_v3;

/// Canonical media type for canonical PLE Question JSON source payloads.
pub const PLE_QUESTION_JSON_MEDIA_TYPE: &str = "application/vnd.peptidyle.question+json";

/// Maximum accepted source size, matching the grading boundary's PLE Question JSON payload backstop.
pub const MAX_PLE_QUESTION_JSON_BYTES: usize =
    grading::ple_question_json::MAX_PLE_QUESTION_JSON_BYTES;

const PLE_QUESTION_JSON_FORMAT_NAME: &str = "pleQuestionJson";
const MAX_CHOICES: usize = 100;
const MAX_CHOICE_ID_BYTES: usize = 64;
const MAX_PROMPT_CHARS: usize = 65_536;
const MAX_CHOICE_TEXT_CHARS: usize = 16_384;
const MAX_FEEDBACK_CHARS: usize = 16_384;
const MAX_HINT_CHARS: usize = 16_384;
const MAX_TAG_CHARS: usize = 128;
const MAX_METADATA_TEXT_CHARS: usize = 256;

/// Answer-bearing authoring document decoded from PLE Question JSON.
///
/// The type intentionally has no `Debug` implementation because it contains
/// the correct choice and private teaching feedback. Use [`Self::compile`] to
/// split it before persistence or delivery.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PleQuestionJsonDocument(schema_v3::PleQuestionJsonDocumentBody);

/// One student-visible choice and its optional private teaching feedback.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonChoice {
    id: String,
    text: String,
    #[serde(default)]
    feedback: Option<String>,
}

/// Feedback selected from the final correctness outcome.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonOutcomeFeedback {
    #[serde(default)]
    correct: Option<String>,
    #[serde(default)]
    incorrect: Option<String>,
}

/// Public draft plus separately persisted server-only grading and pre-response teaching content.
#[derive(Clone)]
pub struct PleQuestionJsonPresentation {
    question_title: String,
    prompt: Vec<QuestionContentBlock>,
    response: question_model::response::QuestionResponseFormat,
    question_type: QuestionType,
}

impl PleQuestionJsonPresentation {
    pub fn question_title(&self) -> &str {
        &self.question_title
    }
    pub fn prompt(&self) -> &[QuestionContentBlock] {
        &self.prompt
    }
    pub fn response(&self) -> &question_model::response::QuestionResponseFormat {
        &self.response
    }
    pub fn question_type(&self) -> QuestionType {
        self.question_type
    }
}

/// Public PLE presentation plus private derivations from exact source bytes.
#[derive(Clone)]
pub struct CompiledPleQuestionJson {
    presentation: PleQuestionJsonPresentation,
    private: PleQuestionJsonPrivateGrading,
    question_hint: Option<QuestionHint>,
}

impl CompiledPleQuestionJson {
    /// Returns the answer-free presentation derived directly from PLE source.
    pub fn presentation(&self) -> &PleQuestionJsonPresentation {
        &self.presentation
    }

    /// Returns the PLE Question JSON Private Grading accepted only by a grading capability.
    pub fn private(&self) -> &PleQuestionJsonPrivateGrading {
        &self.private
    }

    /// Returns the optional pre-response Question Hint persisted separately from the draft and
    /// Private Grading.
    pub fn question_hint(&self) -> Option<&QuestionHint> {
        self.question_hint.as_ref()
    }
}

impl PleQuestionJsonDocument {
    /// Parses and validates one complete answer-bearing JSON source.
    ///
    /// # Errors
    ///
    /// Refuses oversized input, malformed or duplicate members, unknown
    /// fields, unsupported versions, and invalid v3 content.
    pub fn parse(bytes: &[u8]) -> Result<Self, PleQuestionJsonError> {
        if bytes.len() > MAX_PLE_QUESTION_JSON_BYTES {
            return Err(PleQuestionJsonError::TooLarge);
        }
        let document: Self = serde_json::from_slice(bytes)
            .map_err(|error| PleQuestionJsonError::MalformedJson(error.to_string()))?;
        document.validate()?;
        Ok(document)
    }

    /// Returns compact canonical bytes for checksumming and immutable storage.
    ///
    /// # Errors
    ///
    /// Returns an encoding error only if the already validated typed document
    /// cannot be represented as JSON.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, PleQuestionJsonError> {
        serde_json::to_vec(self).map_err(|error| PleQuestionJsonError::Encoding(error.to_string()))
    }

    /// Returns lowercase SHA-256 of [`Self::canonical_bytes`].
    ///
    /// # Errors
    ///
    /// Propagates canonical encoding failure.
    pub fn canonical_sha256(&self) -> Result<String, PleQuestionJsonError> {
        Ok(sha256_hex(&self.canonical_bytes()?))
    }

    /// Compiles the source into separate public and private representations.
    ///
    /// # Errors
    ///
    /// Refuses invalid content before constructing either persistence value.
    pub fn compile(&self) -> Result<CompiledPleQuestionJson, PleQuestionJsonError> {
        self.0.compile()
    }

    /// Returns a publication-only HOTSPOT source with the exact Question Asset
    /// Reference substituted for the private workspace image reference.
    ///
    /// The returned source remains answer-bearing and canonical, so the
    /// published Source Object Reference, PLE Question JSON public content, and server-only key can
    /// all be derived from one immutable version-scoped source document.
    pub fn with_hotspot_surface_asset(
        &self,
        question_asset: question_model::QuestionAssetReference,
    ) -> Result<Self, PleQuestionJsonError> {
        Ok(Self(self.0.with_hotspot_surface_asset(question_asset)?))
    }

    fn validate(&self) -> Result<(), PleQuestionJsonError> {
        self.0.validate()
    }
}

fn markdown_blocks(markdown: &str) -> Vec<QuestionContentBlock> {
    vec![QuestionContentBlock::Text {
        markdown: markdown.to_string(),
    }]
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut value, "{byte:02x}").expect("writing to a String cannot fail");
    }
    value
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

fn validate_markdown(
    name: &str,
    value: &str,
    maximum_chars: usize,
) -> Result<(), PleQuestionJsonError> {
    validate_bounded_text(name, value, maximum_chars)
}

fn validate_optional_feedback(value: Option<&str>) -> Result<(), PleQuestionJsonError> {
    if let Some(value) = value {
        validate_bounded_text("feedback", value, MAX_FEEDBACK_CHARS)?;
    }
    Ok(())
}

fn validate_optional_hint(value: Option<&str>) -> Result<(), PleQuestionJsonError> {
    if let Some(value) = value {
        validate_bounded_text("Question Hint", value, MAX_HINT_CHARS)?;
    }
    Ok(())
}

fn validate_metadata_text(name: &str, value: &str) -> Result<(), PleQuestionJsonError> {
    validate_bounded_text(name, value, MAX_METADATA_TEXT_CHARS)
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

fn invalid<T>(message: &str) -> Result<T, PleQuestionJsonError> {
    Err(PleQuestionJsonError::InvalidDocument(message.to_string()))
}

#[cfg(test)]
mod tests;
