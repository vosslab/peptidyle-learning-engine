use std::fs;
use std::path::Path;

const SOURCE_PATH: &str = "tests/fixtures/ple_question_json_single_choice.json";

pub(crate) fn ple_question_json_single_choice_bytes() -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(SOURCE_PATH))
        .expect("stored PLE Question JSON source")
}
