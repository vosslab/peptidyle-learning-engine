use std::fs;
use std::path::Path;

use serde::Deserialize;

const IMPORTED_PATH: &str =
    "tests/fixtures/imported_ple_question_json_single_choice_schema_v3.json";
const SOURCE_PATH: &str = "tests/fixtures/ple_question_json_single_choice_schema_v3.json";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredChoice {
    pub(crate) id: String,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredResponse {
    pub(crate) choices: Vec<StoredChoice>,
    pub(crate) correct_choice: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredSingleChoiceQuestion {
    pub(crate) title: String,
    pub(crate) question_description: String,
    pub(crate) prompt: String,
    pub(crate) response: StoredResponse,
}

pub(crate) fn imported_single_choice_bytes() -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(IMPORTED_PATH))
        .expect("stored imported source")
}

pub(crate) fn ple_question_json_single_choice_bytes() -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(SOURCE_PATH))
        .expect("stored PLE Question JSON source")
}

pub(crate) fn imported_single_choice() -> StoredSingleChoiceQuestion {
    serde_json::from_slice(&imported_single_choice_bytes()).expect("stored imported source shape")
}
