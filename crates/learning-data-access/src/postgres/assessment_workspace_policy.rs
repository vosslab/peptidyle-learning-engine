//! Decoding of Assessment Workspace policy columns returned by PostgreSQL.

use question_model::{
    AssessmentActivityRules, AssessmentQuestionOrderRule, AssessmentQuestionVariationRule,
    StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming,
};
use sqlx::Row;

use super::{assessment_release::invalid, connection::map_sqlx_error};
use crate::StoreError;

pub(super) fn activity_rules(
    row: &sqlx::postgres::PgRow,
) -> Result<AssessmentActivityRules, StoreError> {
    let value = |column| row.try_get::<String, _>(column).map_err(map_sqlx_error);
    Ok(AssessmentActivityRules {
        question_variation_rule: match value("question_variation_rule")?.as_str() {
            "reuse_variation" => AssessmentQuestionVariationRule::ReuseVariation,
            "new_variation" => AssessmentQuestionVariationRule::NewVariation,
            _ => return Err(invalid("Question Variation Rule")),
        },
        assessment_question_order_rule: match value("assessment_question_order_rule")?.as_str() {
            "authored_order" => AssessmentQuestionOrderRule::AuthoredOrder,
            "shuffled" => AssessmentQuestionOrderRule::Shuffled,
            _ => return Err(invalid("Assessment Question Order Rule")),
        },
    })
}

fn feedback_timing(value: String) -> Result<StudentFeedbackReleaseTiming, StoreError> {
    match value.as_str() {
        "during_attempt" => Ok(StudentFeedbackReleaseTiming::DuringAttempt),
        "after_submit" => Ok(StudentFeedbackReleaseTiming::AfterSubmit),
        "after_due" => Ok(StudentFeedbackReleaseTiming::AfterDue),
        "after_close" => Ok(StudentFeedbackReleaseTiming::AfterClose),
        "never" => Ok(StudentFeedbackReleaseTiming::Never),
        _ => Err(invalid("Student Feedback Release Timing")),
    }
}

pub(super) fn feedback_rules(
    row: &sqlx::postgres::PgRow,
) -> Result<StudentFeedbackReleaseRule, StoreError> {
    Ok(StudentFeedbackReleaseRule {
        score: feedback_timing(row.try_get("feedback_score").map_err(map_sqlx_error)?)?,
        per_item_correctness: feedback_timing(
            row.try_get("feedback_per_item_correctness")
                .map_err(map_sqlx_error)?,
        )?,
        submitted_response: feedback_timing(
            row.try_get("feedback_submitted_response")
                .map_err(map_sqlx_error)?,
        )?,
        question_answer: feedback_timing(
            row.try_get("feedback_question_answer")
                .map_err(map_sqlx_error)?,
        )?,
        question_answer_explanation: feedback_timing(
            row.try_get("feedback_question_answer_explanation")
                .map_err(map_sqlx_error)?,
        )?,
        class_statistics: feedback_timing(
            row.try_get("feedback_class_statistics")
                .map_err(map_sqlx_error)?,
        )?,
    })
}
