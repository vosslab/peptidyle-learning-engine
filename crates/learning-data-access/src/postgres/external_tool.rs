//! PostgreSQL persistence for contracted external learning-tool exchanges.

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{
    ActivityTimestamp, AttemptResult, ObjectId, ProblemId, QuestionAttempt, QuestionAttemptId,
    StudentResponse, TenantId, UserId, VersionId,
};
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::{
    PostgresStore, database_timestamp, load_attempt_for_external_update, load_published_record,
    load_submission_replay, map_sqlx_error, require_attempt_owner, submit_question_attempt,
};
use crate::{
    BeginExternalToolGradeCommand, CommitExternalToolSubmissionCommand,
    CommitVerifiedExternalToolSubmissionCommand, CreateExternalToolLaunchSessionCommand,
    CreatedExternalToolLaunchSession, ExternalToolBegin, ExternalToolBinding,
    ExternalToolBrokerStore, ExternalToolLaunchProof, ExternalToolLaunchSessionStore,
    ExternalToolLaunchToken, ExternalToolLease, ExternalToolLeaseToken,
    ExternalToolVerifiedPending, FeedbackContent, ResolvedExternalToolLaunchSession,
    StageExternalToolVerificationCommand, StoreError, SubmissionRecord,
    SubmitQuestionAttemptCommand, TenantContext, fresh_external_tool_launch_id,
};

#[async_trait]
impl ExternalToolBrokerStore for PostgresStore {
    async fn begin_or_resume_external_grade(
        &self,
        context: TenantContext,
        command: BeginExternalToolGradeCommand,
    ) -> Result<ExternalToolBegin, StoreError> {
        postgres_validate_external_command(&command)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let base =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        require_attempt_owner(&mut transaction, tenant, base.id, command.actor).await?;
        let published =
            load_published_record(&mut transaction, base.problem, base.question_version).await?;
        postgres_validate_external_binding(&base, &published.question.source, &command.binding)?;
        if let Some(replay) = load_submission_replay(
            &mut transaction,
            tenant,
            base.id,
            &command.response,
            &command.idempotency_key,
        )
        .await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolBegin::Committed(Box::new(replay)));
        }
        let row = sqlx::query(
            "SELECT actor_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, idempotency_key, correlation, state, lease_token, EXTRACT(EPOCH FROM lease_expires_at) * 1000 AS lease_millis, result_payload, result_sha256 FROM external_tool_exchange WHERE tenant_id = $1 AND attempt_id = $2 FOR UPDATE",
        ).bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        if let Some(row) = row {
            let stored = postgres_external_binding(&row)?;
            let actor: Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
            let response_hash: Vec<u8> = row.try_get("response_sha256").map_err(map_sqlx_error)?;
            let key: String = row.try_get("idempotency_key").map_err(map_sqlx_error)?;
            if actor != command.actor.as_uuid()
                || !postgres_binding_matches(&stored, &command.binding)
                || response_hash.as_slice() != command.binding.response_sha256.as_bytes()
                || key != command.idempotency_key.as_str()
            {
                return Err(StoreError::Conflict);
            }
            let state: String = row.try_get("state").map_err(map_sqlx_error)?;
            if state == "committed" {
                return Err(StoreError::Conflict);
            }
            if state == "verified_pending" {
                let payload: Value = row.try_get("result_payload").map_err(map_sqlx_error)?;
                let result: AttemptResult = serde_json::from_value(payload).map_err(|error| {
                    StoreError::InvalidRecord(format!("external result decode failed: {error}"))
                })?;
                let raw: String = row.try_get("result_sha256").map_err(map_sqlx_error)?;
                let bytes = serde_json::to_vec(&result)
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
                if raw != Sha256Digest::compute(&bytes).to_string() {
                    return Err(StoreError::Conflict);
                }
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(ExternalToolBegin::VerifiedPending(
                    ExternalToolVerifiedPending {
                        binding: stored,
                        correlation: crate::PersistedCorrelation::from_stored(
                            row.try_get("correlation").map_err(map_sqlx_error)?,
                        )?,
                        result,
                        result_sha256: Sha256Digest::compute(&bytes),
                    },
                ));
            }
            let token = ExternalToolLeaseToken::generate()?;
            let correlation = crate::PersistedCorrelation::from_stored(
                row.try_get("correlation").map_err(map_sqlx_error)?,
            )?;
            let changed = sqlx::query("UPDATE external_tool_exchange SET lease_token = $3, lease_expires_at = transaction_timestamp() + ($4::bigint * interval '1 millisecond'), updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2 AND state = 'verifying' AND lease_expires_at <= transaction_timestamp()")
                .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(token.bytes().as_slice()).bind(i64::from(command.lease_millis)).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
            if changed.rows_affected() == 0 {
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(ExternalToolBegin::InProgress);
            }
            let now = database_timestamp(&mut transaction).await?;
            let expires_at = ActivityTimestamp::from_unix_millis(
                now.as_unix_millis() + i64::from(command.lease_millis),
            );
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolBegin::Lease(ExternalToolLease {
                binding: command.binding,
                correlation,
                token,
                expires_at,
            }));
        }
        let token = ExternalToolLeaseToken::generate()?;
        sqlx::query("INSERT INTO external_tool_exchange (tenant_id, attempt_id, actor_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, idempotency_key, correlation, state, lease_token, lease_expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,'verifying',$14, transaction_timestamp() + ($15::bigint * interval '1 millisecond'))")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.proposed_correlation.bytes()).bind(token.bytes().as_slice()).bind(i64::from(command.lease_millis)).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        let now = database_timestamp(&mut transaction).await?;
        let expires_at = ActivityTimestamp::from_unix_millis(
            now.as_unix_millis() + i64::from(command.lease_millis),
        );
        transaction.commit().await.map_err(map_sqlx_error)?;
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
        postgres_validate_external_response(&command.response, &command.binding)?;
        crate::validate_attempt_result(command.result)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let base =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        require_attempt_owner(&mut transaction, tenant, base.id, command.actor).await?;
        let published =
            load_published_record(&mut transaction, base.problem, base.question_version).await?;
        postgres_validate_external_binding(&base, &published.question.source, &command.binding)?;
        let payload = serde_json::to_value(command.result)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let raw = serde_json::to_vec(&command.result)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let changed = sqlx::query("UPDATE external_tool_exchange SET state = 'verified_pending', lease_token = NULL, lease_expires_at = NULL, verification_token_sha256 = $17, result_payload = $9, result_sha256 = $10, updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2 AND actor_id = $3 AND provider = $4 AND problem_id = $5 AND version_id = $6 AND seed = $7 AND source_object_id = $8 AND source_sha256 = $11 AND integration_profile = $12 AND response_sha256 = $13 AND idempotency_key = $14 AND correlation = $15 AND state = 'verifying' AND lease_token = $16 AND lease_expires_at > transaction_timestamp()")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(payload).bind(Sha256Digest::compute(&raw).to_string()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.correlation.bytes()).bind(command.lease_token.bytes().as_slice()).bind(command.lease_token.hash().as_bytes().as_slice()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn commit_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        postgres_validate_external_response(&command.response, &command.binding)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let base =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        require_attempt_owner(&mut transaction, tenant, base.id, command.actor).await?;
        let published =
            load_published_record(&mut transaction, base.problem, base.question_version).await?;
        postgres_validate_external_binding(&base, &published.question.source, &command.binding)?;
        if let Some(replay) = load_submission_replay(
            &mut transaction,
            tenant,
            base.id,
            &command.response,
            &command.idempotency_key,
        )
        .await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(replay);
        }
        validate_and_lock_external_launch(
            &mut transaction,
            tenant,
            command.actor,
            command.attempt,
            &command.binding,
            &command.launch_proof,
        )
        .await?;
        let row = sqlx::query("SELECT result_payload, result_sha256, verification_token_sha256 FROM external_tool_exchange WHERE tenant_id = $1 AND attempt_id = $2 AND actor_id = $3 AND provider = $4 AND problem_id = $5 AND version_id = $6 AND seed = $7 AND source_object_id = $8 AND source_sha256 = $9 AND integration_profile = $10 AND response_sha256 = $11 AND idempotency_key = $12 AND correlation = $13 AND state = 'verified_pending' FOR UPDATE")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.correlation.bytes()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?.ok_or(StoreError::Conflict)?;
        let payload: Value = row.try_get("result_payload").map_err(map_sqlx_error)?;
        let result: AttemptResult = serde_json::from_value(payload).map_err(|error| {
            StoreError::InvalidRecord(format!("external result decode failed: {error}"))
        })?;
        let expected_hash: Vec<u8> = row
            .try_get("verification_token_sha256")
            .map_err(map_sqlx_error)?;
        if expected_hash.as_slice() != command.lease_token.hash().as_bytes() {
            return Err(StoreError::Conflict);
        }
        let stored_result_hash: String = row.try_get("result_sha256").map_err(map_sqlx_error)?;
        let encoded_result = serde_json::to_vec(&result)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        if stored_result_hash != Sha256Digest::compute(&encoded_result).to_string() {
            return Err(StoreError::Conflict);
        }
        let record = submit_question_attempt(
            &mut transaction,
            context,
            SubmitQuestionAttemptCommand {
                actor: command.actor,
                attempt: command.attempt,
                response: command.response,
                result,
                feedback: FeedbackContent::default(),
                idempotency_key: command.idempotency_key,
            },
        )
        .await?;
        sqlx::query("UPDATE external_tool_exchange SET state = 'committed', verification_token_sha256 = NULL, updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2").bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        revoke_locked_external_launch(&mut transaction, tenant, command.launch_proof.session_id)
            .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn commit_verified_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitVerifiedExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        postgres_validate_external_response(&command.response, &command.binding)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let base =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        require_attempt_owner(&mut transaction, tenant, base.id, command.actor).await?;
        let published =
            load_published_record(&mut transaction, base.problem, base.question_version).await?;
        postgres_validate_external_binding(&base, &published.question.source, &command.binding)?;
        if let Some(replay) = load_submission_replay(
            &mut transaction,
            tenant,
            base.id,
            &command.response,
            &command.idempotency_key,
        )
        .await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(replay);
        }
        validate_and_lock_external_launch(
            &mut transaction,
            tenant,
            command.actor,
            command.attempt,
            &command.binding,
            &command.launch_proof,
        )
        .await?;
        let row = sqlx::query("SELECT result_payload, result_sha256 FROM external_tool_exchange WHERE tenant_id = $1 AND attempt_id = $2 AND actor_id = $3 AND provider = $4 AND problem_id = $5 AND version_id = $6 AND seed = $7 AND source_object_id = $8 AND source_sha256 = $9 AND integration_profile = $10 AND response_sha256 = $11 AND idempotency_key = $12 AND correlation = $13 AND state = 'verified_pending' FOR UPDATE")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.correlation.bytes()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?.ok_or(StoreError::Conflict)?;
        let payload: Value = row.try_get("result_payload").map_err(map_sqlx_error)?;
        let result: AttemptResult = serde_json::from_value(payload).map_err(|error| {
            StoreError::InvalidRecord(format!("external result decode failed: {error}"))
        })?;
        let stored_result_hash: String = row.try_get("result_sha256").map_err(map_sqlx_error)?;
        let encoded_result = serde_json::to_vec(&result)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        if stored_result_hash != Sha256Digest::compute(&encoded_result).to_string() {
            return Err(StoreError::Conflict);
        }
        let record = submit_question_attempt(
            &mut transaction,
            context,
            SubmitQuestionAttemptCommand {
                actor: command.actor,
                attempt: command.attempt,
                response: command.response,
                result,
                feedback: FeedbackContent::default(),
                idempotency_key: command.idempotency_key,
            },
        )
        .await?;
        sqlx::query("UPDATE external_tool_exchange SET state = 'committed', verification_token_sha256 = NULL, updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        revoke_locked_external_launch(&mut transaction, tenant, command.launch_proof.session_id)
            .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[async_trait]
impl ExternalToolLaunchSessionStore for PostgresStore {
    async fn create_external_tool_launch_session(
        &self,
        context: TenantContext,
        command: CreateExternalToolLaunchSessionCommand,
    ) -> Result<CreatedExternalToolLaunchSession, StoreError> {
        postgres_validate_external_response(&StudentResponse::ExternalTool {}, &command.binding)?;
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
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let attempt =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        require_attempt_owner(&mut transaction, tenant, attempt.id, command.actor).await?;
        let published =
            load_published_record(&mut transaction, attempt.problem, attempt.question_version)
                .await?;
        postgres_validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        let id = fresh_external_tool_launch_id()?;
        let token = ExternalToolLaunchToken::generate()?;
        let row = sqlx::query("INSERT INTO external_tool_launch_session (launch_session_id, tenant_id, attempt_id, actor_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256, encrypted_provider_state, expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,transaction_timestamp() + ($15::bigint * interval '1 millisecond')) RETURNING EXTRACT(EPOCH FROM expires_at) * 1000 AS expires_millis")
            .bind(id).bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(token.hash().as_bytes().as_slice()).bind(command.encrypted_provider_state).bind(i64::from(command.lifetime_millis)).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        let expires: f64 = row.try_get("expires_millis").map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CreatedExternalToolLaunchSession {
            id,
            token,
            expires_at: ActivityTimestamp::from_unix_millis(expires as i64),
        })
    }

    async fn resolve_external_tool_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolLaunchToken,
    ) -> Result<Option<ResolvedExternalToolLaunchSession>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let base = load_attempt_for_external_update(&mut transaction, tenant, attempt).await?;
        require_attempt_owner(&mut transaction, tenant, base.id, actor).await?;
        let row = sqlx::query("SELECT provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256, encrypted_provider_state FROM external_tool_launch_session WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND revoked_at IS NULL AND expires_at > transaction_timestamp()")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let hash: Vec<u8> = row.try_get("token_sha256").map_err(map_sqlx_error)?;
        if hash.as_slice() != token.hash().as_bytes() {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let binding = postgres_external_binding(&row)?;
        let published =
            load_published_record(&mut transaction, base.problem, base.question_version).await?;
        postgres_validate_external_binding(&base, &published.question.source, &binding)?;
        let encrypted_provider_state = row
            .try_get("encrypted_provider_state")
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(ResolvedExternalToolLaunchSession {
            binding,
            encrypted_provider_state,
        }))
    }

    async fn revoke_external_tool_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let changed = sqlx::query("UPDATE external_tool_launch_session SET revoked_at = transaction_timestamp() WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND revoked_at IS NULL").bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            return Err(StoreError::NotFound);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

fn postgres_validate_external_command(
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

fn postgres_validate_external_binding(
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

fn postgres_validate_external_response(
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

async fn validate_and_lock_external_launch(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    attempt: QuestionAttemptId,
    binding: &ExternalToolBinding,
    proof: &ExternalToolLaunchProof,
) -> Result<(), StoreError> {
    let row = sqlx::query(
        "SELECT provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256 \
         FROM external_tool_launch_session \
         WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 \
           AND revoked_at IS NULL AND expires_at > transaction_timestamp() FOR UPDATE",
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
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

async fn revoke_locked_external_launch(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    session_id: Uuid,
) -> Result<(), StoreError> {
    let changed = sqlx::query(
        "UPDATE external_tool_launch_session SET revoked_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND launch_session_id = $2 AND revoked_at IS NULL",
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

fn postgres_external_binding(row: &PgRow) -> Result<ExternalToolBinding, StoreError> {
    let response: Vec<u8> = row.try_get("response_sha256").map_err(map_sqlx_error)?;
    let response: [u8; 32] = response.try_into().map_err(|_| {
        StoreError::InvalidRecord("stored external response checksum is malformed".to_string())
    })?;
    Ok(ExternalToolBinding {
        provider: row.try_get("provider").map_err(map_sqlx_error)?,
        problem: ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?),
        version: VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?),
        seed: row.try_get::<i64, _>("seed").map_err(map_sqlx_error)? as u64,
        source_object: ObjectId::from_uuid(
            row.try_get("source_object_id").map_err(map_sqlx_error)?,
        ),
        source_sha256: row.try_get("source_sha256").map_err(map_sqlx_error)?,
        integration_profile: row.try_get("integration_profile").map_err(map_sqlx_error)?,
        response_sha256: Sha256Digest::from_bytes(response),
    })
}

fn postgres_binding_matches(stored: &ExternalToolBinding, requested: &ExternalToolBinding) -> bool {
    stored.provider == requested.provider
        && stored.problem == requested.problem
        && stored.version == requested.version
        && stored.seed == requested.seed
        && stored.source_object == requested.source_object
        && stored.source_sha256 == requested.source_sha256
        && stored.integration_profile == requested.integration_profile
}
