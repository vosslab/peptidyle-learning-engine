//! Server-only iMathAS Question Backend Session persistence boundary.

use crate::{SessionTokenHash, StoreError};
use async_trait::async_trait;

mod identifiers;
mod memory;
mod preparation;
mod protected_state;
mod session;
mod storage_parts;

pub use identifiers::{
    ImathasGradingContext, ImathasLaunchBindingChecksum, ImathasNormalizedScore,
    ImathasQuestionBackendSessionAuthentication, ImathasQuestionBackendSessionChallenge,
    ImathasQuestionBackendSessionId, ImathasResponseChecksum, ImathasResult, ImathasResultToken,
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
pub use session::{
    ImathasQuestionBackendSession, ImathasQuestionBackendSessionCreate,
    ImathasQuestionBackendSessionRestoreExpectation, ImathasQuestionBackendSessionValidation,
    LoadedImathasQuestionBackendSession,
};
pub(crate) use storage_parts::{
    ImathasQuestionBackendSessionCreateParts, ImathasQuestionBackendSessionRestoreParts,
    ImathasQuestionBackendSessionStorageParts,
};

#[async_trait]
pub trait ImathasQuestionBackendSessionStore: Send + Sync {
    async fn create_imathas_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        create: ImathasQuestionBackendSessionCreate,
    ) -> Result<ImathasQuestionBackendSessionId, StoreError>;
    async fn load_imathas_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        reference: ImathasQuestionBackendSessionId,
        expectation: ImathasQuestionBackendSessionRestoreExpectation,
    ) -> Result<LoadedImathasQuestionBackendSession, StoreError>;
}

#[cfg(test)]
#[path = "imathas_question_backend_session_tests.rs"]
mod tests;
