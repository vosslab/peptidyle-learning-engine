//! Server-only iMathAS Question Backend Session persistence boundary.

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
    AutomatedGradingReceipt, AutomatedGradingReceiptId, CommitStagedImathasResultGrading,
    GradingResultId, ImathasGradingJobLease, ImathasResultExchangeIdempotencyKey, JobId,
    LoadedImathasQuestionBackendSession, MAX_IMATHAS_GRADING_JOB_LEASE_MILLIS,
    QuestionSubmissionGradingId, StageVerifiedImathasResult, StagedImathasResultReceipt,
};
pub use identifiers::{
    AutomatedGradingReceiptChecksum, ImathasGradingContext, ImathasLaunchBindingChecksum,
    ImathasNormalizedScore, ImathasQuestionBackendSessionAuthentication,
    ImathasQuestionBackendSessionChallenge, ImathasQuestionBackendSessionReference,
    ImathasResponseChecksum, ImathasResult, ImathasResultChecksum, ImathasResultToken,
    ImathasResultTokenChecksum, derive_imathas_question_backend_evaluation,
};
pub use memory::MemoryImathasQuestionBackendSessionStore;
pub use preparation::{
    ImathasQuestionBackendLaunchPreparationValidation,
    ImathasQuestionBackendSessionPreparationContext,
};
pub(super) const IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES: usize =
    protected_state::IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES;
pub(crate) use protected_state::ImathasQuestionBackendStateCipherStorageParts;
#[cfg(test)]
use protected_state::ImathasQuestionBackendStateNonceSource;
pub use protected_state::{
    ImathasQuestionBackendStateCipher, ImathasQuestionBackendStateKeyId,
    ImathasQuestionBackendStateKeyRing, ImathasQuestionBackendStatePlaintext,
    MAX_IMATHAS_QUESTION_BACKEND_STATE_CIPHERTEXT_BYTES,
    MAX_IMATHAS_QUESTION_BACKEND_STATE_PLAINTEXT_BYTES,
};
pub(crate) use session::ImathasQuestionBackendSessionStorePredicate;
pub use session::{
    ImathasQuestionBackendSession, ImathasQuestionBackendSessionCreate,
    ImathasQuestionBackendSessionLease, ImathasQuestionBackendSessionRestoreExpectation,
    ImathasQuestionBackendSessionValidation,
};
pub(crate) use storage_parts::{
    ImathasGradingJobLeaseParts, ImathasQuestionBackendSessionCreateParts,
    ImathasQuestionBackendSessionLeaseParts, ImathasQuestionBackendSessionRestoreParts,
    ImathasQuestionBackendSessionStorageParts, StageVerifiedImathasResultParts,
};

#[async_trait]
pub trait ImathasQuestionBackendSessionStore: Send + Sync {
    async fn create_imathas_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        create: ImathasQuestionBackendSessionCreate,
    ) -> Result<ImathasQuestionBackendSessionReference, StoreError>;
    async fn load_imathas_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        reference: ImathasQuestionBackendSessionReference,
        expectation: ImathasQuestionBackendSessionRestoreExpectation,
    ) -> Result<LoadedImathasQuestionBackendSession, StoreError>;
    async fn lease_imathas_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        reference: ImathasQuestionBackendSessionReference,
        expectation: ImathasQuestionBackendSessionRestoreExpectation,
        lease_expires_at: Timestamp,
    ) -> Result<ImathasQuestionBackendSessionLease, StoreError>;
    async fn stage_verified_imathas_result(
        &self,
        session_token_hash: SessionTokenHash,
        stage: StageVerifiedImathasResult,
    ) -> Result<StagedImathasResultReceipt, StoreError>;
    async fn claim_imathas_result_grading_job(
        &self,
        grading_job_id: JobId,
        lease_expires_at: Timestamp,
    ) -> Result<ImathasGradingJobLease, StoreError>;
    async fn commit_staged_imathas_result_grading(
        &self,
        command: CommitStagedImathasResultGrading,
    ) -> Result<AutomatedGradingReceipt, StoreError>;
}

#[cfg(test)]
#[path = "imathas_question_backend_session_tests.rs"]
mod tests;
