//! Validation and selection of assets presented with one issued Question.

use std::collections::{BTreeMap, BTreeSet};

use crate::envelope::{AssetRef, ContentBlock, QuestionPresentation};
use crate::response::QuestionResponseFormat;

use super::builder::PresentationBuildError;
use super::model::{
    IssuedQuestionResponseFormatV1, PresentationEnvelopeV1, PresentedQuestionAsset,
};

pub(super) fn validate_assets(
    envelope: &QuestionPresentation,
    bindings: &[PresentedQuestionAsset],
) -> Result<Vec<PresentedQuestionAsset>, PresentationBuildError> {
    let mut referenced = BTreeSet::new();
    collect_assets(&envelope.prompt, &mut referenced);
    collect_response_assets(&envelope.response, &mut referenced);
    validate_asset_refs(&referenced, bindings)
}

pub(super) fn validate_public_assets(
    envelope: &PresentationEnvelopeV1,
    bindings: &[PresentedQuestionAsset],
) -> Result<Vec<PresentedQuestionAsset>, PresentationBuildError> {
    let mut referenced = BTreeSet::new();
    collect_assets(&envelope.prompt, &mut referenced);
    match &envelope.response {
        IssuedQuestionResponseFormatV1::SingleChoice { choices }
        | IssuedQuestionResponseFormatV1::MultipleAnswer { choices, .. } => {
            for choice in choices {
                collect_assets(&choice.body, &mut referenced);
            }
        }
        IssuedQuestionResponseFormatV1::MultiFillIn { blanks } => {
            for blank in blanks {
                collect_assets(&blank.label, &mut referenced);
            }
        }
        IssuedQuestionResponseFormatV1::Matching {
            prompts, choices, ..
        } => {
            for choice in prompts.iter().chain(choices) {
                collect_assets(&choice.body, &mut referenced);
            }
        }
        IssuedQuestionResponseFormatV1::Ordering { items } => {
            for item in items {
                collect_assets(&item.body, &mut referenced);
            }
        }
        IssuedQuestionResponseFormatV1::Hotspot { surface, .. } => {
            referenced.insert(AssetRefKey::from(&surface.asset));
            for region in &surface.regions {
                collect_assets(&region.label, &mut referenced);
            }
        }
        IssuedQuestionResponseFormatV1::FillIn { .. }
        | IssuedQuestionResponseFormatV1::Numerical { .. } => {}
    }
    validate_asset_refs(&referenced, bindings)
}

pub(super) fn content_assets(
    content: &[ContentBlock],
    bindings: &[PresentedQuestionAsset],
) -> Result<Vec<PresentedQuestionAsset>, PresentationBuildError> {
    let mut referenced = BTreeSet::new();
    collect_assets(content, &mut referenced);
    referenced
        .iter()
        .map(|reference| {
            bindings
                .iter()
                .find(|binding| {
                    binding.asset == reference.asset
                        && binding.authored_checksum == reference.checksum
                })
                .cloned()
                .ok_or(PresentationBuildError::InvalidPublicContent(
                    "presentation asset binding is missing or mismatched",
                ))
        })
        .collect()
}

pub(super) fn asset_binding<'a>(
    reference: &AssetRef,
    bindings: &'a [PresentedQuestionAsset],
) -> Result<&'a PresentedQuestionAsset, PresentationBuildError> {
    bindings
        .iter()
        .find(|binding| {
            binding.asset == reference.asset && binding.authored_checksum == reference.checksum
        })
        .ok_or(PresentationBuildError::InvalidPublicContent(
            "presentation asset binding is missing or mismatched",
        ))
}

fn validate_asset_refs(
    referenced: &BTreeSet<AssetRefKey>,
    bindings: &[PresentedQuestionAsset],
) -> Result<Vec<PresentedQuestionAsset>, PresentationBuildError> {
    let mut by_id = BTreeMap::new();
    for binding in bindings {
        if by_id.insert(binding.asset, binding).is_some()
            || !is_sha256(&binding.authored_checksum)
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
        if binding.authored_checksum != reference.checksum {
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
    values.sort_by_key(|binding| binding.asset);
    Ok(values)
}

fn collect_assets(content: &[ContentBlock], target: &mut BTreeSet<AssetRefKey>) {
    for block in content {
        if let ContentBlock::Image { asset, .. } = block {
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
    asset: crate::AssetId,
    checksum: String,
}

impl From<&AssetRef> for AssetRefKey {
    fn from(value: &AssetRef) -> Self {
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
