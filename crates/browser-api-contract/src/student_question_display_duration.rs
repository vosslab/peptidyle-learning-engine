//! Cumulative, idempotent Question display-duration checkpoint contract.

use question_model::AssessmentAttemptId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentQuestionDisplayDurationCheckpointRequest {
    pub cumulative_display_duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentQuestionDisplayDurationCheckpoint {
    pub assessment_attempt_id: AssessmentAttemptId,
    pub position: u32,
    pub cumulative_display_duration_ms: u64,
}
