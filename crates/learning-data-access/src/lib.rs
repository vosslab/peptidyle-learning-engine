//! Focused persistence foundations for the clean single-installation baseline.
//!
//! Product adapters are added only with an exact account, course-membership,
//! Student-ownership, workspace, observer-grant, or worker-lease contract.

use domain::assignment_activity::AssignmentActivityError;

mod assignment_attempt;
mod authentication_ceremony;
mod authentication_email;
mod grading_operations;
mod object_record;
mod pagination;
pub mod postgres;
mod question_source;
mod random_uuid;
mod remote_question_backend_session;
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
pub use object_record::{
    WorkspaceQuestionSourceObjectRecordStore, validate_workspace_question_source_object_record,
};
pub use pagination::{Cursor, Page, PageRequest, PageSize, PaginationError};
pub use question_source::{
    DraftQuestionRevision, DraftQuestionRevisionNumber, DraftQuestionRevisionReference,
    DraftQuestionSourceInput, DraftQuestionSourceStore, DraftQuestionUuid,
    QuestionPublicBindingChecksum, QuestionSourceUuid,
};
pub use remote_question_backend_session::{
    AutomatedGradingReceipt, AutomatedGradingReceiptChecksum, AutomatedGradingReceiptId,
    CommitStagedRemoteQuestionBackendResultGrading, GradingResultId, JobId,
    LoadedRemoteQuestionBackendSession, MAX_REMOTE_QUESTION_BACKEND_STATE_CIPHERTEXT_BYTES,
    MAX_REMOTE_QUESTION_BACKEND_STATE_PLAINTEXT_BYTES, MemoryRemoteQuestionBackendSessionStore,
    QualifiedLaunchBindingDigest, QuestionSubmissionGradingId, RemoteQuestionBackendGradingContext,
    RemoteQuestionBackendGradingJobLease, RemoteQuestionBackendLaunchPreparationValidation,
    RemoteQuestionBackendNormalizedScore, RemoteQuestionBackendResponseChecksum,
    RemoteQuestionBackendResult, RemoteQuestionBackendResultChecksum,
    RemoteQuestionBackendResultExchangeIdempotencyKey, RemoteQuestionBackendResultToken,
    RemoteQuestionBackendResultTokenChecksum, RemoteQuestionBackendSession,
    RemoteQuestionBackendSessionAuthentication, RemoteQuestionBackendSessionChallenge,
    RemoteQuestionBackendSessionCreate, RemoteQuestionBackendSessionLease,
    RemoteQuestionBackendSessionPreparationContext, RemoteQuestionBackendSessionReference,
    RemoteQuestionBackendSessionRestoreExpectation, RemoteQuestionBackendSessionStore,
    RemoteQuestionBackendSessionValidation, RemoteQuestionBackendStateCipher,
    RemoteQuestionBackendStateKeyId, RemoteQuestionBackendStateKeyRing,
    RemoteQuestionBackendStatePlaintext, StageVerifiedRemoteQuestionBackendResult,
    derive_remote_question_backend_grading_result,
};
#[allow(unused_imports)] // Crate-private PostgreSQL Store row-binding surface.
pub(crate) use remote_question_backend_session::{
    RemoteQuestionBackendGradingJobLeaseParts, RemoteQuestionBackendSessionCreateParts,
    RemoteQuestionBackendSessionLeaseParts, RemoteQuestionBackendSessionRestoreParts,
    RemoteQuestionBackendSessionStorageParts, RemoteQuestionBackendStateCipherStorageParts,
    StageVerifiedRemoteQuestionBackendResultParts,
};
pub use session::{
    SessionId, SessionLifetime, SessionRecord, SessionStore, SessionTokenHash,
    SessionTokenHashParseError,
};
pub use store_error::StoreError;
