//! MOD-ADP-WW: WeBWorK PG adapter, isolated renderer boundary, and render cache.
//!
//! Public callers use this deliberately small facade. Capability modules keep
//! trusted artifact resolution, cache projection, issue, and grading details
//! private to the adapter implementation.

/// Bounded, deployment-configured private HTTP client for a renderer service.
pub mod http_renderer;
/// PG source handling and the isolated renderer client contract.
pub mod renderer_contract;
/// The server-side allowlist applied to untrusted renderer markup.
pub mod sanitizer;
/// Fixed upstream `/render_rpc` endpoint facts for the shipped client.
pub(crate) mod shipped_render_rpc;

#[path = "lib/artifact.rs"]
mod artifact;
#[path = "lib/cache.rs"]
mod cache;
#[path = "lib/grade.rs"]
mod grade;
#[path = "lib/issue.rs"]
mod issue;

pub use crate::http_renderer::{
    HttpWebworkRenderer, HttpWebworkRendererConfig, RendererConfigError,
};
pub use artifact::WebworkSource;
pub use issue::{WebworkAdapter, WebworkAdapterError, WebworkIssuedAttempt};
