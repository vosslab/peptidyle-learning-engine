//! Minimal Course-scoped Instructor Student time configuration wire contract.
use serde::{Deserialize, Serialize};

/// Current time configuration and finite preview for one active Course Student.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentStudentTimeAccommodation {
    /// Existing Course roster identifier, never a Student Record identity.
    pub roster_id: String,
    /// Unset means standard Assessment time; finite custom values are >= 1.
    pub time_multiplier: Option<f64>,
    /// Current accommodation Edit Number; unset means no accommodation row.
    pub edit_number: Option<String>,
    /// Current authored/default base; empty drafts have no duration yet.
    pub base_duration_seconds: Option<u32>,
    /// Multiplied duration rounded upward and capped at 24 hours.
    pub effective_duration_seconds: Option<u32>,
    /// True when the 24-hour ceiling reduces the multiplied duration.
    pub capped_at_24_hours: bool,
}

/// Explicit time-only save, preserving date and Attempt-count adjustments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveAssessmentStudentTimeAccommodationInput {
    /// Unset clears only the time multiplier.
    pub time_multiplier: Option<f64>,
    /// Current accommodation Edit Number; unset expects no row.
    pub expected_edit_number: Option<String>,
}
