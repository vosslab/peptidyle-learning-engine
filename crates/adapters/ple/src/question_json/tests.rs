use question_model::response::{ResponseItemReference, StudentResponse};

use super::{PLE_QUESTION_JSON_MEDIA_TYPE, PleQuestionJsonDocument, PleQuestionJsonError};

const SINGLE_CHOICE_SOURCE: &[u8] =
    include_bytes!("../../tests/fixtures/ple_question_json_single_choice_schema_v3.json");

#[test]
fn version_three_source_compiles_private_evaluation_from_its_exact_content() {
    let document = PleQuestionJsonDocument::parse(SINGLE_CHOICE_SOURCE).expect("v3 source parses");
    let compiled = document.compile().expect("v3 source compiles");
    assert_eq!(
        PLE_QUESTION_JSON_MEDIA_TYPE,
        "application/vnd.peptidyle.question+json"
    );
    let result = compiled
        .private()
        .evaluate(
            compiled.private().public_content_checksum(),
            compiled.presentation().question_type(),
            compiled.presentation().response(),
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("blue")],
            },
        )
        .expect("correct response evaluates");
    assert!(result.evaluation.correct());
    assert_eq!(result.evaluation.normalized_credit(), 1.0);
    assert!(result.question_answer.is_some());
}

#[test]
fn source_checksum_refuses_a_substituted_presentation() {
    let document = PleQuestionJsonDocument::parse(SINGLE_CHOICE_SOURCE).expect("v3 source parses");
    let compiled = document.compile().expect("v3 source compiles");
    assert!(matches!(
        compiled.private().evaluate(
            "0000000000000000000000000000000000000000000000000000000000000000",
            compiled.presentation().question_type(),
            compiled.presentation().response(),
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("blue")]
            },
        ),
        Err(PleQuestionJsonError::PublicContentChecksumMismatch)
    ));
}

#[test]
fn unsupported_version_is_refused_without_a_legacy_reader() {
    let source = String::from_utf8(SINGLE_CHOICE_SOURCE.to_vec())
        .expect("fixture utf-8")
        .replacen("\"version\": 3", "\"version\": 2", 1);
    assert!(matches!(
        PleQuestionJsonDocument::parse(source.as_bytes()),
        Err(PleQuestionJsonError::UnsupportedVersion(2))
    ));
}
