//! WeBWorK PG Question Backend adapter and isolated renderer boundary.
//!
//! Public callers use this deliberately small facade. Capability modules keep
//! trusted Source Object Reference resolution, issue, and grading details
//! private to the adapter implementation.

/// Bounded, deployment-configured private HTTP client for a renderer service.
pub mod http_renderer;
/// PG source handling and the isolated renderer client contract.
pub mod renderer_contract;
/// Fixed endpoint facts for the external standalone renderer.
pub(crate) mod standalone_render_api;

#[path = "lib/grade.rs"]
mod grade;
#[path = "lib/issue.rs"]
mod issue;
#[path = "lib/source_object_reference.rs"]
mod source_object_reference;

pub use crate::http_renderer::{
    HttpWebworkRenderer, HttpWebworkRendererConfig, RendererConfigError,
};
pub use issue::{
    WebworkAdapter, WebworkAdapterError, WebworkIssuedAttempt, webwork_source_capabilities,
};
pub use source_object_reference::{ResolvedWebworkQuestionSource, WebworkQuestionSourceBinding};
