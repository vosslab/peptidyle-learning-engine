//! Converts retained delivery asset evidence into renderer-safe bindings.

use learning_data_access::{NativePleIssuanceSource, ReadyQuestionAssetRendition};
use question_model::{QuestionAssetReference, QuestionAssetRendition};

pub(crate) fn question_asset_renditions(
    source: &NativePleIssuanceSource,
) -> Vec<QuestionAssetRendition> {
    question_asset_renditions_from_ready(&source.question_asset_renditions)
}

pub(crate) fn question_asset_renditions_from_ready(
    renditions: &[ReadyQuestionAssetRendition],
) -> Vec<QuestionAssetRendition> {
    renditions
        .iter()
        .map(|rendition| QuestionAssetRendition {
            question_asset: QuestionAssetReference {
                question_asset: rendition.question_asset,
                checksum: rendition.question_asset_checksum.clone(),
            },
            rendition_checksum: rendition.rendition_checksum.clone(),
            intrinsic_width: Some(rendition.intrinsic_width),
            intrinsic_height: Some(rendition.intrinsic_height),
        })
        .collect()
}
