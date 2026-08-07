//! MOD-WASM: the browser-facing bridge.
//!
//! Every export delegates to `domain`, which keeps parameter generation,
//! format validation, and timer verdicts identical on both targets. The
//! allowlist of exports is frozen in M1; `grading` is outside this crate's
//! dependency closure and must stay there.

use wasm_bindgen::prelude::wasm_bindgen;

/// Returns the bridge version string.
///
/// This is the trivial export WP-F2 calls from Node to prove the
/// `wasm-bindgen` toolchain path works end to end.
///
/// # Examples
///
/// ```
/// assert_eq!(wasm_bridge::bridge_version(), env!("CARGO_PKG_VERSION"));
/// ```
#[wasm_bindgen]
pub fn bridge_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
