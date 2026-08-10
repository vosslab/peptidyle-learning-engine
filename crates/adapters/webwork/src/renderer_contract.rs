//! Renderer-client and deterministic render-cache contract (MOD-ADP-WW).
//!
//! This module intentionally does not parse or execute PG.  A constrained
//! renderer service owns that work.  The adapter holds the network boundary,
//! cache key, and conversion to the shared question model so no PG process,
//! answer key, or renderer credential can reach a browser or the database.

use async_trait::async_trait;
use grading::GradeOutcome;
use question_model::{QuestionEnvelope, StudentResponse};
use serde::{Deserialize, Serialize};

/// Untrusted result of rendering one PG question.
///
/// The adapter sanitizes `html` before it reaches its render cache or a
/// browser-facing issued result.  The renderer is intentionally not trusted
/// to make that security decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedWebworkQuestion {
    /// Backend-neutral prompt and response definition safe for a browser.
    pub envelope: QuestionEnvelope,
    /// PG HTML supplied by the isolated renderer; it is still untrusted here.
    pub html: String,
    /// The implementation that actually produced this particular render.
    ///
    /// This is part of renderer output rather than sampled from a client on a
    /// cache hit, so historical output is never relabelled after an upgrade.
    pub renderer: RendererIdentity,
}

/// Stable renderer implementation identity recorded with an attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RendererIdentity {
    /// Renderer service implementation identifier.
    pub id: String,
    /// Additive renderer compatibility implementation version.
    pub version: String,
}

/// One bounded renderer failure that a run route can expose as a WeBWorK-only
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
    /// Renders an immutable source/version/seed into browser-safe output,
    /// including the renderer identity that produced this exact response.
    async fn render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure>;

    /// Grades a structurally valid student response without returning a key.
    async fn grade(&self, request: GradeRequest<'_>) -> Result<GradeOutcome, RendererFailure>;
}

/// Trusted render request assembled only by the server adapter.
#[derive(Debug, Clone, Copy)]
pub struct RenderRequest<'a> {
    /// Immutable PG source bytes, resolved by the server from object storage.
    pub pg_source: &'a [u8],
    /// OPL-style PG location retained for renderer diagnostics.
    pub pg_path: &'a str,
    /// Immutable published version as a string.
    pub version: &'a str,
    /// Deterministic attempt seed.
    pub seed: u64,
}

/// Trusted server-only grading request.
#[derive(Debug, Clone)]
pub struct GradeRequest<'a> {
    /// Immutable PG source bytes, resolved by the server from object storage.
    pub pg_source: &'a [u8],
    /// OPL-style PG location retained for renderer diagnostics.
    pub pg_path: &'a str,
    /// Immutable published version as a string.
    pub version: &'a str,
    /// Deterministic attempt seed.
    pub seed: u64,
    /// Browser-submitted response, never an answer key.
    pub response: &'a StudentResponse,
    /// The published all-or-nothing score ceiling. The renderer returns only
    /// a normalized score and never chooses the assignment's point value.
    pub points_possible: f64,
}
