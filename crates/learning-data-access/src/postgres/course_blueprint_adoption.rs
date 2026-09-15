//! Materialize all immutable Blueprint members with fresh teaching identities.

use question_model::{
    AssessmentEditNumber, AssessmentEntry, AssessmentEntryAvailability, AssessmentEntryId,
    AssessmentTitle, FixedQuestionAssessmentEntry,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction, types::Json};
use uuid::Uuid;

use super::assessment_workspace_save::{assessment_entries_json, assessment_values_json};
use super::connection::map_sqlx_error;
use crate::blueprint_course::{StoredBlueprintAssessmentContent, StoredBlueprintAssessmentEntry};
use crate::{
    CourseInstanceCreationSource, CreateCourseInstanceInput, SaveLiveAssessmentInput, StoreError,
    StoredBlueprintCourseContent,
};

pub(super) async fn creation_assessments(
    transaction: &mut Transaction<'_, Postgres>,
    input: &CreateCourseInstanceInput,
) -> Result<Value, StoreError> {
    let (blueprint_course, blueprint_revision) = match &input.source {
        CourseInstanceCreationSource::Empty => return Ok(Value::Array(Vec::new())),
        CourseInstanceCreationSource::Adopted {
            blueprint_course,
            blueprint_revision,
        } => (blueprint_course, blueprint_revision),
    };
    let row = sqlx::query("SELECT * FROM ple_api.load_course_instance_blueprint($1, $2)")
        .bind(blueprint_course.as_string())
        .bind(i64::try_from(blueprint_revision.value()).map_err(|_| invalid("Blueprint Revision"))?)
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
    let mut assessments = Vec::new();
    for module in &content.modules {
        for assessment in &module.assessments {
            let input = assessment_input(&assessment.content)?;
            assessments.push(json!({
                "source": assessment.blueprint_assessment_reference,
                "values": assessment_values_json(&input)?,
                "entries": assessment_entries_json(&input.entries)?,
            }));
        }
    }
    Ok(Value::Array(assessments))
}

fn assessment_input(
    content: &StoredBlueprintAssessmentContent,
) -> Result<SaveLiveAssessmentInput, StoreError> {
    Ok(SaveLiveAssessmentInput {
        expected_edit_number: AssessmentEditNumber::INITIAL,
        title: AssessmentTitle::try_new(content.title.clone())
            .map_err(|_| invalid("Assessment Title"))?,
        instructions: content.instructions.clone(),
        due_at: None,
        available_at: None,
        closes_at: None,
        late_work_rule: content.defaults.late_work_rule,
        assessment_attempt_time_limit_seconds: content
            .defaults
            .assessment_attempt_time_limit_seconds,
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
    entry: &StoredBlueprintAssessmentEntry,
) -> Result<AssessmentEntry, StoreError> {
    let id = AssessmentEntryId::from_uuid(random_uuid()?);
    let availability = AssessmentEntryAvailability::Available;
    Ok(match entry {
        StoredBlueprintAssessmentEntry::Fixed {
            question_revision,
            points_possible,
            scoring_rule,
            question_attempt_limit,
            question_attempt_time_limit,
        } => AssessmentEntry::FixedQuestion(FixedQuestionAssessmentEntry {
            id,
            availability,
            reference: question_revision.clone(),
            points_possible: *points_possible,
            scoring_rule: *scoring_rule,
            question_attempt_limit: *question_attempt_limit,
            question_attempt_time_limit: *question_attempt_time_limit,
        }),
        // Blueprint content carries historic Question pins, not a published
        // Pool lineage/revision.  Converting those pins into an Assessment
        // Pool here would fabricate a mutable local Pool substitute.  The
        // adoption producer must instead supply a source Pool Revision to the
        // dedicated atomic Assessment Pool fork-import command.
        StoredBlueprintAssessmentEntry::Pool { .. } => {
            return Err(StoreError::InvalidRecord(
                "Blueprint Pool adoption requires an immutable source Pool Revision".to_owned(),
            ));
        }
    })
}

fn random_uuid() -> Result<Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Assessment UUID randomness unavailable".to_string())
    })
}

fn invalid(field: &str) -> StoreError {
    StoreError::InvalidRecord(format!("invalid {field}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint_course::{StoredBlueprintAssessment, StoredBlueprintModule};
    use question_model::{
        AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
        AssessmentPointValue, BlueprintAssessmentDefaults, BlueprintAssessmentReference,
        BlueprintModuleReference, LateWorkRule, QuestionAttemptLimit, QuestionAttemptTimeLimit,
        QuestionPoolSelectedQuestionOrder, QuestionPoolSelectionRule, QuestionRevisionReference,
        StudentFeedbackReleaseRule,
    };

    fn fixture() -> StoredBlueprintCourseContent {
        let pins: Vec<QuestionRevisionReference> = [1, 2]
            .into_iter()
            .map(|revision| QuestionRevisionReference {
                question_id: "ABCDXE12".parse().unwrap(),
                revision_number: question_model::QuestionRevisionNumber::new(revision).unwrap(),
            })
            .collect();
        let content = StoredBlueprintAssessmentContent {
            title: "Quiz".into(),
            instructions: AssessmentInstructions::try_new("Read first.".into()).unwrap(),
            defaults: BlueprintAssessmentDefaults {
                assessment_attempt_time_limit_seconds: std::num::NonZeroU32::new(1800),
                attempt_limit: std::num::NonZeroU32::new(3),
                late_work_rule: LateWorkRule::Reject,
                activity_rules: AssessmentActivityRules::default(),
                student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            },
            entries: vec![
                StoredBlueprintAssessmentEntry::Fixed {
                    question_revision: pins[0].clone(),
                    points_possible: AssessmentPointValue::from_whole(4),
                    scoring_rule: AssessmentEntryScoringRule::ExtraCredit,
                    question_attempt_limit: QuestionAttemptLimit {
                        max_attempts: Some(2),
                    },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Limited {
                        seconds: 60,
                        grace_seconds: 5,
                    },
                },
                StoredBlueprintAssessmentEntry::Pool {
                    question_revisions: pins,
                    selection_count: 1,
                    points_per_item: AssessmentPointValue::from_whole(2),
                    scoring_rule: AssessmentEntryScoringRule::Normal,
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
                assessments: [2, 3]
                    .into_iter()
                    .map(|id| StoredBlueprintAssessment {
                        blueprint_assessment_reference: BlueprintAssessmentReference::from_uuid(
                            Uuid::from_u128(id),
                        ),
                        content: content.clone(),
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn adoption_requires_a_real_immutable_pool_producer() {
        assert!(matches!(
            materialize(&fixture()),
            Err(StoreError::InvalidRecord(message))
                if message == "Blueprint Pool adoption requires an immutable source Pool Revision"
        ));
    }
}
