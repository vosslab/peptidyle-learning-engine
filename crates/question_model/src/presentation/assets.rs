//! Validation and selection of assets presented with one issued Question.

use std::collections::{BTreeMap, BTreeSet};

use crate::envelope::{
    QuestionAssetReference, QuestionContentBlock, QuestionVariationPresentation,
};
use crate::response::QuestionResponseFormat;

use super::builder::PresentationBuildError;
use super::model::{
    PresentedResponseItemContent, QuestionAssetRendition, QuestionPresentation,
    QuestionPresentationResponseFormat,
};

pub(super) fn validate_assets(
    envelope: &QuestionVariationPresentation,
    bindings: &[QuestionAssetRendition],
) -> Result<Vec<QuestionAssetRendition>, PresentationBuildError> {
    let mut referenced = BTreeSet::new();
    collect_assets(&envelope.prompt, &mut referenced);
    collect_response_assets(&envelope.response, &mut referenced);
    validate_asset_refs(&referenced, bindings)
}

pub(super) fn validate_public_assets(
    envelope: &QuestionPresentation,
    bindings: &[QuestionAssetRendition],
) -> Result<Vec<QuestionAssetRendition>, PresentationBuildError> {
    let mut referenced = BTreeSet::new();
    collect_assets(&envelope.prompt, &mut referenced);
    match &envelope.response {
        QuestionPresentationResponseFormat::SingleChoice { choices }
        | QuestionPresentationResponseFormat::MultipleAnswer { choices, .. } => {
            collect_presented_response_item_assets(choices, &mut referenced);
        }
        QuestionPresentationResponseFormat::MultiFillIn { blanks } => {
            for blank in blanks {
                collect_assets(&blank.label, &mut referenced);
            }
        }
        QuestionPresentationResponseFormat::Matching {
            prompts, choices, ..
        } => {
            collect_presented_response_item_assets(prompts, &mut referenced);
            collect_presented_response_item_assets(choices, &mut referenced);
        }
        QuestionPresentationResponseFormat::Ordering { items } => {
            collect_presented_response_item_assets(items, &mut referenced);
        }
        QuestionPresentationResponseFormat::Hotspot { surface, .. } => {
            referenced.insert(AssetRefKey::from(&surface.asset));
            for region in &surface.regions {
                collect_assets(&region.label, &mut referenced);
            }
        }
        QuestionPresentationResponseFormat::FillIn { .. }
        | QuestionPresentationResponseFormat::Numerical { .. } => {}
    }
    validate_asset_refs(&referenced, bindings)
}

fn collect_presented_response_item_assets<T: PresentedResponseItemContent>(
    items: &[T],
    referenced: &mut BTreeSet<AssetRefKey>,
) {
    for item in items {
        collect_assets(item.presentation_item_body(), referenced);
    }
}

pub(super) fn content_assets(
    content: &[QuestionContentBlock],
    bindings: &[QuestionAssetRendition],
) -> Result<Vec<QuestionAssetRendition>, PresentationBuildError> {
    let mut referenced = BTreeSet::new();
    collect_assets(content, &mut referenced);
    referenced
        .iter()
        .map(|reference| {
            bindings
                .iter()
                .find(|binding| {
                    binding.question_asset.asset == reference.asset
                        && binding.question_asset.checksum == reference.checksum
                })
                .cloned()
                .ok_or(PresentationBuildError::InvalidPublicContent(
                    "presentation asset binding is missing or mismatched",
                ))
        })
        .collect()
}

pub(super) fn question_asset_rendition<'a>(
    reference: &QuestionAssetReference,
    bindings: &'a [QuestionAssetRendition],
) -> Result<&'a QuestionAssetRendition, PresentationBuildError> {
    bindings
        .iter()
        .find(|binding| {
            binding.question_asset.asset == reference.asset
                && binding.question_asset.checksum == reference.checksum
        })
        .ok_or(PresentationBuildError::InvalidPublicContent(
            "presentation asset binding is missing or mismatched",
        ))
}

fn validate_asset_refs(
    referenced: &BTreeSet<AssetRefKey>,
    bindings: &[QuestionAssetRendition],
) -> Result<Vec<QuestionAssetRendition>, PresentationBuildError> {
    let mut by_id = BTreeMap::new();
    for binding in bindings {
        if by_id
            .insert(binding.question_asset.asset, binding)
            .is_some()
            || !is_sha256(&binding.question_asset.checksum)
            || !is_sha256(&binding.rendition_checksum)
            || binding.intrinsic_width.is_some() != binding.intrinsic_height.is_some()
            || binding.intrinsic_width == Some(0)
            || binding.intrinsic_height == Some(0)
        {
            return Err(PresentationBuildError::InvalidPublicContent(
                "presentation asset binding is malformed",
            ));
        }
    }
    for reference in referenced {
        let binding =
            by_id
                .get(&reference.asset)
                .ok_or(PresentationBuildError::InvalidPublicContent(
                    "presentation asset binding is missing",
                ))?;
        if binding.question_asset.checksum != reference.checksum {
            return Err(PresentationBuildError::InvalidPublicContent(
                "presentation asset checksum does not match the question",
            ));
        }
    }
    if by_id
        .keys()
        .any(|asset| !referenced.iter().any(|value| value.asset == *asset))
    {
        return Err(PresentationBuildError::InvalidPublicContent(
            "presentation contains an unreferenced asset binding",
        ));
    }
    let mut values = bindings.to_vec();
    values.sort_by_key(|binding| binding.question_asset.asset);
    Ok(values)
}

fn collect_assets(content: &[QuestionContentBlock], target: &mut BTreeSet<AssetRefKey>) {
    for block in content {
        if let QuestionContentBlock::Image { asset, .. } = block {
            target.insert(AssetRefKey::from(asset));
        }
    }
}

fn collect_response_assets(response: &QuestionResponseFormat, target: &mut BTreeSet<AssetRefKey>) {
    match response {
        QuestionResponseFormat::MultipleChoice { choices, .. } => {
            for choice in choices {
                collect_assets(&choice.body, target);
            }
        }
        QuestionResponseFormat::MultiBlank { blanks } => {
            for blank in blanks {
                collect_assets(&blank.label, target);
            }
        }
        QuestionResponseFormat::Matching { prompts, choices } => {
            for choice in prompts {
                collect_assets(&choice.body, target);
            }
            for choice in choices {
                collect_assets(&choice.body, target);
            }
        }
        QuestionResponseFormat::Ordering { items } => {
            for item in items {
                collect_assets(&item.body, target);
            }
        }
        QuestionResponseFormat::Hotspot {
            surface, regions, ..
        } => {
            target.insert(AssetRefKey::from(surface));
            for region in regions {
                collect_assets(&region.label, target);
            }
        }
        QuestionResponseFormat::Numeric { .. }
        | QuestionResponseFormat::ShortText { .. }
        | QuestionResponseFormat::ExternalTool {} => {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AssetRefKey {
    asset: crate::QuestionAssetId,
    checksum: String,
}

impl From<&QuestionAssetReference> for AssetRefKey {
    fn from(value: &QuestionAssetReference) -> Self {
        Self {
            asset: value.asset,
            checksum: value.checksum.clone(),
        }
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
