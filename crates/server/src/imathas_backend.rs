//! Server-only bridge for immutable iMathAS source and durable grade exchange.
//!
//! This module deliberately has no HTTP launch/proxy endpoint and is not part
//! of the production composite backend yet.  Its only responsibility is to
//! bind an issued attempt to one immutable source artifact and, when asked by
//! a future registered backend, submit through the tenant-owned broker.

use std::sync::Arc;

use adapter_imathas::{
    CorrelationIssuer, GradeBinding, ImathasAdapter, ImathasAdapterError, ImathasProvider,
    ImathasSource,
};
use async_trait::async_trait;
use base64::Engine as _;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use objects::{ObjectStore, Sha256Digest};
use question_model::generation::Seed;
use question_model::{
    ProblemVersionRef, QuestionAttempt, QuestionDefinition, QuestionEnvelope, QuestionSource,
    StudentResponse, UserId,
};
use store::{
    BeginExternalToolGradeCommand, CatalogSourceStore, ExternalToolBegin, ExternalToolBinding,
    ExternalToolBrokerStore, PersistedCorrelation, PublishedSourceArtifact,
    StageExternalToolVerificationCommand, StoreError, TenantContext,
};

use crate::run::{
    IssuedAttemptMetadata, RunBackend, RunBackendError, RunSubmission, SubmissionDisposition,
};

const EXTERNAL_TOOL_LEASE_MILLIS: u32 = 30_000;

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
        idempotency_key: store::SubmissionIdempotencyKey,
        launch_proof: store::ExternalToolLaunchProof,
        state_aead: &LaunchStateAead,
    ) -> Result<SubmissionDisposition, RunBackendError>;
}

/// Server configuration for encrypted contracted-launch state.  This is a
/// distinct secret from the signed provider launch and broker correlation
/// keys. It is replica-stable and never becomes a browser value.
pub struct LaunchStateAead {
    cipher: XChaCha20Poly1305,
    cookie_cipher: XChaCha20Poly1305,
    adapter_codec: adapter_imathas::broker_provider::LaunchSessionCodec,
}

impl std::fmt::Debug for LaunchStateAead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LaunchStateAead(REDACTED)")
    }
}

impl LaunchStateAead {
    pub fn from_server_secret(secret: [u8; 32]) -> Result<Self, RunBackendError> {
        if secret.iter().all(|byte| *byte == 0) {
            return Err(RunBackendError::Invalid(
                "iMathAS launch-state secret is invalid".into(),
            ));
        }
        let mut cookie_key = sha2::Sha256::new();
        use sha2::Digest as _;
        cookie_key.update(b"ple:imathas:launch-cookie:v1");
        cookie_key.update(secret);
        let cookie_key: [u8; 32] = cookie_key.finalize().into();
        Ok(Self {
            cipher: XChaCha20Poly1305::new((&secret).into()),
            cookie_cipher: XChaCha20Poly1305::new((&cookie_key).into()),
            adapter_codec:
                adapter_imathas::broker_provider::LaunchSessionCodec::from_server_secret(secret)
                    .map_err(map_adapter_error)?,
        })
    }

    /// Versioned bounded ciphertext.  The Store receives only this value; the
    /// adapter's authenticated launch codec remains entirely inside it.
    pub fn seal(&self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, RunBackendError> {
        if plaintext.is_empty() || plaintext.len() > 8_192 || aad.is_empty() || aad.len() > 2_048 {
            return Err(RunBackendError::Invalid(
                "invalid iMathAS launch state".into(),
            ));
        }
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| {
            RunBackendError::Unavailable("iMathAS launch entropy is unavailable".into())
        })?;
        let nonce = XNonce::try_from(nonce.as_slice())
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch state".into()))?;
        let encrypted = self
            .cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| RunBackendError::Unavailable("iMathAS launch encryption failed".into()))?;
        let mut result = Vec::with_capacity(1 + nonce.len() + encrypted.len());
        result.push(1);
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&encrypted);
        Ok(result)
    }

    pub fn open(&self, ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>, RunBackendError> {
        if !(41..=8_256).contains(&ciphertext.len())
            || aad.is_empty()
            || aad.len() > 2_048
            || ciphertext[0] != 1
        {
            return Err(RunBackendError::Invalid(
                "invalid iMathAS launch state".into(),
            ));
        }
        let nonce = XNonce::try_from(&ciphertext[1..25])
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch state".into()))?;
        self.cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &ciphertext[25..],
                    aad,
                },
            )
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch state".into()))
    }

    fn seal_adapter_session(
        &self,
        session: &adapter_imathas::broker_provider::ContractedLaunchSession,
        aad: &[u8],
    ) -> Result<Vec<u8>, RunBackendError> {
        let inner = self
            .adapter_codec
            .seal(session)
            .map_err(map_adapter_error)?;
        self.seal(inner.to_storage_value().as_bytes(), aad)
    }

    /// Fixed-name cookie codec. Its opaque plaintext is exactly the Store
    /// session UUID and opaque token, never an upstream handle or provider
    /// credential. It uses a distinct derived key/domain from provider state.
    pub fn seal_cookie(
        &self,
        id: uuid::Uuid,
        token: &store::ExternalToolLaunchToken,
        aad: &[u8],
    ) -> Result<String, RunBackendError> {
        let token = token.encode_cookie_value();
        let mut plain = Vec::with_capacity(59);
        plain.extend_from_slice(id.as_bytes());
        plain.extend_from_slice(token.as_bytes());
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| {
            RunBackendError::Unavailable("iMathAS launch entropy is unavailable".into())
        })?;
        let nonce_value = XNonce::try_from(nonce.as_slice())
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        let encrypted = self
            .cookie_cipher
            .encrypt(&nonce_value, Payload { msg: &plain, aad })
            .map_err(|_| RunBackendError::Unavailable("iMathAS launch encryption failed".into()))?;
        let mut wire = Vec::with_capacity(1 + 24 + encrypted.len());
        wire.push(1);
        wire.extend_from_slice(&nonce);
        wire.extend_from_slice(&encrypted);
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(wire))
    }

    pub fn open_cookie(
        &self,
        wire: &str,
        aad: &[u8],
    ) -> Result<(uuid::Uuid, store::ExternalToolLaunchToken), RunBackendError> {
        if wire.len() > 256
            || !wire
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(RunBackendError::Invalid(
                "invalid iMathAS launch cookie".into(),
            ));
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(wire)
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        if !(1 + 24 + 16..=256).contains(&bytes.len()) || bytes[0] != 1 {
            return Err(RunBackendError::Invalid(
                "invalid iMathAS launch cookie".into(),
            ));
        }
        let nonce = XNonce::try_from(&bytes[1..25])
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        let plain = self
            .cookie_cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &bytes[25..],
                    aad,
                },
            )
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        if plain.len() != 59 {
            return Err(RunBackendError::Invalid(
                "invalid iMathAS launch cookie".into(),
            ));
        }
        let id = uuid::Uuid::from_slice(&plain[..16])
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        let token = std::str::from_utf8(&plain[16..])
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        let token =
            store::ExternalToolLaunchToken::parse_cookie_value(token).map_err(map_store_error)?;
        Ok((id, token))
    }
}

/// Canonical associated data prevents a ciphertext copied across tenant,
/// learner, attempt, immutable source, or integration-profile boundaries from
/// being restored. It deliberately contains no secret; its only job is exact
/// cryptographic binding.
#[allow(dead_code)]
fn launch_state_aad(
    context: TenantContext,
    actor: UserId,
    attempt: &QuestionAttempt,
    binding: &ExternalToolBinding,
) -> Vec<u8> {
    let mut result = Vec::with_capacity(512);
    result.extend_from_slice(b"ple:imathas:launch-state:v1\0");
    for value in [
        context.tenant_id().as_uuid().to_string(),
        actor.as_uuid().to_string(),
        attempt.id.as_uuid().to_string(),
        attempt.problem.as_uuid().to_string(),
        attempt.question_version.as_uuid().to_string(),
        attempt.seed.to_string(),
        binding.provider.clone(),
        binding.source_object.as_uuid().to_string(),
        binding.source_sha256.clone(),
        binding.integration_profile.clone(),
    ] {
        result.extend_from_slice(value.as_bytes());
        result.push(0);
    }
    result
}

#[allow(dead_code)]
pub(crate) fn launch_cookie_aad(
    context: TenantContext,
    actor: UserId,
    attempt: question_model::QuestionAttemptId,
) -> Vec<u8> {
    format!(
        "ple:imathas:launch-cookie:v1\\0{}\\0{}\\0{}\\0",
        context.tenant_id().as_uuid(),
        actor.as_uuid(),
        attempt.as_uuid(),
    )
    .into_bytes()
}

/// Encodes the one fixed-name HttpOnly cookie after Store creation. The value
/// is intentionally not a DTO and should be written directly to `Set-Cookie`.
#[allow(dead_code)]
pub(crate) fn launch_cookie_value(
    aead: &LaunchStateAead,
    context: TenantContext,
    actor: UserId,
    attempt: question_model::QuestionAttemptId,
    created: &store::CreatedExternalToolLaunchSession,
) -> Result<String, RunBackendError> {
    aead.seal_cookie(
        created.id,
        &created.token,
        &launch_cookie_aad(context, actor, attempt),
    )
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
}

impl<S, O, P> ImathasBackend<S, O, P> {
    /// Constructs the bridge from already-configured server dependencies.
    /// No provider endpoint, credential, or browser value enters here.
    pub fn new(
        sources: Arc<S>,
        objects: Arc<O>,
        adapter: Arc<ImathasAdapter<O, P>>,
        correlations: Arc<CorrelationIssuer>,
    ) -> Self {
        Self {
            sources,
            objects,
            adapter,
            correlations,
        }
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

    fn restore_correlation(
        &self,
        binding: GradeBinding,
        persisted: &PersistedCorrelation,
    ) -> Result<adapter_imathas::ServerCorrelation, RunBackendError> {
        let bytes = persisted.to_storage_bytes();
        let value = std::str::from_utf8(&bytes).map_err(|_| {
            RunBackendError::Invalid("stored iMathAS correlation is invalid".into())
        })?;
        let encoded = adapter_imathas::PersistedCorrelation::from_storage_value(value)
            .map_err(map_adapter_error)?;
        self.correlations
            .restore(binding, &encoded)
            .map_err(map_adapter_error)
    }

    async fn commit_verified(
        &self,
        submission: &RunSubmission<'_>,
        binding: ExternalToolBinding,
        correlation: PersistedCorrelation,
        token: store::ExternalToolLeaseToken,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        let _ = (submission, binding, correlation, token);
        Err(RunBackendError::Unsupported(
            "iMathAS submission requires an authenticated launch session".into(),
        ))
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
        + store::ExternalToolLaunchSessionStore
        + store::AuthoritativeTimeStore
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
    ) -> Result<store::CreatedExternalToolLaunchSession, RunBackendError> {
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
                store::CreateExternalToolLaunchSessionCommand {
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

#[async_trait]
impl<S, O, T> crate::run::ExternalToolLaunchBackend
    for ImathasBackend<S, O, adapter_imathas::broker_provider::ContractedScoredEmbedProvider<T>>
where
    S: CatalogSourceStore
        + ExternalToolBrokerStore
        + store::ExternalToolLaunchSessionStore
        + store::AuthoritativeTimeStore
        + Send
        + Sync
        + 'static,
    O: ObjectStore + Send + Sync + 'static,
    T: adapter_imathas::broker_provider::ScoredEmbedTransport + 'static,
{
    async fn create_external_tool_launch(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        aead: &LaunchStateAead,
    ) -> Result<store::CreatedExternalToolLaunchSession, RunBackendError> {
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
        token: &store::ExternalToolLaunchToken,
        method: adapter_imathas::broker_provider::ProxyMethod,
        body: &[u8],
        aead: &LaunchStateAead,
    ) -> Result<adapter_imathas::broker_provider::ProxyResponse, RunBackendError> {
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
        let persisted =
            adapter_imathas::broker_provider::PersistedContractedLaunchSession::from_storage_value(
                value,
            )
            .map_err(map_adapter_error)?;
        let expected_launch = adapter_imathas::broker_provider::ContractedLaunchExpectation::new(
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
    for ImathasBackend<S, O, adapter_imathas::broker_provider::ContractedScoredEmbedProvider<T>>
where
    S: CatalogSourceStore
        + ExternalToolBrokerStore
        + store::ExternalToolLaunchSessionStore
        + store::AuthoritativeTimeStore
        + Send
        + Sync
        + 'static,
    O: ObjectStore + Send + Sync + 'static,
    T: adapter_imathas::broker_provider::ScoredEmbedTransport + 'static,
{
    fn serves_provider(&self, provider: &str) -> bool {
        self.contracted_provider_key() == provider
    }
}

#[async_trait]
impl<S, O, T> ExternalToolSubmissionBackend
    for ImathasBackend<S, O, adapter_imathas::broker_provider::ContractedScoredEmbedProvider<T>>
where
    S: CatalogSourceStore
        + ExternalToolBrokerStore
        + store::ExternalToolLaunchSessionStore
        + store::AuthoritativeTimeStore
        + Send
        + Sync
        + 'static,
    O: ObjectStore + Send + Sync + 'static,
    T: adapter_imathas::broker_provider::ScoredEmbedTransport + 'static,
{
    async fn submit_external_tool(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        idempotency_key: store::SubmissionIdempotencyKey,
        launch_proof: store::ExternalToolLaunchProof,
        state_aead: &LaunchStateAead,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        self.reproduce_issued_attempt(context, reference, question, attempt)
            .await?;
        let (_source, artifact) = self.resolve_source(context, reference, question).await?;
        let response = StudentResponse::ExternalTool {};
        let binding = Self::binding(question, attempt, &artifact, &response)?;
        let grade_binding = Self::correlation_binding(context, attempt);
        // Claim the durable exchange before resolving the one-use launch
        // proof: replay and an active replica must not touch provider state.
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
                    store::CommitVerifiedExternalToolSubmissionCommand {
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
                let persisted = adapter_imathas::broker_provider::PersistedContractedLaunchSession::from_storage_value(text)
                    .map_err(map_adapter_error)?;
                let expectation =
                    adapter_imathas::broker_provider::ContractedLaunchExpectation::new(
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
                        store::CommitExternalToolSubmissionCommand {
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

#[async_trait]
impl<S, O, P> RunBackend for ImathasBackend<S, O, P>
where
    S: CatalogSourceStore + ExternalToolBrokerStore + Send + Sync + 'static,
    O: ObjectStore + Send + Sync + 'static,
    P: ImathasProvider + 'static,
{
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        let (source, artifact) = self.resolve_source(context, reference, question).await?;
        let issued = self
            .adapter
            .issue(
                question,
                Seed::new(seed),
                &source,
                artifact.object.created_at,
            )
            .await
            .map_err(map_adapter_error)?;
        Ok(IssuedAttemptMetadata {
            envelope: issued.envelope,
            parameter_hash: issued.parameter_hash,
            provenance: issued.provenance,
        })
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        Ok(self
            .reproduce_issued_attempt(context, reference, question, attempt)
            .await?
            .envelope)
    }

    async fn prepare_external_tool_launch(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<(), RunBackendError> {
        let _ = self
            .reproduce_issued_attempt(context, reference, question, attempt)
            .await?;
        Ok(())
    }

    async fn grade(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
        _question: &QuestionDefinition,
        _attempt: &QuestionAttempt,
        _response: &StudentResponse,
    ) -> Result<grading::GradeOutcome, RunBackendError> {
        Err(RunBackendError::Unsupported(
            "iMathAS grading requires the durable external-tool broker".into(),
        ))
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        let _ = submission;
        return Err(RunBackendError::Unsupported(
            "iMathAS submission requires an authenticated launch session".into(),
        ));
        #[allow(unreachable_code)]
        {
            self.reproduce_issued_attempt(
                submission.context,
                submission.reference,
                submission.question,
                submission.attempt,
            )
            .await?;
            let (source, artifact) = self
                .resolve_source(
                    submission.context,
                    submission.reference,
                    submission.question,
                )
                .await?;
            let binding = Self::binding(
                submission.question,
                submission.attempt,
                &artifact,
                submission.response,
            )?;
            let grade_binding = Self::correlation_binding(submission.context, submission.attempt);
            let proposed_correlation = self.persisted_correlation(grade_binding)?;
            let begin = self
                .sources
                .begin_or_resume_external_grade(
                    submission.context,
                    BeginExternalToolGradeCommand {
                        actor: submission.actor,
                        attempt: submission.attempt.id,
                        response: submission.response.clone(),
                        idempotency_key: submission.idempotency_key.clone(),
                        binding: binding.clone(),
                        proposed_correlation,
                        lease_millis: EXTERNAL_TOOL_LEASE_MILLIS,
                    },
                )
                .await
                .map_err(map_store_error)?;
            match begin {
                ExternalToolBegin::Committed(record) => {
                    Ok(SubmissionDisposition::Committed(record))
                }
                ExternalToolBegin::VerifiedPending(pending) => {
                    if pending.binding != binding {
                        return Err(RunBackendError::Invalid(
                            "iMathAS verified result does not match its request binding".into(),
                        ));
                    }
                    self.sources
                        .commit_verified_external_tool_submission(
                            submission.context,
                            unreachable!(),
                        )
                        .await
                        .map(|record| SubmissionDisposition::Committed(Box::new(record)))
                        .map_err(map_store_error)
                }
                ExternalToolBegin::InProgress => Err(RunBackendError::Unavailable(
                    "external-tool verification is in progress".into(),
                )),
                ExternalToolBegin::Lease(lease) => {
                    let correlation =
                        self.restore_correlation(grade_binding, &lease.correlation)?;
                    let receipt = self
                        .adapter
                        .grade(
                            submission.question,
                            &source,
                            submission.context.tenant_id(),
                            submission.attempt.id,
                            Seed::new(submission.attempt.seed),
                            &correlation,
                        )
                        .await
                        .map_err(map_adapter_error)?;
                    if receipt.binding() != grade_binding {
                        return Err(RunBackendError::Invalid(
                            "iMathAS verifier returned an incorrectly bound result".into(),
                        ));
                    }
                    self.sources
                        .stage_external_tool_verification(
                            submission.context,
                            StageExternalToolVerificationCommand {
                                actor: submission.actor,
                                attempt: submission.attempt.id,
                                response: submission.response.clone(),
                                idempotency_key: submission.idempotency_key.clone(),
                                binding: lease.binding.clone(),
                                correlation: lease.correlation.clone(),
                                lease_token: lease.token.clone(),
                                result: receipt.result(),
                            },
                        )
                        .await
                        .map_err(map_store_error)?;
                    self.commit_verified(&submission, lease.binding, lease.correlation, lease.token)
                        .await
                }
            }
        }
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
mod tests {
    use std::sync::Arc;

    use super::*;

    use adapter_imathas::test_support::{
        RecordedImathasProvider, RecordedImathasProviderFactory, RecordedProviderMode,
    };
    use adapter_imathas::{
        CorrelationIssuer, GradeBinding, PersistedCorrelation as AdapterCorrelation,
    };
    use objects::memory::MemoryObjectStore;
    use objects::{ObjectKey, ObjectStore, PutObject, Sha256Digest};
    use question_model::capability::Capability;
    use question_model::generation::{RandomizationDefinition, Seed};
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, AssignmentEnrollment, AssignmentId, AttemptResult, BackendCapabilities,
        CompletionRequirement, ContinuedPractice, CourseId, CourseMembership, CourseMembershipRole,
        DraftQuestionDefinition, DraftQuestionSource, EnrollmentId, GradePolicy, GradingDefinition,
        ProblemId, QuestionAttemptId, QuestionMetadata, QuestionSource, RunId, RunPolicies,
        StudentId, StudentResponse, TenantId, UserId, VersionId, WorkspaceId,
    };
    use store::memory::MemoryStore;
    use store::{
        AssignmentRecord, BeginExternalToolGradeCommand, CatalogSourceStore, CatalogStore,
        CourseRecord, DraftRecord, ExternalToolBegin, ExternalToolBrokerStore,
        IssueQuestionAttemptCommand, PersistedCorrelation, PublishDraftCommand,
        StageExternalToolVerificationCommand, Store,
    };
    use uuid::Uuid;

    #[test]
    fn launch_state_aead_binds_each_issued_identity() {
        let aead = LaunchStateAead::from_server_secret([9; 32]).expect("aead");
        let aad = b"tenant\0actor\0attempt\0problem\0version\0seed\0provider\0source\0profile\0";
        let sealed = aead.seal(b"adapter-private-session", aad).expect("seal");
        assert_ne!(sealed, b"adapter-private-session");
        assert_eq!(
            aead.open(&sealed, aad).expect("open"),
            b"adapter-private-session"
        );
        assert!(aead.open(&sealed, b"other identity").is_err());
        let mut altered = sealed;
        *altered.last_mut().expect("ciphertext") ^= 1;
        assert!(aead.open(&altered, aad).is_err());
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    type TestBackend = ImathasBackend<MemoryStore, MemoryObjectStore, RecordedImathasProvider>;

    struct Fixture {
        store: Arc<MemoryStore>,
        backend: TestBackend,
        provider: RecordedImathasProvider,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        question: QuestionDefinition,
        attempt: QuestionAttempt,
    }

    async fn fixture() -> Fixture {
        let store = Arc::new(MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
            .expect("fixture clock");
        let objects = Arc::new(MemoryObjectStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let actor = UserId::from_uuid(id(2));
        let instructor = UserId::from_uuid(id(13));
        let workspace = WorkspaceId::from_uuid(id(3));
        let problem = ProblemId::from_uuid(id(4));
        let version = VersionId::from_uuid(id(5));
        let snapshot = question_model::ObjectId::from_uuid(id(6));
        let digest = Sha256Digest::compute(br#"{"recorded":true}"#).to_string();
        let question_source = QuestionSource::Imathas {
            provider: "recorded-provider".into(),
            item_ref: "item-17".into(),
            snapshot,
            snapshot_sha256: digest,
            integration_profile: "recorded-v1".into(),
        };
        let draft = DraftRecord {
            tenant,
            question: DraftQuestionDefinition {
                workspace,
                source: DraftQuestionSource::Imathas {
                    provider: "recorded-provider".into(),
                    item_ref: "item-17".into(),
                },
                prompt: Vec::new(),
                response: question_model::ResponseDefinition::ExternalTool {},
                attempt_policy: AttemptPolicy {
                    max_attempts: None,
                    feedback: FeedbackDisclosure::ImmediateCorrectness,
                },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Static,
                grading: GradingDefinition::AllOrNothing { points: 1.0 },
                metadata: QuestionMetadata {
                    title: "Recorded iMathAS question".into(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::CcBySa,
                    language: "en-US".into(),
                },
            },
            revises: None,
            derived_from: None,
        };
        objects
            .put(PutObject {
                key: ObjectKey::ProblemSource {
                    problem,
                    version,
                    object: snapshot,
                },
                bytes: br#"{"recorded":true}"#.to_vec(),
                media_type: "application/json".into(),
                license: "CC-BY-SA-4.0".into(),
                provenance: "recorded fixture".into(),
                created_at: ActivityTimestamp::from_unix_millis(10_000),
            })
            .await
            .expect("source object");
        let saved = store
            .upsert_draft(context, instructor, None, draft.clone())
            .await
            .expect("draft");
        let reference = ProblemVersionRef { problem, version };
        store
            .publish_draft(
                context,
                instructor,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved.revision,
                    publication: reference,
                    published_source: question_source,
                    source_artifact: Some(store::PublishedSourceArtifact {
                        reference,
                        backend: question_model::QuestionBackend::Imathas,
                        object: objects
                            .get(&ObjectKey::ProblemSource {
                                problem,
                                version,
                                object: snapshot,
                            })
                            .await
                            .expect("stored source")
                            .record,
                    }),
                    qti_promotion: None,
                    publisher: instructor,
                    scope: question_model::PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([
                        Capability::AlgorithmicGeneration,
                        Capability::ServerGrading,
                    ]),
                },
            )
            .await
            .expect("publish");
        let question = store
            .get_catalog_problem(context, reference)
            .await
            .expect("catalog")
            .expect("published")
            .question;
        let course = CourseId::from_uuid(id(7));
        let assignment = AssignmentId::from_uuid(id(8));
        let enrollment = EnrollmentId::from_uuid(id(9));
        store
            .upsert_course(
                context,
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Recorded course".into(),
                    members: vec![
                        CourseMembership {
                            user: instructor,
                            role: CourseMembershipRole::Instructor,
                        },
                        CourseMembership {
                            user: actor,
                            role: CourseMembershipRole::Student,
                        },
                    ],
                },
            )
            .await
            .expect("course");
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    title: "Recorded assignment".into(),
                    items: vec![question_model::AssignmentItem {
                        id: question_model::AssignmentItemId::from_uuid(id(10)),
                        reference,
                        position: 0,
                        points_possible: question_model::PointValue::from_whole(1),
                        delivery_state: question_model::AssignmentDeliveryState::Active,
                        scoring_mode: question_model::AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AllCorrect,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: question_model::VariationPolicy::NewSeeds,
                    },
                },
            )
            .await
            .expect("assignment");
        store
            .create_enrollment(
                context,
                AssignmentEnrollment {
                    id: enrollment,
                    tenant,
                    assignment,
                    user: actor,
                    student: StudentId::from_uuid(id(10)),
                    first_completed_at: None,
                    current_grade_run: None,
                    best_grade_run: None,
                },
            )
            .await
            .expect("enrollment");
        let run = store
            .start_or_resume_run(context, actor, assignment, RunId::from_uuid(id(11)))
            .await
            .expect("run");
        let provider = RecordedImathasProviderFactory::new(RecordedProviderMode::Verified).build();
        let adapter = Arc::new(ImathasAdapter::new(
            objects.as_ref().clone(),
            provider.clone(),
            [
                adapter_imathas::SupportedProfile::new("recorded-v1", true, true, true)
                    .expect("profile"),
            ],
        ));
        let backend = ImathasBackend::new(
            Arc::clone(&store),
            Arc::clone(&objects),
            adapter,
            Arc::new(CorrelationIssuer::from_server_secret([3; 32])),
        );
        let issued = backend
            .issue(context, reference, &question, 17)
            .await
            .expect("issue");
        let attempt = store
            .issue_or_resume_question_attempt(
                context,
                IssueQuestionAttemptCommand {
                    actor,
                    attempt: QuestionAttemptId::from_uuid(id(12)),
                    run: run.id,
                    assignment_position: 0,
                    problem,
                    question_version: version,
                    seed: 17,
                    parameter_hash: issued.parameter_hash,
                    provenance: issued.provenance,
                    prefetched: None,
                    predecessor_submission: None,
                },
            )
            .await
            .expect("attempt");
        Fixture {
            store,
            backend,
            provider,
            context,
            actor,
            reference,
            question,
            attempt,
        }
    }

    fn submission<'a>(
        fixture: &'a Fixture,
        key: &str,
        response: &'a StudentResponse,
    ) -> RunSubmission<'a> {
        RunSubmission {
            context: fixture.context,
            actor: fixture.actor,
            idempotency_key: store::SubmissionIdempotencyKey::parse(key).expect("key"),
            reference: fixture.reference,
            question: &fixture.question,
            attempt: &fixture.attempt,
            response,
        }
    }

    #[tokio::test]
    async fn generic_submission_refuses_without_an_authenticated_launch_session() {
        let fixture = fixture().await;
        let envelope = fixture
            .backend
            .reproduce(
                fixture.context,
                fixture.reference,
                &fixture.question,
                &fixture.attempt,
            )
            .await
            .expect("exact reproduction");
        assert_eq!(envelope.seed, Seed::new(fixture.attempt.seed));
        assert_eq!(
            envelope.response,
            question_model::ResponseDefinition::ExternalTool {}
        );
        let response = StudentResponse::ExternalTool {};
        let refused = fixture
            .backend
            .submit(submission(&fixture, "server-imathas-first", &response))
            .await
            .expect_err("generic submission must not bypass launch ownership");
        assert!(matches!(refused, RunBackendError::Unsupported(_)));
        assert_eq!(fixture.provider.grade_calls(), 0);
    }

    #[tokio::test]
    async fn generic_submission_never_reaches_a_provider_without_launch_ownership() {
        let fixture = fixture().await;
        let response = StudentResponse::ExternalTool {};
        let mut parameter_tamper = fixture.attempt.clone();
        parameter_tamper.parameter_hash.push('x');
        let parameter = fixture
            .backend
            .submit(RunSubmission {
                context: fixture.context,
                actor: fixture.actor,
                idempotency_key: store::SubmissionIdempotencyKey::parse("server-imathas-param")
                    .expect("key"),
                reference: fixture.reference,
                question: &fixture.question,
                attempt: &parameter_tamper,
                response: &response,
            })
            .await;
        assert!(matches!(parameter, Err(RunBackendError::Unsupported(_))));
        assert_eq!(fixture.provider.grade_calls(), 0);

        let mut provenance_tamper = fixture.attempt.clone();
        provenance_tamper
            .provenance
            .rendered_question_sha256
            .push('x');
        let provenance = fixture
            .backend
            .prepare_external_tool_launch(
                fixture.context,
                fixture.reference,
                &fixture.question,
                &provenance_tamper,
            )
            .await;
        assert!(matches!(provenance, Err(RunBackendError::Invalid(_))));
        assert_eq!(fixture.provider.grade_calls(), 0);

        let mut source_tamper = fixture.question.clone();
        if let QuestionSource::Imathas {
            snapshot_sha256, ..
        } = &mut source_tamper.source
        {
            snapshot_sha256.replace_range(..1, "0");
        }
        let source = fixture
            .backend
            .submit(RunSubmission {
                context: fixture.context,
                actor: fixture.actor,
                idempotency_key: store::SubmissionIdempotencyKey::parse("server-imathas-source")
                    .expect("key"),
                reference: fixture.reference,
                question: &source_tamper,
                attempt: &fixture.attempt,
                response: &response,
            })
            .await;
        assert!(matches!(source, Err(RunBackendError::Unsupported(_))));
        assert_eq!(fixture.provider.grade_calls(), 0);
    }

    async fn binding_for(fixture: &Fixture, response: &StudentResponse) -> ExternalToolBinding {
        let artifact = fixture
            .store
            .catalog_source_artifact(fixture.context, fixture.reference)
            .await
            .expect("source lookup")
            .expect("source artifact");
        TestBackend::binding(&fixture.question, &fixture.attempt, &artifact, response)
            .expect("binding")
    }

    #[tokio::test]
    async fn generic_submission_refuses_even_when_a_broker_exchange_exists() {
        let tampered_fixture = fixture().await;
        let response = StudentResponse::ExternalTool {};
        let binding = binding_for(&tampered_fixture, &response).await;
        let key =
            store::SubmissionIdempotencyKey::parse("server-imathas-correlation").expect("key");
        tampered_fixture
            .store
            .begin_or_resume_external_grade(
                tampered_fixture.context,
                BeginExternalToolGradeCommand {
                    actor: tampered_fixture.actor,
                    attempt: tampered_fixture.attempt.id,
                    response: response.clone(),
                    idempotency_key: key.clone(),
                    binding: binding.clone(),
                    proposed_correlation: PersistedCorrelation::new(b"corrupted".to_vec())
                        .expect("bounded corruption"),
                    lease_millis: EXTERNAL_TOOL_LEASE_MILLIS,
                },
            )
            .await
            .expect("tampered lease setup");
        tampered_fixture
            .store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(41_000))
            .expect("expire fixture lease");
        let tampered = tampered_fixture
            .backend
            .submit(submission(
                &tampered_fixture,
                "server-imathas-correlation",
                &response,
            ))
            .await;
        assert!(matches!(tampered, Err(RunBackendError::Unsupported(_))));
        assert_eq!(tampered_fixture.provider.grade_calls(), 0);

        let in_progress = fixture().await;
        let binding = binding_for(&in_progress, &response).await;
        let grade_binding =
            TestBackend::correlation_binding(in_progress.context, &in_progress.attempt);
        in_progress
            .store
            .begin_or_resume_external_grade(
                in_progress.context,
                BeginExternalToolGradeCommand {
                    actor: in_progress.actor,
                    attempt: in_progress.attempt.id,
                    response: response.clone(),
                    idempotency_key: store::SubmissionIdempotencyKey::parse("server-imathas-busy")
                        .expect("key"),
                    binding,
                    proposed_correlation: in_progress
                        .backend
                        .persisted_correlation(grade_binding)
                        .expect("correlation"),
                    lease_millis: EXTERNAL_TOOL_LEASE_MILLIS,
                },
            )
            .await
            .expect("active lease setup");
        let busy = in_progress
            .backend
            .submit(submission(&in_progress, "server-imathas-busy", &response))
            .await;
        assert!(matches!(busy, Err(RunBackendError::Unsupported(_))));
        assert_eq!(in_progress.provider.grade_calls(), 0);
    }

    #[tokio::test]
    async fn generic_submission_cannot_commit_verified_pending_without_launch_proof() {
        let fixture = fixture().await;
        let response = StudentResponse::ExternalTool {};
        let binding = binding_for(&fixture, &response).await;
        let key = store::SubmissionIdempotencyKey::parse("server-imathas-verified").expect("key");
        let grade_binding = TestBackend::correlation_binding(fixture.context, &fixture.attempt);
        let lease = fixture
            .store
            .begin_or_resume_external_grade(
                fixture.context,
                BeginExternalToolGradeCommand {
                    actor: fixture.actor,
                    attempt: fixture.attempt.id,
                    response: response.clone(),
                    idempotency_key: key.clone(),
                    binding: binding.clone(),
                    proposed_correlation: fixture
                        .backend
                        .persisted_correlation(grade_binding)
                        .expect("correlation"),
                    lease_millis: EXTERNAL_TOOL_LEASE_MILLIS,
                },
            )
            .await
            .expect("lease setup");
        let ExternalToolBegin::Lease(lease) = lease else {
            panic!("new exchange must lease")
        };
        fixture
            .store
            .stage_external_tool_verification(
                fixture.context,
                StageExternalToolVerificationCommand {
                    actor: fixture.actor,
                    attempt: fixture.attempt.id,
                    response: response.clone(),
                    idempotency_key: key,
                    binding: binding.clone(),
                    correlation: lease.correlation,
                    lease_token: lease.token,
                    result: AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    },
                },
            )
            .await
            .expect("staged verified receipt");
        let recovered = fixture
            .backend
            .submit(submission(&fixture, "server-imathas-verified", &response))
            .await
            .expect_err("generic submission has no authenticated launch proof");
        assert!(matches!(recovered, RunBackendError::Unsupported(_)));
        assert_eq!(fixture.provider.grade_calls(), 0);
    }

    #[test]
    fn stored_correlation_round_trips_only_with_its_exact_mac_binding() {
        let issuer = CorrelationIssuer::from_server_secret([7; 32]);
        let binding = GradeBinding {
            tenant: TenantId::from_uuid(id(1)),
            attempt: QuestionAttemptId::from_uuid(id(2)),
            problem: ProblemId::from_uuid(id(3)),
            version: VersionId::from_uuid(id(4)),
            seed: Seed::new(5),
        };
        let adapter_value = issuer.begin(binding);
        let stored = PersistedCorrelation::new(adapter_value.to_storage_value().into_bytes())
            .expect("bounded adapter correlation persists");
        let stored_bytes = stored.to_storage_bytes();
        let encoded = std::str::from_utf8(&stored_bytes).expect("adapter correlation is UTF-8");
        let restored = AdapterCorrelation::from_storage_value(encoded)
            .expect("canonical adapter correlation restores");
        assert!(issuer.restore(binding, &restored).is_ok());

        let altered = GradeBinding {
            seed: Seed::new(6),
            ..binding
        };
        assert!(issuer.restore(altered, &restored).is_err());
    }
}
