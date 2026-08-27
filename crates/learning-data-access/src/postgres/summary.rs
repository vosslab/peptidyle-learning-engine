//! Typed PostgreSQL codec for the compact assignment scoring projection.

use question_model::{ActivityTimestamp, EnrollmentId, StudentAssignmentSummary, TenantId};
use sqlx::Row;
use sqlx::postgres::PgRow;
use sqlx::types::Uuid;

use super::map_sqlx_error;
use crate::StoreError;

pub(super) struct SummaryRowValues {
    pub(super) tenant_id: Uuid,
    pub(super) enrollment_id: Uuid,
    pub(super) current_score: Option<f64>,
    pub(super) best_score: Option<f64>,
    pub(super) latest_score: Option<f64>,
    pub(super) completed_run_count: i64,
    pub(super) total_question_attempts: i64,
    pub(super) last_activity_at_millis: Option<i64>,
}

pub(super) fn decode_summary_row(row: &PgRow) -> Result<StudentAssignmentSummary, StoreError> {
    decode_summary_row_named(row, "")
}

pub(super) fn decode_summary_values(
    values: SummaryRowValues,
) -> Result<StudentAssignmentSummary, StoreError> {
    Ok(StudentAssignmentSummary {
        tenant: TenantId::from_uuid(values.tenant_id),
        enrollment: EnrollmentId::from_uuid(values.enrollment_id),
        current_score: values.current_score,
        best_score: values.best_score,
        latest_score: values.latest_score,
        completed_run_count: u32::try_from(values.completed_run_count).map_err(|_| {
            StoreError::Unavailable("stored completed-run count is invalid".to_string())
        })?,
        total_question_attempts: u64::try_from(values.total_question_attempts).map_err(|_| {
            StoreError::Unavailable("stored question-attempt count is invalid".to_string())
        })?,
        last_activity_at: values
            .last_activity_at_millis
            .map(ActivityTimestamp::from_unix_millis),
    })
}

pub(super) fn decode_summary_row_named(
    row: &PgRow,
    prefix: &str,
) -> Result<StudentAssignmentSummary, StoreError> {
    let column = |name: &str| format!("{prefix}{name}");
    let tenant = column("tenant_id");
    let enrollment = column("enrollment_id");
    let current_score = column("current_score");
    let best_score = column("best_score");
    let latest_score = column("latest_score");
    let completed_run_count = column("completed_run_count");
    let total_question_attempts = column("total_question_attempts");
    let last_activity_at_millis = column("last_activity_at_millis");
    let completed = row
        .try_get::<i64, _>(completed_run_count.as_str())
        .map_err(map_sqlx_error)?;
    let attempts = row
        .try_get::<i64, _>(total_question_attempts.as_str())
        .map_err(map_sqlx_error)?;
    decode_summary_values(SummaryRowValues {
        tenant_id: row
            .try_get::<Uuid, _>(tenant.as_str())
            .map_err(map_sqlx_error)?,
        enrollment_id: row
            .try_get::<Uuid, _>(enrollment.as_str())
            .map_err(map_sqlx_error)?,
        current_score: row
            .try_get(current_score.as_str())
            .map_err(map_sqlx_error)?,
        best_score: row.try_get(best_score.as_str()).map_err(map_sqlx_error)?,
        latest_score: row.try_get(latest_score.as_str()).map_err(map_sqlx_error)?,
        completed_run_count: completed,
        total_question_attempts: attempts,
        last_activity_at_millis: row
            .try_get::<Option<i64>, _>(last_activity_at_millis.as_str())
            .map_err(map_sqlx_error)?,
    })
}
