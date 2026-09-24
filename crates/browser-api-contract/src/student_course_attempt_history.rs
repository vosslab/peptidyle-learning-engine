//! Self-only Course-wide Student Attempt History wire contract.

use question_model::{AssessmentAttemptId, AssessmentId, Timestamp};
use serde::{Deserialize, Serialize};

use crate::student_course_progress::AssessmentPointScore;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentCourseAttemptHistoryEntry {
    pub assessment_attempt_id: AssessmentAttemptId,
    pub assessment_id: AssessmentId,
    pub assessment_title: String,
    pub assessment_attempt_number: u32,
    pub started_at: Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<Timestamp>,
    /// Present only when this Attempt's score is complete and disclosed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment_score: Option<AssessmentPointScore>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentCourseAttemptHistoryPage {
    pub items: Vec<StudentCourseAttemptHistoryEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::AssessmentAttemptId;
    use serde_json::json;

    #[test]
    fn absent_disclosed_score_is_omitted_and_page_has_no_timezone_field() {
        let page = StudentCourseAttemptHistoryPage {
            items: vec![StudentCourseAttemptHistoryEntry {
                assessment_attempt_id: "00000000-0000-0000-0000-000000000001"
                    .parse::<AssessmentAttemptId>()
                    .expect("Attempt ID"),
                assessment_id: AssessmentId::new("A5D9Q3XAH").expect("Assessment ID"),
                assessment_title: "Peptide practice".into(),
                assessment_attempt_number: 1,
                started_at: Timestamp::from_unix_millis(1_786_000_000_000),
                submitted_at: None,
                assessment_score: None,
            }],
            next_cursor: None,
        };
        assert_eq!(
            serde_json::to_value(page).expect("page serializes"),
            json!({"items": [{
                "assessmentAttemptId": "00000000-0000-0000-0000-000000000001",
                "assessmentId": "A5D9Q3XAH",
                "assessmentTitle": "Peptide practice",
                "assessmentAttemptNumber": 1,
                "startedAt": 1_786_000_000_000_i64
            }]})
        );
    }
}
