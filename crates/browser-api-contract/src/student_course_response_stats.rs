//! Self-only exact Published Question Revision Response Stats contract.

use question_model::{AssessmentAttemptId, PublishedQuestionRevisionTuple};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentCourseResponseQuestionStats {
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    pub full_credit_attempt_count: u64,
    pub partial_credit_attempt_count: u64,
    pub incorrect_attempt_count: u64,
    pub unanswered_attempt_count: u64,
    pub disclosed_attempt_count: u64,
    pub not_full_credit_count: u64,
    pub average_display_duration_ms: Option<f64>,
    pub display_duration_sample_count: u64,
    pub relevant_assessment_attempt_id: AssessmentAttemptId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentCourseResponseStats {
    pub questions: Vec<StudentCourseResponseQuestionStats>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{AssessmentAttemptId, PublishedQuestionId, QuestionRevisionNumber};

    #[test]
    fn contract_contains_only_disclosed_counts_and_exact_question_revision() {
        let stats = StudentCourseResponseStats {
            questions: vec![StudentCourseResponseQuestionStats {
                published_question_revision_tuple: PublishedQuestionRevisionTuple {
                    published_question_id: "7K3M-79QP"
                        .parse::<PublishedQuestionId>()
                        .expect("Published Question ID"),
                    revision_number: QuestionRevisionNumber::new(3).expect("revision"),
                },
                full_credit_attempt_count: 1,
                partial_credit_attempt_count: 1,
                incorrect_attempt_count: 0,
                unanswered_attempt_count: 0,
                disclosed_attempt_count: 2,
                not_full_credit_count: 1,
                average_display_duration_ms: None,
                display_duration_sample_count: 0,
                relevant_assessment_attempt_id: "00000000-0000-0000-0000-000000000001"
                    .parse::<AssessmentAttemptId>()
                    .expect("Attempt ID"),
            }],
        };
        let wire = serde_json::to_value(stats).expect("Response Stats serializes");
        assert_eq!(
            wire["questions"][0]["publishedQuestionRevisionTuple"]["revisionNumber"],
            3
        );
        assert_eq!(wire["questions"][0]["disclosedAttemptCount"], 2);
        assert_eq!(
            wire["questions"][0]["averageDisplayDurationMs"],
            serde_json::Value::Null
        );
        assert_eq!(wire["questions"][0]["displayDurationSampleCount"], 0);
        assert!(wire["questions"][0].get("correctAnswer").is_none());
        assert!(wire["questions"][0].get("timezone").is_none());
    }
}
