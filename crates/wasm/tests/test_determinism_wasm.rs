//! Headless-browser proof for portable Wasm behavior.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn public_matching_responses_are_valid_blank_matching_is_allowed_and_calls_are_stateless() {
    let response_format = r#"{"kind":"matching","prompts":[{"id":"p","body":[{"kind":"text","markdown":"Prompt"}]}],"choices":[{"id":"c","body":[{"kind":"text","markdown":"Choice"}]}]}"#;
    let complete_response = r#"{"kind":"matching","matches":[{"prompt":"p","choice":"c"}]}"#;
    let blank_response = r#"{"kind":"matching","matches":[]}"#;

    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            &wasm_bridge::validate_response_format(response_format, complete_response)
                .expect("complete matching response is valid"),
        )
        .expect("complete response check is JSON"),
        serde_json::json!({"issues": []}),
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            &wasm_bridge::validate_response_format(
                r#"{"kind":"multipleChoice","choices":[{"id":"a","body":[{"kind":"text","markdown":"A"}]}],"selection":{"kind":"exactlyOne"}}"#,
                r#"{"kind":"multipleChoice","selected":[]}"#,
            )
            .expect("invalid choice count is a response issue"),
        )
        .expect("choice-count check is JSON"),
        serde_json::json!({
            "issues": [{
                "kind": "selectionCount",
                "expected": {"kind": "exactlyOne"},
                "actual": 0,
            }]
        }),
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            &wasm_bridge::validate_response_format(response_format, blank_response)
                .expect("empty matching response is valid"),
        )
        .expect("blank response check is JSON"),
        serde_json::json!({"issues": []}),
    );
    assert_eq!(
        wasm_bridge::validate_response_format(response_format, complete_response)
            .expect("first call"),
        wasm_bridge::validate_response_format(response_format, complete_response)
            .expect("second call"),
        "browser Wasm format validation must be stateless"
    );

    assert!(
        wasm_bridge::validate_response_format("{", "{}").is_err(),
        "malformed public JSON becomes a JavaScript-facing error"
    );
}
