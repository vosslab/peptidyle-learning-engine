//! MOD-ADP-NAT: the first-party algorithmic question adapter.
//!
//! The engine is question agnostic: [`NativeAdapter`] dispatches by the
//! versioned [`generator::NativeQuestionFamily`] contract, while question
//! families own parameter-to-prompt materialization and server-only key
//! derivation. The stable facade coordinates focused registry, issue, grading,
//! reproduction, and capability owners.

use std::collections::BTreeMap;
use std::sync::Arc;

use domain::draft_preview::PresentationError;
use domain::generator::{GeneratedVariant, GenerationError};
use grading::GradingError;
use question_model::envelope::{ContentBlock, QuestionEnvelope};
use question_model::generation::GeneratorReference;
use question_model::{
    AssetId, AttemptProvenance, ImplementationVersion, ObjectId, QuestionTitleError,
};

use crate::generator::NativeQuestionFamily;

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
#[path = "lib/source_family.rs"]
mod source_family;

use registry::{FamilyRegistrationKey, ImplementationRegistrationKey, NativeExecution};

#[cfg(test)]
use grading::GradeOutcome;
#[cfg(test)]
use question_model::QuestionDefinition;
#[cfg(test)]
use question_model::generation::Seed;
#[cfg(test)]
use registry::implementation_version;

/// Strict, versioned JSON source for first-party static flat questions.
pub mod flat_question;
/// Extensible question-family contract and server-only materialization result.
pub mod generator;
/// Reference family proving generation, rendering, and server grading end to end.
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

/// Key-free native question instance returned at attempt issue time.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeIssuedAttempt {
    /// Generated prompt and response shape safe to deliver to the browser.
    pub envelope: QuestionEnvelope,
    /// SHA-256 of the canonical generated parameter map.
    pub parameter_hash: String,
    /// Versions and object identities needed to reproduce the attempt.
    pub provenance: AttemptProvenance,
}

/// Server-only author presentation for one native draft seed.
///
/// Its fields are already-rendered learner-facing blocks. It deliberately
/// excludes answer keys, choice IDs, grading rules, source locators, and
/// published identity.
#[derive(Clone, PartialEq)]
pub struct NativeDraftAuthorPresentation {
    /// Learner-facing title.
    pub title: String,
    /// Materialized learner-facing prompt.
    pub prompt: Vec<ContentBlock>,
    /// Browser-safe response shape.
    pub response: question_model::ResponseDefinition,
    /// Rendered explanation of the correct response.
    pub correct_response: Vec<ContentBlock>,
    /// Optional teaching rationale.
    pub rationale: Option<Vec<ContentBlock>>,
}

/// Versioned native-family registry and orchestration boundary.
pub struct NativeAdapter {
    families: BTreeMap<FamilyRegistrationKey, Arc<dyn NativeQuestionFamily>>,
    adapter_implementations: BTreeMap<ImplementationRegistrationKey, NativeExecution>,
    grading_implementations: BTreeMap<ImplementationRegistrationKey, NativeExecution>,
    current_adapter: ImplementationVersion,
    current_grading: ImplementationVersion,
}

struct PreparedNativeQuestion {
    generated: GeneratedVariant,
    materialized: MaterializedNativeQuestion,
    envelope: QuestionEnvelope,
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
    /// No installed family owns the source identifier.
    UnknownFamily(String),
    /// Two implementations attempted to own one family identifier.
    DuplicateFamily(String),
    /// The source family exists, but its pinned generator is not installed.
    UnknownGenerator {
        family: String,
        generator: Option<GeneratorReference>,
    },
    /// A persisted adapter or grader version has no compiled implementation.
    UnknownImplementation {
        field: &'static str,
        version: ImplementationVersion,
    },
    /// The authored definition does not meet its family's versioned contract.
    InvalidFamilyDefinition { family: String, message: String },
    /// Persisted learner-facing metadata cannot be delivered safely.
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
            Self::UnknownFamily(family) => write!(
                formatter,
                "native question family is not installed: {family}"
            ),
            Self::DuplicateFamily(family) => write!(
                formatter,
                "native question family is registered twice: {family}"
            ),
            Self::UnknownGenerator { family, generator } => write!(
                formatter,
                "native family {family} has no installed implementation for generator {generator:?}"
            ),
            Self::UnknownImplementation { field, version } => write!(
                formatter,
                "native {field} implementation is not installed: {}@{}",
                version.id, version.version
            ),
            Self::InvalidFamilyDefinition { family, message } => write!(
                formatter,
                "native family {family} rejected the definition: {message}"
            ),
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
