//! Server-owned item-analysis generation publication.

use question_model::{AssignmentId, ScoringGeneration, TenantId};
use sqlx::{Postgres, Transaction};

use super::map_sqlx_error;
use crate::{JobId, JobLeaseToken, StoreError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ItemAnalysisPublicationOutcome {
    ClaimNoLongerActive,
    Superseded,
    StagingUnavailable,
    Committed,
}

pub(super) async fn commit_item_analysis_generation(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    job: JobId,
    lease: JobLeaseToken,
    assignment: AssignmentId,
    generation: ScoringGeneration,
) -> Result<ItemAnalysisPublicationOutcome, StoreError> {
    let outcome: String = sqlx::query_scalar(
        "SELECT public.ple_commit_course_item_analysis_generation($1, $2, $3, $4, $5)",
    )
    .bind(tenant.as_uuid())
    .bind(job.as_uuid())
    .bind(lease.as_uuid())
    .bind(assignment.as_uuid())
    .bind(i64::try_from(generation.value()).map_err(|_| StoreError::Conflict)?)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    match outcome.as_str() {
        "claim_no_longer_active" => Ok(ItemAnalysisPublicationOutcome::ClaimNoLongerActive),
        "superseded" => Ok(ItemAnalysisPublicationOutcome::Superseded),
        "staging_unavailable" => Ok(ItemAnalysisPublicationOutcome::StagingUnavailable),
        "committed" => Ok(ItemAnalysisPublicationOutcome::Committed),
        _ => Err(StoreError::Unavailable(
            "course item analysis publication returned an invalid outcome".to_string(),
        )),
    }
}
