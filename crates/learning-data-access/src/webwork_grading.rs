//! Private WeBWorK-only grading lease boundary.

use async_trait::async_trait;
use question_model::{QuestionAttemptId, QuestionId};
use serde_json::Value;
use uuid::Uuid;

use crate::StoreError;

/// One opaque leased WeBWorK grading Job and its immutable reproduction facts.
#[derive(Clone)]
pub struct WebworkGradingJobLease {
    pub job_id: Uuid,
    pub lease_token: Uuid,
    pub question_attempt: QuestionAttemptId,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub source_object_id: String,
    pub source_object_checksum: String,
    pub webwork_pg_path: String,
    pub question_seed: u64,
    pub student_response: Value,
    pub replay_details: Value,
}

impl std::fmt::Debug for WebworkGradingJobLease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WebworkGradingJobLease([redacted])")
    }
}

#[async_trait]
pub trait WebworkGradingStore: Send + Sync {
    async fn claim_webwork_grading_job(
        &self,
        lease_expires_at_unix_millis: i64,
    ) -> Result<Option<WebworkGradingJobLease>, StoreError>;

    async fn commit_webwork_grading(
        &self,
        lease: &WebworkGradingJobLease,
        correct: bool,
        normalized_credit: f64,
        committed_at_unix_millis: i64,
    ) -> Result<(), StoreError>;

    /// Marks only the current typed lease terminal when the isolated renderer
    /// cannot grade it. The accepted submission and its receipt boundary stay intact.
    async fn fail_webwork_grading(
        &self,
        lease: &WebworkGradingJobLease,
        completed_at_unix_millis: i64,
    ) -> Result<(), StoreError>;
}
