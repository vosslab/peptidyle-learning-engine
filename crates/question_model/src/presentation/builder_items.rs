//! Pending response-item assembly and public presentation projection.

use crate::answer::ResponseSelectionRule;
use crate::response::{
    MatchingChoice, MatchingPrompt, OrderingItem, QuestionChoice, QuestionResponseFormat,
    ResponseItemId,
};
use crate::{AuthorContentPresentation, QuestionContentBlock, QuestionVariationPresentation};

use super::super::assets::{content_assets, question_asset_rendition};
use super::super::choice_order::nonce_randomized_choices;
use super::super::model::{
    PresentedHotspotRegion, PresentedHotspotSurface, PresentedMatchingChoice,
    PresentedMatchingPrompt, PresentedOrderingItem, PresentedQuestionChoice,
    PresentedTextEntrySlot, QuestionAssetRendition, QuestionPresentation,
    QuestionPresentationNonce, QuestionPresentationResponseFormat,
};
use super::super::response_validation::selection_bounds;
use super::{
    NUMERIC_MAX_CHARACTERS, PendingHotspotRegionGeometry, PendingResponseItem,
    PresentationBuildError, ResponseItemBasis, ResponseItemBinding, ResponseItemRole,
};

pub(super) fn pending_items(
    presentation: &QuestionVariationPresentation,
    assets: &[QuestionAssetRendition],
    nonce: QuestionPresentationNonce,
) -> Result<Vec<PendingResponseItem>, PresentationBuildError> {
    let mut items = Vec::new();
    match &presentation.response {
        QuestionResponseFormat::MultipleChoice { choices, .. } => {
            let choices =
                if presentation.native_choice_order == crate::NativeChoiceOrder::NonceRandomized {
                    nonce_randomized_choices(choices, nonce)
                } else {
                    choices.clone()
                };
            push_choices(
                &mut items,
                &choices,
                ResponseItemRole::QuestionChoice,
                assets,
            )?;
        }
        QuestionResponseFormat::ShortText { .. } | QuestionResponseFormat::Numeric { .. } => {}
        QuestionResponseFormat::MultiBlank { blanks } => {
            for blank in blanks {
                push_item(
                    &mut items,
                    ResponseItemRole::TextEntrySlot,
                    blank.id.as_str(),
                    blank.label.clone(),
                    assets,
                    None,
                    Vec::new(),
                )?;
            }
        }
        QuestionResponseFormat::Matching { prompts, choices } => {
            push_choices(
                &mut items,
                prompts,
                ResponseItemRole::MatchingPrompt,
                assets,
            )?;
            push_choices(
                &mut items,
                choices,
                ResponseItemRole::MatchingChoice,
                assets,
            )?;
        }
        QuestionResponseFormat::Ordering { items: choices } => {
            push_choices(&mut items, choices, ResponseItemRole::OrderingItem, assets)?;
        }
        QuestionResponseFormat::Hotspot {
            question_asset_tuple,
            description,
            regions,
            ..
        } => {
            let binding = question_asset_rendition(question_asset_tuple, assets)?;
            let hotspot_regions = regions
                .iter()
                .map(pending_hotspot_region_geometry)
                .collect();
            push_item(
                &mut items,
                ResponseItemRole::HotspotSurface,
                &question_asset_tuple.question_asset_id.to_string(),
                vec![QuestionContentBlock::Image {
                    question_asset_tuple: question_asset_tuple.clone(),
                    description: description.clone(),
                }],
                assets,
                Some((
                    binding
                        .intrinsic_width
                        .ok_or(PresentationBuildError::InvalidPublicContent(
                            "hotspot surface lacks intrinsic dimensions",
                        ))?,
                    binding.intrinsic_height.ok_or(
                        PresentationBuildError::InvalidPublicContent(
                            "hotspot surface lacks intrinsic dimensions",
                        ),
                    )?,
                )),
                hotspot_regions,
            )?;
            for region in regions {
                push_item(
                    &mut items,
                    ResponseItemRole::HotspotRegion,
                    region.id.as_str(),
                    Vec::new(),
                    assets,
                    None,
                    Vec::new(),
                )?;
            }
        }
        QuestionResponseFormat::ImathasQuestionBackend {}
        | QuestionResponseFormat::BackendOwned {} => {}
    }
    Ok(items)
}

/// Derives the issued choice order from the durable presentation nonce before bindings are minted.
trait PresentedResponseItem {
    fn id(&self) -> &ResponseItemId;
    fn body(&self) -> &[QuestionContentBlock];
}
impl PresentedResponseItem for QuestionChoice {
    fn id(&self) -> &ResponseItemId {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}
impl PresentedResponseItem for MatchingPrompt {
    fn id(&self) -> &ResponseItemId {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}
impl PresentedResponseItem for MatchingChoice {
    fn id(&self) -> &ResponseItemId {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}
impl PresentedResponseItem for OrderingItem {
    fn id(&self) -> &ResponseItemId {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}
fn push_choices<T: PresentedResponseItem>(
    target: &mut Vec<PendingResponseItem>,
    choices: &[T],
    role: ResponseItemRole,
    assets: &[QuestionAssetRendition],
) -> Result<(), PresentationBuildError> {
    for choice in choices {
        push_item(
            target,
            role,
            choice.id().as_str(),
            choice.body().to_vec(),
            assets,
            None,
            Vec::new(),
        )?;
    }
    Ok(())
}
fn push_item(
    target: &mut Vec<PendingResponseItem>,
    role: ResponseItemRole,
    response_item_id: &str,
    content: Vec<QuestionContentBlock>,
    assets: &[QuestionAssetRendition],
    hotspot_dimensions: Option<(u32, u32)>,
    hotspot_regions: Vec<PendingHotspotRegionGeometry>,
) -> Result<(), PresentationBuildError> {
    if response_item_id.is_empty() {
        return Err(PresentationBuildError::InvalidPublicContent(
            "presentation item has an empty durable identity",
        ));
    }
    let ordinal = u32::try_from(target.len()).map_err(|_| PresentationBuildError::TooManyItems)?;
    let item_assets = content_assets(&content, assets)?;
    target.push(PendingResponseItem {
        role,
        ordinal,
        response_item_id: ResponseItemId::new(response_item_id),
        basis: ResponseItemBasis {
            role,
            ordinal,
            label: None,
            content,
            assets: item_assets,
            hotspot_width: hotspot_dimensions.map(|value| value.0),
            hotspot_height: hotspot_dimensions.map(|value| value.1),
            hotspot_regions,
        },
    });
    Ok(())
}
pub(super) fn public_presentation(
    source: &QuestionVariationPresentation,
    nonce: QuestionPresentationNonce,
    bindings: &[ResponseItemBinding],
) -> Result<QuestionPresentation, PresentationBuildError> {
    let by_role = |role| bindings.iter().filter(move |binding| binding.role == role);
    let presented_item_parts = |role: ResponseItemRole| {
        by_role(role)
            .map(|binding| {
                (
                    binding.presentation_response_item_id.clone(),
                    binding.basis.content.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    let response = match &source.response {
        QuestionResponseFormat::MultipleChoice {
            choices: source_choices,
            selection,
            ..
        } => match selection {
            ResponseSelectionRule::ExactlyOne => QuestionPresentationResponseFormat::SingleChoice {
                choices: presented_item_parts(ResponseItemRole::QuestionChoice)
                    .into_iter()
                    .map(|(id, body)| PresentedQuestionChoice { id, body })
                    .collect(),
            },
            _ => {
                let (minimum, maximum) = selection_bounds(*selection, source_choices.len())?;
                QuestionPresentationResponseFormat::MultipleAnswer {
                    choices: presented_item_parts(ResponseItemRole::QuestionChoice)
                        .into_iter()
                        .map(|(id, body)| PresentedQuestionChoice { id, body })
                        .collect(),
                    minimum,
                    maximum,
                }
            }
        },
        QuestionResponseFormat::ShortText { max_length, .. } => {
            QuestionPresentationResponseFormat::FillIn {
                max_characters: *max_length,
            }
        }
        QuestionResponseFormat::MultiBlank { blanks } => {
            let text_entry_bindings: Vec<_> = by_role(ResponseItemRole::TextEntrySlot).collect();
            if text_entry_bindings.len() != blanks.len() {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "blank presentation mapping is incomplete",
                ));
            }
            QuestionPresentationResponseFormat::MultiFillIn {
                blanks: blanks
                    .iter()
                    .zip(text_entry_bindings)
                    .map(|(blank, binding)| PresentedTextEntrySlot {
                        id: binding.presentation_response_item_id.clone(),
                        label: blank.label.clone(),
                        max_characters: blank.max_length,
                    })
                    .collect(),
            }
        }
        QuestionResponseFormat::Numeric { unit, .. } => {
            QuestionPresentationResponseFormat::Numerical {
                max_characters: NUMERIC_MAX_CHARACTERS,
                displayed_unit: unit.clone(),
            }
        }
        QuestionResponseFormat::Matching { .. } => QuestionPresentationResponseFormat::Matching {
            prompts: presented_item_parts(ResponseItemRole::MatchingPrompt)
                .into_iter()
                .map(|(id, body)| PresentedMatchingPrompt { id, body })
                .collect(),
            choices: presented_item_parts(ResponseItemRole::MatchingChoice)
                .into_iter()
                .map(|(id, body)| PresentedMatchingChoice { id, body })
                .collect(),
            reuse_choices: false,
        },
        QuestionResponseFormat::Ordering { .. } => QuestionPresentationResponseFormat::Ordering {
            items: presented_item_parts(ResponseItemRole::OrderingItem)
                .into_iter()
                .map(|(id, body)| PresentedOrderingItem { id, body })
                .collect(),
        },
        QuestionResponseFormat::Hotspot {
            regions, selection, ..
        } => {
            let surface = bindings
                .iter()
                .find(|binding| binding.role == ResponseItemRole::HotspotSurface)
                .ok_or(PresentationBuildError::InvalidPublicContent(
                    "hotspot surface mapping is absent",
                ))?;
            let QuestionContentBlock::Image {
                question_asset_tuple,
                description,
            } = &surface.basis.content[0]
            else {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "hotspot surface is not image-backed",
                ));
            };
            let (minimum, maximum) = selection_bounds(*selection, regions.len())?;
            let hotspot_region_bindings: Vec<_> =
                by_role(ResponseItemRole::HotspotRegion).collect();
            if hotspot_region_bindings.len() != regions.len() {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "hotspot region presentation mapping is incomplete",
                ));
            }
            QuestionPresentationResponseFormat::Hotspot {
                surface: PresentedHotspotSurface {
                    id: surface.presentation_response_item_id.clone(),
                    question_asset_tuple: question_asset_tuple.clone(),
                    description: description.clone(),
                    regions: regions
                        .iter()
                        .zip(hotspot_region_bindings)
                        .map(|(region, binding)| PresentedHotspotRegion {
                            id: binding.presentation_response_item_id.clone(),
                            label: region.label.clone(),
                            x: region.x,
                            y: region.y,
                            width: region.width,
                            height: region.height,
                        })
                        .collect(),
                },
                minimum,
                maximum,
            }
        }
        QuestionResponseFormat::ImathasQuestionBackend {} => {
            QuestionPresentationResponseFormat::ImathasQuestionBackend {}
        }
        QuestionResponseFormat::BackendOwned {} => {
            QuestionPresentationResponseFormat::BackendOwned {}
        }
    };
    Ok(QuestionPresentation {
        question_revision_tuple: source.variation.question_revision_tuple.clone(),
        presentation_nonce: nonce,
        author_content_digest: source
            .author_content
            .as_ref()
            .map(AuthorContentPresentation::digest),
        question_title: source.question_title.clone(),
        prompt: source.prompt.clone(),
        response,
    })
}

pub fn pending_hotspot_region_geometry(
    region: &impl HotspotRegionGeometry,
) -> PendingHotspotRegionGeometry {
    PendingHotspotRegionGeometry {
        label: region.label().clone(),
        x: region.x(),
        y: region.y(),
        width: region.width(),
        height: region.height(),
    }
}

pub trait HotspotRegionGeometry {
    fn label(&self) -> &Vec<QuestionContentBlock>;
    fn x(&self) -> u16;
    fn y(&self) -> u16;
    fn width(&self) -> u16;
    fn height(&self) -> u16;
}

impl HotspotRegionGeometry for crate::response::HotspotRegion {
    fn label(&self) -> &Vec<QuestionContentBlock> {
        &self.label
    }
    fn x(&self) -> u16 {
        self.x
    }
    fn y(&self) -> u16 {
        self.y
    }
    fn width(&self) -> u16 {
        self.width
    }
    fn height(&self) -> u16 {
        self.height
    }
}

impl HotspotRegionGeometry for PresentedHotspotRegion {
    fn label(&self) -> &Vec<QuestionContentBlock> {
        &self.label
    }
    fn x(&self) -> u16 {
        self.x
    }
    fn y(&self) -> u16 {
        self.y
    }
    fn width(&self) -> u16 {
        self.width
    }
    fn height(&self) -> u16 {
        self.height
    }
}
