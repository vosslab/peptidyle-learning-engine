//! Answer-free fixed inputs shared by native, generated-Node, and browser Wasm parity checks.

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Corpus {
    cases: Vec<ParityCase>,
}

#[derive(Deserialize)]
pub struct ParityCase {
    pub name: String,
    pub definition: Value,
    pub response: Value,
    #[serde(rename = "expectedReport")]
    pub expected_report: Value,
}

pub fn cases() -> Vec<ParityCase> {
    serde_json::from_str::<Corpus>(include_str!(
        "../ple_question_json_response_format_corpus.json"
    ))
    .expect("committed answer-free ple-question-json-v2 response corpus is valid JSON")
    .cases
}

pub fn matching_full_permutation() -> ParityCase {
    cases()
        .into_iter()
        .find(|case| case.name == "ple-question-json-v2-matching-full-permutation")
        .expect("committed corpus includes matching full-permutation case")
}
