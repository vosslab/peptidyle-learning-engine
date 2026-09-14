//! Narrow worker boundary for authoritative Assignment Attempt expiry.

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    StoreError, StudentAssignmentAttemptFinalizationEvaluation,
    StudentAssignmentAttemptFinalizationPreparation,
};

/// One server-attested expired Attempt snapshot selected for backend evaluation.
///
/// The opaque identifier is only meaningful to the deadline worker. PostgreSQL
/// derives the finalization reason and rechecks every saved-response version
/// when this snapshot is committed.
#[derive(Debug, Clone)]
pub struct ExpiredAssignmentAttemptFinalizationPreparation {
    pub assignment_attempt_id: Uuid,
    pub preparation: StudentAssignmentAttemptFinalizationPreparation,
}

/// Store capability owned by the generic worker's deadline sweep.
#[async_trait]
pub trait AssignmentAttemptExpirySweepStore: Send + Sync {
    /// Captures at most `limit` expired Attempt snapshots before backend I/O.
    async fn prepare_expired_assignment_attempt_finalizations(
        &self,
        limit: u32,
    ) -> Result<Vec<ExpiredAssignmentAttemptFinalizationPreparation>, StoreError>;

    /// Atomically accepts one still-current expired snapshot after evaluation.
    async fn commit_expired_assignment_attempt_finalization(
        &self,
        assignment_attempt_id: Uuid,
        preparation: StudentAssignmentAttemptFinalizationPreparation,
        evaluations: Vec<StudentAssignmentAttemptFinalizationEvaluation>,
    ) -> Result<(), StoreError>;
}
