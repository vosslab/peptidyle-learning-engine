//! MOD-WASM: the browser-facing bridge.
//!
//! Every export delegates to `domain`, which keeps parameter generation,
//! format validation, timer verdicts, and capability validation identical on
//! both targets. The allowlist of exports is frozen in M1; `grading` is outside
//! this crate's dependency closure and must stay there.

use domain::{policy, timing, validation};
use question_model::response::{ResponseDefinition, StudentResponse};
use wasm_bindgen::JsValue;
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

/// Validates response format without consulting an answer key.
///
/// Inputs and output use the same JSON shapes as the generated browser API.
/// Malformed JSON raises a JavaScript error; a well-formed but invalid student
/// response returns every safe format violation.
///
/// # Errors
///
/// Returns a JavaScript error when either input is malformed or the report
/// cannot be serialized.
#[wasm_bindgen]
pub fn validate_response_format(
    definition_json: &str,
    response_json: &str,
) -> Result<String, JsValue> {
    let definition: ResponseDefinition = serde_json::from_str(definition_json)
        .map_err(|error| JsValue::from_str(&format!("invalid response definition: {error}")))?;
    let response: StudentResponse = serde_json::from_str(response_json)
        .map_err(|error| JsValue::from_str(&format!("invalid student response: {error}")))?;
    let report = validation::validate_response_format(&definition, &response);
    serde_json::to_string(&report)
        .map_err(|error| JsValue::from_str(&format!("could not serialize format report: {error}")))
}

/// Evaluates one server-authored timer record without reading a browser clock.
///
/// Input and output use lower-camel JSON. The input carries the server's
/// evaluation timestamp and cumulative authorized pause extension; JavaScript
/// time is never consulted here.
///
/// # Errors
///
/// Returns a JavaScript error when the input is malformed, the timer record is
/// inconsistent, or the verdict cannot be serialized.
#[wasm_bindgen]
pub fn timer_verdict(evaluation_json: &str) -> Result<String, JsValue> {
    let evaluation: timing::TimerEvaluation = serde_json::from_str(evaluation_json)
        .map_err(|error| JsValue::from_str(&format!("invalid timer evaluation: {error}")))?;
    let verdict = timing::timer_verdict(&evaluation)
        .map_err(|error| JsValue::from_str(&format!("invalid timer evaluation: {error}")))?;
    serde_json::to_string(&verdict)
        .map_err(|error| JsValue::from_str(&format!("could not serialize timer verdict: {error}")))
}

/// Reports every backend capability missing from an assignment configuration.
///
/// The definition and capability inputs are browser-safe. The same function is
/// called by the server before publication, so editor hints and publish refusal
/// cannot drift.
///
/// # Errors
///
/// Returns a JavaScript error when the input is malformed or the violation
/// list cannot be serialized.
#[wasm_bindgen]
pub fn validate_assignment_config(config_json: &str) -> Result<String, JsValue> {
    let config: policy::AssignmentConfig = serde_json::from_str(config_json)
        .map_err(|error| JsValue::from_str(&format!("invalid assignment config: {error}")))?;
    let violations = policy::validate_assignment_config(&config);
    serde_json::to_string(&violations).map_err(|error| {
        JsValue::from_str(&format!(
            "could not serialize assignment capability violations: {error}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_validation_delegates_to_the_key_free_domain_module() {
        let report = validate_response_format(
            r#"{"kind":"shortText","matchMode":"normalized","maxLength":20}"#,
            r#"{"kind":"shortText","text":"peptide"}"#,
        )
        .expect("valid JSON should produce a report");

        assert_eq!(report, r#"{"violations":[]}"#);
    }

    #[test]
    fn timer_evaluation_delegates_to_the_clock_free_domain_module() {
        let verdict = timer_verdict(
            r#"{
                "policy":{"kind":"perQuestion","seconds":9,"graceSeconds":2},
                "timer":{"issuedAt":1000,"deadline":10000,"submittedAt":10500},
                "evaluatedAt":10500,
                "pauseExtensionMillis":0
            }"#,
        )
        .expect("valid server timestamps should produce a verdict");

        assert_eq!(verdict, r#""submittedWithinGrace""#);
    }
}
