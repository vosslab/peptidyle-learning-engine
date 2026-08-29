//! Decoding boundary for source-specific scoring-invalidation capabilities.
//!
//! Each public SQL capability derives its own authority witness. This adapter
//! supplies only typed identifiers and rejects any result that does not bind
//! the exact generation, job, and immutable source identity requested by the
//! surrounding transaction.

use question_model::{
    AssignmentId, CourseId, GradingOperationReference, ScoringGeneration, TenantId,
};
use sqlx::{Postgres, Row, Transaction};

use super::map_sqlx_error;
use crate::{JobId, StoreError};

const ATTEMPT_SUPPORT_SQL: &str =
    "SELECT * FROM public.ple_bind_attempt_support_invalidation_v1($1,$2,$3)";
const ASSIGNMENT_DEFINITION_SQL: &str =
    "SELECT * FROM public.ple_bind_assignment_definition_invalidation_v1($1,$2,$3,$4,$5,$6)";
const ACCEPTED_COMPLETION_SQL: &str =
    "SELECT * FROM public.ple_bind_accepted_completion_invalidation_v1($1,$2,$3,$4,$5)";

/// The exact recalculation generation and job sealed by a source capability.
#[derive(Clone, Copy)]
pub(super) struct ScoringInvalidationBinding {
    pub(super) generation: ScoringGeneration,
    pub(super) job: JobId,
}

pub(super) async fn bind_attempt_support(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    action: uuid::Uuid,
    expected: ScoringInvalidationBinding,
) -> Result<(), StoreError> {
    let row = sqlx::query(ATTEMPT_SUPPORT_SQL)
        .bind(tenant.as_uuid())
        .bind(action)
        .bind(expected.job.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    validate_binding(&row, expected, action, "attempt-support")
}

pub(super) async fn bind_assignment_definition(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: uuid::Uuid,
    course: CourseId,
    assignment: AssignmentId,
    revision: i64,
    expected: ScoringInvalidationBinding,
) -> Result<(), StoreError> {
    let row = sqlx::query(ASSIGNMENT_DEFINITION_SQL)
        .bind(tenant.as_uuid())
        .bind(actor)
        .bind(course.as_uuid())
        .bind(assignment.as_uuid())
        .bind(revision)
        .bind(expected.job.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    validate_binding(
        &row,
        expected,
        expected.job.as_uuid(),
        "assignment-definition",
    )
}

pub(super) async fn bind_accepted_completion(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    execution_job: JobId,
    submission: uuid::Uuid,
    execution_generation: i64,
    expected: ScoringInvalidationBinding,
) -> Result<(), StoreError> {
    let row = sqlx::query(ACCEPTED_COMPLETION_SQL)
        .bind(tenant.as_uuid())
        .bind(execution_job.as_uuid())
        .bind(submission)
        .bind(execution_generation)
        .bind(expected.job.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    validate_binding(&row, expected, submission, "accepted-completion")
}

fn validate_binding(
    row: &sqlx::postgres::PgRow,
    expected: ScoringInvalidationBinding,
    expected_origin: uuid::Uuid,
    capability: &str,
) -> Result<(), StoreError> {
    let disposition: String = row.try_get("disposition").map_err(map_sqlx_error)?;
    if disposition != "accepted" && disposition != "replayed" {
        return Err(StoreError::Unavailable(format!(
            "{capability} invalidation capability returned an invalid disposition"
        )));
    }
    let operation_reference: i32 = row.try_get("operation_reference").map_err(map_sqlx_error)?;
    u32::try_from(operation_reference)
        .ok()
        .and_then(|value| GradingOperationReference::new(u64::from(value)))
        .ok_or_else(|| {
            StoreError::Unavailable(format!(
                "{capability} invalidation capability returned an invalid operation reference"
            ))
        })?;
    let raw_generation: i64 = row.try_get("scoring_generation").map_err(map_sqlx_error)?;
    let generation = ScoringGeneration::new(u64::try_from(raw_generation).map_err(|_| {
        StoreError::Unavailable(format!(
            "{capability} invalidation capability returned an invalid generation"
        ))
    })?)
    .ok_or_else(|| {
        StoreError::Unavailable(format!(
            "{capability} invalidation capability returned an invalid generation"
        ))
    })?;
    let job: uuid::Uuid = row
        .try_get("recalculation_job_id")
        .map_err(map_sqlx_error)?;
    let origin: uuid::Uuid = row.try_get("origin_id").map_err(map_sqlx_error)?;
    if generation != expected.generation
        || job != expected.job.as_uuid()
        || origin != expected_origin
    {
        return Err(StoreError::Unavailable(format!(
            "{capability} invalidation capability returned mismatched causal evidence"
        )));
    }
    Ok(())
}
