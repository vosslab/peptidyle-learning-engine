//! MOD-ADP-NAT: the first-party algorithmic question adapter.
//!
//! The engine is question agnostic: [`NativeAdapter`] dispatches by the
//! versioned [`generator::NativeQuestionImplementation`] contract, while Question
//! Implementations own parameter-to-prompt materialization and server-only key
//! derivation. The stable facade coordinates focused registry, issue, grading,
//! reproduction, and capability owners.

use std::collections::BTreeMap;
use std::sync::Arc;

use domain::draft_preview::PresentationError;
use domain::generator::{GeneratedVariant, GenerationError};
use grading::GradingError;
use question_model::envelope::{ContentBlock, QuestionPresentation};
use question_model::{
    AssetId, ImplementationVersion, ObjectId, QuestionAttemptSourceRecord, QuestionTitleError,
};

use crate::generator::NativeQuestionImplementation;

#[path = "lib/capabilities.rs"]
mod capabilities;
#[path = "lib/grade.rs"]
mod grade;
#[path = "lib/issue.rs"]
mod issue;
#[path = "lib/registry.rs"]
mod registry;
#[path = "lib/reproduction.rs"]
mod reproduction;
#[path = "lib/source_implementation.rs"]
mod source_implementation;

use registry::{
    ImplementationRegistrationKey, NativeExecution, NativeQuestionImplementationRegistrationKey,
};

#[cfg(test)]
use grading::QuestionGradingOutcome;
#[cfg(test)]
use question_model::QuestionDefinition;
#[cfg(test)]
use question_model::generation::Seed;
#[cfg(test)]
use registry::implementation_version;
#[cfg(test)]
mod test_support;

/// Strict, versioned JSON source for first-party static flat questions.
pub mod flat_question;
/// Extensible Question Implementation contract and server-only materialization result.
pub mod generator;
/// Reference implementation proving generation, rendering, and server grading end to end.
pub mod peptide_bond_geometry;

/// Stable adapter implementation identifier persisted with every native attempt.
pub const ADAPTER_ID: &str = "native-adapter";
/// Current adapter implementation version for issuance and replay.
///
/// This is not the repository CalVer release or a question version.
pub const ADAPTER_VERSION: &str = "1";
/// Stable generic grader identifier persisted with every native attempt.
pub const GRADING_ID: &str = "generic-grader";
/// Current generic-grader implementation version for issuance and replay.
pub const GRADING_VERSION: &str = "1";

/// One trusted, server-side binding from an authored logical asset to its
/// immutable object-store record.
///
/// The browser never constructs this input. A server storage adapter resolves
/// immutable asset records for the published question version before calling
/// native issue, replay, or grading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetObjectBinding {
    /// Logical identifier embedded in a renderable content block.
    pub asset: AssetId,
    /// Immutable object-store record selected by the trusted storage layer.
    pub object: ObjectId,
}

/// Key-free native Issued Question returned at Question Attempt issue time.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeIssuedAttempt {
    /// Generated prompt and response shape safe to deliver to the browser.
    pub envelope: QuestionPresentation,
    /// SHA-256 of the canonical generated parameter map.
    pub parameter_hash: String,
    /// Versions and object identities needed to reproduce the attempt.
    pub source_record: QuestionAttemptSourceRecord,
}

/// Server-only author presentation for one native draft seed.
///
/// Its fields are already-rendered student-facing blocks. It deliberately
/// excludes answer keys, choice IDs, grading rules, source locators, and
/// published identity.
#[derive(Clone, PartialEq)]
pub struct NativeDraftAuthorPresentation {
    /// Student-facing title.
    pub title: String,
    /// Materialized student-facing prompt.
    pub prompt: Vec<ContentBlock>,
    /// Browser-safe response shape.
    pub response: question_model::QuestionResponseFormat,
    /// Rendered explanation of the correct response.
    pub correct_response: Vec<ContentBlock>,
    /// Optional teaching rationale.
    pub rationale: Option<Vec<ContentBlock>>,
}

/// Versioned native Question Implementation registry and orchestration boundary.
pub struct NativeAdapter {
    implementations: BTreeMap<
        NativeQuestionImplementationRegistrationKey,
        Arc<dyn NativeQuestionImplementation>,
    >,
    adapter_implementations: BTreeMap<ImplementationRegistrationKey, NativeExecution>,
    grading_implementations: BTreeMap<ImplementationRegistrationKey, NativeExecution>,
    current_adapter: ImplementationVersion,
    current_grading: ImplementationVersion,
}

struct PreparedNativeQuestion {
    generated: GeneratedVariant,
    materialized: MaterializedNativeQuestion,
    envelope: QuestionPresentation,
    parameter_hash: String,
    rendered_question_sha256: String,
}

struct MaterializedNativeQuestion {
    prompt: Vec<ContentBlock>,
    answer_key: Option<grading::AnswerKey>,
}

/// Explicit native-adapter failure without database, HTTP, or SDK types.
#[derive(Debug)]
pub enum NativeAdapterError {
    /// A caller selected a non-native question source.
    UnsupportedSource,
    /// No installed implementation matches the Question's explicit contract.
    UnknownQuestionImplementation {
        question_format: question_model::QuestionFormat,
        question_type: question_model::QuestionType,
        generator: Option<question_model::GeneratorReference>,
    },
    /// Two implementations attempted to own one exact Question contract.
    DuplicateQuestionImplementation {
        question_format: question_model::QuestionFormat,
        question_type: question_model::QuestionType,
        generator: Option<question_model::GeneratorReference>,
        implementation: ImplementationVersion,
    },
    /// A persisted adapter or grader version has no compiled implementation.
    UnknownImplementation {
        field: &'static str,
        version: ImplementationVersion,
    },
    /// The authored definition does not meet its implementation's contract.
    IncompatibleQuestionImplementation { message: String },
    /// Persisted student-facing metadata cannot be delivered safely.
    InvalidTitle(QuestionTitleError),
    /// Shared deterministic parameter generation failed.
    Generation(GenerationError),
    /// Shared key-free prompt presentation was invalid.
    Presentation(PresentationError),
    /// A browser-safe envelope could not be serialized for hashing.
    Serialization(String),
    /// Stored attempt metadata disagreed with exact regeneration.
    ReproductionMismatch { field: &'static str },
    /// A renderable asset was not bound by the trusted storage layer.
    MissingAssetBinding(AssetId),
    /// A trusted binding was supplied for an asset the envelope does not render.
    UnrelatedAssetBinding(AssetId),
    /// A binding list assigned one logical asset more than once.
    ConflictingAssetBinding(AssetId),
    /// The server-only generic grader refused the response or definition.
    Grading(GradingError),
}

impl std::fmt::Display for NativeAdapterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSource => formatter.write_str("question source is not native"),
            Self::UnknownQuestionImplementation {
                question_format,
                question_type,
                generator,
            } => write!(
                formatter,
                "native Question Implementation is not installed for {question_format:?}/{question_type:?}/{generator:?}",
            ),
            Self::DuplicateQuestionImplementation {
                question_format,
                question_type,
                generator,
                implementation,
            } => write!(
                formatter,
                "native Question Implementation is registered twice for {question_format:?}/{question_type:?}/{generator:?}/{}@{}",
                implementation.id, implementation.version
            ),
            Self::UnknownImplementation { field, version } => write!(
                formatter,
                "native {field} implementation is not installed: {}@{}",
                version.id, version.version
            ),
            Self::IncompatibleQuestionImplementation { message } => {
                write!(
                    formatter,
                    "native Question Implementation rejected the definition: {message}"
                )
            }
            Self::InvalidTitle(error) => {
                write!(formatter, "invalid native question title: {error}")
            }
            Self::Generation(error) => write!(formatter, "native generation failed: {error}"),
            Self::Presentation(error) => {
                write!(formatter, "native prompt presentation failed: {error}")
            }
            Self::Serialization(message) => {
                write!(formatter, "native envelope could not be hashed: {message}")
            }
            Self::ReproductionMismatch { field } => {
                write!(formatter, "native attempt does not reproduce field {field}")
            }
            Self::MissingAssetBinding(asset) => write!(
                formatter,
                "native trusted binding missing for asset: {asset}"
            ),
            Self::UnrelatedAssetBinding(asset) => write!(
                formatter,
                "native trusted binding is unrelated to envelope asset: {asset}"
            ),
            Self::ConflictingAssetBinding(asset) => write!(
                formatter,
                "native trusted bindings conflict for asset: {asset}"
            ),
            Self::Grading(error) => write!(formatter, "native grading failed: {error}"),
        }
    }
}

impl std::error::Error for NativeAdapterError {}

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;
