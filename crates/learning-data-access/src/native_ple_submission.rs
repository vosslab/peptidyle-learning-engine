//! Server-only native PLE Question Submission acceptance boundary.

use async_trait::async_trait;
use question_model::{QuestionAttemptId, QuestionId};
use serde_json::Value;

use crate::{ReadyQuestionAssetRendition, SessionTokenHash, StoreError};

/// Student-visible state of a native PLE Question Submission.
///
/// This is a Store result rather than a Question-model grading operation: the
/// submission boundary exposes only the safe status of its own asynchronous
/// grading work. It carries no response, score, or recovery-operation detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudentQuestionSubmissionGradingState {
    Pending,
    Graded,
    InstructorAttention,
}

/// A server-validated canonical Student Response for one issued native PLE
/// Question Attempt.  The server validates and normalizes this value against
/// the issued Question Presentation Response Format before calling the Store.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptNativePleSubmission {
    pub question_attempt: QuestionAttemptId,
    pub student_response: Value,
}

/// Private reproduction facts for the one issued native PLE presentation a
/// Student is permitted to submit. This never crosses an HTTP boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedNativePleSubmission {
    pub question_attempt: QuestionAttemptId,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub source_object_id: String,
    pub source_object_checksum: String,
    pub question_seed: u64,
    pub presentation_nonce: String,
    pub presentation_checksum: String,
    /// Ready, answer-free public bindings needed to reconstruct an asset-backed
    /// issued presentation checksum. Object addresses never enter this type.
    pub question_asset_renditions: Vec<ReadyQuestionAssetRendition>,
}

/// Nonce-bound public status projection with grading state only; Student Feedback is separate.
#[derive(Debug, Clone, PartialEq)]
pub struct NativePleSubmissionStatus {
    pub presentation_nonce: String,
    pub grading_state: StudentQuestionSubmissionGradingState,
}

/// Accepts one native PLE Student Response and prepares its separate grading
/// work without exposing accepted response evidence or correctness.
#[async_trait]
pub trait NativePleSubmissionStore: Send + Sync {
    /// Resolves one active Student-owned presentation nonce to the private
    /// immutable facts required to reproduce its answer-free response format.
    async fn resolve_native_ple_submission(
        &self,
        session_token_hash: SessionTokenHash,
        course_reference_number: u64,
        assignment_reference_number: u64,
        presentation_nonce: &str,
    ) -> Result<ResolvedNativePleSubmission, StoreError>;

    async fn accept_native_ple_submission(
        &self,
        session_token_hash: SessionTokenHash,
        submission: AcceptNativePleSubmission,
    ) -> Result<StudentQuestionSubmissionGradingState, StoreError>;

    async fn native_ple_submission_status(
        &self,
        session_token_hash: SessionTokenHash,
        course_reference_number: u64,
        assignment_reference_number: u64,
        presentation_nonce: &str,
    ) -> Result<NativePleSubmissionStatus, StoreError>;
}
