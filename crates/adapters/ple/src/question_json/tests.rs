use question_model::response::{QuestionResponseFormat, ResponseItemId, StudentResponse};
use question_model::{GradingResult, QuestionImageAssetId, QuestionImageAssetTuple};
use serde_json::json;
use uuid::Uuid;

use super::{PLE_QUESTION_JSON_MEDIA_TYPE, PleQuestionJsonDocument, PleQuestionJsonError};

const SINGLE_CHOICE_SOURCE: &[u8] =
    include_bytes!("../../tests/fixtures/ple_question_json_single_choice.json");

#[test]
fn native_document_contains_content_only_and_rejects_record_metadata() {
    let source: serde_json::Value =
        serde_json::from_slice(SINGLE_CHOICE_SOURCE).expect("fixture parses");
    let object = source.as_object().expect("source object");
    for key in [
        "questionTitle",
        "questionDescription",
        "tags",
        "questionLicense",
        "questionCitation",
        "language",
    ] {
        assert!(!object.contains_key(key), "Native source must omit {key}");
        let mut with_record_metadata = source.clone();
        with_record_metadata[key] = match key {
            "tags" => json!(["record-owned tag"]),
            "questionLicense" => json!("CC-BY-4.0"),
            "questionCitation" => json!(null),
            "language" => json!("en"),
            _ => json!("record-owned value"),
        };
        let encoded = serde_json::to_vec(&with_record_metadata).expect("source encodes");
        assert!(
            PleQuestionJsonDocument::parse(&encoded).is_err(),
            "Native source must reject {key}"
        );
    }
    let compiled = PleQuestionJsonDocument::parse(SINGLE_CHOICE_SOURCE)
        .expect("content-only source parses")
        .compile()
        .expect("content-only source compiles");
    assert!(!compiled.presentation().prompt().is_empty());
}

#[test]
fn source_compiles_private_evaluation_from_its_exact_content() {
    let document = PleQuestionJsonDocument::parse(SINGLE_CHOICE_SOURCE).expect("source parses");
    let compiled = document.compile().expect("source compiles");
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
                selected: vec![ResponseItemId::new("blue")],
            },
        )
        .expect("correct response evaluates");
    assert!(result.evaluation.correct());
    assert_eq!(result.evaluation.normalized_credit(), 1.0);
    assert!(result.question_answer.is_some());
    assert_eq!(
        compiled.presentation().native_choice_order(),
        question_model::NativeChoiceOrder::Fixed
    );
}

#[test]
fn exact_text_answers_fit_the_student_utf16_response_limit() {
    let mut source: serde_json::Value =
        serde_json::from_slice(SINGLE_CHOICE_SOURCE).expect("fixture parses");
    source["response"] = json!({
        "kind": "fillIn",
        "answers": ["AB"],
        "matchMode": "exact",
        "maxLength": 1
    });
    let exact_too_long = serde_json::to_vec(&source).expect("source encodes");
    assert!(PleQuestionJsonDocument::parse(&exact_too_long).is_err());

    source["response"] = json!({
        "kind": "fillIn",
        "answers": ["\u{1F600}"],
        "matchMode": "exact",
        "maxLength": 1
    });
    let surrogate_pair = serde_json::to_vec(&source).expect("source encodes");
    assert!(PleQuestionJsonDocument::parse(&surrogate_pair).is_err());

    source["response"] = json!({
        "kind": "fillIn",
        "answers": ["long normalized answer"],
        "matchMode": "normalized",
        "maxLength": 3
    });
    let normalized = serde_json::to_vec(&source).expect("source encodes");
    assert!(PleQuestionJsonDocument::parse(&normalized).is_ok());
}

#[test]
fn fib_regex_mode_compiles_patterns_and_reports_invalid_syntax() {
    let mut source: serde_json::Value =
        serde_json::from_slice(SINGLE_CHOICE_SOURCE).expect("fixture parses");
    source["response"] = json!({
        "kind": "fillIn",
        "answers": ["^ATP$"],
        "matchMode": "regex",
        "maxLength": 16
    });
    let encoded = serde_json::to_vec(&source).expect("source encodes");
    let document = PleQuestionJsonDocument::parse(&encoded).expect("regex source parses");
    let compiled = document.compile().expect("valid regex compiles");
    assert!(matches!(
        compiled.presentation().response(),
        QuestionResponseFormat::ShortText {
            match_mode: question_model::answer::TextResponseMatchRule::Regex,
            ..
        }
    ));
    assert_eq!(
        serde_json::to_value(compiled.presentation().response()).expect("response encodes"),
        json!({
            "kind": "shortText",
            "matchMode": "regex",
            "maxLength": 16
        })
    );
    let result = compiled
        .private()
        .evaluate(
            compiled.private().public_content_checksum(),
            compiled.presentation().question_type(),
            compiled.presentation().response(),
            &StudentResponse::ShortText {
                text: "ATP".to_string(),
            },
        )
        .expect("valid regex response evaluates");
    assert_eq!(result.evaluation.normalized_credit(), 1.0);

    source["response"]["answers"] = json!(["["]);
    let invalid = serde_json::to_vec(&source).expect("invalid source encodes");
    let document = PleQuestionJsonDocument::parse(&invalid).expect("source shape parses");
    assert!(
        document.compile().is_err(),
        "invalid regex must fail compilation"
    );
}

#[test]
fn recorded_teaching_projection_uses_recorded_outcome_without_regrading() {
    let document = PleQuestionJsonDocument::parse(SINGLE_CHOICE_SOURCE).expect("source parses");
    let compiled = document.compile().expect("source compiles");
    let response = StudentResponse::MultipleChoice {
        selected: vec![ResponseItemId::new("blue")],
    };

    let content = compiled
        .private()
        .project_recorded_teaching_content(
            compiled.private().public_content_checksum(),
            compiled.presentation().question_type(),
            compiled.presentation().response(),
            Some(&response),
            Some(GradingResult {
                correct: false,
                points_earned: 0.0,
                points_possible: 1.0,
            }),
        )
        .expect("recorded content projects");

    assert!(
        content
            .question_feedback
            .expect("feedback")
            .incorrect_feedback
            .is_some()
    );
    assert!(content.question_answer.is_some());
}

#[test]
fn teaching_projection_keeps_selected_choice_feedback_without_an_outcome() {
    let document = PleQuestionJsonDocument::parse(SINGLE_CHOICE_SOURCE).expect("source parses");
    let compiled = document.compile().expect("source compiles");
    let response = StudentResponse::MultipleChoice {
        selected: vec![ResponseItemId::new("blue")],
    };

    let content = compiled
        .private()
        .project_recorded_teaching_content(
            compiled.private().public_content_checksum(),
            compiled.presentation().question_type(),
            compiled.presentation().response(),
            Some(&response),
            None,
        )
        .expect("choice content projects");
    let feedback = content.question_feedback.expect("selected-choice feedback");

    assert!(feedback.choice_feedback.is_some());
    assert!(feedback.correct_feedback.is_none());
    assert!(feedback.incorrect_feedback.is_none());
}

#[test]
fn choice_randomization_is_choice_owned_and_defaults_when_omitted() {
    let source = br#"{
        "format": "pleQuestionJson",
        "prompt": "Choose one.",
        "response": {
            "kind": "singleChoice",
            "choices": [
                {"id": "a", "text": "A"},
                {"id": "b", "text": "B"}
            ],
            "correctChoice": "a",
            "randomizeChoices": true
        }
    }"#;
    let compiled = PleQuestionJsonDocument::parse(source)
        .expect("choice randomization parses")
        .compile()
        .expect("choice randomization compiles");
    assert_eq!(
        compiled.presentation().native_choice_order(),
        question_model::NativeChoiceOrder::NonceRandomized
    );

    let non_choice_source = br#"{
        "format": "pleQuestionJson",
        "prompt": "Enter the answer.",
        "response": {
            "kind": "fillIn",
            "answers": ["answer"],
            "matchMode": "exact",
            "maxLength": 16,
            "randomizeChoices": true
        }
    }"#;
    assert!(PleQuestionJsonDocument::parse(non_choice_source).is_err());
}

#[test]
fn source_checksum_refuses_a_substituted_presentation() {
    let document = PleQuestionJsonDocument::parse(SINGLE_CHOICE_SOURCE).expect("source parses");
    let compiled = document.compile().expect("source compiles");
    assert!(matches!(
        compiled.private().evaluate(
            "0000000000000000000000000000000000000000000000000000000000000000",
            compiled.presentation().question_type(),
            compiled.presentation().response(),
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemId::new("blue")]
            },
        ),
        Err(PleQuestionJsonError::PublicContentChecksumMismatch)
    ));
}

#[test]
fn external_image_resource_is_distinguishable_from_nonvisual_resources() {
    let image_source = br#"{
        "format": "pleQuestionJson",
        "prompt": "Use the referenced figure.",
        "response": {
            "kind": "singleChoice",
            "choices": [{"id": "a", "text": "A"}, {"id": "b", "text": "B"}],
            "correctChoice": "a"
        },
        "externalResources": [{"url": "https://example.edu/figure", "kind": "image"}]
    }"#;
    let link_source = image_source
        .windows(b"\"image\"".len())
        .position(|window| window == b"\"image\"")
        .map(|index| {
            let mut source = image_source.to_vec();
            source.splice(
                index..index + b"\"image\"".len(),
                b"\"link\"".iter().copied(),
            );
            source
        })
        .expect("image kind marker");

    assert!(
        PleQuestionJsonDocument::parse(image_source)
            .expect("native image source parses")
            .has_external_image_resource()
    );
    assert!(
        !PleQuestionJsonDocument::parse(&link_source)
            .expect("native link source parses")
            .has_external_image_resource()
    );
}

#[test]
fn hotspot_publication_retargets_the_complete_question_image_asset_tuple() {
    let source = br#"{
        "format": "pleQuestionJson",
        "prompt": "Select the active site.",
        "response": {
            "kind": "hotspot",
            "surface": {
                "questionImageAssetId": "00000000-0000-4000-8000-000000000001",
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
        }
    }"#;
    let replacement = QuestionImageAssetTuple {
        question_image_asset_id: QuestionImageAssetId::from_uuid(Uuid::from_u128(2)),
        checksum: "b".repeat(64),
    };

    let document = PleQuestionJsonDocument::parse(source).expect("hotspot source parses");
    let published = document
        .with_hotspot_surface_image(replacement.clone())
        .expect("hotspot asset tuple retargets");
    let compiled = published.compile().expect("retargeted source compiles");

    let QuestionResponseFormat::Hotspot {
        question_image_asset_tuple,
        ..
    } = compiled.presentation().response()
    else {
        panic!("retargeted source remains a hotspot question");
    };
    assert_eq!(question_image_asset_tuple, &replacement);
}

#[test]
fn changing_source_answer_grading_or_question_image_changes_the_revision_source_checksum() {
    let original = PleQuestionJsonDocument::parse(SINGLE_CHOICE_SOURCE).expect("source parses");
    let original_checksum = original.canonical_sha256().expect("source checksum");
    let mut source =
        serde_json::from_slice::<serde_json::Value>(SINGLE_CHOICE_SOURCE).expect("json");
    source["prompt"] = json!("Which color is favorite?");
    let changed_source =
        PleQuestionJsonDocument::parse(&serde_json::to_vec(&source).expect("source bytes"))
            .expect("changed source parses");
    assert_ne!(
        changed_source
            .canonical_sha256()
            .expect("changed source checksum"),
        original_checksum
    );

    let mut answer =
        serde_json::from_slice::<serde_json::Value>(SINGLE_CHOICE_SOURCE).expect("json");
    answer["response"]["correctChoice"] = json!("red");
    let changed_answer =
        PleQuestionJsonDocument::parse(&serde_json::to_vec(&answer).expect("answer bytes"))
            .expect("changed answer parses");
    assert_ne!(
        changed_answer.canonical_sha256().expect("answer checksum"),
        original_checksum
    );

    let numeric = json!({
        "format": "pleQuestionJson",
        "prompt": "What is the thickness in nanometers?",
        "response": {
            "kind": "numeric",
            "answer": 7.5,
            "tolerance": { "kind": "absolute", "epsilon": 0.1 }
        }
    });
    let numeric_document =
        PleQuestionJsonDocument::parse(&serde_json::to_vec(&numeric).expect("numeric bytes"))
            .expect("numeric source parses");
    let numeric_checksum = numeric_document
        .canonical_sha256()
        .expect("numeric checksum");
    let mut grading = numeric;
    grading["response"]["tolerance"]["epsilon"] = json!(0.2);
    let changed_grading =
        PleQuestionJsonDocument::parse(&serde_json::to_vec(&grading).expect("grading bytes"))
            .expect("changed grading parses");
    assert_ne!(
        changed_grading
            .canonical_sha256()
            .expect("grading checksum"),
        numeric_checksum
    );

    let hotspot = br#"{
        "format": "pleQuestionJson",
        "prompt": "Select the active site.",
        "response": {
            "kind": "hotspot",
            "surface": {
                "questionImageAssetId": "00000000-0000-4000-8000-000000000001",
                "checksum": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "description": "Protein structure"
            },
            "regions": [{
                "id": "active-site",
                "label": "Active site",
                "x": 10, "y": 10, "width": 20, "height": 20
            }],
            "correctRegions": ["active-site"]
        }
    }"#;
    let hotspot_document = PleQuestionJsonDocument::parse(hotspot).expect("hotspot source parses");
    let hotspot_checksum = hotspot_document
        .canonical_sha256()
        .expect("hotspot checksum");
    let retargeted = hotspot_document
        .with_hotspot_surface_image(QuestionImageAssetTuple {
            question_image_asset_id: QuestionImageAssetId::from_uuid(Uuid::from_u128(2)),
            checksum: "b".repeat(64),
        })
        .expect("image asset retargets");
    assert_ne!(
        retargeted.canonical_sha256().expect("image checksum"),
        hotspot_checksum
    );
}

#[test]
fn supported_author_javascript_libraries_are_explicitly_recorded_and_reviewable() {
    let mut document: serde_json::Value =
        serde_json::from_slice(SINGLE_CHOICE_SOURCE).expect("fixture");
    document["authorScript"] = json!({
        "source": "document.getElementById('author-content-root').textContent = 'rendered';",
        "libraries": ["rdkit"],
    });
    document["externalResources"] = json!([
        {"url": "https://example.edu/notes", "kind": "link"},
        {"url": "https://cdn.example.org/diagram.svg", "kind": "image"},
        {"url": "https://example.edu/widget.js", "kind": "script"},
        {"url": "https://example.edu/theme.css", "kind": "stylesheet"},
    ]);
    let recorded = serde_json::to_vec(&document).expect("recorded source");
    PleQuestionJsonDocument::parse(&recorded).expect("recorded rdkit source parses");
}
