//! Narrow worker boundary for authoritative Assessment Attempt expiry.

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    StoreError, StudentAssessmentAttemptFinalizationEvaluation,
    StudentAssessmentAttemptFinalizationPreparation,
};

/// One server-attested expired Attempt snapshot selected for backend evaluation.
///
/// The opaque identifier is only meaningful to the deadline worker. PostgreSQL
/// derives the finalization reason and rechecks every saved-response version
/// when this snapshot is committed.
#[derive(Debug, Clone)]
pub struct ExpiredAssessmentAttemptFinalizationPreparation {
    pub assessment_attempt_id: Uuid,
    pub preparation: StudentAssessmentAttemptFinalizationPreparation,
}

/// Store capability owned by the generic worker's deadline sweep.
#[async_trait]
pub trait AssessmentAttemptExpirySweepStore: Send + Sync {
    /// Captures at most `limit` expired Attempt snapshots before backend I/O.
    async fn prepare_expired_assessment_attempt_finalizations(
        &self,
        limit: u32,
    ) -> Result<Vec<ExpiredAssessmentAttemptFinalizationPreparation>, StoreError>;

    /// Atomically accepts one still-current expired snapshot after evaluation.
    async fn commit_expired_assessment_attempt_finalization(
        &self,
        assessment_attempt_id: Uuid,
        preparation: StudentAssessmentAttemptFinalizationPreparation,
        evaluations: Vec<StudentAssessmentAttemptFinalizationEvaluation>,
    ) -> Result<(), StoreError>;
}
