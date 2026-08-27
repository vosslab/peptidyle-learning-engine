//! Private PostgreSQL external-tool activity-state and lease validation.

use question_model::{
    AssignmentRun, AttemptStatus, QuestionAttempt, QuestionAttemptId, TenantId, UserId,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::super::{
    entitlement::PreparedStudentAttemptWork, map_sqlx_error, submission_preparation,
};
use super::validation::{
    postgres_binding_matches, postgres_external_binding, postgres_stored_course_matches,
    postgres_validate_external_response,
};
use crate::{
    BeginExternalToolGradeCommand, ExternalToolBinding, ExternalToolLaunchProof,
    ExternalToolLeaseToken, LearnerWorkRoutingBinding, StoreError,
    validate_external_snapshot_binding,
};

pub(super) async fn postgres_external_activity_is_indeterminate(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<bool, StoreError> {
    sqlx::query_scalar::<_, bool>(
        "SELECT external_tool_indeterminate_at IS NOT NULL FROM question_attempt WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)
}

/// Broker-first authorization and exact external-source validation for every
/// learner-authorized external-tool state transition.
pub(super) async fn prepare_external_student_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    learner_work_binding: LearnerWorkRoutingBinding,
    actor: UserId,
    attempt: QuestionAttemptId,
    external_binding: &ExternalToolBinding,
) -> Result<PreparedStudentAttemptWork, StoreError> {
    let prepared = submission_preparation::prepare_bound_student_attempt(
        transaction,
        tenant,
        learner_work_binding,
        actor,
        attempt,
    )
    .await?;
    validate_active_external_attempt(&prepared.attempt, &prepared.run)?;
    validate_external_snapshot_binding(
        &prepared.attempt,
        &prepared.issued_question_snapshot,
        external_binding,
    )?;
    Ok(prepared)
}

pub(super) fn validate_active_external_attempt(
    attempt: &QuestionAttempt,
    run: &AssignmentRun,
) -> Result<(), StoreError> {
    if attempt.status != AttemptStatus::InProgress
        || run.completed_at.is_some()
        || run.score.is_some()
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

pub(super) fn postgres_validate_external_command(
    command: &BeginExternalToolGradeCommand,
) -> Result<(), StoreError> {
    postgres_validate_external_response(&command.response, &command.binding)?;
    if command.lease_millis == 0 || command.lease_millis > 300_000 {
        return Err(StoreError::InvalidRecord(
            "external-tool lease must be 1 to 300000 milliseconds".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn postgres_validate_external_activity_lease_millis(
    lease_millis: u32,
) -> Result<(), StoreError> {
    if lease_millis == 0 || lease_millis > 60_000 {
        return Err(StoreError::InvalidRecord(
            "external-tool activity lease must be 1 to 60000 milliseconds".to_string(),
        ));
    }
    Ok(())
}

pub(super) async fn postgres_has_live_external_activity(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<bool, StoreError> {
    let row = sqlx::query(
        "SELECT 1 FROM external_tool_launch_session \
         WHERE tenant_id = $1 AND attempt_id = $2 \
           AND activity_lease_expires_at > transaction_timestamp() FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(row.is_some())
}

pub(super) async fn postgres_finalization_blocks_ordinary_activity(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<bool, StoreError> {
    let row = sqlx::query(
        "SELECT state, lease_expires_at > transaction_timestamp() AS lease_live \
         FROM external_tool_exchange WHERE tenant_id = $1 AND attempt_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(false);
    };
    let state: String = row.try_get("state").map_err(map_sqlx_error)?;
    let lease_live: Option<bool> = row.try_get("lease_live").map_err(map_sqlx_error)?;
    Ok(state == "verified_pending" || lease_live == Some(true))
}

pub(super) async fn postgres_finalization_lease_is_current(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    verification_lease: &ExternalToolLeaseToken,
) -> Result<bool, StoreError> {
    let row = sqlx::query(
        "SELECT lease_token FROM external_tool_exchange \
         WHERE tenant_id = $1 AND attempt_id = $2 AND state = 'verifying' \
           AND lease_expires_at > transaction_timestamp() FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(false);
    };
    let lease: Vec<u8> = row.try_get("lease_token").map_err(map_sqlx_error)?;
    Ok(lease.as_slice() == verification_lease.bytes().as_slice())
}

pub(super) async fn validate_and_lock_external_launch(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    learner_work_binding: LearnerWorkRoutingBinding,
    attempt: QuestionAttemptId,
    binding: &ExternalToolBinding,
    proof: &ExternalToolLaunchProof,
) -> Result<(), StoreError> {
    let row = sqlx::query(
        "SELECT course_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256 \
         FROM external_tool_launch_session \
         WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 \
           AND revoked_at IS NULL AND expires_at > transaction_timestamp() \
           AND (activity_lease_expires_at IS NULL OR activity_lease_expires_at <= transaction_timestamp()) FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(proof.session_id)
    .bind(attempt.as_uuid())
    .bind(actor.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::Conflict)?;
    let stored = postgres_external_binding(&row)?;
    let token_hash: Vec<u8> = row.try_get("token_sha256").map_err(map_sqlx_error)?;
    if !postgres_binding_matches(&stored, binding)
        || stored.response_sha256 != binding.response_sha256
        || token_hash.as_slice() != proof.token.hash().as_bytes()
        || !postgres_stored_course_matches(&row, learner_work_binding)?
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

pub(super) async fn revoke_locked_external_launch(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    session_id: Uuid,
) -> Result<(), StoreError> {
    let changed = sqlx::query(
        "UPDATE external_tool_launch_session SET revoked_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND launch_session_id = $2 AND revoked_at IS NULL \
           AND (activity_lease_expires_at IS NULL OR activity_lease_expires_at <= transaction_timestamp())",
    )
    .bind(tenant.as_uuid())
    .bind(session_id)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if changed.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}
