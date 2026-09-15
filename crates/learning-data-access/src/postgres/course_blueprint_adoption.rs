//! Materialize all immutable Blueprint members with fresh teaching identities.

use question_model::{
    AssessmentEditNumber, AssessmentEntry, AssessmentEntryAvailability, AssessmentEntryId,
    AssessmentTitle, FixedQuestionAssessmentEntry, QuestionAttemptTimeLimit,
    QuestionPoolSelectedQuestionOrder,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction, types::Json};
use uuid::Uuid;

use super::assessment_workspace_save::{assessment_entries_json, assessment_values_json};
use super::connection::map_sqlx_error;
use crate::blueprint_course::{StoredBlueprintAssessmentContent, StoredBlueprintAssessmentEntry};
use crate::{
    CourseInstanceCreationSource, CourseInstancePoolIdIssuer, CreateCourseInstanceInput,
    SaveLiveAssessmentInput, StoreError, StoredBlueprintCourseContent,
};

pub(super) async fn creation_assessments(
    transaction: &mut Transaction<'_, Postgres>,
    input: &CreateCourseInstanceInput,
    pool_id_issuer: Option<&dyn CourseInstancePoolIdIssuer>,
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
    materialize(&content, pool_id_issuer)
}

fn materialize(
    content: &StoredBlueprintCourseContent,
    pool_id_issuer: Option<&dyn CourseInstancePoolIdIssuer>,
) -> Result<Value, StoreError> {
    let mut assessments = Vec::new();
    for module in &content.modules {
        for assessment in &module.assessments {
            let input = assessment_input(&assessment.content)?;
            let mut entries = Vec::with_capacity(assessment.content.entries.len());
            // ASVS 2.2.1 and 2.2.3: serialize each closed, typed source
            // variant at its exact Blueprint position.  The database compares
            // this complete ordered projection to the sealed Revision again.
            for (position, entry) in assessment.content.entries.iter().enumerate() {
                entries.push(match entry {
                    StoredBlueprintAssessmentEntry::Fixed { .. } => {
                        fixed_entry_json(position, entry)?
                    }
                    StoredBlueprintAssessmentEntry::Pool {
                        question_pool_revision,
                        selection_count,
                        points_per_item,
                        scoring_rule,
                        selection_rule,
                        question_attempt_limit,
                        question_attempt_time_limit,
                    } => {
                        let issuer = pool_id_issuer.ok_or_else(|| {
                            StoreError::Unavailable(
                                "Question Pool fork identity issuer is unavailable".to_string(),
                            )
                        })?;
                        pool_entry_json(
                            position,
                            question_pool_revision,
                            *selection_count,
                            points_per_item,
                            *scoring_rule,
                            selection_rule.selected_question_order,
                            *question_attempt_limit,
                            *question_attempt_time_limit,
                            issuer.issue_question_pool_id()?,
                        )?
                    }
                });
            }
            assessments.push(json!({
                "source": assessment.blueprint_assessment_reference,
                "values": assessment_values_json(&input)?,
                "entries": entries,
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
        entries: Vec::new(),
    })
}

fn fixed_entry_json(
    position: usize,
    entry: &StoredBlueprintAssessmentEntry,
) -> Result<Value, StoreError> {
    let encoded = assessment_entries_json(&[instantiate_fixed_entry(entry)?])?;
    let Value::Array(mut encoded) = encoded else {
        return Err(invalid("Assessment Entries"));
    };
    let mut encoded = encoded.pop().ok_or_else(|| invalid("Assessment Entries"))?;
    encoded["authoredPosition"] =
        json!(i32::try_from(position).map_err(|_| invalid("Assessment Entry position"))?);
    Ok(encoded)
}

fn instantiate_fixed_entry(
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
        StoredBlueprintAssessmentEntry::Pool { .. } => unreachable!("pool entries use fork import"),
    })
}

#[allow(clippy::too_many_arguments)]
fn pool_entry_json(
    position: usize,
    source: &question_model::QuestionPoolRevisionReference,
    selection_count: std::num::NonZeroU32,
    points_per_item: &question_model::AssessmentPointValue,
    scoring_rule: question_model::AssessmentEntryScoringRule,
    selected_question_order: QuestionPoolSelectedQuestionOrder,
    question_attempt_limit: question_model::QuestionAttemptLimit,
    question_attempt_time_limit: QuestionAttemptTimeLimit,
    fork_public_question_pool_id: question_model::QuestionId,
) -> Result<Value, StoreError> {
    let position = i32::try_from(position).map_err(|_| invalid("Assessment Entry position"))?;
    let (seconds, grace_seconds) = pool_time_limit(question_attempt_time_limit);
    Ok(json!({
        "assessmentEntryId": AssessmentEntryId::from_uuid(random_uuid()?).to_string(),
        "authoredPosition": position,
        "kind": "question_pool",
        "availability": "available",
        "sourceQuestionPoolId": source.question_pool_id.as_compact_str(),
        "sourceQuestionPoolRevisionNumber": source.revision_number.get(),
        "forkQuestionPoolId": random_uuid()?.to_string(),
        "forkPublicQuestionPoolId": fork_public_question_pool_id.as_compact_str(),
        "selectionCount": selection_count.get(),
        "pointsPerItem": points_per_item.to_string(),
        "scoringRule": pool_scoring_rule(scoring_rule),
        "selectedQuestionOrder": pool_selected_question_order(selected_question_order),
        "questionAttemptLimit": question_attempt_limit.max_attempts,
        "questionAttemptTimeLimitSeconds": seconds,
        "questionAttemptGraceSeconds": grace_seconds,
    }))
}

fn pool_time_limit(value: QuestionAttemptTimeLimit) -> (Option<u32>, Option<u32>) {
    match value {
        QuestionAttemptTimeLimit::Unlimited => (None, None),
        QuestionAttemptTimeLimit::Limited {
            seconds,
            grace_seconds,
        } => (Some(seconds), Some(grace_seconds)),
    }
}

fn pool_scoring_rule(value: question_model::AssessmentEntryScoringRule) -> &'static str {
    match value {
        question_model::AssessmentEntryScoringRule::Normal => "normal",
        question_model::AssessmentEntryScoringRule::FullCredit => "full_credit",
        question_model::AssessmentEntryScoringRule::ExtraCredit => "extra_credit",
        question_model::AssessmentEntryScoringRule::Excluded => "excluded",
    }
}

fn pool_selected_question_order(value: QuestionPoolSelectedQuestionOrder) -> &'static str {
    match value {
        QuestionPoolSelectedQuestionOrder::QuestionPoolOrder => "question_pool_order",
        QuestionPoolSelectedQuestionOrder::RandomOrder => "random_order",
    }
}

fn random_uuid() -> Result<Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Assessment UUID randomness unavailable".to_string())
    })
}

fn invalid(field: &str) -> StoreError {
    StoreError::InvalidRecord(format!("invalid {field}"))
}
