//! In-memory external-tool broker and launch-session store.

mod validation;

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{
    ActivityTimestamp, AttemptStatus, QuestionAttemptId, StudentResponse, TenantId, UserId,
};
use uuid::Uuid;

use super::{
    MemoryStore, State, StoredExternalToolExchange, StoredExternalToolLaunchSession,
    assignment_record, enrollment_record, projected_attempt, require_course_records_accessible,
    submit_question_attempt_locked,
};
use crate::{
    BeginExternalToolGradeCommand, ClaimExternalToolFinalizationActivityCommand,
    ClaimedExternalToolActivity, CommitExternalToolSubmissionCommand,
    CommitVerifiedExternalToolSubmissionCommand, CreateExternalToolLaunchSessionCommand,
    CreatedExternalToolLaunchSession, ExternalToolActivityClaim, ExternalToolActivityLeaseToken,
    ExternalToolBegin, ExternalToolBrokerStore, ExternalToolLaunchSessionStore,
    ExternalToolLaunchToken, ExternalToolLease, ExternalToolLeaseToken,
    ExternalToolVerifiedPending, LearnerWorkRoutingBinding, PreparedExternalToolAttempt,
    StageExternalToolVerificationCommand, StoreError, SubmissionRecord,
    SubmitQuestionAttemptCommand, TenantContext, fresh_external_tool_launch_id,
    validate_external_snapshot_binding,
};
use validation::{
    revoke_external_launch, validate_active_external_launch, validate_exchange,
    validate_external_command, validate_external_response,
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
        let prepared = match super::runs::submission_preparation::prepare_question_submission(
            &state,
            context,
            command.actor,
            command.learner_work_binding,
            command.attempt,
            &command.response,
            &command.idempotency_key,
        )? {
            crate::SubmissionPreparation::Replay(record) => {
                return Ok(ExternalToolBegin::Committed(record));
            }
            crate::SubmissionPreparation::Grade(prepared) => prepared,
        };
        if state
            .indeterminate_external_tool_activities
            .contains_key(&(tenant, command.attempt))
        {
            return Err(StoreError::Conflict);
        }
        validate_external_snapshot_binding(
            &prepared.attempt,
            &prepared.issued_question_snapshot,
            &command.binding,
        )?;
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
            learner_work_binding: command.learner_work_binding,
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
        // Exact lease-bound evidence staging is cleanup after provider I/O,
        // not a new learner effect. The subsequent grade commit reauthorizes.
        let exchange = state
            .external_tool_exchanges
            .get_mut(&(tenant, command.attempt))
            .ok_or(StoreError::NotFound)?;
        if exchange.actor != command.actor
            || exchange.learner_work_binding != command.learner_work_binding
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
        let prepared = match super::runs::submission_preparation::prepare_question_submission(
            &state,
            context,
            command.actor,
            command.learner_work_binding,
            command.attempt,
            &command.response,
            &command.idempotency_key,
        )? {
            crate::SubmissionPreparation::Replay(record) => return Ok(*record),
            crate::SubmissionPreparation::Grade(prepared) => prepared,
        };
        validate_external_response(&command.response, &command.binding)?;
        validate_external_snapshot_binding(
            &prepared.attempt,
            &prepared.issued_question_snapshot,
            &command.binding,
        )?;
        validate_active_external_launch(
            &state,
            tenant,
            command.actor,
            command.learner_work_binding,
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
                || exchange.learner_work_binding != command.learner_work_binding
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
                binding: command.learner_work_binding,
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
        let prepared = match super::runs::submission_preparation::prepare_question_submission(
            &state,
            context,
            command.actor,
            command.learner_work_binding,
            command.attempt,
            &command.response,
            &command.idempotency_key,
        )? {
            crate::SubmissionPreparation::Replay(record) => return Ok(*record),
            crate::SubmissionPreparation::Grade(prepared) => prepared,
        };
        validate_external_snapshot_binding(
            &prepared.attempt,
            &prepared.issued_question_snapshot,
            &command.binding,
        )?;
        validate_active_external_launch(
            &state,
            tenant,
            command.actor,
            command.learner_work_binding,
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
                || exchange.learner_work_binding != command.learner_work_binding
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
                binding: command.learner_work_binding,
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
    async fn prepare_external_tool_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        learner_work_binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
    ) -> Result<PreparedExternalToolAttempt, StoreError> {
        let state = self.read_state()?;
        prepare_external_tool_attempt_locked(&state, context, actor, learner_work_binding, attempt)
    }

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
        let prepared = prepare_external_tool_attempt_locked(
            &state,
            context,
            command.actor,
            command.learner_work_binding,
            command.attempt,
        )?;
        if state
            .indeterminate_external_tool_activities
            .contains_key(&(tenant, command.attempt))
        {
            return Err(StoreError::Conflict);
        }
        validate_external_snapshot_binding(
            &prepared.attempt,
            &prepared.issued_question_snapshot,
            &command.binding,
        )?;
        let token = ExternalToolLaunchToken::generate()?;
        let expires_at =
            add_external_launch_millis(state.authoritative_time, command.lifetime_millis)?;
        let id = fresh_external_tool_launch_id()?;
        state.external_tool_launch_sessions.insert(
            (tenant, id),
            StoredExternalToolLaunchSession {
                actor: command.actor,
                attempt: command.attempt,
                learner_work_binding: command.learner_work_binding,
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
        learner_work_binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolLaunchToken,
        lease_millis: u32,
    ) -> Result<ExternalToolActivityClaim, StoreError> {
        let lease_millis = validate_external_activity_lease_millis(lease_millis)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        prepare_external_tool_attempt_locked(
            &state,
            context,
            actor,
            learner_work_binding,
            attempt,
        )?;
        let Some(session) = state.external_tool_launch_sessions.get(&(tenant, id)) else {
            return Ok(ExternalToolActivityClaim::Unavailable);
        };
        if session.actor != actor
            || session.attempt != attempt
            || session.learner_work_binding != learner_work_binding
            || session.token_hash != token.hash()
        {
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

    async fn claim_and_begin_external_tool_activity_dispatch(
        &self,
        context: TenantContext,
        actor: UserId,
        learner_work_binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolLaunchToken,
        lease_millis: u32,
    ) -> Result<ExternalToolActivityClaim, StoreError> {
        let lease_millis = validate_external_activity_lease_millis(lease_millis)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        prepare_external_tool_attempt_locked(
            &state,
            context,
            actor,
            learner_work_binding,
            attempt,
        )?;
        let Some(session) = state.external_tool_launch_sessions.get(&(tenant, id)) else {
            return Ok(ExternalToolActivityClaim::Unavailable);
        };
        if session.actor != actor
            || session.attempt != attempt
            || session.learner_work_binding != learner_work_binding
            || session.revoked
            || session.expires_at <= state.authoritative_time
            || session.token_hash != token.hash()
            || state
                .indeterminate_external_tool_activities
                .contains_key(&(tenant, attempt))
        {
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        if finalization_blocks_ordinary_activity(&state, tenant, attempt, state.authoritative_time)
        {
            return Ok(ExternalToolActivityClaim::InProgress);
        }
        let claim = claim_external_activity_locked(&mut state, tenant, id, lease_millis)?;
        if let ExternalToolActivityClaim::Lease(lease) = &claim {
            state
                .indeterminate_external_tool_activities
                .insert((tenant, attempt), lease.token.hash());
        }
        Ok(claim)
    }

    async fn claim_external_tool_finalization_activity(
        &self,
        context: TenantContext,
        command: ClaimExternalToolFinalizationActivityCommand,
    ) -> Result<ExternalToolActivityClaim, StoreError> {
        let ClaimExternalToolFinalizationActivityCommand {
            actor,
            learner_work_binding,
            attempt,
            id,
            token,
            verification_lease,
            lease_millis,
        } = command;
        let lease_millis = validate_external_activity_lease_millis(lease_millis)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        prepare_external_tool_attempt_locked(
            &state,
            context,
            actor,
            learner_work_binding,
            attempt,
        )?;
        let Some(session) = state.external_tool_launch_sessions.get(&(tenant, id)) else {
            return Ok(ExternalToolActivityClaim::Unavailable);
        };
        if session.actor != actor
            || session.attempt != attempt
            || session.learner_work_binding != learner_work_binding
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
            || exchange.learner_work_binding != learner_work_binding
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
        learner_work_binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolActivityLeaseToken,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let session = state
            .external_tool_launch_sessions
            .get_mut(&(tenant, id))
            .ok_or(StoreError::NotFound)?;
        if session.actor != actor
            || session.attempt != attempt
            || session.learner_work_binding != learner_work_binding
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
        learner_work_binding: LearnerWorkRoutingBinding,
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
            || session.learner_work_binding != learner_work_binding
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
        learner_work_binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
        token: &ExternalToolActivityLeaseToken,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        if state
            .indeterminate_external_tool_activities
            .get(&(tenant, attempt))
            != Some(&token.hash())
        {
            return Err(StoreError::Conflict);
        }
        if !state.external_tool_launch_sessions.values().any(|session| {
            session.actor == actor
                && session.attempt == attempt
                && session.learner_work_binding == learner_work_binding
                && session.activity_lease_hash.as_ref() == Some(&token.hash())
        }) {
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
        learner_work_binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolActivityLeaseToken,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let session = state
            .external_tool_launch_sessions
            .get_mut(&(tenant, id))
            .ok_or(StoreError::NotFound)?;
        if session.actor != actor
            || session.attempt != attempt
            || session.learner_work_binding != learner_work_binding
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
        learner_work_binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolLaunchToken,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
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
        if session.actor != actor
            || session.attempt != attempt
            || session.learner_work_binding != learner_work_binding
            || session.token_hash != token.hash()
        {
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

fn prepare_external_tool_attempt_locked(
    state: &State,
    context: TenantContext,
    actor: UserId,
    learner_work_binding: LearnerWorkRoutingBinding,
    attempt_id: QuestionAttemptId,
) -> Result<PreparedExternalToolAttempt, StoreError> {
    let tenant = context.tenant_id();
    // ASVS 8.2.2: establish route-scoped Student authority before resolving an
    // opaque attempt identity or its protected source.
    super::entitlement::active_membership_for(state, tenant, learner_work_binding.course, actor)
        .filter(|membership| {
            membership.role == question_model::CourseMembershipRole::Student
                && membership.student.is_some()
        })
        .ok_or(StoreError::NotFound)?;
    let assignment = assignment_record(state, tenant, learner_work_binding.assignment)?;
    if assignment.course_id != learner_work_binding.course {
        return Err(StoreError::NotFound);
    }
    require_course_records_accessible(state, tenant, learner_work_binding.course)?;
    let domain::entitlement::EntitlementDecision::Granted(grant) =
        super::entitlement::evaluate_locked(
            state,
            tenant,
            actor,
            learner_work_binding.course,
            learner_work_binding.assignment,
        )?
    else {
        return Err(StoreError::NotFound);
    };
    let base = state
        .attempts
        .get(&(tenant, attempt_id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let attempt = projected_attempt(state, tenant, &base);
    let run = state
        .runs
        .get(&(tenant, attempt.run))
        .ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(state, tenant, run.enrollment)?;
    if enrollment.assignment != learner_work_binding.assignment
        || enrollment.user != actor
        || enrollment.student != grant.student()
    {
        return Err(StoreError::NotFound);
    }
    if attempt.status != AttemptStatus::InProgress
        || run.completed_at.is_some()
        || run.score.is_some()
    {
        return Err(StoreError::Conflict);
    }
    let issued_question_snapshot = state
        .attempt_issued_question_snapshots
        .get(&(tenant, attempt.id))
        .cloned()
        .ok_or_else(|| {
            StoreError::Unavailable("issued question snapshot is missing".to_string())
        })?;
    let crate::IssuedQuestionFamilyWitnessV1::External { .. } =
        issued_question_snapshot.family_witness()
    else {
        return Err(StoreError::Conflict);
    };
    Ok(PreparedExternalToolAttempt {
        attempt,
        issued_question_snapshot,
    })
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
