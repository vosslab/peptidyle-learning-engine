//! Contracted/self-hosted iMathAS scored-embed provider seam.
//!
//! This is intentionally a transport boundary, not an HTTP client. A later
//! adapter-owned private proxy can implement [`ScoredEmbedTransport`] without
//! changing question, server, or browser contracts. Generic hosted MyOpenMath
//! remains unavailable because the official protocol does not prove immutable
//! source execution or echo PLE's signed launch binding claims.

use async_trait::async_trait;
use question_model::generation::Seed;
use question_model::{ActivityTimestamp, QuestionAttemptId, QuestionDefinition, QuestionSource};
use sha2::{Digest, Sha256};

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

mod launch;
mod protocol;
mod result;

pub use launch::{
    ContractedLaunchExpectation, LaunchSessionCodec, PersistedContractedLaunchSession,
};

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
    pub(crate) question_version: question_model::QuestionVersionReference,
    pub(crate) seed: Seed,
}
impl<'a> RenderTransportRequest<'a> {
    pub fn snapshot(&self) -> &'a [u8] {
        self.snapshot
    }
    pub fn provider_key(&self) -> &'a str {
        self.provider_key
    }
    pub fn question_version(&self) -> &question_model::QuestionVersionReference {
        &self.question_version
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
    /// Retrieves a signed grade using a safe, idempotent HTTP GET only.
    /// Implementations must not dispatch a provider mutation from this method.
    async fn fetch_signed_grade_get(
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
            attempt,
            question_version: question_model::QuestionVersionReference {
                question_id: question.question_id.clone(),
                version_number: question.version_number,
            },
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
            signed_launch_jwt: protocol::signed_launch_jwt(
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
        result::retrieve_and_verify(&self.transport, &self.config, session, now).await
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
                question_version: request.question_version,
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

fn map_transport(error: ScoredEmbedTransportFailure) -> ImathasAdapterError {
    ImathasAdapterError::Provider(error.into())
}

#[cfg(test)]
#[path = "broker_provider/tests.rs"]
mod tests;
