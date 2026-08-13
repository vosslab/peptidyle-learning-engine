//! In-memory external-tool broker and launch-session store.

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{
    ActivityTimestamp, QuestionAttempt, QuestionAttemptId, StudentResponse, TenantId, UserId,
};
use uuid::Uuid;

use super::{
    MemoryStore, State, StoredExternalToolExchange, StoredExternalToolLaunchSession,
    require_attempt_course_records_accessible, require_attempt_owner,
    submit_question_attempt_locked,
};
use crate::{
    BeginExternalToolGradeCommand, ClaimExternalToolFinalizationActivityCommand,
    ClaimedExternalToolActivity, CommitExternalToolSubmissionCommand,
    CommitVerifiedExternalToolSubmissionCommand, CreateExternalToolLaunchSessionCommand,
    CreatedExternalToolLaunchSession, ExternalToolActivityClaim, ExternalToolActivityLeaseToken,
    ExternalToolBegin, ExternalToolBinding, ExternalToolBrokerStore, ExternalToolLaunchProof,
    ExternalToolLaunchSessionStore, ExternalToolLaunchToken, ExternalToolLease,
    ExternalToolLeaseToken, ExternalToolVerifiedPending, StageExternalToolVerificationCommand,
    StoreError, SubmissionRecord, SubmitQuestionAttemptCommand, TenantContext,
    fresh_external_tool_launch_id,
};

#[async_trait]
impl ExternalToolBrokerStore for MemoryStore {
    async fn begin_or_resume_external_grade(
        &self,
        context: TenantContext,
        command: BeginExternalToolGradeCommand,
    ) -> Result<ExternalToolBegin, StoreError> {
        validate_external_command(&command.response, &command.binding, command.lease_millis)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt, command.actor)?;
        require_attempt_course_records_accessible(&state, tenant, &attempt)?;
        if state
            .indeterminate_external_tool_activities
            .contains_key(&(tenant, command.attempt))
        {
            return Err(StoreError::Conflict);
        }
        let published = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        if let Some(submission) = state.submissions.get(&(tenant, command.attempt)) {
            if submission.key == command.idempotency_key && submission.response == command.response
            {
                return Ok(ExternalToolBegin::Committed(Box::new(
                    submission.record.clone(),
                )));
            }
            return Err(StoreError::Conflict);
        }
        let now = state.authoritative_time;
        let live_activity = has_live_external_activity(&state, tenant, command.attempt, now);
        if let Some(exchange) = state
            .external_tool_exchanges
            .get_mut(&(tenant, command.attempt))
        {
            validate_exchange(exchange, &command)?;
            if let Some(verified) = &exchange.verified {
                return Ok(ExternalToolBegin::VerifiedPending(verified.clone()));
            }
            if exchange.lease_expires_at.is_some_and(|expiry| expiry > now) {
                return Ok(ExternalToolBegin::InProgress);
            }
            if live_activity {
                return Ok(ExternalToolBegin::InProgress);
            }
            let token = ExternalToolLeaseToken::generate()?;
            let expires_at = add_external_millis(now, command.lease_millis)?;
            exchange.lease = Some(token.clone());
            exchange.lease_expires_at = Some(expires_at);
            return Ok(ExternalToolBegin::Lease(ExternalToolLease {
                binding: exchange.binding.clone(),
                correlation: exchange.correlation.clone(),
                token,
                expires_at,
            }));
        }
        if live_activity {
            return Ok(ExternalToolBegin::InProgress);
        }
        let token = ExternalToolLeaseToken::generate()?;
        let expires_at = add_external_millis(now, command.lease_millis)?;
        let exchange = StoredExternalToolExchange {
            actor: command.actor,
            binding: command.binding.clone(),
            response: command.response,
            key: command.idempotency_key,
            correlation: command.proposed_correlation.clone(),
            lease: Some(token.clone()),
            lease_expires_at: Some(expires_at),
            verified_lease_hash: None,
            verified: None,
        };
        state
            .external_tool_exchanges
            .insert((tenant, command.attempt), exchange);
        Ok(ExternalToolBegin::Lease(ExternalToolLease {
            binding: command.binding,
            correlation: command.proposed_correlation,
            token,
            expires_at,
        }))
    }

    async fn stage_external_tool_verification(
        &self,
        context: TenantContext,
        command: StageExternalToolVerificationCommand,
    ) -> Result<(), StoreError> {
        validate_external_response(&command.response, &command.binding)?;
        crate::validate_attempt_result(command.result)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let now = state.authoritative_time;
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt, command.actor)?;
        require_attempt_course_records_accessible(&state, tenant, &attempt)?;
        let published = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        let exchange = state
            .external_tool_exchanges
            .get_mut(&(tenant, command.attempt))
            .ok_or(StoreError::NotFound)?;
        if exchange.actor != command.actor
            || exchange.binding != command.binding
            || exchange.response != command.response
            || exchange.key != command.idempotency_key
            || exchange.correlation != command.correlation
            || exchange.lease.as_ref() != Some(&command.lease_token)
            || !exchange.lease_expires_at.is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        let bytes = serde_json::to_vec(&command.result).map_err(|error| {
            StoreError::InvalidRecord(format!("external result encoding failed: {error}"))
        })?;
        exchange.verified = Some(ExternalToolVerifiedPending {
            binding: command.binding.clone(),
            correlation: command.correlation.clone(),
            result: command.result,
            result_sha256: Sha256Digest::compute(&bytes),
        });
        exchange.verified_lease_hash = Some(command.lease_token.hash());
        exchange.lease = None;
        exchange.lease_expires_at = None;
        Ok(())
    }

    async fn commit_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt, command.actor)?;
        require_attempt_course_records_accessible(&state, tenant, &attempt)?;
        validate_external_response(&command.response, &command.binding)?;
        let published = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        if let Some(record) = state.submissions.get(&(tenant, command.attempt)) {
            return if record.key == command.idempotency_key && record.response == command.response {
                Ok(record.record.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        validate_active_external_launch(
            &state,
            tenant,
            command.actor,
            command.attempt,
            &command.binding,
            &command.launch_proof,
        )?;
        let result = {
            let exchange = state
                .external_tool_exchanges
                .get(&(tenant, command.attempt))
                .ok_or(StoreError::NotFound)?;
            if exchange.actor != command.actor
                || exchange.binding != command.binding
                || exchange.response != command.response
                || exchange.key != command.idempotency_key
                || exchange.correlation != command.correlation
                || exchange.verified_lease_hash != Some(command.lease_token.hash())
            {
                return Err(StoreError::Conflict);
            }
            exchange
                .verified
                .as_ref()
                .ok_or(StoreError::Conflict)?
                .result
        };
        // Every fallible check in the generic transition precedes its first
        // mutation; no separate lock or visible half-submission is possible.
        let record = submit_question_attempt_locked(
            &mut state,
            context,
            SubmitQuestionAttemptCommand {
                actor: command.actor,
                attempt: command.attempt,
                response: command.response,
                result,
                feedback: question_model::FeedbackContent::default(),
                idempotency_key: command.idempotency_key,
            },
        )?;
        state
            .external_tool_exchanges
            .remove(&(tenant, command.attempt));
        revoke_external_launch(&mut state, tenant, command.launch_proof.session_id)?;
        Ok(record)
    }

    async fn commit_verified_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitVerifiedExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        validate_external_response(&command.response, &command.binding)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt, command.actor)?;
        require_attempt_course_records_accessible(&state, tenant, &attempt)?;
        let published = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        if let Some(record) = state.submissions.get(&(tenant, command.attempt)) {
            return if record.key == command.idempotency_key && record.response == command.response {
                Ok(record.record.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        validate_active_external_launch(
            &state,
            tenant,
            command.actor,
            command.attempt,
            &command.binding,
            &command.launch_proof,
        )?;
        let result = {
            let exchange = state
                .external_tool_exchanges
                .get(&(tenant, command.attempt))
                .ok_or(StoreError::NotFound)?;
            if exchange.actor != command.actor
                || exchange.binding != command.binding
                || exchange.response != command.response
                || exchange.key != command.idempotency_key
                || exchange.correlation != command.correlation
            {
                return Err(StoreError::Conflict);
            }
            exchange
                .verified
                .as_ref()
                .ok_or(StoreError::Conflict)?
                .result
        };
        let record = submit_question_attempt_locked(
            &mut state,
            context,
            SubmitQuestionAttemptCommand {
                actor: command.actor,
                attempt: command.attempt,
                response: command.response,
                result,
                feedback: question_model::FeedbackContent::default(),
                idempotency_key: command.idempotency_key,
            },
        )?;
        state
            .external_tool_exchanges
            .remove(&(tenant, command.attempt));
        revoke_external_launch(&mut state, tenant, command.launch_proof.session_id)?;
        Ok(record)
    }
}

#[async_trait]
impl ExternalToolLaunchSessionStore for MemoryStore {
    async fn create_external_tool_launch_session(
        &self,
        context: TenantContext,
        command: CreateExternalToolLaunchSessionCommand,
    ) -> Result<CreatedExternalToolLaunchSession, StoreError> {
        validate_external_response(&StudentResponse::ExternalTool {}, &command.binding)?;
        if command.lifetime_millis == 0
            || command.lifetime_millis > 900_000
            || command
                .encrypted_provider_state
                .as_ref()
                .is_some_and(|bytes| bytes.len() > 65_536)
        {
            return Err(StoreError::InvalidRecord(
                "external-tool launch session is invalid".to_string(),
            ));
        }
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt, command.actor)?;
        require_attempt_course_records_accessible(&state, tenant, &attempt)?;
        if state
            .indeterminate_external_tool_activities
            .contains_key(&(tenant, command.attempt))
        {
            return Err(StoreError::Conflict);
        }
        let published = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        let token = ExternalToolLaunchToken::generate()?;
        let expires_at =
            add_external_launch_millis(state.authoritative_time, command.lifetime_millis)?;
        let id = fresh_external_tool_launch_id()?;
        state.external_tool_launch_sessions.insert(
            (tenant, id),
            StoredExternalToolLaunchSession {
                actor: command.actor,
                attempt: command.attempt,
                binding: command.binding,
                token_hash: token.hash(),
                encrypted_provider_state: command.encrypted_provider_state,
                expires_at,
                revoked: false,
                activity_lease_hash: None,
                activity_lease_expires_at: None,
            },
        );
        Ok(CreatedExternalToolLaunchSession {
            id,
            token,
            expires_at,
        })
    }
    async fn claim_external_tool_activity(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolLaunchToken,
        lease_millis: u32,
    ) -> Result<ExternalToolActivityClaim, StoreError> {
        let lease_millis = validate_external_activity_lease_millis(lease_millis)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, record, actor)?;
        require_attempt_course_records_accessible(&state, tenant, record)?;
        let Some(session) = state.external_tool_launch_sessions.get(&(tenant, id)) else {
            return Ok(ExternalToolActivityClaim::Unavailable);
        };
        if session.actor != actor || session.attempt != attempt {
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        if session.revoked
            || session.expires_at <= state.authoritative_time
            || session.token_hash != token.hash()
        {
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        if state
            .indeterminate_external_tool_activities
            .contains_key(&(tenant, attempt))
        {
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        if finalization_blocks_ordinary_activity(&state, tenant, attempt, state.authoritative_time)
        {
            return Ok(ExternalToolActivityClaim::InProgress);
        }
        claim_external_activity_locked(&mut state, tenant, id, lease_millis)
    }

    async fn claim_external_tool_finalization_activity(
        &self,
        context: TenantContext,
        command: ClaimExternalToolFinalizationActivityCommand,
    ) -> Result<ExternalToolActivityClaim, StoreError> {
        let ClaimExternalToolFinalizationActivityCommand {
            actor,
            attempt,
            id,
            token,
            verification_lease,
            lease_millis,
        } = command;
        let lease_millis = validate_external_activity_lease_millis(lease_millis)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, record, actor)?;
        require_attempt_course_records_accessible(&state, tenant, record)?;
        let Some(session) = state.external_tool_launch_sessions.get(&(tenant, id)) else {
            return Ok(ExternalToolActivityClaim::Unavailable);
        };
        if session.actor != actor
            || session.attempt != attempt
            || session.revoked
            || session.expires_at <= state.authoritative_time
            || session.token_hash != token.hash()
        {
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        if state
            .indeterminate_external_tool_activities
            .contains_key(&(tenant, attempt))
        {
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        let Some(exchange) = state.external_tool_exchanges.get(&(tenant, attempt)) else {
            return Ok(ExternalToolActivityClaim::Unavailable);
        };
        if exchange.verified.is_some()
            || exchange.lease.as_ref() != Some(&verification_lease)
            || !exchange
                .lease_expires_at
                .is_some_and(|expiry| expiry > state.authoritative_time)
        {
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        claim_external_activity_locked(&mut state, tenant, id, lease_millis)
    }

    async fn release_external_tool_activity(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolActivityLeaseToken,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt_record = state
            .attempts
            .get(&(tenant, attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt_record, actor)?;
        require_attempt_course_records_accessible(&state, tenant, &attempt_record)?;
        let session = state
            .external_tool_launch_sessions
            .get_mut(&(tenant, id))
            .ok_or(StoreError::NotFound)?;
        if session.actor != actor
            || session.attempt != attempt
            || session.activity_lease_hash.as_ref() != Some(&token.hash())
        {
            return Err(StoreError::Conflict);
        }
        session.activity_lease_hash = None;
        session.activity_lease_expires_at = None;
        Ok(())
    }

    async fn begin_external_tool_activity_dispatch(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolActivityLeaseToken,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let session = state
            .external_tool_launch_sessions
            .get(&(tenant, id))
            .ok_or(StoreError::NotFound)?;
        if session.actor != actor
            || session.attempt != attempt
            || session.activity_lease_hash.as_ref() != Some(&token.hash())
            || session
                .activity_lease_expires_at
                .is_none_or(|expiry| expiry <= state.authoritative_time)
            || state
                .indeterminate_external_tool_activities
                .contains_key(&(tenant, attempt))
        {
            return Err(StoreError::Conflict);
        }
        state
            .indeterminate_external_tool_activities
            .insert((tenant, attempt), token.hash());
        Ok(())
    }

    async fn complete_external_tool_activity_dispatch(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        token: &ExternalToolActivityLeaseToken,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt_record = state
            .attempts
            .get(&(tenant, attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt_record, actor)?;
        if state
            .indeterminate_external_tool_activities
            .get(&(tenant, attempt))
            != Some(&token.hash())
        {
            return Err(StoreError::Conflict);
        }
        state
            .indeterminate_external_tool_activities
            .remove(&(tenant, attempt));
        Ok(())
    }

    async fn fence_indeterminate_external_tool_activity(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolActivityLeaseToken,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt_record = state
            .attempts
            .get(&(tenant, attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt_record, actor)?;
        require_attempt_course_records_accessible(&state, tenant, &attempt_record)?;
        let session = state
            .external_tool_launch_sessions
            .get_mut(&(tenant, id))
            .ok_or(StoreError::NotFound)?;
        if session.actor != actor
            || session.attempt != attempt
            || session.activity_lease_hash.as_ref() != Some(&token.hash())
        {
            return Err(StoreError::Conflict);
        }
        session.activity_lease_hash = None;
        session.activity_lease_expires_at = None;
        session.revoked = true;
        state
            .indeterminate_external_tool_activities
            .entry((tenant, attempt))
            .or_insert_with(|| token.hash());
        Ok(())
    }
    async fn revoke_external_tool_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt_record = state
            .attempts
            .get(&(tenant, attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt_record, actor)?;
        require_attempt_course_records_accessible(&state, tenant, &attempt_record)?;
        if state
            .indeterminate_external_tool_activities
            .contains_key(&(tenant, attempt))
        {
            return Err(StoreError::Conflict);
        }
        let now = state.authoritative_time;
        let session = state
            .external_tool_launch_sessions
            .get_mut(&(tenant, id))
            .ok_or(StoreError::NotFound)?;
        if session.actor != actor || session.attempt != attempt {
            return Err(StoreError::NotFound);
        }
        if session
            .activity_lease_expires_at
            .is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        session.revoked = true;
        Ok(())
    }
}

fn add_external_millis(
    now: ActivityTimestamp,
    millis: u32,
) -> Result<ActivityTimestamp, StoreError> {
    if millis == 0 || millis > 300_000 {
        return Err(StoreError::InvalidRecord(
            "external-tool lease must be 1 to 300000 milliseconds".to_string(),
        ));
    }
    now.as_unix_millis()
        .checked_add(i64::from(millis))
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| {
            StoreError::InvalidRecord("external-tool lease timestamp overflow".to_string())
        })
}

fn add_external_launch_millis(
    now: ActivityTimestamp,
    millis: u32,
) -> Result<ActivityTimestamp, StoreError> {
    if millis == 0 || millis > 900_000 {
        return Err(StoreError::InvalidRecord(
            "external-tool launch session is invalid".to_string(),
        ));
    }
    now.as_unix_millis()
        .checked_add(i64::from(millis))
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| {
            StoreError::InvalidRecord("external-tool launch timestamp overflow".to_string())
        })
}

fn validate_external_activity_lease_millis(millis: u32) -> Result<u32, StoreError> {
    if millis == 0 || millis > 60_000 {
        return Err(StoreError::InvalidRecord(
            "external-tool activity lease must be 1 to 60000 milliseconds".to_string(),
        ));
    }
    Ok(millis)
}

fn has_live_external_activity(
    state: &State,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    now: ActivityTimestamp,
) -> bool {
    state
        .external_tool_launch_sessions
        .iter()
        .any(|((session_tenant, _), session)| {
            *session_tenant == tenant
                && session.attempt == attempt
                && session
                    .activity_lease_expires_at
                    .is_some_and(|expiry| expiry > now)
        })
}

fn finalization_blocks_ordinary_activity(
    state: &State,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    now: ActivityTimestamp,
) -> bool {
    if state.submissions.contains_key(&(tenant, attempt)) {
        return false;
    }
    let Some(exchange) = state.external_tool_exchanges.get(&(tenant, attempt)) else {
        return false;
    };
    exchange.verified.is_some() || exchange.lease_expires_at.is_some_and(|expiry| expiry > now)
}

fn claim_external_activity_locked(
    state: &mut State,
    tenant: TenantId,
    id: Uuid,
    lease_millis: u32,
) -> Result<ExternalToolActivityClaim, StoreError> {
    let now = state.authoritative_time;
    let session = state
        .external_tool_launch_sessions
        .get_mut(&(tenant, id))
        .expect("checked launch session remains present while memory state is locked");
    if session
        .activity_lease_expires_at
        .is_some_and(|expiry| expiry > now)
    {
        return Ok(ExternalToolActivityClaim::InProgress);
    }
    let activity_token = ExternalToolActivityLeaseToken::generate()?;
    let expires_at = add_external_activity_millis(now, lease_millis)?;
    session.activity_lease_hash = Some(activity_token.hash());
    session.activity_lease_expires_at = Some(expires_at);
    Ok(ExternalToolActivityClaim::Lease(Box::new(
        ClaimedExternalToolActivity {
            binding: session.binding.clone(),
            encrypted_provider_state: session.encrypted_provider_state.clone(),
            token: activity_token,
            expires_at,
        },
    )))
}

fn add_external_activity_millis(
    now: ActivityTimestamp,
    millis: u32,
) -> Result<ActivityTimestamp, StoreError> {
    let millis = validate_external_activity_lease_millis(millis)?;
    now.as_unix_millis()
        .checked_add(i64::from(millis))
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| {
            StoreError::InvalidRecord("external-tool activity lease timestamp overflow".to_string())
        })
}

fn validate_external_command(
    response: &StudentResponse,
    binding: &ExternalToolBinding,
    lease_millis: u32,
) -> Result<(), StoreError> {
    validate_external_response(response, binding)?;
    let _ = add_external_millis(ActivityTimestamp::from_unix_millis(0), lease_millis)?;
    Ok(())
}

fn validate_external_binding(
    attempt: &QuestionAttempt,
    source: &question_model::QuestionSource,
    binding: &ExternalToolBinding,
) -> Result<(), StoreError> {
    if attempt.problem != binding.problem
        || attempt.question_version != binding.version
        || attempt.seed != binding.seed
    {
        return Err(StoreError::Conflict);
    }
    let provenance_source = attempt
        .provenance
        .source_artifact
        .as_ref()
        .ok_or(StoreError::Conflict)?;
    if provenance_source.object != binding.source_object
        || provenance_source.sha256 != binding.source_sha256
    {
        return Err(StoreError::Conflict);
    }
    let question_model::QuestionSource::Imathas {
        provider,
        snapshot,
        snapshot_sha256,
        integration_profile,
        ..
    } = source
    else {
        return Err(StoreError::Conflict);
    };
    if provider != &binding.provider
        || snapshot != &binding.source_object
        || snapshot_sha256 != &binding.source_sha256
        || integration_profile != &binding.integration_profile
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn validate_external_response(
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

fn validate_active_external_launch(
    state: &State,
    tenant: TenantId,
    actor: UserId,
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

fn revoke_external_launch(
    state: &mut State,
    tenant: TenantId,
    session_id: Uuid,
) -> Result<(), StoreError> {
    let session = state
        .external_tool_launch_sessions
        .get_mut(&(tenant, session_id))
        .ok_or(StoreError::Conflict)?;
    if session.revoked {
        return Err(StoreError::Conflict);
    }
    if session
        .activity_lease_expires_at
        .is_some_and(|expiry| expiry > state.authoritative_time)
    {
        return Err(StoreError::Conflict);
    }
    session.revoked = true;
    Ok(())
}

fn validate_exchange(
    exchange: &StoredExternalToolExchange,
    command: &BeginExternalToolGradeCommand,
) -> Result<(), StoreError> {
    if exchange.actor != command.actor
        || exchange.binding != command.binding
        || exchange.response != command.response
        || exchange.key != command.idempotency_key
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}
