//! Contracted iMathAS provider dispatch from authenticated launch sessions.

use adapter_imathas::broker_provider::{
    ContractedLaunchExpectation, ContractedScoredEmbedProvider, PersistedContractedLaunchSession,
    ProxyMethod, ProxyResponse, ScoredEmbedTransport,
};
use async_trait::async_trait;
use learning_data_access::{
    AuthoritativeTimeStore, CatalogSourceStore, ExternalToolActivityClaim, ExternalToolBrokerStore,
    ExternalToolLaunchSessionStore, ExternalToolLaunchToken, TenantContext,
};
use objects::ObjectStore;
use question_model::{
    ProblemVersionRef, QuestionAttempt, QuestionDefinition, StudentResponse, UserId,
};

use super::{
    ImathasBackend, LaunchStateAead, RunBackendError, launch_state_aad, map_adapter_error,
    map_store_error,
};

#[async_trait]
impl<S, O, T> crate::run::ExternalToolLaunchBackend
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
    async fn create_external_tool_launch(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        aead: &LaunchStateAead,
    ) -> Result<learning_data_access::CreatedExternalToolLaunchSession, RunBackendError> {
        self.create_contracted_launch_session(context, actor, reference, question, attempt, aead)
            .await
    }

    async fn proxy_external_tool_activity(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        session_id: uuid::Uuid,
        token: &ExternalToolLaunchToken,
        method: ProxyMethod,
        body: &[u8],
        aead: &LaunchStateAead,
    ) -> Result<ProxyResponse, RunBackendError> {
        self.reproduce_issued_attempt(context, reference, question, attempt)
            .await?;
        let (_source, artifact) = self.resolve_source(context, reference, question).await?;
        let expected = Self::binding(
            question,
            attempt,
            &artifact,
            &StudentResponse::ExternalTool {},
        )?;
        let claim = self
            .sources
            .claim_external_tool_activity(
                context,
                actor,
                attempt.id,
                session_id,
                token,
                self.timing.activity_lease_millis(),
            )
            .await
            .map_err(map_store_error)?;
        let lease = match claim {
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
        if matches!(method, ProxyMethod::Post) {
            self.sources
                .begin_external_tool_activity_dispatch(
                    context,
                    actor,
                    attempt.id,
                    session_id,
                    &lease.token,
                )
                .await
                .map_err(map_store_error)?;
        }
        let result = async {
            if lease.binding != expected {
                return Err(RunBackendError::Invalid(
                    "external-tool launch binding is invalid".into(),
                ));
            }
            let encrypted = lease.encrypted_provider_state.as_ref().ok_or_else(|| {
                RunBackendError::Invalid("external-tool launch state is unavailable".into())
            })?;
            let aad = launch_state_aad(context, actor, attempt, &expected);
            let value = aead.open(encrypted, &aad)?;
            let value = std::str::from_utf8(&value).map_err(|_| {
                RunBackendError::Invalid("external-tool launch state is invalid".into())
            })?;
            let persisted = PersistedContractedLaunchSession::from_storage_value(value)
                .map_err(map_adapter_error)?;
            let expected_launch = ContractedLaunchExpectation::new(
                Self::correlation_binding(context, attempt),
                expected.provider.clone(),
                expected.source_sha256.clone(),
            )
            .map_err(map_adapter_error)?;
            let session = aead
                .adapter_codec
                .restore(&persisted, &expected_launch)
                .map_err(map_adapter_error)?;
            let now = self
                .sources
                .authoritative_time(context)
                .await
                .map_err(map_store_error)?;
            self.adapter
                .proxy_contracted_activity(&session, method, body, now)
                .await
                .map_err(map_adapter_error)
        }
        .await;
        // A POST may have reached the provider even on timeout, I/O failure,
        // or process death. Its durable pre-dispatch marker is deliberately
        // left in place on every error, forbidding reclaim or relaunch.
        if result.is_err() && matches!(method, ProxyMethod::Post) {
            return Err(RunBackendError::Unavailable(
                "external-tool activity outcome is unknown; contact the course instructor".into(),
            ));
        }
        if matches!(method, ProxyMethod::Post) {
            self.sources
                .complete_external_tool_activity_dispatch(context, actor, attempt.id, &lease.token)
                .await
                .map_err(map_store_error)?;
        }
        let release = self
            .sources
            .release_external_tool_activity(context, actor, attempt.id, session_id, &lease.token)
            .await
            .map_err(map_store_error);
        release?;
        result
    }
}

impl<S, O, T> crate::composite_backend::ConfiguredImathas
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
    fn serves_provider(&self, provider: &str) -> bool {
        self.contracted_provider_key() == provider
    }
}
