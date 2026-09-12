//! PostgreSQL payload encoding for current Assignment Workspace saves.

use question_model::{
    AssignmentActivityRules, AssignmentAttemptContinuationRule, AssignmentAttemptGradeRule,
    AssignmentAttemptResumeRule, AssignmentCompletionRule, AssignmentEntry,
    AssignmentEntryAvailability, AssignmentEntryScoringRule, AssignmentQuestionDisplayRule,
    AssignmentQuestionOrderRule, AssignmentQuestionVariationRule, QuestionAttemptTimeLimit,
    QuestionPoolItemAvailability, QuestionPoolReuseRule, StudentFeedbackReleaseRule,
    StudentFeedbackReleaseTiming,
};
use serde_json::{Value, json};

use super::assignment_release::{invalid, late_work_rule};
use crate::{SaveLiveAssignmentInput, StoreError};

#[rustfmt::skip]
fn activity_rule_values(r: &AssignmentActivityRules) -> [&'static str; 9] { [match r.assignment_completion_rule { AssignmentCompletionRule::AnswerAll => "answer_all", AssignmentCompletionRule::AllCorrect => "all_correct", AssignmentCompletionRule::ScoreAtLeast { .. } => "score_at_least" }, match r.assignment_attempt_grade_rule { AssignmentAttemptGradeRule::First => "first", AssignmentAttemptGradeRule::Latest => "latest", AssignmentAttemptGradeRule::Highest => "highest", AssignmentAttemptGradeRule::InstructorSelected => "instructor_selected" }, match r.assignment_attempt_continuation_rule { AssignmentAttemptContinuationRule::Unlimited => "unlimited", AssignmentAttemptContinuationRule::Capped { .. } => "capped", AssignmentAttemptContinuationRule::Closed => "closed" }, match r.question_pool_reuse_rule { QuestionPoolReuseRule::ReuseSelection => "reuse_selection", QuestionPoolReuseRule::SelectAgain => "select_again" }, match r.question_variation_rule { AssignmentQuestionVariationRule::ReuseVariation => "reuse_variation", AssignmentQuestionVariationRule::NewVariation => "new_variation" }, match r.assignment_attempt_resume_rule { AssignmentAttemptResumeRule::Resumable => "resumable", AssignmentAttemptResumeRule::SingleSession => "single_session" }, match r.assignment_question_display_rule { AssignmentQuestionDisplayRule::AllQuestions => "all_questions", AssignmentQuestionDisplayRule::OneQuestionAtATime => "one_question_at_a_time" }, match r.assignment_navigation_rule { question_model::AssignmentNavigationRule::FreeNavigation => "free_navigation", question_model::AssignmentNavigationRule::ForwardOnly => "forward_only" }, match r.assignment_question_order_rule { AssignmentQuestionOrderRule::AuthoredOrder => "authored_order", AssignmentQuestionOrderRule::Shuffled => "shuffled" }] }
#[rustfmt::skip]
fn activity_rule_extras(r: &AssignmentActivityRules) -> (Option<f64>, Option<i32>) { (match r.assignment_completion_rule { AssignmentCompletionRule::ScoreAtLeast { fraction } => Some(fraction), _ => None }, match r.assignment_attempt_continuation_rule { AssignmentAttemptContinuationRule::Capped { max_additional_assignment_attempts } => i32::try_from(max_additional_assignment_attempts).ok(), _ => None }) }
#[rustfmt::skip]
fn feedback_rule_values(r: &StudentFeedbackReleaseRule) -> [&'static str; 7] { [feedback_value(r.score), feedback_value(r.per_item_correctness), feedback_value(r.submitted_response), feedback_value(r.question_feedback), feedback_value(r.question_answer), feedback_value(r.question_answer_explanation), feedback_value(r.class_statistics)] }
#[rustfmt::skip]
fn feedback_value(v: StudentFeedbackReleaseTiming) -> &'static str { match v { StudentFeedbackReleaseTiming::DuringAttempt => "during_attempt", StudentFeedbackReleaseTiming::AfterSubmit => "after_submit", StudentFeedbackReleaseTiming::AfterDue => "after_due", StudentFeedbackReleaseTiming::AfterClose => "after_close", StudentFeedbackReleaseTiming::Never => "never" } }

#[rustfmt::skip]
pub(super) fn assignment_values_json(input: &SaveLiveAssignmentInput) -> Result<Value, StoreError> {
    let activity = activity_rule_values(&input.activity_rules);
    let (completion_threshold, additional_attempts) = activity_rule_extras(&input.activity_rules);
    let feedback = feedback_rule_values(&input.student_feedback_release_rule);
    Ok(json!({
        "assignment_title": input.title.as_str(), "assignment_instructions": input.instructions.as_str(),
        "available_at": Value::Null, "due_at": Value::Null, "closes_at": Value::Null,
        "assignment_attempt_time_limit_seconds": input.assignment_attempt_time_limit_seconds.map(|value| value.get()), "attempt_limit": input.attempt_limit.map(|value| value.get()), "late_work_rule": late_work_rule(&input.late_work_rule),
        "assignment_completion_rule": activity[0], "assignment_completion_score_threshold": completion_threshold, "assignment_attempt_grade_rule": activity[1], "assignment_attempt_continuation_rule": activity[2], "max_additional_assignment_attempts": additional_attempts,
        "question_pool_reuse_rule": activity[3], "question_variation_rule": activity[4], "assignment_attempt_resume_rule": activity[5], "assignment_question_display_rule": activity[6], "assignment_navigation_rule": activity[7], "assignment_question_order_rule": activity[8],
        "feedback_score": feedback[0], "feedback_per_item_correctness": feedback[1], "feedback_submitted_response": feedback[2], "feedback_question_feedback": feedback[3], "feedback_question_answer": feedback[4], "feedback_question_answer_explanation": feedback[5], "feedback_class_statistics": feedback[6]
    }))
}

pub(super) fn assignment_entries_json(entries: &[AssignmentEntry]) -> Result<Value, StoreError> {
    entries
        .iter()
        .enumerate()
        .map(|(position, entry)| entry_json(entry, position))
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}
#[rustfmt::skip]
fn entry_json(entry: &AssignmentEntry, position: usize) -> Result<Value, StoreError> {
    let position = i32::try_from(position).map_err(|_| invalid("Assignment Entry position"))?;
    match entry {
        AssignmentEntry::FixedQuestion(value) => { let (seconds, grace_seconds) = question_time_limit(&value.question_attempt_time_limit)?; Ok(json!({ "assignmentEntryId": value.id.to_string(), "authoredPosition": position, "kind": "fixed_question", "availability": entry_availability(value.availability), "scoringRule": scoring_rule(value.scoring_rule), "questionId": value.reference.question_id.as_compact_str(), "revisionNumber": value.reference.revision_number.get(), "pointsPossible": value.points_possible.to_string(), "questionAttemptLimit": value.question_attempt_limit.max_attempts, "questionAttemptTimeLimitSeconds": seconds, "questionAttemptGraceSeconds": grace_seconds })) }
        AssignmentEntry::QuestionPool(value) => { let (seconds, grace_seconds) = question_time_limit(&value.question_attempt_time_limit)?; let items = value.items.iter().enumerate().map(|(item_position, item)| Ok(json!({ "questionPoolItemId": item.id.to_string(), "itemPosition": i32::try_from(item_position).map_err(|_| invalid("Question Pool Item position"))?, "questionId": item.reference.question_id.as_compact_str(), "revisionNumber": item.reference.revision_number.get(), "availability": pool_item_availability(item.availability) }))).collect::<Result<Vec<_>, StoreError>>()?; Ok(json!({ "assignmentEntryId": value.id.to_string(), "authoredPosition": position, "kind": "question_pool", "availability": entry_availability(value.availability), "scoringRule": scoring_rule(value.scoring_rule), "selectionCount": value.selection_count, "pointsPerItem": value.points_per_item.to_string(), "selectedQuestionOrder": selected_question_order(value.selection_rule.selected_question_order), "questionAttemptLimit": value.question_attempt_limit.max_attempts, "questionAttemptTimeLimitSeconds": seconds, "questionAttemptGraceSeconds": grace_seconds, "items": items })) }
    }
}
#[rustfmt::skip]
fn question_time_limit(v: &QuestionAttemptTimeLimit) -> Result<(Option<u32>, Option<u32>), StoreError> { Ok(match v { QuestionAttemptTimeLimit::Unlimited => (None, None), QuestionAttemptTimeLimit::Limited { seconds, grace_seconds } => (Some(*seconds), Some(*grace_seconds)) }) }
#[rustfmt::skip]
fn entry_availability(v: AssignmentEntryAvailability) -> &'static str { match v { AssignmentEntryAvailability::Available => "available", AssignmentEntryAvailability::Retired => "retired" } }
#[rustfmt::skip]
fn pool_item_availability(v: QuestionPoolItemAvailability) -> &'static str { match v { QuestionPoolItemAvailability::Available => "available", QuestionPoolItemAvailability::Retired => "retired" } }
#[rustfmt::skip]
fn scoring_rule(v: AssignmentEntryScoringRule) -> &'static str { match v { AssignmentEntryScoringRule::Normal => "normal", AssignmentEntryScoringRule::FullCredit => "full_credit", AssignmentEntryScoringRule::ExtraCredit => "extra_credit", AssignmentEntryScoringRule::Excluded => "excluded" } }
#[rustfmt::skip]
fn selected_question_order(v: question_model::QuestionPoolSelectedQuestionOrder) -> &'static str { match v { question_model::QuestionPoolSelectedQuestionOrder::QuestionPoolOrder => "question_pool_order", question_model::QuestionPoolSelectedQuestionOrder::RandomOrder => "random_order" } }

#[cfg(test)]
mod tests {
    use super::assignment_entries_json;
    use question_model::AssignmentEntry;
    #[test]
    fn mixed_exact_pins_and_pool_policies_encode_without_loss() {
        let entries: Vec<AssignmentEntry> = serde_json::from_value(serde_json::json!([
            {"kind":"fixedQuestion","id":"00000000-0000-0000-0000-000000000001","reference":{"questionId":"7K3-M9QP","revisionNumber":2},"pointsPossible":"3.5","availability":"available","scoringRule":"normal","questionAttemptLimit":{"maxAttempts":2},"questionAttemptTimeLimit":{"kind":"limited","seconds":90,"graceSeconds":5}},
            {"kind":"questionPool","id":"00000000-0000-0000-0000-000000000002","availability":"retired","scoringRule":"extraCredit","selectionCount":1,"pointsPerItem":"2","selectionRule":{"selectedQuestionOrder":"randomOrder"},"questionAttemptLimit":{"maxAttempts":null},"questionAttemptTimeLimit":{"kind":"unlimited"},"items":[{"id":"00000000-0000-0000-0000-000000000003","reference":{"questionId":"7K3-M9QP","revisionNumber":2},"availability":"available"},{"id":"00000000-0000-0000-0000-000000000004","reference":{"questionId":"7K3-M9QX","revisionNumber":4},"availability":"retired"}]}
        ])).expect("mixed exact-pinned entries deserialize");
        let value = assignment_entries_json(&entries).expect("entries encode for PostgreSQL");
        assert_eq!(
            value[0]["assignmentEntryId"],
            "00000000-0000-0000-0000-000000000001"
        );
        assert_eq!(value[0]["revisionNumber"], 2);
        assert_eq!(value[0]["questionId"], "7K3M9QP");
        assert_eq!(value[0]["questionAttemptGraceSeconds"], 5);
        assert_eq!(value[1]["selectedQuestionOrder"], "random_order");
        assert_eq!(
            value[1]["items"][1]["questionPoolItemId"],
            "00000000-0000-0000-0000-000000000004"
        );
        assert_eq!(value[1]["items"][1]["availability"], "retired");
        assert_eq!(value[1]["items"][1]["questionId"], "7K3M9QX");
    }
}
