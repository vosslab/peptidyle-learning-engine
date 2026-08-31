//! Strict JSON source and compiler for static flat questions.
//!
//! The closed version 2 family set follows the reviewed QTI Package Maker item model.
//! Parsing produces two values:
//! a browser-safe draft and answer-bearing private material. The latter stays
//! in this server-only adapter crate and is bound by checksum to the public
//! definition it grades.

use std::fmt::Write as _;

use crate::generator::NativeQuestionFamily;
use grading::AnswerKey;
pub use grading::flat_question::{
    FLAT_FILL_IN_FAMILY, FLAT_HOTSPOT_FAMILY, FLAT_MATCHING_FAMILY, FLAT_MULTI_FILL_IN_FAMILY,
    FLAT_MULTIPLE_ANSWER_FAMILY, FLAT_NUMERIC_FAMILY, FLAT_ORDERING_FAMILY,
    FLAT_SINGLE_CHOICE_V2_FAMILY, FlatQuestionError, FlatQuestionEvaluation, FlatQuestionPrivate,
    is_flat_question_family, validate_flat_question_question, validate_for_draft,
};
use question_model::envelope::ContentBlock;
use question_model::assignment_activity_rules::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::{License, TaxonomyTerm};
use question_model::{
    DraftQuestionDefinition, QuestionDefinition, WorkspaceId,
    capability::{BackendCapabilities, Capability},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Trusted QTI-profile mapping bridge for canonical flat-question source.
pub mod imported;
mod v2;

/// Canonical media type for canonicalized flat-question source payloads.
pub const FLAT_QUESTION_MEDIA_TYPE: &str = "application/vnd.peptidyle.flat-question+json";

/// Maximum accepted source size, matching the plan's problem-payload backstop.
pub const MAX_FLAT_QUESTION_BYTES: usize = grading::flat_question::MAX_FLAT_QUESTION_BYTES;

const FORMAT_NAME: &str = "pleFlatQuestion";
const MAX_CHOICES: usize = 100;
const MAX_CHOICE_ID_BYTES: usize = 64;
const MAX_PROMPT_CHARS: usize = 65_536;
const MAX_CHOICE_TEXT_CHARS: usize = 16_384;
const MAX_FEEDBACK_CHARS: usize = 16_384;
const MAX_TAG_CHARS: usize = 128;
const MAX_METADATA_TEXT_CHARS: usize = 256;

/// Answer-bearing authoring document decoded from PLE flat-question JSON.
///
/// The type intentionally has no `Debug` implementation because it contains
/// the correct choice and private teaching feedback. Use [`Self::compile`] to
/// split it before persistence or delivery.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FlatQuestionDocument(v2::FlatQuestionV2);

/// Closed authoring form of the shared attempt policy.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatAttemptPolicy {
    max_attempts: Option<u32>,
}

impl From<FlatAttemptPolicy> for AttemptPolicy {
    fn from(value: FlatAttemptPolicy) -> Self {
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
enum FlatTimingPolicy {
    Untimed,
    PerQuestion { seconds: u32, grace_seconds: u32 },
    PerAttempt { seconds: u32, grace_seconds: u32 },
}

impl From<FlatTimingPolicy> for TimingPolicy {
    fn from(value: FlatTimingPolicy) -> Self {
        match value {
            FlatTimingPolicy::Untimed => Self::Untimed,
            FlatTimingPolicy::PerQuestion {
                seconds,
                grace_seconds,
            } => Self::PerQuestion {
                seconds,
                grace_seconds,
            },
            FlatTimingPolicy::PerAttempt {
                seconds,
                grace_seconds,
            } => Self::PerAttempt {
                seconds,
                grace_seconds,
            },
        }
    }
}

/// Closed authoring form of one controlled-vocabulary term.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatTaxonomyTerm {
    scheme: String,
    code: String,
    label: String,
}

impl From<&FlatTaxonomyTerm> for TaxonomyTerm {
    fn from(value: &FlatTaxonomyTerm) -> Self {
        Self {
            scheme: value.scheme.clone(),
            code: value.code.clone(),
            label: value.label.clone(),
        }
    }
}

/// Closed authoring form of the shared license contract.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum FlatLicense {
    AllRightsReserved,
    CcBy,
    CcBySa,
    CcByNc,
    Cc0,
    Other { spdx: String },
}

impl From<&FlatLicense> for License {
    fn from(value: &FlatLicense) -> Self {
        match value {
            FlatLicense::AllRightsReserved => Self::AllRightsReserved,
            FlatLicense::CcBy => Self::CcBy,
            FlatLicense::CcBySa => Self::CcBySa,
            FlatLicense::CcByNc => Self::CcByNc,
            FlatLicense::Cc0 => Self::Cc0,
            FlatLicense::Other { spdx } => Self::Other { spdx: spdx.clone() },
        }
    }
}

/// One student-visible choice and its optional private teaching feedback.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatChoice {
    id: String,
    text: String,
    #[serde(default)]
    feedback: Option<String>,
}

/// Feedback selected from the final correctness outcome.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatOutcomeFeedback {
    #[serde(default)]
    correct: Option<String>,
    #[serde(default)]
    incorrect: Option<String>,
}

/// Public draft plus its separately persisted server-only grading material.
pub struct CompiledFlatQuestion {
    draft: DraftQuestionDefinition,
    private: FlatQuestionPrivate,
}

impl CompiledFlatQuestion {
    /// Returns the answer-free canonical draft used by catalog publication.
    pub fn draft(&self) -> &DraftQuestionDefinition {
        &self.draft
    }

    /// Returns the private material accepted only by a grading capability.
    pub fn private(&self) -> &FlatQuestionPrivate {
        &self.private
    }

    /// Transfers the split values to their separate persistence owners.
    pub fn into_parts(self) -> (DraftQuestionDefinition, FlatQuestionPrivate) {
        (self.draft, self.private)
    }
}

/// One registered static version 2 family.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FlatV2Family(&'static str);

impl NativeQuestionFamily for FlatV2Family {
    fn family(&self) -> &'static str {
        self.0
    }

    fn generator(&self) -> Option<question_model::GeneratorReference> {
        None
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::from_iter([
            Capability::ClientRendering,
            Capability::ServerGrading,
            Capability::Hints,
            Capability::PerQuestionTiming,
        ])
    }

    fn derive_answer_key(
        &self,
        question: &QuestionDefinition,
        _generated: &domain::generator::GeneratedVariant,
    ) -> Result<Option<AnswerKey>, crate::NativeAdapterError> {
        let question_model::QuestionSource::Native { family } = &question.source else {
            return Err(crate::NativeAdapterError::InvalidFamilyDefinition {
                family: self.0.to_string(),
                message: "flat family requires a native source".to_string(),
            });
        };
        if family != self.0 {
            return Err(crate::NativeAdapterError::InvalidFamilyDefinition {
                family: self.0.to_string(),
                message: "flat family registry selection changed".to_string(),
            });
        }
        validate_flat_question_question(question).map_err(|error| {
            crate::NativeAdapterError::InvalidFamilyDefinition {
                family: self.0.to_string(),
                message: error.to_string(),
            }
        })?;
        Ok(None)
    }
}

pub(crate) const FLAT_V2_FAMILIES: [FlatV2Family; 8] = [
    FlatV2Family(FLAT_SINGLE_CHOICE_V2_FAMILY),
    FlatV2Family(FLAT_MULTIPLE_ANSWER_FAMILY),
    FlatV2Family(FLAT_FILL_IN_FAMILY),
    FlatV2Family(FLAT_MULTI_FILL_IN_FAMILY),
    FlatV2Family(FLAT_NUMERIC_FAMILY),
    FlatV2Family(FLAT_MATCHING_FAMILY),
    FlatV2Family(FLAT_ORDERING_FAMILY),
    FlatV2Family(FLAT_HOTSPOT_FAMILY),
];

impl FlatQuestionDocument {
    /// Parses and validates one complete answer-bearing JSON source.
    ///
    /// # Errors
    ///
    /// Refuses oversized input, malformed or duplicate members, unknown
    /// fields, unsupported versions, and invalid v2 content.
    pub fn parse(bytes: &[u8]) -> Result<Self, FlatQuestionError> {
        if bytes.len() > MAX_FLAT_QUESTION_BYTES {
            return Err(FlatQuestionError::TooLarge);
        }
        let document: Self = serde_json::from_slice(bytes)
            .map_err(|error| FlatQuestionError::MalformedJson(error.to_string()))?;
        document.validate()?;
        Ok(document)
    }

    /// Returns compact canonical bytes for checksumming and immutable storage.
    ///
    /// # Errors
    ///
    /// Returns an encoding error only if the already validated typed document
    /// cannot be represented as JSON.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, FlatQuestionError> {
        serde_json::to_vec(self).map_err(|error| FlatQuestionError::Encoding(error.to_string()))
    }

    /// Returns lowercase SHA-256 of [`Self::canonical_bytes`].
    ///
    /// # Errors
    ///
    /// Propagates canonical encoding failure.
    pub fn canonical_sha256(&self) -> Result<String, FlatQuestionError> {
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
    ) -> Result<CompiledFlatQuestion, FlatQuestionError> {
        self.0.compile(workspace)
    }

    /// Returns a publication-only HOTSPOT source with the exact catalog asset
    /// identity substituted for the private workspace image identity.
    ///
    /// The returned source remains answer-bearing and canonical, so the
    /// published source artifact, public definition, and server-only key can
    /// all be derived from one immutable version-scoped source document.
    pub fn with_hotspot_surface_asset(
        &self,
        asset: question_model::AssetId,
    ) -> Result<Self, FlatQuestionError> {
        Ok(Self(self.0.with_hotspot_surface_asset(asset)?))
    }

    fn validate(&self) -> Result<(), FlatQuestionError> {
        self.0.validate()
    }
}

fn markdown_blocks(markdown: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
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

fn validate_markdown(
    name: &str,
    value: &str,
    maximum_chars: usize,
) -> Result<(), FlatQuestionError> {
    validate_bounded_text(name, value, maximum_chars)
}

fn validate_optional_feedback(value: Option<&str>) -> Result<(), FlatQuestionError> {
    if let Some(value) = value {
        validate_bounded_text("feedback", value, MAX_FEEDBACK_CHARS)?;
    }
    Ok(())
}

fn validate_metadata_text(name: &str, value: &str) -> Result<(), FlatQuestionError> {
    validate_bounded_text(name, value, MAX_METADATA_TEXT_CHARS)
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

fn invalid<T>(message: &str) -> Result<T, FlatQuestionError> {
    Err(FlatQuestionError::InvalidDocument(message.to_string()))
}

#[cfg(test)]
mod tests;
