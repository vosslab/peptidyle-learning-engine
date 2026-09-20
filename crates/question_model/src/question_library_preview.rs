//! Answer-free response presentation for inspecting a Published Question.

use serde::{Deserialize, Serialize};

use crate::answer::ResponseSelectionRule;
use crate::{QuestionContentBlock, QuestionImageAssetTuple, QuestionResponseFormat};

/// Public geometry and label of an inert Hotspot preview region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPreviewRegion {
    pub label: Vec<QuestionContentBlock>,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Inspection-only controls: no response identities, grading policies, or answer data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum QuestionResponsePreview {
    Numeric {
        unit: Option<String>,
    },
    MultipleChoice {
        choices: Vec<Vec<QuestionContentBlock>>,
        selection: ResponseSelectionRule,
    },
    ShortText {},
    MultiBlank {
        labels: Vec<Vec<QuestionContentBlock>>,
    },
    Matching {
        prompts: Vec<Vec<QuestionContentBlock>>,
        choices: Vec<Vec<QuestionContentBlock>>,
    },
    Ordering {
        items: Vec<Vec<QuestionContentBlock>>,
    },
    Hotspot {
        question_image_asset_tuple: QuestionImageAssetTuple,
        description: String,
        regions: Vec<QuestionPreviewRegion>,
        selection: ResponseSelectionRule,
    },
}

impl QuestionResponsePreview {
    /// Projects only presentation content; backend-owned documents have their own preview path.
    pub fn from_native_response(response: &QuestionResponseFormat) -> Option<Self> {
        Some(match response {
            QuestionResponseFormat::Numeric { unit, .. } => Self::Numeric { unit: unit.clone() },
            QuestionResponseFormat::MultipleChoice { choices, selection } => Self::MultipleChoice {
                choices: choices.iter().map(|choice| choice.body.clone()).collect(),
                selection: *selection,
            },
            QuestionResponseFormat::ShortText { .. } => Self::ShortText {},
            QuestionResponseFormat::MultiBlank { blanks } => Self::MultiBlank {
                labels: blanks.iter().map(|blank| blank.label.clone()).collect(),
            },
            QuestionResponseFormat::Matching { prompts, choices } => Self::Matching {
                prompts: prompts.iter().map(|prompt| prompt.body.clone()).collect(),
                choices: choices.iter().map(|choice| choice.body.clone()).collect(),
            },
            QuestionResponseFormat::Ordering { items } => Self::Ordering {
                items: items.iter().map(|item| item.body.clone()).collect(),
            },
            QuestionResponseFormat::Hotspot {
                question_image_asset_tuple,
                description,
                regions,
                selection,
            } => Self::Hotspot {
                question_image_asset_tuple: question_image_asset_tuple.clone(),
                description: description.clone(),
                regions: regions
                    .iter()
                    .map(|region| QuestionPreviewRegion {
                        label: region.label.clone(),
                        x: region.x,
                        y: region.y,
                        width: region.width,
                        height: region.height,
                    })
                    .collect(),
                selection: *selection,
            },
            QuestionResponseFormat::ImathasQuestionBackend {}
            | QuestionResponseFormat::BackendOwned {} => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_preview_retains_content_without_author_ids_or_grading_policy() {
        let source: QuestionResponseFormat = serde_json::from_value(serde_json::json!({
            "kind": "multipleChoice",
            "choices": [{ "id": "private-correct-choice", "body": [{ "kind": "text", "markdown": "Visible choice" }] }],
            "selection": { "kind": "exactlyOne" }
        })).expect("valid response fixture");
        let preview =
            QuestionResponsePreview::from_native_response(&source).expect("native preview");
        assert_eq!(
            serde_json::to_value(preview).expect("serializes"),
            serde_json::json!({
                "kind": "multipleChoice",
                "choices": [[{ "kind": "text", "markdown": "Visible choice" }]],
                "selection": { "kind": "exactlyOne" }
            })
        );
        let source: QuestionResponseFormat = serde_json::from_value(serde_json::json!({
            "kind": "shortText", "matchMode": "exact", "maxLength": 120
        }))
        .expect("valid response fixture");
        let preview =
            QuestionResponsePreview::from_native_response(&source).expect("native preview");
        assert_eq!(
            serde_json::to_value(preview).expect("serializes"),
            serde_json::json!({ "kind": "shortText" })
        );
        assert!(
            QuestionResponsePreview::from_native_response(&QuestionResponseFormat::BackendOwned {})
                .is_none()
        );
    }
}
