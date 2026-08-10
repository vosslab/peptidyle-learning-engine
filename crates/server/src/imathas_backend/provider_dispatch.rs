//! Contracted iMathAS provider dispatch from authenticated launch sessions.

use adapter_imathas::broker_provider::{
    ContractedLaunchExpectation, ContractedScoredEmbedProvider, PersistedContractedLaunchSession,
    ProxyMethod, ProxyResponse, ScoredEmbedTransport,
};
use async_trait::async_trait;
use learning_data_access::{
    AuthoritativeTimeStore, CatalogSourceStore, ExternalToolBrokerStore,
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
        let resolved = self
            .sources
            .resolve_external_tool_launch_session(context, actor, attempt.id, session_id, token)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| {
                RunBackendError::Invalid("external-tool launch is unavailable".into())
            })?;
        if resolved.binding != expected {
            return Err(RunBackendError::Invalid(
                "external-tool launch binding is invalid".into(),
            ));
        }
        let encrypted = resolved.encrypted_provider_state.ok_or_else(|| {
            RunBackendError::Invalid("external-tool launch state is unavailable".into())
        })?;
        let aad = launch_state_aad(context, actor, attempt, &expected);
        let value = aead.open(&encrypted, &aad)?;
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
