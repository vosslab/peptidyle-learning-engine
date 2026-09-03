//! Focused persistence foundations for the clean single-installation baseline.
//!
//! Product adapters are added only with an exact account, course-membership,
//! Student-ownership, workspace, observer-grant, or worker-lease contract.

use domain::assignment_activity::AssignmentActivityError;

mod assignment_attempt;
mod authentication_ceremony;
mod authentication_email;
mod grading_operations;
mod imathas_question_backend_session;
mod object_record;
mod pagination;
pub mod postgres;
mod question_source;
mod random_uuid;
pub mod session;
#[path = "contracts/store_error.rs"]
mod store_error;

pub use assignment_attempt::{
    AssignmentAttemptStart, AssignmentAttemptStartResult, AssignmentAttemptStore,
    PreparedIssuedQuestion, PreparedQuestionPoolSelection,
};
pub use authentication_ceremony::{
    AuthenticatedAccount, AuthenticationCeremonyLifetime, AuthenticationCeremonyStore,
    AuthenticationSecretHash, EmailAuthenticationChallenge, EmailAuthenticationChallengeId,
    EmailAuthenticationPurpose, MAX_AUTHENTICATION_CEREMONY_SECONDS, Passkey, PasskeyCeremonyId,
    PasskeyId,
};
pub use authentication_email::{
    AuthenticationEmail, AuthenticationEmailError, EmailDomain, MAX_AUTHENTICATION_EMAIL_BYTES,
};
pub use grading_operations::InstructorGradingOperationStore;
pub use imathas_question_backend_session::{
    AutomatedGradingReceipt, AutomatedGradingReceiptChecksum, AutomatedGradingReceiptId,
    CommitStagedImathasResultGrading, GradingResultId, ImathasGradingContext,
    ImathasGradingJobLease, ImathasLaunchBindingChecksum, ImathasNormalizedScore,
    ImathasQuestionBackendLaunchPreparationValidation, ImathasQuestionBackendSession,
    ImathasQuestionBackendSessionAuthentication, ImathasQuestionBackendSessionChallenge,
    ImathasQuestionBackendSessionCreate, ImathasQuestionBackendSessionLease,
    ImathasQuestionBackendSessionPreparationContext, ImathasQuestionBackendSessionReference,
    ImathasQuestionBackendSessionRestoreExpectation, ImathasQuestionBackendSessionStore,
    ImathasQuestionBackendSessionValidation, ImathasQuestionBackendStateCipher,
    ImathasQuestionBackendStateKeyId, ImathasQuestionBackendStateKeyRing,
    ImathasQuestionBackendStatePlaintext, ImathasResponseChecksum, ImathasResult,
    ImathasResultChecksum, ImathasResultExchangeIdempotencyKey, ImathasResultToken,
    ImathasResultTokenChecksum, JobId, LoadedImathasQuestionBackendSession,
    MAX_IMATHAS_QUESTION_BACKEND_STATE_CIPHERTEXT_BYTES,
    MAX_IMATHAS_QUESTION_BACKEND_STATE_PLAINTEXT_BYTES, MemoryImathasQuestionBackendSessionStore,
    QuestionSubmissionGradingId, StageVerifiedImathasResult, StagedImathasResultReceipt,
    derive_imathas_question_backend_grading_result,
};
#[allow(unused_imports)] // Crate-private PostgreSQL Store row-binding surface.
pub(crate) use imathas_question_backend_session::{
    ImathasGradingJobLeaseParts, ImathasQuestionBackendSessionCreateParts,
    ImathasQuestionBackendSessionLeaseParts, ImathasQuestionBackendSessionRestoreParts,
    ImathasQuestionBackendSessionStorageParts, ImathasQuestionBackendSessionStorePredicate,
    ImathasQuestionBackendStateCipherStorageParts, StageVerifiedImathasResultParts,
    automated_grading_receipt_checksum_v1, validate_question_grading_rule,
};
pub use object_record::{
    WorkspaceQuestionSourceObjectRecordStore, validate_workspace_question_source_object_record,
};
pub use pagination::{Cursor, Page, PageRequest, PageSize, PaginationError};
pub use question_source::{
    DraftQuestionEditNumber, DraftQuestionSourceRegistrationInput,
    DraftQuestionSourceRegistrationStore, DraftQuestionUuid,
};
pub use session::{
    SessionId, SessionLifetime, SessionRecord, SessionStore, SessionTokenHash,
    SessionTokenHashParseError,
};
pub use store_error::StoreError;
