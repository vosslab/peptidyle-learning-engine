//! Derived source review and one retained Assessment's explicit reusable-content update.

use question_model::{AssessmentId, BlueprintCourseId, BlueprintRevision, CourseInstanceId};
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
    revision: BlueprintRevision,
    member: Option<StoredBlueprintAssessment>,
    cannot_apply_reason: Option<AssessmentBlueprintUpdateCannotApplyReason>,
}

pub(super) async fn review_course(
    store: &PostgresLiveAssessmentStore,
    token: SessionTokenHash,
    course: CourseInstanceId,
) -> Result<CourseBlueprintUpdateReview, StoreError> {
    let mut tx = store.begin(token).await?;
    // ASVS 8.3.1, 15.4.2-15.4.3: the authorized reader locks parent -> Course
    // -> adopted Assessments, holding one Revision and membership through both reads.
    let source = sqlx::query("SELECT * FROM ple_api.load_course_blueprint_update($1, NULL)")
        .bind(course.as_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(update_error)?
        .ok_or(StoreError::NotFound)?;
    let blueprint_reference: String = source
        .try_get("blueprint_reference")
        .map_err(map_sqlx_error)?;
    let blueprint_reference = blueprint_reference
        .parse::<BlueprintCourseId>()
        .map_err(|_| invalid("Blueprint Course Reference"))?;
    let revision = |field: &str| -> Result<BlueprintRevision, StoreError> {
        let value: i64 = source.try_get(field).map_err(map_sqlx_error)?;
        u64::try_from(value)
            .ok()
            .and_then(BlueprintRevision::new)
            .ok_or_else(|| invalid("Blueprint Revision"))
    };
    let adopted_revision = revision("adopted_revision")?;
    let source_revision = revision("source_revision")?;
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
            .bind(course.as_string())
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
        blueprint_reference,
        adopted_revision,
        source_revision,
        assessments,
    })
}

pub(super) async fn review(
    store: &PostgresLiveAssessmentStore,
    token: SessionTokenHash,
    course: CourseInstanceId,
    assessment: AssessmentId,
) -> Result<AssessmentBlueprintUpdateReview, StoreError> {
    let mut tx = store.begin(token).await?;
    let source = load_source(&mut tx, &course, &assessment).await?;
    let assessment = load_workspace(&mut tx, &course, &assessment).await?;
    let proposed = source.member.as_ref().map(public_content);
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(AssessmentBlueprintUpdateReview {
        assessment,
        source_revision: source.revision,
        proposed,
        cannot_apply_reason: source.cannot_apply_reason,
    })
}

pub(super) async fn apply(
    store: &PostgresLiveAssessmentStore,
    token: SessionTokenHash,
    course: CourseInstanceId,
    assessment: AssessmentId,
    input: ApplyAssessmentBlueprintUpdateInput,
    mut bloom_receipts: crate::PoolBloomPreparationReceipts,
) -> Result<LiveAssessmentWorkspace, StoreError> {
    let mut tx = store.begin(token).await?;
    // ASVS 8.3.1, 15.4.2: the procedure reauthorizes and holds parent,
    // Course and Assessment locks before either qualified precondition is tested.
    let source = load_source(&mut tx, &course, &assessment).await?;
    let workspace = load_workspace(&mut tx, &course, &assessment).await?;
    if source.revision != input.expected_source_revision
        || workspace.edit_number != input.expected_edit_number
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
            .bind(course.as_string())
            .bind(assessment.as_string())
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
        .bind(course.as_string())
        .bind(assessment.as_string())
        .bind(integer(
            input.expected_source_revision.value(),
            "Blueprint Revision",
        )?)
        .bind(integer(
            input.expected_edit_number.value(),
            "Assessment Edit Number",
        )?)
        .bind(materialized)
        .execute(&mut *tx)
        .await
        .map_err(update_error)?;
    let workspace = load_workspace(&mut tx, &course, &assessment).await?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(workspace)
}

async fn load_source(
    tx: &mut Transaction<'_, Postgres>,
    course: &CourseInstanceId,
    assessment: &AssessmentId,
) -> Result<UpdateSource, StoreError> {
    let row = sqlx::query("SELECT * FROM ple_api.load_assessment_blueprint_update($1, $2)")
        .bind(course.as_string())
        .bind(assessment.as_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(update_error)?
        .ok_or(StoreError::NotFound)?;
    let raw_revision: i64 = row.try_get("source_revision").map_err(map_sqlx_error)?;
    let revision = u64::try_from(raw_revision)
        .ok()
        .and_then(BlueprintRevision::new)
        .ok_or_else(|| invalid("Blueprint Revision"))?;
    let reference: Uuid = row
        .try_get("source_assessment_reference")
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
        .find(|member| member.blueprint_assessment_reference.as_uuid() == reference);
    if member.is_none()
        != (cannot_apply_reason
            == Some(AssessmentBlueprintUpdateCannotApplyReason::RetainedSourceMissing))
    {
        return Err(invalid("Blueprint retained Assessment"));
    }
    Ok(UpdateSource {
        revision,
        member,
        cannot_apply_reason,
    })
}

async fn load_workspace(
    tx: &mut Transaction<'_, Postgres>,
    course: &CourseInstanceId,
    assessment: &AssessmentId,
) -> Result<LiveAssessmentWorkspace, StoreError> {
    let context = schedule_context(tx, course).await?;
    let rows = workspace_rows(tx, course, assessment).await?;
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
                    question_revision,
                    points_possible,
                    scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => AssessmentBlueprintUpdateEntry::FixedQuestion {
                    reference: question_revision.clone(),
                    points_possible: *points_possible,
                    scoring_rule: *scoring_rule,
                    question_attempt_limit: *question_attempt_limit,
                    question_attempt_time_limit: *question_attempt_time_limit,
                },
                StoredBlueprintAssessmentEntry::Pool {
                    question_pool_revision,
                    selection_count,
                    points_per_item,
                    scoring_rule,
                    selection_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => AssessmentBlueprintUpdateEntry::QuestionPool {
                    question_pool_revision: question_pool_revision.clone(),
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
