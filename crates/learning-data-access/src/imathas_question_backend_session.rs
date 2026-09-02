//! Server-only Remote Question Backend Session persistence boundary.
//!
//! A Session owns the authenticated, leased interaction with one remote
//! Question Backend. Opaque backend state remains server-only.

use async_trait::async_trait;
use question_model::Timestamp;

use crate::{SessionTokenHash, StoreError};

mod grading;
mod identifiers;
mod memory;
mod preparation;
mod protected_state;
mod session;
mod storage_parts;

pub(crate) use grading::automated_grading_receipt_checksum_v1;
pub use grading::{
    AutomatedGradingReceipt, AutomatedGradingReceiptId,
    CommitStagedRemoteQuestionBackendResultGrading, GradingResultId, JobId,
    LoadedRemoteQuestionBackendSession, MAX_REMOTE_QUESTION_BACKEND_GRADING_JOB_LEASE_MILLIS,
    QuestionSubmissionGradingId, RemoteQuestionBackendGradingJobLease,
    RemoteQuestionBackendResultExchangeIdempotencyKey, StageVerifiedRemoteQuestionBackendResult,
    StagedRemoteQuestionBackendResultReceipt,
};
pub(crate) use identifiers::validate_question_grading_rule;
pub use identifiers::{
    AutomatedGradingReceiptChecksum, QualifiedLaunchBindingDigest,
    RemoteQuestionBackendGradingContext, RemoteQuestionBackendNormalizedScore,
    RemoteQuestionBackendResponseChecksum, RemoteQuestionBackendResult,
    RemoteQuestionBackendResultChecksum, RemoteQuestionBackendResultToken,
    RemoteQuestionBackendResultTokenChecksum, RemoteQuestionBackendSessionAuthentication,
    RemoteQuestionBackendSessionChallenge, RemoteQuestionBackendSessionReference,
    derive_remote_question_backend_grading_result,
};
pub use memory::MemoryRemoteQuestionBackendSessionStore;
pub use preparation::{
    RemoteQuestionBackendLaunchPreparationValidation,
    RemoteQuestionBackendSessionPreparationContext,
};
pub use protected_state::{
    MAX_REMOTE_QUESTION_BACKEND_STATE_CIPHERTEXT_BYTES,
    MAX_REMOTE_QUESTION_BACKEND_STATE_PLAINTEXT_BYTES, RemoteQuestionBackendStateCipher,
    RemoteQuestionBackendStateKeyId, RemoteQuestionBackendStateKeyRing,
    RemoteQuestionBackendStatePlaintext,
};
pub(crate) use protected_state::{
    REMOTE_QUESTION_BACKEND_STATE_NONCE_BYTES, RemoteQuestionBackendStateCipherStorageParts,
};
pub use session::{
    RemoteQuestionBackendSession, RemoteQuestionBackendSessionCreate,
    RemoteQuestionBackendSessionLease, RemoteQuestionBackendSessionRestoreExpectation,
    RemoteQuestionBackendSessionValidation,
};
pub(crate) use storage_parts::{
    RemoteQuestionBackendGradingJobLeaseParts, RemoteQuestionBackendSessionCreateParts,
    RemoteQuestionBackendSessionLeaseParts, RemoteQuestionBackendSessionRestoreParts,
    RemoteQuestionBackendSessionStorageParts, StageVerifiedRemoteQuestionBackendResultParts,
};

/// Server-only Store for one Remote Question Backend Session and Result Exchange.
#[async_trait]
pub trait RemoteQuestionBackendSessionStore: Send + Sync {
    async fn create_remote_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        create: RemoteQuestionBackendSessionCreate,
    ) -> Result<RemoteQuestionBackendSessionReference, StoreError>;
    async fn load_remote_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        reference: RemoteQuestionBackendSessionReference,
        expectation: RemoteQuestionBackendSessionRestoreExpectation,
    ) -> Result<LoadedRemoteQuestionBackendSession, StoreError>;
    async fn lease_remote_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        reference: RemoteQuestionBackendSessionReference,
        expectation: RemoteQuestionBackendSessionRestoreExpectation,
        lease_expires_at: Timestamp,
    ) -> Result<RemoteQuestionBackendSessionLease, StoreError>;
    async fn stage_verified_remote_question_backend_result(
        &self,
        session_token_hash: SessionTokenHash,
        stage: StageVerifiedRemoteQuestionBackendResult,
    ) -> Result<StagedRemoteQuestionBackendResultReceipt, StoreError>;
    async fn claim_remote_question_backend_result_grading_job(
        &self,
        grading_job_id: JobId,
        lease_expires_at: Timestamp,
    ) -> Result<RemoteQuestionBackendGradingJobLease, StoreError>;
    async fn commit_staged_remote_question_backend_result_grading(
        &self,
        command: CommitStagedRemoteQuestionBackendResultGrading,
    ) -> Result<AutomatedGradingReceipt, StoreError>;
}

#[cfg(test)]
#[path = "remote_question_backend_session_tests.rs"]
mod tests;
