//! Fixed Coursework shortcut to the latest eligible resumable Course Attempt.

use question_model::{AssessmentAttemptId, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentCourseActiveAttempt {
    pub assessment_attempt_id: Option<AssessmentAttemptId>,
    pub started_at: Option<Timestamp>,
    pub latest_activity_at: Option<Timestamp>,
}
