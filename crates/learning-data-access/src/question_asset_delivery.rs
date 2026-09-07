//! Closed read boundary for one immutable published Question Asset rendition.

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::{ObjectId, QuestionAssetId, QuestionRevisionReference};

use crate::{SessionTokenHash, StoreError};

/// A database-authorized immutable public rendition.
///
/// This remains server-only.  The HTTP route derives the sole CDN path from
/// these typed identities; it never receives a caller-selected object address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyQuestionAssetDelivery {
    pub question_revision: QuestionRevisionReference,
    pub asset_id: QuestionAssetId,
    pub public_object_id: ObjectId,
    pub rendition_checksum: Sha256Checksum,
}

/// Resolves exactly one ready public asset only after the database has proved
/// Instructor Question Library or issued Student Assignment Access.
#[async_trait]
pub trait QuestionAssetDeliveryStore: Send + Sync {
    async fn resolve_ready_question_asset_delivery(
        &self,
        session_token_hash: SessionTokenHash,
        asset_id: QuestionAssetId,
    ) -> Result<ReadyQuestionAssetDelivery, StoreError>;
}
