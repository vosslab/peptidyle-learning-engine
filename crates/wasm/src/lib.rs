//! Browser-facing WebAssembly bridge.
//!
//! Every export delegates to `domain`, which keeps parameter generation,
//! format validation, timer verdicts, and capability validation identical on
//! both targets. The allowlist of exports is frozen in M1; `grading` is outside
//! this crate's dependency closure and must stay there.

use domain::{draft_preview, policy, timing, validation};
use question_model::presentation::{
    QuestionAssetRendition, QuestionPresentation, QuestionPresentationResponseFormat,
    QuestionPresentationToken, rebuild_public_question_presentation,
};
use question_model::response::{QuestionResponseFormat, StudentResponse};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// Returns the bridge version string.
///
/// The Node binding check calls this export to prove the `wasm-bindgen`
/// toolchain path works end to end.
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
/// response returns every safe format issue.
///
/// # Errors
///
/// Returns a JavaScript error when either input is malformed or the check
/// cannot be serialized.
#[wasm_bindgen]
pub fn validate_response_format(
    response_format_json: &str,
    response_json: &str,
) -> Result<String, JsValue> {
    let response_format: QuestionResponseFormat = serde_json::from_str(response_format_json)
        .map_err(|error| {
            JsValue::from_str(&format!("invalid Question Response Format: {error}"))
        })?;
    let response: StudentResponse = serde_json::from_str(response_json)
        .map_err(|error| JsValue::from_str(&format!("invalid student response: {error}")))?;
    // Typed domain deserialization keeps this browser boundary allow-listed and
    // answer-free before trusted validation evaluates the response shape
    // (ASVS 1.5.2, 2.2.1).
    let check = validation::validate_response_format(&response_format, &response);
    serde_json::to_string(&check).map_err(|error| {
        JsValue::from_str(&format!(
            "could not serialize Student Response Format Check: {error}"
        ))
    })
}

/// Validates one Student Response against the answer-free format frozen with an issued Question
/// Presentation.
///
/// # Errors
///
/// Returns a JavaScript error when either input is malformed or the check cannot be serialized.
#[wasm_bindgen]
pub fn validate_presentation_response_format(
    response_format_json: &str,
    response_json: &str,
) -> Result<String, JsValue> {
    let response_format: QuestionPresentationResponseFormat =
        serde_json::from_str(response_format_json).map_err(|error| {
            JsValue::from_str(&format!(
                "invalid Question Presentation Response Format: {error}"
            ))
        })?;
    let response: StudentResponse = serde_json::from_str(response_json)
        .map_err(|error| JsValue::from_str(&format!("invalid student response: {error}")))?;
    let check = validation::validate_presentation_response_format(&response_format, &response);
    serde_json::to_string(&check).map_err(|error| {
        JsValue::from_str(&format!(
            "could not serialize Student Response Format Check: {error}"
        ))
    })
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
pub fn question_attempt_timing_decision(evaluation_json: &str) -> Result<String, JsValue> {
    let evaluation: timing::QuestionAttemptTimingEvaluation = serde_json::from_str(evaluation_json)
        .map_err(|error| JsValue::from_str(&format!("invalid timer evaluation: {error}")))?;
    let verdict = timing::question_attempt_timing_decision(&evaluation)
        .map_err(|error| JsValue::from_str(&format!("invalid timer evaluation: {error}")))?;
    serde_json::to_string(&verdict)
        .map_err(|error| JsValue::from_str(&format!("could not serialize timer verdict: {error}")))
}

/// Reports every backend capability missing from an assignment configuration.
///
/// The Assignment Configuration and Question Backend capability inputs are
/// browser-safe. The same function is called by the server before publication,
/// so editor hints and publish refusal cannot drift.
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

/// Builds one key-free, unversioned PLE Question JSON workspace-draft preview.
///
/// Non-PLE Question Sources return the explicit `unavailable` capability result.
/// The bridge never imports Question Backend or Question Grader code: it can only
/// prepare the browser-safe static prompt only.
#[wasm_bindgen]
pub fn preview_ple_draft(draft_json: &str) -> Result<String, JsValue> {
    let request: draft_preview::DraftPreviewRequest = serde_json::from_str(draft_json)
        .map_err(|error| JsValue::from_str(&format!("invalid draft preview request: {error}")))?;
    let preview = draft_preview::preview_ple_draft(&request);
    serde_json::to_string(&preview)
        .map_err(|error| JsValue::from_str(&format!("could not serialize draft preview: {error}")))
}

/// Recomputes the Rust-owned presentation descriptor and verifies its public Question Presentation Token.
///
/// The browser passes only answer-free values it already received. TypeScript
/// never implements the binary codec, CRC, or SHA-256 rules independently.
///
/// # Errors
///
/// Returns a JavaScript error for malformed or internally inconsistent public
/// presentation data. A well-formed but mismatched Question Presentation Token returns `false`.
#[wasm_bindgen]
pub fn verify_presentation_descriptor(
    presentation_json: &str,
    question_asset_renditions_json: &str,
    presentation_token: &str,
) -> Result<bool, JsValue> {
    let presentation: QuestionPresentation = serde_json::from_str(presentation_json)
        .map_err(|error| JsValue::from_str(&format!("invalid Question Presentation: {error}")))?;
    let assets: Vec<QuestionAssetRendition> = serde_json::from_str(question_asset_renditions_json)
        .map_err(|error| JsValue::from_str(&format!("invalid presentation assets: {error}")))?;
    let expected = QuestionPresentationToken::parse(presentation_token).map_err(|error| {
        JsValue::from_str(&format!("invalid Question Presentation Token: {error}"))
    })?;
    let presentation = rebuild_public_question_presentation(&presentation, &assets)
        .map_err(|error| JsValue::from_str(&format!("invalid presentation: {error}")))?;
    Ok(presentation.checksum.public_token() == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_validation_delegates_to_the_key_free_domain_module() {
        let check = validate_response_format(
            r#"{"kind":"shortText","matchMode":"normalized","maxLength":20}"#,
            r#"{"kind":"shortText","text":"peptide"}"#,
        )
        .expect("valid JSON should produce a Student Response Format Check");

        assert_eq!(check, r#"{"issues":[]}"#);
    }

    #[test]
    fn timer_evaluation_delegates_to_the_clock_free_domain_module() {
        let verdict = question_attempt_timing_decision(
            r#"{
                "policy":{"kind":"limited","seconds":9,"graceSeconds":2},
                "timer":{"issuedAt":1000,"deadline":10000,"submittedAt":10500},
                "evaluatedAt":10500,
                "pauseExtensionMillis":0
            }"#,
        )
        .expect("valid server timestamps should produce a verdict");

        assert_eq!(verdict, r#""submittedWithinGrace""#);
    }

    #[test]
    fn presentation_verification_uses_the_rust_descriptor_codec() {
        let presentation = r#"{
            "questionRevision":{"questionId":"ABC-DEFG","revisionNumber":1},
            "question_seed":42,
            "presentationNonce":"11111111111111111111111111111111",
            "questionTitle":"Peptide bond",
            "prompt":[{"kind":"text","markdown":"Which group forms the peptide bond?"}],
            "response":{"kind":"singleChoice","choices":[
                {"id":"cfdf","body":[{"kind":"text","markdown":"Amino group"}]},
                {"id":"6603","body":[{"kind":"text","markdown":"Carboxyl group"}]}
            ]}
        }"#;
        let rebuilt = rebuild_public_question_presentation(
            &serde_json::from_str(presentation).expect("Question Presentation fixture"),
            &[],
        )
        .expect("descriptor");
        let checksum = rebuilt.checksum.public_token();
        assert_eq!(checksum.as_str(), "pd1_q2fE1ezXCkT6_yd7zeqkCQ");

        assert!(verify_presentation_descriptor(presentation, "[]", checksum.as_str()).unwrap());
        assert!(
            !verify_presentation_descriptor(
                &presentation.replace("Peptide bond", "Changed Question Title"),
                "[]",
                checksum.as_str(),
            )
            .unwrap()
        );
    }
}
