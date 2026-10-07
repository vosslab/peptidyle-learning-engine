//! Ordinary single-Question metadata replacement for the exact current Revision.

use async_trait::async_trait;
use question_model::{SaveQuestionMetadataRequest, SavedQuestionMetadata};

use crate::{SessionTokenHash, StoreError};

/// One authenticated owner or Sysadmin Question metadata operation.
#[async_trait]
pub trait QuestionMetadataStore: Send + Sync {
    /// Replaces editable fields on the exact current Question Revision using metadata CAS.
    async fn save_question_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        request: SaveQuestionMetadataRequest,
    ) -> Result<SavedQuestionMetadata, StoreError>;
}
