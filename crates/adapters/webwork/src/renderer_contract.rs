//! Private renderer transport and immutable per-attempt document evidence.
//!
//! This module intentionally does not parse or execute PG.  A constrained
//! renderer service owns that work. The adapter validates the private network
//! boundary and preserves the exact document and renderer version issued for
//! each attempt, so no PG process, answer key, or renderer credential can
//! reach a browser or the database.

use async_trait::async_trait;
use grading::QuestionGradingOutcome;
use question_model::{BackendOwnedLifecycleState, QuestionRendererVersion};
use sha2::{Digest, Sha256};

/// Untrusted result of rendering one PG question after envelope validation.
///
/// The document remains verbatim backend-owned bytes.  PLE neither parses PG
/// controls nor derives educational metadata from this value.
#[derive(Clone, PartialEq, Eq)]
pub struct RenderedWebworkQuestion {
    /// The bounded standalone renderer document that the attempt store serves.
    pub document: Vec<u8>,
    /// SHA-256 of exactly `document`, retained as an immutable audit fact.
    pub document_sha256: [u8; 32],
    /// The implementation that actually produced this particular render.
    ///
    /// This is preserved with the issued document, so historical attempt
    /// evidence is never relabelled after a renderer upgrade.
    pub renderer_version: QuestionRendererVersion,
    /// WeBWorK grading is stateless, so the shared slot is explicitly `None`.
    pub lifecycle_state: BackendOwnedLifecycleState,
}

impl RenderedWebworkQuestion {
    pub(crate) fn from_document(
        document: Vec<u8>,
        renderer_version: QuestionRendererVersion,
    ) -> Self {
        let document_sha256 = Sha256::digest(&document).into();
        Self {
            document,
            document_sha256,
            renderer_version,
            lifecycle_state: BackendOwnedLifecycleState::none(),
        }
    }
}

impl std::fmt::Debug for RenderedWebworkQuestion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RenderedWebworkQuestion")
            .field("document", &format_args!("[{} bytes]", self.document.len()))
            .field("document_sha256", &"[REDACTED]")
            .field("renderer_version", &self.renderer_version)
            .field(
                "lifecycle_state",
                &self.lifecycle_state.as_deref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

/// One bounded renderer failure that an Assessment Attempt route can expose as a WeBWorK-only
/// degraded state.  No renderer implementation detail or answer data leaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RendererFailure {
    /// The isolated service cannot currently be reached.
    Unavailable,
    /// The request exceeded its configured deadline.
    TimedOut,
    /// The renderer rejected the request because its bounded resources were exhausted.
    ResourceExhausted,
    /// The renderer returned malformed or unsafe output.
    InvalidOutput(String),
}

impl std::fmt::Display for RendererFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("WeBWorK renderer is unavailable"),
            Self::TimedOut => formatter.write_str("WeBWorK renderer timed out"),
            Self::ResourceExhausted => formatter.write_str("WeBWorK renderer is at capacity"),
            Self::InvalidOutput(message) => {
                write!(formatter, "invalid WeBWorK renderer output: {message}")
            }
        }
    }
}

impl std::error::Error for RendererFailure {}

/// Narrow service boundary for an isolated, non-public PG renderer.
///
/// Implementations must enforce request deadline, CPU, and memory limits at
/// the renderer boundary.  The renderer is given an immutable source object
/// reference by the trusted server; neither browser requests nor database
/// credentials cross this trait.
#[async_trait]
pub trait WebworkRenderer: Send + Sync {
    /// Returns the deployment identity that this client will use for render
    /// and grade requests.
    fn identity(&self) -> &QuestionRendererVersion;

    /// Renders an immutable source/version/seed into browser-safe output,
    /// including the renderer identity that produced this exact response.
    async fn render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure>;

    /// Re-renders a Question Backend document with its previously saved opaque
    /// response. This is presentation-only: it does not submit or create a
    /// durable grading outcome.
    async fn render_saved_response(
        &self,
        request: ResumeRenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure>;

    /// Grades a structurally valid student response without returning a key.
    async fn grade(
        &self,
        request: GradeRequest<'_>,
    ) -> Result<QuestionGradingOutcome, RendererFailure>;
}

/// Trusted render request assembled only by the server adapter.
#[derive(Debug, Clone, Copy)]
pub struct RenderRequest<'a> {
    /// Immutable PG source bytes, resolved by the server from object storage.
    pub pg_source: &'a [u8],
    /// OPL-style PG location retained for renderer diagnostics.
    pub pg_path: &'a str,
    /// Exact immutable Question Revision selected by the server.
    pub question_revision: &'a question_model::QuestionRevisionReference,
    /// Deterministic attempt seed.
    pub seed: u64,
}

/// Trusted render request for reopening an in-progress backend-owned response.
///
/// The response remains canonical opaque `[name, value]` pairs. Only the
/// backend adapter validates and forwards those pairs; PLE does not project
/// them into control-specific state.
#[derive(Clone, Copy)]
pub struct ResumeRenderRequest<'a> {
    /// Immutable PG source bytes, resolved by the server from object storage.
    pub pg_source: &'a [u8],
    /// OPL-style PG location retained for renderer diagnostics.
    pub pg_path: &'a str,
    /// Exact immutable Question Revision selected by the server.
    pub question_revision: &'a question_model::QuestionRevisionReference,
    /// Deterministic attempt seed.
    pub seed: u64,
    /// Canonical bounded JSON `[name, value]` pairs captured from the backend document.
    pub response_payload: &'a [u8],
}

/// Trusted server-only grading request.
#[derive(Clone)]
pub struct GradeRequest<'a> {
    /// Immutable PG source bytes, resolved by the server from object storage.
    pub pg_source: &'a [u8],
    /// OPL-style PG location retained for renderer diagnostics.
    pub pg_path: &'a str,
    /// Exact immutable Question Revision selected by the server.
    pub question_revision: &'a question_model::QuestionRevisionReference,
    /// Deterministic attempt seed.
    pub seed: u64,
    /// Canonical bounded JSON `[name, value]` pairs from the backend document.
    pub response_payload: &'a [u8],
    /// WeBWorK grading is stateless; supplied for the shared lifecycle contract.
    pub lifecycle_state: &'a BackendOwnedLifecycleState,
}
