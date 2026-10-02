//! Student shortcut to the newest submitted Attempt with released feedback.

use question_model::{AssessmentAttemptId, CourseInstanceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentLatestFeedback {
    pub assessment_attempt_id: AssessmentAttemptId,
    pub course_instance_id: CourseInstanceId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn latest_feedback_selection_contains_both_navigation_ids() {
        assert_eq!(
            serde_json::to_value(StudentLatestFeedback {
                assessment_attempt_id: "00000000-0000-0000-0000-000000000001"
                    .parse::<AssessmentAttemptId>()
                    .expect("Attempt ID"),
                course_instance_id: "CI6F2R8TA0"
                    .parse::<CourseInstanceId>()
                    .expect("Course Instance ID"),
            })
            .expect("response serializes"),
            json!({
                "assessmentAttemptId": "00000000-0000-0000-0000-000000000001",
                "courseInstanceId": "CI6F2R8TA0"
            })
        );
    }
}
