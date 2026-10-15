//! Converts retained Question Image Rendition evidence into renderer-safe bindings.

use learning_data_access::ReadyQuestionImageRendition;
use question_model::{QuestionImageAssetTuple, QuestionImageRendition};

pub(crate) fn question_image_renditions_from_ready(
    renditions: &[ReadyQuestionImageRendition],
) -> Vec<QuestionImageRendition> {
    renditions
        .iter()
        .map(|rendition| QuestionImageRendition {
            question_image_asset_tuple: QuestionImageAssetTuple {
                question_image_asset_id: rendition.question_image_asset_id,
                checksum: rendition.question_image_checksum.clone(),
            },
            rendition_checksum: rendition.rendition_checksum.clone(),
            intrinsic_width: Some(rendition.intrinsic_width),
            intrinsic_height: Some(rendition.intrinsic_height),
        })
        .collect()
}

/// Private publication name registry used only while compiling exact Native JSON.
pub(crate) fn question_html_image_bindings(
    renditions: &[ReadyQuestionImageRendition],
) -> Vec<question_model::QuestionHtmlImageBinding> {
    renditions
        .iter()
        .filter_map(|rendition| {
            rendition.source_relative_name.as_ref().map(|name| {
                question_model::QuestionHtmlImageBinding {
                    source_relative_name: name.clone(),
                    question_image_asset_tuple: QuestionImageAssetTuple {
                        question_image_asset_id: rendition.question_image_asset_id,
                        checksum: rendition.question_image_checksum.clone(),
                    },
                }
            })
        })
        .collect()
}

/// Retains only prompt/response assets in the initially issued browser content.
pub(crate) fn presentation_renditions(
    presentation: &question_model::QuestionVariationPresentation,
    renditions: &[ReadyQuestionImageRendition],
) -> Vec<ReadyQuestionImageRendition> {
    let mut ids = std::collections::BTreeSet::new();
    fn collect(
        blocks: &[question_model::QuestionContentBlock],
        ids: &mut std::collections::BTreeSet<question_model::QuestionImageAssetId>,
    ) {
        for block in blocks {
            match block {
                question_model::QuestionContentBlock::Image {
                    question_image_asset_tuple,
                    ..
                } => {
                    ids.insert(question_image_asset_tuple.question_image_asset_id);
                }
                question_model::QuestionContentBlock::Html { images, .. } => {
                    ids.extend(
                        images
                            .iter()
                            .map(|image| image.question_image_asset_tuple.question_image_asset_id),
                    );
                }
                _ => {}
            }
        }
    }
    collect(&presentation.prompt, &mut ids);
    match &presentation.response {
        question_model::QuestionResponseFormat::MultipleChoice { choices, .. } => {
            for item in choices {
                collect(&item.body, &mut ids);
            }
        }
        question_model::QuestionResponseFormat::MultiBlank { blanks } => {
            for item in blanks {
                collect(&item.label, &mut ids);
            }
        }
        question_model::QuestionResponseFormat::Matching { prompts, choices } => {
            for item in prompts {
                collect(&item.body, &mut ids);
            }
            for item in choices {
                collect(&item.body, &mut ids);
            }
        }
        question_model::QuestionResponseFormat::Ordering { items } => {
            for item in items {
                collect(&item.body, &mut ids);
            }
        }
        question_model::QuestionResponseFormat::Hotspot {
            question_image_asset_tuple,
            regions,
            ..
        } => {
            ids.insert(question_image_asset_tuple.question_image_asset_id);
            for item in regions {
                collect(&item.label, &mut ids);
            }
        }
        _ => {}
    }
    renditions
        .iter()
        .filter(|rendition| ids.contains(&rendition.question_image_asset_id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{
        PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionReproduction,
        QuestionRevisionNumber, QuestionVariation, QuestionVariationPresentation,
    };

    #[test]
    fn initial_presentation_retains_only_displayed_images() {
        let registry = [
            rendition("prompt.png", 1),
            rendition("choice.png", 2),
            rendition("hint.png", 3),
        ];
        let source = serde_json::json!({
            "format":"pleQuestionJson", "prompt":"<img src='prompt.png'>Select",
            "response":{"kind":"singleChoice", "choices":[{"id":"a","text":"<img src='choice.png'>A"},{"id":"b","text":"B"}], "correctChoice":"a"},
            "questionHint":"<img src='hint.png'>Hint"
        });
        let document = adapter_ple::question_json::PleQuestionJsonDocument::parse(
            &serde_json::to_vec(&source).unwrap(),
        )
        .unwrap();
        let compiled = document
            .compile_with_images(&question_html_image_bindings(&registry))
            .unwrap();
        let presentation = QuestionVariationPresentation {
            variation: QuestionVariation::from_question_revision_and_reproduction(
                PublishedQuestionRevisionTuple {
                    published_question_id: PublishedQuestionId::from_random_identifier("1234567")
                        .unwrap(),
                    revision_number: QuestionRevisionNumber::new(1).unwrap(),
                },
                QuestionReproduction::Static,
            ),
            question_title: "Images".to_owned(),
            prompt: compiled.presentation().prompt().to_vec(),
            response: compiled.presentation().response().clone(),
            native_choice_order: compiled.presentation().native_choice_order(),
            author_content: None,
        };
        let retained = presentation_renditions(&presentation, &registry);
        assert_eq!(retained, registry[..2]);
        let built = question_model::presentation::build_question_presentation(
            &presentation,
            &question_image_renditions_from_ready(&retained),
        )
        .unwrap();
        assert_eq!(built.question_image_renditions.len(), 2);
        assert!(
            !serde_json::to_string(&built.presentation)
                .unwrap()
                .contains("hint.png")
        );
        assert!(
            question_model::presentation::build_question_presentation(
                &presentation,
                &question_image_renditions_from_ready(&registry)
            )
            .is_err()
        );
    }

    fn rendition(name: &str, id: u128) -> ReadyQuestionImageRendition {
        ReadyQuestionImageRendition {
            source_relative_name: Some(name.to_owned()),
            question_image_asset_id: question_model::QuestionImageAssetId::from_uuid(
                uuid::Uuid::from_u128(id),
            ),
            question_image_checksum: "a".repeat(64),
            rendition_checksum: "b".repeat(64),
            intrinsic_width: 24,
            intrinsic_height: 24,
        }
    }
}
