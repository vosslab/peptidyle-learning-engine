//! Validation and selection of Question images presented with one issued Question.

use std::collections::{BTreeMap, BTreeSet};

use crate::question_content::{QuestionContentBlock, QuestionImageAssetTuple};
use crate::question_variation::QuestionVariationPresentation;
use crate::response::QuestionResponseFormat;

use super::builder::PresentationBuildError;
use super::model::{
    PresentedResponseItemContent, QuestionImageRendition, QuestionPresentation,
    QuestionPresentationResponseFormat,
};

pub(super) fn validate_question_images(
    presentation: &QuestionVariationPresentation,
    bindings: &[QuestionImageRendition],
) -> Result<Vec<QuestionImageRendition>, PresentationBuildError> {
    let mut referenced = BTreeSet::new();
    collect_question_images(&presentation.prompt, &mut referenced);
    collect_response_question_images(&presentation.response, &mut referenced);
    validate_question_image_refs(&referenced, bindings)
}

pub(super) fn validate_public_question_images(
    presentation: &QuestionPresentation,
    bindings: &[QuestionImageRendition],
) -> Result<Vec<QuestionImageRendition>, PresentationBuildError> {
    let mut referenced = BTreeSet::new();
    collect_question_images(&presentation.prompt, &mut referenced);
    match &presentation.response {
        QuestionPresentationResponseFormat::SingleChoice { choices }
        | QuestionPresentationResponseFormat::MultipleAnswer { choices, .. } => {
            collect_presented_response_item_images(choices, &mut referenced);
        }
        QuestionPresentationResponseFormat::MultiFillIn { blanks } => {
            for blank in blanks {
                collect_question_images(&blank.label, &mut referenced);
            }
        }
        QuestionPresentationResponseFormat::Matching {
            prompts, choices, ..
        } => {
            collect_presented_response_item_images(prompts, &mut referenced);
            collect_presented_response_item_images(choices, &mut referenced);
        }
        QuestionPresentationResponseFormat::Ordering { items } => {
            collect_presented_response_item_images(items, &mut referenced);
        }
        QuestionPresentationResponseFormat::Hotspot { surface, .. } => {
            referenced.insert(QuestionImageRefKey::from(
                &surface.question_image_asset_tuple,
            ));
            for region in &surface.regions {
                collect_question_images(&region.label, &mut referenced);
            }
        }
        QuestionPresentationResponseFormat::FillIn { .. }
        | QuestionPresentationResponseFormat::Numerical { .. }
        | QuestionPresentationResponseFormat::ImathasQuestionBackend {}
        | QuestionPresentationResponseFormat::BackendOwned {} => {}
    }
    validate_question_image_refs(&referenced, bindings)
}

fn collect_presented_response_item_images<T: PresentedResponseItemContent>(
    items: &[T],
    referenced: &mut BTreeSet<QuestionImageRefKey>,
) {
    for item in items {
        collect_question_images(item.presentation_item_body(), referenced);
    }
}

pub(super) fn content_question_images(
    content: &[QuestionContentBlock],
    bindings: &[QuestionImageRendition],
) -> Result<Vec<QuestionImageRendition>, PresentationBuildError> {
    let mut referenced = BTreeSet::new();
    collect_question_images(content, &mut referenced);
    referenced
        .iter()
        .map(|asset_key| {
            bindings
                .iter()
                .find(|binding| {
                    binding.question_image_asset_tuple.question_image_asset_id
                        == asset_key.question_image_asset_id
                        && binding.question_image_asset_tuple.checksum == asset_key.checksum
                })
                .cloned()
                .ok_or(PresentationBuildError::InvalidPublicContent(
                    "presentation image binding is missing or mismatched",
                ))
        })
        .collect()
}

pub(super) fn question_image_rendition<'a>(
    question_image_asset_tuple: &QuestionImageAssetTuple,
    bindings: &'a [QuestionImageRendition],
) -> Result<&'a QuestionImageRendition, PresentationBuildError> {
    bindings
        .iter()
        .find(|binding| {
            binding.question_image_asset_tuple.question_image_asset_id
                == question_image_asset_tuple.question_image_asset_id
                && binding.question_image_asset_tuple.checksum
                    == question_image_asset_tuple.checksum
        })
        .ok_or(PresentationBuildError::InvalidPublicContent(
            "presentation image binding is missing or mismatched",
        ))
}

fn validate_question_image_refs(
    referenced: &BTreeSet<QuestionImageRefKey>,
    bindings: &[QuestionImageRendition],
) -> Result<Vec<QuestionImageRendition>, PresentationBuildError> {
    let mut by_id = BTreeMap::new();
    for binding in bindings {
        if by_id
            .insert(
                binding.question_image_asset_tuple.question_image_asset_id,
                binding,
            )
            .is_some()
            || !is_sha256(&binding.question_image_asset_tuple.checksum)
            || !is_sha256(&binding.rendition_checksum)
            || binding.intrinsic_width.is_some() != binding.intrinsic_height.is_some()
            || binding.intrinsic_width == Some(0)
            || binding.intrinsic_height == Some(0)
        {
            return Err(PresentationBuildError::InvalidPublicContent(
                "presentation image binding is malformed",
            ));
        }
    }
    for asset_key in referenced {
        let binding = by_id.get(&asset_key.question_image_asset_id).ok_or(
            PresentationBuildError::InvalidPublicContent("presentation image binding is missing"),
        )?;
        if binding.question_image_asset_tuple.checksum != asset_key.checksum {
            return Err(PresentationBuildError::InvalidPublicContent(
                "presentation image checksum does not match the question",
            ));
        }
    }
    if by_id.keys().any(|asset| {
        !referenced
            .iter()
            .any(|value| value.question_image_asset_id == *asset)
    }) {
        return Err(PresentationBuildError::InvalidPublicContent(
            "presentation contains an unreferenced image binding",
        ));
    }
    let mut values = bindings.to_vec();
    values.sort_by_key(|binding| binding.question_image_asset_tuple.question_image_asset_id);
    Ok(values)
}

fn collect_question_images(
    content: &[QuestionContentBlock],
    target: &mut BTreeSet<QuestionImageRefKey>,
) {
    for block in content {
        if let QuestionContentBlock::Image {
            question_image_asset_tuple,
            ..
        } = block
        {
            target.insert(QuestionImageRefKey::from(question_image_asset_tuple));
        }
    }
}

fn collect_response_question_images(
    response: &QuestionResponseFormat,
    target: &mut BTreeSet<QuestionImageRefKey>,
) {
    match response {
        QuestionResponseFormat::MultipleChoice { choices, .. } => {
            for choice in choices {
                collect_question_images(&choice.body, target);
            }
        }
        QuestionResponseFormat::MultiBlank { blanks } => {
            for blank in blanks {
                collect_question_images(&blank.label, target);
            }
        }
        QuestionResponseFormat::Matching { prompts, choices } => {
            for choice in prompts {
                collect_question_images(&choice.body, target);
            }
            for choice in choices {
                collect_question_images(&choice.body, target);
            }
        }
        QuestionResponseFormat::Ordering { items } => {
            for item in items {
                collect_question_images(&item.body, target);
            }
        }
        QuestionResponseFormat::Hotspot {
            question_image_asset_tuple,
            regions,
            ..
        } => {
            target.insert(QuestionImageRefKey::from(question_image_asset_tuple));
            for region in regions {
                collect_question_images(&region.label, target);
            }
        }
        QuestionResponseFormat::Numeric { .. }
        | QuestionResponseFormat::ShortText { .. }
        | QuestionResponseFormat::ImathasQuestionBackend {}
        | QuestionResponseFormat::BackendOwned {} => {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct QuestionImageRefKey {
    question_image_asset_id: crate::QuestionImageAssetId,
    checksum: String,
}

impl From<&QuestionImageAssetTuple> for QuestionImageRefKey {
    fn from(value: &QuestionImageAssetTuple) -> Self {
        Self {
            question_image_asset_id: value.question_image_asset_id,
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
