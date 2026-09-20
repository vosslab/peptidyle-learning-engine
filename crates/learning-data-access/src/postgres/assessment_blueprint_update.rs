//! Derived source review and one retained Assessment's explicit reusable-content update.

use question_model::{
    AssessmentId, BlueprintCourseId, BlueprintRevisionNumber, BlueprintRevisionTuple,
    CourseInstanceId,
};
use sqlx::{Postgres, Row, Transaction, types::Json};
use uuid::Uuid;

use super::{
    assessment_release::{decode_workspace, schedule_context, workspace_rows},
    assessment_workspace_connection::PostgresLiveAssessmentStore,
    connection::map_sqlx_error,
    course_blueprint_adoption::{materialize_assessment, reusable_assessment_projection},
};
use crate::{
    ApplyAssessmentBlueprintUpdateInput, AssessmentBlueprintUpdateCannotApplyReason,
    AssessmentBlueprintUpdateContent, AssessmentBlueprintUpdateEntry,
    AssessmentBlueprintUpdateReview, CourseAssessmentBlueprintUpdateSummary,
    CourseBlueprintUpdateReview, LiveAssessmentWorkspace, SessionTokenHash, StoreError,
    StoredBlueprintCourseContent,
    blueprint_course::{StoredBlueprintAssessment, StoredBlueprintAssessmentEntry},
};

struct UpdateSource {
    source_blueprint_revision_tuple: BlueprintRevisionTuple,
    member: Option<StoredBlueprintAssessment>,
    cannot_apply_reason: Option<AssessmentBlueprintUpdateCannotApplyReason>,
}

pub(super) async fn review_course(
    store: &PostgresLiveAssessmentStore,
    token: SessionTokenHash,
    course_instance_id: CourseInstanceId,
) -> Result<CourseBlueprintUpdateReview, StoreError> {
    let mut tx = store.begin(token).await?;
    // ASVS 8.3.1, 15.4.2-15.4.3: the authorized reader locks parent -> Course
    // -> adopted Assessments, holding one Revision and membership through both reads.
    let source = sqlx::query("SELECT * FROM ple_api.load_course_blueprint_update($1, NULL)")
        .bind(course_instance_id.as_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(update_error)?
        .ok_or(StoreError::NotFound)?;
    let blueprint_course_id: String = source
        .try_get("blueprint_course_id")
        .map_err(map_sqlx_error)?;
    let blueprint_course_id = blueprint_course_id
        .parse::<BlueprintCourseId>()
        .map_err(|_| invalid("Blueprint Course ID"))?;
    let parse_revision_number = |field: &str| -> Result<BlueprintRevisionNumber, StoreError> {
        let value: i64 = source.try_get(field).map_err(map_sqlx_error)?;
        u64::try_from(value)
            .ok()
            .and_then(BlueprintRevisionNumber::new)
            .ok_or_else(|| invalid("Blueprint Revision"))
    };
    let adopted_revision_number = parse_revision_number("adopted_revision_number")?;
    let source_revision_number = parse_revision_number("source_revision_number")?;
    let adopted_blueprint_revision_tuple = BlueprintRevisionTuple {
        blueprint_course_id: blueprint_course_id.clone(),
        revision_number: adopted_revision_number,
    };
    let current_blueprint_revision_tuple = BlueprintRevisionTuple {
        blueprint_course_id,
        revision_number: source_revision_number,
    };
    let Json(content): Json<StoredBlueprintCourseContent> =
        source.try_get("content").map_err(map_sqlx_error)?;
    let checksum: Vec<u8> = source.try_get("content_checksum").map_err(map_sqlx_error)?;
    if content.checksum()?.as_bytes() != checksum.as_slice() {
        return Err(invalid("Blueprint Content Checksum"));
    }
    let projections = content
        .modules
        .iter()
        .flat_map(|module| &module.assessments)
        .map(reusable_assessment_projection)
        .collect::<Result<Vec<_>, _>>()?;
    // ASVS 1.2.4, 15.3.1: batch semantic comparison reuses the Apply comparator;
    // it mints no Entry/Pool identities and returns no Student Work or answers.
    let Json(assessments): Json<Vec<CourseAssessmentBlueprintUpdateSummary>> =
        sqlx::query_scalar("SELECT assessments FROM ple_api.load_course_blueprint_update($1, $2)")
            .bind(course_instance_id.as_string())
            .bind(Json(projections))
            .fetch_optional(&mut *tx)
            .await
            .map_err(update_error)?
            .ok_or(StoreError::NotFound)?;
    if assessments
        .iter()
        .any(|row| row.matches_source && row.cannot_apply_reason.is_some())
    {
        return Err(invalid("Course Blueprint update summary"));
    }
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(CourseBlueprintUpdateReview {
        adopted_blueprint_revision_tuple,
        current_blueprint_revision_tuple,
        assessments,
    })
}

pub(super) async fn review(
    store: &PostgresLiveAssessmentStore,
    token: SessionTokenHash,
    course_instance_id: CourseInstanceId,
    assessment_id: AssessmentId,
) -> Result<AssessmentBlueprintUpdateReview, StoreError> {
    let mut tx = store.begin(token).await?;
    let source = load_source(&mut tx, &course_instance_id, &assessment_id).await?;
    let assessment = load_workspace(&mut tx, &course_instance_id, &assessment_id).await?;
    let proposed = source.member.as_ref().map(public_content);
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(AssessmentBlueprintUpdateReview {
        assessment,
        source_blueprint_revision_tuple: source.source_blueprint_revision_tuple,
        proposed,
        cannot_apply_reason: source.cannot_apply_reason,
    })
}

pub(super) async fn apply(
    store: &PostgresLiveAssessmentStore,
    token: SessionTokenHash,
    course_instance_id: CourseInstanceId,
    assessment_id: AssessmentId,
    input: ApplyAssessmentBlueprintUpdateInput,
    mut bloom_receipts: crate::PoolBloomPreparationReceipts,
) -> Result<LiveAssessmentWorkspace, StoreError> {
    let mut tx = store.begin(token).await?;
    // ASVS 8.3.1, 15.4.2: the procedure reauthorizes and holds parent,
    // Course and Assessment locks before either qualified precondition is tested.
    let source = load_source(&mut tx, &course_instance_id, &assessment_id).await?;
    let workspace = load_workspace(&mut tx, &course_instance_id, &assessment_id).await?;
    if source.source_blueprint_revision_tuple != input.expected_source_blueprint_revision_tuple
        || workspace.assessment_edit_number != input.expected_assessment_edit_number
    {
        return Err(StoreError::Conflict);
    }
    if source.cannot_apply_reason.is_some() {
        return Err(StoreError::LifecycleConflict);
    }
    let member = source.member.ok_or(StoreError::LifecycleConflict)?;
    let reusable = reusable_assessment_projection(&member)?;
    // Compare before minting identities: an equivalent update requires no Pool
    // issuer and leaves the current Edit Number and owned fork graph unchanged.
    let equivalent: bool =
        sqlx::query_scalar("SELECT ple_api.assessment_blueprint_update_equivalent($1, $2, $3, $4)")
            .bind(course_instance_id.as_string())
            .bind(assessment_id.as_string())
            .bind(reusable["values"].clone())
            .bind(reusable["entries"].clone())
            .fetch_one(&mut *tx)
            .await
            .map_err(update_error)?;
    let materialized = if equivalent {
        reusable
    } else {
        materialize_assessment(
            &member,
            store.pool_id_issuer.as_deref(),
            &mut bloom_receipts,
        )?
    };
    // ASVS 1.2.4, 2.3.3: parameterized exact-source projection; the database
    // validates it, preserves locked dates, establishes forks, and saves once.
    sqlx::query("SELECT ple_api.apply_assessment_blueprint_update($1, $2, $3, $4, $5)")
        .bind(course_instance_id.as_string())
        .bind(assessment_id.as_string())
        .bind(integer(
            input
                .expected_source_blueprint_revision_tuple
                .revision_number
                .value(),
            "Blueprint Revision Number",
        )?)
        .bind(integer(
            input.expected_assessment_edit_number.value(),
            "Assessment Edit Number",
        )?)
        .bind(materialized)
        .execute(&mut *tx)
        .await
        .map_err(update_error)?;
    let workspace = load_workspace(&mut tx, &course_instance_id, &assessment_id).await?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(workspace)
}

async fn load_source(
    tx: &mut Transaction<'_, Postgres>,
    course_instance_id: &CourseInstanceId,
    assessment_id: &AssessmentId,
) -> Result<UpdateSource, StoreError> {
    let row = sqlx::query("SELECT * FROM ple_api.load_assessment_blueprint_update($1, $2)")
        .bind(course_instance_id.as_string())
        .bind(assessment_id.as_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(update_error)?
        .ok_or(StoreError::NotFound)?;
    let raw_revision: i64 = row
        .try_get("source_revision_number")
        .map_err(map_sqlx_error)?;
    let revision_number = u64::try_from(raw_revision)
        .ok()
        .and_then(BlueprintRevisionNumber::new)
        .ok_or_else(|| invalid("Blueprint Revision Number"))?;
    let source_blueprint_course_id: String = row
        .try_get("source_blueprint_course_id")
        .map_err(map_sqlx_error)?;
    let source_blueprint_revision_tuple = BlueprintRevisionTuple {
        blueprint_course_id: source_blueprint_course_id
            .parse()
            .map_err(|_| invalid("Blueprint Course ID"))?,
        revision_number,
    };
    let source_assessment_id: Uuid = row
        .try_get("source_assessment_id")
        .map_err(map_sqlx_error)?;
    let Json(content): Json<StoredBlueprintCourseContent> =
        row.try_get("content").map_err(map_sqlx_error)?;
    let checksum: Vec<u8> = row.try_get("content_checksum").map_err(map_sqlx_error)?;
    if content.checksum()?.as_bytes() != checksum.as_slice() {
        return Err(invalid("Blueprint Content Checksum"));
    }
    let reason: Option<String> = row.try_get("cannot_apply_reason").map_err(map_sqlx_error)?;
    let cannot_apply_reason = match reason.as_deref() {
        None => None,
        Some("retained_source_missing") => {
            Some(AssessmentBlueprintUpdateCannotApplyReason::RetainedSourceMissing)
        }
        Some("assessment_type_mismatch") => {
            Some(AssessmentBlueprintUpdateCannotApplyReason::AssessmentTypeMismatch)
        }
        Some(_) => return Err(invalid("Blueprint update reason")),
    };
    let member = content
        .modules
        .into_iter()
        .flat_map(|module| module.assessments)
        .find(|member| member.blueprint_assessment_id.as_uuid() == source_assessment_id);
    if member.is_none()
        != (cannot_apply_reason
            == Some(AssessmentBlueprintUpdateCannotApplyReason::RetainedSourceMissing))
    {
        return Err(invalid("Blueprint retained Assessment"));
    }
    Ok(UpdateSource {
        source_blueprint_revision_tuple,
        member,
        cannot_apply_reason,
    })
}

async fn load_workspace(
    tx: &mut Transaction<'_, Postgres>,
    course_instance_id: &CourseInstanceId,
    assessment_id: &AssessmentId,
) -> Result<LiveAssessmentWorkspace, StoreError> {
    let context = schedule_context(tx, course_instance_id).await?;
    let rows = workspace_rows(tx, course_instance_id, assessment_id).await?;
    decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)
}

fn public_content(member: &StoredBlueprintAssessment) -> AssessmentBlueprintUpdateContent {
    let content = &member.content;
    AssessmentBlueprintUpdateContent {
        assessment_type: content.assessment_type,
        title: content.title.clone(),
        instructions: content.instructions.clone(),
        defaults: content.defaults.clone(),
        entries: content
            .entries
            .iter()
            .map(|entry| match entry {
                StoredBlueprintAssessmentEntry::Fixed {
                    question_revision_tuple,
                    points_possible,
                    scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => AssessmentBlueprintUpdateEntry::FixedQuestion {
                    question_revision_tuple: question_revision_tuple.clone(),
                    points_possible: *points_possible,
                    scoring_rule: *scoring_rule,
                    question_attempt_limit: *question_attempt_limit,
                    question_attempt_time_limit: *question_attempt_time_limit,
                },
                StoredBlueprintAssessmentEntry::Pool {
                    question_pool_id,
                    question_pool_edit_number,
                    selection_count,
                    points_per_item,
                    scoring_rule,
                    selection_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => AssessmentBlueprintUpdateEntry::QuestionPool {
                    question_pool_id: question_pool_id.clone(),
                    question_pool_edit_number: *question_pool_edit_number,
                    selection_count: *selection_count,
                    points_per_item: *points_per_item,
                    scoring_rule: *scoring_rule,
                    selection_rule: *selection_rule,
                    question_attempt_limit: *question_attempt_limit,
                    question_attempt_time_limit: *question_attempt_time_limit,
                },
            })
            .collect(),
    }
}

fn integer(value: u64, field: &str) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| invalid(field))
}

fn invalid(field: &str) -> StoreError {
    StoreError::InvalidRecord(format!("invalid {field}"))
}

fn update_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database) = &error {
        match database.code().as_deref() {
            Some("40001") => return StoreError::Conflict,
            Some("42501") => return StoreError::NotFound,
            _ => {}
        }
    }
    map_sqlx_error(error)
}
