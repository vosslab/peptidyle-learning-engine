//! Server-only WeBWorK Question Submission acceptance boundary.

use async_trait::async_trait;
use question_model::{QuestionAttemptId, QuestionId};
use serde_json::Value;

use crate::{AcceptNativePleSubmission, SessionTokenHash, StoreError};

/// Private reproduction facts for one Student-authorized WeBWorK presentation.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedWebworkSubmission {
    pub question_attempt: QuestionAttemptId,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub source_object_id: String,
    pub source_object_checksum: String,
    pub webwork_pg_path: String,
    pub question_seed: u64,
    pub presentation_nonce: String,
    pub presentation_checksum: String,
    pub replay_details: Value,
}

#[async_trait]
pub trait WebworkSubmissionStore: Send + Sync {
    async fn resolve_webwork_submission(
        &self,
        session_token_hash: SessionTokenHash,
        course_reference_number: u64,
        assignment_reference_number: u64,
        presentation_nonce: &str,
    ) -> Result<ResolvedWebworkSubmission, StoreError>;

    async fn accept_webwork_submission(
        &self,
        session_token_hash: SessionTokenHash,
        submission: AcceptNativePleSubmission,
    ) -> Result<(), StoreError>;
}
