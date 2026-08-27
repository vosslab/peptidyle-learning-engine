//! PostgreSQL persistence for contracted external learning-tool exchanges.

mod activity_state;
mod validation;

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{
    ActivityTimestamp, AttemptResult, QuestionAttemptId, StudentResponse, UserId,
};
use serde_json::Value;
use sqlx::Row;
use sqlx::types::Uuid;

use super::{PostgresStore, database_timestamp, map_sqlx_error, submit_question_attempt};
use crate::{
    BeginExternalToolGradeCommand, ClaimExternalToolFinalizationActivityCommand,
    ClaimedExternalToolActivity, CommitExternalToolSubmissionCommand,
    CommitVerifiedExternalToolSubmissionCommand, CreateExternalToolLaunchSessionCommand,
    CreatedExternalToolLaunchSession, ExternalToolActivityClaim, ExternalToolActivityLeaseToken,
    ExternalToolBegin, ExternalToolBrokerStore, ExternalToolLaunchSessionStore,
    ExternalToolLaunchToken, ExternalToolLease, ExternalToolLeaseToken,
    ExternalToolVerifiedPending, FeedbackContent, LearnerWorkRoutingBinding,
    PreparedExternalToolAttempt, StageExternalToolVerificationCommand, StoreError,
    SubmissionRecord, SubmitQuestionAttemptCommand, TenantContext, fresh_external_tool_launch_id,
    validate_external_snapshot_binding,
};
use activity_state::{
    postgres_external_activity_is_indeterminate, postgres_finalization_blocks_ordinary_activity,
    postgres_finalization_lease_is_current, postgres_has_live_external_activity,
    postgres_validate_external_activity_lease_millis, postgres_validate_external_command,
    prepare_external_student_attempt, revoke_locked_external_launch,
    validate_active_external_attempt, validate_and_lock_external_launch,
};
use validation::{
    postgres_binding_matches, postgres_external_binding, postgres_stored_course_matches,
    postgres_validate_external_response,
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
        let prepared = prepare_external_student_attempt(
            &mut transaction,
            tenant,
            command.learner_work_binding,
            command.actor,
            command.attempt,
            &command.binding,
        )
        .await?;
        if postgres_external_activity_is_indeterminate(
            &mut transaction,
            tenant,
            prepared.attempt.id,
        )
        .await?
        {
            return Err(StoreError::Conflict);
        }
        match super::submission_preparation::prepared_submission_replay(
            &mut transaction,
            tenant,
            &command.response,
            &command.idempotency_key,
            &prepared,
        )
        .await?
        {
            crate::SubmissionReceiptRead::Completed(replay) => {
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(ExternalToolBegin::Committed(replay));
            }
            crate::SubmissionReceiptRead::AcceptedPending(_) => return Err(StoreError::Conflict),
            crate::SubmissionReceiptRead::Missing => {}
        }
        let row = sqlx::query(
            "SELECT actor_id, course_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, idempotency_key, correlation, state, lease_token, EXTRACT(EPOCH FROM lease_expires_at) * 1000 AS lease_millis, result_payload, result_sha256 FROM external_tool_exchange WHERE tenant_id = $1 AND attempt_id = $2 FOR UPDATE",
        ).bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        if let Some(row) = row {
            let stored = postgres_external_binding(&row)?;
            let actor: Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
            let response_hash: Vec<u8> = row.try_get("response_sha256").map_err(map_sqlx_error)?;
            let key: String = row.try_get("idempotency_key").map_err(map_sqlx_error)?;
            if actor != command.actor.as_uuid()
                || !postgres_stored_course_matches(&row, command.learner_work_binding)?
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
            if postgres_has_live_external_activity(&mut transaction, tenant, command.attempt)
                .await?
            {
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(ExternalToolBegin::InProgress);
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
        if postgres_has_live_external_activity(&mut transaction, tenant, command.attempt).await? {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolBegin::InProgress);
        }
        let token = ExternalToolLeaseToken::generate()?;
        sqlx::query("INSERT INTO external_tool_exchange (tenant_id, attempt_id, actor_id, course_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, idempotency_key, correlation, state, lease_token, lease_expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,'verifying',$15, transaction_timestamp() + ($16::bigint * interval '1 millisecond'))")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(command.learner_work_binding.course.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.proposed_correlation.bytes()).bind(token.bytes().as_slice()).bind(i64::from(command.lease_millis)).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
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
        let payload = serde_json::to_value(command.result)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let raw = serde_json::to_vec(&command.result)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        // ASVS 2.3.1/8.2.2: this is lease-bound provider evidence cleanup,
        // not a new learner-work effect. It may close only the exact stored
        // operation after remote I/O; a later ordinary submission reauthorizes.
        let changed = sqlx::query("UPDATE external_tool_exchange AS exchange SET state = 'verified_pending', lease_token = NULL, lease_expires_at = NULL, verification_token_sha256 = $17, result_payload = $9, result_sha256 = $10, updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2 AND actor_id = $3 AND provider = $4 AND problem_id = $5 AND version_id = $6 AND seed = $7 AND source_object_id = $8 AND source_sha256 = $11 AND integration_profile = $12 AND response_sha256 = $13 AND idempotency_key = $14 AND correlation = $15 AND state = 'verifying' AND lease_token = $16 AND lease_expires_at > transaction_timestamp() AND course_id = $18 AND EXISTS (SELECT 1 FROM question_attempt AS attempt JOIN assignment_run AS run ON run.tenant_id=attempt.tenant_id AND run.run_id=attempt.run_id JOIN enrollment AS enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id WHERE attempt.tenant_id=$1 AND attempt.attempt_id=$2 AND enrollment.course_id=$18 AND enrollment.assignment_id=$19)")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(payload).bind(Sha256Digest::compute(&raw).to_string()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.correlation.bytes()).bind(command.lease_token.bytes().as_slice()).bind(command.lease_token.hash().as_bytes().as_slice()).bind(command.learner_work_binding.course.as_uuid()).bind(command.learner_work_binding.assignment.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
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
        let prepared = prepare_external_student_attempt(
            &mut transaction,
            tenant,
            command.learner_work_binding,
            command.actor,
            command.attempt,
            &command.binding,
        )
        .await?;
        match super::submission_preparation::prepared_submission_replay(
            &mut transaction,
            tenant,
            &command.response,
            &command.idempotency_key,
            &prepared,
        )
        .await?
        {
            crate::SubmissionReceiptRead::Completed(replay) => {
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(*replay);
            }
            crate::SubmissionReceiptRead::AcceptedPending(_) => return Err(StoreError::Conflict),
            crate::SubmissionReceiptRead::Missing => {}
        }
        validate_and_lock_external_launch(
            &mut transaction,
            tenant,
            command.actor,
            command.learner_work_binding,
            command.attempt,
            &command.binding,
            &command.launch_proof,
        )
        .await?;
        let row = sqlx::query("SELECT result_payload, result_sha256, verification_token_sha256 FROM external_tool_exchange WHERE tenant_id = $1 AND attempt_id = $2 AND actor_id = $3 AND provider = $4 AND problem_id = $5 AND version_id = $6 AND seed = $7 AND source_object_id = $8 AND source_sha256 = $9 AND integration_profile = $10 AND response_sha256 = $11 AND idempotency_key = $12 AND correlation = $13 AND state = 'verified_pending' AND course_id = $14 FOR UPDATE")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.correlation.bytes()).bind(command.learner_work_binding.course.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?.ok_or(StoreError::Conflict)?;
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
                binding: command.learner_work_binding,
                attempt: command.attempt,
                response: command.response,
                result,
                feedback: FeedbackContent::default(),
                idempotency_key: command.idempotency_key,
            },
        )
        .await?;
        sqlx::query("UPDATE external_tool_exchange SET state = 'committed', verification_token_sha256 = NULL, updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2 AND course_id = $3").bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.learner_work_binding.course.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
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
        let prepared = prepare_external_student_attempt(
            &mut transaction,
            tenant,
            command.learner_work_binding,
            command.actor,
            command.attempt,
            &command.binding,
        )
        .await?;
        match super::submission_preparation::prepared_submission_replay(
            &mut transaction,
            tenant,
            &command.response,
            &command.idempotency_key,
            &prepared,
        )
        .await?
        {
            crate::SubmissionReceiptRead::Completed(replay) => {
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(*replay);
            }
            crate::SubmissionReceiptRead::AcceptedPending(_) => return Err(StoreError::Conflict),
            crate::SubmissionReceiptRead::Missing => {}
        }
        validate_and_lock_external_launch(
            &mut transaction,
            tenant,
            command.actor,
            command.learner_work_binding,
            command.attempt,
            &command.binding,
            &command.launch_proof,
        )
        .await?;
        let row = sqlx::query("SELECT result_payload, result_sha256 FROM external_tool_exchange WHERE tenant_id = $1 AND attempt_id = $2 AND actor_id = $3 AND provider = $4 AND problem_id = $5 AND version_id = $6 AND seed = $7 AND source_object_id = $8 AND source_sha256 = $9 AND integration_profile = $10 AND response_sha256 = $11 AND idempotency_key = $12 AND correlation = $13 AND state = 'verified_pending' AND course_id = $14 FOR UPDATE")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.correlation.bytes()).bind(command.learner_work_binding.course.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?.ok_or(StoreError::Conflict)?;
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
                binding: command.learner_work_binding,
                attempt: command.attempt,
                response: command.response,
                result,
                feedback: FeedbackContent::default(),
                idempotency_key: command.idempotency_key,
            },
        )
        .await?;
        sqlx::query("UPDATE external_tool_exchange SET state = 'committed', verification_token_sha256 = NULL, updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2 AND course_id = $3")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.learner_work_binding.course.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        revoke_locked_external_launch(&mut transaction, tenant, command.launch_proof.session_id)
            .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[async_trait]
impl ExternalToolLaunchSessionStore for PostgresStore {
    async fn prepare_external_tool_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        learner_work_binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
    ) -> Result<PreparedExternalToolAttempt, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        // ASVS 2.3.1/2.3.3/8.2.2: route authority precedes every protected
        // attempt or published-source read and is retained through hydration.
        let prepared = super::submission_preparation::prepare_bound_student_attempt(
            &mut transaction,
            tenant,
            learner_work_binding,
            actor,
            attempt,
        )
        .await?;
        validate_active_external_attempt(&prepared.attempt, &prepared.run)?;
        if !matches!(
            prepared.issued_question_snapshot.family_witness(),
            crate::IssuedQuestionFamilyWitnessV1::External { .. }
        ) {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(PreparedExternalToolAttempt {
            attempt: prepared.attempt,
            issued_question_snapshot: prepared.issued_question_snapshot,
        })
    }

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
        let prepared = prepare_external_student_attempt(
            &mut transaction,
            tenant,
            command.learner_work_binding,
            command.actor,
            command.attempt,
            &command.binding,
        )
        .await?;
        let attempt = &prepared.attempt;
        if postgres_external_activity_is_indeterminate(&mut transaction, tenant, attempt.id).await?
        {
            return Err(StoreError::Conflict);
        }
        let id = fresh_external_tool_launch_id()?;
        let token = ExternalToolLaunchToken::generate()?;
        let row = sqlx::query("INSERT INTO external_tool_launch_session (launch_session_id, tenant_id, attempt_id, actor_id, course_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256, encrypted_provider_state, expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,transaction_timestamp() + ($16::bigint * interval '1 millisecond')) RETURNING EXTRACT(EPOCH FROM expires_at) * 1000 AS expires_millis")
            .bind(id).bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(command.learner_work_binding.course.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(token.hash().as_bytes().as_slice()).bind(command.encrypted_provider_state).bind(i64::from(command.lifetime_millis)).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        let expires: f64 = row.try_get("expires_millis").map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CreatedExternalToolLaunchSession {
            id,
            token,
            expires_at: ActivityTimestamp::from_unix_millis(expires as i64),
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
        postgres_validate_external_activity_lease_millis(lease_millis)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let prepared = super::submission_preparation::prepare_bound_student_attempt(
            &mut transaction,
            tenant,
            learner_work_binding,
            actor,
            attempt,
        )
        .await?;
        validate_active_external_attempt(&prepared.attempt, &prepared.run)?;
        if postgres_external_activity_is_indeterminate(
            &mut transaction,
            tenant,
            prepared.attempt.id,
        )
        .await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        let row = sqlx::query("SELECT course_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256, encrypted_provider_state, EXTRACT(EPOCH FROM activity_lease_expires_at) * 1000 AS activity_lease_millis FROM external_tool_launch_session WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND revoked_at IS NULL AND expires_at > transaction_timestamp() FOR UPDATE")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::Unavailable);
        };
        let hash: Vec<u8> = row.try_get("token_sha256").map_err(map_sqlx_error)?;
        if hash.as_slice() != token.hash().as_bytes()
            || !postgres_stored_course_matches(&row, learner_work_binding)?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        let now = database_timestamp(&mut transaction).await?;
        let active_expiry: Option<f64> = row
            .try_get("activity_lease_millis")
            .map_err(map_sqlx_error)?;
        if active_expiry.is_some_and(|expiry| expiry as i64 > now.as_unix_millis()) {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::InProgress);
        }
        if postgres_finalization_blocks_ordinary_activity(&mut transaction, tenant, attempt).await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::InProgress);
        }
        let binding = postgres_external_binding(&row)?;
        validate_external_snapshot_binding(
            &prepared.attempt,
            &prepared.issued_question_snapshot,
            &binding,
        )?;
        let encrypted_provider_state = row
            .try_get("encrypted_provider_state")
            .map_err(map_sqlx_error)?;
        let activity_token = ExternalToolActivityLeaseToken::generate()?;
        let expires_at =
            ActivityTimestamp::from_unix_millis(now.as_unix_millis() + i64::from(lease_millis));
        let changed = sqlx::query("UPDATE external_tool_launch_session SET activity_lease_token_sha256 = $5, activity_lease_expires_at = transaction_timestamp() + ($6::bigint * interval '1 millisecond') WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND revoked_at IS NULL AND expires_at > transaction_timestamp() AND (activity_lease_expires_at IS NULL OR activity_lease_expires_at <= transaction_timestamp())")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).bind(activity_token.hash().as_bytes().as_slice()).bind(i64::from(lease_millis)).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::InProgress);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(ExternalToolActivityClaim::Lease(Box::new(
            ClaimedExternalToolActivity {
                binding,
                encrypted_provider_state,
                token: activity_token,
                expires_at,
            },
        )))
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
        postgres_validate_external_activity_lease_millis(lease_millis)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let prepared = super::submission_preparation::prepare_bound_student_attempt(
            &mut transaction,
            tenant,
            learner_work_binding,
            actor,
            attempt,
        )
        .await?;
        validate_active_external_attempt(&prepared.attempt, &prepared.run)?;
        if postgres_external_activity_is_indeterminate(&mut transaction, tenant, attempt).await? {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        let row = sqlx::query("SELECT course_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256, encrypted_provider_state, EXTRACT(EPOCH FROM activity_lease_expires_at) * 1000 AS activity_lease_millis FROM external_tool_launch_session WHERE tenant_id=$1 AND launch_session_id=$2 AND attempt_id=$3 AND actor_id=$4 AND revoked_at IS NULL AND expires_at > transaction_timestamp() FOR UPDATE")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::Unavailable);
        };
        let hash: Vec<u8> = row.try_get("token_sha256").map_err(map_sqlx_error)?;
        if hash.as_slice() != token.hash().as_bytes()
            || !postgres_stored_course_matches(&row, learner_work_binding)?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        let now = database_timestamp(&mut transaction).await?;
        let active_expiry: Option<f64> = row
            .try_get("activity_lease_millis")
            .map_err(map_sqlx_error)?;
        if active_expiry.is_some_and(|expiry| expiry as i64 > now.as_unix_millis())
            || postgres_finalization_blocks_ordinary_activity(&mut transaction, tenant, attempt)
                .await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::InProgress);
        }
        let binding = postgres_external_binding(&row)?;
        validate_external_snapshot_binding(
            &prepared.attempt,
            &prepared.issued_question_snapshot,
            &binding,
        )?;
        let activity_token = ExternalToolActivityLeaseToken::generate()?;
        let changed = sqlx::query("UPDATE external_tool_launch_session SET activity_lease_token_sha256=$5, activity_lease_expires_at=transaction_timestamp() + ($6::bigint * interval '1 millisecond') WHERE tenant_id=$1 AND launch_session_id=$2 AND attempt_id=$3 AND actor_id=$4 AND revoked_at IS NULL AND expires_at > transaction_timestamp() AND (activity_lease_expires_at IS NULL OR activity_lease_expires_at <= transaction_timestamp())")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).bind(activity_token.hash().as_bytes().as_slice()).bind(i64::from(lease_millis)).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::InProgress);
        }
        let fenced = sqlx::query("UPDATE question_attempt SET external_tool_indeterminate_at=transaction_timestamp(), external_tool_indeterminate_token_sha256=$3 WHERE tenant_id=$1 AND attempt_id=$2 AND external_tool_indeterminate_at IS NULL")
            .bind(tenant.as_uuid()).bind(attempt.as_uuid()).bind(activity_token.hash().as_bytes().as_slice()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if fenced.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        let expires_at =
            ActivityTimestamp::from_unix_millis(now.as_unix_millis() + i64::from(lease_millis));
        let encrypted_provider_state = row
            .try_get("encrypted_provider_state")
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(ExternalToolActivityClaim::Lease(Box::new(
            ClaimedExternalToolActivity {
                binding,
                encrypted_provider_state,
                token: activity_token,
                expires_at,
            },
        )))
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
        postgres_validate_external_activity_lease_millis(lease_millis)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let prepared = super::submission_preparation::prepare_bound_student_attempt(
            &mut transaction,
            tenant,
            learner_work_binding,
            actor,
            attempt,
        )
        .await?;
        validate_active_external_attempt(&prepared.attempt, &prepared.run)?;
        if postgres_external_activity_is_indeterminate(
            &mut transaction,
            tenant,
            prepared.attempt.id,
        )
        .await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        let row = sqlx::query("SELECT course_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256, encrypted_provider_state, EXTRACT(EPOCH FROM activity_lease_expires_at) * 1000 AS activity_lease_millis FROM external_tool_launch_session WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND revoked_at IS NULL AND expires_at > transaction_timestamp() FOR UPDATE")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::Unavailable);
        };
        let hash: Vec<u8> = row.try_get("token_sha256").map_err(map_sqlx_error)?;
        if hash.as_slice() != token.hash().as_bytes()
            || !postgres_stored_course_matches(&row, learner_work_binding)?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        if !postgres_finalization_lease_is_current(
            &mut transaction,
            tenant,
            attempt,
            &verification_lease,
        )
        .await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::Unavailable);
        }
        let now = database_timestamp(&mut transaction).await?;
        let active_expiry: Option<f64> = row
            .try_get("activity_lease_millis")
            .map_err(map_sqlx_error)?;
        if active_expiry.is_some_and(|expiry| expiry as i64 > now.as_unix_millis()) {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::InProgress);
        }
        let binding = postgres_external_binding(&row)?;
        validate_external_snapshot_binding(
            &prepared.attempt,
            &prepared.issued_question_snapshot,
            &binding,
        )?;
        let encrypted_provider_state = row
            .try_get("encrypted_provider_state")
            .map_err(map_sqlx_error)?;
        let activity_token = ExternalToolActivityLeaseToken::generate()?;
        let expires_at =
            ActivityTimestamp::from_unix_millis(now.as_unix_millis() + i64::from(lease_millis));
        let changed = sqlx::query("UPDATE external_tool_launch_session SET activity_lease_token_sha256 = $5, activity_lease_expires_at = transaction_timestamp() + ($6::bigint * interval '1 millisecond') WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND revoked_at IS NULL AND expires_at > transaction_timestamp() AND (activity_lease_expires_at IS NULL OR activity_lease_expires_at <= transaction_timestamp())")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).bind(activity_token.hash().as_bytes().as_slice()).bind(i64::from(lease_millis)).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolActivityClaim::InProgress);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(ExternalToolActivityClaim::Lease(Box::new(
            ClaimedExternalToolActivity {
                binding,
                encrypted_provider_state,
                token: activity_token,
                expires_at,
            },
        )))
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
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let changed = sqlx::query("UPDATE external_tool_launch_session AS launch SET activity_lease_token_sha256 = NULL, activity_lease_expires_at = NULL WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND activity_lease_token_sha256 = $5 AND course_id = $6 AND EXISTS (SELECT 1 FROM question_attempt AS attempt JOIN assignment_run AS run ON run.tenant_id=attempt.tenant_id AND run.run_id=attempt.run_id JOIN enrollment AS enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id WHERE attempt.tenant_id=$1 AND attempt.attempt_id=$3 AND enrollment.course_id=$6 AND enrollment.assignment_id=$7)")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).bind(token.hash().as_bytes().as_slice()).bind(learner_work_binding.course.as_uuid()).bind(learner_work_binding.assignment.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
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
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let prepared = super::submission_preparation::prepare_bound_student_attempt(
            &mut transaction,
            tenant,
            learner_work_binding,
            actor,
            attempt,
        )
        .await?;
        validate_active_external_attempt(&prepared.attempt, &prepared.run)?;
        let claimed = sqlx::query("SELECT 1 FROM external_tool_launch_session WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND activity_lease_token_sha256 = $5 AND activity_lease_expires_at > transaction_timestamp() AND course_id = $6 FOR UPDATE")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).bind(token.hash().as_bytes().as_slice()).bind(learner_work_binding.course.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        if claimed.is_none() {
            return Err(StoreError::Conflict);
        }
        let changed = sqlx::query("UPDATE question_attempt SET external_tool_indeterminate_at = transaction_timestamp(), external_tool_indeterminate_token_sha256 = $3 WHERE tenant_id = $1 AND attempt_id = $2 AND external_tool_indeterminate_at IS NULL")
            .bind(tenant.as_uuid()).bind(attempt.as_uuid()).bind(token.hash().as_bytes().as_slice()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn complete_external_tool_activity_dispatch(
        &self,
        context: TenantContext,
        actor: UserId,
        learner_work_binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
        token: &ExternalToolActivityLeaseToken,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let changed = sqlx::query("UPDATE question_attempt SET external_tool_indeterminate_at = NULL, external_tool_indeterminate_token_sha256 = NULL WHERE tenant_id = $1 AND attempt_id = $2 AND external_tool_indeterminate_token_sha256 = $3 AND EXISTS (SELECT 1 FROM external_tool_launch_session AS launch JOIN assignment_run AS run ON run.tenant_id=launch.tenant_id AND run.run_id=question_attempt.run_id JOIN enrollment AS enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id WHERE launch.tenant_id=$1 AND launch.attempt_id=$2 AND launch.actor_id=$4 AND launch.course_id=$5 AND enrollment.course_id=$5 AND enrollment.assignment_id=$6)")
            .bind(tenant.as_uuid()).bind(attempt.as_uuid()).bind(token.hash().as_bytes().as_slice()).bind(actor.as_uuid()).bind(learner_work_binding.course.as_uuid()).bind(learner_work_binding.assignment.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
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
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let changed = sqlx::query("UPDATE external_tool_launch_session AS launch SET activity_lease_token_sha256 = NULL, activity_lease_expires_at = NULL, revoked_at = transaction_timestamp() WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND revoked_at IS NULL AND activity_lease_token_sha256 = $5 AND course_id = $6 AND EXISTS (SELECT 1 FROM question_attempt AS attempt JOIN assignment_run AS run ON run.tenant_id=attempt.tenant_id AND run.run_id=attempt.run_id JOIN enrollment AS enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id WHERE attempt.tenant_id=$1 AND attempt.attempt_id=$3 AND enrollment.course_id=$6 AND enrollment.assignment_id=$7)")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).bind(token.hash().as_bytes().as_slice()).bind(learner_work_binding.course.as_uuid()).bind(learner_work_binding.assignment.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        let fenced = sqlx::query("UPDATE question_attempt SET external_tool_indeterminate_at = COALESCE(external_tool_indeterminate_at, transaction_timestamp()), external_tool_indeterminate_token_sha256 = COALESCE(external_tool_indeterminate_token_sha256, $3) WHERE tenant_id = $1 AND attempt_id = $2")
            .bind(tenant.as_uuid()).bind(attempt.as_uuid()).bind(token.hash().as_bytes().as_slice()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if fenced.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
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
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let changed = sqlx::query("UPDATE external_tool_launch_session AS launch SET revoked_at = transaction_timestamp() WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND course_id=$5 AND token_sha256=$6 AND revoked_at IS NULL AND (activity_lease_expires_at IS NULL OR activity_lease_expires_at <= transaction_timestamp()) AND EXISTS (SELECT 1 FROM question_attempt AS attempt JOIN assignment_run AS run ON run.tenant_id=attempt.tenant_id AND run.run_id=attempt.run_id JOIN enrollment AS enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id WHERE attempt.tenant_id=$1 AND attempt.attempt_id=$3 AND enrollment.course_id=$5 AND enrollment.assignment_id=$7)").bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).bind(learner_work_binding.course.as_uuid()).bind(token.hash().as_bytes().as_slice()).bind(learner_work_binding.assignment.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            return Err(StoreError::NotFound);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }
}
