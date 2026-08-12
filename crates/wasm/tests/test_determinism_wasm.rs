//! Headless-browser proof for portable Wasm behavior.

#![cfg(target_arch = "wasm32")]

#[path = "../../domain/tests/determinism_support.rs"]
mod determinism_support;

use wasm_bindgen_test::wasm_bindgen_test;

#[path = "flat_v2_response_corpus.rs"]
mod flat_v2_response_corpus;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn committed_seed_vectors_match_browser_generation() {
    determinism_support::assert_committed_seed_vectors();
}

#[wasm_bindgen_test]
fn flat_v2_public_response_corpus_matches_browser_wasm() {
    for case in flat_v2_response_corpus::cases() {
        let report = wasm_bridge::validate_response_format(
            &serde_json::to_string(&case.definition).expect("definition serializes"),
            &serde_json::to_string(&case.response).expect("response serializes"),
        )
        .expect("public fixture shape is valid");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&report).expect("report is JSON"),
            case.expected_report,
            "browser Wasm case {}",
            case.name
        );
    }

    let repeated = flat_v2_response_corpus::matching_full_permutation();
    let definition = serde_json::to_string(&repeated.definition).expect("definition serializes");
    let response = serde_json::to_string(&repeated.response).expect("response serializes");
    assert_eq!(
        wasm_bridge::validate_response_format(&definition, &response).expect("first call"),
        wasm_bridge::validate_response_format(&definition, &response).expect("second call"),
        "browser Wasm format validation must be stateless"
    );

    assert!(
        wasm_bridge::validate_response_format("{", "{}").is_err(),
        "malformed public JSON becomes a JavaScript-facing error"
    );
}
