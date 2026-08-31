use super::*;
use grading::GradeOutcome;
use question_model::response::{
    ChoiceId, HotspotPoint, MatchPair, StudentResponse, TextEntryAnswer,
};
use question_model::{DraftQuestionSource, QuestionId, QuestionSource, QuestionVersionNumber};
use serde_json::Value;
use uuid::Uuid;

const FAVORITE_COLOR: &str = r#"{
  "format": "pleFlatQuestion",
  "version": 2,
  "title": "Favorite color",
  "prompt": "What is my favorite color?",
  "response": {
    "kind": "singleChoice",
    "choices": [
      {"id": "blue", "text": "Blue", "feedback": "Blue is a calm choice."},
      {"id": "red", "text": "Red", "feedback": "Red is not my favorite."},
      {"id": "yellow", "text": "Yellow", "feedback": "Yellow is bright."}
    ],
    "correctChoice": "blue"
  },
  "feedback": {
    "correct": "Exactly right.",
    "incorrect": "Try thinking of a cool color."
  },
  "points": 1.0,
  "attemptPolicy": {"maxAttempts": null},
  "timingPolicy": {"kind": "untimed"},
  "tags": ["example"],
  "taxonomy": [],
  "license": {"kind": "ccBySa"},
  "language": "en-US"
}"#;

fn published(draft: DraftQuestionDefinition) -> QuestionDefinition {
    if !matches!(draft.source, DraftQuestionSource::Native) {
        panic!("flat fixture must use the native Question Backend");
    }
    QuestionDefinition::from_draft(
        draft,
        QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
        QuestionVersionNumber::new(1).expect("positive version"),
        QuestionSource::Native,
    )
}

fn v2_source(response: Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "format": "pleFlatQuestion",
        "version": 2,
        "title": "Version 2 flat Question fixture",
        "prompt": "Complete the response.",
        "response": response,
        "feedback": {"correct": "Correct.", "incorrect": "Try again."},
        "points": 2.0,
        "attemptPolicy": {"maxAttempts": null},
        "timingPolicy": {"kind": "untimed"},
        "tags": ["fixture"],
        "taxonomy": [],
        "license": {"kind": "ccBySa"},
        "language": "en-US"
    }))
    .expect("version 2 fixture should encode")
}

#[test]
fn version_two_compiles_and_grades_all_eight_flat_families() {
    let cases = vec![
        (
            "single choice",
            serde_json::json!({
                "kind": "singleChoice",
                "choices": [
                    {"id":"a", "text":"A", "feedback":null},
                    {"id":"b", "text":"B", "feedback":null}
                ],
                "correctChoice": "b"
            }),
            StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("b")],
            },
            StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("a")],
            },
        ),
        (
            "multiple answer",
            serde_json::json!({
                "kind": "multipleAnswer",
                "choices": [
                    {"id":"a", "text":"A", "feedback":null},
                    {"id":"b", "text":"B", "feedback":null},
                    {"id":"c", "text":"C", "feedback":null}
                ],
                "correctChoices": ["a", "c"]
            }),
            StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("c"), ChoiceId::new("a")],
            },
            StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("a")],
            },
        ),
        (
            "fill in",
            serde_json::json!({
                "kind": "fillIn",
                "answers": ["adenine"],
                "matchMode": "normalized",
                "maxLength": 40
            }),
            StudentResponse::ShortText {
                text: " Adenine ".to_string(),
            },
            StudentResponse::ShortText {
                text: "guanine".to_string(),
            },
        ),
        (
            "multi fill in",
            serde_json::json!({
                "kind": "multiFillIn",
                "blanks": [
                    {"id":"purine", "label":"Purine", "answers":["adenine"], "matchMode":"normalized", "maxLength":40},
                    {"id":"pyrimidine", "label":"Pyrimidine", "answers":["cytosine"], "matchMode":"normalized", "maxLength":40}
                ]
            }),
            StudentResponse::MultiBlank {
                answers: vec![
                    TextEntryAnswer {
                        slot: ChoiceId::new("purine"),
                        text: "adenine".to_string(),
                    },
                    TextEntryAnswer {
                        slot: ChoiceId::new("pyrimidine"),
                        text: "cytosine".to_string(),
                    },
                ],
            },
            StudentResponse::MultiBlank {
                answers: vec![
                    TextEntryAnswer {
                        slot: ChoiceId::new("purine"),
                        text: "guanine".to_string(),
                    },
                    TextEntryAnswer {
                        slot: ChoiceId::new("pyrimidine"),
                        text: "cytosine".to_string(),
                    },
                ],
            },
        ),
        (
            "numeric",
            serde_json::json!({
                "kind": "numeric",
                "answer": 7.4,
                "tolerance": {"kind":"absolute", "epsilon":0.1},
                "unit": "pH"
            }),
            StudentResponse::Numeric { value: 7.5 },
            StudentResponse::Numeric { value: 7.6 },
        ),
        (
            "matching",
            serde_json::json!({
                "kind": "matching",
                "prompts": [{"id":"dna", "text":"DNA"}, {"id":"rna", "text":"RNA"}],
                "choices": [{"id":"deoxy", "text":"Deoxyribose"}, {"id":"ribose", "text":"Ribose"}],
                "matches": [{"prompt":"dna", "choice":"deoxy"}, {"prompt":"rna", "choice":"ribose"}]
            }),
            StudentResponse::Matching {
                matches: vec![
                    MatchPair {
                        prompt: ChoiceId::new("dna"),
                        choice: ChoiceId::new("deoxy"),
                    },
                    MatchPair {
                        prompt: ChoiceId::new("rna"),
                        choice: ChoiceId::new("ribose"),
                    },
                ],
            },
            StudentResponse::Matching {
                matches: vec![
                    MatchPair {
                        prompt: ChoiceId::new("dna"),
                        choice: ChoiceId::new("ribose"),
                    },
                    MatchPair {
                        prompt: ChoiceId::new("rna"),
                        choice: ChoiceId::new("deoxy"),
                    },
                ],
            },
        ),
        (
            "ordering",
            serde_json::json!({
                "kind": "ordering",
                "items": [{"id":"one", "text":"One"}, {"id":"two", "text":"Two"}, {"id":"three", "text":"Three"}],
                "correctOrder": ["one", "two", "three"]
            }),
            StudentResponse::Ordering {
                order: ["one", "two", "three"]
                    .into_iter()
                    .map(ChoiceId::new)
                    .collect(),
            },
            StudentResponse::Ordering {
                order: ["two", "one", "three"]
                    .into_iter()
                    .map(ChoiceId::new)
                    .collect(),
            },
        ),
        (
            "hotspot",
            serde_json::json!({
                "kind": "hotspot",
                "surface": {
                    "asset":"00000000-0000-0000-0000-000000000123",
                    "checksum":"1111111111111111111111111111111111111111111111111111111111111111",
                    "description":"A labeled cell diagram"
                },
                "regions": [
                    {"id":"nucleus", "label":"Nucleus", "x":1000, "y":1000, "width":2000, "height":2000},
                    {"id":"membrane", "label":"Cell membrane", "x":6000, "y":6000, "width":2000, "height":2000}
                ],
                "correctRegions": ["nucleus"]
            }),
            StudentResponse::Hotspot {
                points: vec![HotspotPoint { x: 2000, y: 2000 }],
            },
            StudentResponse::Hotspot {
                points: vec![HotspotPoint { x: 7000, y: 7000 }],
            },
        ),
    ];

    for (name, source, correct_response, wrong_response) in cases {
        let document = FlatQuestionDocument::parse(&v2_source(source))
            .unwrap_or_else(|error| panic!("{name} source should parse: {error}"));
        let (draft, private) = document
            .compile(WorkspaceId::from_uuid(Uuid::from_u128(41)))
            .unwrap_or_else(|error| panic!("{name} source should compile: {error}"))
            .into_parts();
        let public_json = serde_json::to_string(&draft).expect("public draft serializes");
        assert!(!public_json.contains("correctChoice"), "{name}");
        assert!(!public_json.contains("correctChoices"), "{name}");
        assert!(!public_json.contains("correctOrder"), "{name}");
        assert!(!public_json.contains("correctRegions"), "{name}");
        let question = published(draft);
        let correct = private
            .evaluate(&question, &correct_response)
            .unwrap_or_else(|error| panic!("{name} correct response should grade: {error}"));
        assert!(
            matches!(correct.outcome, GradeOutcome::Graded(result) if result.correct && result.points_earned == 2.0),
            "{name}"
        );
        let wrong = private
            .evaluate(&question, &wrong_response)
            .unwrap_or_else(|error| panic!("{name} wrong response should grade: {error}"));
        assert!(
            matches!(wrong.outcome, GradeOutcome::Graded(result) if !result.correct && result.points_earned == 0.0),
            "{name}"
        );
    }
}

#[test]
fn hotspot_public_definition_does_not_reveal_correct_region_cardinality() {
    let base = serde_json::json!({
        "kind": "hotspot",
        "surface": {
            "asset":"00000000-0000-0000-0000-000000000123",
            "checksum":"1111111111111111111111111111111111111111111111111111111111111111",
            "description":"A labeled cell diagram"
        },
        "regions": [
            {"id":"nucleus", "label":"Nucleus", "x":1000, "y":1000, "width":2000, "height":2000},
            {"id":"membrane", "label":"Cell membrane", "x":6000, "y":6000, "width":2000, "height":2000}
        ]
    });
    let mut one_correct = base.clone();
    one_correct["correctRegions"] = serde_json::json!(["nucleus"]);
    let mut two_correct = base;
    two_correct["correctRegions"] = serde_json::json!(["nucleus", "membrane"]);
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(41));
    let (one_draft, _) = FlatQuestionDocument::parse(&v2_source(one_correct))
        .expect("one-correct hotspot should parse")
        .compile(workspace)
        .expect("one-correct hotspot should compile")
        .into_parts();
    let (two_draft, _) = FlatQuestionDocument::parse(&v2_source(two_correct))
        .expect("two-correct hotspot should parse")
        .compile(workspace)
        .expect("two-correct hotspot should compile")
        .into_parts();

    assert_eq!(one_draft.response, two_draft.response);
    assert!(matches!(
        one_draft.response,
        question_model::response::QuestionResponseFormat::Hotspot {
            selection: question_model::answer::SelectionCardinality::AtLeastOne,
            ..
        }
    ));
    assert!(
        domain::validation::validate_response_format(
            &one_draft.response,
            &StudentResponse::Hotspot {
                points: vec![HotspotPoint { x: 2000, y: 2000 }],
            },
        )
        .is_valid()
    );

    let public_json = serde_json::to_string(&one_draft).expect("public draft should serialize");
    assert!(!public_json.contains("correctRegions"));
    assert!(!public_json.contains("correct_region"));
}

#[test]
fn version_two_refuses_ambiguous_or_incomplete_private_bindings() {
    let invalid = [
        serde_json::json!({
            "kind":"singleChoice",
            "choices":[{"id":"a","text":"A"},{"id":"b","text":"B"}],
            "correctChoice":"missing"
        }),
        serde_json::json!({
            "kind":"multipleAnswer",
            "choices":[{"id":"a","text":"A"},{"id":"b","text":"B"}],
            "correctChoices":["a","a"]
        }),
        serde_json::json!({
            "kind":"fillIn","answers":[],"matchMode":"normalized","maxLength":40
        }),
        serde_json::json!({
            "kind":"multiFillIn",
            "blanks":[
                {"id":"same","label":"First","answers":["a"],"matchMode":"exact","maxLength":10},
                {"id":"same","label":"Second","answers":["b"],"matchMode":"exact","maxLength":10}
            ]
        }),
        serde_json::json!({
            "kind":"numeric","answer":1.0,
            "tolerance":{"kind":"absolute","epsilon":-0.1}
        }),
        serde_json::json!({
            "kind":"matching",
            "prompts":[{"id":"p1","text":"P1"},{"id":"p2","text":"P2"}],
            "choices":[{"id":"c1","text":"C1"},{"id":"c2","text":"C2"}],
            "matches":[{"prompt":"p1","choice":"c1"},{"prompt":"p2","choice":"c1"}]
        }),
        serde_json::json!({
            "kind":"ordering",
            "items":[{"id":"a","text":"A"},{"id":"b","text":"B"},{"id":"c","text":"C"}],
            "correctOrder":["a","b","b"]
        }),
        serde_json::json!({
            "kind":"hotspot",
            "surface":{
                "asset":"00000000-0000-0000-0000-000000000123",
                "checksum":"1111111111111111111111111111111111111111111111111111111111111111",
                "description":"A diagram"
            },
            "regions":[
                {"id":"left","label":"Left","x":1000,"y":1000,"width":3000,"height":3000},
                {"id":"right","label":"Right","x":2000,"y":2000,"width":3000,"height":3000}
            ],
            "correctRegions":["left"]
        }),
    ];

    for source in invalid {
        let Err(error) = FlatQuestionDocument::parse(&v2_source(source)) else {
            panic!("invalid version 2 private binding must refuse");
        };
        assert!(matches!(error, FlatQuestionError::InvalidDocument(_)));
    }
}

fn text(blocks: Option<&Vec<ContentBlock>>) -> Vec<&str> {
    blocks
        .into_iter()
        .flatten()
        .filter_map(|block| match block {
            ContentBlock::Text { markdown } => Some(markdown.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn favorite_color_splits_public_content_and_private_grading() {
    let document =
        FlatQuestionDocument::parse(FAVORITE_COLOR.as_bytes()).expect("flat source should parse");
    let compiled = document
        .compile(WorkspaceId::from_uuid(Uuid::from_u128(1)))
        .expect("flat source should compile");
    let public_json = serde_json::to_string(compiled.draft()).expect("public draft serializes");
    assert!(!public_json.contains("correctChoice"));
    assert!(!public_json.contains("Exactly right"));

    let (draft, private) = compiled.into_parts();
    let question = published(draft);
    let wrong = private
        .evaluate(
            &question,
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("red")],
            },
        )
        .expect("valid wrong choice should grade");
    assert!(matches!(
        wrong.outcome,
        GradeOutcome::Graded(result) if !result.correct && result.points_earned == 0.0
    ));
    assert_eq!(
        text(wrong.feedback.hint.as_ref()),
        vec!["Red is not my favorite.", "Try thinking of a cool color."]
    );

    let correct = private
        .evaluate(
            &question,
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("blue")],
            },
        )
        .expect("valid correct choice should grade");
    assert!(matches!(
        correct.outcome,
        GradeOutcome::Graded(result) if result.correct && result.points_earned == 1.0
    ));
    assert_eq!(
        text(correct.feedback.hint.as_ref()),
        vec!["Blue is a calm choice.", "Exactly right."]
    );
}

#[test]
fn canonical_bytes_ignore_input_whitespace_and_member_order() {
    let first =
        FlatQuestionDocument::parse(FAVORITE_COLOR.as_bytes()).expect("first source should parse");
    let mut reordered: Value =
        serde_json::from_str(FAVORITE_COLOR).expect("fixture JSON should parse");
    let object = reordered.as_object_mut().expect("fixture is an object");
    let format = object.remove("format").expect("format exists");
    object.insert("format".to_string(), format);
    let second_bytes = serde_json::to_vec_pretty(&reordered).expect("fixture should encode");
    let second = FlatQuestionDocument::parse(&second_bytes).expect("second source should parse");

    assert_eq!(
        first.canonical_sha256().expect("first hash"),
        second.canonical_sha256().expect("second hash")
    );
}

#[test]
fn malformed_or_ambiguous_sources_are_refused() {
    let duplicate_member =
        FAVORITE_COLOR.replacen("\"version\": 2,", "\"version\": 2, \"version\": 2,", 1);
    assert!(matches!(
        FlatQuestionDocument::parse(duplicate_member.as_bytes()),
        Err(FlatQuestionError::MalformedJson(_))
    ));

    let mut duplicate_choice: Value =
        serde_json::from_str(FAVORITE_COLOR).expect("fixture JSON should parse");
    duplicate_choice["response"]["choices"][1]["id"] = Value::String("blue".to_string());
    assert!(matches!(
        FlatQuestionDocument::parse(
            &serde_json::to_vec(&duplicate_choice).expect("modified fixture encodes")
        ),
        Err(FlatQuestionError::InvalidDocument(_))
    ));

    let mut unknown: Value =
        serde_json::from_str(FAVORITE_COLOR).expect("fixture JSON should parse");
    unknown["responseProcessing"] = Value::String("kitchen sink".to_string());
    assert!(matches!(
        FlatQuestionDocument::parse(
            &serde_json::to_vec(&unknown).expect("modified fixture encodes")
        ),
        Err(FlatQuestionError::MalformedJson(_))
    ));

    let mut nested_unknown: Value =
        serde_json::from_str(FAVORITE_COLOR).expect("fixture JSON should parse");
    nested_unknown["attemptPolicy"]["qtiExtension"] = Value::Bool(true);
    assert!(matches!(
        FlatQuestionDocument::parse(
            &serde_json::to_vec(&nested_unknown).expect("modified fixture encodes")
        ),
        Err(FlatQuestionError::MalformedJson(_))
    ));

    let mut legacy_feedback: Value =
        serde_json::from_str(FAVORITE_COLOR).expect("fixture JSON should parse");
    legacy_feedback["attemptPolicy"]["feedback"] = Value::String("immediateFull".to_string());
    assert!(matches!(
        FlatQuestionDocument::parse(
            &serde_json::to_vec(&legacy_feedback).expect("modified fixture encodes")
        ),
        Err(FlatQuestionError::MalformedJson(_))
    ));
}

#[test]
fn version_one_flat_source_is_refused_without_a_legacy_reader() {
    let version_one = FAVORITE_COLOR.replacen("\"version\": 2", "\"version\": 1", 1);

    assert!(matches!(
        FlatQuestionDocument::parse(version_one.as_bytes()),
        Err(FlatQuestionError::UnsupportedVersion(1))
    ));
}

#[test]
fn private_material_refuses_a_different_public_definition() {
    let document =
        FlatQuestionDocument::parse(FAVORITE_COLOR.as_bytes()).expect("flat source should parse");
    let (draft, private) = document
        .compile(WorkspaceId::from_uuid(Uuid::from_u128(1)))
        .expect("flat source should compile")
        .into_parts();
    let mut question = published(draft);
    question.prompt = markdown_blocks("A substituted prompt");

    assert!(matches!(
        private.evaluate(
            &question,
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("blue")],
            }
        ),
        Err(FlatQuestionError::PublicBindingMismatch)
    ));
}

#[test]
fn source_size_has_one_explicit_backstop() {
    let oversized = vec![b' '; MAX_FLAT_QUESTION_BYTES + 1];
    assert_eq!(
        FlatQuestionDocument::parse(&oversized).err(),
        Some(FlatQuestionError::TooLarge)
    );
}

#[test]
fn private_material_roundtrips_canonical_bytes_and_exposes_binding_digest_only() {
    let document =
        FlatQuestionDocument::parse(FAVORITE_COLOR.as_bytes()).expect("flat source should parse");
    let (draft, private) = document
        .compile(WorkspaceId::from_uuid(Uuid::from_u128(1)))
        .expect("flat source should compile")
        .into_parts();
    let encoded = private
        .canonical_bytes()
        .expect("private material should encode");
    let reloaded = FlatQuestionPrivate::from_canonical_bytes(&encoded)
        .expect("canonical payload should reload exactly");

    assert_eq!(
        encoded,
        reloaded
            .canonical_bytes()
            .expect("loaded payload re-encodes")
    );
    assert_eq!(
        private.public_binding_sha256(),
        reloaded.public_binding_sha256()
    );

    let question = published(draft);
    let correct = reloaded
        .evaluate(
            &question,
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("blue")],
            },
        )
        .expect("roundtripped private material should evaluate like original");
    assert!(matches!(
        correct.outcome,
        GradeOutcome::Graded(result) if result.correct && result.points_earned == 1.0
    ));
}

#[test]
fn private_material_validates_canonical_shape_and_rejects_substitutions() {
    let document =
        FlatQuestionDocument::parse(FAVORITE_COLOR.as_bytes()).expect("flat source should parse");
    let private = document
        .compile(WorkspaceId::from_uuid(Uuid::from_u128(1)))
        .expect("flat source should compile")
        .private;

    let canonical = private
        .canonical_bytes()
        .expect("private material should encode");
    assert!(matches!(
        FlatQuestionPrivate::from_canonical_bytes(&canonical),
        Ok(value) if value == private
    ));
    let canonical_value: Value = String::from_utf8(canonical.clone())
        .expect("canonical private payload should be valid UTF-8")
        .parse()
        .expect("canonical payload should decode");
    let with_whitespace = serde_json::to_string_pretty(&canonical_value)
        .expect("canonical payload should format with canonical layout");
    assert!(matches!(
        FlatQuestionPrivate::from_canonical_bytes(with_whitespace.as_bytes()),
        Err(FlatQuestionError::MalformedJson(_))
    ));
    let mut mutated: Value =
        serde_json::from_slice(&canonical).expect("canonical payload should parse to JSON");
    mutated["schemaVersion"] = 3.into();
    assert!(
        FlatQuestionPrivate::from_canonical_bytes(&serde_json::to_vec(&mutated).expect("mutated"))
            .is_err()
    );
    mutated["schemaVersion"] = 1.into();
    mutated["publicSha256"] = Value::String("bad digest".to_string());
    assert!(
        FlatQuestionPrivate::from_canonical_bytes(&serde_json::to_vec(&mutated).expect("mutated"))
            .is_err()
    );
}

#[test]
fn private_material_validates_against_the_exact_draft_before_publication() {
    let document =
        FlatQuestionDocument::parse(FAVORITE_COLOR.as_bytes()).expect("flat source should parse");
    let (draft, private) = document
        .compile(WorkspaceId::from_uuid(Uuid::from_u128(1)))
        .expect("flat source should compile")
        .into_parts();
    assert!(private.validate_for_draft(&draft).is_ok());

    let mut substituted_draft = draft.clone();
    substituted_draft.prompt = markdown_blocks("Substituted prompt");
    assert!(matches!(
        private.validate_for_draft(&substituted_draft),
        Err(FlatQuestionError::PublicBindingMismatch)
    ));

    let canonical = String::from_utf8(
        private
            .canonical_bytes()
            .expect("private material should encode"),
    )
    .expect("canonical private material should be UTF-8");
    let substituted_canonical =
        canonical.replacen(r#""choice":"blue""#, r#""choice":"not-a-choice""#, 1);
    let substituted_private =
        FlatQuestionPrivate::from_canonical_bytes(substituted_canonical.as_bytes())
            .expect("private material shape alone cannot know the draft choices");
    assert!(matches!(
        substituted_private.validate_for_draft(&draft),
        Err(FlatQuestionError::PublicBindingMismatch)
    ));
}
