use question_model::response::{QuestionResponseFormat, ResponseItemReference, StudentResponse};
use question_model::{QuestionAssetId, QuestionAssetReference};
use uuid::Uuid;

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

#[test]
fn hotspot_publication_retargets_the_complete_question_asset_reference() {
    let source = br#"{
        "format": "pleQuestionJson",
        "version": 3,
        "questionTitle": "Locate the active site",
        "questionDescription": "A hotspot question.",
        "prompt": "Select the active site.",
        "response": {
            "kind": "hotspot",
            "surface": {
                "questionAsset": "00000000-0000-4000-8000-000000000001",
                "checksum": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "description": "Protein structure"
            },
            "regions": [{
                "id": "active-site",
                "label": "Active site",
                "x": 10,
                "y": 10,
                "width": 20,
                "height": 20
            }],
            "correctRegions": ["active-site"]
        },
        "language": "en"
    }"#;
    let replacement = QuestionAssetReference {
        question_asset: QuestionAssetId::from_uuid(Uuid::from_u128(2)),
        checksum: "b".repeat(64),
    };

    let document = PleQuestionJsonDocument::parse(source).expect("hotspot source parses");
    let published = document
        .with_hotspot_surface_asset(replacement.clone())
        .expect("hotspot asset reference retargets");
    let compiled = published.compile().expect("retargeted source compiles");

    let QuestionResponseFormat::Hotspot { surface, .. } = compiled.presentation().response() else {
        panic!("retargeted source remains a hotspot question");
    };
    assert_eq!(surface, &replacement);
}
