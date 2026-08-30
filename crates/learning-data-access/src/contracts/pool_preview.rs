//! Read-only persistence capability for WP-INST-T5 item-pool samples.

use async_trait::async_trait;
use question_model::{
    AssignmentReference, CourseId, PoolDrawPreview, PoolDrawPreviewNonce,
    TeachingOperationRevision, UserId,
};

use crate::{ActorContext, StoreError};

/// Executes one authorized pool sample without creating learner work or
/// evidence. The supplied nonce is freshly minted by the trusted server and
/// remains deliberately absent from every browser contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolPreviewCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub group_position: u32,
    pub nonce: PoolDrawPreviewNonce,
}

#[async_trait]
pub trait PoolPreviewStore: Send + Sync {
    async fn preview_pool_draw(
        &self,
        context: ActorContext,
        command: PoolPreviewCommand,
    ) -> Result<PoolDrawPreview, StoreError>;
}
