//! Headless-browser half of the WP-C5 cross-target determinism gate.

#![cfg(target_arch = "wasm32")]

#[path = "../../domain/tests/determinism_support.rs"]
mod determinism_support;

use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn committed_seed_vectors_match_browser_generation() {
    determinism_support::assert_committed_seed_vectors();
}
