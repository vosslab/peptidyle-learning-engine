//! Safe readable projection for an exact completed Student response.

use question_model::{
    QuestionContentBlock,
    presentation::{
        PresentedResponseItemContent, QuestionPresentationResponseFormat, StudentResponseInspection,
    },
};

/// Produces readable public blocks only when every submitted response-item ID
/// resolves in the exact reproduced presentation. Unknown IDs fail closed
/// rather than exposing an opaque identifier or guessing at authored content.
pub(super) fn project(
    response: StudentResponseInspection,
    format: &QuestionPresentationResponseFormat,
) -> Option<Vec<QuestionContentBlock>> {
    match (response, format) {
        (
            StudentResponseInspection::Numeric { value },
            QuestionPresentationResponseFormat::Numerical { .. },
        ) if value.is_finite() => Some(text(value.to_string())),
        (
            StudentResponseInspection::ShortText { text: value },
            QuestionPresentationResponseFormat::FillIn { .. },
        ) => Some(text(value)),
        (
            StudentResponseInspection::MultipleChoice { selected },
            QuestionPresentationResponseFormat::SingleChoice { choices },
        )
        | (
            StudentResponseInspection::MultipleChoice { selected },
            QuestionPresentationResponseFormat::MultipleAnswer { choices, .. },
        ) => selected_bodies(choices, &selected),
        (
            StudentResponseInspection::Ordering { order },
            QuestionPresentationResponseFormat::Ordering { items },
        ) => selected_bodies(items, &order),
        (
            StudentResponseInspection::MultiBlank { answers },
            QuestionPresentationResponseFormat::MultiFillIn { blanks },
        ) => {
            let rows = answers
                .iter()
                .map(|answer| {
                    let blank = blanks.iter().find(|blank| blank.id == answer.slot)?;
                    Some(vec![block_text(&blank.label), answer.text.clone()])
                })
                .collect::<Option<Vec<_>>>()?;
            Some(vec![QuestionContentBlock::Table {
                headers: vec!["Blank".to_string(), "Your response".to_string()],
                rows,
                description: "Your responses for each blank".to_string(),
            }])
        }
        (
            StudentResponseInspection::Matching { matches },
            QuestionPresentationResponseFormat::Matching {
                prompts, choices, ..
            },
        ) => {
            let mut answered = std::collections::BTreeMap::new();
            for pair in &matches {
                if answered
                    .insert(pair.prompt.clone(), pair.choice.clone())
                    .is_some()
                {
                    return None;
                }
            }
            if answered
                .keys()
                .any(|prompt_id| !prompts.iter().any(|prompt| prompt.id == *prompt_id))
            {
                return None;
            }
            let rows = prompts
                .iter()
                .map(|prompt| {
                    let response = match answered.get(&prompt.id) {
                        Some(choice_id) => {
                            block_text(&choices.iter().find(|choice| choice.id == *choice_id)?.body)
                        }
                        None => "Unanswered".to_string(),
                    };
                    Some(vec![block_text(&prompt.body), response])
                })
                .collect::<Option<Vec<_>>>()?;
            Some(vec![QuestionContentBlock::Table {
                headers: vec!["Prompt".to_string(), "Your match".to_string()],
                rows,
                description: "Your matching response".to_string(),
            }])
        }
        (
            StudentResponseInspection::Hotspot { selected_regions },
            QuestionPresentationResponseFormat::Hotspot { surface, .. },
        ) => selected_regions
            .iter()
            .map(|id| surface.regions.iter().find(|region| region.id == *id))
            .collect::<Option<Vec<_>>>()
            .map(|regions| {
                regions
                    .into_iter()
                    .flat_map(|region| region.label.clone())
                    .collect()
            }),
        (
            StudentResponseInspection::ImathasQuestionBackend { .. },
            QuestionPresentationResponseFormat::ImathasQuestionBackend {},
        ) => Some(text(
            "Your iMathAS Question Backend response was recorded.".to_string(),
        )),
        (
            StudentResponseInspection::BackendOwned { .. },
            QuestionPresentationResponseFormat::BackendOwned {},
        ) => Some(text(
            "Your Question Backend response was recorded.".to_string(),
        )),
        _ => None,
    }
}

fn selected_bodies<T: PresentedResponseItemContent>(
    items: &[T],
    selected: &[question_model::presentation::PresentationResponseItemId],
) -> Option<Vec<QuestionContentBlock>> {
    selected
        .iter()
        .map(|id| items.iter().find(|item| item.presentation_item_id() == id))
        .collect::<Option<Vec<_>>>()
        .map(|items| {
            items
                .into_iter()
                .flat_map(|item| item.presentation_item_body().to_vec())
                .collect()
        })
}

fn text(markdown: String) -> Vec<QuestionContentBlock> {
    vec![QuestionContentBlock::Text { markdown }]
}

fn block_text(blocks: &[QuestionContentBlock]) -> String {
    blocks
        .iter()
        .map(|block| match block {
            QuestionContentBlock::Text { markdown } => markdown.clone(),
            QuestionContentBlock::Math { description, .. }
            | QuestionContentBlock::Image { description, .. }
            | QuestionContentBlock::Table { description, .. } => description.clone(),
            QuestionContentBlock::Code { source, .. } => source.clone(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}
