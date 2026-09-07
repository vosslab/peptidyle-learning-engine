//! Closed persistence boundary for immutable public Question Asset publication.

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::{ObjectId, QuestionAssetId, QuestionRevisionReference};
use uuid::Uuid;

use crate::StoreError;

/// One database-selected private source and fixed public rendition target.
///
/// This is never built from a browser request or a queue payload. The
/// publication registry fixes every field before the publisher can claim it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedQuestionAssetPublication {
    pub job_id: Uuid,
    pub question_revision: QuestionRevisionReference,
    pub asset_id: QuestionAssetId,
    pub source_object_id: ObjectId,
    pub source_checksum: Sha256Checksum,
    pub public_object_id: ObjectId,
    pub public_checksum: Sha256Checksum,
    pub public_byte_length: u64,
    pub verified_media_type: String,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}

/// Dedicated publisher capability. It can claim a registry-backed Job and
/// activate that same lease, but has no generic Job, source, or API method.
#[async_trait]
pub trait PublicAssetPublicationStore: Send + Sync {
    async fn claim_question_asset_publication(
        &self,
        lease_token: Uuid,
    ) -> Result<Option<ClaimedQuestionAssetPublication>, StoreError>;

    async fn activate_question_asset_publication(
        &self,
        job_id: Uuid,
        lease_token: Uuid,
    ) -> Result<(), StoreError>;
}
