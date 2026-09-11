//! Safe readable projection for an exact completed Student response.

use question_model::{
    QuestionContentBlock,
    presentation::{
        PresentedResponseItemContent, QuestionPresentationResponseFormat, StudentResponseInspection,
    },
};

/// Produces readable public blocks only when every submitted response reference
/// resolves in the exact reproduced presentation. Unknown references fail closed
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
            let rows = matches
                .iter()
                .map(|pair| {
                    let prompt = prompts.iter().find(|prompt| prompt.id == pair.prompt)?;
                    let choice = choices.iter().find(|choice| choice.id == pair.choice)?;
                    Some(vec![block_text(&prompt.body), block_text(&choice.body)])
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
        _ => None,
    }
}

fn selected_bodies<T: PresentedResponseItemContent>(
    items: &[T],
    selected: &[question_model::presentation::PresentationResponseItemReference],
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

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::presentation::{
        PresentationResponseItemReference, PresentedQuestionChoice,
    };

    fn reference(value: &str) -> PresentationResponseItemReference {
        PresentationResponseItemReference::parse(value).expect("valid presentation reference")
    }

    fn single_choice() -> QuestionPresentationResponseFormat {
        QuestionPresentationResponseFormat::SingleChoice {
            choices: vec![PresentedQuestionChoice {
                id: reference("a101"),
                body: text("Readable selected answer".to_string()),
            }],
        }
    }

    #[test]
    fn selected_pinned_choice_becomes_readable_content_without_feedback() {
        let response = StudentResponseInspection::MultipleChoice {
            selected: vec![reference("a101")],
        };

        assert_eq!(
            project(response, &single_choice()),
            Some(text("Readable selected answer".to_string()))
        );
    }

    #[test]
    fn unmatched_reference_omits_response_instead_of_disclosing_opaque_identifier() {
        let response = StudentResponseInspection::MultipleChoice {
            selected: vec![reference("beef")],
        };

        assert_eq!(project(response, &single_choice()), None);
    }
}
