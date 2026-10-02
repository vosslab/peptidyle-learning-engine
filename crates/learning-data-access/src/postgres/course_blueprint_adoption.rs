//! Materialize all immutable Blueprint members with fresh teaching identities.

use std::collections::BTreeSet;

use question_model::{
    AssessmentEditNumber, AssessmentEntryId, AssessmentTitle, QuestionAttemptTimeLimit,
    QuestionPoolSelectedQuestionOrder,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction, types::Json};
use uuid::Uuid;

use super::assessment_workspace_save::assessment_values_json;
use super::connection::map_sqlx_error;
use crate::blueprint_course::{
    StoredBlueprintAssessment, StoredBlueprintAssessmentContent, StoredBlueprintAssessmentEntry,
};
use crate::{
    CourseInstanceCreationSource, CourseInstancePoolIdIssuer, CreateCourseInstanceInput,
    SaveLiveAssessmentInput, StoreError, StoredBlueprintCourseContent,
};

pub(super) async fn creation_assessments(
    transaction: &mut Transaction<'_, Postgres>,
    input: &CreateCourseInstanceInput,
    pool_id_issuer: Option<&dyn CourseInstancePoolIdIssuer>,
    bloom_receipts: &mut crate::PoolBloomPreparationReceipts,
) -> Result<Value, StoreError> {
    let (blueprint_course, blueprint_revision_number) = match &input.source {
        CourseInstanceCreationSource::Empty => return Ok(Value::Array(Vec::new())),
        CourseInstanceCreationSource::Adopted {
            blueprint_revision_tuple,
        } => (
            &blueprint_revision_tuple.blueprint_course_id,
            &blueprint_revision_tuple.revision_number,
        ),
    };
    let row = sqlx::query("SELECT * FROM ple_api.load_course_instance_blueprint($1, $2)")
        .bind(blueprint_course.as_string())
        .bind(
            i64::try_from(blueprint_revision_number.value())
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
    materialize(&content, pool_id_issuer, bloom_receipts)
}

pub(super) fn materialize(
    content: &StoredBlueprintCourseContent,
    pool_id_issuer: Option<&dyn CourseInstancePoolIdIssuer>,
    bloom_receipts: &mut crate::PoolBloomPreparationReceipts,
) -> Result<Value, StoreError> {
    let mut assessments = Vec::new();
    for module in &content.modules {
        for assessment in &module.assessments {
            assessments.push(materialize_assessment(
                assessment,
                pool_id_issuer,
                bloom_receipts,
            )?);
        }
    }
    Ok(Value::Array(assessments))
}

/// Assessments on the saved Revision that the prior Revision does not already contain.
pub(super) fn newly_added_assessments(
    prior: &StoredBlueprintCourseContent,
    saved: &StoredBlueprintCourseContent,
) -> StoredBlueprintCourseContent {
    let prior_sources: BTreeSet<_> = prior
        .modules
        .iter()
        .flat_map(|module| &module.assessments)
        .map(|assessment| assessment.blueprint_assessment_id)
        .collect();
    let mut additions = saved.clone();
    for module in &mut additions.modules {
        module
            .assessments
            .retain(|assessment| !prior_sources.contains(&assessment.blueprint_assessment_id));
    }
    additions
}

/// Exact reusable projection, deliberately independent of fresh identity issuance.
pub(super) fn reusable_assessment_projection(
    assessment: &StoredBlueprintAssessment,
) -> Result<Value, StoreError> {
    let input = assessment_input(&assessment.content)?;
    let mut values = assessment_values_json(&input)?;
    values["assessment_type"] = json!(assessment.content.assessment_type);
    let mut entries = Vec::with_capacity(assessment.content.entries.len());
    // ASVS 2.2.1 and 2.2.3: serialize each closed, typed source
    // variant at its exact Blueprint position.  The database compares
    // this complete ordered projection to the sealed Revision again.
    for (position, entry) in assessment.content.entries.iter().enumerate() {
        entries.push(match entry {
            StoredBlueprintAssessmentEntry::Fixed { .. } => fixed_entry_json(position, entry)?,
            StoredBlueprintAssessmentEntry::Pool {
                question_pool_id,
                question_pool_edit_number,
                selection_count,
                points_per_item,
                scoring_rule,
                selection_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => pool_entry_json(
                position,
                question_pool_id,
                *question_pool_edit_number,
                *selection_count,
                points_per_item,
                *scoring_rule,
                selection_rule.selected_question_order,
                *question_attempt_limit,
                *question_attempt_time_limit,
            )?,
        });
    }
    Ok(json!({
        "source": assessment.blueprint_assessment_id,
        "values": values,
        "entries": entries,
    }))
}

/// Materializes one retained member with new teaching identities only after semantic comparison.
pub(super) fn materialize_assessment(
    assessment: &StoredBlueprintAssessment,
    pool_id_issuer: Option<&dyn CourseInstancePoolIdIssuer>,
    _bloom_receipts: &mut crate::PoolBloomPreparationReceipts,
) -> Result<Value, StoreError> {
    let mut member = reusable_assessment_projection(assessment)?;
    let entries = member["entries"]
        .as_array_mut()
        .ok_or_else(|| invalid("Assessment Entries"))?;
    for entry in entries {
        entry["assessmentEntryId"] =
            json!(AssessmentEntryId::from_uuid(random_uuid()?).to_string());
        if entry["kind"] == "question_pool" {
            let issuer = pool_id_issuer.ok_or_else(|| {
                StoreError::Unavailable(
                    "Question Pool fork identity issuer is unavailable".to_string(),
                )
            })?;
            entry["forkQuestionPoolId"] = json!(issuer.issue_question_pool_id()?.as_str());
        }
    }
    Ok(member)
}

fn assessment_input(
    content: &StoredBlueprintAssessmentContent,
) -> Result<SaveLiveAssessmentInput, StoreError> {
    Ok(SaveLiveAssessmentInput {
        expected_assessment_edit_number: AssessmentEditNumber::INITIAL,
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
    match entry {
        StoredBlueprintAssessmentEntry::Fixed {
            published_question_revision_tuple,
            points_possible,
            scoring_rule,
            question_attempt_limit,
            question_attempt_time_limit,
        } => {
            let (seconds, grace_seconds) = pool_time_limit(*question_attempt_time_limit);
            Ok(
                json!({"authoredPosition": i32::try_from(position).map_err(|_| invalid("Assessment Entry position"))?,
                "kind": "fixed_question", "availability": "available",
                "questionId": published_question_revision_tuple.published_question_id.as_str(),
                "revisionNumber": published_question_revision_tuple.revision_number.get(),
                "pointsPossible": points_possible.to_string(),
                "scoringRule": pool_scoring_rule(*scoring_rule),
                "questionAttemptLimit": question_attempt_limit.max_attempts,
                "questionAttemptTimeLimitSeconds": seconds, "questionAttemptGraceSeconds": grace_seconds}),
            )
        }
        StoredBlueprintAssessmentEntry::Pool { .. } => unreachable!("pool entries use fork import"),
    }
}

#[allow(clippy::too_many_arguments)]
fn pool_entry_json(
    position: usize,
    source_question_pool_id: &question_model::QuestionPoolId,
    question_pool_edit_number: question_model::QuestionPoolEditNumber,
    selection_count: std::num::NonZeroU32,
    points_per_item: &question_model::AssessmentPointValue,
    scoring_rule: question_model::AssessmentEntryScoringRule,
    selected_question_order: QuestionPoolSelectedQuestionOrder,
    question_attempt_limit: question_model::QuestionAttemptLimit,
    question_attempt_time_limit: QuestionAttemptTimeLimit,
) -> Result<Value, StoreError> {
    let position = i32::try_from(position).map_err(|_| invalid("Assessment Entry position"))?;
    let (seconds, grace_seconds) = pool_time_limit(question_attempt_time_limit);
    Ok(json!({
        "authoredPosition": position,
        "kind": "question_pool",
        "availability": "available",
        "sourceQuestionPoolId": source_question_pool_id.as_str(),
        "sourceQuestionPoolEditNumber": question_pool_edit_number.get(),
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

#[cfg(test)]
mod tests {
    use question_model::{
        AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
        AssessmentPointValue, BlueprintAssessmentDefaults, BlueprintAssessmentId,
        BlueprintModuleId, LateWorkRule, PublishedQuestionRevisionTuple, QuestionAttemptLimit,
        QuestionAttemptTimeLimit, StudentFeedbackReleaseRule,
    };
    use serde_json::json;
    use uuid::Uuid;

    use super::{materialize, newly_added_assessments};
    use crate::{
        PoolBloomPreparationReceipts, StoredBlueprintAssessment, StoredBlueprintAssessmentContent,
        StoredBlueprintAssessmentEntry, StoredBlueprintCourseContent, StoredBlueprintModule,
    };

    fn assessment(identity: u128, title: &str) -> StoredBlueprintAssessment {
        StoredBlueprintAssessment {
            blueprint_assessment_id: BlueprintAssessmentId::from_uuid(Uuid::from_u128(identity)),
            content: StoredBlueprintAssessmentContent {
                assessment_type: question_model::AssessmentType::RegularAssignment,
                title: title.to_string(),
                instructions: AssessmentInstructions::default(),
                entries: vec![StoredBlueprintAssessmentEntry::Fixed {
                    published_question_revision_tuple: PublishedQuestionRevisionTuple {
                        published_question_id: "7K3M-19QX".parse().expect("Question ID"),
                        revision_number: question_model::QuestionRevisionNumber::new(1)
                            .expect("revision"),
                    },
                    points_possible: AssessmentPointValue::from_whole(1),
                    scoring_rule: AssessmentEntryScoringRule::Normal,
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                }],
                defaults: BlueprintAssessmentDefaults {
                    assessment_attempt_time_limit_seconds: None,
                    attempt_limit: None,
                    late_work_rule: LateWorkRule::Accept,
                    activity_rules: AssessmentActivityRules::default(),
                    student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
                },
            },
        }
    }

    fn course(modules: Vec<StoredBlueprintModule>) -> StoredBlueprintCourseContent {
        StoredBlueprintCourseContent { modules }
    }

    fn module(
        identity: u128,
        assessments: Vec<StoredBlueprintAssessment>,
    ) -> StoredBlueprintModule {
        StoredBlueprintModule {
            blueprint_module_id: BlueprintModuleId::from_uuid(Uuid::from_u128(identity)),
            label: "Module".to_string(),
            assessments,
        }
    }

    #[test]
    fn daughter_creation_copies_every_blueprint_assessment() {
        let membrane = assessment(0xA1, "Membrane review");
        let groups = assessment(0xA2, "Functional groups");
        let content = course(vec![
            module(1, vec![membrane.clone()]),
            module(2, vec![groups.clone()]),
        ]);
        let payload = materialize(&content, None, &mut PoolBloomPreparationReceipts::default())
            .expect("daughter copy payload");
        let members = payload
            .as_array()
            .expect("one member per Blueprint Assessment");
        assert_eq!(members.len(), 2);
        assert_eq!(
            members[0]["source"],
            json!(membrane.blueprint_assessment_id.to_string())
        );
        assert_eq!(
            members[1]["source"],
            json!(groups.blueprint_assessment_id.to_string())
        );
        for member in members {
            assert!(member.get("assessment_status").is_none());
            assert!(member["values"]["available_at"].is_null());
            assert!(member["values"]["due_at"].is_null());
            assert!(member["values"]["closes_at"].is_null());
            assert_eq!(member["entries"][0]["kind"], "fixed_question");
            assert_eq!(member["entries"][0]["questionId"], "7K3M-19QX");
            assert_eq!(member["entries"][0]["revisionNumber"], 1);
        }
    }

    #[test]
    fn newly_added_blueprint_assessments_are_the_daughter_append() {
        let membrane = assessment(0xA1, "Membrane review");
        let groups = assessment(0xA2, "Functional groups");
        let prior = course(vec![module(1, vec![membrane.clone()])]);
        let saved = course(vec![module(1, vec![membrane, groups.clone()])]);
        let additions = newly_added_assessments(&prior, &saved);
        assert_eq!(additions.modules[0].assessments.len(), 1);
        assert_eq!(
            additions.modules[0].assessments[0].blueprint_assessment_id,
            groups.blueprint_assessment_id
        );
        let payload = materialize(
            &additions,
            None,
            &mut PoolBloomPreparationReceipts::default(),
        )
        .expect("newly added copy payload");
        let members = payload.as_array().expect("only the new Assessment");
        assert_eq!(members.len(), 1);
        assert_eq!(
            members[0]["source"],
            json!(groups.blueprint_assessment_id.to_string())
        );
        assert!(members[0].get("assessment_status").is_none());
        assert!(members[0]["values"]["available_at"].is_null());
        assert!(members[0]["values"]["due_at"].is_null());
        assert!(members[0]["values"]["closes_at"].is_null());
    }
}
