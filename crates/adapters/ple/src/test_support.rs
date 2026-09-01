use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

const MAX_STORED_QUESTION_BYTES: u64 = 1024 * 1024;
const FLAT_SINGLE_CHOICE_PATH: &str = "tests/fixtures/flat_single_choice_v2.json";
const IMPORTED_SINGLE_CHOICE_PATH: &str = "tests/fixtures/imported_single_choice_v2.json";

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
pub(crate) struct StoredFlatQuestion {
    pub(crate) response: StoredResponse,
    pub(crate) feedback: StoredFeedback,
    pub(crate) points: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StoredSingleChoiceQuestion {
    pub(crate) title: String,
    pub(crate) prompt: String,
    pub(crate) response: StoredResponse,
}

fn stored_question_bytes(relative: &str) -> Vec<u8> {
    // ASVS 5.3.2: this path is a compile-time repository path and never
    // incorporates a caller-provided filename.
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let metadata = fs::metadata(&path).expect("stored flat Question metadata");
    assert!(metadata.is_file(), "stored flat Question must be a file");
    assert!(
        metadata.len() <= MAX_STORED_QUESTION_BYTES,
        "stored flat Question exceeds its test-data limit"
    );
    fs::read(path).expect("stored flat Question data")
}

pub(crate) fn flat_single_choice_bytes() -> Vec<u8> {
    stored_question_bytes(FLAT_SINGLE_CHOICE_PATH)
}

pub(crate) fn flat_single_choice_value() -> Value {
    // ASVS 1.5.2 and 2.2.1: JSON enters through the same strict parser tests;
    // this generic value exists only so tests can make deliberate mutations.
    serde_json::from_slice(&flat_single_choice_bytes()).expect("stored flat Question JSON")
}

pub(crate) fn flat_single_choice_source() -> String {
    String::from_utf8(flat_single_choice_bytes()).expect("stored flat Question UTF-8")
}

pub(crate) fn flat_single_choice() -> StoredFlatQuestion {
    serde_json::from_slice(&flat_single_choice_bytes()).expect("stored flat Question shape")
}

pub(crate) fn imported_single_choice_bytes() -> Vec<u8> {
    stored_question_bytes(IMPORTED_SINGLE_CHOICE_PATH)
}

pub(crate) fn imported_single_choice() -> StoredSingleChoiceQuestion {
    serde_json::from_slice(&imported_single_choice_bytes()).expect("stored imported Question shape")
}
