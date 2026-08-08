//! Contracted/self-hosted iMathAS scored-embed provider seam.
//!
//! This is intentionally a transport boundary, not an HTTP client. A later
//! adapter-owned private proxy can implement [`ScoredEmbedTransport`] without
//! changing question, server, or browser contracts. Generic hosted MyOpenMath
//! remains unavailable because the official protocol does not prove immutable
//! source execution or echo PLE's signed launch binding claims.

use async_trait::async_trait;
use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use question_model::generation::Seed;
use question_model::{
    ActivityTimestamp, ProblemId, QuestionAttemptId, QuestionDefinition, QuestionSource, TenantId,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::scored_embed::{
    LaunchLedgerStorageParts, SCORED_EMBED_BROKER_PROFILE_ID, ScoredEmbedFailure,
    ScoredEmbedLaunchLedger, ScoredEmbedNonce, ScoredEmbedProfileConfig, ScoredEmbedResultVerifier,
};
use crate::{
    DraftLocator, GradeBinding, ImathasAdapterError, ImathasProvider, ImathasSource,
    ProviderFailure, ProviderGradeRequest, ProviderRenderRequest, SafeProviderRender,
    ServerCorrelation, SupportedProfile, VerifiedProviderGrade, hex, sealed, verify_binding,
};

const MAX_SNAPSHOT_BYTES: usize = 1_048_576;
const MAX_RESULT_BYTES: usize = 8_192;
const MAX_LAUNCH_TTL_MILLIS: u64 = 300_000;
const MAX_PROXY_BODY_BYTES: usize = 262_144;

/// Protected deployment settings for one contracted/self-hosted provider.
/// No endpoint, browser URL, credential getter, or author-controlled field is
/// represented here.
pub struct ContractedScoredEmbedConfig {
    profile: ScoredEmbedProfileConfig,
    launch_signing_secret: Vec<u8>,
    result_verifier: ScoredEmbedResultVerifier,
    launch_ttl_millis: u64,
    max_snapshot_bytes: usize,
    max_result_bytes: usize,
}

impl std::fmt::Debug for ContractedScoredEmbedConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ContractedScoredEmbedConfig")
            .field("provider_key", &self.profile.provider_key())
            .field("profile", &SCORED_EMBED_BROKER_PROFILE_ID)
            .field("launch_signing_secret", &"REDACTED")
            .field("result_verifier", &self.result_verifier)
            .field("launch_ttl_millis", &self.launch_ttl_millis)
            .field("max_snapshot_bytes", &self.max_snapshot_bytes)
            .field("max_result_bytes", &self.max_result_bytes)
            .finish()
    }
}

impl ContractedScoredEmbedConfig {
    /// Creates a bounded configuration only for the explicit contracted profile.
    pub fn new(
        profile: ScoredEmbedProfileConfig,
        launch_signing_secret: impl AsRef<[u8]>,
        result_verification_secret: impl AsRef<[u8]>,
        launch_ttl_millis: u64,
    ) -> Result<Self, ScoredEmbedFailure> {
        let launch_signing_secret = launch_signing_secret.as_ref();
        if !profile.allows_published_server_grading()
            || launch_signing_secret.is_empty()
            || launch_ttl_millis == 0
            || launch_ttl_millis > MAX_LAUNCH_TTL_MILLIS
        {
            return Err(ScoredEmbedFailure::UnsupportedProfile);
        }
        Ok(Self {
            result_verifier: ScoredEmbedResultVerifier::new(
                profile.clone(),
                result_verification_secret,
            )?,
            profile,
            launch_signing_secret: launch_signing_secret.to_vec(),
            launch_ttl_millis,
            max_snapshot_bytes: MAX_SNAPSHOT_BYTES,
            max_result_bytes: MAX_RESULT_BYTES,
        })
    }

    /// Tightens protected server-side body limits for a provider deployment.
    pub fn with_limits(
        mut self,
        max_snapshot_bytes: usize,
        max_result_bytes: usize,
    ) -> Result<Self, ScoredEmbedFailure> {
        if max_snapshot_bytes == 0
            || max_snapshot_bytes > MAX_SNAPSHOT_BYTES
            || max_result_bytes == 0
            || max_result_bytes > MAX_RESULT_BYTES
        {
            return Err(ScoredEmbedFailure::InvalidLedger);
        }
        self.max_snapshot_bytes = max_snapshot_bytes;
        self.max_result_bytes = max_result_bytes;
        Ok(self)
    }

    fn supported_profile(&self) -> SupportedProfile {
        // Construction has already frozen these compatibility claims.
        SupportedProfile::new(SCORED_EMBED_BROKER_PROFILE_ID, true, true, true)
            .expect("contracted scored-embed profile is valid")
    }
}

/// Bounded failure classification for adapter-owned transport implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoredEmbedTransportFailure {
    Unavailable,
    Timeout,
    InvalidResponse,
    SourceChanged,
    Unsupported,
}

impl From<ScoredEmbedTransportFailure> for ProviderFailure {
    fn from(value: ScoredEmbedTransportFailure) -> Self {
        match value {
            ScoredEmbedTransportFailure::Unavailable => Self::Unavailable,
            ScoredEmbedTransportFailure::Timeout => Self::Timeout,
            ScoredEmbedTransportFailure::InvalidResponse
            | ScoredEmbedTransportFailure::SourceChanged => Self::InvalidResponse,
            ScoredEmbedTransportFailure::Unsupported => Self::UnsupportedProfile,
        }
    }
}

/// Exact bytes from an authorized contracted-provider snapshot endpoint.
/// The bytes are answer-bearing and never implement Debug or serde.
pub struct ContractedSnapshot(Vec<u8>);

impl ContractedSnapshot {
    /// Adapter-owned transport implementations create a bounded snapshot result.
    pub fn from_protected_bytes(bytes: Vec<u8>) -> Result<Self, ScoredEmbedTransportFailure> {
        if bytes.is_empty() || bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err(ScoredEmbedTransportFailure::InvalidResponse);
        }
        Ok(Self(bytes))
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for ContractedSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ContractedSnapshot")
            .field("bytes", &"REDACTED")
            .field("len", &self.0.len())
            .finish()
    }
}

/// Opaque provider-proxy handle. It must be a server-held identifier, never a
/// URL, browser capability, JWT, credential, or provider session cookie.
pub struct ProviderLaunchHandle(String);

impl ProviderLaunchHandle {
    /// Wraps an adapter/private-proxy generated opaque handle.
    pub fn from_server_handle(
        value: impl Into<String>,
    ) -> Result<Self, ScoredEmbedTransportFailure> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(ScoredEmbedTransportFailure::InvalidResponse);
        }
        Ok(Self(value))
    }

    #[allow(dead_code)]
    pub(crate) fn protected_value(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ProviderLaunchHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ProviderLaunchHandle(REDACTED)")
    }
}

/// Server-only request sent to a transport's authorized snapshot operation.
pub struct SnapshotTransportRequest<'a> {
    pub(crate) locator: &'a DraftLocator,
    pub(crate) provider_key: &'a str,
}
impl<'a> SnapshotTransportRequest<'a> {
    pub fn provider_key(&self) -> &'a str {
        self.provider_key
    }
    pub fn item_ref(&self) -> &'a str {
        self.locator.item_ref()
    }
}

/// Server-only immutable safe-render request. The snapshot remains private.
pub struct RenderTransportRequest<'a> {
    pub(crate) snapshot: &'a [u8],
    pub(crate) provider_key: &'a str,
    pub(crate) version: question_model::VersionId,
    pub(crate) seed: Seed,
}
impl<'a> RenderTransportRequest<'a> {
    pub fn snapshot(&self) -> &'a [u8] {
        self.snapshot
    }
    pub fn provider_key(&self) -> &'a str {
        self.provider_key
    }
    pub fn version(&self) -> question_model::VersionId {
        self.version
    }
    pub fn seed(&self) -> Seed {
        self.seed
    }
}

/// Protected request containing a signed launch JWT. It intentionally has no
/// public token getter: only an adapter-owned private transport can forward it
/// through the constrained server proxy.
pub struct ProtectedLaunchRequest {
    pub(crate) provider_key: String,
    pub(crate) item_ref: String,
    pub(crate) provider_seed: u16,
    pub(crate) source_digest: String,
    pub(crate) signed_launch_jwt: String,
}
impl std::fmt::Debug for ProtectedLaunchRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProtectedLaunchRequest")
            .field("provider_key", &self.provider_key)
            .field("item_ref", &self.item_ref)
            .field("provider_seed", &self.provider_seed)
            .field("source_digest", &"REDACTED")
            .field("signed_launch_jwt", &"REDACTED")
            .finish()
    }
}
impl ProtectedLaunchRequest {
    pub fn provider_key(&self) -> &str {
        &self.provider_key
    }
    pub fn item_ref(&self) -> &str {
        &self.item_ref
    }
    pub fn provider_seed(&self) -> u16 {
        self.provider_seed
    }
    #[allow(dead_code)]
    pub(crate) fn source_digest(&self) -> &str {
        &self.source_digest
    }
    #[allow(dead_code)]
    pub(crate) fn signed_launch_jwt(&self) -> &str {
        &self.signed_launch_jwt
    }
}

/// Server-only request for an already-created provider proxy session.
pub struct ResultTransportRequest<'a> {
    pub(crate) handle: &'a ProviderLaunchHandle,
    pub(crate) correlation: &'a ServerCorrelation,
    pub(crate) provider_key: &'a str,
}

/// The only provider-facing browser-proxy operation.  There is deliberately
/// no URL, header map, redirect flag, or provider cookie in this contract.
/// A transport maps this fixed activity resource onto its deployment-owned
/// upstream configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyMethod {
    Get,
    Post,
}

/// A private request bound to one restored opaque provider handle.  Server
/// routes cannot select an arbitrary upstream path or forward browser headers.
pub struct ProxyRequest<'a> {
    pub(crate) handle: &'a ProviderLaunchHandle,
    pub(crate) method: ProxyMethod,
    pub(crate) body: &'a [u8],
}
impl<'a> ProxyRequest<'a> {
    fn activity_get(handle: &'a ProviderLaunchHandle) -> Self {
        Self {
            handle,
            method: ProxyMethod::Get,
            body: &[],
        }
    }
    fn activity_post(
        handle: &'a ProviderLaunchHandle,
        body: &'a [u8],
    ) -> Result<Self, ScoredEmbedTransportFailure> {
        if body.is_empty() || body.len() > MAX_PROXY_BODY_BYTES {
            return Err(ScoredEmbedTransportFailure::InvalidResponse);
        }
        Ok(Self {
            handle,
            method: ProxyMethod::Post,
            body,
        })
    }
    pub fn method(&self) -> ProxyMethod {
        self.method
    }
    pub fn body(&self) -> &'a [u8] {
        self.body
    }
    #[allow(dead_code)]
    pub(crate) fn handle(&self) -> &ProviderLaunchHandle {
        self.handle
    }
}

/// A bounded provider document.  The server maps this closed type to a fixed
/// response-header allowlist; transport implementations cannot return a
/// redirect, `Set-Cookie`, external location, or arbitrary headers.
pub struct ProxyResponse {
    html: Vec<u8>,
}
impl ProxyResponse {
    pub fn protected_html(html: Vec<u8>) -> Result<Self, ScoredEmbedTransportFailure> {
        if html.is_empty() || html.len() > MAX_PROXY_BODY_BYTES {
            return Err(ScoredEmbedTransportFailure::InvalidResponse);
        }
        Ok(Self { html })
    }
    pub fn html(&self) -> &[u8] {
        &self.html
    }
}
impl std::fmt::Debug for ProxyResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyResponse")
            .field("html", &"REDACTED")
            .field("len", &self.html.len())
            .finish()
    }
}
impl<'a> ResultTransportRequest<'a> {
    pub fn provider_key(&self) -> &'a str {
        self.provider_key
    }
    #[allow(dead_code)]
    pub(crate) fn handle(&self) -> &ProviderLaunchHandle {
        self.handle
    }
    #[allow(dead_code)]
    pub(crate) fn correlation(&self) -> &ServerCorrelation {
        self.correlation
    }
}

/// Adapter-owned transport seam. Implementations must be server-only,
/// allowlisted, redirect-free, bounded, and never expose launch JWTs or result
/// tokens to browser/API DTOs. It deliberately has no generic URL method.
#[async_trait]
pub trait ScoredEmbedTransport: Send + Sync {
    async fn fetch_snapshot(
        &self,
        request: SnapshotTransportRequest<'_>,
    ) -> Result<ContractedSnapshot, ScoredEmbedTransportFailure>;
    async fn render_safe(
        &self,
        request: RenderTransportRequest<'_>,
    ) -> Result<SafeProviderRender, ScoredEmbedTransportFailure>;
    async fn start_protected_launch(
        &self,
        request: ProtectedLaunchRequest,
    ) -> Result<ProviderLaunchHandle, ScoredEmbedTransportFailure>;
    async fn fetch_signed_result(
        &self,
        request: ResultTransportRequest<'_>,
    ) -> Result<Vec<u8>, ScoredEmbedTransportFailure>;
    /// Fetches the fixed provider activity resource through a deployment-owned
    /// proxy. Browser headers, URLs, cookies, and redirects never cross this
    /// seam.
    async fn proxy_activity(
        &self,
        request: ProxyRequest<'_>,
    ) -> Result<ProxyResponse, ScoredEmbedTransportFailure>;
}

/// Adapter provider implementation that is ready for server composition but
/// requires an explicit protected launch session before a result can grade.
pub struct ContractedScoredEmbedProvider<T> {
    config: ContractedScoredEmbedConfig,
    transport: T,
}
impl<T> std::fmt::Debug for ContractedScoredEmbedProvider<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ContractedScoredEmbedProvider")
            .field("config", &self.config)
            .field("transport", &"REDACTED")
            .finish()
    }
}
impl<T: ScoredEmbedTransport> ContractedScoredEmbedProvider<T> {
    pub fn new(config: ContractedScoredEmbedConfig, transport: T) -> Self {
        Self { config, transport }
    }

    /// Maximum server-held lifetime for a launch.  This exposes only a bounded
    /// duration, never deployment credentials or a provider endpoint.
    pub fn launch_lifetime_millis(&self) -> u32 {
        self.config.launch_ttl_millis as u32
    }
    pub fn provider_key(&self) -> &str {
        self.config.profile.provider_key()
    }

    /// Starts a protected provider session after source-digest revalidation.
    /// The returned session is server-only and must be persisted atomically by
    /// the future broker store alongside its first-grade idempotency key.
    #[allow(clippy::too_many_arguments)]
    pub async fn begin_launch(
        &self,
        question: &QuestionDefinition,
        source: &ImathasSource,
        tenant: TenantId,
        attempt: QuestionAttemptId,
        seed: Seed,
        correlation: ServerCorrelation,
        nonce: ScoredEmbedNonce,
        now: ActivityTimestamp,
    ) -> Result<ContractedLaunchSession, ImathasAdapterError> {
        verify_binding(question, source)?;
        if source.provider != self.config.profile.provider_key() {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        let QuestionSource::Imathas {
            integration_profile,
            ..
        } = &question.source
        else {
            return Err(ImathasAdapterError::UnsupportedSource);
        };
        if integration_profile != SCORED_EMBED_BROKER_PROFILE_ID {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        let locator = DraftLocator::from_draft(&question_model::DraftQuestionSource::Imathas {
            provider: source.provider.clone(),
            item_ref: source.item_ref.clone(),
        })?;
        let fresh = self
            .transport
            .fetch_snapshot(SnapshotTransportRequest {
                locator: &locator,
                provider_key: self.config.profile.provider_key(),
            })
            .await
            .map_err(map_transport)?;
        if fresh.bytes().len() > self.config.max_snapshot_bytes
            || hex(Sha256::digest(fresh.bytes()).as_slice()) != source.artifact.sha256
        {
            return Err(ImathasAdapterError::SourceChecksumMismatch);
        }
        let binding = GradeBinding {
            tenant,
            attempt,
            problem: question.problem,
            version: question.version,
            seed,
        };
        let expiry = now
            .as_unix_millis()
            .checked_add(self.config.launch_ttl_millis as i64)
            .ok_or(ImathasAdapterError::InvalidCorrelation)?;
        let ledger = ScoredEmbedLaunchLedger::begin(
            &self.config.profile,
            binding,
            &source.item_ref,
            &source.artifact.sha256,
            ActivityTimestamp::from_unix_millis(expiry),
            correlation,
            nonce,
        )
        .map_err(ScoredEmbedFailure::into_adapter_error)?;
        let claims = ledger.signed_launch_claims();
        let protected = ProtectedLaunchRequest {
            provider_key: self.config.profile.provider_key().to_owned(),
            item_ref: source.item_ref.clone(),
            provider_seed: ledger.provider_seed(),
            source_digest: source.artifact.sha256.clone(),
            signed_launch_jwt: signed_launch_jwt(
                &self.config.launch_signing_secret,
                &source.item_ref,
                ledger.provider_seed(),
                expiry,
                claims.nonce(),
                claims.binding_digest(),
            )?,
        };
        let handle = self
            .transport
            .start_protected_launch(protected)
            .await
            .map_err(map_transport)?;
        Ok(ContractedLaunchSession { ledger, handle })
    }

    /// Retrieves the provider response through the protected transport and
    /// passes it to the sealed scored-embed verifier. No browser callback can
    /// reach this method's token input.
    pub async fn retrieve_and_verify(
        &self,
        session: &mut ContractedLaunchSession,
        now: ActivityTimestamp,
    ) -> Result<VerifiedProviderGrade, ImathasAdapterError> {
        session
            .ledger
            .ensure_eligible_at(now)
            .map_err(ScoredEmbedFailure::into_adapter_error)?;
        let bytes = self
            .transport
            .fetch_signed_result(ResultTransportRequest {
                handle: &session.handle,
                correlation: session.ledger.correlation(),
                provider_key: self.config.profile.provider_key(),
            })
            .await
            .map_err(map_transport)?;
        if bytes.is_empty() || bytes.len() > self.config.max_result_bytes {
            return Err(ImathasAdapterError::Provider(
                ProviderFailure::InvalidResponse,
            ));
        }
        let token = std::str::from_utf8(&bytes)
            .map_err(|_| ImathasAdapterError::Provider(ProviderFailure::InvalidResponse))?;
        self.config
            .result_verifier
            .verify_result(&mut session.ledger, token, now)
            .map_err(ScoredEmbedFailure::into_adapter_error)
    }

    /// Proxies the one fixed activity resource for a restored session.  This
    /// preserves the session's provider handle inside the adapter and keeps
    /// route code from selecting URLs or headers.
    pub async fn proxy_activity(
        &self,
        session: &ContractedLaunchSession,
        method: ProxyMethod,
        body: &[u8],
        now: ActivityTimestamp,
    ) -> Result<ProxyResponse, ImathasAdapterError> {
        session
            .ledger
            .ensure_eligible_at(now)
            .map_err(ScoredEmbedFailure::into_adapter_error)?;
        let request = match method {
            ProxyMethod::Get if body.is_empty() => ProxyRequest::activity_get(&session.handle),
            ProxyMethod::Post => {
                ProxyRequest::activity_post(&session.handle, body).map_err(map_transport)?
            }
            _ => {
                return Err(ImathasAdapterError::Provider(
                    ProviderFailure::InvalidResponse,
                ));
            }
        };
        self.transport
            .proxy_activity(request)
            .await
            .map_err(map_transport)
    }
}

impl<T: ScoredEmbedTransport> sealed::ProviderSealed for ContractedScoredEmbedProvider<T> {}

#[async_trait]
impl<T: ScoredEmbedTransport> ImathasProvider for ContractedScoredEmbedProvider<T> {
    async fn snapshot(
        &self,
        locator: &DraftLocator,
    ) -> Result<(Vec<u8>, SupportedProfile), ProviderFailure> {
        if locator.provider() != self.config.profile.provider_key() {
            return Err(ProviderFailure::UnsupportedProfile);
        }
        let snapshot = self
            .transport
            .fetch_snapshot(SnapshotTransportRequest {
                locator,
                provider_key: self.config.profile.provider_key(),
            })
            .await
            .map_err(ProviderFailure::from)?;
        if snapshot.bytes().len() > self.config.max_snapshot_bytes {
            return Err(ProviderFailure::InvalidResponse);
        }
        Ok((snapshot.0, self.config.supported_profile()))
    }

    async fn render(
        &self,
        request: ProviderRenderRequest<'_>,
    ) -> Result<SafeProviderRender, ProviderFailure> {
        if request.profile != SCORED_EMBED_BROKER_PROFILE_ID
            || request.snapshot.len() > self.config.max_snapshot_bytes
        {
            return Err(ProviderFailure::UnsupportedProfile);
        }
        self.transport
            .render_safe(RenderTransportRequest {
                snapshot: request.snapshot,
                provider_key: self.config.profile.provider_key(),
                version: request.version,
                seed: request.seed,
            })
            .await
            .map_err(ProviderFailure::from)
    }

    async fn verify_grade(
        &self,
        _request: ProviderGradeRequest<'_>,
    ) -> Result<VerifiedProviderGrade, ProviderFailure> {
        // The generic adapter method has no protected launch session. Refuse
        // rather than accepting any caller-provided token or claiming a grade.
        Err(ProviderFailure::UnsupportedProfile)
    }
}

/// Server-only launch state. It intentionally cannot serialize, expose a
/// provider URL/JWT, or be constructed from browser data.
pub struct ContractedLaunchSession {
    ledger: ScoredEmbedLaunchLedger,
    handle: ProviderLaunchHandle,
}
impl std::fmt::Debug for ContractedLaunchSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ContractedLaunchSession")
            .field("ledger", &self.ledger)
            .field("handle", &self.handle)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractedLaunchExpectation {
    binding: GradeBinding,
    provider_key: String,
    source_digest: String,
}
impl ContractedLaunchExpectation {
    pub fn new(
        binding: GradeBinding,
        provider_key: impl Into<String>,
        source_digest: impl Into<String>,
    ) -> Result<Self, ImathasAdapterError> {
        let provider_key = provider_key.into();
        let source_digest = source_digest.into();
        if !valid_provider(&provider_key) || !valid_digest(&source_digest) {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        Ok(Self {
            binding,
            provider_key,
            source_digest,
        })
    }
}

/// Opaque non-serde launch state for protected tenant-owned storage.
#[derive(Clone, PartialEq, Eq)]
pub struct PersistedContractedLaunchSession(String);
impl std::fmt::Debug for PersistedContractedLaunchSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PersistedContractedLaunchSession(REDACTED)")
    }
}
impl PersistedContractedLaunchSession {
    pub fn to_storage_value(&self) -> String {
        self.0.clone()
    }
    pub fn from_storage_value(value: &str) -> Result<Self, ImathasAdapterError> {
        if value.is_empty() || value.len() > 8192 || !value.is_ascii() {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| ImathasAdapterError::InvalidCorrelation)?;
        if base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes) != value {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        Ok(Self(value.to_owned()))
    }
}

/// MAC codec for replica-safe launch state; its secret is server-held only.
pub struct LaunchSessionCodec {
    secret: [u8; 32],
}
impl std::fmt::Debug for LaunchSessionCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LaunchSessionCodec(REDACTED)")
    }
}
impl LaunchSessionCodec {
    pub fn from_server_secret(secret: [u8; 32]) -> Result<Self, ImathasAdapterError> {
        if secret.iter().all(|byte| *byte == 0) {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        Ok(Self { secret })
    }
    pub fn seal(
        &self,
        session: &ContractedLaunchSession,
    ) -> Result<PersistedContractedLaunchSession, ImathasAdapterError> {
        let p = session.ledger.storage_parts();
        let mut data = Vec::with_capacity(512);
        data.extend_from_slice(b"PLEIMLS1");
        data.push(1);
        write_binding(&mut data, p.binding);
        for value in [
            &p.provider_key,
            &p.provider_question_id,
            &p.source_digest,
            &p.profile,
        ] {
            write_text(&mut data, value)?;
        }
        data.extend_from_slice(&p.provider_seed.to_be_bytes());
        data.extend_from_slice(&p.expires_at.as_unix_millis().to_be_bytes());
        write_text(&mut data, &p.correlation)?;
        data.extend_from_slice(&p.nonce);
        write_text(&mut data, &p.binding_digest)?;
        data.push(u8::from(p.consumed));
        write_text(&mut data, session.handle.protected_value())?;
        data.extend_from_slice(&self.mac(&data));
        Ok(PersistedContractedLaunchSession(
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data),
        ))
    }
    pub fn restore(
        &self,
        value: &PersistedContractedLaunchSession,
        expected: &ContractedLaunchExpectation,
    ) -> Result<ContractedLaunchSession, ImathasAdapterError> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&value.0)
            .map_err(|_| ImathasAdapterError::InvalidCorrelation)?;
        if bytes.len() < 256 || bytes.len() > 6144 {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        let (data, mac) = bytes.split_at(bytes.len() - 32);
        if !constant_time_equal(&self.mac(data), mac) {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        let mut c = Cursor::new(data);
        if c.take(8)? != b"PLEIMLS1" || c.u8()? != 1 {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        let binding = read_binding(&mut c)?;
        let provider_key = c.text()?;
        let provider_question_id = c.text()?;
        let source_digest = c.text()?;
        let profile = c.text()?;
        let provider_seed = c.u16()?;
        let expires_at = ActivityTimestamp::from_unix_millis(c.i64()?);
        let correlation = c.text()?;
        let nonce: [u8; 32] = c
            .take(32)?
            .try_into()
            .map_err(|_| ImathasAdapterError::InvalidCorrelation)?;
        let binding_digest = c.text()?;
        let consumed = match c.u8()? {
            0 => false,
            1 => true,
            _ => return Err(ImathasAdapterError::InvalidCorrelation),
        };
        let handle = ProviderLaunchHandle::from_server_handle(c.text()?).map_err(map_transport)?;
        if !c.finished()
            || binding != expected.binding
            || provider_key != expected.provider_key
            || source_digest != expected.source_digest
        {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        let ledger = ScoredEmbedLaunchLedger::from_storage_parts(LaunchLedgerStorageParts {
            binding,
            provider_key,
            provider_question_id,
            source_digest,
            profile,
            provider_seed,
            expires_at,
            correlation,
            nonce,
            binding_digest,
            consumed,
        })
        .map_err(ScoredEmbedFailure::into_adapter_error)?;
        Ok(ContractedLaunchSession { ledger, handle })
    }
    fn mac(&self, data: &[u8]) -> [u8; 32] {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret).expect("fixed key");
        mac.update(b"ple:imathas:launch-session-codec:v1");
        mac.update(data);
        mac.finalize().into_bytes().into()
    }
}

fn write_binding(data: &mut Vec<u8>, binding: GradeBinding) {
    for id in [
        binding.tenant.as_uuid(),
        binding.attempt.as_uuid(),
        binding.problem.as_uuid(),
        binding.version.as_uuid(),
    ] {
        data.extend_from_slice(id.as_bytes());
    }
    data.extend_from_slice(&binding.seed.value().to_be_bytes());
}
fn read_binding(c: &mut Cursor<'_>) -> Result<GradeBinding, ImathasAdapterError> {
    let id = |c: &mut Cursor<'_>| -> Result<Uuid, ImathasAdapterError> {
        Ok(Uuid::from_bytes(
            c.take(16)?
                .try_into()
                .map_err(|_| ImathasAdapterError::InvalidCorrelation)?,
        ))
    };
    Ok(GradeBinding {
        tenant: TenantId::from_uuid(id(c)?),
        attempt: QuestionAttemptId::from_uuid(id(c)?),
        problem: ProblemId::from_uuid(id(c)?),
        version: question_model::VersionId::from_uuid(id(c)?),
        seed: Seed::new(c.u64()?),
    })
}
fn write_text(data: &mut Vec<u8>, value: &str) -> Result<(), ImathasAdapterError> {
    if value.is_empty() || value.len() > 512 || !value.is_ascii() {
        return Err(ImathasAdapterError::InvalidCorrelation);
    }
    let length: u16 = value
        .len()
        .try_into()
        .map_err(|_| ImathasAdapterError::InvalidCorrelation)?;
    data.extend_from_slice(&length.to_be_bytes());
    data.extend_from_slice(value.as_bytes());
    Ok(())
}
struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], ImathasAdapterError> {
        let end = self
            .at
            .checked_add(n)
            .ok_or(ImathasAdapterError::InvalidCorrelation)?;
        let out = self
            .data
            .get(self.at..end)
            .ok_or(ImathasAdapterError::InvalidCorrelation)?;
        self.at = end;
        Ok(out)
    }
    fn u8(&mut self) -> Result<u8, ImathasAdapterError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ImathasAdapterError> {
        Ok(u16::from_be_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| ImathasAdapterError::InvalidCorrelation)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, ImathasAdapterError> {
        Ok(u64::from_be_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| ImathasAdapterError::InvalidCorrelation)?,
        ))
    }
    fn i64(&mut self) -> Result<i64, ImathasAdapterError> {
        Ok(i64::from_be_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| ImathasAdapterError::InvalidCorrelation)?,
        ))
    }
    fn text(&mut self) -> Result<String, ImathasAdapterError> {
        let length = usize::from(self.u16()?);
        let v = std::str::from_utf8(self.take(length)?)
            .map_err(|_| ImathasAdapterError::InvalidCorrelation)?;
        if v.is_empty() || !v.is_ascii() {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        Ok(v.to_owned())
    }
    fn finished(&self) -> bool {
        self.at == self.data.len()
    }
}
fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && left.iter().zip(right).fold(0u8, |v, (a, b)| v | (a ^ b)) == 0
}
fn valid_provider(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

fn map_transport(error: ScoredEmbedTransportFailure) -> ImathasAdapterError {
    ImathasAdapterError::Provider(error.into())
}

fn signed_launch_jwt(
    secret: &[u8],
    item_ref: &str,
    provider_seed: u16,
    expiry_millis: i64,
    nonce: &str,
    binding: &str,
) -> Result<String, ImathasAdapterError> {
    let exp = expiry_millis
        .checked_add(999)
        .and_then(|value| value.checked_div(1_000))
        .ok_or(ImathasAdapterError::InvalidCorrelation)?;
    let payload = serde_json::json!({
        "id": item_ref,
        "seed": provider_seed,
        "exp": exp,
        "ple_nonce": nonce,
        "ple_binding": binding,
    });
    let payload =
        serde_json::to_vec(&payload).map_err(|_| ImathasAdapterError::InvalidCorrelation)?;
    let base64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = base64.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = base64.encode(payload);
    let signed = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .map_err(|_| ImathasAdapterError::InvalidCorrelation)?;
    mac.update(signed.as_bytes());
    Ok(format!(
        "{signed}.{}",
        base64.encode(mac.finalize().into_bytes())
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use base64::Engine as _;
    use hmac::{Hmac, KeyInit, Mac};
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        GradingDefinition, ObjectId, ProblemId, QuestionMetadata, SourceArtifact, VersionId,
        WorkspaceId,
    };
    use uuid::Uuid;

    use super::*;
    use crate::CorrelationIssuer;

    #[derive(Clone)]
    struct RecordedTransport {
        snapshot: Arc<Mutex<Result<Vec<u8>, ScoredEmbedTransportFailure>>>,
        result: Arc<Mutex<Result<Vec<u8>, ScoredEmbedTransportFailure>>>,
        launches: Arc<Mutex<Vec<(String, String, String)>>>,
        result_calls: Arc<AtomicUsize>,
    }

    impl RecordedTransport {
        fn stable() -> Self {
            Self {
                snapshot: Arc::new(Mutex::new(Ok(br#"{"recorded":true}"#.to_vec()))),
                result: Arc::new(Mutex::new(Ok(Vec::new()))),
                launches: Arc::new(Mutex::new(Vec::new())),
                result_calls: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    #[async_trait]
    impl ScoredEmbedTransport for RecordedTransport {
        async fn fetch_snapshot(
            &self,
            _request: SnapshotTransportRequest<'_>,
        ) -> Result<ContractedSnapshot, ScoredEmbedTransportFailure> {
            let bytes = self.snapshot.lock().unwrap().clone()?;
            ContractedSnapshot::from_protected_bytes(bytes)
        }
        async fn render_safe(
            &self,
            _request: RenderTransportRequest<'_>,
        ) -> Result<SafeProviderRender, ScoredEmbedTransportFailure> {
            Ok(SafeProviderRender {
                title: "Recorded broker question".into(),
                prompt: vec![ContentBlock::Text {
                    markdown: "Use the protected activity.".into(),
                }],
            })
        }
        async fn start_protected_launch(
            &self,
            request: ProtectedLaunchRequest,
        ) -> Result<ProviderLaunchHandle, ScoredEmbedTransportFailure> {
            self.launches.lock().unwrap().push((
                request.item_ref().to_owned(),
                request.source_digest().to_owned(),
                request.signed_launch_jwt().to_owned(),
            ));
            ProviderLaunchHandle::from_server_handle("recorded-proxy-session")
        }
        async fn fetch_signed_result(
            &self,
            request: ResultTransportRequest<'_>,
        ) -> Result<Vec<u8>, ScoredEmbedTransportFailure> {
            self.result_calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(request.handle().protected_value(), "recorded-proxy-session");
            assert!(format!("{:?}", request.correlation()).contains("REDACTED"));
            self.result.lock().unwrap().clone()
        }
        async fn proxy_activity(
            &self,
            request: ProxyRequest<'_>,
        ) -> Result<ProxyResponse, ScoredEmbedTransportFailure> {
            assert_eq!(request.handle().protected_value(), "recorded-proxy-session");
            assert!(matches!(
                request.method(),
                ProxyMethod::Get | ProxyMethod::Post
            ));
            ProxyResponse::protected_html(
                b"<!doctype html><title>Recorded protected activity</title>".to_vec(),
            )
        }
    }

    fn config() -> ContractedScoredEmbedConfig {
        ContractedScoredEmbedConfig::new(
            ScoredEmbedProfileConfig::contracted_self_hosted("institution-imathas", true, true)
                .unwrap(),
            b"launch-secret",
            b"result-secret",
            30_000,
        )
        .unwrap()
    }

    fn question_and_source() -> (QuestionDefinition, ImathasSource) {
        let problem = ProblemId::from_uuid(Uuid::from_u128(1));
        let version = VersionId::from_uuid(Uuid::from_u128(2));
        let bytes = br#"{"recorded":true}"#.to_vec();
        let digest = hex(Sha256::digest(&bytes).as_slice());
        let object = ObjectId::from_uuid(Uuid::from_u128(3));
        let source_artifact = SourceArtifact {
            object,
            sha256: digest.clone(),
        };
        let question = QuestionDefinition {
            problem,
            version,
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(4)),
            source: QuestionSource::Imathas {
                provider: "institution-imathas".into(),
                item_ref: "17".into(),
                snapshot: object,
                snapshot_sha256: digest,
                integration_profile: SCORED_EMBED_BROKER_PROFILE_ID.into(),
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
                title: "Recorded broker question".into(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".into(),
            },
        };
        let source = ImathasSource {
            problem,
            version,
            artifact: source_artifact,
            provider: "institution-imathas".into(),
            item_ref: "17".into(),
            profile: SCORED_EMBED_BROKER_PROFILE_ID.into(),
            bytes,
        };
        (question, source)
    }

    fn correlation(question: &QuestionDefinition, seed: Seed) -> ServerCorrelation {
        let binding = GradeBinding {
            tenant: TenantId::from_uuid(Uuid::from_u128(5)),
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
            problem: question.problem,
            version: question.version,
            seed,
        };
        let issuer = CorrelationIssuer::from_server_secret([3; 32]);
        issuer.restore(binding, &issuer.begin(binding)).unwrap()
    }

    fn result_token(session: &ContractedLaunchSession, score: f64) -> Vec<u8> {
        let claims = session.ledger.signed_launch_claims();
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(
            r#"{{"id":17,"score":{score},"ple_nonce":"{}","ple_binding":"{}"}}"#,
            claims.nonce(),
            claims.binding_digest(),
        ));
        let signed = format!("{header}.{payload}");
        let mut mac = Hmac::<Sha256>::new_from_slice(b"result-secret").unwrap();
        mac.update(signed.as_bytes());
        format!(
            "{signed}.{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        )
        .into_bytes()
    }

    async fn launch(
        provider: &ContractedScoredEmbedProvider<RecordedTransport>,
        question: &QuestionDefinition,
        source: &ImathasSource,
        nonce: u8,
    ) -> ContractedLaunchSession {
        provider
            .begin_launch(
                question,
                source,
                TenantId::from_uuid(Uuid::from_u128(5)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                Seed::new(10_001),
                correlation(question, Seed::new(10_001)),
                ScoredEmbedNonce::from_server_random([nonce; 32]).unwrap(),
                ActivityTimestamp::from_unix_millis(1_000),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn recorded_transport_launches_and_verifies_only_a_bound_result() {
        let transport = RecordedTransport::stable();
        let provider = ContractedScoredEmbedProvider::new(config(), transport.clone());
        let (question, source) = question_and_source();
        let mut session = launch(&provider, &question, &source, 7).await;
        *transport.result.lock().unwrap() = Ok(result_token(&session, 1.0));
        assert!(
            provider
                .retrieve_and_verify(&mut session, ActivityTimestamp::from_unix_millis(2_000))
                .await
                .unwrap()
                .result
                .correct
        );
        let launches = transport.launches.lock().unwrap();
        assert_eq!(launches.len(), 1);
        assert!(!launches[0].2.contains("result-secret"));
        assert!(!format!("{:?}", session).contains("eyJ"));
    }

    #[tokio::test]
    async fn mutation_outage_timeout_oversize_and_cross_binding_refuse() {
        let (question, source) = question_and_source();
        let transport = RecordedTransport::stable();
        let provider = ContractedScoredEmbedProvider::new(config(), transport.clone());
        *transport.snapshot.lock().unwrap() = Ok(b"changed".to_vec());
        assert_eq!(
            provider
                .begin_launch(
                    &question,
                    &source,
                    TenantId::from_uuid(Uuid::from_u128(5)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                    Seed::new(10_001),
                    correlation(&question, Seed::new(10_001)),
                    ScoredEmbedNonce::from_server_random([7; 32]).unwrap(),
                    ActivityTimestamp::from_unix_millis(1_000),
                )
                .await
                .unwrap_err(),
            ImathasAdapterError::SourceChecksumMismatch
        );
        *transport.snapshot.lock().unwrap() = Err(ScoredEmbedTransportFailure::Unavailable);
        assert!(matches!(
            provider
                .begin_launch(
                    &question,
                    &source,
                    TenantId::from_uuid(Uuid::from_u128(5)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                    Seed::new(10_001),
                    correlation(&question, Seed::new(10_001)),
                    ScoredEmbedNonce::from_server_random([7; 32]).unwrap(),
                    ActivityTimestamp::from_unix_millis(1_000),
                )
                .await,
            Err(ImathasAdapterError::Provider(ProviderFailure::Unavailable))
        ));
        *transport.snapshot.lock().unwrap() = Ok(br#"{"recorded":true}"#.to_vec());
        let mut first = launch(&provider, &question, &source, 7).await;
        let second = launch(&provider, &question, &source, 8).await;
        *transport.result.lock().unwrap() = Ok(result_token(&second, 1.0));
        assert_eq!(
            provider
                .retrieve_and_verify(&mut first, ActivityTimestamp::from_unix_millis(2_000))
                .await,
            Err(ImathasAdapterError::VerificationRefused)
        );
        *transport.result.lock().unwrap() = Err(ScoredEmbedTransportFailure::Timeout);
        let mut third = launch(&provider, &question, &source, 9).await;
        assert!(matches!(
            provider
                .retrieve_and_verify(&mut third, ActivityTimestamp::from_unix_millis(2_000))
                .await,
            Err(ImathasAdapterError::Provider(ProviderFailure::Timeout))
        ));
        *transport.result.lock().unwrap() = Ok(vec![b'x'; MAX_RESULT_BYTES + 1]);
        let mut fourth = launch(&provider, &question, &source, 10).await;
        assert!(matches!(
            provider
                .retrieve_and_verify(&mut fourth, ActivityTimestamp::from_unix_millis(2_000))
                .await,
            Err(ImathasAdapterError::Provider(
                ProviderFailure::InvalidResponse
            ))
        ));
    }

    #[tokio::test]
    async fn cross_provider_draft_and_published_sources_refuse_before_transport() {
        let transport = RecordedTransport::stable();
        let provider = ContractedScoredEmbedProvider::new(config(), transport.clone());
        let foreign_draft = question_model::DraftQuestionSource::Imathas {
            provider: "foreign-imathas".into(),
            item_ref: "17".into(),
        };
        let locator = DraftLocator::from_draft(&foreign_draft).unwrap();
        assert_eq!(
            provider.snapshot(&locator).await,
            Err(ProviderFailure::UnsupportedProfile)
        );
        assert!(transport.launches.lock().unwrap().is_empty());

        let (mut question, mut source) = question_and_source();
        source.provider = "foreign-imathas".into();
        if let QuestionSource::Imathas { provider, .. } = &mut question.source {
            *provider = "foreign-imathas".into();
        }
        assert!(matches!(
            provider
                .begin_launch(
                    &question,
                    &source,
                    TenantId::from_uuid(Uuid::from_u128(5)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                    Seed::new(10_001),
                    correlation(&question, Seed::new(10_001)),
                    ScoredEmbedNonce::from_server_random([7; 32]).unwrap(),
                    ActivityTimestamp::from_unix_millis(1_000),
                )
                .await,
            Err(ImathasAdapterError::UnsupportedProfile)
        ));
        assert!(transport.launches.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn launch_session_storage_is_replica_safe_and_hostile_input_refuses() {
        let transport = RecordedTransport::stable();
        let provider = ContractedScoredEmbedProvider::new(config(), transport.clone());
        let (question, source) = question_and_source();
        let session = launch(&provider, &question, &source, 7).await;
        let codec = LaunchSessionCodec::from_server_secret([11; 32]).unwrap();
        let expected = ContractedLaunchExpectation::new(
            GradeBinding {
                tenant: TenantId::from_uuid(Uuid::from_u128(5)),
                attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                problem: question.problem,
                version: question.version,
                seed: Seed::new(10_001),
            },
            "institution-imathas",
            source.artifact.sha256.clone(),
        )
        .unwrap();
        let persisted = codec.seal(&session).unwrap();
        let storage = persisted.to_storage_value();
        assert!(!format!("{persisted:?}").contains("17"));
        assert!(!storage.contains("eyJ"));
        let persisted = PersistedContractedLaunchSession::from_storage_value(&storage).unwrap();
        let mut restored = codec.restore(&persisted, &expected).unwrap();
        *transport.result.lock().unwrap() = Ok(result_token(&restored, 1.0));
        assert!(
            provider
                .retrieve_and_verify(&mut restored, ActivityTimestamp::from_unix_millis(2_000))
                .await
                .unwrap()
                .result
                .correct
        );

        let mut mutated = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&storage)
            .unwrap();
        mutated[20] ^= 1;
        let mutated = PersistedContractedLaunchSession::from_storage_value(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mutated),
        )
        .unwrap();
        assert!(codec.restore(&mutated, &expected).is_err());
        assert!(
            LaunchSessionCodec::from_server_secret([12; 32])
                .unwrap()
                .restore(&persisted, &expected)
                .is_err()
        );
        assert!(
            PersistedContractedLaunchSession::from_storage_value(&storage[..storage.len() - 1])
                .is_err()
        );
        assert!(
            PersistedContractedLaunchSession::from_storage_value(&(storage.clone() + "=")).is_err()
        );
        assert!(PersistedContractedLaunchSession::from_storage_value(&"a".repeat(8_193)).is_err());
        let wrong_version = ContractedLaunchExpectation::new(
            GradeBinding {
                version: VersionId::from_uuid(Uuid::from_u128(99)),
                ..expected.binding
            },
            "institution-imathas",
            source.artifact.sha256,
        )
        .unwrap();
        assert!(codec.restore(&persisted, &wrong_version).is_err());
    }

    #[tokio::test]
    async fn restored_expired_or_consumed_sessions_do_not_fetch_provider_results() {
        let transport = RecordedTransport::stable();
        let provider = ContractedScoredEmbedProvider::new(config(), transport.clone());
        let (question, source) = question_and_source();
        let expected = ContractedLaunchExpectation::new(
            GradeBinding {
                tenant: TenantId::from_uuid(Uuid::from_u128(5)),
                attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                problem: question.problem,
                version: question.version,
                seed: Seed::new(10_001),
            },
            "institution-imathas",
            source.artifact.sha256.clone(),
        )
        .unwrap();
        let codec = LaunchSessionCodec::from_server_secret([11; 32]).unwrap();

        let mut expired = launch(&provider, &question, &source, 7).await;
        let mut parts = expired.ledger.storage_parts();
        parts.expires_at = ActivityTimestamp::from_unix_millis(999);
        expired.ledger = ScoredEmbedLaunchLedger::from_storage_parts(parts).unwrap();
        let expired_blob = codec.seal(&expired).unwrap();
        let mut expired = codec.restore(&expired_blob, &expected).unwrap();
        let before = transport.result_calls.load(Ordering::SeqCst);
        let before_blob = codec.seal(&expired).unwrap().to_storage_value();
        assert_eq!(
            provider
                .retrieve_and_verify(&mut expired, ActivityTimestamp::from_unix_millis(1_000))
                .await,
            Err(ImathasAdapterError::InvalidCorrelation)
        );
        assert_eq!(transport.result_calls.load(Ordering::SeqCst), before);
        assert_eq!(
            codec.seal(&expired).unwrap().to_storage_value(),
            before_blob
        );

        let mut consumed = launch(&provider, &question, &source, 8).await;
        *transport.result.lock().unwrap() = Ok(result_token(&consumed, 1.0));
        provider
            .retrieve_and_verify(&mut consumed, ActivityTimestamp::from_unix_millis(2_000))
            .await
            .unwrap();
        let consumed_blob = codec.seal(&consumed).unwrap();
        let mut consumed = codec.restore(&consumed_blob, &expected).unwrap();
        let before = transport.result_calls.load(Ordering::SeqCst);
        let before_blob = codec.seal(&consumed).unwrap().to_storage_value();
        assert_eq!(
            provider
                .retrieve_and_verify(&mut consumed, ActivityTimestamp::from_unix_millis(2_000))
                .await,
            Err(ImathasAdapterError::InvalidCorrelation)
        );
        assert_eq!(transport.result_calls.load(Ordering::SeqCst), before);
        assert_eq!(
            codec.seal(&consumed).unwrap().to_storage_value(),
            before_blob
        );
    }
}
