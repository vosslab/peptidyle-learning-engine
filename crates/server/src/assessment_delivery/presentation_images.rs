//! Converts retained Question Image Rendition evidence into renderer-safe bindings.

use learning_data_access::{NativePleIssuanceSource, ReadyQuestionImageRendition};
use question_model::{QuestionImageAssetTuple, QuestionImageRendition};

pub(crate) fn question_image_renditions(
    source: &NativePleIssuanceSource,
) -> Vec<QuestionImageRendition> {
    question_image_renditions_from_ready(&source.question_image_renditions)
}

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
