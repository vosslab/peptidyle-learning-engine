//! Validation and response-item bindings for an already-public presentation.

use crate::QuestionContentBlock;

use super::assets::{content_assets, question_asset_rendition};
use super::builder::{
    PendingHotspotRegionGeometry, PresentationBuildError, ResponseItemBasis, ResponseItemBinding,
    ResponseItemRole, pending_hotspot_region_geometry,
};
use super::model::{
    PresentationResponseItemReference, PresentedResponseItemContent, QuestionAssetRendition,
    QuestionPresentationResponseFormat,
};
use super::response_validation::validate_regions;

pub(super) fn public_item_bindings(
    response: &QuestionPresentationResponseFormat,
    assets: &[QuestionAssetRendition],
) -> Result<Vec<ResponseItemBinding>, PresentationBuildError> {
    let mut target = Vec::new();
    match response {
        QuestionPresentationResponseFormat::SingleChoice { choices }
        | QuestionPresentationResponseFormat::MultipleAnswer { choices, .. } => {
            push_public_response_items(
                &mut target,
                choices,
                ResponseItemRole::QuestionChoice,
                assets,
            )?;
        }
        QuestionPresentationResponseFormat::FillIn { max_characters } => {
            require_positive(*max_characters, "fill-in maximum must be positive")?;
        }
        QuestionPresentationResponseFormat::MultiFillIn { blanks } => {
            if blanks.is_empty() {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "multi-fill presentation has no blanks",
                ));
            }
            for blank in blanks {
                require_positive(blank.max_characters, "blank maximum must be positive")?;
                push_public_item(
                    &mut target,
                    blank.id.clone(),
                    ResponseItemRole::TextEntrySlot,
                    blank.label.clone(),
                    assets,
                    None,
                    Vec::new(),
                )?;
            }
        }
        QuestionPresentationResponseFormat::Numerical { max_characters, .. } => {
            require_positive(*max_characters, "numeric maximum must be positive")?;
        }
        QuestionPresentationResponseFormat::Matching {
            prompts,
            choices,
            reuse_choices,
        } => {
            if prompts.is_empty()
                || choices.is_empty()
                || (!reuse_choices && prompts.len() > choices.len())
            {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "matching presentation has incompatible sides",
                ));
            }
            push_public_response_items(
                &mut target,
                prompts,
                ResponseItemRole::MatchingPrompt,
                assets,
            )?;
            push_public_response_items(
                &mut target,
                choices,
                ResponseItemRole::MatchingChoice,
                assets,
            )?;
        }
        QuestionPresentationResponseFormat::Ordering { items } => {
            if items.len() < 2 {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "ordering presentation requires at least two items",
                ));
            }
            push_public_response_items(&mut target, items, ResponseItemRole::OrderingItem, assets)?;
        }
        QuestionPresentationResponseFormat::Hotspot {
            surface,
            minimum,
            maximum,
        } => {
            validate_public_bounds(*minimum, *maximum, u32::MAX)?;
            let binding = question_asset_rendition(&surface.question_asset, assets)?;
            let dimensions = Some((
                binding
                    .intrinsic_width
                    .ok_or(PresentationBuildError::InvalidPublicContent(
                        "hotspot surface lacks intrinsic dimensions",
                    ))?,
                binding
                    .intrinsic_height
                    .ok_or(PresentationBuildError::InvalidPublicContent(
                        "hotspot surface lacks intrinsic dimensions",
                    ))?,
            ));
            validate_regions(&surface.regions)?;
            push_public_item(
                &mut target,
                surface.id.clone(),
                ResponseItemRole::HotspotSurface,
                vec![QuestionContentBlock::Image {
                    question_asset: surface.question_asset.clone(),
                    description: surface.description.clone(),
                }],
                assets,
                dimensions,
                surface
                    .regions
                    .iter()
                    .map(pending_hotspot_region_geometry)
                    .collect(),
            )?;
            for region in &surface.regions {
                push_public_item(
                    &mut target,
                    region.id.clone(),
                    ResponseItemRole::HotspotRegion,
                    Vec::new(),
                    assets,
                    None,
                    Vec::new(),
                )?;
            }
        }
        QuestionPresentationResponseFormat::ImathasQuestionBackend {} => {}
    }
    match response {
        QuestionPresentationResponseFormat::SingleChoice { choices } if choices.len() < 2 => {
            return Err(PresentationBuildError::InvalidPublicContent(
                "single-choice presentation requires at least two choices",
            ));
        }
        QuestionPresentationResponseFormat::MultipleAnswer {
            choices,
            minimum,
            maximum,
        } => validate_public_bounds(
            *minimum,
            *maximum,
            u32::try_from(choices.len()).map_err(|_| PresentationBuildError::TooManyItems)?,
        )?,
        _ => {}
    }
    Ok(target)
}

fn push_public_response_items<T: PresentedResponseItemContent>(
    target: &mut Vec<ResponseItemBinding>,
    items: &[T],
    role: ResponseItemRole,
    assets: &[QuestionAssetRendition],
) -> Result<(), PresentationBuildError> {
    for item in items {
        push_public_item(
            target,
            item.presentation_item_id().clone(),
            role,
            item.presentation_item_body().to_vec(),
            assets,
            None,
            Vec::new(),
        )?;
    }
    Ok(())
}

fn push_public_item(
    target: &mut Vec<ResponseItemBinding>,
    presentation_response_item_reference: PresentationResponseItemReference,
    role: ResponseItemRole,
    content: Vec<QuestionContentBlock>,
    assets: &[QuestionAssetRendition],
    hotspot_dimensions: Option<(u32, u32)>,
    hotspot_regions: Vec<PendingHotspotRegionGeometry>,
) -> Result<(), PresentationBuildError> {
    let ordinal = u32::try_from(target.len()).map_err(|_| PresentationBuildError::TooManyItems)?;
    let item_assets = content_assets(&content, assets)?;
    target.push(ResponseItemBinding {
        presentation_response_item_reference,
        role,
        ordinal,
        response_item_reference: None,
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

fn require_positive(value: u32, message: &'static str) -> Result<(), PresentationBuildError> {
    if value == 0 {
        Err(PresentationBuildError::InvalidPublicContent(message))
    } else {
        Ok(())
    }
}

fn validate_public_bounds(
    minimum: u32,
    maximum: u32,
    available: u32,
) -> Result<(), PresentationBuildError> {
    if minimum > maximum || maximum > available {
        Err(PresentationBuildError::InvalidPublicContent(
            "presentation selection bounds are invalid",
        ))
    } else {
        Ok(())
    }
}
