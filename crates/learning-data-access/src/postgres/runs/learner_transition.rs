//! Authorization and lock ordering for learner transitions with a predecessor.
//!
//! The 1817 learner-work broker prepares and locks the route, membership,
//! enrollment, run, and attempt before these helpers link immutable transition
//! evidence.

use question_model::{QuestionAttemptId, RunId, TenantId};
use sqlx::{Postgres, Transaction};

use crate::{ReceiptNextAttempt, StoreError};

use super::super::connection::map_sqlx_error;
use super::super::row_decode::encode_payload;
use super::super::submission::load_attempt_for_external_update;

/// Locks and verifies a predecessor after a run-specific broker preparation.
///
/// This helper performs no route, run, enrollment, assignment, or membership
/// discovery. The caller already holds the 1817 run witness and its canonical
/// source locks.
pub(super) async fn lock_prepared_predecessor_for_learner_run(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    run: RunId,
    predecessor: QuestionAttemptId,
) -> Result<(), StoreError> {
    if load_attempt_for_external_update(transaction, tenant, predecessor)
        .await?
        .run
        != run
    {
        return Err(StoreError::Conflict);
    }
    let submitted: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM submission_idempotency \
         WHERE tenant_id = $1 AND attempt_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(predecessor.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if !submitted {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

/// Links an issued successor to its submitted predecessor. The unique key
/// serializes concurrent writers; a loser accepts only the exact durable link.
pub(super) async fn record_submission_successor(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    predecessor: QuestionAttemptId,
    next: &ReceiptNextAttempt,
) -> Result<(), StoreError> {
    let submitted: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM submission_idempotency \
         WHERE tenant_id = $1 AND attempt_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(predecessor.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if !submitted {
        return Err(StoreError::Conflict);
    }
    let (next_payload, next_payload_sha256) = encode_payload(next)?;
    let inserted = sqlx::query(
        "INSERT INTO submission_next_attempt \
         (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at, \
          next_payload, next_payload_sha256) \
         VALUES ($1, $2, $3, transaction_timestamp(), $4, $5) \
         ON CONFLICT (tenant_id, predecessor_attempt_id) DO NOTHING",
    )
    .bind(tenant.as_uuid())
    .bind(predecessor.as_uuid())
    .bind(next.id.as_uuid())
    .bind(next_payload)
    .bind(next_payload_sha256)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if inserted.rows_affected() == 0 {
        super::require_exact_submission_successor(transaction, tenant, predecessor, Some(next))
            .await?;
    }
    Ok(())
}
