//! MOD-ADP-WW: WeBWorK PG adapter, isolated renderer boundary, and render cache.
//!
//! WeBWorK execution remains in a separate, non-public renderer service.  This
//! crate turns its output into the shared question model, caches only
//! browser-safe rendered output by immutable `(version, seed)`, and delegates
//! grading back to that service.  Answer material never enters this crate's
//! public results or the browser cache.

use std::fmt::Write as _;

use objects::{ObjectKey, ObjectStore, ObjectStoreError, PutObject};
use question_model::capability::{BackendCapabilities, Capability};
use question_model::generation::Seed;
use question_model::{
    ActivityTimestamp, AttemptProvenance, ImplementationVersion, ObjectId, ProblemId,
    QuestionDefinition, QuestionEnvelope, QuestionSource, QuestionTitleError, SourceArtifact,
    StudentResponse, VersionId,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::pg_parser_stub::{GradeRequest, RenderRequest, RendererFailure, WebworkRenderer};
use crate::sanitizer::sanitize_webwork_html;

/// Bounded, deployment-configured private HTTP client for a renderer service.
pub mod http_renderer;
/// PG source handling and the isolated renderer client contract.
pub mod pg_parser_stub;
/// The server-side allowlist applied to untrusted renderer markup.
pub mod sanitizer;

pub use crate::http_renderer::{
    HttpWebworkRenderer, HttpWebworkRendererConfig, RendererConfigError,
};

/// Stable adapter identifier recorded for WeBWorK attempts.
pub const ADAPTER_ID: &str = "webwork-adapter";
/// Current adapter compatibility implementation version.
///
/// This is intentionally independent of the repository's CalVer release.
pub const ADAPTER_VERSION: &str = "1";
/// Stable identifier for renderer-owned grading.
pub const GRADING_ID: &str = "webwork-renderer-grader";

/// Immutable PG source resolved from trusted object storage before adapter use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebworkSource {
    /// Published problem this source was resolved for.
    problem: ProblemId,
    /// Immutable published version this source was resolved for.
    version: VersionId,
    /// Immutable source artifact captured in every attempt provenance record.
    artifact: SourceArtifact,
    /// Exact stored PG bytes.  This input never originates in a browser request.
    pg_source: Vec<u8>,
}

impl WebworkSource {
    /// Resolves PG source only from its exact immutable published object key.
    ///
    /// The caller may hold catalog metadata, but it cannot self-assert source
    /// bytes: object identity, key, category, version, and digest are all
    /// checked at this boundary before the adapter receives the source.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        problem: ProblemId,
        version: VersionId,
        artifact: SourceArtifact,
    ) -> Result<Self, WebworkAdapterError> {
        let expected_key = ObjectKey::ProblemSource {
            problem,
            version,
            object: artifact.object,
        };
        let stored = store
            .get(&expected_key)
            .await
            .map_err(WebworkAdapterError::ObjectStore)?;
        if stored.record.key != expected_key
            || stored.record.id != artifact.object
            || stored.record.category != objects::ObjectCategory::Source
            || stored.record.version != Some(version)
            || stored.record.sha256.to_string() != artifact.sha256
        {
            return Err(WebworkAdapterError::UntrustedSource);
        }
        Ok(Self {
            problem,
            version,
            artifact,
            pg_source: stored.bytes,
        })
    }

    /// Immutable source artifact carried into attempt provenance.
    pub fn artifact(&self) -> &SourceArtifact {
        &self.artifact
    }
}

/// Key-free result returned when a WeBWorK question is issued.
#[derive(Debug, Clone, PartialEq)]
pub struct WebworkIssuedAttempt {
    /// Reusable browser-safe response contract and prompt blocks.
    pub envelope: QuestionEnvelope,
    /// Sanitized supplied markup for the dedicated renderer component.
    pub sanitized_html: String,
    /// Deterministic parameter record for the version/seed pair.
    pub parameter_hash: String,
    /// Immutable source, implementation, and rendered-output evidence.
    pub provenance: AttemptProvenance,
    /// Whether this response came from object storage rather than the renderer.
    pub cache_hit: bool,
}

/// Failures that are confined to this backend and can become a WeBWorK-only
/// degraded run state at the HTTP boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebworkAdapterError {
    /// A non-WeBWorK question reached this adapter.
    UnsupportedSource,
    /// The trusted source bytes do not match the immutable source record.
    SourceChecksumMismatch,
    /// Source was not resolved through its exact immutable published key.
    UntrustedSource,
    /// A valid source was resolved for a different published question.
    SourceDoesNotMatchQuestion,
    /// The renderer failed under its bounded service policy.
    Renderer(RendererFailure),
    /// Render-cache storage failed.
    ObjectStore(ObjectStoreError),
    /// Cache bytes could not be decoded as a browser-safe rendered question.
    InvalidCache(String),
    /// Renderer output did not match the immutable version/seed requested.
    InvalidRendererEnvelope(String),
    /// Persisted learner-facing metadata cannot be delivered safely.
    InvalidTitle(QuestionTitleError),
}

impl std::fmt::Display for WebworkAdapterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSource => formatter.write_str("question source is not WeBWorK"),
            Self::SourceChecksumMismatch => {
                formatter.write_str("PG source bytes do not match source record")
            }
            Self::UntrustedSource => formatter
                .write_str("PG source does not match its immutable published object identity"),
            Self::SourceDoesNotMatchQuestion => {
                formatter.write_str("PG source was resolved for a different published question")
            }
            Self::Renderer(error) => error.fmt(formatter),
            Self::ObjectStore(error) => error.fmt(formatter),
            Self::InvalidCache(message) => {
                write!(formatter, "invalid WeBWorK render cache: {message}")
            }
            Self::InvalidRendererEnvelope(message) => {
                write!(formatter, "invalid WeBWorK renderer envelope: {message}")
            }
            Self::InvalidTitle(error) => {
                write!(formatter, "invalid WeBWorK question title: {error}")
            }
        }
    }
}

impl std::error::Error for WebworkAdapterError {}

/// Question-agnostic WeBWorK adapter composed from an object store and a
/// separately deployed renderer client.
pub struct WebworkAdapter<S, R> {
    store: S,
    renderer: R,
}

impl<S, R> WebworkAdapter<S, R>
where
    S: ObjectStore,
    R: WebworkRenderer,
{
    /// Composes the adapter with its trusted storage and renderer boundaries.
    pub fn new(store: S, renderer: R) -> Self {
        Self { store, renderer }
    }

    /// Returns only capabilities this adapter can deliver for every PG source.
    pub fn capabilities(
        &self,
        source: &QuestionSource,
    ) -> Result<BackendCapabilities, WebworkAdapterError> {
        if matches!(source, QuestionSource::Webwork { .. }) {
            Ok(BackendCapabilities::from_iter([
                Capability::AlgorithmicGeneration,
                Capability::ServerGrading,
            ]))
        } else {
            Err(WebworkAdapterError::UnsupportedSource)
        }
    }

    /// Issues a browser-safe render, using the immutable `(version, seed)` cache first.
    ///
    /// The cache stores no grading material.  A successful cache hit performs
    /// no renderer call; cache misses render once and write immutable bytes.
    pub async fn issue(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        source: &WebworkSource,
        created_at: ActivityTimestamp,
    ) -> Result<WebworkIssuedAttempt, WebworkAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(WebworkAdapterError::InvalidTitle)?;
        let (problem, pg_path) = webwork_identity(question)?;
        verify_source(source)?;
        verify_source_binding(source, problem, question.version)?;
        let cache_key = render_key(problem, question.version, seed);
        match self.store.get(&cache_key).await {
            Ok(stored) => {
                let cached = decode_render(&stored.bytes)?;
                validate_cached(
                    &cached,
                    question.version,
                    seed,
                    source,
                    &question.metadata.title,
                )?;
                self.issued(cached, seed, source, true)
            }
            Err(ObjectStoreError::NotFound) => {
                let version = question.version.to_string();
                let mut untrusted = self
                    .renderer
                    .render(RenderRequest {
                        pg_source: &source.pg_source,
                        pg_path,
                        version: &version,
                        seed: seed.value(),
                    })
                    .await
                    .map_err(WebworkAdapterError::Renderer)?;
                validate_envelope(&untrusted.envelope, question.version, seed)?;
                // Renderer output is untrusted. The learner title is durable
                // published metadata, not a renderer/source-provided field.
                untrusted.envelope.title = question.metadata.title.clone();
                let rendered = CachedWebworkRender {
                    schema_version: CACHE_SCHEMA_VERSION,
                    source_artifact: source.artifact.clone(),
                    rendered: SafeRenderedWebworkQuestion {
                        envelope: untrusted.envelope,
                        sanitized_html: sanitize_webwork_html(&untrusted.html),
                        renderer: untrusted.renderer,
                    },
                };
                validate_cached(
                    &rendered,
                    question.version,
                    seed,
                    source,
                    &question.metadata.title,
                )?;
                let bytes = serde_json::to_vec(&rendered).map_err(|error| {
                    WebworkAdapterError::InvalidRendererEnvelope(error.to_string())
                })?;
                match self
                    .store
                    .put(PutObject {
                        key: cache_key.clone(),
                        bytes,
                        media_type: "application/json".to_string(),
                        license: license_label(&question.metadata.license),
                        provenance: format!(
                            "WeBWorK render for {} seed {}",
                            question.version,
                            seed.value()
                        ),
                        created_at,
                    })
                    .await
                {
                    Ok(_) => self.issued(rendered, seed, source, false),
                    Err(ObjectStoreError::AlreadyExists) => {
                        let stored = self
                            .store
                            .get(&cache_key)
                            .await
                            .map_err(WebworkAdapterError::ObjectStore)?;
                        let cached = decode_render(&stored.bytes)?;
                        validate_cached(
                            &cached,
                            question.version,
                            seed,
                            source,
                            &question.metadata.title,
                        )?;
                        self.issued(cached, seed, source, true)
                    }
                    Err(error) => Err(WebworkAdapterError::ObjectStore(error)),
                }
            }
            Err(error) => Err(WebworkAdapterError::ObjectStore(error)),
        }
    }

    /// Delegates correctness to the server-only renderer without exposing a key.
    pub async fn grade(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        source: &WebworkSource,
        response: &StudentResponse,
    ) -> Result<grading::GradeOutcome, WebworkAdapterError> {
        let (problem, pg_path) = webwork_identity(question)?;
        verify_source(source)?;
        verify_source_binding(source, problem, question.version)?;
        let version = question.version.to_string();
        self.renderer
            .grade(GradeRequest {
                pg_source: &source.pg_source,
                pg_path,
                version: &version,
                seed: seed.value(),
                response,
            })
            .await
            .map_err(WebworkAdapterError::Renderer)
    }

    fn issued(
        &self,
        rendered: CachedWebworkRender,
        seed: Seed,
        source: &WebworkSource,
        cache_hit: bool,
    ) -> Result<WebworkIssuedAttempt, WebworkAdapterError> {
        let rendered_question_sha256 = rendered_hash(&rendered)?;
        let renderer = rendered.rendered.renderer;
        Ok(WebworkIssuedAttempt {
            envelope: rendered.rendered.envelope,
            sanitized_html: rendered.rendered.sanitized_html,
            parameter_hash: parameter_hash(seed),
            provenance: AttemptProvenance {
                adapter: implementation(ADAPTER_ID, ADAPTER_VERSION),
                renderer: Some(implementation(&renderer.id, &renderer.version)),
                generator: None,
                source_artifact: Some(source.artifact.clone()),
                asset_objects: Vec::new(),
                grading: implementation(GRADING_ID, &renderer.version),
                rendered_question_sha256,
            },
            cache_hit,
        })
    }
}

fn webwork_identity(
    question: &QuestionDefinition,
) -> Result<(ProblemId, &str), WebworkAdapterError> {
    match &question.source {
        QuestionSource::Webwork { pg_path } => Ok((question.problem, pg_path)),
        _ => Err(WebworkAdapterError::UnsupportedSource),
    }
}

fn verify_source(source: &WebworkSource) -> Result<(), WebworkAdapterError> {
    let actual = objects::Sha256Digest::compute(&source.pg_source).to_string();
    if actual == source.artifact.sha256 {
        Ok(())
    } else {
        Err(WebworkAdapterError::SourceChecksumMismatch)
    }
}

fn verify_source_binding(
    source: &WebworkSource,
    problem: ProblemId,
    version: VersionId,
) -> Result<(), WebworkAdapterError> {
    if source.problem == problem && source.version == version {
        Ok(())
    } else {
        Err(WebworkAdapterError::SourceDoesNotMatchQuestion)
    }
}

fn render_key(problem: ProblemId, version: VersionId, seed: Seed) -> ObjectKey {
    ObjectKey::ProblemRender {
        problem,
        version,
        seed,
        object: deterministic_render_object_id(version, seed),
    }
}

fn deterministic_render_object_id(version: VersionId, seed: Seed) -> ObjectId {
    let mut hash = Sha256::new();
    hash.update(b"peptidyle:webwork-render-cache:v1");
    hash.update(version.as_uuid().as_bytes());
    hash.update(seed.value().to_be_bytes());
    let digest = hash.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(Uuid::from_bytes(bytes))
}

const CACHE_SCHEMA_VERSION: u8 = 1;

/// The only markup representation that may be serialized into cache or sent
/// to a browser.  Its construction is private to this module.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SafeRenderedWebworkQuestion {
    envelope: QuestionEnvelope,
    sanitized_html: String,
    renderer: crate::pg_parser_stub::RendererIdentity,
}

/// Immutable render-cache record with the evidence required for reproduction.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedWebworkRender {
    schema_version: u8,
    source_artifact: SourceArtifact,
    rendered: SafeRenderedWebworkQuestion,
}

fn decode_render(bytes: &[u8]) -> Result<CachedWebworkRender, WebworkAdapterError> {
    serde_json::from_slice(bytes)
        .map_err(|error| WebworkAdapterError::InvalidCache(error.to_string()))
}

fn validate_cached(
    cached: &CachedWebworkRender,
    version: VersionId,
    seed: Seed,
    source: &WebworkSource,
    title: &str,
) -> Result<(), WebworkAdapterError> {
    if cached.schema_version != CACHE_SCHEMA_VERSION
        || cached.source_artifact != source.artifact
        || cached.rendered.renderer.id.is_empty()
        || cached.rendered.renderer.version.is_empty()
    {
        return Err(WebworkAdapterError::InvalidCache(
            "cache provenance is incomplete or does not match the published source".to_string(),
        ));
    }
    validate_envelope(&cached.rendered.envelope, version, seed)?;
    if cached.rendered.envelope.title != title {
        return Err(WebworkAdapterError::InvalidCache(
            "cache title does not match immutable published metadata".to_string(),
        ));
    }
    Ok(())
}

fn validate_envelope(
    envelope: &QuestionEnvelope,
    version: VersionId,
    seed: Seed,
) -> Result<(), WebworkAdapterError> {
    if envelope.version != version {
        return Err(WebworkAdapterError::InvalidRendererEnvelope(
            "renderer returned a different immutable version".to_string(),
        ));
    }
    if envelope.seed != seed {
        return Err(WebworkAdapterError::InvalidRendererEnvelope(
            "renderer returned a different deterministic seed".to_string(),
        ));
    }
    Ok(())
}

fn implementation(id: &str, version: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.to_string(),
        version: version.to_string(),
    }
}

fn parameter_hash(seed: Seed) -> String {
    let mut hash = Sha256::new();
    hash.update(b"peptidyle:webwork-parameters:v1");
    hash.update(seed.value().to_be_bytes());
    hex_digest(hash.finalize().as_slice())
}

fn rendered_hash(rendered: &CachedWebworkRender) -> Result<String, WebworkAdapterError> {
    let bytes = serde_json::to_vec(rendered)
        .map_err(|error| WebworkAdapterError::InvalidRendererEnvelope(error.to_string()))?;
    Ok(hex_digest(Sha256::digest(bytes).as_slice()))
}

fn license_label(license: &question_model::taxonomy::License) -> String {
    use question_model::taxonomy::License;

    match license {
        License::AllRightsReserved => "all-rights-reserved".to_string(),
        License::CcBy => "CC-BY-4.0".to_string(),
        License::CcBySa => "CC-BY-SA-4.0".to_string(),
        License::CcByNc => "CC-BY-NC-4.0".to_string(),
        License::Cc0 => "CC0-1.0".to_string(),
        License::Other { spdx } => spdx.clone(),
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut value, "{byte:02x}");
    }
    value
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use grading::{AnswerKey, GradeOutcome, grade};
    use objects::Sha256Digest;
    use objects::memory::MemoryObjectStore;
    use question_model::answer::SelectionCardinality;
    use question_model::capability::Capability;
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{GradingDefinition, QuestionMetadata, WorkspaceId};

    use super::*;
    use crate::pg_parser_stub::{RenderedWebworkQuestion, RendererIdentity};

    const OPL_FIXTURE: &str = concat!(
        "## Recorded OPL-style example: a small multiple-choice PG question.\n",
        "DOCUMENT();\n",
        "loadMacros(\"PGstandard.pl\", \"PGchoicemacros.pl\");\n",
        "BEGIN_TEXT\n",
        "Which molecule is water?\n",
        "END_TEXT\n",
        "$showPartialCorrectAnswers = 0;\n",
        "ANS(str_cmp(\"H2O\"));\n",
        "ENDDOCUMENT();\n",
    );

    #[derive(Clone)]
    struct RecordedRenderer {
        calls: Arc<AtomicUsize>,
        failure: Option<RendererFailure>,
        identity: RendererIdentity,
        html: String,
    }

    #[async_trait]
    impl WebworkRenderer for RecordedRenderer {
        async fn render(
            &self,
            request: RenderRequest<'_>,
        ) -> Result<RenderedWebworkQuestion, RendererFailure> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(failure) = &self.failure {
                return Err(failure.clone());
            }
            if request.pg_source != OPL_FIXTURE.as_bytes()
                || request.pg_path != "Library/OPL/select-one.pg"
            {
                return Err(RendererFailure::InvalidOutput(
                    "recorded fixture source did not match request".to_string(),
                ));
            }
            Ok(RenderedWebworkQuestion {
                envelope: QuestionEnvelope {
                    version: VersionId::from_uuid(Uuid::from_u128(2)),
                    seed: Seed::new(request.seed),
                    title: "Untrusted renderer title".to_string(),
                    prompt: vec![ContentBlock::Text {
                        markdown: "Which molecule is water?".to_string(),
                    }],
                    response: ResponseDefinition::MultipleChoice {
                        choices: vec![
                            ChoiceOption {
                                id: ChoiceId::new("water"),
                                body: vec![ContentBlock::Text {
                                    markdown: "H&#x2082;O".to_string(),
                                }],
                            },
                            ChoiceOption {
                                id: ChoiceId::new("oxygen"),
                                body: vec![ContentBlock::Text {
                                    markdown: "O&#x2082;".to_string(),
                                }],
                            },
                        ],
                        selection: SelectionCardinality::ExactlyOne,
                    },
                },
                html: self.html.clone(),
                renderer: self.identity.clone(),
            })
        }

        async fn grade(&self, request: GradeRequest<'_>) -> Result<GradeOutcome, RendererFailure> {
            let rendered = self
                .render(RenderRequest {
                    pg_source: request.pg_source,
                    pg_path: request.pg_path,
                    version: request.version,
                    seed: request.seed,
                })
                .await?;
            let question = question_with_response(rendered.envelope.response);
            grade(
                &question,
                request.response,
                Some(&AnswerKey::MultipleChoice {
                    correct: [ChoiceId::new("water")].into_iter().collect(),
                }),
            )
            .map_err(|error| RendererFailure::InvalidOutput(error.to_string()))
        }
    }

    fn recorded_renderer(calls: Arc<AtomicUsize>) -> RecordedRenderer {
        RecordedRenderer {
            calls,
            failure: None,
            identity: RendererIdentity {
                id: "recorded-opl-renderer".to_string(),
                version: "1".to_string(),
            },
            html: "<p>Which molecule is water?</p>".to_string(),
        }
    }

    fn question_with_response(response: ResponseDefinition) -> QuestionDefinition {
        QuestionDefinition {
            version: VersionId::from_uuid(Uuid::from_u128(2)),
            problem: ProblemId::from_uuid(Uuid::from_u128(1)),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(3)),
            source: QuestionSource::Webwork {
                pg_path: "Library/OPL/select-one.pg".to_string(),
            },
            prompt: Vec::new(),
            response,
            attempt_policy: AttemptPolicy {
                max_attempts: Some(2),
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Recorded OPL selection".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        }
    }

    async fn source(store: &MemoryObjectStore, question: &QuestionDefinition) -> WebworkSource {
        let artifact = SourceArtifact {
            object: ObjectId::from_uuid(Uuid::from_u128(4)),
            sha256: Sha256Digest::compute(OPL_FIXTURE.as_bytes()).to_string(),
        };
        store
            .put(PutObject {
                key: ObjectKey::ProblemSource {
                    problem: question.problem,
                    version: question.version,
                    object: artifact.object,
                },
                bytes: OPL_FIXTURE.as_bytes().to_vec(),
                media_type: "text/x-wework-pg".to_string(),
                license: "CC-BY-SA-4.0".to_string(),
                provenance: "recorded OPL fixture".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .expect("fixture source should be stored under its immutable key");
        WebworkSource::resolve(store, question.problem, question.version, artifact)
            .await
            .expect("fixture source should resolve through trusted storage")
    }

    #[tokio::test]
    async fn recorded_opl_fixture_renders_and_grades_through_the_shared_model() {
        let calls = Arc::new(AtomicUsize::new(0));
        let store = MemoryObjectStore::default();
        let adapter = WebworkAdapter::new(store.clone(), recorded_renderer(calls.clone()));
        let question = question_with_response(fixture_response());
        let source = source(&store, &question).await;
        let issued = adapter
            .issue(
                &question,
                Seed::new(17),
                &source,
                ActivityTimestamp::from_unix_millis(1),
            )
            .await
            .expect("recorded OPL fixture should render");
        assert!(!issued.cache_hit);
        assert_eq!(issued.envelope.seed, Seed::new(17));
        assert_eq!(issued.envelope.title, question.metadata.title);
        assert_ne!(issued.envelope.title, "Untrusted renderer title");
        assert!(
            !serde_json::to_string(&issued.envelope)
                .expect("browser envelope serializes")
                .contains("\"correct\"")
        );

        let correct = adapter
            .grade(
                &question,
                Seed::new(17),
                &source,
                &StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("water")],
                },
            )
            .await
            .expect("renderer should grade server-side");
        assert!(matches!(correct, GradeOutcome::Graded(result) if result.correct));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn historical_invalid_title_is_refused_before_cache_or_renderer() {
        let calls = Arc::new(AtomicUsize::new(0));
        let store = MemoryObjectStore::default();
        let adapter = WebworkAdapter::new(store.clone(), recorded_renderer(calls.clone()));
        let mut question = question_with_response(fixture_response());
        question.metadata.title = "\u{1F9EC}".repeat(513);
        let source = source(&store, &question).await;
        assert!(matches!(
            adapter
                .issue(
                    &question,
                    Seed::new(17),
                    &source,
                    ActivityTimestamp::from_unix_millis(1),
                )
                .await,
            Err(WebworkAdapterError::InvalidTitle(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn repeated_version_and_seed_are_served_without_a_renderer_call() {
        let calls = Arc::new(AtomicUsize::new(0));
        let store = MemoryObjectStore::default();
        let adapter = WebworkAdapter::new(store.clone(), recorded_renderer(calls.clone()));
        let question = question_with_response(fixture_response());
        let source = source(&store, &question).await;
        let first = adapter
            .issue(
                &question,
                Seed::new(18),
                &source,
                ActivityTimestamp::from_unix_millis(1),
            )
            .await
            .expect("first render should fill the cache");
        let second = adapter
            .issue(
                &question,
                Seed::new(18),
                &source,
                ActivityTimestamp::from_unix_millis(2),
            )
            .await
            .expect("second request should use the cache");
        assert!(!first.cache_hit);
        assert!(second.cache_hit);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(first.envelope, second.envelope);
        assert_eq!(second.envelope.title, question.metadata.title);
    }

    #[tokio::test]
    async fn renderer_outage_is_an_explicit_backend_local_failure() {
        let calls = Arc::new(AtomicUsize::new(0));
        let store = MemoryObjectStore::default();
        let adapter = WebworkAdapter::new(
            store.clone(),
            RecordedRenderer {
                failure: Some(RendererFailure::TimedOut),
                ..recorded_renderer(calls.clone())
            },
        );
        let question = question_with_response(fixture_response());
        let source = source(&store, &question).await;
        assert_eq!(
            adapter
                .issue(
                    &question,
                    Seed::new(19),
                    &source,
                    ActivityTimestamp::from_unix_millis(1),
                )
                .await,
            Err(WebworkAdapterError::Renderer(RendererFailure::TimedOut))
        );
        assert!(
            adapter
                .capabilities(&question.source)
                .expect("WeBWorK capability declaration remains available")
                .supports(Capability::ServerGrading)
        );
    }

    #[tokio::test]
    async fn renderer_markup_is_sanitized_before_cache_or_issued_envelope() {
        let calls = Arc::new(AtomicUsize::new(0));
        let store = MemoryObjectStore::default();
        let adapter = WebworkAdapter::new(
            store.clone(),
            RecordedRenderer {
                html: r#"<p onclick="steal()">Prompt</p><script>alert(1)</script><img src="javascript:alert(1)" onerror="steal()"><img src="/api/assets/asset-1">"#.to_string(),
                ..recorded_renderer(calls.clone())
            },
        );
        let question = question_with_response(fixture_response());
        let source = source(&store, &question).await;
        let issued = adapter
            .issue(
                &question,
                Seed::new(20),
                &source,
                ActivityTimestamp::from_unix_millis(1),
            )
            .await
            .expect("untrusted renderer output should be sanitized server-side");
        assert_eq!(
            issued.sanitized_html,
            r#"<p>Prompt</p><img><img src="/api/assets/asset-1">"#
        );
        assert!(!issued.sanitized_html.contains("script"));
        assert!(!issued.sanitized_html.contains("javascript:"));
        assert!(!issued.sanitized_html.contains("onerror"));
        let cached = adapter
            .issue(
                &question,
                Seed::new(20),
                &source,
                ActivityTimestamp::from_unix_millis(2),
            )
            .await
            .expect("cache hit should retain already-sanitized markup");
        assert!(cached.cache_hit);
        assert_eq!(cached.sanitized_html, issued.sanitized_html);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cache_hit_keeps_the_renderer_that_produced_cached_bytes() {
        let store = MemoryObjectStore::default();
        let question = question_with_response(fixture_response());
        let source = source(&store, &question).await;
        let first_calls = Arc::new(AtomicUsize::new(0));
        let first_renderer = RecordedRenderer {
            identity: RendererIdentity {
                id: "renderer-a".to_string(),
                version: "1".to_string(),
            },
            ..recorded_renderer(first_calls.clone())
        };
        let first_adapter = WebworkAdapter::new(store.clone(), first_renderer);
        let first = first_adapter
            .issue(
                &question,
                Seed::new(21),
                &source,
                ActivityTimestamp::from_unix_millis(1),
            )
            .await
            .expect("first renderer should populate cache");

        let second_calls = Arc::new(AtomicUsize::new(0));
        let second_renderer = RecordedRenderer {
            identity: RendererIdentity {
                id: "renderer-b".to_string(),
                version: "2".to_string(),
            },
            ..recorded_renderer(second_calls.clone())
        };
        let second_adapter = WebworkAdapter::new(store, second_renderer);
        let cached = second_adapter
            .issue(
                &question,
                Seed::new(21),
                &source,
                ActivityTimestamp::from_unix_millis(2),
            )
            .await
            .expect("cache hit should not call a newly deployed renderer");
        assert!(!first.cache_hit);
        assert!(cached.cache_hit);
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
        assert_eq!(cached.provenance.renderer, first.provenance.renderer);
        assert_eq!(
            cached.provenance.renderer.expect("renderer").id,
            "renderer-a"
        );
    }

    #[tokio::test]
    async fn source_resolution_refuses_digest_and_published_key_mismatches() {
        let store = MemoryObjectStore::default();
        let question = question_with_response(fixture_response());
        let trusted = source(&store, &question).await;
        let wrong_digest = SourceArtifact {
            object: trusted.artifact().object,
            sha256: "00".repeat(32),
        };
        assert_eq!(
            WebworkSource::resolve(&store, question.problem, question.version, wrong_digest,).await,
            Err(WebworkAdapterError::UntrustedSource)
        );
        assert_eq!(
            WebworkSource::resolve(
                &store,
                ProblemId::from_uuid(Uuid::from_u128(99)),
                question.version,
                trusted.artifact().clone(),
            )
            .await,
            Err(WebworkAdapterError::ObjectStore(ObjectStoreError::NotFound))
        );
    }

    #[tokio::test]
    async fn source_from_another_published_question_is_refused_before_renderer_or_cache() {
        let store = MemoryObjectStore::default();
        let question = question_with_response(fixture_response());
        let foreign_question = QuestionDefinition {
            problem: ProblemId::from_uuid(Uuid::from_u128(101)),
            version: VersionId::from_uuid(Uuid::from_u128(102)),
            ..question.clone()
        };
        let foreign_source = source(&store, &foreign_question).await;
        let calls = Arc::new(AtomicUsize::new(0));
        let adapter = WebworkAdapter::new(store, recorded_renderer(calls.clone()));

        assert_eq!(
            adapter
                .issue(
                    &question,
                    Seed::new(22),
                    &foreign_source,
                    ActivityTimestamp::from_unix_millis(1),
                )
                .await,
            Err(WebworkAdapterError::SourceDoesNotMatchQuestion)
        );
        assert_eq!(
            adapter
                .grade(
                    &question,
                    Seed::new(22),
                    &foreign_source,
                    &StudentResponse::MultipleChoice {
                        selected: vec![ChoiceId::new("water")],
                    },
                )
                .await,
            Err(WebworkAdapterError::SourceDoesNotMatchQuestion)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn partial_credit_is_not_claimed_without_per_source_evidence() {
        let adapter = WebworkAdapter::new(
            MemoryObjectStore::default(),
            recorded_renderer(Arc::new(AtomicUsize::new(0))),
        );
        let source = QuestionSource::Webwork {
            pg_path: "Library/OPL/select-one.pg".to_string(),
        };
        assert!(
            !adapter
                .capabilities(&source)
                .expect("WeBWorK source is supported")
                .supports(Capability::PartialCredit)
        );
    }

    fn fixture_response() -> ResponseDefinition {
        ResponseDefinition::MultipleChoice {
            choices: vec![
                ChoiceOption {
                    id: ChoiceId::new("water"),
                    body: vec![ContentBlock::Text {
                        markdown: "H&#x2082;O".to_string(),
                    }],
                },
                ChoiceOption {
                    id: ChoiceId::new("oxygen"),
                    body: vec![ContentBlock::Text {
                        markdown: "O&#x2082;".to_string(),
                    }],
                },
            ],
            selection: SelectionCardinality::ExactlyOne,
        }
    }
}
