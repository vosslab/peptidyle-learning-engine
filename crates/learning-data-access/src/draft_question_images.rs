//! Immutable images owned by one real private Draft, never an image catalog.

use async_trait::async_trait;
use objects::image_validation::MAX_STILL_IMAGE_BYTES;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea};
use question_model::QuestionImageAssetId;

use crate::{DraftQuestionEditNumber, SessionTokenHash, StoreError};

/// Bytes-first registration under the Draft's ordinary source CAS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterDraftQuestionImageInput {
    pub draft_question_uuid: crate::DraftQuestionUuid,
    pub expected_edit_number: DraftQuestionEditNumber,
    pub question_image_asset_id: QuestionImageAssetId,
    pub source_record: ObjectRecord,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}

/// Private server-only immutable raster facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedDraftQuestionImage {
    pub question_image_asset_id: QuestionImageAssetId,
    pub source_record: ObjectRecord,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}

impl RegisterDraftQuestionImageInput {
    pub fn validate(&self) -> Result<(), StoreError> {
        let ObjectAddress::DraftQuestionImage {
            question_image_asset_id: asset,
            object_id: object,
            ..
        } = &self.source_record.address
        else {
            return Err(StoreError::InvalidRecord(
                "Draft Question image requires its semantic address".into(),
            ));
        };
        if *asset != self.question_image_asset_id
            || *object != self.source_record.id
            || self.source_record.storage_area != ObjectStorageArea::PrivateContent
            || self.source_record.data_class != ObjectDataClass::AuthoringContent
            || self
                .source_record
                .published_question_revision_tuple
                .is_some()
        {
            return Err(StoreError::InvalidRecord(
                "Draft Question image identity is incoherent".into(),
            ));
        }
        validate_raster_facts(
            &self.source_record,
            self.intrinsic_width,
            self.intrinsic_height,
        )
    }
}

pub(crate) fn validate_raster_facts(
    record: &ObjectRecord,
    width: u32,
    height: u32,
) -> Result<(), StoreError> {
    // ASVS 2.2.1-2.2.3: trusted measured raster facts remain bounded at persistence.
    if !matches!(
        record.media_type.as_str(),
        "image/png" | "image/jpeg" | "image/webp"
    ) || record.size_bytes == 0
        || record.size_bytes > MAX_STILL_IMAGE_BYTES as u64
        || width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > 20_000_000
    {
        return Err(StoreError::InvalidRecord(
            "Draft Question image raster facts are invalid".into(),
        ));
    }
    Ok(())
}

/// Session-authorized exact Draft image registration and preview lookup.
#[async_trait]
pub trait DraftQuestionImageStore: Send + Sync {
    async fn register_draft_question_image(
        &self,
        session_hash: SessionTokenHash,
        input: RegisterDraftQuestionImageInput,
    ) -> Result<OwnedDraftQuestionImage, StoreError>;
    async fn load_draft_question_image(
        &self,
        session_hash: SessionTokenHash,
        draft_question_uuid: crate::DraftQuestionUuid,
        question_image_asset_id: QuestionImageAssetId,
    ) -> Result<OwnedDraftQuestionImage, StoreError>;
}
