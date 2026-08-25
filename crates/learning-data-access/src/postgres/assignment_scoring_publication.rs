//! Server-owned publication of one assignment scoring generation.

use question_model::{AssignmentId, ScoringGeneration, TenantId};
use sqlx::{Postgres, Transaction};

use super::map_sqlx_error;
use crate::{JobId, JobLeaseToken, StoreError};

/// Publishes one still-current generation and creates its analysis handoff.
pub(super) async fn publish_assignment_scoring_generation(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    scoring_job: JobId,
    scoring_lease: JobLeaseToken,
    assignment: AssignmentId,
    generation: ScoringGeneration,
) -> Result<bool, StoreError> {
    let analysis_job = JobId::generate()?;
    sqlx::query_scalar(
        "SELECT public.ple_publish_assignment_scoring_generation(\
            $1, $2, $3, $4, $5, $6)",
    )
    .bind(tenant.as_uuid())
    .bind(scoring_job.as_uuid())
    .bind(scoring_lease.as_uuid())
    .bind(assignment.as_uuid())
    .bind(i64::try_from(generation.value()).map_err(|_| StoreError::Conflict)?)
    .bind(analysis_job.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
}
