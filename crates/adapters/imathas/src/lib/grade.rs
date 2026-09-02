//! Launch-session authentication and verified imathas-question-backend results.

use objects::{ObjectStoreError, QuestionSourceResolutionError};
use question_model::QuestionTitleError;

/// Authenticated iMathAS verdict. The fields are private and this type has no
/// serde implementation, so an HTTP/browser payload cannot deserialize into it.
#[derive(Clone, PartialEq)]
pub struct VerifiedImathasQuestionBackendResult {
    pub(crate) imathas_question_backend_result: learning_data_access::ImathasQuestionBackendResult,
    pub(crate) grading_context: learning_data_access::ImathasQuestionBackendGradingContext,
    pub(crate) launch_session_authentication:
        learning_data_access::ImathasQuestionBackendSessionAuthentication,
    imathas_question_backend_result_token_checksum:
        learning_data_access::ImathasQuestionBackendResultTokenChecksum,
}

impl std::fmt::Debug for VerifiedImathasQuestionBackendResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedImathasQuestionBackendResult")
            .field(
                "imathas_question_backend_result",
                &self.imathas_question_backend_result,
            )
            .field("grading_context", &self.grading_context)
            .field("launch_session_authentication", &"REDACTED")
            .field(
                "imathas_question_backend_result_token_checksum",
                &"REDACTED",
            )
            .finish()
    }
}

impl VerifiedImathasQuestionBackendResult {
    /// Server-only verified result; this type is non-serde and can only be
    /// obtained from the sealed Result Verifier.
    pub fn imathas_question_backend_result(
        &self,
    ) -> learning_data_access::ImathasQuestionBackendResult {
        self.imathas_question_backend_result.clone()
    }

    /// Exact identity authenticated by the iMathAS verifier.
    pub fn grading_context(&self) -> learning_data_access::ImathasQuestionBackendGradingContext {
        self.grading_context.clone()
    }

    /// Durable receipt of the exact iMathAS result token accepted by this adapter.
    /// The caller supplies it to the later one-use Store transaction; this
    /// adapter performs no Store consumption itself.
    pub fn imathas_question_backend_result_token_checksum(
        &self,
    ) -> learning_data_access::ImathasQuestionBackendResultTokenChecksum {
        self.imathas_question_backend_result_token_checksum
    }

    /// Converts this sealed verifier output into the sole LDA staging command.
    /// The authenticated context and Session authentication remain private to
    /// this proof and are compared against the exact leased Session by LDA.
    pub fn stage(
        self,
        lease: learning_data_access::ImathasQuestionBackendSessionLease,
        idempotency_key: learning_data_access::ImathasQuestionBackendResultExchangeIdempotencyKey,
        transitioned_at: question_model::Timestamp,
    ) -> Result<
        learning_data_access::StageVerifiedImathasQuestionBackendResult,
        learning_data_access::StoreError,
    > {
        learning_data_access::StageVerifiedImathasQuestionBackendResult::new(
            lease,
            self.grading_context,
            self.launch_session_authentication,
            idempotency_key,
            self.imathas_question_backend_result_token_checksum,
            self.imathas_question_backend_result,
            transitioned_at,
        )
    }

    /// iMathAS Question Backend implementations use this only after their signature/audience/
    /// expiry/challenge verification succeeds.
    #[cfg(test)]
    pub(crate) fn verified(
        imathas_question_backend_result: learning_data_access::ImathasQuestionBackendResult,
        grading_context: learning_data_access::ImathasQuestionBackendGradingContext,
        launch_session_authentication: &learning_data_access::ImathasQuestionBackendSessionAuthentication,
        imathas_question_backend_result_token_checksum: learning_data_access::ImathasQuestionBackendResultTokenChecksum,
    ) -> Self {
        Self {
            imathas_question_backend_result,
            grading_context,
            launch_session_authentication: launch_session_authentication.clone(),
            imathas_question_backend_result_token_checksum,
        }
    }

    /// The iMathAS iMathAS Question Backend verifier is the only production constructor. Its
    /// result token has already passed signature, expiry, exact question, and
    /// exact iMathAS-binding checks before this sealed grade exists. The
    /// caller-owned LDA Store transaction performs single-use consumption and
    /// replay finalization after protocol verification succeeds.
    pub(crate) fn from_result_verification(
        _seal: crate::result_verification::ImathasResultVerificationSeal,
        imathas_question_backend_result: learning_data_access::ImathasQuestionBackendResult,
        grading_context: learning_data_access::ImathasQuestionBackendGradingContext,
        launch_session_authentication: &learning_data_access::ImathasQuestionBackendSessionAuthentication,
        imathas_question_backend_result_token_checksum: learning_data_access::ImathasQuestionBackendResultTokenChecksum,
    ) -> Self {
        Self {
            imathas_question_backend_result,
            grading_context,
            launch_session_authentication: launch_session_authentication.clone(),
            imathas_question_backend_result_token_checksum,
        }
    }

    #[cfg(feature = "test-support")]
    pub(crate) fn from_test_support(
        imathas_question_backend_result: learning_data_access::ImathasQuestionBackendResult,
        grading_context: learning_data_access::ImathasQuestionBackendGradingContext,
        launch_session_authentication: &learning_data_access::ImathasQuestionBackendSessionAuthentication,
        imathas_question_backend_result_token_checksum: learning_data_access::ImathasQuestionBackendResultTokenChecksum,
    ) -> Self {
        Self {
            imathas_question_backend_result,
            grading_context,
            launch_session_authentication: launch_session_authentication.clone(),
            imathas_question_backend_result_token_checksum,
        }
    }
}

/// iMathAS-backend-local failures. They are deliberately classified as unavailable or
/// invalid rather than a student correctness decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImathasQuestionBackendFailure {
    Unavailable,
    Timeout,
    UnsupportedProfile,
    Authentication,
    LaunchSessionAuthentication,
    InvalidResponse,
}

/// Adapter failures suitable for a backend-local retry/degraded state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImathasAdapterError {
    UnsupportedSource,
    InvalidDraft,
    UnsupportedProfile,
    SourceChecksumMismatch,
    UntrustedSource,
    SourceDoesNotMatchQuestion,
    InvalidCache,
    InvalidImathasQuestionBackendRender,
    InvalidTitle(QuestionTitleError),
    InvalidImathasQuestionBackendSessionAuthentication,
    VerificationRefused,
    QuestionBackend(ImathasQuestionBackendFailure),
    ObjectStore(ObjectStoreError),
    QuestionSourceResolution(QuestionSourceResolutionError),
}

impl std::fmt::Display for ImathasAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSource => f.write_str("question source is not iMathAS"),
            Self::InvalidDraft => f.write_str("invalid private iMathAS draft locator"),
            Self::UnsupportedProfile => f.write_str("unsupported iMathAS integration profile"),
            Self::SourceChecksumMismatch => f.write_str("iMathAS snapshot checksum mismatch"),
            Self::UntrustedSource => {
                f.write_str("iMathAS source was not resolved through its immutable object")
            }
            Self::SourceDoesNotMatchQuestion => {
                f.write_str("iMathAS source does not match its published question")
            }
            Self::InvalidCache => f.write_str("invalid iMathAS render cache"),
            Self::InvalidImathasQuestionBackendRender => {
                f.write_str("invalid iMathAS Question Backend render")
            }
            Self::InvalidTitle(error) => write!(f, "invalid iMathAS question title: {error}"),
            Self::InvalidImathasQuestionBackendSessionAuthentication => f.write_str(
                "invalid server-held iMathAS iMathAS Question Backend Session authentication",
            ),
            Self::VerificationRefused => {
                f.write_str("iMathAS verified grade did not match its server-held binding")
            }
            Self::QuestionBackend(_) => {
                f.write_str("iMathAS Question Backend unavailable or rejected request")
            }
            Self::ObjectStore(value) => value.fmt(f),
            Self::QuestionSourceResolution(value) => value.fmt(f),
        }
    }
}

impl std::error::Error for ImathasAdapterError {}
