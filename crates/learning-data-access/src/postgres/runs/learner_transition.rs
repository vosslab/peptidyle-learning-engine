//! Authorization and lock ordering for learner transitions with a predecessor.
//!
//! A transition screens the learner before it observes or locks the predecessor
//! attempt. It then locks that attempt before the run. Callers revalidate the
//! locked run, enrollment, and active membership before changing state.

use question_model::{
    AssignmentEnrollment, AssignmentRun, QuestionAttemptId, RunId, TenantId, UserId,
};
use sqlx::{Postgres, Transaction};

use crate::{ReceiptNextAttempt, StoreError};

use super::super::assignment_records::load_assignment;
use super::super::connection::map_sqlx_error;
use super::super::row_decode::{decode_payload_row, encode_payload};
use super::super::submission::load_attempt_for_external_update;

/// Screens the active learner capability before a transition locks a related
/// attempt. Pair-locking transitions take attempt then run; this preliminary
/// read keeps an unauthorized caller from using the attempt lookup or its lock
/// as an authorization oracle.
pub(super) async fn preauthorize_learner_run(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    run: RunId,
) -> Result<(), StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM assignment_run \
         WHERE tenant_id = $1 AND run_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let run: AssignmentRun = decode_payload_row(&row)?;
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM enrollment \
         WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run.enrollment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let enrollment: AssignmentEnrollment = decode_payload_row(&row)?;
    if enrollment.user != actor {
        return Err(StoreError::Forbidden);
    }
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM course_member AS cm \
         WHERE cm.tenant_id = $1 AND cm.course_id = $2 AND cm.user_id = $3 \
           AND cm.role = 'student' \
           AND public.ple_course_records_accessible($1, $2))",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .bind(actor.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if !active {
        return Err(StoreError::NotFound);
    }
    Ok(())
}

/// Locks a predecessor attempt only after its owning learner can access the
/// requested run. The caller must lock and revalidate the run afterward.
pub(super) async fn lock_predecessor_for_learner_run(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    run: RunId,
    predecessor: QuestionAttemptId,
) -> Result<(), StoreError> {
    preauthorize_learner_run(transaction, tenant, actor, run).await?;
    if load_attempt_for_external_update(transaction, tenant, predecessor)
        .await?
        .run
        != run
    {
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
