//! Stable display, image binding, and private teaching-content contracts.

use super::PleQuestionJsonDocument;
use question_model::response::ResponseItemId;
use question_model::{
    QuestionContentBlock, QuestionHtmlImageBinding, QuestionImageAssetId, QuestionImageAssetTuple,
    StudentResponse,
};
use serde_json::json;

fn document(prompt: &str, choices: serde_json::Value) -> PleQuestionJsonDocument {
    PleQuestionJsonDocument::parse(
        &serde_json::to_vec(&json!({
            "format": "pleQuestionJson", "prompt": prompt,
            "response": { "kind": "singleChoice", "choices": choices, "correctChoice": "a" },
            "questionHint": "<p style=\"color:navy\">Hint<img src='hint.png'></p>",
            "feedback": { "correct": "<img src='right.png'>Correct", "incorrect": "Try again" },
            "externalResources": [{"url":"https://example.org/inventory-only.png", "kind":"image"}]
        }))
        .unwrap(),
    )
    .unwrap()
}

fn binding(name: &str, number: u128) -> QuestionHtmlImageBinding {
    QuestionHtmlImageBinding {
        source_relative_name: name.to_owned(),
        question_image_asset_tuple: QuestionImageAssetTuple {
            question_image_asset_id: QuestionImageAssetId::from_uuid(uuid::Uuid::from_u128(number)),
            checksum: "a".repeat(64),
        },
    }
}

#[test]
fn native_source_preserves_a_large_table_prompt() {
    let rows = "<tr><td>Genetic marker</td><td>Observed allele</td></tr>".repeat(2_000);
    let prompt = format!("<table>{rows}</table>");
    let source = document(
        &prompt,
        json!([{"id":"a", "text":"First"}, {"id":"b", "text":"Second"}]),
    );
    let compiled = source.compile().unwrap();
    assert_eq!(
        compiled.presentation().prompt(),
        &[QuestionContentBlock::Html {
            html: prompt,
            images: Vec::new(),
        }]
    );
}

#[test]
fn html_and_inline_image_bindings_preserve_source_and_private_disclosure() {
    let prompt = "<table style='width:50%'><tr><td><strong>Structure</strong><img SRC='assets/a &amp; b.png' alt='diagram'></td></tr></table>";
    let source = document(
        prompt,
        json!([
            { "id":"a", "text":"<em>ATP</em><img src='choice.png'>", "feedback":"<p>Selected A<img src='selected.png'></p>" },
            { "id":"b", "text":"Plain text" }
        ]),
    );
    let names = source.relative_image_names().unwrap();
    assert_eq!(
        names,
        [
            "assets/a & b.png",
            "choice.png",
            "hint.png",
            "right.png",
            "selected.png"
        ]
    );
    let images: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(index, name)| binding(name, index as u128 + 1))
        .collect();
    let canonical_source = source.canonical_bytes().unwrap();
    let compiled = source.compile_with_images(&images).unwrap();
    assert_eq!(source.canonical_bytes().unwrap(), canonical_source);
    let [
        QuestionContentBlock::Html {
            html,
            images: prompt_images,
        },
    ] = compiled.presentation().prompt()
    else {
        panic!("prompt HTML");
    };
    assert_eq!(html, prompt);
    assert_eq!(prompt_images, &images[..1]);
    let public = serde_json::to_string(&(
        compiled.presentation().prompt(),
        compiled.presentation().response(),
    ))
    .unwrap();
    assert!(!public.contains("hint.png"));
    assert!(!public.contains("right.png"));
    assert!(!public.contains("selected.png"));
    let evaluated = compiled
        .private()
        .evaluate(
            compiled.private().public_content_checksum(),
            compiled.presentation().question_type(),
            compiled.presentation().response(),
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemId::new("a")],
            },
        )
        .unwrap();
    assert!(evaluated.evaluation.correct());
    let [
        QuestionContentBlock::Html {
            images: choice_images,
            ..
        },
    ] = evaluated
        .question_feedback
        .choice_feedback
        .as_deref()
        .unwrap()
    else {
        panic!("choice feedback HTML");
    };
    assert_eq!(choice_images, &images[4..5]);
    let [
        QuestionContentBlock::Html {
            images: outcome_images,
            ..
        },
    ] = evaluated
        .question_feedback
        .correct_feedback
        .as_deref()
        .unwrap()
    else {
        panic!("outcome feedback HTML");
    };
    assert_eq!(outcome_images, &images[3..4]);
    let [
        QuestionContentBlock::Html {
            images: hint_images,
            ..
        },
    ] = compiled.question_hint().unwrap().content()
    else {
        panic!("hint HTML");
    };
    assert_eq!(hint_images, &images[2..3]);
    let mut changed = images.clone();
    changed[3].question_image_asset_tuple.checksum = "b".repeat(64);
    let changed = source.compile_with_images(&changed).unwrap();
    assert_ne!(
        compiled.private().public_content_checksum(),
        changed.private().public_content_checksum()
    );
}

#[test]
fn image_resolution_requires_used_assets_and_inventory_does_not_add_usage() {
    let source = document(
        "<p>No image</p><!-- <img src='comment.png'> -->",
        json!([
            {"id":"a", "text":"First"}, {"id":"b", "text":"Second"}
        ]),
    );
    assert_eq!(
        source.relative_image_names().unwrap(),
        ["hint.png", "right.png"]
    );
    assert!(source.compile().is_ok());
    assert!(source.compile_with_images(&[]).is_err());
    let images = [binding("hint.png", 1), binding("right.png", 2)];
    assert!(source.compile_with_images(&images).is_ok());
    let duplicate = [images[0].clone(), images[0].clone(), images[1].clone()];
    assert!(source.compile_with_images(&duplicate).is_err());
}

#[test]
fn matching_ordering_and_blank_labels_are_authored_html() {
    for response in [
        json!({"kind":"matching", "prompts":[{"id":"a","text":"<b>A</b>"},{"id":"b","text":"<b>B</b>"}], "choices":[{"id":"x","text":"<i>X</i>"},{"id":"y","text":"<i>Y</i>"}], "matches":[{"prompt":"a","choice":"x"},{"prompt":"b","choice":"y"}]}),
        json!({"kind":"ordering", "items":[{"id":"a","text":"<b>A</b>"},{"id":"b","text":"<b>B</b>"},{"id":"c","text":"<b>C</b>"}], "correctOrder":["a","b","c"]}),
        json!({"kind":"multiFillIn", "blanks":[{"id":"a","label":"<b>A</b>","answers":["ATP"],"matchMode":"exact","maxLength":8}]}),
    ] {
        let source = PleQuestionJsonDocument::parse(&serde_json::to_vec(&json!({"format":"pleQuestionJson", "prompt":"<p>Respond</p>", "response":response})).unwrap()).unwrap();
        let compiled = source.compile_with_images(&[]).unwrap();
        let response = serde_json::to_string(compiled.presentation().response()).unwrap();
        assert!(response.contains("\"kind\":\"html\""));
        assert!(!response.contains("\"markdown\""));
    }
}

#[test]
fn bloom_evidence_retains_task_and_answer_mapping_without_teaching_feedback() {
    let source = document(
        "Which <em>base</em>?",
        json!([
            {"id":"a", "text":"Adenine", "feedback":"This is correct"},
            {"id":"b", "text":"Guanine"}
        ]),
    );
    let evidence = source.bloom_evidence().unwrap();
    assert_eq!(evidence["response"]["correctChoice"], "a");
    assert_eq!(evidence["response"]["choices"][0]["text"], "Adenine");
    assert_eq!(evidence["prompt"], "Which <em>base</em>?");
    assert!(evidence.get("questionHint").is_none());
    assert!(evidence["response"]["choices"][0].get("feedback").is_none());
}
