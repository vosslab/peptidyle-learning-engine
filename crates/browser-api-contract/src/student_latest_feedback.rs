//! Student shortcut to the newest submitted Attempt with released feedback.

use question_model::{AssessmentAttemptId, CourseInstanceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentLatestFeedback {
    pub assessment_attempt_id: AssessmentAttemptId,
    pub course_instance_id: CourseInstanceId,
}
