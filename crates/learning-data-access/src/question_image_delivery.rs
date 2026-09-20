//! Closed read boundary for one immutable published Question Image rendition.

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::{ObjectId, QuestionImageAssetId, QuestionRevisionTuple};

use crate::{SessionTokenHash, StoreError};

/// A database-authorized immutable public rendition.
///
/// This remains server-only.  The HTTP route derives the sole CDN path from
/// these typed identities; it never receives a caller-selected object address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyQuestionImageDelivery {
    pub question_revision_tuple: QuestionRevisionTuple,
    pub question_image_asset_id: QuestionImageAssetId,
    pub public_object_id: ObjectId,
    pub rendition_checksum: Sha256Checksum,
}

/// Resolves exactly one ready public Question Image only after the database has proved
/// Instructor Question Library or issued Student Assessment Access.
#[async_trait]
pub trait QuestionImageDeliveryStore: Send + Sync {
    async fn resolve_ready_question_image_delivery(
        &self,
        session_token_hash: SessionTokenHash,
        question_revision_tuple: QuestionRevisionTuple,
        question_image_asset_id: QuestionImageAssetId,
    ) -> Result<ReadyQuestionImageDelivery, StoreError>;
}
