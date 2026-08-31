//! Closed browser-safe contracts for automated-grading recovery.
//!
//! These values describe state and safe next actions. They carry neither a
//! Student response nor a grade, answer, provider diagnostic, or private source.

use serde::{Deserialize, Serialize};

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

/// Metadata-only public operation projection.
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
}
