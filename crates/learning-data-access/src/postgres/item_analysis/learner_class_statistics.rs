//! Learner-safe class-statistics read over the current item-analysis report.

use domain::entitlement::EntitlementDecision;
use domain::item_analysis::CourseItemAnalysisReport;
use question_model::{AssignmentId, CourseId, LearnerClassStatistics, ScoringGeneration, UserId};
use sqlx::Row;
use uuid::Uuid;

use super::super::{decode_payload_row_named, map_sqlx_error};
use super::{PostgresStore, REPORT_SCHEMA_VERSION, validate_report_identity};
use crate::{StoreError, TenantContext};

pub(super) async fn learner_class_statistics(
    store: &PostgresStore,
    context: TenantContext,
    learner: UserId,
    course: CourseId,
    assignment: AssignmentId,
) -> Result<LearnerClassStatistics, StoreError> {
    let tenant = context.tenant_id();
    // S5 takes row locks while it derives current membership and group facts,
    // so this must use the ordinary writable transaction helper rather than a
    // read-only snapshot transaction.
    let mut transaction = store.begin_tenant(context).await?;
    // S5 owns all current student membership and audience decisions. Do
    // not substitute historical enrollment evidence for this evaluation.
    if !matches!(
        super::super::entitlement::evaluate_current(
            &mut transaction,
            tenant,
            learner,
            course,
            assignment,
        )
        .await?,
        EntitlementDecision::Granted(_)
    ) {
        transaction.commit().await.map_err(map_sqlx_error)?;
        return Err(StoreError::NotFound);
    }

    let row = sqlx::query(
            "SELECT analysis.course_id AS analysis_course_id, \
                    analysis.assignment_id AS analysis_assignment_id, \
                    analysis.report_schema_version, analysis.report_payload, \
                    analysis.report_payload_sha256, analysis.source_scoring_generation, \
                    floor(extract(epoch FROM analysis.analyzed_at) * 1000)::bigint AS analyzed_at_millis, \
                    assignment.scoring_generation, assignment.scoring_status \
               FROM assignment \
               LEFT JOIN course_item_analysis_current AS analysis \
                 ON analysis.tenant_id = assignment.tenant_id \
                AND analysis.assignment_id = assignment.assignment_id \
              WHERE assignment.tenant_id = $1 AND assignment.assignment_id = $2 \
                AND assignment.course_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(course.as_uuid())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        transaction.commit().await.map_err(map_sqlx_error)?;
        return Ok(LearnerClassStatistics::InsufficientEvidence);
    };
    let Some(source_generation) = row
        .try_get::<Option<i64>, _>("source_scoring_generation")
        .map_err(map_sqlx_error)?
    else {
        transaction.commit().await.map_err(map_sqlx_error)?;
        return Ok(LearnerClassStatistics::InsufficientEvidence);
    };
    let source_generation = u64::try_from(source_generation)
        .ok()
        .and_then(ScoringGeneration::new)
        .ok_or_else(|| {
            StoreError::Unavailable(
                "stored item-analysis scoring generation is invalid".to_string(),
            )
        })?;
    let schema_version: i32 = row
        .try_get("report_schema_version")
        .map_err(map_sqlx_error)?;
    if schema_version != REPORT_SCHEMA_VERSION {
        return Err(StoreError::Unavailable(
            "stored item-analysis schema version is unsupported".to_string(),
        ));
    }
    let mut report: CourseItemAnalysisReport =
        decode_payload_row_named(&row, "report_payload", "report_payload_sha256")?;
    validate_report_identity(&report, tenant, course, assignment, source_generation)?;
    let analysis_course: Uuid = row.try_get("analysis_course_id").map_err(map_sqlx_error)?;
    let analysis_assignment: Uuid = row
        .try_get("analysis_assignment_id")
        .map_err(map_sqlx_error)?;
    let analyzed_at_millis: i64 = row.try_get("analyzed_at_millis").map_err(map_sqlx_error)?;
    if analysis_course != course.as_uuid()
        || analysis_assignment != assignment.as_uuid()
        || analyzed_at_millis != report.analyzed_at.as_unix_millis()
    {
        return Err(StoreError::Unavailable(
            "stored item-analysis identity is inconsistent".to_string(),
        ));
    }
    let scoring_generation: i64 = row.try_get("scoring_generation").map_err(map_sqlx_error)?;
    let scoring_status: String = row.try_get("scoring_status").map_err(map_sqlx_error)?;
    report.recent_rescoring = i64::try_from(source_generation.value()).ok()
        != Some(scoring_generation)
        || scoring_status != "current";
    let result = LearnerClassStatistics::from_current_analysis(
        report.completed_run_count,
        report.incomplete_manual_grading,
        report.recent_rescoring,
        report.assignment_average_score,
    );
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}
