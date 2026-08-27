//! Automated-grading persistence contract front door.

pub use crate::contracts::{
    AcceptedSubmission, AcceptedSubmissionCommand, AcceptedSubmissionCommitError,
    AcceptedSubmissionExecution, AcceptedSubmissionExecutionClaim,
    AcceptedSubmissionExecutionDisposition, AcceptedSubmissionExecutionOutcome,
    AcceptedSubmissionExecutionStore, AcceptedSubmissionExecutionWorkerStore,
    AcceptedSubmissionGrade, AcceptedSubmissionId, AutomatedGradingStore, CanonicalAttemptResult,
    GradingExecution, GradingExecutionGeneration, GradingExecutionReceipt, GradingExecutionState,
    GradingOperation, GradingOperationActionId, GradingOperationReceipt, GradingOperationRevision,
    GradingOperationTarget, WorkerId, canonical_attempt_result_json,
    canonical_student_response_json,
};
