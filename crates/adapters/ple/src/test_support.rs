use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

const MAX_STORED_QUESTION_BYTES: u64 = 1024 * 1024;
const PLE_QUESTION_JSON_SINGLE_CHOICE_SCHEMA_V2_PATH: &str =
    "tests/fixtures/ple_question_json_single_choice_schema_v2.json";
const IMPORTED_PLE_QUESTION_JSON_SINGLE_CHOICE_SCHEMA_V2_PATH: &str =
    "tests/fixtures/imported_ple_question_json_single_choice_schema_v2.json";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredChoice {
    pub(crate) id: String,
    pub(crate) text: String,
    pub(crate) feedback: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredResponse {
    pub(crate) choices: Vec<StoredChoice>,
    pub(crate) correct_choice: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StoredFeedback {
    pub(crate) correct: String,
    pub(crate) incorrect: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StoredPleQuestionJson {
    pub(crate) response: StoredResponse,
    pub(crate) feedback: StoredFeedback,
    pub(crate) points: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredSingleChoiceQuestion {
    pub(crate) title: String,
    pub(crate) question_description: String,
    pub(crate) prompt: String,
    pub(crate) response: StoredResponse,
}

fn stored_question_bytes(relative: &str) -> Vec<u8> {
    // ASVS 5.3.2: this path is a compile-time repository path and never
    // incorporates a caller-provided filename.
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let metadata = fs::metadata(&path).expect("stored PLE Question JSON metadata");
    assert!(
        metadata.is_file(),
        "stored PLE Question JSON must be a file"
    );
    assert!(
        metadata.len() <= MAX_STORED_QUESTION_BYTES,
        "stored PLE Question JSON exceeds its test-data limit"
    );
    fs::read(path).expect("stored PLE Question JSON data")
}

pub(crate) fn ple_question_json_single_choice_bytes() -> Vec<u8> {
    stored_question_bytes(PLE_QUESTION_JSON_SINGLE_CHOICE_SCHEMA_V2_PATH)
}

pub(crate) fn ple_question_json_single_choice_value() -> Value {
    // ASVS 1.5.2 and 2.2.1: JSON enters through the same strict parser tests;
    // this generic value exists only so tests can make deliberate mutations.
    serde_json::from_slice(&ple_question_json_single_choice_bytes())
        .expect("stored PLE Question JSON")
}

pub(crate) fn ple_question_json_single_choice_source() -> String {
    String::from_utf8(ple_question_json_single_choice_bytes())
        .expect("stored PLE Question JSON UTF-8")
}

pub(crate) fn ple_question_json_single_choice() -> StoredPleQuestionJson {
    serde_json::from_slice(&ple_question_json_single_choice_bytes())
        .expect("stored PLE Question JSON shape")
}

pub(crate) fn imported_single_choice_bytes() -> Vec<u8> {
    stored_question_bytes(IMPORTED_PLE_QUESTION_JSON_SINGLE_CHOICE_SCHEMA_V2_PATH)
}

pub(crate) fn imported_single_choice() -> StoredSingleChoiceQuestion {
    serde_json::from_slice(&imported_single_choice_bytes()).expect("stored imported Question shape")
}
