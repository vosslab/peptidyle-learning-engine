//! Closed browser-safe contracts for automated-grading recovery.
//!
//! These values describe state and safe next actions. They carry neither a
//! learner response nor a grade, answer, provider diagnostic, or private source.

use serde::{Deserialize, Serialize};

/// Current learner-safe evaluation projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubmissionEvaluationStatus {
    AutomatedPending,
    AutomatedException,
    NeedsManualGrading,
    Graded,
    Exempt,
}

/// The learner-visible subset of an automated evaluation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomatedGradingStatus {
    Pending,
    Graded,
    InstructorAttention,
}

impl From<SubmissionEvaluationStatus> for AutomatedGradingStatus {
    fn from(value: SubmissionEvaluationStatus) -> Self {
        match value {
            SubmissionEvaluationStatus::AutomatedPending => Self::Pending,
            SubmissionEvaluationStatus::Graded | SubmissionEvaluationStatus::Exempt => Self::Graded,
            SubmissionEvaluationStatus::AutomatedException
            | SubmissionEvaluationStatus::NeedsManualGrading => Self::InstructorAttention,
        }
    }
}

/// Safe Instructor-visible reason for an actionable recovery thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GradingOperationReason {
    GraderContractFailure,
    GraderExecutionFailure,
    IssuedEvidenceIntegrity,
    RetryExhausted,
    ScoringRecalculationFailed,
}

/// Current server-owned operation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GradingOperationState {
    Actionable,
    ActionInProgress,
    Completed,
    RepairRequired,
    Failed,
    Superseded,
}

/// The one action an operation may expose at a given revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GradingOperationAction {
    Retry,
    Recalculate,
}

/// Metadata-only public operation projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GradingOperationVisibleState {
    pub state: GradingOperationState,
    pub reason: GradingOperationReason,
    pub next_action: Option<GradingOperationAction>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learner_status_is_closed_and_does_not_expose_exception_detail() {
        assert_eq!(
            AutomatedGradingStatus::from(SubmissionEvaluationStatus::AutomatedException),
            AutomatedGradingStatus::InstructorAttention
        );
        assert_eq!(
            serde_json::to_string(&AutomatedGradingStatus::Pending).expect("serializes"),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&AutomatedGradingStatus::Graded).expect("serializes"),
            "\"graded\""
        );
        assert_eq!(
            serde_json::to_string(&AutomatedGradingStatus::InstructorAttention)
                .expect("serializes"),
            "\"instructor_attention\""
        );
    }
}
