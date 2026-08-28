//! Automated-grading persistence contract front door.

pub use crate::contracts::{
    ACCEPTED_SUBMISSION_JOB_MAX_ATTEMPTS, AcceptedSubmission, AcceptedSubmissionCommand,
    AcceptedSubmissionCommitError, AcceptedSubmissionExecution, AcceptedSubmissionExecutionClaim,
    AcceptedSubmissionExecutionDisposition, AcceptedSubmissionExecutionFastPathClaimStore,
    AcceptedSubmissionExecutionLoadError, AcceptedSubmissionExecutionOutcome,
    AcceptedSubmissionExecutionRecoveryClaimStore, AcceptedSubmissionExecutionStore,
    AcceptedSubmissionExecutionTarget, AcceptedSubmissionGrade, AcceptedSubmissionId,
    AutomatedGradingStore, CanonicalAttemptResult, GradingExecution, GradingExecutionGeneration,
    GradingExecutionReceipt, GradingExecutionReceiptSafeCategory, GradingExecutionState,
    GradingOperation, GradingOperationActionId, GradingOperationActionReceipt,
    GradingOperationGroup, GradingOperationGroupBy, GradingOperationReceiptSafeCategory,
    GradingOperationRevision, GradingOperationStore, GradingOperationTarget,
    GradingOperationTrustGeneration, InstructorGradingOperationProjection,
    InstructorGradingOperationRow, ListInstructorGradingOperationsCommand,
    MAX_INSTRUCTOR_GRADING_RETRY_COUNT, RecalculateAssignmentCommand, RetryGradingOperationCommand,
    WorkerId, canonical_attempt_result_json, canonical_student_response_json,
};
