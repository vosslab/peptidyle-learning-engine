//! Fixed endpoint details for the external standalone PG renderer.
//!
//! Keeping the upstream name here makes it impossible for another adapter
//! module to quietly revive either the old PLE-invented `/v1` dialect or the
//! stateful WebWork2 `render_rpc` dependency.

/// The only upstream path used by the version-1 WeBWorK adapter.
pub(crate) const PATH: &str = "render-api";
/// The request media type accepted by the standalone renderer.
pub(crate) const FORM_MEDIA_TYPE: &str = "application/x-www-form-urlencoded";
