//! Authenticated one-use iMathAS submission and durable recovery.

use adapter_imathas::broker_provider::{
    ContractedLaunchExpectation, ContractedScoredEmbedProvider, PersistedContractedLaunchSession,
    ScoredEmbedTransport,
};
use async_trait::async_trait;
use learning_data_access::{
    AuthoritativeTimeStore, BeginExternalToolGradeCommand, CatalogSourceStore,
    CommitExternalToolSubmissionCommand, CommitVerifiedExternalToolSubmissionCommand,
    ExternalToolBegin, ExternalToolBrokerStore, ExternalToolLaunchProof,
    ExternalToolLaunchSessionStore, StageExternalToolVerificationCommand, SubmissionIdempotencyKey,
    TenantContext,
};
use objects::ObjectStore;
use question_model::{
    ProblemVersionRef, QuestionAttempt, QuestionDefinition, StudentResponse, UserId,
};

use super::{
    EXTERNAL_TOOL_LEASE_MILLIS, ExternalToolSubmissionBackend, ImathasBackend, LaunchStateAead,
    RunBackendError, SubmissionDisposition, launch_state_aad, map_adapter_error, map_store_error,
};

#[async_trait]
impl<S, O, T> ExternalToolSubmissionBackend
    for ImathasBackend<S, O, ContractedScoredEmbedProvider<T>>
where
    S: CatalogSourceStore
        + ExternalToolBrokerStore
        + ExternalToolLaunchSessionStore
        + AuthoritativeTimeStore
        + Send
        + Sync
        + 'static,
    O: ObjectStore + Send + Sync + 'static,
    T: ScoredEmbedTransport + 'static,
{
    async fn submit_external_tool(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        idempotency_key: SubmissionIdempotencyKey,
        launch_proof: ExternalToolLaunchProof,
        state_aead: &LaunchStateAead,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        self.reproduce_issued_attempt(context, reference, question, attempt)
            .await?;
        let (_source, artifact) = self.resolve_source(context, reference, question).await?;
        let response = StudentResponse::ExternalTool {};
        let binding = Self::binding(question, attempt, &artifact, &response)?;
        let grade_binding = Self::correlation_binding(context, attempt);
        // Claim before resolving the one-use proof, so replays and replica races
        // never reach the provider.
        let begin = self
            .sources
            .begin_or_resume_external_grade(
                context,
                BeginExternalToolGradeCommand {
                    actor,
                    attempt: attempt.id,
                    response: response.clone(),
                    idempotency_key: idempotency_key.clone(),
                    binding: binding.clone(),
                    proposed_correlation: self.persisted_correlation(grade_binding)?,
                    lease_millis: EXTERNAL_TOOL_LEASE_MILLIS,
                },
            )
            .await
            .map_err(map_store_error)?;
        match begin {
            ExternalToolBegin::Committed(record) => Ok(SubmissionDisposition::Committed(record)),
            ExternalToolBegin::InProgress => Err(RunBackendError::Unavailable(
                "external-tool verification is in progress".into(),
            )),
            ExternalToolBegin::VerifiedPending(pending) => self
                .sources
                .commit_verified_external_tool_submission(
                    context,
                    CommitVerifiedExternalToolSubmissionCommand {
                        actor,
                        attempt: attempt.id,
                        response,
                        idempotency_key,
                        binding: pending.binding,
                        correlation: pending.correlation,
                        launch_proof,
                    },
                )
                .await
                .map(|record| SubmissionDisposition::Committed(Box::new(record)))
                .map_err(map_store_error),
            ExternalToolBegin::Lease(lease) => {
                let resolved = self
                    .sources
                    .resolve_external_tool_launch_session(
                        context,
                        actor,
                        attempt.id,
                        launch_proof.session_id,
                        &launch_proof.token,
                    )
                    .await
                    .map_err(map_store_error)?
                    .ok_or_else(|| {
                        RunBackendError::Invalid("external-tool launch is unavailable".into())
                    })?;
                if resolved.binding != binding {
                    return Err(RunBackendError::Invalid(
                        "external-tool launch binding is invalid".into(),
                    ));
                }
                let encrypted = resolved.encrypted_provider_state.ok_or_else(|| {
                    RunBackendError::Invalid("external-tool launch state is unavailable".into())
                })?;
                let aad = launch_state_aad(context, actor, attempt, &binding);
                let plain = state_aead.open(&encrypted, &aad)?;
                let text = std::str::from_utf8(&plain).map_err(|_| {
                    RunBackendError::Invalid("external-tool launch state is invalid".into())
                })?;
                let persisted = PersistedContractedLaunchSession::from_storage_value(text)
                    .map_err(map_adapter_error)?;
                let expectation = ContractedLaunchExpectation::new(
                    grade_binding,
                    binding.provider.clone(),
                    binding.source_sha256.clone(),
                )
                .map_err(map_adapter_error)?;
                let mut session = state_aead
                    .adapter_codec
                    .restore(&persisted, &expectation)
                    .map_err(map_adapter_error)?;
                let now = self
                    .sources
                    .authoritative_time(context)
                    .await
                    .map_err(map_store_error)?;
                let receipt = self
                    .adapter
                    .retrieve_contracted_grade(&mut session, now)
                    .await
                    .map_err(map_adapter_error)?;
                if receipt.binding() != grade_binding {
                    return Err(RunBackendError::Invalid(
                        "iMathAS verifier returned an incorrectly bound result".into(),
                    ));
                }
                self.sources
                    .stage_external_tool_verification(
                        context,
                        StageExternalToolVerificationCommand {
                            actor,
                            attempt: attempt.id,
                            response: response.clone(),
                            idempotency_key: idempotency_key.clone(),
                            binding: lease.binding.clone(),
                            correlation: lease.correlation.clone(),
                            lease_token: lease.token.clone(),
                            result: receipt.result(),
                        },
                    )
                    .await
                    .map_err(map_store_error)?;
                self.sources
                    .commit_external_tool_submission(
                        context,
                        CommitExternalToolSubmissionCommand {
                            actor,
                            attempt: attempt.id,
                            response,
                            idempotency_key,
                            binding: lease.binding,
                            correlation: lease.correlation,
                            lease_token: lease.token,
                            launch_proof,
                        },
                    )
                    .await
                    .map(|record| SubmissionDisposition::Committed(Box::new(record)))
                    .map_err(map_store_error)
            }
        }
    }
}
