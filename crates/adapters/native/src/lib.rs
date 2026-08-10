//! MOD-ADP-NAT: the first-party algorithmic question adapter.
//!
//! The engine is question agnostic: [`NativeAdapter`] dispatches by the
//! versioned [`generator::NativeQuestionFamily`] contract, while a family owns
//! only parameter-to-prompt materialization and server-only key derivation.
//! Issue returns a key-free envelope and reproducibility record. Grading
//! regenerates that exact instance, verifies its hashes and implementation
//! versions, and delegates correctness to `grading`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use domain::draft_preview::{PresentationError, materialize_prompt};
use domain::generator::{GeneratedVariant, GenerationError, generate};
use grading::{GradeOutcome, GradingError, grade};
use question_model::capability::BackendCapabilities;
use question_model::envelope::{ContentBlock, QuestionEnvelope};
use question_model::generation::{GeneratorReference, Seed};
use question_model::{
    AssetId, AttemptProvenance, DraftQuestionDefinition, DraftQuestionSource, FeedbackContent,
    ImplementationVersion, ObjectId, QuestionDefinition, QuestionSource, QuestionTitleError,
    StudentResponse,
};
use sha2::{Digest, Sha256};

use crate::generator::{AuthorPresentationContent, NativeQuestionFamily};
use crate::peptide_bond_geometry::PeptideBondGeometryV1;

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
/// the immutable asset records belonging to the published question version,
/// then passes those bindings to native issue, replay, and grading.
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
/// Its fields are already-rendered learner-facing blocks.  It intentionally
/// contains no answer key, choice ID, grading rule, source locator, or
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

impl NativeAdapter {
    /// Builds the production registry with the first reviewed reference family.
    pub fn new() -> Self {
        let mut adapter = Self::empty();
        adapter
            .register_family(flat_question::FlatSingleChoiceFamily)
            .expect("the built-in static flat family registration is unique");
        adapter
            .register_family(PeptideBondGeometryV1)
            .expect("the built-in family registration is unique");
        adapter
    }

    /// Builds an empty registry for explicit composition and contract tests.
    pub fn empty() -> Self {
        let current_adapter = implementation_version(ADAPTER_ID, ADAPTER_VERSION);
        let current_grading = implementation_version(GRADING_ID, GRADING_VERSION);
        Self {
            families: BTreeMap::new(),
            adapter_implementations: BTreeMap::from([(
                ImplementationRegistrationKey::from(&current_adapter),
                NativeExecution::V1,
            )]),
            grading_implementations: BTreeMap::from([(
                ImplementationRegistrationKey::from(&current_grading),
                NativeExecution::V1,
            )]),
            current_adapter,
            current_grading,
        }
    }

    /// Selects installed execution versions for newly issued attempts.
    ///
    /// Future execution versions must first be added to the exact registry;
    /// unknown persisted versions are refused.
    ///
    /// # Errors
    ///
    /// Refuses a version without a compiled execution implementation.
    pub fn select_current_implementations(
        &mut self,
        adapter: ImplementationVersion,
        grading: ImplementationVersion,
    ) -> Result<(), NativeAdapterError> {
        self.execution_for(&self.adapter_implementations, &adapter, "adapter")?;
        self.execution_for(&self.grading_implementations, &grading, "grading")?;
        self.current_adapter = adapter;
        self.current_grading = grading;
        Ok(())
    }

    /// Adds one family without changing adapter dispatch.
    ///
    /// # Errors
    ///
    /// Returns [`NativeAdapterError::DuplicateFamily`] when another
    /// implementation already owns the same family and generator version.
    /// Versions of one family coexist, so published content can continue to
    /// regenerate with its pinned generator after a new version is added.
    pub fn register_family<F>(&mut self, family: F) -> Result<(), NativeAdapterError>
    where
        F: NativeQuestionFamily + 'static,
    {
        let key = FamilyRegistrationKey::from_family(&family);
        if self.families.contains_key(&key) {
            return Err(NativeAdapterError::DuplicateFamily(key.family));
        }
        self.families.insert(key, Arc::new(family));
        Ok(())
    }

    /// Returns the trusted catalog capabilities for one native source.
    ///
    /// A source names a family but not a generator version. When several
    /// versions are installed, this returns their capability intersection: a
    /// catalog page never advertises a feature unavailable to an older
    /// published version. Exact issue and grading dispatch use the immutable
    /// generator reference in the question definition.
    ///
    /// # Errors
    ///
    /// Returns [`NativeAdapterError::UnsupportedSource`] for a non-native
    /// source and [`NativeAdapterError::UnknownFamily`] for an unregistered
    /// native family.
    pub fn capabilities(
        &self,
        source: &QuestionSource,
    ) -> Result<BackendCapabilities, NativeAdapterError> {
        let families = self.families_for_source(source)?;
        let mut capabilities = families
            .first()
            .expect("a nonempty registry selection has a first family")
            .capabilities();
        for family in &families[1..] {
            capabilities = BackendCapabilities::from_iter(
                capabilities
                    .declared()
                    .filter(|capability| family.capabilities().supports(*capability)),
            );
        }
        Ok(capabilities)
    }

    /// Generates one key-free native question instance.
    ///
    /// `asset_bindings` comes from the trusted asset registry rather than from
    /// the browser. The adapter derives all referenced logical assets from the
    /// generated envelope, refuses incomplete or unrelated bindings, and
    /// records the resulting canonical immutable object list in provenance.
    ///
    /// # Errors
    ///
    /// Returns [`NativeAdapterError`] when the source or generator is not
    /// installed, the authored family definition is invalid, generation
    /// fails, or the envelope cannot be hashed.
    pub fn issue(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        asset_bindings: &[AssetObjectBinding],
    ) -> Result<NativeIssuedAttempt, NativeAdapterError> {
        let prepared = self.prepare(question, seed)?;
        let asset_objects = resolve_asset_objects(&prepared.envelope, asset_bindings)?;
        let provenance = AttemptProvenance {
            adapter: self.current_adapter.clone(),
            renderer: None,
            generator: prepared.generated.generator.clone(),
            source_artifact: None,
            asset_objects,
            grading: self.current_grading.clone(),
            rendered_question_sha256: prepared.rendered_question_sha256,
        };
        Ok(NativeIssuedAttempt {
            envelope: prepared.envelope,
            parameter_hash: prepared.parameter_hash,
            provenance,
        })
    }

    /// Builds one deterministic instructor presentation for an unversioned
    /// native draft without constructing a published identity or returning
    /// server-only grading material.
    ///
    /// `Ok(None)` means the installed native family has not implemented a
    /// safe display-ready author presentation.  Callers must report that
    /// state explicitly; they must not fall back to serializing an answer key.
    pub fn author_presentation(
        &self,
        question: &DraftQuestionDefinition,
        seed: Seed,
    ) -> Result<Option<NativeDraftAuthorPresentation>, NativeAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(NativeAdapterError::InvalidTitle)?;
        let generated =
            generate(seed, &question.randomization).map_err(NativeAdapterError::Generation)?;
        let family =
            self.family_for_draft_source(&question.source, generated.generator.as_ref())?;
        let prompt = materialize_prompt(&question.prompt, seed, &question.randomization)
            .map_err(NativeAdapterError::Presentation)?;
        let Some(AuthorPresentationContent {
            correct_response,
            rationale,
        }) = family.derive_author_presentation(question, &generated, &prompt)?
        else {
            return Ok(None);
        };
        Ok(Some(NativeDraftAuthorPresentation {
            title: question.metadata.title.clone(),
            prompt,
            response: question.response.clone(),
            correct_response,
            rationale,
        }))
    }

    /// Reproduces an issued browser-safe envelope and verifies its record.
    ///
    /// `asset_bindings` must be resolved by a trusted server-side asset
    /// registry for this immutable question version. It is deliberately not
    /// browser input. No answer key is returned.
    pub fn reproduce(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        recorded_parameter_hash: &str,
        recorded_provenance: &AttemptProvenance,
        asset_bindings: &[AssetObjectBinding],
    ) -> Result<QuestionEnvelope, NativeAdapterError> {
        let adapter_execution = self.execution_for(
            &self.adapter_implementations,
            &recorded_provenance.adapter,
            "adapter",
        )?;
        let prepared = self.prepare_with_execution(question, seed, adapter_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_provenance,
            &resolve_asset_objects(&prepared.envelope, asset_bindings)?,
        )?;
        Ok(prepared.envelope)
    }

    /// Reproduces an issued instance, verifies its record, and grades a response.
    ///
    /// The answer key exists only inside this call. It is derived again from
    /// the immutable definition and recorded seed, passed directly to
    /// `grading`, and never returned or serialized.
    ///
    /// # Errors
    ///
    /// Returns [`NativeAdapterError::ReproductionMismatch`] when the recorded
    /// attempt cannot be reproduced exactly, or another explicit adapter,
    /// generation, family, or grading error.
    pub fn grade(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        recorded_parameter_hash: &str,
        recorded_provenance: &AttemptProvenance,
        asset_bindings: &[AssetObjectBinding],
        response: &StudentResponse,
    ) -> Result<GradeOutcome, NativeAdapterError> {
        let adapter_execution = self.execution_for(
            &self.adapter_implementations,
            &recorded_provenance.adapter,
            "adapter",
        )?;
        let grading_execution = self.execution_for(
            &self.grading_implementations,
            &recorded_provenance.grading,
            "grading",
        )?;
        let prepared = self.prepare_with_execution(question, seed, adapter_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_provenance,
            &resolve_asset_objects(&prepared.envelope, asset_bindings)?,
        )?;
        let outcome = grading_execution
            .grade(
                question,
                response,
                prepared.materialized.answer_key.as_ref(),
            )
            .map_err(NativeAdapterError::Grading)?;
        Ok(outcome)
    }

    /// Reproduces, verifies, grades, and materializes private teaching content
    /// in one trusted pass. This is deliberately separate from [`Self::grade`]
    /// so a route cannot grade once and later recreate feedback against a
    /// different instance or provenance record.
    pub fn grade_with_feedback(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        recorded_parameter_hash: &str,
        recorded_provenance: &AttemptProvenance,
        asset_bindings: &[AssetObjectBinding],
        response: &StudentResponse,
    ) -> Result<(GradeOutcome, FeedbackContent), NativeAdapterError> {
        let adapter_execution = self.execution_for(
            &self.adapter_implementations,
            &recorded_provenance.adapter,
            "adapter",
        )?;
        let grading_execution = self.execution_for(
            &self.grading_implementations,
            &recorded_provenance.grading,
            "grading",
        )?;
        let prepared = self.prepare_with_execution(question, seed, adapter_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_provenance,
            &resolve_asset_objects(&prepared.envelope, asset_bindings)?,
        )?;
        let outcome = grading_execution
            .grade(
                question,
                response,
                prepared.materialized.answer_key.as_ref(),
            )
            .map_err(NativeAdapterError::Grading)?;
        let GradeOutcome::Graded(result) = &outcome else {
            return Ok((outcome, FeedbackContent::default()));
        };
        let family = self
            .family_for_generated_source(&question.source, prepared.generated.generator.as_ref())?;
        let feedback = family.derive_feedback(
            question,
            &prepared.generated,
            &prepared.envelope,
            prepared.materialized.answer_key.as_ref(),
            result,
            response,
        )?;
        Ok((outcome, feedback))
    }

    fn prepare_with_execution(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        execution: &NativeExecution,
    ) -> Result<PreparedNativeQuestion, NativeAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(NativeAdapterError::InvalidTitle)?;
        let generated =
            generate(seed, &question.randomization).map_err(NativeAdapterError::Generation)?;
        let family =
            self.family_for_generated_source(&question.source, generated.generator.as_ref())?;
        let parameter_hash = generated.sha256().map_err(NativeAdapterError::Generation)?;
        let prompt = materialize_prompt(&question.prompt, seed, &question.randomization)
            .map_err(NativeAdapterError::Presentation)?;
        let materialized = MaterializedNativeQuestion {
            prompt,
            answer_key: execution.derive_answer_key(family, question, &generated)?,
        };
        let envelope = QuestionEnvelope {
            version: question.version,
            seed,
            title: question.metadata.title.clone(),
            prompt: materialized.prompt.clone(),
            response: question.response.clone(),
        };
        let rendered_question_sha256 = hash_json(&envelope)?;
        Ok(PreparedNativeQuestion {
            generated,
            materialized,
            envelope,
            parameter_hash,
            rendered_question_sha256,
        })
    }

    fn prepare(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
    ) -> Result<PreparedNativeQuestion, NativeAdapterError> {
        let execution = self.execution_for(
            &self.adapter_implementations,
            &self.current_adapter,
            "adapter",
        )?;
        self.prepare_with_execution(question, seed, execution)
    }

    fn execution_for<'a>(
        &self,
        implementations: &'a BTreeMap<ImplementationRegistrationKey, NativeExecution>,
        version: &ImplementationVersion,
        field: &'static str,
    ) -> Result<&'a NativeExecution, NativeAdapterError> {
        implementations
            .get(&ImplementationRegistrationKey::from(version))
            .ok_or(NativeAdapterError::UnknownImplementation {
                field,
                version: version.clone(),
            })
    }

    fn families_for_source(
        &self,
        source: &QuestionSource,
    ) -> Result<Vec<&dyn NativeQuestionFamily>, NativeAdapterError> {
        let QuestionSource::Native { family } = source else {
            return Err(NativeAdapterError::UnsupportedSource);
        };
        let families: Vec<_> = self
            .families
            .iter()
            .filter(|(key, _)| key.family == *family)
            .map(|(_, registered)| Arc::as_ref(registered))
            .collect();
        if families.is_empty() {
            Err(NativeAdapterError::UnknownFamily(family.clone()))
        } else {
            Ok(families)
        }
    }

    fn family_for_generated_source(
        &self,
        source: &QuestionSource,
        generator: Option<&GeneratorReference>,
    ) -> Result<&dyn NativeQuestionFamily, NativeAdapterError> {
        let QuestionSource::Native { family } = source else {
            return Err(NativeAdapterError::UnsupportedSource);
        };
        let key = FamilyRegistrationKey {
            family: family.clone(),
            generator: generator.cloned(),
        };
        self.families.get(&key).map(Arc::as_ref).ok_or_else(|| {
            NativeAdapterError::UnknownGenerator {
                family: family.clone(),
                generator: generator.cloned(),
            }
        })
    }

    fn family_for_draft_source(
        &self,
        source: &DraftQuestionSource,
        generator: Option<&GeneratorReference>,
    ) -> Result<&dyn NativeQuestionFamily, NativeAdapterError> {
        let DraftQuestionSource::Native { family } = source else {
            return Err(NativeAdapterError::UnsupportedSource);
        };
        let key = FamilyRegistrationKey {
            family: family.clone(),
            generator: generator.cloned(),
        };
        self.families.get(&key).map(Arc::as_ref).ok_or_else(|| {
            NativeAdapterError::UnknownGenerator {
                family: family.clone(),
                generator: generator.cloned(),
            }
        })
    }
}

impl Default for NativeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

struct PreparedNativeQuestion {
    generated: GeneratedVariant,
    materialized: MaterializedNativeQuestion,
    envelope: QuestionEnvelope,
    parameter_hash: String,
    rendered_question_sha256: String,
}

/// Server-only result assembled after the shared key-free materializer runs.
struct MaterializedNativeQuestion {
    prompt: Vec<question_model::envelope::ContentBlock>,
    answer_key: Option<grading::AnswerKey>,
}

/// Compiled native execution semantics selected by persisted implementation
/// versions. Future semantics are additive: do not replace an old variant while
/// published attempts still reference it.
#[derive(Debug, Clone, Copy)]
enum NativeExecution {
    /// Initial reviewed execution semantics.
    V1,
}

impl NativeExecution {
    fn derive_answer_key(
        self,
        family: &dyn NativeQuestionFamily,
        question: &QuestionDefinition,
        generated: &GeneratedVariant,
    ) -> Result<Option<grading::AnswerKey>, NativeAdapterError> {
        match self {
            Self::V1 => family.derive_answer_key(question, generated),
        }
    }

    fn grade(
        self,
        question: &QuestionDefinition,
        response: &StudentResponse,
        answer_key: Option<&grading::AnswerKey>,
    ) -> Result<GradeOutcome, GradingError> {
        match self {
            Self::V1 => grade(question, response, answer_key),
        }
    }
}

/// Stable dispatch identity for one additive family implementation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FamilyRegistrationKey {
    family: String,
    generator: Option<GeneratorReference>,
}

/// Ordered registry key kept local so domain provenance stays a simple wire
/// record rather than acquiring adapter-specific collection traits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ImplementationRegistrationKey {
    id: String,
    version: String,
}

impl From<&ImplementationVersion> for ImplementationRegistrationKey {
    fn from(value: &ImplementationVersion) -> Self {
        Self {
            id: value.id.clone(),
            version: value.version.clone(),
        }
    }
}

impl FamilyRegistrationKey {
    fn from_family(family: &dyn NativeQuestionFamily) -> Self {
        Self {
            family: family.family().to_string(),
            generator: family.generator(),
        }
    }
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
        /// Registered source family.
        family: String,
        /// Generator implementation pinned by the question definition.
        generator: Option<GeneratorReference>,
    },
    /// A persisted adapter or grader version has no compiled implementation.
    UnknownImplementation {
        /// Provenance field that selected the missing implementation.
        field: &'static str,
        /// Exact persisted implementation identity.
        version: ImplementationVersion,
    },
    /// The authored definition does not meet its family's versioned contract.
    InvalidFamilyDefinition {
        /// Family rejecting the definition.
        family: String,
        /// Actionable description of the violated contract.
        message: String,
    },
    /// Persisted learner-facing metadata cannot be delivered safely.
    InvalidTitle(QuestionTitleError),
    /// Shared deterministic parameter generation failed.
    Generation(GenerationError),
    /// Shared key-free prompt presentation was invalid.
    Presentation(PresentationError),
    /// A browser-safe envelope could not be serialized for hashing.
    Serialization(String),
    /// Stored attempt metadata disagreed with exact regeneration.
    ReproductionMismatch {
        /// Name of the mismatched provenance field.
        field: &'static str,
    },
    /// A renderable asset in the generated envelope was not bound by the
    /// trusted storage layer.
    MissingAssetBinding(AssetId),
    /// A trusted binding was supplied for an asset the generated envelope does
    /// not render. Provenance must describe exactly the delivered assets.
    UnrelatedAssetBinding(AssetId),
    /// A binding list assigned one logical asset more than once. This rejects
    /// both conflicting object identities and ambiguous duplicate input.
    ConflictingAssetBinding(AssetId),
    /// The server-only generic grader refused the response or definition.
    Grading(GradingError),
}

impl std::fmt::Display for NativeAdapterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSource => formatter.write_str("question source is not native"),
            Self::UnknownFamily(family) => {
                write!(
                    formatter,
                    "native question family is not installed: {family}"
                )
            }
            Self::DuplicateFamily(family) => {
                write!(
                    formatter,
                    "native question family is registered twice: {family}"
                )
            }
            Self::UnknownGenerator { family, generator } => write!(
                formatter,
                "native family {family} has no installed implementation for generator {generator:?}"
            ),
            Self::UnknownImplementation { field, version } => write!(
                formatter,
                "native {field} implementation is not installed: {}@{}",
                version.id, version.version
            ),
            Self::InvalidFamilyDefinition { family, message } => {
                write!(
                    formatter,
                    "native family {family} rejected the definition: {message}"
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
            Self::MissingAssetBinding(asset) => {
                write!(
                    formatter,
                    "native rendered asset has no trusted object binding: {asset}"
                )
            }
            Self::UnrelatedAssetBinding(asset) => {
                write!(
                    formatter,
                    "native trusted binding is unrelated to the rendered envelope: {asset}"
                )
            }
            Self::ConflictingAssetBinding(asset) => {
                write!(
                    formatter,
                    "native trusted bindings conflict for asset: {asset}"
                )
            }
            Self::Grading(error) => write!(formatter, "native grading failed: {error}"),
        }
    }
}

impl std::error::Error for NativeAdapterError {}

fn verify_record(
    prepared: &PreparedNativeQuestion,
    recorded_parameter_hash: &str,
    recorded: &AttemptProvenance,
    expected_asset_objects: &[ObjectId],
) -> Result<(), NativeAdapterError> {
    verify_equal(
        prepared.parameter_hash == recorded_parameter_hash,
        "parameterHash",
    )?;
    verify_equal(recorded.renderer.is_none(), "renderer")?;
    verify_equal(
        recorded.generator == prepared.generated.generator,
        "generator",
    )?;
    verify_equal(recorded.source_artifact.is_none(), "sourceArtifact")?;
    verify_equal(
        recorded.asset_objects.as_slice() == expected_asset_objects,
        "assetObjects",
    )?;
    verify_equal(
        recorded.rendered_question_sha256 == prepared.rendered_question_sha256,
        "renderedQuestionSha256",
    )?;
    Ok(())
}

fn verify_equal(matches: bool, field: &'static str) -> Result<(), NativeAdapterError> {
    if matches {
        Ok(())
    } else {
        Err(NativeAdapterError::ReproductionMismatch { field })
    }
}

/// Resolves the exact immutable objects rendered by one browser-safe envelope.
///
/// Asset IDs are collected from prompt blocks and the nested content blocks of
/// multiple-choice and ordering response widgets. The output is ordered by
/// logical `AssetId`, not caller order, so an attempt provenance record is
/// stable across trusted storage implementations.
fn resolve_asset_objects(
    envelope: &QuestionEnvelope,
    asset_bindings: &[AssetObjectBinding],
) -> Result<Vec<ObjectId>, NativeAdapterError> {
    let referenced_assets = envelope_asset_ids(envelope);
    let mut bindings = BTreeMap::new();
    for binding in asset_bindings {
        if bindings.insert(binding.asset, binding.object).is_some() {
            return Err(NativeAdapterError::ConflictingAssetBinding(binding.asset));
        }
    }

    for asset in &referenced_assets {
        if !bindings.contains_key(asset) {
            return Err(NativeAdapterError::MissingAssetBinding(*asset));
        }
    }
    for asset in bindings.keys() {
        if !referenced_assets.contains(asset) {
            return Err(NativeAdapterError::UnrelatedAssetBinding(*asset));
        }
    }

    Ok(referenced_assets
        .iter()
        .map(|asset| {
            *bindings
                .get(asset)
                .expect("all referenced assets were verified as bound")
        })
        .collect())
}

fn envelope_asset_ids(envelope: &QuestionEnvelope) -> std::collections::BTreeSet<AssetId> {
    let mut assets = std::collections::BTreeSet::new();
    collect_content_assets(&envelope.prompt, &mut assets);
    match &envelope.response {
        question_model::response::ResponseDefinition::MultipleChoice { choices, .. }
        | question_model::response::ResponseDefinition::Ordering { items: choices } => {
            for choice in choices {
                collect_content_assets(&choice.body, &mut assets);
            }
        }
        question_model::response::ResponseDefinition::Numeric { .. }
        | question_model::response::ResponseDefinition::ShortText { .. }
        | question_model::response::ResponseDefinition::FileUpload { .. }
        // Native families do not produce external-tool questions. If a future
        // family does, its prompt can still be asset-free; launch and grading
        // remain an explicit server-owned backend capability.
        | question_model::response::ResponseDefinition::ExternalTool {} => {}
    }
    assets
}

fn collect_content_assets(
    blocks: &[question_model::envelope::ContentBlock],
    assets: &mut std::collections::BTreeSet<AssetId>,
) {
    for block in blocks {
        if let question_model::envelope::ContentBlock::Image { asset, .. } = block {
            assets.insert(asset.asset);
        }
    }
}

fn implementation_version(id: &str, version: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.to_string(),
        version: version.to_string(),
    }
}

/// Hashes the canonical browser envelope encoding.
///
/// The canonical bytes are `serde_json::to_vec(QuestionEnvelope)` using the
/// model's fixed field declaration order and camelCase serde names. The fixed
/// vector test below locks this compatibility boundary; change it only with an
/// additive renderer/provenance migration.
fn hash_json(value: &QuestionEnvelope) -> Result<String, NativeAdapterError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| NativeAdapterError::Serialization(error.to_string()))?;
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use domain::draft_preview::{DraftPreviewRequest, DraftPreviewResult, preview_native_draft};
    use domain::generator::GeneratedVariant;
    use grading::{AnswerKey, GradingError};
    use question_model::answer::{NumericTolerance, SelectionCardinality};
    use question_model::capability::{BackendCapabilities, Capability};
    use question_model::envelope::{AssetRef, ContentBlock};
    use question_model::generation::{ParameterSpec, RandomizationDefinition};
    use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        AssetId, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, ProblemId,
        QuestionMetadata, QuestionSource, StudentResponse, VersionId, WorkspaceId,
    };
    use uuid::Uuid;

    use super::*;

    const FLAT_FAVORITE_COLOR: &str = r#"{
  "format": "pleFlatQuestion",
  "version": 1,
  "kind": "singleChoice",
  "title": "Favorite color",
  "prompt": "What is my favorite color?",
  "choices": [
    {"id": "blue", "text": "Blue", "feedback": "Blue is a calm choice."},
    {"id": "red", "text": "Red", "feedback": "Red is not my favorite."},
    {"id": "yellow", "text": "Yellow", "feedback": "Yellow is bright."}
  ],
  "correctChoice": "blue",
  "feedback": {
    "correct": "Exactly right.",
    "incorrect": "Try thinking of a cool color."
  },
  "points": 1.0,
  "attemptPolicy": {"maxAttempts": null, "feedback": "immediateFull"},
  "timingPolicy": {"kind": "untimed"},
  "tags": ["example"],
  "taxonomy": [],
  "license": {"kind": "ccBySa"},
  "language": "en-US"
}"#;

    fn flat_question() -> QuestionDefinition {
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
        let document =
            crate::flat_question::FlatQuestionDocument::parse(FLAT_FAVORITE_COLOR.as_bytes())
                .expect("flat fixture should parse");
        let draft = document
            .compile(workspace)
            .expect("flat fixture should compile")
            .into_parts()
            .0;
        QuestionDefinition::from_draft(
            draft,
            ProblemId::from_uuid(Uuid::from_u128(2)),
            VersionId::from_uuid(Uuid::from_u128(3)),
            QuestionSource::Native {
                family: crate::flat_question::FLAT_SINGLE_CHOICE_FAMILY.to_string(),
            },
        )
    }

    fn choice(id: &str, label: &str) -> ChoiceOption {
        ChoiceOption {
            id: ChoiceId::new(id),
            body: vec![ContentBlock::Text {
                markdown: label.to_string(),
            }],
        }
    }

    fn metadata(title: &str) -> QuestionMetadata {
        QuestionMetadata {
            title: title.to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBySa,
            language: "en-US".to_string(),
        }
    }

    fn peptide_question() -> QuestionDefinition {
        peptide_question_with_generator_version(peptide_bond_geometry::GENERATOR_VERSION)
    }

    fn peptide_question_with_generator_version(generator_version: &str) -> QuestionDefinition {
        let mut parameters = BTreeMap::new();
        parameters.insert(
            "residue".to_string(),
            ParameterSpec::Choice {
                options: vec!["alanine".to_string(), "glycine".to_string()],
            },
        );
        QuestionDefinition {
            version: VersionId::from_uuid(Uuid::from_u128(1)),
            problem: ProblemId::from_uuid(Uuid::from_u128(10)),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(2)),
            source: QuestionSource::Native {
                family: peptide_bond_geometry::FAMILY_ID.to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "In a peptide containing {{residue}}, which linkage is planar?"
                    .to_string(),
            }],
            response: ResponseDefinition::MultipleChoice {
                choices: vec![
                    choice("ester", "ester"),
                    choice("amide", "amide"),
                    choice("ether", "ether"),
                ],
                selection: SelectionCardinality::ExactlyOne,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::PerQuestion {
                seconds: 90,
                grace_seconds: 5,
            },
            randomization: RandomizationDefinition::Seeded {
                generator: GeneratorReference {
                    id: peptide_bond_geometry::GENERATOR_ID.to_string(),
                    version: generator_version.to_string(),
                },
                parameters,
            },
            grading: GradingDefinition::AllOrNothing { points: 2.0 },
            metadata: metadata("Peptide-bond geometry"),
        }
    }

    fn peptide_draft() -> DraftQuestionDefinition {
        let question = peptide_question();
        DraftQuestionDefinition {
            workspace: question.workspace,
            source: DraftQuestionSource::Native {
                family: peptide_bond_geometry::FAMILY_ID.to_string(),
            },
            prompt: question.prompt,
            response: question.response,
            attempt_policy: question.attempt_policy,
            timing_policy: question.timing_policy,
            randomization: question.randomization,
            grading: question.grading,
            metadata: question.metadata,
        }
    }

    #[test]
    fn author_presentation_is_deterministic_varied_and_contains_only_display_material() {
        let adapter = NativeAdapter::new();
        let draft = peptide_draft();
        let first = adapter
            .author_presentation(&draft, Seed::new(1))
            .expect("valid peptide draft should materialize")
            .expect("peptide family supplies an author presentation");
        let replay = adapter
            .author_presentation(&draft, Seed::new(1))
            .expect("valid peptide draft should replay")
            .expect("peptide family supplies an author presentation");

        assert!(first == replay, "the same seed must replay exactly");
        assert!(matches!(
            &first.prompt[0],
            ContentBlock::Text { markdown }
                if !markdown.contains("{{residue}}")
                    && (markdown.contains("alanine") || markdown.contains("glycine"))
        ));
        assert_eq!(
            first.correct_response,
            vec![ContentBlock::Text {
                markdown: "amide".to_string(),
            }],
            "the presentation copies the public choice body, never its identifier"
        );
        assert!(matches!(
            first.rationale.as_deref(),
            Some([ContentBlock::Text { markdown }])
                if markdown.contains("partial double-bond") && markdown.contains("planar")
        ));

        let varies = (2..=256).any(|seed| {
            adapter
                .author_presentation(&draft, Seed::new(seed))
                .expect("valid seeded draft should materialize")
                .is_some_and(|presentation| presentation.prompt != first.prompt)
        });
        assert!(
            varies,
            "the author preview must reveal an actual generated variant"
        );
    }

    #[test]
    fn a_family_without_an_author_presentation_is_honestly_unavailable() {
        let mut adapter = NativeAdapter::empty();
        adapter
            .register_family(NumericReferenceFamily)
            .expect("the test family is unique");
        let mut draft = peptide_draft();
        draft.source = DraftQuestionSource::Native {
            family: "numeric-reference".to_string(),
        };
        draft.randomization = RandomizationDefinition::Static;
        draft.prompt = vec![ContentBlock::Text {
            markdown: "Enter the reference value.".to_string(),
        }];

        assert!(
            adapter
                .author_presentation(&draft, Seed::new(4))
                .expect("the default author-presentation implementation is safe")
                .is_none(),
            "families opt in explicitly; the engine never serializes a grading key as a fallback"
        );
    }

    #[test]
    fn same_seed_issues_the_same_key_free_question_and_reproduction_record() {
        let adapter = NativeAdapter::new();
        let question = peptide_question();

        let first = adapter
            .issue(&question, Seed::new(37), &[])
            .expect("valid peptide question should issue");
        let replay = adapter
            .issue(&question, Seed::new(37), &[])
            .expect("same question and seed should issue again");

        assert_eq!(first, replay);
        let delivered = serde_json::to_string(&first.envelope)
            .expect("issued envelope should serialize for the browser");
        assert!(!delivered.contains("correct"));
        assert!(!delivered.contains("expected"));
        assert_eq!(first.provenance.adapter.version, ADAPTER_VERSION);
        assert_eq!(first.provenance.grading.version, GRADING_VERSION);
        assert_eq!(
            first.provenance.generator,
            Some(GeneratorReference {
                id: peptide_bond_geometry::GENERATOR_ID.to_string(),
                version: peptide_bond_geometry::GENERATOR_VERSION.to_string(),
            })
        );
    }

    #[test]
    fn flat_family_capabilities_are_installed_and_reproducible_without_answer_keys() {
        let adapter = NativeAdapter::new();
        let question = flat_question();
        let expected = BackendCapabilities::from_iter([
            Capability::ClientRendering,
            Capability::ServerGrading,
            Capability::Hints,
            Capability::PerQuestionTiming,
        ]);

        assert_eq!(
            adapter
                .capabilities(&question.source)
                .expect("family is installed"),
            expected
        );
        let issue = adapter
            .issue(&question, Seed::new(10), &[])
            .expect("flat family issue should be key free");
        let replay = adapter
            .reproduce(
                &question,
                Seed::new(10),
                &issue.parameter_hash,
                &issue.provenance,
                &[],
            )
            .expect("flat issue should reproduce exactly");

        assert_eq!(issue.envelope, replay);
        let public = serde_json::to_string(&issue.envelope)
            .expect("issued envelope should serialize for learner");
        assert!(!public.contains("correctChoice"));
        assert!(!public.contains("publicSha256"));
    }

    #[test]
    fn flat_family_grade_refuses_without_server_persisted_material() {
        let adapter = NativeAdapter::new();
        let question = flat_question();
        let issue = adapter
            .issue(&question, Seed::new(11), &[])
            .expect("flat issue should deliver reproducible envelope");

        assert!(matches!(
            adapter.grade(
                &question,
                Seed::new(11),
                &issue.parameter_hash,
                &issue.provenance,
                &[],
                &StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("blue")],
                },
            ),
            Err(NativeAdapterError::Grading(GradingError::MissingAnswerKey))
        ));
    }

    #[test]
    fn native_draft_preview_matches_the_published_envelope_presentation() {
        let adapter = NativeAdapter::new();
        let question = peptide_question();
        let seed = Seed::new(37);
        let issued = adapter.issue(&question, seed, &[]).expect("native issue");
        let preview = preview_native_draft(
            &DraftPreviewRequest {
                workspace: question.workspace,
                source: DraftQuestionSource::Native {
                    family: peptide_bond_geometry::FAMILY_ID.to_string(),
                },
                title: question.metadata.title.clone(),
                prompt: question.prompt.clone(),
                response: question.response.clone(),
                randomization: question.randomization.clone(),
            },
            seed,
        )
        .expect("native preview");
        let DraftPreviewResult::Ready { preview } = preview else {
            panic!("native previews locally")
        };
        assert_eq!(preview.title, issued.envelope.title);
        assert_eq!(preview.prompt, issued.envelope.prompt);
        assert_eq!(preview.response, issued.envelope.response);
    }

    #[test]
    fn correct_and_wrong_responses_are_graded_only_after_regeneration() {
        let adapter = NativeAdapter::new();
        let question = peptide_question();
        let issued = adapter
            .issue(&question, Seed::new(99), &[])
            .expect("valid peptide question should issue");

        let correct = adapter
            .grade(
                &question,
                Seed::new(99),
                &issued.parameter_hash,
                &issued.provenance,
                &[],
                &StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
            )
            .expect("matching attempt should grade");
        let wrong = adapter
            .grade(
                &question,
                Seed::new(99),
                &issued.parameter_hash,
                &issued.provenance,
                &[],
                &StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("ester")],
                },
            )
            .expect("matching attempt should grade");

        assert!(matches!(
            correct,
            GradeOutcome::Graded(result) if result.correct && result.points_earned == 2.0
        ));
        assert!(matches!(
            wrong,
            GradeOutcome::Graded(result) if !result.correct && result.points_earned == 0.0
        ));
    }

    #[test]
    fn peptide_feedback_uses_public_choice_blocks_without_exposing_key_material() {
        let adapter = NativeAdapter::new();
        let question = peptide_question();
        let issued = adapter
            .issue(&question, Seed::new(99), &[])
            .expect("native issue");
        let (outcome, feedback) = adapter
            .grade_with_feedback(
                &question,
                Seed::new(99),
                &issued.parameter_hash,
                &issued.provenance,
                &[],
                &StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("ester")],
                },
            )
            .expect("verified wrong response receives teaching feedback");
        assert!(matches!(outcome, GradeOutcome::Graded(result) if !result.correct));
        assert_eq!(
            feedback.correct_response,
            Some(vec![ContentBlock::Text {
                markdown: "amide".to_string(),
            }])
        );
        let hint = feedback
            .hint
            .expect("implemented family advertises a real hint");
        let rationale = feedback
            .rationale
            .expect("implemented family provides rationale");
        assert!(
            matches!(&hint[0], ContentBlock::Text { markdown } if markdown.contains("lone pair"))
        );
        assert!(
            matches!(&rationale[0], ContentBlock::Text { markdown } if markdown.contains("partial double-bond") && markdown.contains("planar"))
        );
        assert!(
            adapter
                .capabilities(&question.source)
                .expect("registered family")
                .supports(Capability::Hints)
        );
    }

    #[test]
    fn altered_attempt_provenance_is_refused_before_grading() {
        let adapter = NativeAdapter::new();
        let question = peptide_question();
        let issued = adapter
            .issue(&question, Seed::new(5), &[])
            .expect("valid peptide question should issue");
        let mut altered = issued.provenance;
        altered.grading.version = "different".to_string();

        assert!(matches!(
            adapter.grade(
                &question,
                Seed::new(5),
                &issued.parameter_hash,
                &altered,
                &[],
                &StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
            ),
            Err(NativeAdapterError::UnknownImplementation {
                field: "grading",
                ..
            })
        ));
    }

    #[test]
    fn uninstalled_execution_versions_are_refused_before_issue_or_grading() {
        let mut adapter = NativeAdapter::new();
        assert!(matches!(
            adapter.select_current_implementations(
                implementation_version(ADAPTER_ID, "2"),
                implementation_version(GRADING_ID, GRADING_VERSION),
            ),
            Err(NativeAdapterError::UnknownImplementation {
                field: "adapter",
                ..
            })
        ));
    }

    #[test]
    fn uninstalled_generator_versions_are_refused_without_fallback() {
        let adapter = NativeAdapter::new();
        let mut question = peptide_question();
        let RandomizationDefinition::Seeded { generator, .. } = &mut question.randomization else {
            panic!("peptide fixture is seeded")
        };
        generator.version = "2".to_string();

        assert!(matches!(
            adapter.issue(&question, Seed::new(61), &[]),
            Err(NativeAdapterError::UnknownGenerator { family, generator: Some(found) })
                if family == peptide_bond_geometry::FAMILY_ID && found.version == "2"
        ));
    }

    fn asset(id: u128) -> AssetId {
        AssetId::from_uuid(Uuid::from_u128(id))
    }

    fn image(asset: AssetId) -> ContentBlock {
        ContentBlock::Image {
            asset: AssetRef {
                asset,
                checksum: "fixture-checksum".to_string(),
            },
            description: "A trusted fixture image.".to_string(),
        }
    }

    #[test]
    fn asset_provenance_requires_exact_complete_trusted_bindings() {
        let adapter = NativeAdapter::new();
        let mut question = peptide_question();
        let rendered_asset = asset(81);
        question.prompt.push(image(rendered_asset));
        let bindings = [AssetObjectBinding {
            asset: rendered_asset,
            object: ObjectId::from_uuid(Uuid::from_u128(82)),
        }];
        let issued = adapter
            .issue(&question, Seed::new(82), &bindings)
            .expect("trusted assets should be recorded at issue time");
        assert_eq!(issued.provenance.asset_objects, vec![bindings[0].object]);

        assert!(matches!(
            adapter.issue(&question, Seed::new(82), &[]),
            Err(NativeAdapterError::MissingAssetBinding(found)) if found == rendered_asset
        ));
        assert!(matches!(
            adapter.issue(
                &peptide_question(),
                Seed::new(82),
                &bindings,
            ),
            Err(NativeAdapterError::UnrelatedAssetBinding(found)) if found == rendered_asset
        ));
        assert!(matches!(
            adapter.issue(
                &question,
                Seed::new(82),
                &[
                    bindings[0],
                    AssetObjectBinding {
                        asset: rendered_asset,
                        object: ObjectId::from_uuid(Uuid::from_u128(83)),
                    },
                ],
            ),
            Err(NativeAdapterError::ConflictingAssetBinding(found)) if found == rendered_asset
        ));

        let mut altered_provenance = issued.provenance.clone();
        altered_provenance.asset_objects = vec![ObjectId::from_uuid(Uuid::from_u128(84))];

        assert!(matches!(
            adapter.reproduce(
                &question,
                Seed::new(82),
                &issued.parameter_hash,
                &altered_provenance,
                &bindings,
            ),
            Err(NativeAdapterError::ReproductionMismatch {
                field: "assetObjects"
            })
        ));
    }

    #[test]
    fn nested_response_assets_are_bound_in_canonical_logical_asset_order() {
        let adapter = NativeAdapter::new();
        let mut question = peptide_question();
        let prompt_asset = asset(91);
        let response_asset = asset(90);
        question.prompt.push(image(prompt_asset));
        let ResponseDefinition::MultipleChoice { choices, .. } = &mut question.response else {
            panic!("peptide fixture has multiple-choice response")
        };
        choices[0].body.push(image(response_asset));
        let bindings = [
            AssetObjectBinding {
                asset: prompt_asset,
                object: ObjectId::from_uuid(Uuid::from_u128(191)),
            },
            AssetObjectBinding {
                asset: response_asset,
                object: ObjectId::from_uuid(Uuid::from_u128(190)),
            },
        ];

        let issued = adapter
            .issue(&question, Seed::new(91), &bindings)
            .expect("prompt and nested response assets should resolve");

        assert_eq!(
            issued.provenance.asset_objects,
            vec![bindings[1].object, bindings[0].object],
            "asset IDs, not caller order, canonically order persisted objects"
        );
    }

    #[test]
    fn rendered_envelope_hash_has_a_fixed_compatibility_vector() {
        let adapter = NativeAdapter::new();
        let issued = adapter
            .issue(&peptide_question(), Seed::new(37), &[])
            .expect("fixed vector should issue");
        assert_eq!(
            issued.provenance.rendered_question_sha256,
            "7300981097ff06e8237a30336738efcba49eb5236219d8002934666c01334a86"
        );
    }

    #[test]
    fn historical_blank_or_oversized_titles_are_refused_before_issue() {
        let adapter = NativeAdapter::new();
        for title in [" \t".to_string(), "\u{1F9EC}".repeat(513)] {
            let mut question = peptide_question();
            question.metadata.title = title;
            assert!(matches!(
                adapter.issue(&question, Seed::new(37), &[]),
                Err(NativeAdapterError::InvalidTitle(_))
            ));
        }
    }

    #[test]
    fn a_family_refuses_seeded_content_that_cannot_show_its_variation() {
        let adapter = NativeAdapter::new();
        let mut question = peptide_question();
        question.prompt = vec![ContentBlock::Text {
            markdown: "Which linkage is planar?".to_string(),
        }];

        assert!(matches!(
            adapter.issue(&question, Seed::new(1), &[]),
            Err(NativeAdapterError::InvalidFamilyDefinition { .. })
        ));
    }

    #[derive(Debug, Clone, Copy)]
    struct NumericReferenceFamily;

    impl NativeQuestionFamily for NumericReferenceFamily {
        fn family(&self) -> &'static str {
            "numeric-reference"
        }

        fn generator(&self) -> Option<GeneratorReference> {
            None
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::from_iter([Capability::ClientRendering, Capability::ServerGrading])
        }

        fn derive_answer_key(
            &self,
            question: &QuestionDefinition,
            _generated: &GeneratedVariant,
        ) -> Result<Option<AnswerKey>, NativeAdapterError> {
            if !matches!(question.response, ResponseDefinition::Numeric { .. }) {
                return Err(NativeAdapterError::InvalidFamilyDefinition {
                    family: self.family().to_string(),
                    message: "numeric response required".to_string(),
                });
            }
            Ok(Some(AnswerKey::Numeric { expected: 7.0 }))
        }
    }

    #[test]
    fn a_second_family_plugs_into_the_registry_without_engine_changes() {
        let mut adapter = NativeAdapter::empty();
        adapter
            .register_family(NumericReferenceFamily)
            .expect("new family identifier should register");
        let question = QuestionDefinition {
            version: VersionId::from_uuid(Uuid::from_u128(3)),
            problem: ProblemId::from_uuid(Uuid::from_u128(11)),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(4)),
            source: QuestionSource::Native {
                family: "numeric-reference".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Enter the reference value.".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Exact,
                unit: None,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: Some(1),
                feedback: FeedbackDisclosure::Deferred,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: metadata("Numeric registry extension"),
        };
        let issued = adapter
            .issue(&question, Seed::new(123), &[])
            .expect("registered family should issue through the generic adapter");

        assert!(
            adapter
                .capabilities(&question.source)
                .expect("registered source should expose capabilities")
                .supports(Capability::ServerGrading)
        );
        assert!(matches!(
            adapter.grade(
                &question,
                Seed::new(123),
                &issued.parameter_hash,
                &issued.provenance,
                &[],
                &StudentResponse::Numeric { value: 7.0 },
            ),
            Ok(GradeOutcome::Graded(result)) if result.correct
        ));
    }

    #[derive(Debug, Clone, Copy)]
    struct VersionedNumericFamily {
        version: &'static str,
        expected: f64,
        supports_client_rendering: bool,
    }

    impl NativeQuestionFamily for VersionedNumericFamily {
        fn family(&self) -> &'static str {
            "versioned-numeric"
        }

        fn generator(&self) -> Option<GeneratorReference> {
            Some(GeneratorReference {
                id: "versioned-numeric-generator".to_string(),
                version: self.version.to_string(),
            })
        }

        fn capabilities(&self) -> BackendCapabilities {
            let mut capabilities = vec![Capability::ServerGrading];
            if self.supports_client_rendering {
                capabilities.push(Capability::ClientRendering);
            }
            BackendCapabilities::from_iter(capabilities)
        }

        fn derive_answer_key(
            &self,
            question: &QuestionDefinition,
            _generated: &GeneratedVariant,
        ) -> Result<Option<AnswerKey>, NativeAdapterError> {
            let _ = question;
            Ok(Some(AnswerKey::Numeric {
                expected: self.expected,
            }))
        }
    }

    fn versioned_numeric_question(version: &str) -> QuestionDefinition {
        QuestionDefinition {
            version: VersionId::from_uuid(Uuid::from_u128(if version == "1" { 5 } else { 6 })),
            problem: ProblemId::from_uuid(Uuid::from_u128(12)),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(7)),
            source: QuestionSource::Native {
                family: "versioned-numeric".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Enter the generated reference value.".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Exact,
                unit: None,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: Some(1),
                feedback: FeedbackDisclosure::Deferred,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Seeded {
                generator: GeneratorReference {
                    id: "versioned-numeric-generator".to_string(),
                    version: version.to_string(),
                },
                parameters: BTreeMap::new(),
            },
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: metadata("Versioned numeric family"),
        }
    }

    #[test]
    fn additive_generator_versions_coexist_while_catalog_capabilities_stay_conservative() {
        let mut adapter = NativeAdapter::empty();
        adapter
            .register_family(VersionedNumericFamily {
                version: "1",
                expected: 1.0,
                supports_client_rendering: true,
            })
            .expect("first generator version should register");
        adapter
            .register_family(VersionedNumericFamily {
                version: "2",
                expected: 2.0,
                supports_client_rendering: false,
            })
            .expect("additive generator version should coexist with the first");

        let version_one = versioned_numeric_question("1");
        let version_two = versioned_numeric_question("2");
        let first_issue = adapter
            .issue(&version_one, Seed::new(41), &[])
            .expect("published generator version 1 remains dispatchable");
        let second_issue = adapter
            .issue(&version_two, Seed::new(41), &[])
            .expect("published generator version 2 dispatches independently");

        let catalog_capabilities = adapter
            .capabilities(&version_one.source)
            .expect("family capabilities should resolve without a generator reference");
        assert!(catalog_capabilities.supports(Capability::ServerGrading));
        assert!(!catalog_capabilities.supports(Capability::ClientRendering));
        assert!(matches!(
            adapter.grade(
                &version_one,
                Seed::new(41),
                &first_issue.parameter_hash,
                &first_issue.provenance,
                &[],
                &StudentResponse::Numeric { value: 1.0 },
            ),
            Ok(GradeOutcome::Graded(result)) if result.correct
        ));
        assert!(matches!(
            adapter.grade(
                &version_two,
                Seed::new(41),
                &second_issue.parameter_hash,
                &second_issue.provenance,
                &[],
                &StudentResponse::Numeric { value: 2.0 },
            ),
            Ok(GradeOutcome::Graded(result)) if result.correct
        ));
    }
}
