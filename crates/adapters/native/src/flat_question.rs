//! Strict JSON source and compiler for static flat questions.
//!
//! Version 1 remains the original static exactly-one multiple-choice contract.
//! Version 2 adds the closed family set based on the reviewed QTI Package Maker item model.
//! Parsing produces two values:
//! a browser-safe draft and answer-bearing private material. The latter stays
//! in this server-only adapter crate and is bound by checksum to the public
//! definition it grades.

use std::collections::HashSet;
use std::fmt::Write as _;

use crate::generator::NativeQuestionFamily;
use grading::AnswerKey;
pub use grading::flat_question::{
    FLAT_FILL_IN_FAMILY, FLAT_HOTSPOT_FAMILY, FLAT_MATCHING_FAMILY, FLAT_MULTI_FILL_IN_FAMILY,
    FLAT_MULTIPLE_ANSWER_FAMILY, FLAT_NUMERIC_FAMILY, FLAT_ORDERING_FAMILY,
    FLAT_SINGLE_CHOICE_FAMILY, FLAT_SINGLE_CHOICE_V2_FAMILY, FlatQuestionError,
    FlatQuestionEvaluation, FlatQuestionPrivate, is_flat_question_family,
    validate_flat_question_question, validate_flat_single_choice_draft,
    validate_flat_single_choice_question, validate_for_draft,
};
use question_model::answer::SelectionCardinality;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::{License, Tag, TaxonomyTerm};
use question_model::{
    DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, QuestionDefinition,
    QuestionMetadata, WorkspaceId,
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
const FORMAT_VERSION_V1: u32 = 1;
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
pub struct FlatQuestionDocument(FlatDocumentVersion);

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum FlatDocumentVersion {
    V1(FlatSingleChoiceV1),
    V2(v2::FlatQuestionV2),
}

/// Preserved answer-bearing version 1 single-choice document.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatSingleChoiceV1 {
    format: String,
    version: u32,
    kind: FlatQuestionKind,
    title: String,
    prompt: String,
    choices: Vec<FlatChoice>,
    correct_choice: String,
    #[serde(default)]
    feedback: FlatOutcomeFeedback,
    points: f64,
    attempt_policy: FlatAttemptPolicy,
    timing_policy: FlatTimingPolicy,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    taxonomy: Vec<FlatTaxonomyTerm>,
    license: FlatLicense,
    language: String,
}

/// Question shape supported by flat-question JSON v1.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FlatQuestionKind {
    SingleChoice,
}

/// Closed authoring form of the shared attempt policy.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatAttemptPolicy {
    max_attempts: Option<u32>,
    feedback: FeedbackDisclosure,
}

impl From<FlatAttemptPolicy> for AttemptPolicy {
    fn from(value: FlatAttemptPolicy) -> Self {
        Self {
            max_attempts: value.max_attempts,
            feedback: value.feedback,
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

/// One learner-visible choice and its optional private teaching feedback.
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

/// Built-in static multiple-choice family for canonical flat questions.
///
/// This family compiles a fixed public shape and delegates answer-bearing
/// grading material to `FlatQuestionPrivate`, persisted separately in a
/// server-only store.
#[derive(Debug, Clone, Copy, Default)]
pub struct FlatSingleChoiceFamily;

impl NativeQuestionFamily for FlatSingleChoiceFamily {
    fn family(&self) -> &'static str {
        FLAT_SINGLE_CHOICE_FAMILY
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
        validate_flat_question_shape(question)?;
        Ok(None)
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
    /// fields, unsupported versions, and invalid v1 content.
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
        match &self.0 {
            FlatDocumentVersion::V1(document) => document.compile(workspace),
            FlatDocumentVersion::V2(document) => document.compile(workspace),
        }
    }

    fn validate(&self) -> Result<(), FlatQuestionError> {
        match &self.0 {
            FlatDocumentVersion::V1(document) => document.validate(),
            FlatDocumentVersion::V2(document) => document.validate(),
        }
    }
}

impl FlatSingleChoiceV1 {
    fn compile(&self, workspace: WorkspaceId) -> Result<CompiledFlatQuestion, FlatQuestionError> {
        self.validate()?;
        let choices = self
            .choices
            .iter()
            .map(|choice| ChoiceOption {
                id: ChoiceId::new(&choice.id),
                body: markdown_blocks(&choice.text),
            })
            .collect();
        let draft = DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: FLAT_SINGLE_CHOICE_FAMILY.to_string(),
            },
            prompt: markdown_blocks(&self.prompt),
            response: ResponseDefinition::MultipleChoice {
                choices,
                selection: SelectionCardinality::ExactlyOne,
            },
            attempt_policy: self.attempt_policy.into(),
            timing_policy: self.timing_policy.into(),
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
        let choice_feedback = self
            .choices
            .iter()
            .filter_map(|choice| {
                choice
                    .feedback
                    .as_ref()
                    .map(|markdown| (ChoiceId::new(&choice.id), markdown.clone()))
            })
            .collect();
        let private = FlatQuestionPrivate::new(
            &draft,
            ChoiceId::new(&self.correct_choice),
            choice_feedback,
            self.feedback.correct.clone(),
            self.feedback.incorrect.clone(),
        )?;
        Ok(CompiledFlatQuestion { draft, private })
    }

    fn validate(&self) -> Result<(), FlatQuestionError> {
        if self.format != FORMAT_NAME {
            return Err(FlatQuestionError::UnsupportedFormat);
        }
        if self.version != FORMAT_VERSION_V1 {
            return Err(FlatQuestionError::UnsupportedVersion(self.version));
        }
        question_model::validate_question_title(&self.title)
            .map_err(FlatQuestionError::InvalidTitle)?;
        validate_markdown("prompt", &self.prompt, MAX_PROMPT_CHARS)?;
        if !(2..=MAX_CHOICES).contains(&self.choices.len()) {
            return invalid("single-choice questions require 2 to 100 choices");
        }
        let mut identifiers = HashSet::with_capacity(self.choices.len());
        for choice in &self.choices {
            validate_choice_id(&choice.id)?;
            if !identifiers.insert(choice.id.as_str()) {
                return invalid("choice identifiers must be unique");
            }
            validate_markdown("choice text", &choice.text, MAX_CHOICE_TEXT_CHARS)?;
            validate_optional_feedback(choice.feedback.as_deref())?;
        }
        if !identifiers.contains(self.correct_choice.as_str()) {
            return invalid("correctChoice must name an available choice");
        }
        validate_optional_feedback(self.feedback.correct.as_deref())?;
        validate_optional_feedback(self.feedback.incorrect.as_deref())?;
        if !self.points.is_finite() || self.points < 0.0 {
            return invalid("points must be finite and nonnegative");
        }
        if self.attempt_policy.max_attempts == Some(0) {
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
        Ok(())
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

fn validate_flat_question_shape(
    question: &QuestionDefinition,
) -> Result<(), crate::NativeAdapterError> {
    validate_flat_single_choice_question(question).map_err(|error| {
        crate::NativeAdapterError::InvalidFamilyDefinition {
            family: FLAT_SINGLE_CHOICE_FAMILY.to_string(),
            message: error.to_string(),
        }
    })
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
