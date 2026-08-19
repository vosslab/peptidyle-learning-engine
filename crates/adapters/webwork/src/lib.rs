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
/// Fixed endpoint facts for the external standalone renderer.
pub(crate) mod standalone_render_api;

#[path = "lib/artifact.rs"]
mod artifact;
#[path = "lib/cache.rs"]
mod cache;
#[path = "lib/grade.rs"]
mod grade;
#[path = "lib/issue.rs"]
mod issue;
#[path = "lib/source_profile.rs"]
mod source_profile;

pub use crate::http_renderer::{
    HttpWebworkRenderer, HttpWebworkRendererConfig, RendererConfigError,
};
pub use artifact::WebworkSource;
pub use issue::{
    WebworkAdapter, WebworkAdapterError, WebworkIssuedAttempt,
    reviewed_webwork_source_capabilities, reviewed_webwork_source_profile_capabilities,
    webwork_source_capabilities,
};
