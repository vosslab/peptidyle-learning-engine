use super::*;
use serde_json::{Value, json};

fn metadata_value() -> Value {
    json!({
        "questionTitle": "Metadata title",
        "questionDescription": "Metadata description",
        "tags": [],
        "questionLicense": null,
        "questionCitation": null,
        "language": null
    })
}

fn request_value(backend: &str, format: &str, webwork_pg_path: Value, source: &str) -> Value {
    json!({
        "metadata": metadata_value(),
        "questionBackend": backend,
        "questionFormat": format,
        "webworkPgPath": webwork_pg_path,
        "source": source
    })
}

fn prepare_value(value: Value) -> Result<PreparedCreateDraft, ()> {
    let bytes = serde_json::to_vec(&value).expect("request JSON encodes");
    prepare_request(decode_request(&bytes).expect("request contract decodes"))
}

#[test]
fn request_decoding_requires_exact_top_level_members() {
    let request = request_value("ple", "pleQuestionJson", Value::Null, "{}");
    let bytes = serde_json::to_vec(&request).expect("request JSON encodes");
    assert!(decode_request(&bytes).is_ok());

    for member in [
        "metadata",
        "questionBackend",
        "questionFormat",
        "webworkPgPath",
        "source",
    ] {
        let mut missing = request.clone();
        missing
            .as_object_mut()
            .expect("request is an object")
            .remove(member);
        let bytes = serde_json::to_vec(&missing).expect("request JSON encodes");
        assert!(decode_request(&bytes).is_err(), "accepted missing {member}");
    }

    let mut non_text_source = request.clone();
    non_text_source
        .as_object_mut()
        .expect("request is an object")
        .insert("source".to_owned(), json!({ "text": "source" }));
    let bytes = serde_json::to_vec(&non_text_source).expect("request JSON encodes");
    assert!(decode_request(&bytes).is_err(), "accepted non-text source");

    let mut non_text_path = request.clone();
    non_text_path
        .as_object_mut()
        .expect("request is an object")
        .insert("webworkPgPath".to_owned(), json!(42));
    let bytes = serde_json::to_vec(&non_text_path).expect("request JSON encodes");
    assert!(decode_request(&bytes).is_err(), "accepted non-text PG path");

    let mut unknown_backend = request.clone();
    unknown_backend
        .as_object_mut()
        .expect("request is an object")
        .insert("questionBackend".to_owned(), json!("unknown"));
    let bytes = serde_json::to_vec(&unknown_backend).expect("request JSON encodes");
    assert!(
        decode_request(&bytes).is_err(),
        "accepted an unknown backend"
    );

    let mut unknown_format = request.clone();
    unknown_format
        .as_object_mut()
        .expect("request is an object")
        .insert("questionFormat".to_owned(), json!("unknown"));
    let bytes = serde_json::to_vec(&unknown_format).expect("request JSON encodes");
    assert!(
        decode_request(&bytes).is_err(),
        "accepted an unknown format"
    );

    let mut unknown = request;
    unknown
        .as_object_mut()
        .expect("request is an object")
        .insert("questionType".to_owned(), json!("singleChoice"));
    let bytes = serde_json::to_vec(&unknown).expect("request JSON encodes");
    assert!(
        decode_request(&bytes).is_err(),
        "accepted an unknown member"
    );
}

#[test]
fn registered_bindings_select_the_registered_media_type() {
    let cases = [
        (
            "ple",
            "pleQuestionJson",
            Value::Null,
            PLE_QUESTION_JSON_MEDIA_TYPE,
        ),
        (
            "webwork",
            "webworkPg",
            json!("Library/Algebra.pg"),
            WEBWORK_PG_MEDIA_TYPE,
        ),
        (
            "webwork",
            "webworkPgml",
            json!("Library/Algebra.pgml"),
            WEBWORK_PG_MEDIA_TYPE,
        ),
    ];

    for (backend, format, path, expected_media_type) in cases {
        let prepared = prepare_value(request_value(backend, format, path, ""))
            .expect("registered binding prepares");
        assert_eq!(prepared.source_media_type, expected_media_type);
    }
}

#[test]
fn empty_and_broken_source_stays_raw_for_every_registered_format() {
    let cases = [
        ("ple", "pleQuestionJson", Value::Null),
        ("webwork", "webworkPg", json!("Library/Algebra.pg")),
        ("webwork", "webworkPgml", json!("Library/Algebra.pgml")),
    ];

    for (backend, format, path) in cases {
        for source in ["", "broken source: &lambda;\n{"] {
            let prepared = prepare_value(request_value(backend, format, path.clone(), source))
                .expect("source validity does not affect binding");
            assert_eq!(prepared.source_bytes, source.as_bytes());
        }
    }
}

#[test]
fn invalid_backend_format_and_path_bindings_are_rejected() {
    let cases = [
        ("ple", "pleQuestionJson", json!("Library/Algebra.pg")),
        ("webwork", "webworkPg", Value::Null),
        ("webwork", "pleQuestionJson", json!("Library/Algebra.pg")),
        ("imathas", "imathas", json!("Library/Algebra.pg")),
        ("webwork", "webworkPg", json!("../Algebra.pg")),
        ("webwork", "webworkPg", json!("/Library/Algebra.pg")),
        ("webwork", "webworkPg", json!("Library//Algebra.pg")),
        ("webwork", "webworkPg", json!("Library/./Algebra.pg")),
        ("webwork", "webworkPg", json!("Library/../Algebra.pg")),
        ("webwork", "webworkPg", json!("Library\\Algebra.pg")),
        ("webwork", "webworkPg", json!("Library/\u{0}Algebra.pg")),
        ("webwork", "webworkPg", json!("")),
        ("webwork", "webworkPg", json!("a".repeat(1_025))),
    ];

    for (backend, format, path) in cases {
        assert!(
            prepare_value(request_value(
                backend,
                format,
                path.clone(),
                "unfinished source"
            ))
            .is_err(),
            "accepted {backend}/{format} with path {path}"
        );
    }
}

#[test]
fn metadata_stays_separate_and_initial_type_stays_unset() {
    let request = request_value(
        "ple",
        "pleQuestionJson",
        Value::Null,
        "Metadata title is not source metadata: {unfinished",
    );
    let prepared = prepare_value(request).expect("unfinished source prepares");

    assert_eq!(prepared.metadata.question_title, "Metadata title");
    assert_eq!(
        prepared.metadata.question_description,
        "Metadata description"
    );
    assert_eq!(
        prepared.source_bytes,
        b"Metadata title is not source metadata: {unfinished"
    );
    assert_eq!(prepared.question_type, None);
}

#[test]
fn valid_native_source_derives_its_initial_type_including_hotspot() {
    let native_sources = [
        (
            json!({
                "format": "pleQuestionJson",
                "prompt": "Choose one.",
                "response": {
                    "kind": "singleChoice",
                    "choices": [
                        { "id": "a", "text": "A" },
                        { "id": "b", "text": "B" }
                    ],
                    "correctChoice": "a"
                }
            })
            .to_string(),
            QuestionType::MultipleChoice,
        ),
        (
            json!({
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
            })
            .to_string(),
            QuestionType::Hotspot,
        ),
    ];

    for (source, expected_type) in native_sources {
        let prepared = prepare_value(request_value(
            "ple",
            "pleQuestionJson",
            Value::Null,
            &source,
        ))
        .expect("valid Native source prepares");
        assert_eq!(prepared.question_type, Some(expected_type));
    }
}

#[test]
fn webwork_create_keeps_type_unassigned_even_for_json_source() {
    let source = json!({
        "format": "pleQuestionJson",
        "prompt": "Choose one.",
        "response": {
            "kind": "singleChoice",
            "choices": [
                { "id": "a", "text": "A" },
                { "id": "b", "text": "B" }
            ],
            "correctChoice": "a"
        }
    })
    .to_string();
    let prepared = prepare_value(request_value(
        "webwork",
        "webworkPg",
        json!("Library/Question.pg"),
        &source,
    ))
    .expect("registered WebWork binding prepares");

    assert_eq!(prepared.question_type, None);
}
