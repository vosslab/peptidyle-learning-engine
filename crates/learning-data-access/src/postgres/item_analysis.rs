//! PostgreSQL current, course-local item-analysis projection.
//!
//! The database query deliberately selects only delivery metadata, lifecycle
//! state, timestamps, and current numeric credit.  Responses, learner
//! identities, and object references never enter the report or its staging
//! payload.

use std::collections::BTreeMap;

use async_trait::async_trait;
use domain::item_analysis::{
    AssignmentItemAnalysis, CourseItemAnalysisReport, ItemAnalysisMetricInput,
    calculate_item_analysis_metrics,
};
use question_model::{
    ActivityTimestamp, AssignmentId, AssignmentItemId, CourseId, ProblemId, ProblemVersionRef,
    ScoringGeneration, TenantId, VersionId,
};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{
    PostgresStore, database_timestamp, decode_payload_row_named, encode_payload, map_sqlx_error,
};
use crate::{
    CourseItemAnalysisCommitOutcome, CourseItemAnalysisStore, CourseItemAnalysisWorkerCommand,
    CourseItemAnalysisWorkerStore, JobPayload, SessionTokenHash, StoreError, TenantContext,
};

const REPORT_SCHEMA_VERSION: i32 = 1;
type DeliveredItemKey = (AssignmentItemId, ProblemId, VersionId);

#[derive(Debug)]
struct DeliveredItem {
    assignment_item: AssignmentItemId,
    reference: ProblemVersionRef,
    run: Uuid,
    completed: bool,
    completion_millis: Option<u64>,
    status: Option<String>,
    grading_status: Option<String>,
    credit: Option<Decimal>,
    correct: Option<bool>,
    earned_points: Option<Decimal>,
    possible_points: Option<Decimal>,
}

#[derive(Clone, Copy)]
struct AnalysisReportContext {
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    generation: ScoringGeneration,
    analyzed_at: ActivityTimestamp,
    completed_run_count: u32,
    in_progress_run_count: u32,
}

#[async_trait]
impl CourseItemAnalysisStore for PostgresStore {
    async fn course_item_analysis(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        assignment: AssignmentId,
    ) -> Result<Option<CourseItemAnalysisReport>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant_snapshot(context).await?;
        let authorized: bool = sqlx::query_scalar("SELECT ple_retention_authorize($1, $2, false)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !authorized {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
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
            return Ok(None);
        };
        let Some(source_generation) = row
            .try_get::<Option<i64>, _>("source_scoring_generation")
            .map_err(map_sqlx_error)?
        else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
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
        let analyzed_at_millis: i64 = row.try_get("analyzed_at_millis").map_err(map_sqlx_error)?;
        let analysis_course: Uuid = row.try_get("analysis_course_id").map_err(map_sqlx_error)?;
        let analysis_assignment: Uuid = row
            .try_get("analysis_assignment_id")
            .map_err(map_sqlx_error)?;
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
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(report))
    }
}

#[async_trait]
impl CourseItemAnalysisWorkerStore for PostgresStore {
    async fn prepare_course_item_analysis(
        &self,
        context: TenantContext,
        command: CourseItemAnalysisWorkerCommand,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let expected_payload = analysis_payload(command.assignment, command.generation)?;
        let mut transaction = self.begin_tenant(context).await?;
        if !analysis_claim_active(&mut transaction, tenant, command, expected_payload).await? {
            return Err(StoreError::Conflict);
        }
        let assignment =
            analysis_assignment_state(&mut transaction, tenant, command.assignment).await?;
        if assignment.generation != command.generation || assignment.status != "current" {
            sqlx::query(
                "DELETE FROM course_item_analysis_staging WHERE tenant_id = $1 AND job_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(());
        }
        sqlx::query(
            "DELETE FROM course_item_analysis_staging WHERE tenant_id = $1 AND job_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        let analyzed_at = database_timestamp(&mut transaction).await?;
        let report = build_course_item_analysis_report(
            &mut transaction,
            tenant,
            assignment.course,
            command.assignment,
            command.generation,
            analyzed_at,
        )
        .await?;
        let (payload, checksum) = encode_payload(&report)?;
        sqlx::query(
            "INSERT INTO course_item_analysis_staging \
             (tenant_id, job_id, course_id, assignment_id, source_scoring_generation, \
              report_schema_version, report_payload, report_payload_sha256) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .bind(assignment.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(i64::try_from(command.generation.value()).map_err(|_| StoreError::Conflict)?)
        .bind(REPORT_SCHEMA_VERSION)
        .bind(payload)
        .bind(checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn commit_course_item_analysis(
        &self,
        context: TenantContext,
        command: CourseItemAnalysisWorkerCommand,
    ) -> Result<CourseItemAnalysisCommitOutcome, StoreError> {
        let tenant = context.tenant_id();
        let expected_payload = analysis_payload(command.assignment, command.generation)?;
        let mut transaction = self.begin_tenant(context).await?;

        // The assignment row serializes generation publication. The exact leased
        // claim owns its private staging row, so this ordinary read keeps the
        // worker on the existing SELECT/DELETE least-privilege grant.
        let assignment =
            analysis_assignment_state_for_update(&mut transaction, tenant, command.assignment)
                .await?;
        let staging = sqlx::query(
            "SELECT course_id, assignment_id, source_scoring_generation, report_schema_version, \
                    report_payload, report_payload_sha256, \
                    floor(extract(epoch FROM prepared_at) * 1000)::bigint AS prepared_at_millis \
               FROM course_item_analysis_staging \
              WHERE tenant_id = $1 AND job_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let claim_active =
            analysis_claim_active(&mut transaction, tenant, command, expected_payload).await?;
        if !claim_active {
            transaction.rollback().await.map_err(map_sqlx_error)?;
            return Ok(CourseItemAnalysisCommitOutcome::ClaimNoLongerActive);
        }
        let generation =
            i64::try_from(command.generation.value()).map_err(|_| StoreError::Conflict)?;
        let current = assignment.generation == command.generation && assignment.status == "current";
        if !current {
            sqlx::query(
                "DELETE FROM course_item_analysis_staging WHERE tenant_id = $1 AND job_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            super::jobs::complete_postgres_claimed_job(
                &mut transaction,
                command.job,
                command.lease,
            )
            .await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(CourseItemAnalysisCommitOutcome::Superseded);
        }
        let Some(staging) = staging else {
            return Err(StoreError::Conflict);
        };
        let staging_course: Uuid = staging.try_get("course_id").map_err(map_sqlx_error)?;
        let staging_assignment: Uuid = staging.try_get("assignment_id").map_err(map_sqlx_error)?;
        let staging_generation: i64 = staging
            .try_get("source_scoring_generation")
            .map_err(map_sqlx_error)?;
        let staging_schema_version: i32 = staging
            .try_get("report_schema_version")
            .map_err(map_sqlx_error)?;
        let prepared_at_millis: i64 = staging
            .try_get("prepared_at_millis")
            .map_err(map_sqlx_error)?;
        let report: CourseItemAnalysisReport =
            decode_payload_row_named(&staging, "report_payload", "report_payload_sha256")?;
        validate_report_identity(
            &report,
            tenant,
            assignment.course,
            command.assignment,
            command.generation,
        )?;
        if staging_course != assignment.course.as_uuid()
            || staging_assignment != command.assignment.as_uuid()
            || staging_generation != generation
            || staging_schema_version != REPORT_SCHEMA_VERSION
            || prepared_at_millis != report.analyzed_at.as_unix_millis()
        {
            return Err(StoreError::Unavailable(
                "staged item-analysis identity is inconsistent".to_string(),
            ));
        }
        sqlx::query(
            "DELETE FROM course_item_analysis_current WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO course_item_analysis_current \
             (tenant_id, course_id, assignment_id, source_scoring_generation, \
              report_schema_version, report_payload, report_payload_sha256, analyzed_at) \
             SELECT tenant_id, course_id, assignment_id, source_scoring_generation, \
                    report_schema_version, report_payload, report_payload_sha256, prepared_at \
               FROM course_item_analysis_staging \
              WHERE tenant_id = $1 AND job_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "DELETE FROM course_item_analysis_staging WHERE tenant_id = $1 AND job_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        super::jobs::complete_postgres_claimed_job(&mut transaction, command.job, command.lease)
            .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseItemAnalysisCommitOutcome::Committed)
    }
}

#[derive(Clone, Copy)]
struct AnalysisAssignmentState {
    course: CourseId,
    generation: ScoringGeneration,
    status: &'static str,
}

async fn analysis_assignment_state(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AnalysisAssignmentState, StoreError> {
    let row = sqlx::query(
        "SELECT course_id, scoring_generation, scoring_status FROM assignment \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_analysis_assignment_state(&row)
}

async fn analysis_assignment_state_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AnalysisAssignmentState, StoreError> {
    let row = sqlx::query(
        "SELECT course_id, scoring_generation, scoring_status FROM assignment \
         WHERE tenant_id = $1 AND assignment_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_analysis_assignment_state(&row)
}

fn decode_analysis_assignment_state(row: &PgRow) -> Result<AnalysisAssignmentState, StoreError> {
    let generation = u64::try_from(
        row.try_get::<i64, _>("scoring_generation")
            .map_err(map_sqlx_error)?,
    )
    .ok()
    .and_then(ScoringGeneration::new)
    .ok_or_else(|| StoreError::Unavailable("stored scoring generation is invalid".to_string()))?;
    let status = match row
        .try_get::<String, _>("scoring_status")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "current" => "current",
        "recalculating" => "recalculating",
        "failed" => "failed",
        _ => {
            return Err(StoreError::Unavailable(
                "stored scoring status is invalid".to_string(),
            ));
        }
    };
    Ok(AnalysisAssignmentState {
        course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        generation,
        status,
    })
}

fn analysis_payload(
    assignment: AssignmentId,
    generation: ScoringGeneration,
) -> Result<Value, StoreError> {
    serde_json::to_value(JobPayload::RecalculateCourseItemAnalysis {
        assignment,
        generation,
    })
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}

async fn analysis_claim_active(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    command: CourseItemAnalysisWorkerCommand,
    expected_payload: Value,
) -> Result<bool, StoreError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM worker_job \
         WHERE job_id = $1 AND tenant_id = $2 AND state = 'leased' \
           AND lease_token = $3 AND lease_expires_at > transaction_timestamp() \
           AND payload = $4)",
    )
    .bind(command.job.as_uuid())
    .bind(tenant.as_uuid())
    .bind(command.lease.as_uuid())
    .bind(expected_payload)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
}

async fn build_course_item_analysis_report(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    generation: ScoringGeneration,
    analyzed_at: ActivityTimestamp,
) -> Result<CourseItemAnalysisReport, StoreError> {
    let rows = sqlx::query(
        "WITH latest_run AS ( \
             SELECT DISTINCT ON (run.enrollment_id) run.run_id, run.enrollment_id, run.started_at, run.completed_at \
               FROM assignment_run AS run \
               JOIN enrollment AS enrollment \
                 ON enrollment.tenant_id = run.tenant_id AND enrollment.enrollment_id = run.enrollment_id \
              WHERE run.tenant_id = $1 AND enrollment.assignment_id = $2 \
              ORDER BY run.enrollment_id, run.run_number DESC, run.run_id DESC \
         ), delivered AS ( \
             SELECT latest_run.run_id, latest_run.enrollment_id, latest_run.started_at, latest_run.completed_at, \
                    item.assignment_item_id, item.problem_id, item.version_id, item.issued_position \
               FROM latest_run \
               JOIN assignment_run_item AS item \
                 ON item.tenant_id = $1 AND item.run_id = latest_run.run_id \
         ), selected_attempt AS ( \
             SELECT delivered.*, attempt.attempt_id, attempt.attempt_status, attempt.submitted_at \
               FROM delivered \
               LEFT JOIN LATERAL ( \
                    SELECT candidate.attempt_id, candidate.attempt_status, candidate.submitted_at \
                      FROM question_attempt AS candidate \
                     WHERE candidate.tenant_id = $1 AND candidate.run_id = delivered.run_id \
                       AND candidate.assignment_position = delivered.issued_position \
                     ORDER BY candidate.occurred_at DESC, candidate.attempt_id DESC \
                     LIMIT 1 \
               ) AS attempt ON true \
         ), terminal_run AS ( \
             SELECT run_id, \
                    bool_and(attempt_status IN ('submitted', 'auto_submitted', 'needs_manual_grading', 'cleared', 'exempt')) AS terminal, \
                    max(submitted_at) AS terminal_submitted_at \
               FROM selected_attempt GROUP BY run_id \
         ) \
         SELECT selected_attempt.run_id, selected_attempt.enrollment_id, selected_attempt.assignment_item_id, \
                selected_attempt.problem_id, selected_attempt.version_id, \
                floor(extract(epoch FROM terminal_run.terminal_submitted_at) * 1000)::bigint AS completed_at_millis, \
                floor(extract(epoch FROM selected_attempt.started_at) * 1000)::bigint AS started_at_millis, \
                score.earned_points, score.possible_points, \
                selected_attempt.attempt_status, evaluation.grading_status, evaluation.credit_fraction, evaluation.correct \
           FROM selected_attempt \
           JOIN terminal_run ON terminal_run.run_id = selected_attempt.run_id AND terminal_run.terminal \
           LEFT JOIN submission_evaluation AS evaluation \
             ON evaluation.tenant_id = $1 AND evaluation.attempt_id = selected_attempt.attempt_id \
           LEFT JOIN attempt_score_current AS score \
             ON score.tenant_id = $1 AND score.attempt_id = selected_attempt.attempt_id \
            AND score.scoring_generation = $3 \
          ORDER BY selected_attempt.assignment_item_id, selected_attempt.run_id",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(i64::try_from(generation.value()).map_err(|_| StoreError::Conflict)?)
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    let latest_run_counts = sqlx::query(
        "WITH latest_run AS ( \
                SELECT DISTINCT ON (run.enrollment_id) run.run_id \
                  FROM assignment_run AS run \
                  JOIN enrollment AS enrollment \
                    ON enrollment.tenant_id = run.tenant_id AND enrollment.enrollment_id = run.enrollment_id \
                 WHERE run.tenant_id = $1 AND enrollment.assignment_id = $2 \
                 ORDER BY run.enrollment_id, run.run_number DESC, run.run_id DESC \
           ), selected_attempt AS ( \
                SELECT latest_run.run_id, item.issued_position, attempt.attempt_status \
                  FROM latest_run JOIN assignment_run_item AS item \
                    ON item.tenant_id = $1 AND item.run_id = latest_run.run_id \
                  LEFT JOIN LATERAL ( \
                    SELECT candidate.attempt_status FROM question_attempt AS candidate \
                     WHERE candidate.tenant_id = $1 AND candidate.run_id = latest_run.run_id \
                       AND candidate.assignment_position = item.issued_position \
                     ORDER BY candidate.occurred_at DESC, candidate.attempt_id DESC LIMIT 1 \
                  ) AS attempt ON true \
           ), terminal_run AS ( \
                SELECT latest_run.run_id, COALESCE(bool_and(selected_attempt.attempt_status IN \
                    ('submitted', 'auto_submitted', 'needs_manual_grading', 'cleared', 'exempt')), false) AS terminal \
                  FROM latest_run LEFT JOIN selected_attempt ON selected_attempt.run_id = latest_run.run_id \
                 GROUP BY latest_run.run_id \
           ) \
           SELECT count(*) FILTER (WHERE terminal) AS completed, \
                  count(*) FILTER (WHERE NOT terminal) AS in_progress FROM terminal_run",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let completed_run_count = checked_count(
        latest_run_counts
            .try_get("completed")
            .map_err(map_sqlx_error)?,
    )?;
    let in_progress_run_count = checked_count(
        latest_run_counts
            .try_get("in_progress")
            .map_err(map_sqlx_error)?,
    )?;

    let mut delivered = Vec::with_capacity(rows.len());
    for row in rows {
        let completed_at = row
            .try_get::<Option<i64>, _>("completed_at_millis")
            .map_err(map_sqlx_error)?;
        let started_at: i64 = row.try_get("started_at_millis").map_err(map_sqlx_error)?;
        let completion_millis = completed_at.and_then(|completed| {
            completed
                .checked_sub(started_at)
                .and_then(|elapsed| u64::try_from(elapsed).ok())
        });
        delivered.push(DeliveredItem {
            assignment_item: AssignmentItemId::from_uuid(
                row.try_get("assignment_item_id").map_err(map_sqlx_error)?,
            ),
            reference: ProblemVersionRef {
                problem: ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?),
                version: VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?),
            },
            run: row.try_get("run_id").map_err(map_sqlx_error)?,
            completed: completed_at.is_some(),
            completion_millis,
            status: row.try_get("attempt_status").map_err(map_sqlx_error)?,
            grading_status: row.try_get("grading_status").map_err(map_sqlx_error)?,
            credit: row.try_get("credit_fraction").map_err(map_sqlx_error)?,
            correct: row.try_get("correct").map_err(map_sqlx_error)?,
            earned_points: row.try_get("earned_points").map_err(map_sqlx_error)?,
            possible_points: row.try_get("possible_points").map_err(map_sqlx_error)?,
        });
    }
    course_report_from_deliveries(
        AnalysisReportContext {
            tenant,
            course,
            assignment,
            generation,
            analyzed_at,
            completed_run_count,
            in_progress_run_count,
        },
        delivered,
    )
}

fn course_report_from_deliveries(
    context: AnalysisReportContext,
    delivered: Vec<DeliveredItem>,
) -> Result<CourseItemAnalysisReport, StoreError> {
    let AnalysisReportContext {
        tenant,
        course,
        assignment,
        generation,
        analyzed_at,
        completed_run_count,
        in_progress_run_count,
        ..
    } = context;
    let mut references = BTreeMap::new();
    let mut inputs: BTreeMap<DeliveredItemKey, ItemAnalysisMetricInput> = BTreeMap::new();
    let mut graded = Vec::new();
    let mut completion_times = BTreeMap::new();
    let mut incomplete_manual_grading = false;
    let mut fully_graded_runs: BTreeMap<Uuid, bool> = BTreeMap::new();
    let mut run_scores: BTreeMap<Uuid, (f64, f64)> = BTreeMap::new();

    for item in delivered {
        let key = (
            item.assignment_item,
            item.reference.problem,
            item.reference.version,
        );
        references.entry(key).or_insert(item.reference);
        let input = inputs.entry(key).or_default();
        if item.completed
            && let Some(elapsed) = item.completion_millis
        {
            input.completion_times_millis.push(elapsed);
            completion_times.insert(item.run, elapsed);
        }
        fully_graded_runs.entry(item.run).or_insert(true);
        match (
            item.status.as_deref(),
            item.grading_status.as_deref(),
            item.credit,
            item.correct,
        ) {
            (Some("cleared" | "exempt"), _, _, _) => {}
            (_, Some("needs_manual_grading"), _, _) => {
                input.pending_manual_attempt_count =
                    input.pending_manual_attempt_count.saturating_add(1);
                incomplete_manual_grading = true;
                fully_graded_runs.insert(item.run, false);
            }
            (Some("submitted" | "auto_submitted"), Some("graded"), Some(credit), Some(correct))
                if item.completed =>
            {
                let credit = checked_credit(credit)?;
                let (earned, possible) =
                    item.earned_points
                        .zip(item.possible_points)
                        .ok_or_else(|| {
                            StoreError::Unavailable(
                                "current item-analysis score is missing".to_string(),
                            )
                        })?;
                let aggregate = run_scores.entry(item.run).or_default();
                aggregate.0 += checked_earned_points(earned)?;
                aggregate.1 += checked_possible_points(possible)?;
                graded.push((item.run, key, credit, correct));
            }
            (Some("submitted" | "auto_submitted"), Some("exempt"), _, _) => {}
            _ => {
                input.unanswered_attempt_count = input.unanswered_attempt_count.saturating_add(1);
                fully_graded_runs.insert(item.run, false);
            }
        }
    }
    let mut run_totals: BTreeMap<Uuid, f64> = BTreeMap::new();
    for (run, _, credit, _) in &graded {
        *run_totals.entry(*run).or_default() += *credit;
    }
    for (run, item, credit, correct) in graded {
        let input = inputs.entry(item).or_default();
        input.graded_credits.push(credit);
        input.graded_correct.push(correct);
        input.rest_of_run_credits.push(run_totals[&run] - credit);
    }
    let assignment_scores = run_scores
        .iter()
        .filter_map(|(run, (earned, possible))| {
            fully_graded_runs
                .get(run)
                .copied()
                .filter(|complete| *complete)
                .map(|_| {
                    if *possible > 0.0 {
                        earned / possible
                    } else {
                        *earned
                    }
                })
        })
        .collect::<Vec<_>>();
    let assignment_average_score = (!assignment_scores.is_empty())
        .then(|| assignment_scores.iter().sum::<f64>() / assignment_scores.len() as f64);
    let average_completion_time_millis = (!completion_times.is_empty()).then(|| {
        let total = completion_times
            .values()
            .map(|value| u128::from(*value))
            .sum::<u128>();
        u64::try_from(total / completion_times.len() as u128).unwrap_or(u64::MAX)
    });
    let items = references
        .into_iter()
        .map(|(key @ (assignment_item, _, _), reference)| {
            let metrics = calculate_item_analysis_metrics(
                inputs
                    .get(&key)
                    .unwrap_or(&ItemAnalysisMetricInput::default()),
            )
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            Ok(AssignmentItemAnalysis {
                tenant,
                course,
                assignment,
                assignment_item,
                reference,
                source_scoring_generation: generation,
                analyzed_at,
                graded_attempt_count: metrics.graded_attempt_count,
                unanswered_attempt_count: metrics.response_distribution.unanswered,
                pending_manual_attempt_count: metrics.response_distribution.pending_manual,
                difficulty: metrics.difficulty,
                average_credit: metrics.average_credit,
                credit_standard_deviation: metrics.credit_standard_deviation,
                discrimination: metrics.discrimination,
                response_distribution: metrics.response_distribution,
                average_completion_time_millis: metrics.average_completion_time_millis,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(CourseItemAnalysisReport {
        tenant,
        course,
        assignment,
        source_scoring_generation: generation,
        analyzed_at,
        completed_run_count,
        in_progress_run_count,
        incomplete_manual_grading,
        recent_rescoring: false,
        assignment_average_score,
        average_completion_time_millis,
        items,
    })
}

fn checked_count(value: i64) -> Result<u32, StoreError> {
    u32::try_from(value)
        .map_err(|_| StoreError::Unavailable("stored item-analysis count is invalid".to_string()))
}

fn checked_credit(value: Decimal) -> Result<f64, StoreError> {
    let credit = value
        .to_f64()
        .filter(|credit| credit.is_finite() && (-1000.0..=1000.0).contains(credit))
        .ok_or_else(|| {
            StoreError::Unavailable("stored item-analysis credit is invalid".to_string())
        })?;
    Ok(credit)
}

fn checked_earned_points(value: Decimal) -> Result<f64, StoreError> {
    value
        .to_f64()
        .filter(|points| points.is_finite())
        .ok_or_else(|| {
            StoreError::Unavailable("stored item-analysis earned points are invalid".to_string())
        })
}

fn checked_possible_points(value: Decimal) -> Result<f64, StoreError> {
    value
        .to_f64()
        .filter(|points| points.is_finite() && *points >= 0.0)
        .ok_or_else(|| {
            StoreError::Unavailable("stored item-analysis possible points are invalid".to_string())
        })
}

fn validate_report_identity(
    report: &CourseItemAnalysisReport,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    generation: ScoringGeneration,
) -> Result<(), StoreError> {
    let valid_items = report.items.iter().all(|item| {
        item.tenant == tenant
            && item.course == course
            && item.assignment == assignment
            && item.source_scoring_generation == generation
            && item.analyzed_at == report.analyzed_at
    });
    if report.tenant != tenant
        || report.course != course
        || report.assignment != assignment
        || report.source_scoring_generation != generation
        || !valid_items
    {
        return Err(StoreError::Unavailable(
            "stored item-analysis identity is inconsistent".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn context() -> AnalysisReportContext {
        AnalysisReportContext {
            tenant: TenantId::from_uuid(uuid(1)),
            course: CourseId::from_uuid(uuid(2)),
            assignment: AssignmentId::from_uuid(uuid(3)),
            generation: ScoringGeneration::INITIAL,
            analyzed_at: ActivityTimestamp::from_unix_millis(4),
            completed_run_count: 1,
            in_progress_run_count: 0,
        }
    }

    fn delivered(
        assignment_item: u128,
        status: &str,
        grading_status: &str,
        credit: Option<Decimal>,
        correct: Option<bool>,
        earned_points: Option<Decimal>,
        possible_points: Option<Decimal>,
    ) -> DeliveredItem {
        DeliveredItem {
            assignment_item: AssignmentItemId::from_uuid(uuid(assignment_item)),
            reference: ProblemVersionRef {
                problem: ProblemId::from_uuid(uuid(assignment_item + 100)),
                version: VersionId::from_uuid(uuid(assignment_item + 200)),
            },
            run: uuid(5),
            completed: true,
            completion_millis: Some(6),
            status: Some(status.to_string()),
            grading_status: Some(grading_status.to_string()),
            credit,
            correct,
            earned_points,
            possible_points,
        }
    }

    #[test]
    fn assignment_average_accepts_authored_points_above_manual_credit_range() {
        let report = course_report_from_deliveries(
            context(),
            vec![delivered(
                10,
                "submitted",
                "graded",
                Some(Decimal::new(8, 1)),
                Some(false),
                Some(Decimal::from(1_600)),
                Some(Decimal::from(2_000)),
            )],
        )
        .expect("large exact point values remain valid");

        assert_eq!(report.assignment_average_score, Some(0.8));
        assert_eq!(report.items[0].average_credit, Some(0.8));
    }

    #[test]
    fn pending_manual_item_prevents_partial_assignment_average() {
        let report = course_report_from_deliveries(
            context(),
            vec![
                delivered(
                    10,
                    "submitted",
                    "graded",
                    Some(Decimal::ONE),
                    Some(true),
                    Some(Decimal::ONE),
                    Some(Decimal::ONE),
                ),
                delivered(
                    11,
                    "needs_manual_grading",
                    "needs_manual_grading",
                    None,
                    None,
                    None,
                    None,
                ),
            ],
        )
        .expect("pending item remains an aggregate flag");

        assert_eq!(report.assignment_average_score, None);
        assert!(report.incomplete_manual_grading);
    }
}
