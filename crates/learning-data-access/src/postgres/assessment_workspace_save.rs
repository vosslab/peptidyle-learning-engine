//! PostgreSQL payload encoding for current Assessment Workspace saves.

use question_model::{
    AssessmentActivityRules, AssessmentAttemptContinuationRule, AssessmentAttemptGradeRule,
    AssessmentAttemptResumeRule, AssessmentCompletionRule, AssessmentEntry,
    AssessmentEntryAvailability, AssessmentEntryScoringRule, AssessmentQuestionDisplayRule,
    AssessmentQuestionOrderRule, AssessmentQuestionVariationRule, QuestionAttemptTimeLimit,
    QuestionPoolReuseRule, StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming,
};
use serde_json::{Value, json};

use super::assessment_release::{invalid, late_work_rule};
use crate::{SaveBaseAssessmentPolicyInput, SaveLiveAssessmentInput, StoreError};

#[rustfmt::skip]
fn activity_rule_values(r: &AssessmentActivityRules) -> [&'static str; 9] { [match r.assessment_completion_rule { AssessmentCompletionRule::AnswerAll => "answer_all", AssessmentCompletionRule::AllCorrect => "all_correct", AssessmentCompletionRule::ScoreAtLeast { .. } => "score_at_least" }, match r.assessment_attempt_grade_rule { AssessmentAttemptGradeRule::First => "first", AssessmentAttemptGradeRule::Latest => "latest", AssessmentAttemptGradeRule::Highest => "highest", AssessmentAttemptGradeRule::InstructorSelected => "instructor_selected" }, match r.assessment_attempt_continuation_rule { AssessmentAttemptContinuationRule::Unlimited => "unlimited", AssessmentAttemptContinuationRule::Capped { .. } => "capped", AssessmentAttemptContinuationRule::Closed => "closed" }, match r.question_pool_reuse_rule { QuestionPoolReuseRule::ReuseSelection => "reuse_selection", QuestionPoolReuseRule::SelectAgain => "select_again" }, match r.question_variation_rule { AssessmentQuestionVariationRule::ReuseVariation => "reuse_variation", AssessmentQuestionVariationRule::NewVariation => "new_variation" }, match r.assessment_attempt_resume_rule { AssessmentAttemptResumeRule::Resumable => "resumable", AssessmentAttemptResumeRule::SingleSession => "single_session" }, match r.assessment_question_display_rule { AssessmentQuestionDisplayRule::AllQuestions => "all_questions", AssessmentQuestionDisplayRule::OneQuestionAtATime => "one_question_at_a_time" }, match r.assessment_navigation_rule { question_model::AssessmentNavigationRule::FreeNavigation => "free_navigation", question_model::AssessmentNavigationRule::ForwardOnly => "forward_only" }, match r.assessment_question_order_rule { AssessmentQuestionOrderRule::AuthoredOrder => "authored_order", AssessmentQuestionOrderRule::Shuffled => "shuffled" }] }
#[rustfmt::skip]
fn activity_rule_extras(r: &AssessmentActivityRules) -> (Option<f64>, Option<i32>) { (match r.assessment_completion_rule { AssessmentCompletionRule::ScoreAtLeast { fraction } => Some(fraction), _ => None }, match r.assessment_attempt_continuation_rule { AssessmentAttemptContinuationRule::Capped { max_additional_assessment_attempts } => i32::try_from(max_additional_assessment_attempts).ok(), _ => None }) }
#[rustfmt::skip]
fn feedback_rule_values(r: &StudentFeedbackReleaseRule) -> [&'static str; 7] { [feedback_value(r.score), feedback_value(r.per_item_correctness), feedback_value(r.submitted_response), feedback_value(r.question_feedback), feedback_value(r.question_answer), feedback_value(r.question_answer_explanation), feedback_value(r.class_statistics)] }
#[rustfmt::skip]
fn feedback_value(v: StudentFeedbackReleaseTiming) -> &'static str { match v { StudentFeedbackReleaseTiming::DuringAttempt => "during_attempt", StudentFeedbackReleaseTiming::AfterSubmit => "after_submit", StudentFeedbackReleaseTiming::AfterDue => "after_due", StudentFeedbackReleaseTiming::AfterClose => "after_close", StudentFeedbackReleaseTiming::Never => "never" } }

#[rustfmt::skip]
pub(super) fn assessment_values_json(input: &SaveLiveAssessmentInput) -> Result<Value, StoreError> {
    let activity = activity_rule_values(&input.activity_rules);
    let (completion_threshold, additional_attempts) = activity_rule_extras(&input.activity_rules);
    let feedback = feedback_rule_values(&input.student_feedback_release_rule);
    Ok(json!({
        "assessment_title": input.title.as_str(), "assessment_instructions": input.instructions.as_str(),
        "available_at": Value::Null, "due_at": Value::Null, "closes_at": Value::Null,
        "assessment_attempt_time_limit_seconds": input.assessment_attempt_time_limit_seconds.map(|value| value.get()), "assessment_attempt_limit": input.attempt_limit.map(|value| value.get()), "late_work_rule": late_work_rule(&input.late_work_rule),
        "assessment_completion_rule": activity[0], "assessment_completion_score_threshold": completion_threshold, "assessment_attempt_grade_rule": activity[1], "assessment_attempt_continuation_rule": activity[2], "max_additional_assessment_attempts": additional_attempts,
        "question_pool_reuse_rule": activity[3], "question_variation_rule": activity[4], "assessment_attempt_resume_rule": activity[5], "assessment_question_display_rule": activity[6], "assessment_navigation_rule": activity[7], "assessment_question_order_rule": activity[8],
        "feedback_score": feedback[0], "feedback_per_item_correctness": feedback[1], "feedback_submitted_response": feedback[2], "feedback_question_feedback": feedback[3], "feedback_question_answer": feedback[4], "feedback_question_answer_explanation": feedback[5], "feedback_class_statistics": feedback[6]
    }))
}

/// Encodes the closed Base Assessment Policy allowlist; title and Entries have no representation.
pub(super) fn base_assessment_policy_values_json(input: &SaveBaseAssessmentPolicyInput) -> Value {
    let activity = activity_rule_values(&input.activity_rules);
    let (completion_threshold, additional_attempts) = activity_rule_extras(&input.activity_rules);
    let feedback = feedback_rule_values(&input.student_feedback_release_rule);
    json!({
        "assessment_instructions": input.instructions.as_str(), "available_at": Value::Null, "due_at": Value::Null, "closes_at": Value::Null,
        "assessment_attempt_time_limit_seconds": input.assessment_attempt_time_limit_seconds.map(|value| value.get()), "assessment_attempt_limit": input.attempt_limit.map(|value| value.get()), "late_work_rule": late_work_rule(&input.late_work_rule),
        "assessment_completion_rule": activity[0], "assessment_completion_score_threshold": completion_threshold, "assessment_attempt_grade_rule": activity[1], "assessment_attempt_continuation_rule": activity[2], "max_additional_assessment_attempts": additional_attempts,
        "question_pool_reuse_rule": activity[3], "question_variation_rule": activity[4], "assessment_attempt_resume_rule": activity[5], "assessment_question_display_rule": activity[6], "assessment_navigation_rule": activity[7], "assessment_question_order_rule": activity[8],
        "feedback_score": feedback[0], "feedback_per_item_correctness": feedback[1], "feedback_submitted_response": feedback[2], "feedback_question_feedback": feedback[3], "feedback_question_answer": feedback[4], "feedback_question_answer_explanation": feedback[5], "feedback_class_statistics": feedback[6]
    })
}

pub(super) fn assessment_entries_json(entries: &[AssessmentEntry]) -> Result<Value, StoreError> {
    entries
        .iter()
        .enumerate()
        .map(|(position, entry)| entry_json(entry, position))
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}
#[rustfmt::skip]
fn entry_json(entry: &AssessmentEntry, position: usize) -> Result<Value, StoreError> {
    let position = i32::try_from(position).map_err(|_| invalid("Assessment Entry position"))?;
    match entry {
        AssessmentEntry::FixedQuestion(value) => { let (seconds, grace_seconds) = question_time_limit(&value.question_attempt_time_limit)?; Ok(json!({ "assessmentEntryId": value.id.to_string(), "authoredPosition": position, "kind": "fixed_question", "availability": entry_availability(value.availability), "scoringRule": scoring_rule(value.scoring_rule), "questionId": value.reference.question_id.as_compact_str(), "revisionNumber": value.reference.revision_number.get(), "pointsPossible": value.points_possible.to_string(), "questionAttemptLimit": value.question_attempt_limit.max_attempts, "questionAttemptTimeLimitSeconds": seconds, "questionAttemptGraceSeconds": grace_seconds })) }
        AssessmentEntry::QuestionPool(value) => { let (seconds, grace_seconds) = question_time_limit(&value.question_attempt_time_limit)?; Ok(json!({ "assessmentEntryId": value.id.to_string(), "authoredPosition": position, "kind": "question_pool", "availability": entry_availability(value.availability), "scoringRule": scoring_rule(value.scoring_rule), "questionPoolId": value.question_pool_revision.question_pool_id.as_compact_str(), "questionPoolRevisionNumber": value.question_pool_revision.revision_number.get(), "selectionCount": value.selection_count.get(), "pointsPerItem": value.points_per_item.to_string(), "selectedQuestionOrder": selected_question_order(value.selection_rule.selected_question_order), "questionAttemptLimit": value.question_attempt_limit.max_attempts, "questionAttemptTimeLimitSeconds": seconds, "questionAttemptGraceSeconds": grace_seconds })) }
    }
}
#[rustfmt::skip]
fn question_time_limit(v: &QuestionAttemptTimeLimit) -> Result<(Option<u32>, Option<u32>), StoreError> { Ok(match v { QuestionAttemptTimeLimit::Unlimited => (None, None), QuestionAttemptTimeLimit::Limited { seconds, grace_seconds } => (Some(*seconds), Some(*grace_seconds)) }) }
#[rustfmt::skip]
fn entry_availability(v: AssessmentEntryAvailability) -> &'static str { match v { AssessmentEntryAvailability::Available => "available", AssessmentEntryAvailability::Retired => "retired" } }
#[rustfmt::skip]
#[rustfmt::skip]
fn scoring_rule(v: AssessmentEntryScoringRule) -> &'static str { match v { AssessmentEntryScoringRule::Normal => "normal", AssessmentEntryScoringRule::FullCredit => "full_credit", AssessmentEntryScoringRule::ExtraCredit => "extra_credit", AssessmentEntryScoringRule::Excluded => "excluded" } }
#[rustfmt::skip]
fn selected_question_order(v: question_model::QuestionPoolSelectedQuestionOrder) -> &'static str { match v { question_model::QuestionPoolSelectedQuestionOrder::QuestionPoolOrder => "question_pool_order", question_model::QuestionPoolSelectedQuestionOrder::RandomOrder => "random_order" } }

#[cfg(test)]
mod tests {
    use super::assessment_entries_json;
    use question_model::AssessmentEntry;
    #[test]
    fn mixed_exact_pins_and_pool_policies_encode_without_loss() {
        let entries: Vec<AssessmentEntry> = serde_json::from_value(serde_json::json!([
            {"kind":"fixedQuestion","id":"00000000-0000-0000-0000-000000000001","reference":{"questionId":"7K3M-X9QP","revisionNumber":2},"pointsPossible":"3.5","availability":"available","scoringRule":"normal","questionAttemptLimit":{"maxAttempts":2},"questionAttemptTimeLimit":{"kind":"limited","seconds":90,"graceSeconds":5}},
            {"kind":"questionPool","id":"00000000-0000-0000-0000-000000000002","questionPoolRevision":{"questionPoolId":"7K3M-X9QP","revisionNumber":1},"availability":"retired","scoringRule":"extraCredit","selectionCount":1,"pointsPerItem":"2","selectionRule":{"selectedQuestionOrder":"randomOrder"},"questionAttemptLimit":{"maxAttempts":null},"questionAttemptTimeLimit":{"kind":"unlimited"}}
        ])).expect("mixed exact-pinned entries deserialize");
        let value = assessment_entries_json(&entries).expect("entries encode for PostgreSQL");
        assert_eq!(
            value[0]["assessmentEntryId"],
            "00000000-0000-0000-0000-000000000001"
        );
        assert_eq!(value[0]["revisionNumber"], 2);
        assert_eq!(value[0]["questionId"], "7K3MX9QP");
        assert_eq!(value[0]["questionAttemptGraceSeconds"], 5);
        assert_eq!(value[1]["selectedQuestionOrder"], "random_order");
        assert_eq!(value[1]["questionPoolId"], "7K3MX9QP");
        assert_eq!(value[1]["questionPoolRevisionNumber"], 1);
    }
}
