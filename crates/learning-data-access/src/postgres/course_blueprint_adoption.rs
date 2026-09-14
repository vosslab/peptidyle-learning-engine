//! Materialize all immutable Blueprint members with fresh teaching identities.

use question_model::{
    AssignmentEditNumber, AssignmentEntry, AssignmentEntryAvailability, AssignmentEntryId,
    AssignmentTitle, FixedQuestionAssignmentEntry, QuestionPoolAssignmentEntry, QuestionPoolItem,
    QuestionPoolItemAvailability, QuestionPoolItemId,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction, types::Json};
use uuid::Uuid;

use super::assignment_workspace_save::{assignment_entries_json, assignment_values_json};
use super::connection::map_sqlx_error;
use crate::blueprint_course::{StoredBlueprintAssignmentContent, StoredBlueprintAssignmentEntry};
use crate::{
    CreateCourseInstanceInput, SaveLiveAssignmentInput, StoreError, StoredBlueprintCourseContent,
};

pub(super) async fn creation_assignments(
    transaction: &mut Transaction<'_, Postgres>,
    input: &CreateCourseInstanceInput,
) -> Result<Value, StoreError> {
    let row = sqlx::query("SELECT * FROM ple_api.load_course_instance_blueprint($1, $2)")
        .bind(i64::from(input.blueprint_course.number()))
        .bind(
            i64::try_from(input.blueprint_revision.value())
                .map_err(|_| invalid("Blueprint Revision"))?,
        )
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
    let Json(content): Json<StoredBlueprintCourseContent> =
        row.try_get("content").map_err(map_sqlx_error)?;
    let checksum: Vec<u8> = row.try_get("content_checksum").map_err(map_sqlx_error)?;
    if content.checksum()?.as_bytes() != checksum.as_slice() {
        return Err(invalid("Blueprint Content Checksum"));
    }
    materialize(&content)
}

fn materialize(content: &StoredBlueprintCourseContent) -> Result<Value, StoreError> {
    let mut assignments = Vec::new();
    for module in &content.modules {
        for assignment in &module.assignments {
            let input = assignment_input(&assignment.content)?;
            assignments.push(json!({
                "source": assignment.blueprint_assignment_reference,
                "values": assignment_values_json(&input)?,
                "entries": assignment_entries_json(&input.entries)?,
            }));
        }
    }
    Ok(Value::Array(assignments))
}

fn assignment_input(
    content: &StoredBlueprintAssignmentContent,
) -> Result<SaveLiveAssignmentInput, StoreError> {
    Ok(SaveLiveAssignmentInput {
        expected_edit_number: AssignmentEditNumber::INITIAL,
        title: AssignmentTitle::try_new(content.title.clone())
            .map_err(|_| invalid("Assignment Title"))?,
        instructions: content.instructions.clone(),
        due_at: None,
        available_at: None,
        closes_at: None,
        late_work_rule: content.defaults.late_work_rule,
        assignment_attempt_time_limit_seconds: content
            .defaults
            .assignment_attempt_time_limit_seconds,
        attempt_limit: content.defaults.attempt_limit,
        activity_rules: content.defaults.activity_rules,
        student_feedback_release_rule: content.defaults.student_feedback_release_rule,
        entries: content
            .entries
            .iter()
            .map(instantiate_entry)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn instantiate_entry(
    entry: &StoredBlueprintAssignmentEntry,
) -> Result<AssignmentEntry, StoreError> {
    let id = AssignmentEntryId::from_uuid(random_uuid()?);
    let availability = AssignmentEntryAvailability::Available;
    Ok(match entry {
        StoredBlueprintAssignmentEntry::Fixed {
            question_revision,
            points_possible,
            scoring_rule,
            question_attempt_limit,
            question_attempt_time_limit,
        } => AssignmentEntry::FixedQuestion(FixedQuestionAssignmentEntry {
            id,
            availability,
            reference: question_revision.clone(),
            points_possible: *points_possible,
            scoring_rule: *scoring_rule,
            question_attempt_limit: *question_attempt_limit,
            question_attempt_time_limit: *question_attempt_time_limit,
        }),
        StoredBlueprintAssignmentEntry::Pool {
            question_revisions,
            selection_count,
            points_per_item,
            scoring_rule,
            selection_rule,
            question_attempt_limit,
            question_attempt_time_limit,
        } => AssignmentEntry::QuestionPool(QuestionPoolAssignmentEntry {
            id,
            availability,
            selection_count: *selection_count,
            points_per_item: *points_per_item,
            scoring_rule: *scoring_rule,
            selection_rule: *selection_rule,
            question_attempt_limit: *question_attempt_limit,
            question_attempt_time_limit: *question_attempt_time_limit,
            items: question_revisions
                .iter()
                .map(|reference| {
                    Ok(QuestionPoolItem {
                        id: QuestionPoolItemId::from_uuid(random_uuid()?),
                        reference: reference.clone(),
                        availability: QuestionPoolItemAvailability::Available,
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?,
        }),
    })
}

fn random_uuid() -> Result<Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Assignment UUID randomness unavailable".to_string())
    })
}

fn invalid(field: &str) -> StoreError {
    StoreError::InvalidRecord(format!("invalid {field}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint_course::{StoredBlueprintAssignment, StoredBlueprintModule};
    use question_model::{
        AssignmentActivityRules, AssignmentEntryScoringRule, AssignmentInstructions,
        AssignmentPointValue, BlueprintAssignmentDefaults, BlueprintAssignmentReference,
        BlueprintModuleReference, LateWorkRule, QuestionAttemptLimit, QuestionAttemptTimeLimit,
        QuestionPoolSelectedQuestionOrder, QuestionPoolSelectionRule, QuestionRevisionReference,
        StudentFeedbackReleaseRule,
    };

    fn fixture() -> StoredBlueprintCourseContent {
        let pins: Vec<QuestionRevisionReference> = [1, 2]
            .into_iter()
            .map(|revision| QuestionRevisionReference {
                question_id: "ABCDE12".parse().unwrap(),
                revision_number: question_model::QuestionRevisionNumber::new(revision).unwrap(),
            })
            .collect();
        let content = StoredBlueprintAssignmentContent {
            title: "Quiz".into(),
            instructions: AssignmentInstructions::try_new("Read first.".into()).unwrap(),
            defaults: BlueprintAssignmentDefaults {
                assignment_attempt_time_limit_seconds: std::num::NonZeroU32::new(1800),
                attempt_limit: std::num::NonZeroU32::new(3),
                late_work_rule: LateWorkRule::Reject,
                activity_rules: AssignmentActivityRules::default(),
                student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            },
            entries: vec![
                StoredBlueprintAssignmentEntry::Fixed {
                    question_revision: pins[0].clone(),
                    points_possible: AssignmentPointValue::from_whole(4),
                    scoring_rule: AssignmentEntryScoringRule::ExtraCredit,
                    question_attempt_limit: QuestionAttemptLimit {
                        max_attempts: Some(2),
                    },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Limited {
                        seconds: 60,
                        grace_seconds: 5,
                    },
                },
                StoredBlueprintAssignmentEntry::Pool {
                    question_revisions: pins,
                    selection_count: 1,
                    points_per_item: AssignmentPointValue::from_whole(2),
                    scoring_rule: AssignmentEntryScoringRule::Normal,
                    selection_rule: QuestionPoolSelectionRule {
                        selected_question_order: QuestionPoolSelectedQuestionOrder::RandomOrder,
                    },
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                },
            ],
        };
        StoredBlueprintCourseContent {
            modules: vec![StoredBlueprintModule {
                blueprint_module_reference: BlueprintModuleReference::from_uuid(Uuid::from_u128(1)),
                label: "Topic".into(),
                assignments: [2, 3]
                    .into_iter()
                    .map(|id| StoredBlueprintAssignment {
                        blueprint_assignment_reference: BlueprintAssignmentReference::from_uuid(
                            Uuid::from_u128(id),
                        ),
                        content: content.clone(),
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn adoption_preserves_complete_content_and_pins_with_independent_identities() {
        let source = fixture();

        let first = materialize(&source).unwrap();
        let second = materialize(&source).unwrap();
        assert_eq!(first.as_array().unwrap().len(), 2);
        for (index, assignment) in first.as_array().unwrap().iter().enumerate() {
            assert_eq!(
                assignment["source"],
                json!(source.modules[0].assignments[index].blueprint_assignment_reference)
            );
            assert_eq!(assignment["values"]["assignment_title"], "Quiz");
            assert_eq!(
                assignment["values"]["assignment_instructions"],
                "Read first."
            );
            assert_eq!(assignment["values"]["attempt_limit"], 3);
            assert_eq!(
                assignment["values"]["assignment_attempt_time_limit_seconds"],
                1800
            );
            assert_eq!(assignment["values"]["late_work_rule"], "reject");
            assert!(assignment["values"]["due_at"].is_null());
            assert_eq!(assignment["entries"][0]["revisionNumber"], 1);
            assert_eq!(assignment["entries"][0]["scoringRule"], "extra_credit");
            assert_eq!(assignment["entries"][0]["pointsPossible"], "4");
            assert_eq!(
                assignment["entries"][0]["questionAttemptTimeLimitSeconds"],
                60
            );
            assert_eq!(assignment["entries"][1]["selectionCount"], 1);
            assert_eq!(
                assignment["entries"][1]["selectedQuestionOrder"],
                "random_order"
            );
            assert_eq!(assignment["entries"][1]["items"][1]["revisionNumber"], 2);
            assert_ne!(
                assignment["entries"][0]["assignmentEntryId"],
                second[index]["entries"][0]["assignmentEntryId"]
            );
            assert_ne!(
                assignment["entries"][1]["items"][0]["questionPoolItemId"],
                second[index]["entries"][1]["items"][0]["questionPoolItemId"]
            );
        }
    }
}
