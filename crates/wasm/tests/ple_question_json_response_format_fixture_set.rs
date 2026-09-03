//! Answer-free fixed inputs shared by native, generated-Node, and browser Wasm parity checks.

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct FixtureSet {
    cases: Vec<ParityCase>,
}

#[derive(Deserialize)]
pub struct ParityCase {
    pub name: String,
    #[serde(rename = "responseFormat")]
    pub response_format: Value,
    pub response: Value,
    #[serde(rename = "expectedCheck")]
    pub expected_check: Value,
}

pub fn cases() -> Vec<ParityCase> {
    serde_json::from_str::<FixtureSet>(include_str!(
        "../ple_question_json_response_format_fixture_set.json"
    ))
    .expect("committed answer-free Question Response Format fixture set is valid JSON")
    .cases
}

pub fn matching_full_permutation() -> ParityCase {
    cases()
        .into_iter()
        .find(|case| case.name == "ple-question-json-v2-matching-full-permutation")
        .expect("fixture set includes the matching full-permutation case")
}
