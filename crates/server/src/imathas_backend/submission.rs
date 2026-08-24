//! Authenticated one-use iMathAS submission and durable recovery.

use adapter_imathas::broker_provider::{
    ContractedLaunchExpectation, ContractedScoredEmbedProvider, PersistedContractedLaunchSession,
    ScoredEmbedTransport,
};
use async_trait::async_trait;
use learning_data_access::{
    AuthoritativeTimeStore, BeginExternalToolGradeCommand, CatalogSourceStore,
    ClaimExternalToolFinalizationActivityCommand, CommitExternalToolSubmissionCommand,
    CommitVerifiedExternalToolSubmissionCommand, ExternalToolActivityClaim, ExternalToolBegin,
    ExternalToolBrokerStore, ExternalToolLaunchProof, ExternalToolLaunchSessionStore,
    LearnerWorkRoutingBinding, StageExternalToolVerificationCommand, SubmissionIdempotencyKey,
    TenantContext,
};
use objects::ObjectStore;
use question_model::{QuestionAttempt, StudentResponse, UserId};

use super::{
    ExternalToolSubmissionBackend, ImathasBackend, LaunchStateAead, RunBackendError,
    SubmissionDisposition, launch_state_aad, map_adapter_error, map_store_error,
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
        learner_work_binding: LearnerWorkRoutingBinding,
        issued_question_snapshot: &learning_data_access::IssuedQuestionSnapshotV1,
        attempt: &QuestionAttempt,
        idempotency_key: SubmissionIdempotencyKey,
        launch_proof: ExternalToolLaunchProof,
        state_aead: &LaunchStateAead,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        let response = StudentResponse::ExternalTool {};
        let binding = Self::binding(issued_question_snapshot, attempt, &response)?;
        // A first finalization revalidates the captured object before any
        // broker mutation. Exact committed replays are selected by the route
        // before this backend boundary, so they never hydrate this source.
        let _source = self
            .resolve_issued_source(attempt, issued_question_snapshot)
            .await?;
        let grade_binding = Self::correlation_binding(context, attempt);
        // Claim before resolving the one-use proof, so replays and replica races
        // never reach the provider.
        let begin = self
            .sources
            .begin_or_resume_external_grade(
                context,
                BeginExternalToolGradeCommand {
                    actor,
                    learner_work_binding,
                    attempt: attempt.id,
                    response: response.clone(),
                    idempotency_key: idempotency_key.clone(),
                    binding: binding.clone(),
                    proposed_correlation: self.persisted_correlation(grade_binding)?,
                    lease_millis: self.timing.verification_lease_millis(),
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
                        learner_work_binding,
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
                let activity_claim = self
                    .sources
                    .claim_external_tool_finalization_activity(
                        context,
                        ClaimExternalToolFinalizationActivityCommand {
                            actor,
                            learner_work_binding,
                            attempt: attempt.id,
                            id: launch_proof.session_id,
                            token: launch_proof.token.clone(),
                            verification_lease: lease.token.clone(),
                            lease_millis: self.timing.activity_lease_millis(),
                        },
                    )
                    .await
                    .map_err(map_store_error)?;
                let activity_lease = match activity_claim {
                    ExternalToolActivityClaim::Unavailable => {
                        return Err(RunBackendError::Invalid(
                            "external-tool launch is unavailable".into(),
                        ));
                    }
                    ExternalToolActivityClaim::InProgress => {
                        return Err(RunBackendError::Unavailable(
                            "external-tool activity is in progress; retry shortly".into(),
                        ));
                    }
                    ExternalToolActivityClaim::Lease(lease) => lease,
                };
                let verification = async {
                    if activity_lease.binding != binding {
                        return Err(RunBackendError::Invalid(
                            "external-tool launch binding is invalid".into(),
                        ));
                    }
                    let encrypted = activity_lease
                        .encrypted_provider_state
                        .as_ref()
                        .ok_or_else(|| {
                            RunBackendError::Invalid(
                                "external-tool launch state is unavailable".into(),
                            )
                        })?;
                    let aad =
                        launch_state_aad(context, actor, learner_work_binding, attempt, &binding);
                    let plain = state_aead.open(encrypted, &aad)?;
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
                    Ok::<_, RunBackendError>(receipt)
                }
                .await;
                let release = self
                    .sources
                    .release_external_tool_activity(
                        context,
                        actor,
                        learner_work_binding,
                        attempt.id,
                        launch_proof.session_id,
                        &activity_lease.token,
                    )
                    .await
                    .map_err(map_store_error);
                release?;
                let receipt = verification?;
                self.sources
                    .stage_external_tool_verification(
                        context,
                        StageExternalToolVerificationCommand {
                            actor,
                            learner_work_binding,
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
                            learner_work_binding,
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
