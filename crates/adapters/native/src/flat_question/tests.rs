use super::*;
use grading::GradeOutcome;
use question_model::response::StudentResponse;
use question_model::{ProblemId, QuestionSource, VersionId};
use serde_json::Value;
use uuid::Uuid;

const FAVORITE_COLOR: &str = r#"{
  "format": "pleFlatQuestion",
  "version": 1,
  "kind": "singleChoice",
  "title": "Favorite color",
  "prompt": "What is my favorite color?",
  "choices": [
    {"id": "blue", "text": "Blue", "feedback": "Blue is a calm choice."},
    {"id": "red", "text": "Red", "feedback": "Red is not my favorite."},
    {"id": "yellow", "text": "Yellow", "feedback": "Yellow is bright."}
  ],
  "correctChoice": "blue",
  "feedback": {
    "correct": "Exactly right.",
    "incorrect": "Try thinking of a cool color."
  },
  "points": 1.0,
  "attemptPolicy": {"maxAttempts": null, "feedback": "immediateFull"},
  "timingPolicy": {"kind": "untimed"},
  "tags": ["example"],
  "taxonomy": [],
  "license": {"kind": "ccBySa"},
  "language": "en-US"
}"#;

fn published(draft: DraftQuestionDefinition) -> QuestionDefinition {
    QuestionDefinition::from_draft(
        draft,
        ProblemId::from_uuid(Uuid::from_u128(2)),
        VersionId::from_uuid(Uuid::from_u128(3)),
        QuestionSource::Native {
            family: FLAT_SINGLE_CHOICE_FAMILY.to_string(),
        },
    )
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
        FAVORITE_COLOR.replacen("\"version\": 1,", "\"version\": 1, \"version\": 1,", 1);
    assert!(matches!(
        FlatQuestionDocument::parse(duplicate_member.as_bytes()),
        Err(FlatQuestionError::MalformedJson(_))
    ));

    let mut duplicate_choice: Value =
        serde_json::from_str(FAVORITE_COLOR).expect("fixture JSON should parse");
    duplicate_choice["choices"][1]["id"] = Value::String("blue".to_string());
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
    mutated["schemaVersion"] = 2.into();
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
