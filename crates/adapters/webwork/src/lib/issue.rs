//! Issue and cache composition for the WeBWorK PG adapter.
//!
//! WeBWorK execution remains in a separate, non-public renderer service.  This
//! crate turns its output into the shared question model, caches only
//! browser-safe rendered output by immutable `(version, seed)`, and delegates
//! grading back to that service.  Answer material never enters this crate's
//! public results or the browser cache.

use objects::{ObjectStore, ObjectStoreError, PutObject};
use question_model::capability::{BackendCapabilities, Capability};
use question_model::generation::Seed;
use question_model::{
    ActivityTimestamp, AttemptProvenance, ImplementationVersion, QuestionDefinition,
    QuestionEnvelope, QuestionSource, QuestionTitleError, StudentResponse,
};
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::cell::RefCell;

use crate::artifact::WebworkSource;
use crate::renderer_contract::{
    RenderRequest, RendererFailure, WebworkRenderer, WebworkReplayMappingV1,
};
use crate::sanitizer::sanitize_webwork_html;

/// Stable adapter identifier recorded for WeBWorK attempts.
pub const ADAPTER_ID: &str = "webwork-adapter";
/// Current adapter compatibility implementation version.
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
    pub envelope: QuestionEnvelope,
    /// Sanitized supplied markup for the dedicated renderer component.
    pub sanitized_html: String,
    /// Deterministic parameter record for the version/seed pair.
    pub parameter_hash: String,
    /// Immutable source, implementation, and rendered-output evidence.
    pub provenance: AttemptProvenance,
    /// Private field/value mapping captured from the exact trusted render.
    /// It is persisted under the attempt's tenant boundary and is never part
    /// of the browser envelope or safe render cache.
    pub replay: Option<WebworkReplayMappingV1>,
    /// Whether this response came from object storage rather than the renderer.
    pub cache_hit: bool,
}

impl std::fmt::Debug for WebworkIssuedAttempt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebworkIssuedAttempt")
            .field("envelope", &self.envelope)
            .field("sanitized_html", &self.sanitized_html)
            .field("parameter_hash", &self.parameter_hash)
            .field("provenance", &self.provenance)
            .field("replay", &self.replay.as_ref().map(|_| "[REDACTED]"))
            .field("cache_hit", &self.cache_hit)
            .finish()
    }
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

/// Returns the conservative capabilities common to arbitrary PG sources.
pub fn webwork_source_capabilities(
    source: &QuestionSource,
) -> Result<BackendCapabilities, WebworkAdapterError> {
    let QuestionSource::Webwork { .. } = source else {
        return Err(WebworkAdapterError::UnsupportedSource);
    };
    Ok(BackendCapabilities::from_iter([
        Capability::AlgorithmicGeneration,
        Capability::ServerGrading,
    ]))
}

/// Returns capabilities proven for an exact immutable PG source artifact.
pub fn reviewed_webwork_source_capabilities(
    source: &QuestionSource,
    source_sha256: &str,
) -> Result<BackendCapabilities, WebworkAdapterError> {
    let QuestionSource::Webwork { pg_path } = source else {
        return Err(WebworkAdapterError::UnsupportedSource);
    };
    let mut capabilities = vec![Capability::AlgorithmicGeneration, Capability::ServerGrading];
    if crate::source_profile::supports_partial_credit(pg_path, source_sha256) {
        capabilities.push(Capability::PartialCredit);
    }
    Ok(BackendCapabilities::from_iter(capabilities))
}

/// Returns reviewed capabilities for an exact immutable source profile.
///
/// The capability describes whether the source can produce teaching feedback;
/// assignment-owned learner disclosure controls when that content is shown.
pub fn reviewed_webwork_source_profile_capabilities(
    source: &QuestionSource,
    source_sha256: &str,
) -> Result<BackendCapabilities, WebworkAdapterError> {
    let QuestionSource::Webwork { pg_path } = source else {
        return Err(WebworkAdapterError::UnsupportedSource);
    };
    let mut capabilities = reviewed_webwork_source_capabilities(source, source_sha256)?;
    if crate::source_profile::supports_immediate_correctness(pg_path, source_sha256) {
        capabilities =
            BackendCapabilities::from_iter(capabilities.declared().chain([Capability::Hints]));
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
    pub fn renderer_identity(&self) -> &crate::renderer_contract::RendererIdentity {
        self.renderer.identity()
    }

    /// Returns the evidence-bounded capabilities of this exact PG source.
    pub fn capabilities(
        &self,
        source: &QuestionSource,
    ) -> Result<BackendCapabilities, WebworkAdapterError> {
        webwork_source_capabilities(source)
    }

    /// Issues a browser-safe render, consulting the immutable `(version, seed)` cache first.
    ///
    /// The cache stores no grading material. A cache miss renders once and
    /// writes immutable safe bytes. A cache hit still renders once to capture
    /// and verify the newly issued attempt's private replay mapping; that call
    /// must reproduce the cached safe output exactly. Persisted attempt GETs
    /// are owned by the server snapshot path and do not invoke this method.
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
        let (problem, pg_path) = crate::artifact::webwork_identity(question)?;
        crate::artifact::verify_source(source)?;
        crate::artifact::verify_source_binding(source, problem, question.version)?;
        let cache_key = crate::cache::render_key(problem, question.version, seed);
        match self.store.get(&cache_key).await {
            Ok(stored) => {
                let cached = crate::cache::decode_render(&stored.bytes)?;
                crate::cache::validate_cached(
                    &cached,
                    question.version,
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
                let version = question.version.to_string();
                cache_witness("renderer_call");
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
                crate::cache::validate_envelope(&untrusted.envelope, question.version, seed)?;
                // Renderer output is untrusted. The learner title is durable
                // published metadata, not a renderer/source-provided field.
                untrusted.envelope.title = question.metadata.title.clone();
                let replay = untrusted.replay.take().ok_or_else(|| {
                    WebworkAdapterError::InvalidRendererEnvelope(
                        "renderer omitted private replay mapping".to_string(),
                    )
                })?;
                let rendered = crate::cache::CachedWebworkRender {
                    schema_version: crate::cache::CACHE_SCHEMA_VERSION,
                    source_artifact: source.artifact.clone(),
                    rendered: crate::cache::SafeRenderedWebworkQuestion {
                        envelope: untrusted.envelope,
                        sanitized_html: sanitize_webwork_html(&untrusted.html),
                        renderer: untrusted.renderer,
                    },
                };
                crate::cache::validate_cached(
                    &rendered,
                    question.version,
                    seed,
                    source,
                    &question.metadata.title,
                    self.renderer.identity(),
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
                            question.version,
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
        question: &QuestionDefinition,
        seed: Seed,
        source: &WebworkSource,
        response: &StudentResponse,
        replay: &WebworkReplayMappingV1,
    ) -> Result<grading::GradeOutcome, WebworkAdapterError> {
        crate::grade::grade(&self.renderer, question, seed, source, response, replay).await
    }

    /// Reproduces only the browser-safe cached render for an existing attempt.
    /// Attempt-bound replay state is loaded separately from tenant storage.
    pub async fn reproduce(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        source: &WebworkSource,
    ) -> Result<WebworkIssuedAttempt, WebworkAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(WebworkAdapterError::InvalidTitle)?;
        let (problem, _) = crate::artifact::webwork_identity(question)?;
        crate::artifact::verify_source(source)?;
        crate::artifact::verify_source_binding(source, problem, question.version)?;
        let stored = self
            .store
            .get(&crate::cache::render_key(problem, question.version, seed))
            .await
            .map_err(WebworkAdapterError::ObjectStore)?;
        let cached = crate::cache::decode_render(&stored.bytes)?;
        crate::cache::validate_cached(
            &cached,
            question.version,
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
        question: &QuestionDefinition,
        seed: Seed,
        source: &WebworkSource,
        pg_path: &str,
        cached: &crate::cache::CachedWebworkRender,
    ) -> Result<WebworkReplayMappingV1, WebworkAdapterError> {
        let version = question.version.to_string();
        cache_witness("renderer_call");
        let mut rendered = self
            .renderer
            .render(RenderRequest {
                pg_source: &source.pg_source,
                pg_path,
                version: &version,
                seed: seed.value(),
            })
            .await
            .map_err(WebworkAdapterError::Renderer)?;
        crate::cache::validate_envelope(&rendered.envelope, question.version, seed)?;
        rendered.envelope.title = question.metadata.title.clone();
        let replay = rendered.replay.take().ok_or_else(|| {
            WebworkAdapterError::InvalidRendererEnvelope(
                "renderer omitted private replay mapping".to_string(),
            )
        })?;
        let reproduced = crate::cache::CachedWebworkRender {
            schema_version: crate::cache::CACHE_SCHEMA_VERSION,
            source_artifact: source.artifact.clone(),
            rendered: crate::cache::SafeRenderedWebworkQuestion {
                envelope: rendered.envelope,
                sanitized_html: sanitize_webwork_html(&rendered.html),
                renderer: rendered.renderer,
            },
        };
        if &reproduced != cached {
            return Err(WebworkAdapterError::InvalidRendererEnvelope(
                "renderer replay did not match the immutable safe cache".to_string(),
            ));
        }
        Ok(replay)
    }

    fn issued(
        &self,
        rendered: crate::cache::CachedWebworkRender,
        seed: Seed,
        source: &WebworkSource,
        replay: Option<WebworkReplayMappingV1>,
        cache_hit: bool,
    ) -> Result<WebworkIssuedAttempt, WebworkAdapterError> {
        let rendered_question_sha256 = crate::cache::rendered_hash(&rendered)?;
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
            replay,
            cache_hit,
        })
    }
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
    crate::cache::hex_digest(hash.finalize().as_slice())
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

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
