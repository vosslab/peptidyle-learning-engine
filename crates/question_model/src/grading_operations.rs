//! Closed browser-safe contracts for automated-grading recovery.
//!
//! These values describe state and safe next actions. They carry neither a
//! Student response nor a grade, answer, iMathAS Question Backend diagnostic, or private source.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{InstructorGradingOperationReference, Timestamp};

/// Exact SHA-256 Request Checksum held by the server for one grading action.
///
/// This is integrity evidence, not a browser identifier or authorization grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructorGradingOperationRequestChecksum([u8; 32]);

impl InstructorGradingOperationRequestChecksum {
    /// Records the trusted request boundary's complete checksum bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Opaque UUID supplied for one Instructor Grading Operation decision.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct InstructorGradingOperationRetryToken(uuid::Uuid);

impl InstructorGradingOperationRetryToken {
    /// Parses the canonical UUID transport value without granting authority.
    pub fn parse(value: &str) -> Result<Self, InstructorGradingOperationRetryTokenError> {
        uuid::Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| InstructorGradingOperationRetryTokenError)
    }
}

impl TryFrom<String> for InstructorGradingOperationRetryToken {
    type Error = InstructorGradingOperationRetryTokenError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<InstructorGradingOperationRetryToken> for String {
    fn from(value: InstructorGradingOperationRetryToken) -> Self {
        value.0.hyphenated().to_string()
    }
}

impl std::fmt::Debug for InstructorGradingOperationRetryToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("InstructorGradingOperationRetryToken([opaque])")
    }
}

/// A supplied Instructor Grading Operation Retry Token was not a UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructorGradingOperationRetryTokenError;

impl std::fmt::Display for InstructorGradingOperationRetryTokenError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Instructor Grading Operation Retry Token must be a UUID")
    }
}

impl std::error::Error for InstructorGradingOperationRetryTokenError {}

/// Current state of one accepted Question Submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionSubmissionGradingState {
    Pending,
    InstructorAttention,
    Graded,
    Exempt,
}

/// Student-visible state for one accepted Question Submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentQuestionSubmissionGradingState {
    Pending,
    Graded,
    InstructorAttention,
}

impl From<QuestionSubmissionGradingState> for StudentQuestionSubmissionGradingState {
    fn from(value: QuestionSubmissionGradingState) -> Self {
        match value {
            QuestionSubmissionGradingState::Pending => Self::Pending,
            QuestionSubmissionGradingState::Graded | QuestionSubmissionGradingState::Exempt => {
                Self::Graded
            }
            QuestionSubmissionGradingState::InstructorAttention => Self::InstructorAttention,
        }
    }
}

/// Safe Instructor-visible reason for an actionable recovery thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradingOperationReason {
    GraderContractFailure,
    GraderExecutionFailure,
    IssuedEvidenceIntegrity,
    RetryExhausted,
    ScoringRecalculationRequested,
    InstructorRequestedRecalculation,
    ScoringRecalculationFailed,
}

/// Current server-owned operation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructorGradingOperationState {
    Actionable,
    ActionInProgress,
    Completed,
    RepairRequired,
    Failed,
    Superseded,
}

/// The one action an operation may expose at a given revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradingOperationAction {
    Retry,
    Recalculate,
}

/// One server-held request binding for an Instructor Grading Operation action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructorGradingOperationActionRequest {
    operation: InstructorGradingOperationReference,
    action: GradingOperationAction,
    request_checksum: InstructorGradingOperationRequestChecksum,
    retry_token: InstructorGradingOperationRetryToken,
}

impl InstructorGradingOperationActionRequest {
    /// Binds the exact controlled operation, action, Request Checksum, and Retry Token.
    pub fn new(
        operation: InstructorGradingOperationReference,
        action: GradingOperationAction,
        request_checksum: InstructorGradingOperationRequestChecksum,
        retry_token: InstructorGradingOperationRetryToken,
    ) -> Self {
        Self {
            operation,
            action,
            request_checksum,
            retry_token,
        }
    }

    pub fn operation(&self) -> InstructorGradingOperationReference {
        self.operation
    }

    pub fn action(&self) -> GradingOperationAction {
        self.action
    }

    pub fn request_checksum(&self) -> InstructorGradingOperationRequestChecksum {
        self.request_checksum
    }

    pub fn retry_token(&self) -> &InstructorGradingOperationRetryToken {
        &self.retry_token
    }
}

/// Immutable server-held evidence that one grading action was accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructorGradingOperationReceipt {
    request: InstructorGradingOperationActionRequest,
    resulting_operation_revision: u64,
    assignment_revision: Option<u64>,
    scoring_generation: Option<u64>,
    occurred_at: Timestamp,
}

impl InstructorGradingOperationReceipt {
    /// Records the accepted result after the Store has performed the action exactly once.
    pub fn new(
        request: InstructorGradingOperationActionRequest,
        resulting_operation_revision: u64,
        assignment_revision: Option<u64>,
        scoring_generation: Option<u64>,
        occurred_at: Timestamp,
    ) -> Self {
        Self {
            request,
            resulting_operation_revision,
            assignment_revision,
            scoring_generation,
            occurred_at,
        }
    }

    pub fn request(&self) -> &InstructorGradingOperationActionRequest {
        &self.request
    }
}

/// In-memory conformance model for the Store's durable retry-token uniqueness rule.
///
/// Production persistence must apply the same comparison in its transaction; this
/// value deliberately makes no durability claim.
#[derive(Debug, Default)]
pub struct InstructorGradingOperationReplayLedger {
    receipts: BTreeMap<InstructorGradingOperationRetryToken, InstructorGradingOperationReceipt>,
}

impl InstructorGradingOperationReplayLedger {
    /// Returns the first accepted Receipt for an equal request without rerunning its effect.
    pub fn accept_or_replay(
        &mut self,
        request: InstructorGradingOperationActionRequest,
        accept: impl FnOnce(
            InstructorGradingOperationActionRequest,
        ) -> InstructorGradingOperationReceipt,
    ) -> Result<InstructorGradingOperationReplay, InstructorGradingOperationReplayError> {
        if let Some(receipt) = self.receipts.get(request.retry_token()) {
            return (receipt.request == request)
                .then(|| InstructorGradingOperationReplay::Replayed(receipt.clone()))
                .ok_or(InstructorGradingOperationReplayError::BindingMismatch);
        }
        let receipt = accept(request.clone());
        if receipt.request != request {
            return Err(InstructorGradingOperationReplayError::ReceiptBindingMismatch);
        }
        self.receipts
            .insert(request.retry_token.clone(), receipt.clone());
        Ok(InstructorGradingOperationReplay::Accepted(receipt))
    }
}

/// Whether a Store accepted a new grading action or replayed its accepted Receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructorGradingOperationReplay {
    Accepted(InstructorGradingOperationReceipt),
    Replayed(InstructorGradingOperationReceipt),
}

/// A Retry Token was reused for a different request, or a producer forged its Receipt binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructorGradingOperationReplayError {
    BindingMismatch,
    ReceiptBindingMismatch,
}

/// Metadata-only Grading Operation Visible State.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GradingOperationVisibleState {
    pub state: InstructorGradingOperationState,
    pub reason: GradingOperationReason,
    pub next_action: Option<GradingOperationAction>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn student_status_is_closed_and_does_not_expose_exception_detail() {
        assert_eq!(
            serde_json::to_string(&QuestionSubmissionGradingState::Pending).expect("serializes"),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&QuestionSubmissionGradingState::InstructorAttention)
                .expect("serializes"),
            "\"instructor_attention\""
        );
        assert_eq!(
            serde_json::to_string(&QuestionSubmissionGradingState::Graded).expect("serializes"),
            "\"graded\""
        );
        assert_eq!(
            serde_json::to_string(&QuestionSubmissionGradingState::Exempt).expect("serializes"),
            "\"exempt\""
        );
        assert_eq!(
            serde_json::from_str::<QuestionSubmissionGradingState>("\"pending\"")
                .expect("deserializes"),
            QuestionSubmissionGradingState::Pending
        );
        assert_eq!(
            serde_json::from_str::<QuestionSubmissionGradingState>("\"instructor_attention\"")
                .expect("deserializes"),
            QuestionSubmissionGradingState::InstructorAttention
        );
        assert_eq!(
            serde_json::from_str::<QuestionSubmissionGradingState>("\"graded\"")
                .expect("deserializes"),
            QuestionSubmissionGradingState::Graded
        );
        assert_eq!(
            serde_json::from_str::<QuestionSubmissionGradingState>("\"exempt\"")
                .expect("deserializes"),
            QuestionSubmissionGradingState::Exempt
        );
        assert!(
            serde_json::from_str::<QuestionSubmissionGradingState>("\"automated_pending\"")
                .is_err()
        );
        assert_eq!(
            StudentQuestionSubmissionGradingState::from(
                QuestionSubmissionGradingState::InstructorAttention
            ),
            StudentQuestionSubmissionGradingState::InstructorAttention
        );
        assert_eq!(
            StudentQuestionSubmissionGradingState::from(QuestionSubmissionGradingState::Pending),
            StudentQuestionSubmissionGradingState::Pending
        );
        assert_eq!(
            StudentQuestionSubmissionGradingState::from(QuestionSubmissionGradingState::Graded),
            StudentQuestionSubmissionGradingState::Graded
        );
        assert_eq!(
            StudentQuestionSubmissionGradingState::from(QuestionSubmissionGradingState::Exempt),
            StudentQuestionSubmissionGradingState::Graded
        );
        assert_eq!(
            serde_json::to_string(&StudentQuestionSubmissionGradingState::Pending)
                .expect("serializes"),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&StudentQuestionSubmissionGradingState::Graded)
                .expect("serializes"),
            "\"graded\""
        );
        assert_eq!(
            serde_json::to_string(&StudentQuestionSubmissionGradingState::InstructorAttention)
                .expect("serializes"),
            "\"instructor_attention\""
        );
    }

    #[test]
    fn operation_symbols_use_snake_case_across_runtimes() {
        assert_eq!(
            serde_json::to_string(&GradingOperationReason::InstructorRequestedRecalculation)
                .expect("serializes"),
            "\"instructor_requested_recalculation\""
        );
        assert_eq!(
            serde_json::to_string(&GradingOperationReason::ScoringRecalculationRequested)
                .expect("serializes"),
            "\"scoring_recalculation_requested\""
        );
        assert_eq!(
            serde_json::to_string(&InstructorGradingOperationState::ActionInProgress)
                .expect("serializes"),
            "\"action_in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&GradingOperationAction::Recalculate).expect("serializes"),
            "\"recalculate\""
        );
    }

    #[test]
    fn retry_token_replays_only_the_exact_accepted_request() {
        let token =
            InstructorGradingOperationRetryToken::parse("00000000-0000-0000-0000-000000000001")
                .expect("token");
        let request = InstructorGradingOperationActionRequest::new(
            InstructorGradingOperationReference::new(7).expect("operation"),
            GradingOperationAction::Retry,
            InstructorGradingOperationRequestChecksum::from_bytes([1; 32]),
            token.clone(),
        );
        let mut ledger = InstructorGradingOperationReplayLedger::default();
        let mut effects = 0;
        let first = ledger
            .accept_or_replay(request.clone(), |accepted_request| {
                effects += 1;
                InstructorGradingOperationReceipt::new(
                    accepted_request,
                    4,
                    None,
                    None,
                    Timestamp::from_unix_millis(1),
                )
            })
            .expect("first acceptance");
        let replay = ledger
            .accept_or_replay(request.clone(), |_| {
                panic!("replay must not repeat the effect")
            })
            .expect("replay");

        assert_eq!(effects, 1);
        assert_eq!(
            first,
            InstructorGradingOperationReplay::Accepted(match replay {
                InstructorGradingOperationReplay::Replayed(receipt) => receipt,
                InstructorGradingOperationReplay::Accepted(_) => panic!("must replay"),
            })
        );

        let changed_action = InstructorGradingOperationActionRequest::new(
            request.operation(),
            GradingOperationAction::Recalculate,
            request.request_checksum(),
            token,
        );
        assert_eq!(
            ledger.accept_or_replay(changed_action, |_| panic!("must refuse mismatch")),
            Err(InstructorGradingOperationReplayError::BindingMismatch)
        );
    }
}
