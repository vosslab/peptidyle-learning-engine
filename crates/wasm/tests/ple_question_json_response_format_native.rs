//! Native checks for the public Question Response Format boundary.

#[test]
fn public_matching_responses_are_valid_blank_matching_is_allowed_and_calls_are_stateless() {
    let response_format = r#"{"kind":"matching","prompts":[{"id":"p","body":[{"kind":"text","markdown":"Prompt"}]}],"choices":[{"id":"c","body":[{"kind":"text","markdown":"Choice"}]}]}"#;
    let complete_response = r#"{"kind":"matching","matches":[{"prompt":"p","choice":"c"}]}"#;
    let blank_response = r#"{"kind":"matching","matches":[]}"#;

    let complete_check: serde_json::Value = serde_json::from_str(
        &wasm_bridge::validate_response_format(response_format, complete_response).unwrap(),
    )
    .unwrap();
    assert_eq!(complete_check, serde_json::json!({"issues": []}));
    let blank_check: serde_json::Value = serde_json::from_str(
        &wasm_bridge::validate_response_format(response_format, blank_response).unwrap(),
    )
    .unwrap();
    assert_eq!(blank_check, serde_json::json!({"issues": []}));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            &wasm_bridge::validate_response_format(
                r#"{"kind":"multipleChoice","choices":[{"id":"a","body":[{"kind":"text","markdown":"A"}]}],"selection":{"kind":"exactlyOne"}}"#,
                r#"{"kind":"multipleChoice","selected":[]}"#,
            )
            .unwrap(),
        )
        .unwrap(),
        serde_json::json!({
            "issues": [{
                "kind": "selectionCount",
                "expected": {"kind": "exactlyOne"},
                "actual": 0,
            }]
        })
    );
    let first = wasm_bridge::validate_response_format(response_format, complete_response).unwrap();
    let second = wasm_bridge::validate_response_format(response_format, complete_response).unwrap();
    assert_eq!(first, second, "same public input must be stateless");
}
