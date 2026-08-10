//! Fixed upstream endpoint details shared by the private WeBWorK client.
//!
//! Keeping the upstream name here makes it impossible for another adapter
//! module to quietly revive the old PLE-invented `/v1` renderer dialect.

/// The only upstream RPC path used by the version-1 WeBWorK adapter.
pub(crate) const PATH: &str = "render_rpc";
/// The request media type accepted by upstream `RenderViaRPC`.
pub(crate) const FORM_MEDIA_TYPE: &str = "application/x-www-form-urlencoded";
