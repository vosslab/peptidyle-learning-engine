//! Native half of the ple-question-json-v2 public-response/Wasm-boundary parity gate.
//!
//! The shared Question Response Format Fixture Set deliberately contains only
//! compiled, answer-free Question Response Formats. Parsing source documents
//! or grading them here would pull the server-only PLE Question JSON adapter
//! and answer keys into the browser boundary.

use serde_json::Value;

#[path = "ple_question_json_response_format_fixture_set.rs"]
mod ple_question_json_response_format_fixture_set;

#[test]
fn ple_question_json_public_response_fixture_set_matches_native_bridge() {
    for case in ple_question_json_response_format_fixture_set::cases() {
        let check = wasm_bridge::validate_response_format(
            &serde_json::to_string(&case.response_format).expect("response format serializes"),
            &serde_json::to_string(&case.response).expect("response serializes"),
        )
        .expect("fixture has a valid public response shape");
        let actual: Value = serde_json::from_str(&check).expect("bridge check is JSON");
        assert_eq!(actual, case.expected_check, "native case {}", case.name);
    }
}

#[test]
fn ple_question_json_public_response_calls_are_repeatable_natively() {
    let case = ple_question_json_response_format_fixture_set::matching_full_permutation();
    let response_format =
        serde_json::to_string(&case.response_format).expect("response format serializes");
    let response = serde_json::to_string(&case.response).expect("response serializes");

    let first = wasm_bridge::validate_response_format(&response_format, &response)
        .expect("first bridge call succeeds");
    let second = wasm_bridge::validate_response_format(&response_format, &response)
        .expect("second bridge call succeeds");
    assert_eq!(first, second, "same public input must be stateless");
}
