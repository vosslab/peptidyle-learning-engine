//! PostgreSQL payload encoding for current Assessment Workspace saves.

use question_model::{
    AssessmentActivityRules, AssessmentEntry, AssessmentEntryAvailability,
    AssessmentEntryScoringRule, AssessmentQuestionOrderRule, AssessmentQuestionVariationRule,
    QuestionAttemptTimeLimit, StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming,
};
use serde_json::{Value, json};

use super::assessment_release::{invalid, late_work_rule};
use crate::{SaveBaseAssessmentPolicyInput, SaveLiveAssessmentInput, StoreError};

#[rustfmt::skip]
fn activity_rule_values(r: &AssessmentActivityRules) -> [&'static str; 2] { [match r.question_variation_rule { AssessmentQuestionVariationRule::ReuseVariation => "reuse_variation", AssessmentQuestionVariationRule::NewVariation => "new_variation" }, match r.assessment_question_order_rule { AssessmentQuestionOrderRule::AuthoredOrder => "authored_order", AssessmentQuestionOrderRule::Shuffled => "shuffled" }] }
#[rustfmt::skip]
fn feedback_rule_values(r: &StudentFeedbackReleaseRule) -> [&'static str; 6] { [feedback_value(r.score), feedback_value(r.per_item_correctness), feedback_value(r.submitted_response), feedback_value(r.question_answer), feedback_value(r.question_answer_explanation), feedback_value(r.class_statistics)] }
#[rustfmt::skip]
fn feedback_value(v: StudentFeedbackReleaseTiming) -> &'static str { match v { StudentFeedbackReleaseTiming::DuringAttempt => "during_attempt", StudentFeedbackReleaseTiming::AfterSubmit => "after_submit", StudentFeedbackReleaseTiming::AfterDue => "after_due", StudentFeedbackReleaseTiming::AfterClose => "after_close", StudentFeedbackReleaseTiming::Never => "never" } }

#[rustfmt::skip]
pub(super) fn assessment_values_json(input: &SaveLiveAssessmentInput) -> Result<Value, StoreError> {
    let activity = activity_rule_values(&input.activity_rules);
    let feedback = feedback_rule_values(&input.student_feedback_release_rule);
    Ok(json!({
        "assessment_title": input.title.as_str(), "assessment_instructions": input.instructions.as_str(),
        "available_at": Value::Null, "due_at": Value::Null, "closes_at": Value::Null,
        "assessment_attempt_time_limit_seconds": input.assessment_attempt_time_limit_seconds.map(|value| value.get()), "assessment_attempt_limit": input.attempt_limit.map(|value| value.get()), "late_work_rule": late_work_rule(&input.late_work_rule),
        "question_variation_rule": activity[0], "assessment_question_order_rule": activity[1],
        "feedback_score": feedback[0], "feedback_per_item_correctness": feedback[1], "feedback_submitted_response": feedback[2], "feedback_question_answer": feedback[3], "feedback_question_answer_explanation": feedback[4], "feedback_class_statistics": feedback[5]
    }))
}

/// Encodes the closed Base Assessment Policy allowlist; title and Entries have no representation.
pub(super) fn base_assessment_policy_values_json(input: &SaveBaseAssessmentPolicyInput) -> Value {
    let activity = activity_rule_values(&input.activity_rules);
    let feedback = feedback_rule_values(&input.student_feedback_release_rule);
    json!({
        "assessment_instructions": input.instructions.as_str(), "available_at": Value::Null, "due_at": Value::Null, "closes_at": Value::Null,
        "assessment_attempt_time_limit_seconds": input.assessment_attempt_time_limit_seconds.map(|value| value.get()), "assessment_attempt_limit": input.attempt_limit.map(|value| value.get()), "late_work_rule": late_work_rule(&input.late_work_rule),
        "question_variation_rule": activity[0], "assessment_question_order_rule": activity[1],
        "feedback_score": feedback[0], "feedback_per_item_correctness": feedback[1], "feedback_submitted_response": feedback[2], "feedback_question_answer": feedback[3], "feedback_question_answer_explanation": feedback[4], "feedback_class_statistics": feedback[5]
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
        AssessmentEntry::FixedQuestion(value) => { let (seconds, grace_seconds) = question_time_limit(&value.question_attempt_time_limit)?; Ok(json!({ "assessmentEntryId": value.id.to_string(), "authoredPosition": position, "kind": "fixed_question", "availability": entry_availability(value.availability), "scoringRule": scoring_rule(value.scoring_rule), "questionId": value.question_revision.question_id.as_str(), "revisionNumber": value.question_revision.revision_number.get(), "pointsPossible": value.points_possible.to_string(), "questionAttemptLimit": value.question_attempt_limit.max_attempts, "questionAttemptTimeLimitSeconds": seconds, "questionAttemptGraceSeconds": grace_seconds })) }
        AssessmentEntry::QuestionPool(value) => { let (seconds, grace_seconds) = question_time_limit(&value.question_attempt_time_limit)?; Ok(json!({ "assessmentEntryId": value.id.to_string(), "authoredPosition": position, "kind": "question_pool", "availability": entry_availability(value.availability), "scoringRule": scoring_rule(value.scoring_rule), "questionPoolId": value.question_pool_id.as_str(), "questionPoolEditNumber": value.question_pool_edit_number.get(), "selectionCount": value.selection_count.get(), "pointsPerItem": value.points_per_item.to_string(), "selectedQuestionOrder": selected_question_order(value.selection_rule.selected_question_order), "questionAttemptLimit": value.question_attempt_limit.max_attempts, "questionAttemptTimeLimitSeconds": seconds, "questionAttemptGraceSeconds": grace_seconds })) }
    }
}
#[rustfmt::skip]
fn question_time_limit(v: &QuestionAttemptTimeLimit) -> Result<(Option<u32>, Option<u32>), StoreError> { Ok(match v { QuestionAttemptTimeLimit::Unlimited => (None, None), QuestionAttemptTimeLimit::Limited { seconds, grace_seconds } => (Some(*seconds), Some(*grace_seconds)) }) }
#[rustfmt::skip]
fn entry_availability(v: AssessmentEntryAvailability) -> &'static str { match v { AssessmentEntryAvailability::Available => "available", AssessmentEntryAvailability::Retired => "retired" } }
#[rustfmt::skip]
fn scoring_rule(v: AssessmentEntryScoringRule) -> &'static str { match v { AssessmentEntryScoringRule::Normal => "normal", AssessmentEntryScoringRule::FullCredit => "full_credit", AssessmentEntryScoringRule::ExtraCredit => "extra_credit", AssessmentEntryScoringRule::Excluded => "excluded" } }
#[rustfmt::skip]
fn selected_question_order(v: question_model::QuestionPoolSelectedQuestionOrder) -> &'static str { match v { question_model::QuestionPoolSelectedQuestionOrder::QuestionPoolOrder => "question_pool_order", question_model::QuestionPoolSelectedQuestionOrder::RandomOrder => "random_order" } }

#[cfg(test)]
mod tests {
    use super::assessment_entries_json;
    use question_model::{AssessmentEntry, QuestionId};

    #[test]
    fn mixed_exact_pins_and_pool_policies_encode_without_loss() {
        let question_id = QuestionId::from_random_identifier("ABCDEFG")
            .expect("test Question random identity is canonical");
        let question_pool_id = QuestionId::from_random_identifier("1234567")
            .expect("test Question Pool random identity is canonical");
        let entries: Vec<AssessmentEntry> = serde_json::from_value(serde_json::json!([
            {"kind":"fixedQuestion","id":"00000000-0000-0000-0000-000000000001","questionRevision":{"questionId":question_id,"revisionNumber":2},"pointsPossible":"3.5","availability":"available","scoringRule":"normal","questionAttemptLimit":{"maxAttempts":2},"questionAttemptTimeLimit":{"kind":"limited","seconds":90,"graceSeconds":5}},
            {"kind":"questionPool","id":"00000000-0000-0000-0000-000000000002","questionPoolId":question_pool_id,"questionPoolEditNumber":1,"availability":"retired","scoringRule":"extraCredit","selectionCount":1,"pointsPerItem":"2","selectionRule":{"selectedQuestionOrder":"randomOrder"},"questionAttemptLimit":{"maxAttempts":null},"questionAttemptTimeLimit":{"kind":"unlimited"}}
        ])).expect("mixed exact-pinned entries deserialize");
        let value = assessment_entries_json(&entries).expect("entries encode for PostgreSQL");
        assert_eq!(
            value[0]["assessmentEntryId"],
            "00000000-0000-0000-0000-000000000001"
        );
        assert_eq!(value[0]["revisionNumber"], 2);
        assert_eq!(value[0]["questionId"], question_id.as_str());
        assert_eq!(value[0]["questionAttemptGraceSeconds"], 5);
        assert_eq!(value[1]["selectedQuestionOrder"], "random_order");
        assert_eq!(value[1]["questionPoolId"], question_pool_id.as_str());
        assert_eq!(value[1]["questionPoolEditNumber"], 1);
    }
}
