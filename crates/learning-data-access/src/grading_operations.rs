//! Store boundary for accepting and replaying Instructor grading actions.

use async_trait::async_trait;
use question_model::{InstructorGradingOperationActionRequest, InstructorGradingOperationReplay};

use crate::StoreError;

/// Session-authorized durable acceptance for one Instructor Grading Operation action.
///
/// An implementation compares the complete request binding atomically by Retry
/// Token. An equal request returns the original accepted Receipt without a
/// second effect; a changed operation, action, or Request Checksum is refused.
#[async_trait]
pub trait InstructorGradingOperationStore: Send + Sync {
    async fn accept_or_replay_instructor_grading_operation(
        &self,
        request: InstructorGradingOperationActionRequest,
    ) -> Result<InstructorGradingOperationReplay, StoreError>;
}
