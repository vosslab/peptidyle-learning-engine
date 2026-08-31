//! MOD-WASM: the browser-facing bridge.
//!
//! Every export delegates to `domain`, which keeps parameter generation,
//! format validation, timer verdicts, and capability validation identical on
//! both targets. The allowlist of exports is frozen in M1; `grading` is outside
//! this crate's dependency closure and must stay there.

use domain::{draft_preview, policy, timing, validation};
use question_model::generation::QuestionSeed;
use question_model::presentation::{
    PresentationEnvelopeV1, PresentedQuestionAsset, QuestionPresentationToken,
    rebuild_public_presentation_v1,
};
use question_model::response::{QuestionResponseFormat, StudentResponse};
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
    let definition: QuestionResponseFormat =
        serde_json::from_str(definition_json).map_err(|error| {
            JsValue::from_str(&format!("invalid Question Response Format: {error}"))
        })?;
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

/// Materializes one key-free, unversioned native workspace-draft preview.
///
/// Non-native sources return the explicit `unavailable` capability result.
/// The bridge never imports Question Backend or Question Grader code: it can only
/// generate disclosed parameters and apply them to safe prompt fields.
#[wasm_bindgen]
pub fn preview_native_draft(draft_json: &str, seed_json: &str) -> Result<String, JsValue> {
    let request: draft_preview::DraftPreviewRequest = serde_json::from_str(draft_json)
        .map_err(|error| JsValue::from_str(&format!("invalid draft preview request: {error}")))?;
    let seed: QuestionSeed = serde_json::from_str(seed_json)
        .map_err(|error| JsValue::from_str(&format!("invalid draft preview seed: {error}")))?;
    let preview = draft_preview::preview_native_draft(&request, seed)
        .map_err(|error| JsValue::from_str(&format!("invalid draft preview: {error}")))?;
    serde_json::to_string(&preview)
        .map_err(|error| JsValue::from_str(&format!("could not serialize draft preview: {error}")))
}

/// Recomputes the Rust-owned presentation descriptor and verifies its public digest.
///
/// The browser passes only answer-free values it already received. TypeScript
/// never implements the binary codec, CRC, or SHA-256 rules independently.
///
/// # Errors
///
/// Returns a JavaScript error for malformed or internally inconsistent public
/// presentation data. A well-formed but mismatched digest returns `false`.
#[wasm_bindgen]
pub fn verify_presentation_descriptor(
    envelope_json: &str,
    asset_bindings_json: &str,
    digest: &str,
) -> Result<bool, JsValue> {
    let envelope: PresentationEnvelopeV1 = serde_json::from_str(envelope_json)
        .map_err(|error| JsValue::from_str(&format!("invalid presentation envelope: {error}")))?;
    let assets: Vec<PresentedQuestionAsset> = serde_json::from_str(asset_bindings_json)
        .map_err(|error| JsValue::from_str(&format!("invalid presentation assets: {error}")))?;
    let expected = QuestionPresentationToken::parse(digest)
        .map_err(|error| JsValue::from_str(&format!("invalid presentation digest: {error}")))?;
    let presentation = rebuild_public_presentation_v1(&envelope, &assets)
        .map_err(|error| JsValue::from_str(&format!("invalid presentation: {error}")))?;
    Ok(presentation.digest.public_token() == expected)
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
    fn native_draft_preview_stays_key_free() {
        let preview = preview_native_draft(
            r#"{"workspace":"00000000-0000-0000-0000-000000000001","source":{"backend":"native"},"title":"Fixture","prompt":[{"kind":"text","markdown":"Value {{value}}"}],"response":{"kind":"shortText","matchMode":"normalized","maxLength":20},"questionVariationDefinition":{"kind":"seeded","generator":{"id":"fixture","version":"1"},"parameters":{"value":{"kind":"fixed","value":"safe"}}}}"#,
            "4",
        )
        .expect("native draft preview");
        assert_eq!(
            preview,
            r#"{"kind":"ready","preview":{"workspace":"00000000-0000-0000-0000-000000000001","seed":4,"title":"Fixture","prompt":[{"kind":"text","markdown":"Value safe"}],"response":{"kind":"shortText","matchMode":"normalized","maxLength":20}}}"#
        );
    }

    #[test]
    fn presentation_verification_uses_the_rust_descriptor_codec() {
        let envelope = r#"{
            "questionVersion":{"questionId":"ABC-DEFG","versionNumber":1},
            "seed":42,
            "presentationNonce":"11111111111111111111111111111111",
            "title":"Peptide bond",
            "prompt":[{"kind":"text","markdown":"Which group forms the peptide bond?"}],
            "response":{"kind":"singleChoice","choices":[
                {"id":"cfdf","body":[{"kind":"text","markdown":"Amino group"}]},
                {"id":"6603","body":[{"kind":"text","markdown":"Carboxyl group"}]}
            ]}
        }"#;
        let rebuilt = rebuild_public_presentation_v1(
            &serde_json::from_str(envelope).expect("fixture envelope"),
            &[],
        )
        .expect("descriptor");
        let digest = rebuilt.digest.public_token();
        assert_eq!(digest.as_str(), "pd1_q2fE1ezXCkT6_yd7zeqkCQ");

        assert!(verify_presentation_descriptor(envelope, "[]", digest.as_str()).unwrap());
        assert!(
            !verify_presentation_descriptor(
                &envelope.replace("Peptide bond", "Changed title"),
                "[]",
                digest.as_str(),
            )
            .unwrap()
        );
    }
}
