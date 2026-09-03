//! Headless-browser proof for portable Wasm behavior.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::wasm_bindgen_test;

#[path = "ple_question_json_response_format_fixture_set.rs"]
mod ple_question_json_response_format_fixture_set;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn ple_question_json_public_response_fixture_set_matches_browser_wasm() {
    for case in ple_question_json_response_format_fixture_set::cases() {
        let check = wasm_bridge::validate_response_format(
            &serde_json::to_string(&case.response_format).expect("response format serializes"),
            &serde_json::to_string(&case.response).expect("response serializes"),
        )
        .expect("public fixture shape is valid");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&check).expect("check is JSON"),
            case.expected_check,
            "browser Wasm case {}",
            case.name
        );
    }

    let repeated = ple_question_json_response_format_fixture_set::matching_full_permutation();
    let response_format =
        serde_json::to_string(&repeated.response_format).expect("response format serializes");
    let response = serde_json::to_string(&repeated.response).expect("response serializes");
    assert_eq!(
        wasm_bridge::validate_response_format(&response_format, &response).expect("first call"),
        wasm_bridge::validate_response_format(&response_format, &response).expect("second call"),
        "browser Wasm format validation must be stateless"
    );

    assert!(
        wasm_bridge::validate_response_format("{", "{}").is_err(),
        "malformed public JSON becomes a JavaScript-facing error"
    );
}
