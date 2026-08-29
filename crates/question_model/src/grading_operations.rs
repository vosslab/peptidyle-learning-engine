//! Closed browser-safe contracts for automated-grading recovery.
//!
//! These values describe state and safe next actions. They carry neither a
//! Student response nor a grade, answer, provider diagnostic, or private source.

use serde::{Deserialize, Serialize};

/// Current Student-safe evaluation projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionEvaluationStatus {
    AutomatedPending,
    AutomatedException,
    Graded,
    Exempt,
}

/// The Student-visible subset of an automated evaluation state.
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
            SubmissionEvaluationStatus::AutomatedException => Self::InstructorAttention,
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
#[serde(rename_all = "snake_case")]
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
    fn student_status_is_closed_and_does_not_expose_exception_detail() {
        assert_eq!(
            serde_json::to_string(&SubmissionEvaluationStatus::AutomatedPending)
                .expect("serializes"),
            "\"automated_pending\""
        );
        assert_eq!(
            serde_json::to_string(&SubmissionEvaluationStatus::AutomatedException)
                .expect("serializes"),
            "\"automated_exception\""
        );
        assert_eq!(
            serde_json::to_string(&SubmissionEvaluationStatus::Graded).expect("serializes"),
            "\"graded\""
        );
        assert_eq!(
            serde_json::to_string(&SubmissionEvaluationStatus::Exempt).expect("serializes"),
            "\"exempt\""
        );
        assert_eq!(
            serde_json::from_str::<SubmissionEvaluationStatus>("\"automated_pending\"")
                .expect("deserializes"),
            SubmissionEvaluationStatus::AutomatedPending
        );
        assert_eq!(
            serde_json::from_str::<SubmissionEvaluationStatus>("\"automated_exception\"")
                .expect("deserializes"),
            SubmissionEvaluationStatus::AutomatedException
        );
        assert_eq!(
            serde_json::from_str::<SubmissionEvaluationStatus>("\"graded\"").expect("deserializes"),
            SubmissionEvaluationStatus::Graded
        );
        assert_eq!(
            serde_json::from_str::<SubmissionEvaluationStatus>("\"exempt\"").expect("deserializes"),
            SubmissionEvaluationStatus::Exempt
        );
        assert!(
            serde_json::from_str::<SubmissionEvaluationStatus>("\"automatedPending\"").is_err()
        );
        assert_eq!(
            AutomatedGradingStatus::from(SubmissionEvaluationStatus::AutomatedException),
            AutomatedGradingStatus::InstructorAttention
        );
        assert_eq!(
            AutomatedGradingStatus::from(SubmissionEvaluationStatus::AutomatedPending),
            AutomatedGradingStatus::Pending
        );
        assert_eq!(
            AutomatedGradingStatus::from(SubmissionEvaluationStatus::Graded),
            AutomatedGradingStatus::Graded
        );
        assert_eq!(
            AutomatedGradingStatus::from(SubmissionEvaluationStatus::Exempt),
            AutomatedGradingStatus::Graded
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
            serde_json::to_string(&GradingOperationState::ActionInProgress).expect("serializes"),
            "\"action_in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&GradingOperationAction::Recalculate).expect("serializes"),
            "\"recalculate\""
        );
    }
}
