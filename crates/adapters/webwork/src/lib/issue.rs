//! Issue and cache composition for the WeBWorK PG adapter.
//!
//! WeBWorK execution remains in a separate, non-public renderer service.  This
//! crate turns its output into the shared question model, caches only
//! browser-safe rendered output by immutable `(version, seed)`, and delegates
//! grading back to that service.  The Answer Key never enters this crate's
//! public results or the browser cache.

use objects::{ObjectStore, ObjectStoreError, PutObject};
use question_model::capability::{Capability, QuestionBackendCapabilities};
use question_model::generation::QuestionSeed;
use question_model::{
    QuestionAttemptReproductionDetails, QuestionBackend, QuestionBackendVersion,
    QuestionGraderVersion, QuestionRendererVersion, QuestionRevision, QuestionTitleError,
    QuestionVariationPresentation, StudentResponse, Timestamp,
};
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::cell::RefCell;

use crate::renderer_contract::{
    RenderRequest, RendererFailure, WebworkQuestionAttemptReplayDetails, WebworkRenderer,
};
use crate::source_object_reference::ResolvedWebworkQuestionSource;

/// Stable Question Backend identifier recorded for WeBWorK attempts.
pub const ADAPTER_ID: &str = "webwork-adapter";
/// Current Question Backend Version.
///
/// This is intentionally independent of the repository's CalVer release.
pub const ADAPTER_VERSION: &str = "1";
/// Stable identifier for renderer-owned grading.
pub const GRADING_ID: &str = "webwork-renderer-grader";

/// Emits one fixed, non-sensitive cache witness for the local-stack E2E.
/// The event name is the entire payload: request and content identifiers stay
/// out of operational logs.
fn cache_witness(event: &'static str) {
    tracing::info!(target: "ple.webwork.cache", event);
    #[cfg(test)]
    TEST_CACHE_EVENTS.with(|events| events.borrow_mut().push(event));
}

#[cfg(test)]
thread_local! {
    static TEST_CACHE_EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn take_test_cache_events() -> Vec<&'static str> {
    TEST_CACHE_EVENTS.with(|events| std::mem::take(&mut *events.borrow_mut()))
}

/// Key-free result returned when a WeBWorK question is issued.
#[derive(Clone, PartialEq)]
pub struct WebworkIssuedAttempt {
    /// Reusable browser-safe response contract and prompt blocks.
    pub presentation: QuestionVariationPresentation,
    /// Deterministic parameter record for the version/seed pair.
    pub parameter_hash: String,
    /// Immutable source, implementation, and rendered-output evidence.
    pub reproduction_details: QuestionAttemptReproductionDetails,
    /// Private field/value mapping captured from the exact trusted render.
    /// It is persisted under the attempt's course boundary and is never part
    /// of the browser Question Presentation or safe render cache.
    pub replay: Option<WebworkQuestionAttemptReplayDetails>,
    /// Whether this response came from object storage rather than the renderer.
    pub cache_hit: bool,
}

impl std::fmt::Debug for WebworkIssuedAttempt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebworkIssuedAttempt")
            .field("presentation", &self.presentation)
            .field("parameter_hash", &self.parameter_hash)
            .field("reproduction_details", &self.reproduction_details)
            .field("replay", &self.replay.as_ref().map(|_| "[REDACTED]"))
            .field("cache_hit", &self.cache_hit)
            .finish()
    }
}

/// Failures that are confined to this backend and can become a WeBWorK-only
/// degraded Assignment Attempt state at the HTTP boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebworkAdapterError {
    /// A non-WeBWorK question reached this adapter.
    UnsupportedSource,
    /// The trusted source bytes do not match the immutable Question Source.
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
    InvalidRendererQuestionPresentation(String),
    /// Persisted student-facing metadata cannot be delivered safely.
    InvalidTitle(QuestionTitleError),
}

impl std::fmt::Display for WebworkAdapterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSource => formatter.write_str("question source is not WeBWorK"),
            Self::SourceChecksumMismatch => {
                formatter.write_str("PG source bytes do not match Question Source")
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
            Self::InvalidRendererQuestionPresentation(message) => {
                write!(
                    formatter,
                    "invalid WeBWorK renderer Question Presentation: {message}"
                )
            }
            Self::InvalidTitle(error) => {
                write!(formatter, "invalid WeBWorK question title: {error}")
            }
        }
    }
}

impl std::error::Error for WebworkAdapterError {}

/// Returns the conservative capabilities common to arbitrary PG sources.
pub fn webwork_source_capabilities(
    question_backend: QuestionBackend,
) -> Result<QuestionBackendCapabilities, WebworkAdapterError> {
    if question_backend != QuestionBackend::Webwork {
        return Err(WebworkAdapterError::UnsupportedSource);
    }
    Ok(QuestionBackendCapabilities::from_iter([
        Capability::AlgorithmicGeneration,
        Capability::ServerGrading,
    ]))
}

/// Returns capabilities proven for an exact immutable PG Source Object Reference.
pub fn reviewed_webwork_source_capabilities(
    question_backend: QuestionBackend,
    webwork_pg_path: &str,
    source_sha256: &str,
) -> Result<QuestionBackendCapabilities, WebworkAdapterError> {
    if question_backend != QuestionBackend::Webwork {
        return Err(WebworkAdapterError::UnsupportedSource);
    }
    let mut capabilities = vec![Capability::AlgorithmicGeneration, Capability::ServerGrading];
    if crate::source_profile::supports_partial_credit(webwork_pg_path, source_sha256) {
        capabilities.push(Capability::PartialCredit);
    }
    Ok(QuestionBackendCapabilities::from_iter(capabilities))
}

/// Returns reviewed capabilities for an exact immutable source profile.
///
/// The capability describes whether the source can produce teaching feedback;
/// assignment-owned student disclosure controls when that content is shown.
pub fn reviewed_webwork_source_profile_capabilities(
    question_backend: QuestionBackend,
    webwork_pg_path: &str,
    source_sha256: &str,
) -> Result<QuestionBackendCapabilities, WebworkAdapterError> {
    if question_backend != QuestionBackend::Webwork {
        return Err(WebworkAdapterError::UnsupportedSource);
    }
    let mut capabilities =
        reviewed_webwork_source_capabilities(question_backend, webwork_pg_path, source_sha256)?;
    if crate::source_profile::supports_immediate_correctness(webwork_pg_path, source_sha256) {
        capabilities = QuestionBackendCapabilities::from_iter(
            capabilities.declared().chain([Capability::Hints]),
        );
    }
    Ok(capabilities)
}

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

    /// Returns the configured renderer identity used by this adapter.
    pub fn renderer_version(&self) -> &QuestionRendererVersion {
        self.renderer.identity()
    }

    /// Returns the evidence-bounded capabilities of this exact PG source.
    pub fn capabilities(
        &self,
        question: &QuestionRevision,
    ) -> Result<QuestionBackendCapabilities, WebworkAdapterError> {
        webwork_source_capabilities(question.question_backend)
    }

    /// Issues a browser-safe render, consulting the immutable `(version, seed)` cache first.
    ///
    /// The cache stores no Answer Key, Question Feedback, Question Answer
    /// Explanation, or Question Grading Input. A cache miss renders once and
    /// writes immutable safe bytes. A cache hit still renders once to capture
    /// and verify the newly issued attempt's private replay mapping; that call
    /// must reproduce the cached safe output exactly. Persisted attempt GETs
    /// are owned by the server snapshot path and do not invoke this method.
    pub async fn issue(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
        created_at: Timestamp,
    ) -> Result<WebworkIssuedAttempt, WebworkAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(WebworkAdapterError::InvalidTitle)?;
        let (question_revision, pg_path) =
            crate::source_object_reference::webwork_identity(question)?;
        crate::source_object_reference::verify_source(source)?;
        crate::source_object_reference::verify_source_binding(source, &question_revision)?;
        let cache_key = crate::cache::render_key(&question_revision, seed);
        match self.store.get(&cache_key).await {
            Ok(stored) => {
                let cached = crate::cache::decode_render(&stored.bytes)?;
                crate::cache::validate_cached(
                    &cached,
                    &question_revision,
                    seed,
                    source,
                    &question.metadata.title,
                    self.renderer.identity(),
                )?;
                cache_witness("cache_hit");
                let replay = self
                    .render_replay(question, seed, source, pg_path, &cached)
                    .await?;
                self.issued(cached, seed, source, Some(replay), true)
            }
            Err(ObjectStoreError::NotFound) => {
                cache_witness("renderer_call");
                let mut untrusted = self
                    .renderer
                    .render(RenderRequest {
                        pg_source: source.pg_source(),
                        pg_path,
                        question_revision: &question_revision,
                        seed: seed.value(),
                    })
                    .await
                    .map_err(WebworkAdapterError::Renderer)?;
                crate::cache::validate_presentation(
                    &untrusted.presentation,
                    &question_revision,
                    seed,
                )?;
                // Renderer output is untrusted. The student title is durable
                // published metadata, not a renderer/source-provided field.
                untrusted.presentation.title = question.metadata.title.clone();
                let replay = untrusted.replay.take().ok_or_else(|| {
                    WebworkAdapterError::InvalidRendererQuestionPresentation(
                        "renderer omitted private replay mapping".to_string(),
                    )
                })?;
                let rendered = crate::cache::CachedWebworkRender {
                    schema_version: crate::cache::CACHE_SCHEMA_VERSION,
                    source_object_reference: source.source_object_reference().clone(),
                    source_object_checksum: source.source_object_checksum().clone(),
                    rendered: crate::cache::SafeRenderedWebworkQuestion {
                        presentation: untrusted.presentation,
                        renderer_version: untrusted.renderer_version,
                    },
                };
                crate::cache::validate_cached(
                    &rendered,
                    &question_revision,
                    seed,
                    source,
                    &question.metadata.title,
                    self.renderer.identity(),
                )?;
                let bytes = serde_json::to_vec(&rendered).map_err(|error| {
                    WebworkAdapterError::InvalidRendererQuestionPresentation(error.to_string())
                })?;
                match self
                    .store
                    .put(PutObject {
                        address: cache_key.clone(),
                        bytes,
                        media_type: "application/json".to_string(),
                        created_at,
                    })
                    .await
                {
                    Ok(_) => self.issued(rendered, seed, source, Some(replay), false),
                    Err(ObjectStoreError::AlreadyExists) => {
                        let stored = self
                            .store
                            .get(&cache_key)
                            .await
                            .map_err(WebworkAdapterError::ObjectStore)?;
                        let cached = crate::cache::decode_render(&stored.bytes)?;
                        crate::cache::validate_cached(
                            &cached,
                            &question_revision,
                            seed,
                            source,
                            &question.metadata.title,
                            self.renderer.identity(),
                        )?;
                        cache_witness("cache_hit");
                        self.issued(cached, seed, source, Some(replay), true)
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
        question: &QuestionRevision,
        seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
        response: &StudentResponse,
        replay: &WebworkQuestionAttemptReplayDetails,
    ) -> Result<grading::QuestionGradingOutcome, WebworkAdapterError> {
        crate::grade::grade(&self.renderer, question, seed, source, response, replay).await
    }

    /// Reproduces only the browser-safe cached render for an existing attempt.
    /// Attempt-bound replay state is loaded separately from course storage.
    pub async fn reproduce(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
    ) -> Result<WebworkIssuedAttempt, WebworkAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(WebworkAdapterError::InvalidTitle)?;
        let (question_revision, _) = crate::source_object_reference::webwork_identity(question)?;
        crate::source_object_reference::verify_source(source)?;
        crate::source_object_reference::verify_source_binding(source, &question_revision)?;
        let stored = self
            .store
            .get(&crate::cache::render_key(&question_revision, seed))
            .await
            .map_err(WebworkAdapterError::ObjectStore)?;
        let cached = crate::cache::decode_render(&stored.bytes)?;
        crate::cache::validate_cached(
            &cached,
            &question_revision,
            seed,
            source,
            &question.metadata.title,
            self.renderer.identity(),
        )?;
        cache_witness("cache_hit");
        self.issued(cached, seed, source, None, true)
    }

    async fn render_replay(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
        pg_path: &str,
        cached: &crate::cache::CachedWebworkRender,
    ) -> Result<WebworkQuestionAttemptReplayDetails, WebworkAdapterError> {
        let (question_revision, _) = crate::source_object_reference::webwork_identity(question)?;
        cache_witness("renderer_call");
        let mut rendered = self
            .renderer
            .render(RenderRequest {
                pg_source: source.pg_source(),
                pg_path,
                question_revision: &question_revision,
                seed: seed.value(),
            })
            .await
            .map_err(WebworkAdapterError::Renderer)?;
        crate::cache::validate_presentation(&rendered.presentation, &question_revision, seed)?;
        rendered.presentation.title = question.metadata.title.clone();
        let replay = rendered.replay.take().ok_or_else(|| {
            WebworkAdapterError::InvalidRendererQuestionPresentation(
                "renderer omitted private replay mapping".to_string(),
            )
        })?;
        let reproduced = crate::cache::CachedWebworkRender {
            schema_version: crate::cache::CACHE_SCHEMA_VERSION,
            source_object_reference: source.source_object_reference().clone(),
            source_object_checksum: source.source_object_checksum().clone(),
            rendered: crate::cache::SafeRenderedWebworkQuestion {
                presentation: rendered.presentation,
                renderer_version: rendered.renderer_version,
            },
        };
        if &reproduced != cached {
            return Err(WebworkAdapterError::InvalidRendererQuestionPresentation(
                "renderer replay did not match the immutable safe cache".to_string(),
            ));
        }
        Ok(replay)
    }

    fn issued(
        &self,
        rendered: crate::cache::CachedWebworkRender,
        seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
        replay: Option<WebworkQuestionAttemptReplayDetails>,
        cache_hit: bool,
    ) -> Result<WebworkIssuedAttempt, WebworkAdapterError> {
        let rendered_question_sha256 = crate::cache::rendered_hash(&rendered)?;
        let renderer_version = rendered.rendered.renderer_version;
        let grader = grader_version(GRADING_ID, &renderer_version.version);
        Ok(WebworkIssuedAttempt {
            presentation: rendered.rendered.presentation,
            parameter_hash: parameter_hash(seed),
            reproduction_details: QuestionAttemptReproductionDetails {
                backend: backend_version(ADAPTER_ID, ADAPTER_VERSION),
                renderer_version: Some(renderer_version),
                source_object_reference: Some(source.source_object_reference().clone()),
                source_object_checksum: Some(source.source_object_checksum().clone()),
                asset_objects: Vec::new(),
                grader,
                rendered_question_sha256,
            },
            replay,
            cache_hit,
        })
    }
}

fn backend_version(name: &str, version: &str) -> QuestionBackendVersion {
    QuestionBackendVersion {
        name: name.to_string(),
        version: version.to_string(),
    }
}

fn grader_version(name: &str, version: &str) -> QuestionGraderVersion {
    QuestionGraderVersion {
        name: name.to_string(),
        version: version.to_string(),
    }
}

fn parameter_hash(seed: QuestionSeed) -> String {
    let mut hash = Sha256::new();
    hash.update(b"peptidyle:webwork-parameters:v1");
    hash.update(seed.value().to_be_bytes());
    crate::cache::hex_digest(hash.finalize().as_slice())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
