//! MOD-ADP-PLE: the first-party algorithmic Question Backend adapter.
//!
//! The engine is question agnostic: [`PleQuestionBackend`] dispatches by the
//! versioned [`generator::PleQuestionImplementation`] contract, while Question
//! Implementations own parameter-to-prompt construction and server-only key
//! derivation. The stable facade coordinates focused registry, issue, grading,
//! reproduction, and capability owners.

use std::collections::BTreeMap;
use std::sync::Arc;

use domain::draft_preview::PresentationError;
use domain::generator::{GenerationError, QuestionVariationParameters};
use grading::GradingError;
use question_model::envelope::{QuestionContentBlock, QuestionVariationPresentation};
use question_model::{
    ObjectId, QuestionAssetId, QuestionAttemptReproductionDetails, QuestionBackendVersion,
    QuestionGraderVersion, QuestionTitleError,
};

use crate::generator::PleQuestionImplementation;

#[path = "lib/capabilities.rs"]
mod capabilities;
#[path = "lib/grade.rs"]
mod grade;
#[path = "lib/issue.rs"]
mod issue;
#[path = "lib/question_json_source.rs"]
mod question_json_source;
#[path = "lib/registry.rs"]
mod registry;
#[path = "lib/reproduction.rs"]
mod reproduction;
#[path = "lib/source_implementation.rs"]
mod source_implementation;

pub use question_json_source::ResolvedPleQuestionJsonSource;
use registry::{PleQuestionExecution, PleQuestionImplementationKey};

#[cfg(test)]
use grading::QuestionGradingOutcome;
#[cfg(test)]
use registry::{backend_version, grader_version};
#[cfg(test)]
#[path = "lib/question_json_source_tests.rs"]
mod question_json_source_tests;
#[cfg(test)]
mod test_support;

/// Extensible Question Implementation contract and server-only derived Question data.
pub mod generator;
/// Strict, versioned PLE Question JSON source for first-party static Questions.
pub mod question_json;

/// Stable Question Backend identifier persisted with every PLE Question Attempt.
pub const ADAPTER_ID: &str = "ple-question-backend";
/// Current Question Backend Version for issuance and replay.
///
/// This is distinct from the repository CalVer release and from a Question Revision.
pub const ADAPTER_VERSION: &str = "1";
/// Stable Question Grader identifier persisted with every PLE Question Attempt.
pub const GRADING_ID: &str = "generic-grader";
/// Current Question Grader Version for issuance and replay.
pub const GRADING_VERSION: &str = "1";

/// One trusted, server-side relationship between a Question Asset and immutable Object Reference.
///
/// The browser never constructs this input. A server storage adapter resolves
/// immutable asset records for the published question revision before calling
/// PLE issue, replay, or grading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionAssetObjectReference {
    /// Logical identifier embedded in a renderable content block.
    pub question_asset: QuestionAssetId,
    /// Immutable object-store record selected by the trusted storage layer.
    pub object_reference: ObjectId,
}

/// Key-free PLE Issued Question returned at Question Attempt issue time.
#[derive(Debug, Clone, PartialEq)]
pub struct PleIssuedQuestion {
    /// Generated prompt and response shape safe to deliver to the browser.
    pub envelope: QuestionVariationPresentation,
    /// SHA-256 of the canonical generated parameter map.
    pub parameter_hash: String,
    /// Versions and object identities needed to reproduce the attempt.
    pub reproduction_details: QuestionAttemptReproductionDetails,
}

/// Server-only author presentation for one PLE Draft Question seed.
///
/// Its fields are already-rendered student-facing blocks. It deliberately
/// excludes answer keys, choice IDs, grading rules, source locators, and
/// published identity.
#[derive(Clone, PartialEq)]
pub struct PleDraftAuthorPresentation {
    /// Student-facing title.
    pub title: String,
    /// Constructed student-facing prompt for the generated Question Variation.
    pub prompt: Vec<QuestionContentBlock>,
    /// Browser-safe response shape.
    pub response: question_model::QuestionResponseFormat,
    /// Display-ready accepted response for the exact generated variation.
    pub question_answer: Vec<QuestionContentBlock>,
    /// Optional display-ready explanation of how or why the answer is reached.
    pub question_answer_explanation: Option<Vec<QuestionContentBlock>>,
}

/// Versioned PLE Question Implementation registry and orchestration boundary.
pub struct PleQuestionBackend {
    implementations: BTreeMap<PleQuestionImplementationKey, Arc<dyn PleQuestionImplementation>>,
    backend_versions: BTreeMap<(String, String), PleQuestionExecution>,
    grader_versions: BTreeMap<(String, String), PleQuestionExecution>,
    current_backend: QuestionBackendVersion,
    current_grader: QuestionGraderVersion,
}

struct PreparedPleQuestion {
    generated: QuestionVariationParameters,
    derived: DerivedPleQuestion,
    envelope: QuestionVariationPresentation,
    parameter_hash: String,
    rendered_question_sha256: String,
}

struct DerivedPleQuestion {
    prompt: Vec<QuestionContentBlock>,
    answer_key: Option<grading::AnswerKey>,
}

/// Explicit ple-question-backend failure without database, HTTP, or SDK types.
#[derive(Debug)]
pub enum PleQuestionBackendError {
    /// A caller selected a non-PLE Question source.
    UnsupportedSource,
    /// No installed implementation matches the Question's explicit contract.
    UnknownQuestionImplementation {
        question_format: question_model::QuestionFormat,
        question_type: question_model::QuestionType,
        generator: Option<question_model::QuestionGeneratorReference>,
    },
    /// Two implementations attempted to own one exact Question contract.
    DuplicateQuestionImplementation {
        question_format: question_model::QuestionFormat,
        question_type: question_model::QuestionType,
        generator: Option<question_model::QuestionGeneratorReference>,
    },
    /// A persisted Question Backend Version has no compiled implementation.
    UnknownQuestionBackendVersion { version: QuestionBackendVersion },
    /// A persisted Question Grader Version has no compiled implementation.
    UnknownQuestionGraderVersion { version: QuestionGraderVersion },
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
    MissingAssetBinding(QuestionAssetId),
    /// A trusted binding was supplied for an asset the envelope does not render.
    UnrelatedAssetBinding(QuestionAssetId),
    /// A binding list assigned one logical asset more than once.
    ConflictingAssetBinding(QuestionAssetId),
    /// Immutable Question Source bytes could not be resolved from object storage.
    QuestionSourceResolution(objects::QuestionSourceResolutionError),
    /// PLE Question JSON source bytes did not use the canonical media type.
    UnexpectedQuestionSourceMediaType { media_type: String },
    /// PLE Question JSON source bytes did not compile into their Question Revision.
    QuestionSourceDoesNotMatchQuestion,
    /// PLE Question JSON source bytes were malformed or invalid.
    QuestionSourceDocument(question_json::PleQuestionJsonError),
    /// The server-only generic grader refused the response or definition.
    Grading(GradingError),
}

impl std::fmt::Display for PleQuestionBackendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSource => formatter.write_str("question source is not PLE"),
            Self::UnknownQuestionImplementation {
                question_format,
                question_type,
                generator,
            } => write!(
                formatter,
                "PLE Question Implementation is not installed for {question_format:?}/{question_type:?}/{generator:?}",
            ),
            Self::DuplicateQuestionImplementation {
                question_format,
                question_type,
                generator,
            } => write!(
                formatter,
                "PLE Question Implementation is registered twice for {question_format:?}/{question_type:?}/{generator:?}"
            ),
            Self::UnknownQuestionBackendVersion { version } => write!(
                formatter,
                "PLE Question Backend Version is not installed: {}@{}",
                version.name, version.version
            ),
            Self::UnknownQuestionGraderVersion { version } => write!(
                formatter,
                "PLE Question Grader Version is not installed: {}@{}",
                version.name, version.version
            ),
            Self::IncompatibleQuestionImplementation { message } => {
                write!(
                    formatter,
                    "PLE Question Implementation rejected the definition: {message}"
                )
            }
            Self::InvalidTitle(error) => {
                write!(formatter, "invalid PLE Question title: {error}")
            }
            Self::Generation(error) => write!(formatter, "PLE generation failed: {error}"),
            Self::Presentation(error) => {
                write!(formatter, "PLE prompt presentation failed: {error}")
            }
            Self::Serialization(message) => {
                write!(
                    formatter,
                    "PLE Question Presentation could not be hashed: {message}"
                )
            }
            Self::ReproductionMismatch { field } => {
                write!(
                    formatter,
                    "PLE Question Attempt does not reproduce field {field}"
                )
            }
            Self::MissingAssetBinding(asset) => {
                write!(formatter, "PLE trusted binding missing for asset: {asset}")
            }
            Self::UnrelatedAssetBinding(asset) => write!(
                formatter,
                "PLE trusted binding is unrelated to Question Presentation asset: {asset}"
            ),
            Self::ConflictingAssetBinding(asset) => write!(
                formatter,
                "PLE trusted bindings conflict for asset: {asset}"
            ),
            Self::QuestionSourceResolution(error) => error.fmt(formatter),
            Self::UnexpectedQuestionSourceMediaType { media_type } => write!(
                formatter,
                "PLE Question Source has media type {media_type:?} instead of the canonical PLE Question JSON media type"
            ),
            Self::QuestionSourceDoesNotMatchQuestion => formatter
                .write_str("PLE Question JSON source does not compile into its Question Revision"),
            Self::QuestionSourceDocument(error) => error.fmt(formatter),
            Self::Grading(error) => write!(formatter, "PLE grading failed: {error}"),
        }
    }
}

impl std::error::Error for PleQuestionBackendError {}

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "lib/source_evidence_tests.rs"]
mod source_evidence_tests;
