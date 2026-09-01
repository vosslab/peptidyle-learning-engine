//! Strict PLE Question JSON source and compiler for static Questions.
//!
//! The closed version 2 Question Type set follows the reviewed QTI Package Maker item model.
//! Parsing produces two values:
//! a browser-safe draft and answer-bearing private material. The latter stays
//! in this server-only adapter crate and is bound by checksum to the public
//! definition it grades.

use std::fmt::Write as _;

use crate::generator::{PleQuestionImplementation, PleQuestionImplementationRelease};
use grading::AnswerKey;
pub use grading::ple_question_json::{
    PleQuestionJsonError, PleQuestionJsonEvaluation, PleQuestionJsonPrivateGrading,
    validate_for_draft, validate_ple_question_json_question,
};
use question_model::assignment_activity_rules::{QuestionAttemptLimit, QuestionAttemptTimeLimit};
use question_model::classification::QuestionClassification;
use question_model::envelope::QuestionContentBlock;
use question_model::{
    DraftQuestionContent, QuestionFormat, QuestionHint, QuestionRevision, QuestionType,
    WorkspaceId,
    capability::{Capability, QuestionBackendCapabilities},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Trusted QTI-profile mapping bridge for canonical PLE Question JSON source.
pub mod imported;
mod schema_v2;

/// Canonical media type for canonical PLE Question JSON source payloads.
pub const PLE_QUESTION_JSON_MEDIA_TYPE: &str = "application/vnd.peptidyle.question+json";

/// Maximum accepted source size, matching the plan's problem-payload backstop.
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
pub struct PleQuestionJsonDocument(schema_v2::PleQuestionJsonDocumentBody);

/// Closed authoring form of the shared Question Attempt Limit.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonAttemptLimit {
    max_attempts: Option<u32>,
}

impl From<PleQuestionJsonAttemptLimit> for QuestionAttemptLimit {
    fn from(value: PleQuestionJsonAttemptLimit) -> Self {
        Self {
            max_attempts: value.max_attempts,
        }
    }
}

/// Closed authoring form of the shared timing policy.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum PleQuestionJsonAttemptTimeLimit {
    Unlimited,
    Limited { seconds: u32, grace_seconds: u32 },
}

impl From<PleQuestionJsonAttemptTimeLimit> for QuestionAttemptTimeLimit {
    fn from(value: PleQuestionJsonAttemptTimeLimit) -> Self {
        match value {
            PleQuestionJsonAttemptTimeLimit::Unlimited => Self::Unlimited,
            PleQuestionJsonAttemptTimeLimit::Limited {
                seconds,
                grace_seconds,
            } => Self::Limited {
                seconds,
                grace_seconds,
            },
        }
    }
}

/// Closed authoring form of one exact Question Classification.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonClassification {
    system: String,
    code: String,
    name: String,
}

impl From<&PleQuestionJsonClassification> for QuestionClassification {
    fn from(value: &PleQuestionJsonClassification) -> Self {
        Self {
            system: value.system.clone(),
            code: value.code.clone(),
            name: value.name.clone(),
        }
    }
}

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
pub struct CompiledPleQuestionJson {
    draft: DraftQuestionContent,
    private: PleQuestionJsonPrivateGrading,
    question_hint: Option<QuestionHint>,
}

impl CompiledPleQuestionJson {
    /// Returns the answer-free canonical draft used by Question Library publication.
    pub fn draft(&self) -> &DraftQuestionContent {
        &self.draft
    }

    /// Returns the private material accepted only by a grading capability.
    pub fn private(&self) -> &PleQuestionJsonPrivateGrading {
        &self.private
    }

    /// Returns the optional pre-response Question Hint for its separate private owner.
    pub fn question_hint(&self) -> Option<&QuestionHint> {
        self.question_hint.as_ref()
    }

    /// Transfers the split values to their separate persistence owners.
    pub fn into_parts(
        self,
    ) -> (
        DraftQuestionContent,
        PleQuestionJsonPrivateGrading,
        Option<QuestionHint>,
    ) {
        (self.draft, self.private, self.question_hint)
    }
}

/// One registered static version 2 Question Implementation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PleQuestionJsonImplementation(QuestionType);

impl PleQuestionImplementation for PleQuestionJsonImplementation {
    fn question_format(&self) -> QuestionFormat {
        QuestionFormat::PleQuestionJson
    }

    fn question_type(&self) -> QuestionType {
        self.0
    }

    fn implementation_release(&self) -> PleQuestionImplementationRelease {
        PleQuestionImplementationRelease {
            id: "ple-question-json".to_string(),
            version: "2".to_string(),
        }
    }

    fn generator(&self) -> Option<question_model::QuestionGeneratorReference> {
        None
    }

    fn capabilities(&self) -> QuestionBackendCapabilities {
        QuestionBackendCapabilities::from_iter([
            Capability::ClientRendering,
            Capability::ServerGrading,
            Capability::Hints,
            Capability::QuestionAttemptTimeLimit,
        ])
    }

    fn derive_answer_key(
        &self,
        question: &QuestionRevision,
        _generated: &domain::generator::QuestionVariationParameters,
    ) -> Result<Option<AnswerKey>, crate::PleQuestionBackendError> {
        if !matches!(
            question.backend_locator,
            question_model::QuestionBackendLocator::Ple
        ) || question.question_format != self.question_format()
            || question.question_type != self.question_type()
        {
            return Err(crate::PleQuestionBackendError::IncompatibleQuestionImplementation {
                message: "PLE Question JSON Implementation requires the matching PLE Question Format and Question Type".to_string(),
            });
        }
        validate_ple_question_json_question(question).map_err(|error| {
            crate::PleQuestionBackendError::IncompatibleQuestionImplementation {
                message: error.to_string(),
            }
        })?;
        Ok(None)
    }
}

pub(crate) const PLE_QUESTION_JSON_IMPLEMENTATIONS: [PleQuestionJsonImplementation; 8] = [
    PleQuestionJsonImplementation(QuestionType::MultipleChoice),
    PleQuestionJsonImplementation(QuestionType::MultipleAnswer),
    PleQuestionJsonImplementation(QuestionType::FillInBlank),
    PleQuestionJsonImplementation(QuestionType::MultipleFillInBlank),
    PleQuestionJsonImplementation(QuestionType::Numeric),
    PleQuestionJsonImplementation(QuestionType::Matching),
    PleQuestionJsonImplementation(QuestionType::Ordering),
    PleQuestionJsonImplementation(QuestionType::Hotspot),
];

impl PleQuestionJsonDocument {
    /// Parses and validates one complete answer-bearing JSON source.
    ///
    /// # Errors
    ///
    /// Refuses oversized input, malformed or duplicate members, unknown
    /// fields, unsupported versions, and invalid v2 content.
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
    pub fn compile(
        &self,
        workspace: WorkspaceId,
    ) -> Result<CompiledPleQuestionJson, PleQuestionJsonError> {
        self.0.compile(workspace)
    }

    /// Returns a publication-only HOTSPOT source with the exact Question Library asset
    /// identity substituted for the private workspace image identity.
    ///
    /// The returned source remains answer-bearing and canonical, so the
    /// published Source Object Reference, public definition, and server-only key can
    /// all be derived from one immutable version-scoped source document.
    pub fn with_hotspot_surface_asset(
        &self,
        asset: question_model::QuestionAssetId,
    ) -> Result<Self, PleQuestionJsonError> {
        Ok(Self(self.0.with_hotspot_surface_asset(asset)?))
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
