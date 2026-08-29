//! In-memory external-tool request and launch validation.

use objects::Sha256Digest;
use question_model::{QuestionAttemptId, StudentResponse, TenantId, UserId};
use uuid::Uuid;

use super::super::{State, StoredExternalToolExchange};
use crate::{
    BeginExternalToolGradeCommand, ExternalToolBinding, ExternalToolLaunchProof, StoreError,
    StudentWorkRoutingBinding,
};

pub(super) fn validate_external_command(
    response: &StudentResponse,
    binding: &ExternalToolBinding,
    lease_millis: u32,
) -> Result<(), StoreError> {
    validate_external_response(response, binding)?;
    if lease_millis == 0 || lease_millis > 300_000 {
        return Err(StoreError::InvalidRecord(
            "external-tool lease must be 1 to 300000 milliseconds".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_external_response(
    response: &StudentResponse,
    binding: &ExternalToolBinding,
) -> Result<(), StoreError> {
    if !matches!(response, StudentResponse::ExternalTool {}) {
        return Err(StoreError::InvalidRecord(
            "external-tool exchange requires the external marker response".to_string(),
        ));
    }
    binding.validate()?;
    let canonical = serde_json::to_vec(response).map_err(|error| {
        StoreError::InvalidRecord(format!("external response encoding failed: {error}"))
    })?;
    if Sha256Digest::compute(&canonical) != binding.response_sha256 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

pub(super) fn validate_active_external_launch(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    student_work_binding: StudentWorkRoutingBinding,
    attempt: QuestionAttemptId,
    binding: &ExternalToolBinding,
    proof: &ExternalToolLaunchProof,
) -> Result<(), StoreError> {
    let session = state
        .external_tool_launch_sessions
        .get(&(tenant, proof.session_id))
        .ok_or(StoreError::Conflict)?;
    if session.actor != actor
        || session.attempt != attempt
        || session.student_work_binding != student_work_binding
        || session.binding != *binding
        || session.revoked
        || session.expires_at <= state.authoritative_time
        || session.token_hash != proof.token.hash()
        || session
            .activity_lease_expires_at
            .is_some_and(|expiry| expiry > state.authoritative_time)
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

pub(super) fn revoke_external_launch(
    state: &mut State,
    tenant: TenantId,
    session_id: Uuid,
) -> Result<(), StoreError> {
    let session = state
        .external_tool_launch_sessions
        .get_mut(&(tenant, session_id))
        .ok_or(StoreError::Conflict)?;
    if session.revoked
        || session
            .activity_lease_expires_at
            .is_some_and(|expiry| expiry > state.authoritative_time)
    {
        return Err(StoreError::Conflict);
    }
    session.revoked = true;
    Ok(())
}

pub(super) fn validate_exchange(
    exchange: &StoredExternalToolExchange,
    command: &BeginExternalToolGradeCommand,
) -> Result<(), StoreError> {
    if exchange.actor != command.actor
        || exchange.student_work_binding != command.student_work_binding
        || exchange.binding != command.binding
        || exchange.response != command.response
        || exchange.key != command.idempotency_key
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}
