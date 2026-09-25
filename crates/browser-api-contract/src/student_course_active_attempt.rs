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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn absent_active_attempt_is_explicit_and_closed() {
        let response = StudentCourseActiveAttempt {
            assessment_attempt_id: None,
            started_at: None,
            latest_activity_at: None,
        };
        assert_eq!(
            serde_json::to_value(response).expect("response serializes"),
            json!({
                "assessmentAttemptId": null,
                "startedAt": null,
                "latestActivityAt": null
            })
        );
    }
}
