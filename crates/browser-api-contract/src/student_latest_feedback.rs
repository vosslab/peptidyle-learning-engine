//! Student shortcut to the newest submitted Attempt with released feedback.

use question_model::AssessmentAttemptId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentLatestFeedback {
    pub assessment_attempt_id: Option<AssessmentAttemptId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn no_feedback_is_an_explicit_empty_shortcut() {
        assert_eq!(
            serde_json::to_value(StudentLatestFeedback {
                assessment_attempt_id: None,
            })
            .expect("response serializes"),
            json!({"assessmentAttemptId": null})
        );
    }
}
