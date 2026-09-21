//! Issue lifecycle for the opaque WeBWorK PG adapter.

use objects::ObjectStoreError;
use question_model::capability::{Capability, QuestionBackendCapabilities};
use question_model::generation::QuestionSeed;
use question_model::{
    BackendOwnedLifecycleState, QuestionAttemptReproductionDetails, QuestionBackend,
    QuestionBackendVersion, QuestionGraderVersion, QuestionRendererVersion, QuestionResponseFormat,
    QuestionVariation, QuestionVariationPresentation, StudentResponse,
};
use sha2::{Digest, Sha256};

use crate::renderer_contract::{
    RenderRequest, RenderedWebworkQuestion, RendererFailure, ResumeRenderRequest, WebworkRenderer,
};
use crate::source_object_id::ResolvedWebworkQuestionSource;

pub const ADAPTER_ID: &str = "webwork-adapter";
pub const ADAPTER_VERSION: &str = "1";
pub const GRADING_ID: &str = "webwork-renderer-grader";

/// Issuance facts which the server persists with the Question Attempt.
#[derive(Clone, PartialEq)]
pub struct WebworkIssuedAttempt {
    /// Marker that tells PLE to present the separately stored backend document.
    pub presentation: QuestionVariationPresentation,
    pub parameter_hash: String,
    pub reproduction_details: QuestionAttemptReproductionDetails,
    /// Exact backend-owned document to persist for this issued attempt.
    pub document: Vec<u8>,
    /// SHA-256 of exactly `document`.
    pub document_sha256: [u8; 32],
    /// WeBWorK grading is stateless, so lifecycle state is always absent.
    pub lifecycle_state: BackendOwnedLifecycleState,
}

impl std::fmt::Debug for WebworkIssuedAttempt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebworkIssuedAttempt")
            .field("presentation", &self.presentation)
            .field("parameter_hash", &self.parameter_hash)
            .field("reproduction_details", &self.reproduction_details)
            .field("document", &format_args!("[{} bytes]", self.document.len()))
            .field("document_sha256", &"[REDACTED]")
            .field(
                "lifecycle_state",
                &self.lifecycle_state.as_deref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebworkAdapterError {
    UnsupportedSource,
    SourceChecksumMismatch,
    UntrustedSource,
    InvalidPgPath,
    /// Immutable source resolution failed at its object-store boundary.
    ObjectStore(ObjectStoreError),
    Renderer(RendererFailure),
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
            Self::InvalidPgPath => formatter.write_str("WeBWorK PG path is invalid"),
            Self::ObjectStore(error) => error.fmt(formatter),
            Self::Renderer(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WebworkAdapterError {}

/// Returns the capabilities common to every opaque WeBWorK question.
pub fn webwork_source_capabilities(
    question_backend: QuestionBackend,
) -> Result<QuestionBackendCapabilities, WebworkAdapterError> {
    if question_backend != QuestionBackend::Webwork {
        return Err(WebworkAdapterError::UnsupportedSource);
    }
    Ok(QuestionBackendCapabilities::from_iter([
        Capability::AlgorithmicGeneration,
        Capability::ServerGrading,
        Capability::PartialCredit,
    ]))
}

/// Opaque renderer adapter.
pub struct WebworkAdapter<R> {
    renderer: R,
}

impl<R: WebworkRenderer> WebworkAdapter<R> {
    pub fn new(renderer: R) -> Self {
        Self { renderer }
    }

    pub fn renderer_version(&self) -> &QuestionRendererVersion {
        self.renderer.identity()
    }

    pub fn capabilities(
        &self,
        source: &ResolvedWebworkQuestionSource,
    ) -> Result<QuestionBackendCapabilities, WebworkAdapterError> {
        let _ = source;
        webwork_source_capabilities(QuestionBackend::Webwork)
    }

    /// Renders exactly once and returns the opaque document for attempt persistence.
    pub async fn issue(
        &self,
        question_seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
    ) -> Result<WebworkIssuedAttempt, WebworkAdapterError> {
        let rendered = self.render(question_seed, source).await?;
        Ok(issued(rendered, question_seed, source))
    }

    /// Produces one answer-free, pre-submission opaque document without constructing
    /// Attempt reproduction, grading, lifecycle, or persistence values.
    pub async fn preview_document(
        &self,
        question_seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
    ) -> Result<Vec<u8>, WebworkAdapterError> {
        // ASVS 8.2.3: only the backend-owned browser document crosses this
        // preview boundary; source and Attempt-shaped evidence stay private.
        Ok(self.render(question_seed, source).await?.document)
    }

    /// Produces transient backend-owned correct answers after the caller authorizes disclosure.
    pub async fn answer_review_document(
        &self,
        question_seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
    ) -> Result<Vec<u8>, WebworkAdapterError> {
        crate::source_object_id::verify_source(source)?;
        let rendered = self
            .renderer
            .render_answer_review(RenderRequest {
                pg_source: source.pg_source(),
                pg_path: source.pg_path(),
                published_question_revision_tuple: source.published_question_revision_tuple(),
                seed: question_seed.value(),
            })
            .await
            .map_err(WebworkAdapterError::Renderer)?;
        if rendered.lifecycle_state.as_deref().is_some() {
            return Err(WebworkAdapterError::Renderer(
                RendererFailure::InvalidOutput(
                    "WeBWorK answer review returned unexpected lifecycle state".into(),
                ),
            ));
        }
        Ok(rendered.document)
    }

    async fn render(
        &self,
        question_seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
    ) -> Result<RenderedWebworkQuestion, WebworkAdapterError> {
        crate::source_object_id::verify_source(source)?;
        let rendered = self
            .renderer
            .render(RenderRequest {
                pg_source: source.pg_source(),
                pg_path: source.pg_path(),
                published_question_revision_tuple: source.published_question_revision_tuple(),
                seed: question_seed.value(),
            })
            .await
            .map_err(WebworkAdapterError::Renderer)?;
        if rendered.lifecycle_state.as_deref().is_some() {
            return Err(WebworkAdapterError::Renderer(
                RendererFailure::InvalidOutput(
                    "WeBWorK renderer returned unexpected lifecycle state; this integration grades one submission without renderer-issued state"
                        .to_string(),
                ),
            ));
        }
        Ok(rendered)
    }

    /// Delegates an opaque browser payload to the renderer once.
    pub async fn grade(
        &self,
        question_seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
        response: &StudentResponse,
    ) -> Result<grading::QuestionGradingOutcome, WebworkAdapterError> {
        crate::grade::grade(&self.renderer, question_seed, source, response).await
    }

    /// Re-renders an active saved backend response for presentation only.
    ///
    /// This does not alter the immutable issued document, persist a new
    /// document, submit the response, or create a grading outcome.
    pub async fn resume_document(
        &self,
        question_seed: QuestionSeed,
        source: &ResolvedWebworkQuestionSource,
        response: &StudentResponse,
    ) -> Result<Vec<u8>, WebworkAdapterError> {
        crate::source_object_id::verify_source(source)?;
        let StudentResponse::BackendOwned { payload } = response else {
            return Err(WebworkAdapterError::Renderer(
                RendererFailure::InvalidOutput("WeBWorK requires a backend-owned response".into()),
            ));
        };
        let rendered = self
            .renderer
            .render_saved_response(ResumeRenderRequest {
                pg_source: source.pg_source(),
                pg_path: source.pg_path(),
                published_question_revision_tuple: source.published_question_revision_tuple(),
                seed: question_seed.value(),
                response_payload: payload,
            })
            .await
            .map_err(WebworkAdapterError::Renderer)?;
        if rendered.lifecycle_state.as_deref().is_some() {
            return Err(WebworkAdapterError::Renderer(RendererFailure::InvalidOutput(
                "WeBWorK renderer returned unexpected lifecycle state; this integration resumes one stateless response"
                    .to_string(),
            )));
        }
        Ok(rendered.document)
    }
}

fn issued(
    rendered: RenderedWebworkQuestion,
    question_seed: QuestionSeed,
    source: &ResolvedWebworkQuestionSource,
) -> WebworkIssuedAttempt {
    let renderer_version = rendered.renderer_version;
    let grader = grader_version(GRADING_ID, &renderer_version.version);
    let generated_parameter_sha256 = parameter_hash(question_seed);
    WebworkIssuedAttempt {
        presentation: QuestionVariationPresentation {
            variation: QuestionVariation::from_question_revision_and_question_seed(
                source.published_question_revision_tuple().clone(),
                question_seed,
                generated_parameter_sha256.clone(),
            ),
            question_title: "WeBWorK question".to_string(),
            prompt: Vec::new(),
            response: QuestionResponseFormat::BackendOwned {},
            native_choice_order: question_model::NativeChoiceOrder::Fixed,
            author_content: None,
        },
        parameter_hash: generated_parameter_sha256,
        reproduction_details: QuestionAttemptReproductionDetails {
            backend: backend_version(ADAPTER_ID, ADAPTER_VERSION),
            renderer_version: Some(renderer_version),
            source_object_id: Some(*source.source_object_id()),
            source_object_checksum: Some(source.source_object_checksum().clone()),
            asset_objects: Vec::new(),
            grader,
            rendered_question_sha256: hex_digest(&rendered.document_sha256),
        },
        document: rendered.document,
        document_sha256: rendered.document_sha256,
        lifecycle_state: BackendOwnedLifecycleState::none(),
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

fn parameter_hash(question_seed: QuestionSeed) -> String {
    let mut hash = Sha256::new();
    hash.update(b"peptidyle:webwork-parameters:v1");
    hash.update(question_seed.value().to_be_bytes());
    hex_digest(&hash.finalize())
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
