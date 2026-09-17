//! Immutable images owned by one real private Draft, never an image catalog.

use async_trait::async_trait;
use objects::image_validation::MAX_STILL_IMAGE_BYTES;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea};
use question_model::QuestionAssetId;

use crate::{DraftQuestionEditNumber, SessionTokenHash, StoreError};

/// Bytes-first registration under the Draft's ordinary source CAS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterDraftQuestionAssetInput {
    pub draft_question_uuid: crate::DraftQuestionUuid,
    pub expected_edit_number: DraftQuestionEditNumber,
    pub asset_id: QuestionAssetId,
    pub source_record: ObjectRecord,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}

/// Private server-only immutable raster facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedDraftQuestionAsset {
    pub asset_id: QuestionAssetId,
    pub source_record: ObjectRecord,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}

impl RegisterDraftQuestionAssetInput {
    pub fn validate(&self) -> Result<(), StoreError> {
        let ObjectAddress::DraftQuestionAsset { asset, object, .. } = &self.source_record.address
        else {
            return Err(StoreError::InvalidRecord(
                "Draft asset requires its semantic address".into(),
            ));
        };
        if *asset != self.asset_id
            || *object != self.source_record.id
            || self.source_record.storage_area != ObjectStorageArea::PrivateContent
            || self.source_record.data_class != ObjectDataClass::AuthoringContent
            || self.source_record.question_revision.is_some()
        {
            return Err(StoreError::InvalidRecord(
                "Draft asset identity is incoherent".into(),
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
            "Draft asset raster facts are invalid".into(),
        ));
    }
    Ok(())
}

/// Session-authorized exact Draft image registration and preview lookup.
#[async_trait]
pub trait AuthoringAssetsStore: Send + Sync {
    async fn register_draft_question_asset(
        &self,
        session_hash: SessionTokenHash,
        input: RegisterDraftQuestionAssetInput,
    ) -> Result<OwnedDraftQuestionAsset, StoreError>;
    async fn load_draft_question_asset(
        &self,
        session_hash: SessionTokenHash,
        draft_question_uuid: crate::DraftQuestionUuid,
        asset_id: QuestionAssetId,
    ) -> Result<OwnedDraftQuestionAsset, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use objects::Sha256Checksum;
    use question_model::{ObjectId, Timestamp, WorkspaceId};
    use uuid::Uuid;

    fn input() -> RegisterDraftQuestionAssetInput {
        let asset = QuestionAssetId::from_uuid(Uuid::from_u128(4));
        let object = ObjectId::from_uuid(Uuid::from_u128(5));
        RegisterDraftQuestionAssetInput {
            draft_question_uuid: crate::DraftQuestionUuid::from_uuid(Uuid::from_u128(1)),
            expected_edit_number: DraftQuestionEditNumber::new(1).expect("positive CAS"),
            asset_id: asset,
            source_record: ObjectRecord {
                id: object,
                storage_area: ObjectStorageArea::PrivateContent,
                data_class: ObjectDataClass::AuthoringContent,
                address: ObjectAddress::DraftQuestionAsset {
                    workspace: WorkspaceId::from_uuid(Uuid::from_u128(2)),
                    draft_question_uuid: Uuid::from_u128(3),
                    asset,
                    object,
                },
                sha256: Sha256Checksum::compute(b"verified original raster"),
                size_bytes: 24,
                media_type: "image/png".into(),
                question_revision: None,
                created_at: Timestamp::from_unix_millis(1000),
            },
            intrinsic_width: 10,
            intrinsic_height: 20,
        }
    }

    #[test]
    fn registration_refuses_forged_asset_identity_and_unbounded_or_nonraster_facts() {
        assert_eq!(input().validate(), Ok(()));
        let mut forged = input();
        forged.asset_id = QuestionAssetId::from_uuid(Uuid::from_u128(6));
        assert!(forged.validate().is_err());
        let mut wrong_area = input();
        wrong_area.source_record.storage_area = ObjectStorageArea::PublicAssets;
        assert!(wrong_area.validate().is_err());
        let mut svg = input();
        svg.source_record.media_type = "image/svg+xml".into();
        assert!(svg.validate().is_err());
        let mut oversized = input();
        oversized.source_record.size_bytes = MAX_STILL_IMAGE_BYTES as u64 + 1;
        assert!(oversized.validate().is_err());
        let mut pixel_flood = input();
        pixel_flood.intrinsic_width = 20_000_001;
        pixel_flood.intrinsic_height = 1;
        assert!(pixel_flood.validate().is_err());
    }
}
