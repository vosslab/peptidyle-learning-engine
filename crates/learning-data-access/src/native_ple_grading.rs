//! Private, native-PLE-only grading lease boundary.

use async_trait::async_trait;
use question_model::{QuestionAttemptId, QuestionId};
use serde_json::Value;
use uuid::Uuid;

use crate::StoreError;

/// One opaque, currently leased native PLE grading Job and the immutable facts
/// needed to reproduce and grade it. None of these facts are browser data.
#[derive(Clone)]
pub struct NativePleGradingJobLease {
    pub job_id: Uuid,
    pub lease_token: Uuid,
    pub question_attempt: QuestionAttemptId,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub source_object_id: String,
    pub source_object_checksum: String,
    pub question_seed: u64,
    pub student_response: Value,
}

impl std::fmt::Debug for NativePleGradingJobLease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("NativePleGradingJobLease([redacted])")
    }
}

#[async_trait]
pub trait NativePleGradingStore: Send + Sync {
    /// Claims at most one ready or expired native-PLE Job. `None` means no
    /// work, rather than a failed claim.
    async fn claim_native_ple_grading_job(
        &self,
        lease_expires_at_unix_millis: i64,
    ) -> Result<Option<NativePleGradingJobLease>, StoreError>;

    /// Atomically persists one terminal grading result and its receipt under
    /// the exact lease returned by `claim_native_ple_grading_job`.
    async fn commit_native_ple_grading(
        &self,
        lease: &NativePleGradingJobLease,
        correct: bool,
        normalized_credit: f64,
        committed_at_unix_millis: i64,
    ) -> Result<(), StoreError>;
}
