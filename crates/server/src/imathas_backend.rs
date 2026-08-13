//! Server-only bridge for immutable iMathAS source and durable grade exchange.
//!
//! This module deliberately has no HTTP launch/proxy endpoint and is not part
//! of the production composite backend yet.  Its only responsibility is to
//! bind an issued attempt to one immutable source artifact and, when asked by
//! a future registered backend, submit through the tenant-owned broker.

#[path = "imathas_backend/launch_state.rs"]
mod launch_state;
#[path = "imathas_backend/projection.rs"]
mod projection;
#[path = "imathas_backend/provider_dispatch.rs"]
mod provider_dispatch;
#[path = "imathas_backend/submission.rs"]
mod submission;

pub use launch_state::LaunchStateAead;
use launch_state::launch_state_aad;
pub(crate) use launch_state::{launch_cookie_aad, launch_cookie_value};

use std::sync::Arc;
use std::time::Duration;

use adapter_imathas::{
    CorrelationIssuer, GradeBinding, ImathasAdapter, ImathasAdapterError, ImathasProvider,
    ImathasSource,
};
use async_trait::async_trait;
use learning_data_access::{
    CatalogSourceStore, ExternalToolBinding, ExternalToolBrokerStore, PersistedCorrelation,
    PublishedSourceArtifact, StoreError, TenantContext,
};
use objects::{ObjectStore, Sha256Digest};
use question_model::generation::Seed;
use question_model::{
    ProblemVersionRef, QuestionAttempt, QuestionDefinition, QuestionSource, StudentResponse, UserId,
};

use crate::run::{
    IssuedAttemptMetadata, RunBackend, RunBackendError, RunSubmission, SubmissionDisposition,
};

/// A provider call is permitted to use almost all of its configured timeout;
/// the remaining time is reserved for cancellation and releasing its durable
/// activity capability. Keep this relationship here, rather than allowing an
/// independent deployment constant to make a remote call outlive its lease.
const EXTERNAL_TOOL_ACTIVITY_CANCELLATION_MARGIN_MILLIS: u32 = 5_000;
const EXTERNAL_TOOL_VERIFICATION_FINALIZATION_MARGIN_MILLIS: u32 = 5_000;
const MAX_EXTERNAL_TOOL_ACTIVITY_LEASE_MILLIS: u32 = 60_000;
const MAX_EXTERNAL_TOOL_VERIFICATION_LEASE_MILLIS: u32 = 300_000;

/// Server-side timing derived from the one bounded provider request timeout.
///
/// Every remote provider operation is protected by `activity_lease_millis`.
/// The enclosing verification lease includes another bounded finalization
/// interval for durable staging and commit after that activity lease releases.
/// No caller can construct an iMathAS backend without this relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExternalToolTiming {
    activity_lease_millis: u32,
    verification_lease_millis: u32,
}

impl ExternalToolTiming {
    pub(crate) fn from_provider_timeout(timeout: Duration) -> Result<Self, &'static str> {
        let provider_millis = u32::try_from(timeout.as_millis())
            .map_err(|_| "iMathAS provider request timeout is too large")?;
        if provider_millis == 0 {
            return Err("iMathAS provider request timeout must be positive");
        }
        let activity_lease_millis = provider_millis
            .checked_add(EXTERNAL_TOOL_ACTIVITY_CANCELLATION_MARGIN_MILLIS)
            .ok_or("iMathAS activity lease duration overflow")?;
        if activity_lease_millis > MAX_EXTERNAL_TOOL_ACTIVITY_LEASE_MILLIS {
            return Err(
                "iMathAS provider request timeout exceeds the external activity lease bound",
            );
        }
        let verification_lease_millis = activity_lease_millis
            .checked_add(EXTERNAL_TOOL_VERIFICATION_FINALIZATION_MARGIN_MILLIS)
            .ok_or("iMathAS verification lease duration overflow")?;
        if verification_lease_millis > MAX_EXTERNAL_TOOL_VERIFICATION_LEASE_MILLIS {
            return Err("iMathAS verification lease exceeds its bound");
        }
        Ok(Self {
            activity_lease_millis,
            verification_lease_millis,
        })
    }

    pub(crate) const fn activity_lease_millis(self) -> u32 {
        self.activity_lease_millis
    }

    pub(crate) const fn verification_lease_millis(self) -> u32 {
        self.verification_lease_millis
    }
}

/// Contracted-only marker submission. The launch proof is decoded from the
/// authenticated same-origin cookie by a later route owner; it can never be
/// supplied in browser JSON or a generic response type.
#[async_trait]
pub trait ExternalToolSubmissionBackend: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn submit_external_tool(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        idempotency_key: learning_data_access::SubmissionIdempotencyKey,
        launch_proof: learning_data_access::ExternalToolLaunchProof,
        state_aead: &LaunchStateAead,
    ) -> Result<SubmissionDisposition, RunBackendError>;
}

/// The server-owned iMathAS adapter and its durable broker store.
///
/// `S` is deliberately the only route from a tenant attempt to a source
/// artifact and broker exchange. `O` is used only to verify the exact
/// immutable object that `S` made visible.
pub struct ImathasBackend<S, O, P> {
    sources: Arc<S>,
    objects: Arc<O>,
    adapter: Arc<ImathasAdapter<O, P>>,
    correlations: Arc<CorrelationIssuer>,
    timing: ExternalToolTiming,
}

impl<S, O, P> ImathasBackend<S, O, P> {
    /// Constructs the bridge from already-configured server dependencies.
    /// No provider endpoint, credential, or browser value enters here.
    pub(crate) fn new(
        sources: Arc<S>,
        objects: Arc<O>,
        adapter: Arc<ImathasAdapter<O, P>>,
        correlations: Arc<CorrelationIssuer>,
        timing: ExternalToolTiming,
    ) -> Self {
        Self {
            sources,
            objects,
            adapter,
            correlations,
            timing,
        }
    }

    #[cfg(test)]
    pub(crate) const fn activity_lease_millis(&self) -> u32 {
        self.timing.activity_lease_millis()
    }
}

impl<S, O, P> ImathasBackend<S, O, P>
where
    S: CatalogSourceStore + ExternalToolBrokerStore + Send + Sync + 'static,
    O: ObjectStore + Send + Sync + 'static,
    P: ImathasProvider + 'static,
{
    async fn resolve_source(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
    ) -> Result<(ImathasSource, PublishedSourceArtifact), RunBackendError> {
        validate_reference(reference, question)?;
        let artifact = self
            .sources
            .catalog_source_artifact(context, reference)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| {
                RunBackendError::Invalid("published iMathAS source is unavailable".into())
            })?;
        let source = ImathasSource::resolve(self.objects.as_ref(), question, &artifact)
            .await
            .map_err(map_adapter_error)?;
        Ok((source, artifact))
    }

    /// Recreates the issued envelope and proves it is byte-for-byte the
    /// persisted attempt contract before any launch, broker, or provider-grade
    /// side effect. The adapter owns cache validation and source provenance;
    /// this bridge owns comparison with the tenant record.
    async fn reproduce_issued_attempt(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<adapter_imathas::ImathasIssuedAttempt, RunBackendError> {
        validate_attempt_reference(reference, question, attempt)?;
        let (source, artifact) = self.resolve_source(context, reference, question).await?;
        let issued = self
            .adapter
            .issue(
                question,
                Seed::new(attempt.seed),
                &source,
                artifact.object.created_at,
            )
            .await
            .map_err(map_adapter_error)?;
        if issued.parameter_hash != attempt.parameter_hash
            || issued.provenance != attempt.provenance
        {
            return Err(RunBackendError::Invalid(
                "iMathAS issued output does not match attempt provenance".into(),
            ));
        }
        Ok(issued)
    }

    fn binding(
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        artifact: &PublishedSourceArtifact,
        response: &StudentResponse,
    ) -> Result<ExternalToolBinding, RunBackendError> {
        let QuestionSource::Imathas {
            provider,
            snapshot,
            snapshot_sha256,
            integration_profile,
            ..
        } = &question.source
        else {
            return Err(RunBackendError::Unsupported(
                "published question is not iMathAS".into(),
            ));
        };
        if artifact.reference.problem != question.problem
            || artifact.reference.version != question.version
            || artifact.object.id != *snapshot
            || artifact.object.sha256.to_string() != *snapshot_sha256
            || attempt.problem != question.problem
            || attempt.question_version != question.version
            || !matches!(response, StudentResponse::ExternalTool {})
        {
            return Err(RunBackendError::Invalid(
                "iMathAS external-tool binding does not match the issued attempt".into(),
            ));
        }
        let canonical = serde_json::to_vec(response).map_err(|_| {
            RunBackendError::Invalid("external-tool response cannot be encoded".into())
        })?;
        let binding = ExternalToolBinding {
            provider: provider.clone(),
            problem: question.problem,
            version: question.version,
            seed: attempt.seed,
            source_object: *snapshot,
            source_sha256: snapshot_sha256.clone(),
            integration_profile: integration_profile.clone(),
            response_sha256: Sha256Digest::compute(&canonical),
        };
        binding.validate().map_err(map_store_error)?;
        Ok(binding)
    }

    fn correlation_binding(context: TenantContext, attempt: &QuestionAttempt) -> GradeBinding {
        GradeBinding {
            tenant: context.tenant_id(),
            attempt: attempt.id,
            problem: attempt.problem,
            version: attempt.question_version,
            seed: Seed::new(attempt.seed),
        }
    }

    fn persisted_correlation(
        &self,
        binding: GradeBinding,
    ) -> Result<PersistedCorrelation, RunBackendError> {
        PersistedCorrelation::new(
            self.correlations
                .begin(binding)
                .to_storage_value()
                .into_bytes(),
        )
        .map_err(map_store_error)
    }
}

/// Server-only launch-session operations are available solely for the
/// explicitly contracted provider.  Generic iMathAS providers cannot obtain
/// this capability by accident.
impl<S, O, T>
    ImathasBackend<S, O, adapter_imathas::broker_provider::ContractedScoredEmbedProvider<T>>
where
    S: CatalogSourceStore
        + ExternalToolBrokerStore
        + learning_data_access::ExternalToolLaunchSessionStore
        + learning_data_access::AuthoritativeTimeStore
        + Send
        + Sync
        + 'static,
    O: ObjectStore + Send + Sync + 'static,
    T: adapter_imathas::broker_provider::ScoredEmbedTransport + 'static,
{
    pub fn contracted_provider_key(&self) -> &str {
        self.adapter.contracted_provider_key()
    }
    /// Creates a short-lived, replica-portable session after reproducing the
    /// exact issued attempt. The returned cookie token is intentionally the
    /// only value that may reach the browser; provider state is AEAD-wrapped
    /// before it enters the Store.
    pub async fn create_contracted_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        state_aead: &LaunchStateAead,
    ) -> Result<learning_data_access::CreatedExternalToolLaunchSession, RunBackendError> {
        self.reproduce_issued_attempt(context, reference, question, attempt)
            .await?;
        let (source, artifact) = self.resolve_source(context, reference, question).await?;
        let response = StudentResponse::ExternalTool {};
        let binding = Self::binding(question, attempt, &artifact, &response)?;
        let grade_binding = Self::correlation_binding(context, attempt);
        let correlation = self
            .correlations
            .restore(grade_binding, &self.correlations.begin(grade_binding))
            .map_err(map_adapter_error)?;
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| {
            RunBackendError::Unavailable("iMathAS launch entropy is unavailable".into())
        })?;
        let nonce = adapter_imathas::scored_embed::ScoredEmbedNonce::from_server_random(bytes)
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch nonce".into()))?;
        let now = self
            .sources
            .authoritative_time(context)
            .await
            .map_err(map_store_error)?;
        let session = self
            .adapter
            .begin_contracted_launch(
                question,
                &source,
                context.tenant_id(),
                attempt.id,
                Seed::new(attempt.seed),
                correlation,
                nonce,
                now,
            )
            .await
            .map_err(map_adapter_error)?;
        let aad = launch_state_aad(context, actor, attempt, &binding);
        let encrypted = state_aead.seal_adapter_session(&session, &aad)?;
        self.sources
            .create_external_tool_launch_session(
                context,
                learning_data_access::CreateExternalToolLaunchSessionCommand {
                    actor,
                    attempt: attempt.id,
                    binding,
                    encrypted_provider_state: Some(encrypted),
                    lifetime_millis: self.adapter.contracted_launch_lifetime_millis(),
                },
            )
            .await
            .map_err(map_store_error)
    }
}

fn validate_reference(
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
) -> Result<(), RunBackendError> {
    if question.problem != reference.problem || question.version != reference.version {
        return Err(RunBackendError::Invalid(
            "published iMathAS question identity does not match its reference".into(),
        ));
    }
    if !matches!(question.source, QuestionSource::Imathas { .. }) {
        return Err(RunBackendError::Unsupported(
            "published question is not iMathAS".into(),
        ));
    }
    Ok(())
}

fn validate_attempt_reference(
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
    attempt: &QuestionAttempt,
) -> Result<(), RunBackendError> {
    validate_reference(reference, question)?;
    if attempt.problem != reference.problem || attempt.question_version != reference.version {
        return Err(RunBackendError::Invalid(
            "attempt does not match its published iMathAS question".into(),
        ));
    }
    Ok(())
}

fn map_store_error(error: StoreError) -> RunBackendError {
    match error {
        StoreError::Unavailable(_) => {
            RunBackendError::Unavailable("external-tool broker is temporarily unavailable".into())
        }
        StoreError::NotFound => {
            RunBackendError::Invalid("published iMathAS source is unavailable".into())
        }
        StoreError::Conflict => RunBackendError::Unavailable(
            "external-tool submission is being processed; retry with the same key".into(),
        ),
        _ => RunBackendError::Invalid("invalid iMathAS external-tool binding".into()),
    }
}

fn map_adapter_error(error: ImathasAdapterError) -> RunBackendError {
    match error {
        ImathasAdapterError::Provider(_) | ImathasAdapterError::ObjectStore(_) => {
            RunBackendError::Unavailable("iMathAS is temporarily unavailable".into())
        }
        ImathasAdapterError::UnsupportedSource | ImathasAdapterError::UnsupportedProfile => {
            RunBackendError::Unsupported("iMathAS profile is not configured".into())
        }
        _ => RunBackendError::Invalid("published iMathAS source or response was refused".into()),
    }
}

#[cfg(test)]
#[path = "imathas_backend/tests/mod.rs"]
mod tests;
