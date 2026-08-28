//! Server-owned assignment scoring invalidation and worker enqueue.

use question_model::{AssignmentId, TenantId};
use sqlx::{Postgres, Transaction};

use super::{decode_scoring_generation, map_sqlx_error};
use crate::StoreError;

use super::scoring_invalidation::ScoringInvalidationBinding;

/// Atomically advances one assignment generation and creates its matching job.
pub(super) async fn enqueue_assignment_recalculation(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    job: crate::JobId,
) -> Result<ScoringInvalidationBinding, StoreError> {
    let row = sqlx::query(
        "SELECT public.ple_enqueue_assignment_recalculation($1, $2, $3, 10) \
                AS scoring_generation",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(job.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(ScoringInvalidationBinding {
        generation: decode_scoring_generation(&row)?,
        job,
    })
}
