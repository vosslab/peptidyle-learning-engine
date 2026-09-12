//! Decoding of Assignment Workspace policy columns returned by PostgreSQL.

use question_model::{
    AssignmentActivityRules, AssignmentAttemptContinuationRule, AssignmentAttemptGradeRule,
    AssignmentAttemptResumeRule, AssignmentCompletionRule, AssignmentNavigationRule,
    AssignmentQuestionDisplayRule, AssignmentQuestionOrderRule, AssignmentQuestionVariationRule,
    QuestionPoolReuseRule, StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming,
};
use sqlx::Row;

use super::{assignment_release::invalid, connection::map_sqlx_error};
use crate::StoreError;

pub(super) fn activity_rules(
    row: &sqlx::postgres::PgRow,
) -> Result<AssignmentActivityRules, StoreError> {
    let value = |column| row.try_get::<String, _>(column).map_err(map_sqlx_error);
    Ok(AssignmentActivityRules {
        assignment_completion_rule: match value("assignment_completion_rule")?.as_str() {
            "answer_all" => AssignmentCompletionRule::AnswerAll,
            "all_correct" => AssignmentCompletionRule::AllCorrect,
            "score_at_least" => AssignmentCompletionRule::ScoreAtLeast {
                fraction: row
                    .try_get::<Option<f64>, _>("assignment_completion_score_threshold")
                    .map_err(map_sqlx_error)?
                    .ok_or_else(|| invalid("Assignment Completion Score Threshold"))?,
            },
            _ => return Err(invalid("Assignment Completion Rule")),
        },
        assignment_attempt_grade_rule: match value("assignment_attempt_grade_rule")?.as_str() {
            "first" => AssignmentAttemptGradeRule::First,
            "latest" => AssignmentAttemptGradeRule::Latest,
            "highest" => AssignmentAttemptGradeRule::Highest,
            "instructor_selected" => AssignmentAttemptGradeRule::InstructorSelected,
            _ => return Err(invalid("Assignment Attempt Grade Rule")),
        },
        assignment_attempt_continuation_rule: match value("assignment_attempt_continuation_rule")?
            .as_str()
        {
            "unlimited" => AssignmentAttemptContinuationRule::Unlimited,
            "closed" => AssignmentAttemptContinuationRule::Closed,
            "capped" => AssignmentAttemptContinuationRule::Capped {
                max_additional_assignment_attempts: u32::try_from(
                    row.try_get::<Option<i32>, _>("max_additional_assignment_attempts")
                        .map_err(map_sqlx_error)?
                        .ok_or_else(|| invalid("Maximum Additional Assignment Attempts"))?,
                )
                .map_err(|_| invalid("Maximum Additional Assignment Attempts"))?,
            },
            _ => return Err(invalid("Assignment Attempt Continuation Rule")),
        },
        question_pool_reuse_rule: match value("question_pool_reuse_rule")?.as_str() {
            "reuse_selection" => QuestionPoolReuseRule::ReuseSelection,
            "select_again" => QuestionPoolReuseRule::SelectAgain,
            _ => return Err(invalid("Question Pool Reuse Rule")),
        },
        question_variation_rule: match value("question_variation_rule")?.as_str() {
            "reuse_variation" => AssignmentQuestionVariationRule::ReuseVariation,
            "new_variation" => AssignmentQuestionVariationRule::NewVariation,
            _ => return Err(invalid("Question Variation Rule")),
        },
        assignment_attempt_resume_rule: match value("assignment_attempt_resume_rule")?.as_str() {
            "resumable" => AssignmentAttemptResumeRule::Resumable,
            "single_session" => AssignmentAttemptResumeRule::SingleSession,
            _ => return Err(invalid("Assignment Attempt Resume Rule")),
        },
        assignment_question_display_rule: match value("assignment_question_display_rule")?.as_str()
        {
            "one_question_at_a_time" => AssignmentQuestionDisplayRule::OneQuestionAtATime,
            "all_questions" => AssignmentQuestionDisplayRule::AllQuestions,
            _ => return Err(invalid("Assignment Question Display Rule")),
        },
        assignment_navigation_rule: match value("assignment_navigation_rule")?.as_str() {
            "free_navigation" => AssignmentNavigationRule::FreeNavigation,
            "forward_only" => AssignmentNavigationRule::ForwardOnly,
            _ => return Err(invalid("Assignment Navigation Rule")),
        },
        assignment_question_order_rule: match value("assignment_question_order_rule")?.as_str() {
            "authored_order" => AssignmentQuestionOrderRule::AuthoredOrder,
            "shuffled" => AssignmentQuestionOrderRule::Shuffled,
            _ => return Err(invalid("Assignment Question Order Rule")),
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
        question_feedback: feedback_timing(
            row.try_get("feedback_question_feedback")
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
