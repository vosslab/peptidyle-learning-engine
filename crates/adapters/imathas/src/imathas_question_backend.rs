//! iMathAS Question Backend seam.
//!
//! This is intentionally a transport boundary, not an HTTP client. A later
//! adapter-owned private proxy can implement [`ImathasQuestionBackendTransport`] without
//! changing question, server, or browser contracts. Generic hosted MyOpenMath
//! remains unavailable because the official protocol does not prove immutable
//! source execution or echo PLE's signed launch binding claims.

use async_trait::async_trait;
use base64::Engine as _;
use question_model::generation::QuestionSeed;
use question_model::{DraftImathasQuestionBackendBinding, QuestionBackend, QuestionRevision, Timestamp};
use sha2::{Digest, Sha256};

use crate::result_verification::{
    IMATHAS_GRADING_PROFILE_ID, ImathasGradingFailure, ImathasGradingProfile,
    ImathasResultVerifier, launch_binding_digest, normalize_imathas_seed,
};
use crate::{
    ImathasAdapterError, ImathasQuestionBackendFailure, ImathasQuestionLocation,
    ImathasRenderRequest, ImathasResultRequest, QuestionBackend, ResolvedImathasQuestionSource,
    SafeImathasQuestionRender, SupportedImathasProfile, VerifiedImathasResult, hex, sealed,
    verify_binding,
};

const MAX_SNAPSHOT_BYTES: usize = 1_048_576;
const MAX_RESULT_BYTES: usize = 8_192;
const MAX_LAUNCH_TTL_MILLIS: u64 = 300_000;
const MAX_PROXY_BODY_BYTES: usize = 262_144;

mod launch;
mod protocol;
mod result;

pub use launch::{ImathasLaunchState, ImathasSessionAuthenticationCodec};

/// Protected deployment settings for one iMathAS Question Backend deployment.
/// No endpoint, browser URL, credential getter, or author-controlled field is
/// represented here.
pub struct ImathasQuestionBackendConfig {
    profile: ImathasGradingProfile,
    authentication_codec: ImathasSessionAuthenticationCodec,
    launch_signing_secret: Vec<u8>,
    result_verifier: ImathasResultVerifier,
    launch_ttl_millis: u64,
    max_snapshot_bytes: usize,
    max_result_bytes: usize,
}

impl std::fmt::Debug for ImathasQuestionBackendConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasQuestionBackendConfig")
            .field("deployment_reference", &self.profile.deployment_reference())
            .field("profile", &IMATHAS_GRADING_PROFILE_ID)
            .field("authentication_codec", &self.authentication_codec)
            .field("launch_signing_secret", &"REDACTED")
            .field("result_verifier", &self.result_verifier)
            .field("launch_ttl_millis", &self.launch_ttl_millis)
            .field("max_snapshot_bytes", &self.max_snapshot_bytes)
            .field("max_result_bytes", &self.max_result_bytes)
            .finish()
    }
}

impl ImathasQuestionBackendConfig {
    /// Creates a bounded configuration only for the explicit iMathAS grading profile.
    pub fn new(
        profile: ImathasGradingProfile,
        launch_signing_secret: impl AsRef<[u8]>,
        result_verification_secret: impl AsRef<[u8]>,
        authentication_codec: ImathasSessionAuthenticationCodec,
        launch_ttl_millis: u64,
    ) -> Result<Self, ImathasGradingFailure> {
        let launch_signing_secret = launch_signing_secret.as_ref();
        if !profile.allows_grading()
            || launch_signing_secret.is_empty()
            || launch_ttl_millis == 0
            || launch_ttl_millis > MAX_LAUNCH_TTL_MILLIS
        {
            return Err(ImathasGradingFailure::UnsupportedProfile);
        }
        Ok(Self {
            result_verifier: ImathasResultVerifier::new(
                profile.clone(),
                result_verification_secret,
            )?,
            profile,
            authentication_codec,
            launch_signing_secret: launch_signing_secret.to_vec(),
            launch_ttl_millis,
            max_snapshot_bytes: MAX_SNAPSHOT_BYTES,
            max_result_bytes: MAX_RESULT_BYTES,
        })
    }

    /// Tightens protected server-side body limits for an iMathAS deployment.
    pub fn with_limits(
        mut self,
        max_snapshot_bytes: usize,
        max_result_bytes: usize,
    ) -> Result<Self, ImathasGradingFailure> {
        if max_snapshot_bytes == 0
            || max_snapshot_bytes > MAX_SNAPSHOT_BYTES
            || max_result_bytes == 0
            || max_result_bytes > MAX_RESULT_BYTES
        {
            return Err(ImathasGradingFailure::InvalidLimits);
        }
        self.max_snapshot_bytes = max_snapshot_bytes;
        self.max_result_bytes = max_result_bytes;
        Ok(self)
    }

    fn supported_profile(&self) -> SupportedImathasProfile {
        // Construction has already frozen these compatibility claims.
        SupportedImathasProfile::new(
            question_model::ImathasProfile::new(IMATHAS_GRADING_PROFILE_ID)
                .expect("iMathAS grading profile identifier is valid"),
            true,
            true,
            true,
        )
        .expect("iMathAS grading profile is valid")
    }
}

/// Bounded failure classification for adapter-owned transport implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImathasTransportFailure {
    Unavailable,
    Timeout,
    InvalidResponse,
    SourceChanged,
    Unsupported,
}

impl From<ImathasTransportFailure> for ImathasQuestionBackendFailure {
    fn from(value: ImathasTransportFailure) -> Self {
        match value {
            ImathasTransportFailure::Unavailable => Self::Unavailable,
            ImathasTransportFailure::Timeout => Self::Timeout,
            ImathasTransportFailure::InvalidResponse | ImathasTransportFailure::SourceChanged => {
                Self::InvalidResponse
            }
            ImathasTransportFailure::Unsupported => Self::UnsupportedProfile,
        }
    }
}

/// Exact bytes from an authorized iMathAS Question Backend snapshot endpoint.
/// The bytes are answer-bearing and never implement Debug or serde.
pub struct ImathasQuestionBackendSnapshot(Vec<u8>);

impl ImathasQuestionBackendSnapshot {
    /// Adapter-owned transport implementations create a bounded snapshot result.
    pub fn from_protected_bytes(bytes: Vec<u8>) -> Result<Self, ImathasTransportFailure> {
        if bytes.is_empty() || bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err(ImathasTransportFailure::InvalidResponse);
        }
        Ok(Self(bytes))
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for ImathasQuestionBackendSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasQuestionBackendSnapshot")
            .field("bytes", &"REDACTED")
            .field("len", &self.0.len())
            .finish()
    }
}

/// Opaque iMathAS-proxy handle. It must be a server-held identifier, never a
/// URL, browser capability, JWT, credential, or iMathAS session cookie.
pub struct ImathasLaunchReference(String);

impl ImathasLaunchReference {
    /// Wraps an adapter/private-proxy generated opaque handle.
    pub fn from_server_handle(value: impl Into<String>) -> Result<Self, ImathasTransportFailure> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(ImathasTransportFailure::InvalidResponse);
        }
        Ok(Self(value))
    }

    #[allow(dead_code)]
    pub(crate) fn protected_value(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ImathasLaunchReference {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasLaunchReference(REDACTED)")
    }
}

/// Server-only request sent to a transport's authorized snapshot operation.
pub struct SnapshotTransportRequest<'a> {
    pub(crate) locator: &'a ImathasQuestionLocation,
    pub(crate) deployment_reference: &'a str,
}
impl<'a> SnapshotTransportRequest<'a> {
    pub fn deployment_reference(&self) -> &'a str {
        self.deployment_reference
    }
    pub fn item_reference(&self) -> &'a str {
        self.locator.item_reference().as_str()
    }
}

/// Server-only immutable safe-render request. The snapshot remains private.
pub struct RenderTransportRequest<'a> {
    pub(crate) snapshot: &'a [u8],
    pub(crate) deployment_reference: &'a str,
    pub(crate) question_revision: question_model::QuestionRevisionReference,
    pub(crate) seed: QuestionSeed,
}
impl<'a> RenderTransportRequest<'a> {
    pub fn snapshot(&self) -> &'a [u8] {
        self.snapshot
    }
    pub fn deployment_reference(&self) -> &'a str {
        self.deployment_reference
    }
    pub fn question_revision(&self) -> &question_model::QuestionRevisionReference {
        &self.question_revision
    }
    pub fn seed(&self) -> QuestionSeed {
        self.seed
    }
}

/// Protected request containing a signed launch JWT. It intentionally has no
/// public token getter: only an adapter-owned private transport can forward it
/// through the constrained server proxy.
pub struct ProtectedLaunchRequest {
    pub(crate) deployment_reference: String,
    pub(crate) item_reference: String,
    pub(crate) imathas_seed: u16,
    pub(crate) source_object_checksum: String,
    pub(crate) signed_launch_jwt: String,
}
impl std::fmt::Debug for ProtectedLaunchRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProtectedLaunchRequest")
            .field("deployment_reference", &self.deployment_reference)
            .field("item_reference", &self.item_reference)
            .field("imathas_seed", &self.imathas_seed)
            .field("source_object_checksum", &"REDACTED")
            .field("signed_launch_jwt", &"REDACTED")
            .finish()
    }
}
impl ProtectedLaunchRequest {
    pub fn deployment_reference(&self) -> &str {
        &self.deployment_reference
    }
    pub fn item_reference(&self) -> &str {
        &self.item_reference
    }
    pub fn imathas_seed(&self) -> u16 {
        self.imathas_seed
    }
    #[allow(dead_code)]
    pub(crate) fn source_object_checksum(&self) -> &str {
        &self.source_object_checksum
    }
    #[allow(dead_code)]
    pub(crate) fn signed_launch_jwt(&self) -> &str {
        &self.signed_launch_jwt
    }
}

/// Server-only request for an already-created iMathAS proxy session.
pub struct ResultTransportRequest<'a> {
    pub(crate) handle: &'a ImathasLaunchReference,
    pub(crate) launch_session_authentication: &'a str,
    pub(crate) deployment_reference: &'a str,
}

/// The only iMathAS-facing browser-proxy operation. There is deliberately no
/// URL, header map, redirect flag, or iMathAS cookie in this contract.
/// A transport maps this fixed activity resource onto its deployment-owned
/// upstream configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyMethod {
    Get,
    Post,
}

/// A private request bound to one restored opaque iMathAS handle. Server
/// routes cannot select an arbitrary upstream path or forward browser headers.
pub struct ProxyRequest<'a> {
    pub(crate) handle: &'a ImathasLaunchReference,
    pub(crate) method: ProxyMethod,
    pub(crate) body: &'a [u8],
}
impl<'a> ProxyRequest<'a> {
    fn activity_get(handle: &'a ImathasLaunchReference) -> Self {
        Self {
            handle,
            method: ProxyMethod::Get,
            body: &[],
        }
    }
    fn activity_post(
        handle: &'a ImathasLaunchReference,
        body: &'a [u8],
    ) -> Result<Self, ImathasTransportFailure> {
        if body.is_empty() || body.len() > MAX_PROXY_BODY_BYTES {
            return Err(ImathasTransportFailure::InvalidResponse);
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
    pub(crate) fn handle(&self) -> &ImathasLaunchReference {
        self.handle
    }
}

/// A bounded iMathAS document. The server maps this closed type to a fixed
/// response-header allowlist; transport implementations cannot return a
/// redirect, `Set-Cookie`, location, or arbitrary headers.
pub struct ProxyResponse {
    html: Vec<u8>,
}
impl ProxyResponse {
    pub fn protected_html(html: Vec<u8>) -> Result<Self, ImathasTransportFailure> {
        if html.is_empty() || html.len() > MAX_PROXY_BODY_BYTES {
            return Err(ImathasTransportFailure::InvalidResponse);
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
    pub fn deployment_reference(&self) -> &'a str {
        self.deployment_reference
    }
    #[allow(dead_code)]
    pub(crate) fn handle(&self) -> &ImathasLaunchReference {
        self.handle
    }
    #[allow(dead_code)]
    pub(crate) fn launch_session_authentication(&self) -> &str {
        self.launch_session_authentication
    }
}

/// Adapter-owned transport seam. Implementations must be server-only,
/// allowlisted, redirect-free, bounded, and never expose launch JWTs or result
/// tokens to browser/API DTOs. It deliberately has no generic URL method.
#[async_trait]
pub trait ImathasQuestionBackendTransport: Send + Sync {
    async fn fetch_snapshot(
        &self,
        request: SnapshotTransportRequest<'_>,
    ) -> Result<ImathasQuestionBackendSnapshot, ImathasTransportFailure>;
    async fn render_safe(
        &self,
        request: RenderTransportRequest<'_>,
    ) -> Result<SafeImathasQuestionRender, ImathasTransportFailure>;
    async fn start_protected_launch(
        &self,
        request: ProtectedLaunchRequest,
    ) -> Result<ImathasLaunchReference, ImathasTransportFailure>;
    /// Retrieves a signed grade using a safe, idempotent HTTP GET only.
    /// Implementations must not dispatch an iMathAS mutation from this method.
    async fn fetch_signed_grade_get(
        &self,
        request: ResultTransportRequest<'_>,
    ) -> Result<Vec<u8>, ImathasTransportFailure>;
    /// Fetches the fixed iMathAS activity resource through a deployment-owned
    /// proxy. Browser headers, URLs, cookies, and redirects never cross this
    /// seam.
    async fn proxy_activity(
        &self,
        request: ProxyRequest<'_>,
    ) -> Result<ProxyResponse, ImathasTransportFailure>;
}

/// Adapter Question Backend implementation that is ready for server composition but
/// requires an explicit protected iMathAS Question Backend Session before a result can grade.
pub struct ImathasQuestionBackend<T> {
    config: ImathasQuestionBackendConfig,
    transport: T,
}
impl<T> std::fmt::Debug for ImathasQuestionBackend<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasQuestionBackend")
            .field("config", &self.config)
            .field("transport", &"REDACTED")
            .finish()
    }
}
impl<T: ImathasQuestionBackendTransport> ImathasQuestionBackend<T> {
    pub fn new(config: ImathasQuestionBackendConfig, transport: T) -> Self {
        Self { config, transport }
    }

    /// Maximum server-held lifetime for a launch.  This exposes only a bounded
    /// duration, never deployment credentials or an iMathAS endpoint.
    pub fn launch_lifetime_millis(&self) -> u32 {
        self.config.launch_ttl_millis as u32
    }
    pub fn deployment_reference(&self) -> &str {
        self.config.profile.deployment_reference()
    }

    /// Starts the iMathAS launch and returns only LDA-ready iMathAS bytes.
    pub async fn prepare_imathas_question_backend_launch(
        &self,
        question: &QuestionRevision,
        source: &ResolvedImathasQuestionSource,
        validation: &learning_data_access::ImathasQuestionBackendLaunchPreparationValidation,
        now: Timestamp,
    ) -> Result<ImathasLaunchPreparation, ImathasAdapterError> {
        verify_binding(question, source)?;
        if source.binding().deployment_reference().as_str()
            != self.config.profile.deployment_reference()
        {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        if question.question_backend != QuestionBackend::Imathas {
            return Err(ImathasAdapterError::UnsupportedSource);
        }
        let binding = question
            .imathas_question_backend_binding
            .as_ref()
            .ok_or(ImathasAdapterError::UnsupportedSource)?;
        if binding.profile().as_str() != IMATHAS_GRADING_PROFILE_ID {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        let grading_context = &validation.grading_context;
        let question_revision = question_model::QuestionRevisionReference {
            question_id: question.question_id.clone(),
            revision_number: question.revision_number,
        };
        if validation
            .imathas_question_backend_binding
            .deployment_reference()
            .as_str()
            != source.binding().deployment_reference().as_str()
            || validation
                .imathas_question_backend_binding
                .item_reference()
                .as_str()
                != source.binding().item_reference().as_str()
            || validation
                .imathas_question_backend_binding
                .profile()
                .as_str()
                != source.binding().profile().as_str()
            || validation.source_object != *source.artifact()
            || validation.source_object_checksum != *source.source_object_checksum()
            || grading_context.question_revision() != &question_revision
            || validation.expires_at <= now
            || validation
                .challenge
                .as_bytes()
                .iter()
                .all(|byte| *byte == 0)
            || !self.config.authentication_codec.verifies_for_lda(
                grading_context,
                &validation.challenge,
                &validation.authentication,
            )
        {
            return Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication);
        }
        let imathas_seed = normalize_imathas_seed(grading_context.question_seed());
        let binding_digest = launch_binding_digest(
            grading_context,
            source.binding().item_reference().as_str(),
            source.source_object_checksum().as_str(),
            imathas_seed,
            validation.authentication.as_str(),
        );
        let qualified_launch_binding_digest =
            learning_data_access::QualifiedLaunchBindingDigest::parse(binding_digest.clone())
                .map_err(|_| {
                    ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication
                })?;
        let draft_binding = DraftImathasQuestionBackendBinding::new(
            source.binding().deployment_reference().clone(),
            source.binding().item_reference().clone(),
        );
        let locator = ImathasQuestionLocation::from_draft_imathas_question_backend_binding(
            &draft_binding,
        );
        let fresh = self
            .transport
            .fetch_snapshot(SnapshotTransportRequest {
                locator: &locator,
                deployment_reference: self.config.profile.deployment_reference(),
            })
            .await
            .map_err(map_transport)?;
        if fresh.bytes().len() > self.config.max_snapshot_bytes
            || hex(Sha256::digest(fresh.bytes()).as_slice())
                != source.source_object_checksum().as_str()
        {
            return Err(ImathasAdapterError::SourceChecksumMismatch);
        }
        let protected = ProtectedLaunchRequest {
            deployment_reference: self.config.profile.deployment_reference().to_owned(),
            item_reference: source.binding().item_reference().as_str().to_owned(),
            imathas_seed,
            source_object_checksum: source.source_object_checksum().to_string(),
            signed_launch_jwt: protocol::signed_launch_jwt(
                &self.config.launch_signing_secret,
                source.binding().item_reference().as_str(),
                imathas_seed,
                validation.expires_at.as_unix_millis(),
                &base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(validation.challenge.as_bytes()),
                &binding_digest,
            )?,
        };
        let handle = self
            .transport
            .start_protected_launch(protected)
            .await
            .map_err(map_transport)?;
        Ok(ImathasLaunchPreparation {
            imathas_launch_state: ImathasLaunchState::from_launch_handle(handle).encode()?,
            qualified_launch_binding_digest,
        })
    }

    /// Retrieves the iMathAS response through the protected transport and
    /// passes it to the sealed iMathAS Question Backend verifier. No browser callback can
    /// reach this method's token input.
    pub async fn retrieve_and_verify(
        &self,
        validation: &learning_data_access::ImathasQuestionBackendSessionValidation,
        imathas_launch_state: &ImathasLaunchState,
        now: Timestamp,
    ) -> Result<VerifiedImathasResult, ImathasAdapterError> {
        result::retrieve_and_verify(
            &self.transport,
            &self.config,
            validation,
            imathas_launch_state,
            now,
        )
        .await
    }

    /// Proxies the one fixed activity resource for a restored session.  This
    /// preserves the session's iMathAS handle inside the adapter and keeps
    /// route code from selecting URLs or headers.
    pub async fn proxy_activity(
        &self,
        validation: &learning_data_access::ImathasQuestionBackendSessionValidation,
        imathas_launch_state: &ImathasLaunchState,
        method: ProxyMethod,
        body: &[u8],
        now: Timestamp,
    ) -> Result<ProxyResponse, ImathasAdapterError> {
        validate_loaded_imathas_launch_state(&self.config, validation, now)?;
        let request = match method {
            ProxyMethod::Get if body.is_empty() => {
                ProxyRequest::activity_get(imathas_launch_state.handle())
            }
            ProxyMethod::Post => ProxyRequest::activity_post(imathas_launch_state.handle(), body)
                .map_err(map_transport)?,
            _ => {
                return Err(ImathasAdapterError::QuestionBackend(
                    ImathasQuestionBackendFailure::InvalidResponse,
                ));
            }
        };
        self.transport
            .proxy_activity(request)
            .await
            .map_err(map_transport)
    }
}

impl<T: ImathasQuestionBackendTransport> sealed::QuestionBackendSealed
    for ImathasQuestionBackend<T>
{
}

#[async_trait]
impl<T: ImathasQuestionBackendTransport> QuestionBackend for ImathasQuestionBackend<T> {
    async fn snapshot(
        &self,
        locator: &ImathasQuestionLocation,
    ) -> Result<(Vec<u8>, SupportedImathasProfile), ImathasQuestionBackendFailure> {
        if locator.deployment_reference().as_str() != self.config.profile.deployment_reference() {
            return Err(ImathasQuestionBackendFailure::UnsupportedProfile);
        }
        let snapshot = self
            .transport
            .fetch_snapshot(SnapshotTransportRequest {
                locator,
                deployment_reference: self.config.profile.deployment_reference(),
            })
            .await
            .map_err(ImathasQuestionBackendFailure::from)?;
        if snapshot.bytes().len() > self.config.max_snapshot_bytes {
            return Err(ImathasQuestionBackendFailure::InvalidResponse);
        }
        Ok((snapshot.0, self.config.supported_profile()))
    }

    async fn render(
        &self,
        request: ImathasRenderRequest<'_>,
    ) -> Result<SafeImathasQuestionRender, ImathasQuestionBackendFailure> {
        if request.profile != IMATHAS_GRADING_PROFILE_ID
            || request.snapshot.len() > self.config.max_snapshot_bytes
        {
            return Err(ImathasQuestionBackendFailure::UnsupportedProfile);
        }
        self.transport
            .render_safe(RenderTransportRequest {
                snapshot: request.snapshot,
                deployment_reference: self.config.profile.deployment_reference(),
                question_revision: request.question_revision,
                seed: request.seed,
            })
            .await
            .map_err(ImathasQuestionBackendFailure::from)
    }

    async fn verify_result(
        &self,
        _request: ImathasResultRequest<'_>,
    ) -> Result<VerifiedImathasResult, ImathasQuestionBackendFailure> {
        // The generic adapter method has no protected iMathAS Question Backend
        // Session. Refuse
        // rather than accepting any caller-provided token or claiming a grade.
        Err(ImathasQuestionBackendFailure::UnsupportedProfile)
    }
}

/// Transient adapter output used by composition to create the sole LDA session.
pub struct ImathasLaunchPreparation {
    imathas_launch_state: learning_data_access::ImathasQuestionBackendStatePlaintext,
    qualified_launch_binding_digest: learning_data_access::QualifiedLaunchBindingDigest,
}

impl ImathasLaunchPreparation {
    pub fn imathas_launch_state(
        &self,
    ) -> &learning_data_access::ImathasQuestionBackendStatePlaintext {
        &self.imathas_launch_state
    }
    pub fn qualified_launch_binding_digest(
        &self,
    ) -> &learning_data_access::QualifiedLaunchBindingDigest {
        &self.qualified_launch_binding_digest
    }
}

impl std::fmt::Debug for ImathasLaunchPreparation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasLaunchPreparation(REDACTED)")
    }
}

fn validate_loaded_imathas_launch_state(
    config: &ImathasQuestionBackendConfig,
    validation: &learning_data_access::ImathasQuestionBackendSessionValidation,
    now: Timestamp,
) -> Result<(), ImathasAdapterError> {
    if validation
        .imathas_question_backend_binding
        .deployment_reference()
        .as_str()
        != config.profile.deployment_reference()
        || validation
            .imathas_question_backend_binding
            .profile()
            .as_str()
            != IMATHAS_GRADING_PROFILE_ID
        || validation.expires_at <= now
        || validation
            .imathas_question_backend_binding
            .item_reference()
            .as_str()
            .is_empty()
        || validation.source_object_checksum.as_str().len() != 64
        || validation
            .challenge
            .as_bytes()
            .iter()
            .all(|byte| *byte == 0)
    {
        return Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication);
    }
    let grading_context = &validation.grading_context;
    if !config.authentication_codec.verifies_for_lda(
        grading_context,
        &validation.challenge,
        &validation.authentication,
    ) {
        return Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication);
    }
    let expected = launch_binding_digest(
        grading_context,
        validation
            .imathas_question_backend_binding
            .item_reference()
            .as_str(),
        validation.source_object_checksum.as_str(),
        normalize_imathas_seed(grading_context.question_seed()),
        validation.authentication.as_str(),
    );
    if expected != validation.qualified_launch_binding_digest.as_str() {
        return Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication);
    }
    Ok(())
}

fn map_transport(error: ImathasTransportFailure) -> ImathasAdapterError {
    ImathasAdapterError::QuestionBackend(error.into())
}

#[cfg(test)]
#[path = "imathas_question_backend/tests.rs"]
mod tests;
