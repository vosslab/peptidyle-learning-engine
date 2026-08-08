//! MOD-ADP-IMATHAS: isolated iMathAS/MyOpenMath adapter.
//!
//! A provider is deployment configuration selected by an opaque key.  This
//! crate never accepts an author-supplied endpoint, never serializes launch or
//! answer material, and only accepts a grade that a server-side provider
//! verifier has authenticated and correlated to one exact attempt.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use async_trait::async_trait;
use objects::{ObjectCategory, ObjectKey, ObjectStore, ObjectStoreError, PutObject};
use question_model::capability::{BackendCapabilities, Capability};
use question_model::envelope::ContentBlock;
use question_model::generation::Seed;
use question_model::{
    ActivityTimestamp, AttemptProvenance, AttemptResult, ImplementationVersion, ObjectId,
    ProblemId, QuestionAttemptId, QuestionDefinition, QuestionEnvelope, QuestionSource,
    QuestionTitleError, SourceArtifact, TenantId, VersionId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use store::PublishedSourceArtifact;
use uuid::Uuid;

pub mod broker_provider;
#[cfg(feature = "http-transport")]
pub mod http_transport;
pub mod scored_embed;
#[cfg(feature = "test-support")]
pub mod test_support;

/// Stable adapter identity persisted in provenance.  This is a compatibility
/// identifier, deliberately independent of the repository CalVer release.
pub const ADAPTER_ID: &str = "imathas-adapter";
/// Current compatible adapter implementation.
pub const ADAPTER_VERSION: &str = "1";
/// Stable identity for server-verified provider grading.
pub const GRADING_ID: &str = "imathas-verified-grader";
/// Current compatible server verifier implementation.
pub const GRADING_VERSION: &str = "1";

/// A provider's publication-safe integration profile.
///
/// No endpoint, credential, accepted origin, or launch protocol is carried in
/// this value.  Those belong to deployment configuration behind `provider`.
#[derive(Clone, PartialEq, Eq)]
pub struct SupportedProfile {
    name: String,
    deterministic_seeded_render: bool,
    verified_server_grading: bool,
    partial_credit: bool,
}

impl std::fmt::Debug for SupportedProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SupportedProfile")
            .field("name", &self.name)
            .field(
                "deterministic_seeded_render",
                &self.deterministic_seeded_render,
            )
            .field("verified_server_grading", &self.verified_server_grading)
            .field("partial_credit", &self.partial_credit)
            .finish()
    }
}

impl SupportedProfile {
    /// Constructs an explicitly supported protocol profile.
    pub fn new(
        name: impl Into<String>,
        deterministic_seeded_render: bool,
        verified_server_grading: bool,
        partial_credit: bool,
    ) -> Result<Self, ImathasAdapterError> {
        let name = name.into();
        if name.is_empty() || name.len() > 128 {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        if partial_credit && !verified_server_grading {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        Ok(Self {
            name,
            deterministic_seeded_render,
            verified_server_grading,
            partial_credit,
        })
    }

    /// Pinned profile name persisted with a published source.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A private draft locator. It intentionally cannot name a published problem
/// or version and cannot carry an endpoint or credential.
#[derive(Clone, PartialEq, Eq)]
pub struct DraftLocator {
    provider: String,
    item_ref: String,
}

impl DraftLocator {
    /// Creates a private sandbox locator from the draft source only.
    pub fn from_draft(
        source: &question_model::DraftQuestionSource,
    ) -> Result<Self, ImathasAdapterError> {
        match source {
            question_model::DraftQuestionSource::Imathas { provider, item_ref }
                if valid_opaque_key(provider) && valid_item_ref(item_ref) =>
            {
                Ok(Self {
                    provider: provider.clone(),
                    item_ref: item_ref.clone(),
                })
            }
            question_model::DraftQuestionSource::Imathas { .. } => {
                Err(ImathasAdapterError::InvalidDraft)
            }
            _ => Err(ImathasAdapterError::UnsupportedSource),
        }
    }

    /// Opaque deployment configuration selector.
    pub fn provider(&self) -> &str {
        &self.provider
    }
    /// Provider-local question reference.
    pub fn item_ref(&self) -> &str {
        &self.item_ref
    }
}

impl std::fmt::Debug for DraftLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DraftLocator(REDACTED)")
    }
}

/// Server-private immutable bytes prepared before publication. It has no
/// `ProblemId`/`VersionId`; the publication transaction alone owns identity.
#[derive(Clone, PartialEq, Eq)]
pub struct PreparedSnapshot {
    bytes: Vec<u8>,
    sha256: String,
    profile: SupportedProfile,
}

impl PreparedSnapshot {
    /// Exact source bytes for the trusted worker/object-store handoff.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Pinned digest to put into the published source record.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    /// Validated integration profile.
    pub fn profile(&self) -> &SupportedProfile {
        &self.profile
    }
}

impl std::fmt::Debug for PreparedSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedSnapshot")
            .field("sha256", &self.sha256)
            .field("profile", &self.profile)
            .finish_non_exhaustive()
    }
}

/// Browser-safe provider rendering material. It cannot contain iframe markup,
/// launch URLs, tokens, callbacks, answers, or scores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeProviderRender {
    /// Plain prompt blocks, already constrained by the adapter boundary.
    pub prompt: Vec<ContentBlock>,
    /// A browser-safe label for the external tool.
    pub title: String,
}

/// Server-side provider client. Implementations keep provider URLs,
/// credentials, network timeout policy, and trust verification private.
/// Implementation seal: external crates cannot install a provider that
/// constructs a grade proof without the adapter-owned verifier.
mod sealed {
    pub trait ProviderSealed {}
}

#[async_trait]
pub trait ImathasProvider: sealed::ProviderSealed + Send + Sync {
    /// Fetches exact source bytes and an explicitly supported profile for an unversioned draft.
    async fn snapshot(
        &self,
        locator: &DraftLocator,
    ) -> Result<(Vec<u8>, SupportedProfile), ProviderFailure>;
    /// Produces only browser-safe prompt material from archived source bytes.
    async fn render(
        &self,
        request: ProviderRenderRequest<'_>,
    ) -> Result<SafeProviderRender, ProviderFailure>;
    /// Authenticates and correlates an upstream grade server-to-server.
    async fn verify_grade(
        &self,
        request: ProviderGradeRequest<'_>,
    ) -> Result<VerifiedProviderGrade, ProviderFailure>;
}

/// Immutable inputs for an external render. No browser data is present.
pub struct ProviderRenderRequest<'a> {
    /// Exact archived source bytes.
    pub snapshot: &'a [u8],
    /// Pinned source profile.
    pub profile: &'a str,
    /// Immutable version.
    pub version: VersionId,
    /// Deterministic variation seed.
    pub seed: Seed,
}

/// Server-held, attempt-bound provider grade request. Private fields prevent a
/// browser request from constructing a correlation or score payload.
pub struct ProviderGradeRequest<'a> {
    snapshot: &'a [u8],
    profile: &'a str,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    problem: ProblemId,
    version: VersionId,
    seed: Seed,
    correlation: &'a ServerCorrelation,
}

impl<'a> ProviderGradeRequest<'a> {
    pub fn snapshot(&self) -> &'a [u8] {
        self.snapshot
    }
    pub fn profile(&self) -> &'a str {
        self.profile
    }
    pub fn tenant(&self) -> TenantId {
        self.tenant
    }
    pub fn attempt(&self) -> QuestionAttemptId {
        self.attempt
    }
    pub fn problem(&self) -> ProblemId {
        self.problem
    }
    pub fn version(&self) -> VersionId {
        self.version
    }
    pub fn seed(&self) -> Seed {
        self.seed
    }
    /// Opaque server-held value transmitted only by the provider client.
    pub fn correlation(&self) -> &ServerCorrelation {
        self.correlation
    }
}

/// Opaque server broker correlation. It is deliberately neither serializable
/// nor constructible from browser data.  The server stores the opaque encoding
/// returned by [`CorrelationIssuer::begin`] and restores it only after MAC
/// validation.
#[derive(Clone, PartialEq, Eq)]
pub struct ServerCorrelation(String);

/// Adapter-owned server-only issuer for persisted grade correlation handles.
///
/// The secret comes from protected server configuration; its byte array is not
/// an HTTP shape and no browser input can construct an accepted restoration.
pub struct CorrelationIssuer {
    secret: [u8; 32],
}

impl CorrelationIssuer {
    /// Installs protected deployment configuration in the server composition.
    pub fn from_server_secret(secret: [u8; 32]) -> Self {
        Self { secret }
    }

    /// Begins a broker exchange after server-side authentication and attempt
    /// idempotency selection. The returned handle is safe to persist, not send
    /// to a browser or provider.
    pub fn begin(&self, binding: GradeBinding) -> PersistedCorrelation {
        let payload = binding_payload(&binding);
        let mac = self.mac(&payload);
        PersistedCorrelation(format!("{}.{}", hex(&payload), hex(&mac)))
    }

    /// Restores a previously persisted correlation, refusing altered, stale,
    /// wrong-binding, or non-canonical values before any provider call.
    pub fn restore(
        &self,
        binding: GradeBinding,
        persisted: &PersistedCorrelation,
    ) -> Result<ServerCorrelation, ImathasAdapterError> {
        let expected = self.begin(binding);
        if !constant_time_eq(expected.0.as_bytes(), persisted.0.as_bytes()) {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        Ok(ServerCorrelation(expected.0))
    }

    fn mac(&self, payload: &[u8]) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(b"ple:imathas:broker-correlation:v1");
        digest.update(self.secret);
        digest.update(payload);
        digest.finalize().into()
    }
}

/// Exact server-owned grade identity persisted alongside its idempotency row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradeBinding {
    pub tenant: TenantId,
    pub attempt: QuestionAttemptId,
    pub problem: ProblemId,
    pub version: VersionId,
    pub seed: Seed,
}

/// Opaque database-persistable correlation encoding. It has no serde impl and
/// its inner string is inaccessible to clients.
#[derive(Clone, PartialEq, Eq)]
pub struct PersistedCorrelation(String);

impl PersistedCorrelation {
    /// Returns the bounded opaque value the tenant-owned broker row may store.
    /// This is intentionally not serde: callers must opt into this protected
    /// storage boundary rather than accidentally placing it in an HTTP DTO.
    pub fn to_storage_value(&self) -> String {
        self.0.clone()
    }

    /// Rehydrates an opaque value read from protected storage. This performs
    /// only canonical bounded syntax validation; callers must still call
    /// [`CorrelationIssuer::restore`] to validate the issuer MAC and exact
    /// attempt binding before a provider request.
    pub fn from_storage_value(value: &str) -> Result<Self, ImathasAdapterError> {
        const PAYLOAD_HEX_LEN: usize = (16 * 4 + 8) * 2;
        const MAC_HEX_LEN: usize = 32 * 2;
        const ENCODED_LEN: usize = PAYLOAD_HEX_LEN + 1 + MAC_HEX_LEN;
        if value.len() != ENCODED_LEN {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        let Some((payload, mac)) = value.split_once('.') else {
            return Err(ImathasAdapterError::InvalidCorrelation);
        };
        if payload.len() != PAYLOAD_HEX_LEN
            || mac.len() != MAC_HEX_LEN
            || !payload
                .bytes()
                .chain(mac.bytes())
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        Ok(Self(value.to_owned()))
    }
}
impl std::fmt::Debug for PersistedCorrelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PersistedCorrelation(REDACTED)")
    }
}

impl std::fmt::Debug for ServerCorrelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ServerCorrelation(REDACTED)")
    }
}

/// Authenticated provider verdict. The fields are private and this type has no
/// serde implementation, so an HTTP/browser payload cannot deserialize into it.
#[derive(Clone, PartialEq)]
pub struct VerifiedProviderGrade {
    result: AttemptResult,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    problem: ProblemId,
    version: VersionId,
    seed: Seed,
    correlation: String,
}

impl std::fmt::Debug for VerifiedProviderGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedProviderGrade")
            .field("result", &self.result)
            .field("tenant", &self.tenant)
            .field("attempt", &self.attempt)
            .field("problem", &self.problem)
            .field("version", &self.version)
            .field("seed", &self.seed)
            .field("correlation", &"REDACTED")
            .finish()
    }
}

impl VerifiedProviderGrade {
    /// Server-only verified result; this type is non-serde and can only be
    /// obtained from the sealed contracted verifier.
    pub fn result(&self) -> AttemptResult {
        self.result
    }
    /// Exact identity authenticated by the provider verifier.
    pub fn binding(&self) -> GradeBinding {
        GradeBinding {
            tenant: self.tenant,
            attempt: self.attempt,
            problem: self.problem,
            version: self.version,
            seed: self.seed,
        }
    }
    /// Provider implementations use this only after their signature/audience/
    /// expiry/nonce verification succeeds.
    #[cfg(test)]
    fn verified(
        result: AttemptResult,
        tenant: TenantId,
        attempt: QuestionAttemptId,
        problem: ProblemId,
        version: VersionId,
        seed: Seed,
        correlation: &ServerCorrelation,
    ) -> Self {
        Self {
            result,
            tenant,
            attempt,
            problem,
            version,
            seed,
            correlation: correlation.0.clone(),
        }
    }

    /// The scored-embed verifier is the only production constructor.  Its
    /// result token has already passed signature, expiry, exact question, and
    /// single-use server-ledger checks before this sealed grade exists.
    pub(crate) fn from_scored_embed(
        result: AttemptResult,
        binding: GradeBinding,
        correlation: &ServerCorrelation,
    ) -> Self {
        Self {
            result,
            tenant: binding.tenant,
            attempt: binding.attempt,
            problem: binding.problem,
            version: binding.version,
            seed: binding.seed,
            correlation: correlation.0.clone(),
        }
    }
}

/// Provider-local failures. They are deliberately classified as unavailable or
/// invalid rather than a learner correctness decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderFailure {
    Unavailable,
    Timeout,
    UnsupportedProfile,
    Authentication,
    Correlation,
    InvalidResponse,
}

/// Adapter failures suitable for a backend-local retry/degraded state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImathasAdapterError {
    UnsupportedSource,
    InvalidDraft,
    UnsupportedProfile,
    SourceChecksumMismatch,
    UntrustedSource,
    SourceDoesNotMatchQuestion,
    InvalidCache,
    InvalidProviderRender,
    InvalidTitle(QuestionTitleError),
    InvalidCorrelation,
    VerificationRefused,
    Provider(ProviderFailure),
    ObjectStore(ObjectStoreError),
}

impl std::fmt::Display for ImathasAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSource => f.write_str("question source is not iMathAS"),
            Self::InvalidDraft => f.write_str("invalid private iMathAS draft locator"),
            Self::UnsupportedProfile => f.write_str("unsupported iMathAS integration profile"),
            Self::SourceChecksumMismatch => f.write_str("iMathAS snapshot checksum mismatch"),
            Self::UntrustedSource => {
                f.write_str("iMathAS source was not resolved through its immutable object")
            }
            Self::SourceDoesNotMatchQuestion => {
                f.write_str("iMathAS source does not match its published question")
            }
            Self::InvalidCache => f.write_str("invalid iMathAS render cache"),
            Self::InvalidProviderRender => f.write_str("invalid iMathAS provider render"),
            Self::InvalidTitle(error) => write!(f, "invalid iMathAS question title: {error}"),
            Self::InvalidCorrelation => f.write_str("invalid server-held iMathAS correlation"),
            Self::VerificationRefused => {
                f.write_str("iMathAS verified grade did not match its server-held binding")
            }
            Self::Provider(_) => f.write_str("iMathAS provider unavailable or rejected request"),
            Self::ObjectStore(v) => v.fmt(f),
        }
    }
}
impl std::error::Error for ImathasAdapterError {}

/// Exact immutable source loaded through trusted storage.
#[derive(Clone)]
pub struct ImathasSource {
    problem: ProblemId,
    version: VersionId,
    artifact: SourceArtifact,
    provider: String,
    item_ref: String,
    profile: String,
    bytes: Vec<u8>,
}
impl std::fmt::Debug for ImathasSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImathasSource")
            .field("problem", &self.problem)
            .field("version", &self.version)
            .field("artifact", &self.artifact)
            .field("provider", &self.provider)
            .field("item_ref", &self.item_ref)
            .field("profile", &self.profile)
            .finish_non_exhaustive()
    }
}

impl ImathasSource {
    /// Resolves a published iMathAS source only from its exact object key.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question: &QuestionDefinition,
        published_artifact: &PublishedSourceArtifact,
    ) -> Result<Self, ImathasAdapterError> {
        let QuestionSource::Imathas {
            provider,
            item_ref,
            snapshot,
            snapshot_sha256,
            integration_profile,
        } = &question.source
        else {
            return Err(ImathasAdapterError::UnsupportedSource);
        };
        if !valid_opaque_key(provider)
            || !valid_item_ref(item_ref)
            || !valid_opaque_key(integration_profile)
        {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        if published_artifact.reference.problem != question.problem
            || published_artifact.reference.version != question.version
            || published_artifact.backend != question_model::QuestionBackend::Imathas
            || published_artifact.object.id != *snapshot
            || published_artifact.object.key
                != (ObjectKey::ProblemSource {
                    problem: question.problem,
                    version: question.version,
                    object: *snapshot,
                })
            || published_artifact.object.category != ObjectCategory::Source
            || published_artifact.object.version != Some(question.version)
            || published_artifact.object.sha256.to_string() != *snapshot_sha256
        {
            return Err(ImathasAdapterError::UntrustedSource);
        }
        let artifact = SourceArtifact {
            object: *snapshot,
            sha256: snapshot_sha256.clone(),
        };
        let key = ObjectKey::ProblemSource {
            problem: question.problem,
            version: question.version,
            object: *snapshot,
        };
        let stored = store
            .get(&key)
            .await
            .map_err(ImathasAdapterError::ObjectStore)?;
        if stored.record != published_artifact.object
            || stored.record.id != *snapshot
            || stored.record.key != key
            || stored.record.category != ObjectCategory::Source
            || stored.record.version != Some(question.version)
            || stored.record.sha256.to_string() != *snapshot_sha256
        {
            return Err(ImathasAdapterError::UntrustedSource);
        }
        let digest = hex(Sha256::digest(&stored.bytes).as_slice());
        if digest != *snapshot_sha256 {
            return Err(ImathasAdapterError::SourceChecksumMismatch);
        }
        Ok(Self {
            problem: question.problem,
            version: question.version,
            artifact,
            provider: provider.clone(),
            item_ref: item_ref.clone(),
            profile: integration_profile.clone(),
            bytes: stored.bytes,
        })
    }
    pub fn artifact(&self) -> &SourceArtifact {
        &self.artifact
    }
}

/// Key-free issued external-tool question.
#[derive(Debug, Clone, PartialEq)]
pub struct ImathasIssuedAttempt {
    pub envelope: QuestionEnvelope,
    pub parameter_hash: String,
    pub provenance: AttemptProvenance,
    pub cache_hit: bool,
}

/// Server-only verified grade receipt. The attempt store persists the first
/// receipt under its own idempotency key and returns it on replay; this adapter
/// intentionally performs no process-local grade caching.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedGradeReceipt {
    result: AttemptResult,
    binding: GradeBinding,
}

impl VerifiedGradeReceipt {
    /// The result accepted from the authenticated provider verifier.
    pub fn result(&self) -> AttemptResult {
        self.result
    }
    /// Exact identity the API/store must use when persisting the first receipt.
    pub fn binding(&self) -> GradeBinding {
        self.binding
    }
}

/// iMathAS adapter with immutable source and deterministic, browser-safe cache.
pub struct ImathasAdapter<S, P> {
    store: S,
    provider: P,
    profiles: BTreeSet<String>,
}
impl<S: ObjectStore, P: ImathasProvider> ImathasAdapter<S, P> {
    pub fn new(
        store: S,
        provider: P,
        profiles: impl IntoIterator<Item = SupportedProfile>,
    ) -> Self {
        Self {
            store,
            provider,
            profiles: profiles.into_iter().map(|p| p.name).collect(),
        }
    }
    /// Snapshot a draft before publication. This neither knows nor mints a published identity.
    pub async fn prepare_snapshot(
        &self,
        draft: &question_model::DraftQuestionSource,
    ) -> Result<PreparedSnapshot, ImathasAdapterError> {
        let locator = DraftLocator::from_draft(draft)?;
        let (bytes, profile) = self
            .provider
            .snapshot(&locator)
            .await
            .map_err(ImathasAdapterError::Provider)?;
        if bytes.is_empty() || !self.profiles.contains(profile.name()) {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        Ok(PreparedSnapshot {
            sha256: hex(Sha256::digest(&bytes).as_slice()),
            bytes,
            profile,
        })
    }
    /// Derives only capabilities actually delivered by the pinned profile.
    pub fn capabilities(
        &self,
        source: &QuestionSource,
        profile: &SupportedProfile,
    ) -> Result<BackendCapabilities, ImathasAdapterError> {
        let QuestionSource::Imathas {
            integration_profile,
            ..
        } = source
        else {
            return Err(ImathasAdapterError::UnsupportedSource);
        };
        if integration_profile != profile.name() || !self.profiles.contains(profile.name()) {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        let mut values = vec![Capability::PerQuestionTiming];
        if profile.deterministic_seeded_render {
            values.push(Capability::AlgorithmicGeneration);
        }
        if profile.verified_server_grading {
            values.push(Capability::ServerGrading);
        }
        if profile.partial_credit {
            values.push(Capability::PartialCredit);
        }
        Ok(BackendCapabilities::from_iter(values))
    }
    /// Issues an ExternalTool marker and safe provider prompt; repeated exact version/seed is cache-only.
    pub async fn issue(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        source: &ImathasSource,
        created_at: ActivityTimestamp,
    ) -> Result<ImathasIssuedAttempt, ImathasAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(ImathasAdapterError::InvalidTitle)?;
        verify_binding(question, source)?;
        if !self.profiles.contains(&source.profile) {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        let key = render_key(question.problem, question.version, seed);
        match self.store.get(&key).await {
            Ok(stored) => {
                let value = decode_cache(&stored.bytes)?;
                validate_cache(&value, question, seed, source)?;
                return self.issued(value, source, true);
            }
            Err(ObjectStoreError::NotFound) => {}
            Err(error) => return Err(ImathasAdapterError::ObjectStore(error)),
        };
        let safe = self
            .provider
            .render(ProviderRenderRequest {
                snapshot: &source.bytes,
                profile: &source.profile,
                version: question.version,
                seed,
            })
            .await
            .map_err(ImathasAdapterError::Provider)?;
        if question_model::validate_question_title(&safe.title).is_err() {
            return Err(ImathasAdapterError::InvalidProviderRender);
        }
        let record = CachedRender {
            schema: 1,
            source: source.artifact.clone(),
            provider: source.provider.clone(),
            profile: source.profile.clone(),
            envelope: QuestionEnvelope {
                version: question.version,
                seed,
                title: safe.title,
                prompt: safe.prompt,
                response: question_model::ResponseDefinition::ExternalTool {},
            },
        };
        validate_cache(&record, question, seed, source)?;
        let bytes = serde_json::to_vec(&record).map_err(|_| ImathasAdapterError::InvalidCache)?;
        match self
            .store
            .put(PutObject {
                key: key.clone(),
                bytes,
                media_type: "application/vnd.peptidyle.imathas-render+json".into(),
                license: "derived-render".into(),
                provenance: "safe iMathAS external-tool render cache".into(),
                created_at,
            })
            .await
        {
            Ok(_) => self.issued(record, source, false),
            // Concurrent stateless replicas may render the same immutable
            // key. The winning immutable record is authoritative, so reload
            // and fully validate it rather than turning normal scale-out into
            // a learner-visible failure.
            Err(ObjectStoreError::AlreadyExists) => {
                let stored = self
                    .store
                    .get(&key)
                    .await
                    .map_err(ImathasAdapterError::ObjectStore)?;
                let cached = decode_cache(&stored.bytes)?;
                validate_cache(&cached, question, seed, source)?;
                self.issued(cached, source, true)
            }
            Err(error) => Err(ImathasAdapterError::ObjectStore(error)),
        }
    }
    fn issued(
        &self,
        cached: CachedRender,
        source: &ImathasSource,
        cache_hit: bool,
    ) -> Result<ImathasIssuedAttempt, ImathasAdapterError> {
        let hash = hex(Sha256::digest(
            serde_json::to_vec(&cached).map_err(|_| ImathasAdapterError::InvalidCache)?,
        )
        .as_slice());
        Ok(ImathasIssuedAttempt {
            parameter_hash: parameter_hash(cached.envelope.seed),
            provenance: AttemptProvenance {
                adapter: implementation(ADAPTER_ID, ADAPTER_VERSION),
                renderer: Some(implementation("imathas-profile", &source.profile)),
                generator: None,
                source_artifact: Some(source.artifact.clone()),
                asset_objects: Vec::new(),
                grading: implementation(GRADING_ID, GRADING_VERSION),
                rendered_question_sha256: hash,
            },
            envelope: cached.envelope,
            cache_hit,
        })
    }
    /// Accepts only a provider-verifier result that matches every server-held binding.
    pub async fn grade(
        &self,
        question: &QuestionDefinition,
        source: &ImathasSource,
        tenant: TenantId,
        attempt: QuestionAttemptId,
        seed: Seed,
        correlation: &ServerCorrelation,
    ) -> Result<VerifiedGradeReceipt, ImathasAdapterError> {
        verify_binding(question, source)?;
        let verdict = self
            .provider
            .verify_grade(ProviderGradeRequest {
                snapshot: &source.bytes,
                profile: &source.profile,
                tenant,
                attempt,
                problem: question.problem,
                version: question.version,
                seed,
                correlation,
            })
            .await
            .map_err(ImathasAdapterError::Provider)?;
        if verdict.tenant != tenant
            || verdict.attempt != attempt
            || verdict.problem != question.problem
            || verdict.version != question.version
            || verdict.seed != seed
            || verdict.correlation != correlation.0
        {
            return Err(ImathasAdapterError::VerificationRefused);
        }
        Ok(VerifiedGradeReceipt {
            result: verdict.result,
            binding: GradeBinding {
                tenant,
                attempt,
                problem: question.problem,
                version: question.version,
                seed,
            },
        })
    }
}

// The protected scored-embed path is deliberately an opt-in extension of the
// ordinary adapter, rather than a method on `ImathasProvider`.  Recorded and
// future render-only providers must not accidentally acquire a launch or
// result-verification capability merely by implementing the generic provider
// trait.
impl<S, T> ImathasAdapter<S, broker_provider::ContractedScoredEmbedProvider<T>>
where
    S: ObjectStore,
    T: broker_provider::ScoredEmbedTransport,
{
    pub fn contracted_provider_key(&self) -> &str {
        self.provider.provider_key()
    }
    pub fn contracted_launch_lifetime_millis(&self) -> u32 {
        self.provider.launch_lifetime_millis()
    }
    /// Starts one server-held contracted launch.  The returned value is
    /// intentionally non-serde and can only cross into protected server
    /// storage through `LaunchSessionCodec`.
    #[allow(clippy::too_many_arguments)]
    pub async fn begin_contracted_launch(
        &self,
        question: &QuestionDefinition,
        source: &ImathasSource,
        tenant: TenantId,
        attempt: QuestionAttemptId,
        seed: Seed,
        correlation: ServerCorrelation,
        nonce: scored_embed::ScoredEmbedNonce,
        now: ActivityTimestamp,
    ) -> Result<broker_provider::ContractedLaunchSession, ImathasAdapterError> {
        self.provider
            .begin_launch(
                question,
                source,
                tenant,
                attempt,
                seed,
                correlation,
                nonce,
                now,
            )
            .await
    }

    /// Retrieves a result only from a restored server-held launch session.
    /// The contracted provider checks expiry and single-use before contacting
    /// its transport.
    pub async fn retrieve_contracted_grade(
        &self,
        session: &mut broker_provider::ContractedLaunchSession,
        now: ActivityTimestamp,
    ) -> Result<VerifiedProviderGrade, ImathasAdapterError> {
        self.provider.retrieve_and_verify(session, now).await
    }

    /// Serves only the fixed, adapter-owned provider activity resource.  The
    /// caller cannot select upstream URLs or supply provider headers/cookies.
    pub async fn proxy_contracted_activity(
        &self,
        session: &broker_provider::ContractedLaunchSession,
        method: broker_provider::ProxyMethod,
        body: &[u8],
        now: ActivityTimestamp,
    ) -> Result<broker_provider::ProxyResponse, ImathasAdapterError> {
        self.provider
            .proxy_activity(session, method, body, now)
            .await
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CachedRender {
    schema: u8,
    source: SourceArtifact,
    provider: String,
    profile: String,
    envelope: QuestionEnvelope,
}
fn decode_cache(bytes: &[u8]) -> Result<CachedRender, ImathasAdapterError> {
    serde_json::from_slice(bytes).map_err(|_| ImathasAdapterError::InvalidCache)
}
fn validate_cache(
    c: &CachedRender,
    q: &QuestionDefinition,
    seed: Seed,
    s: &ImathasSource,
) -> Result<(), ImathasAdapterError> {
    if c.schema != 1
        || c.source != s.artifact
        || c.provider != s.provider
        || c.profile != s.profile
        || c.envelope.version != q.version
        || c.envelope.seed != seed
        || question_model::validate_question_title(&c.envelope.title).is_err()
        || !matches!(
            c.envelope.response,
            question_model::ResponseDefinition::ExternalTool {}
        )
    {
        return Err(ImathasAdapterError::InvalidCache);
    }
    Ok(())
}
fn verify_binding(q: &QuestionDefinition, s: &ImathasSource) -> Result<(), ImathasAdapterError> {
    if q.problem != s.problem || q.version != s.version {
        return Err(ImathasAdapterError::SourceDoesNotMatchQuestion);
    }
    match &q.source {
        QuestionSource::Imathas {
            provider,
            item_ref,
            snapshot,
            snapshot_sha256,
            integration_profile,
        } if provider == &s.provider
            && item_ref == &s.item_ref
            && snapshot == &s.artifact.object
            && snapshot_sha256 == &s.artifact.sha256
            && integration_profile == &s.profile =>
        {
            Ok(())
        }
        _ => Err(ImathasAdapterError::SourceDoesNotMatchQuestion),
    }
}
fn render_key(problem: ProblemId, version: VersionId, seed: Seed) -> ObjectKey {
    ObjectKey::ProblemRender {
        problem,
        version,
        seed,
        object: deterministic_id(version, seed),
    }
}
fn deterministic_id(version: VersionId, seed: Seed) -> ObjectId {
    let mut h = Sha256::new();
    h.update(b"peptidyle:imathas:render-cache:v1");
    h.update(version.as_uuid().as_bytes());
    h.update(seed.value().to_be_bytes());
    let digest = h.finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(Uuid::from_bytes(bytes))
}
fn parameter_hash(seed: Seed) -> String {
    let mut h = Sha256::new();
    h.update(b"peptidyle:imathas:parameters:v1");
    h.update(seed.value().to_be_bytes());
    hex(h.finalize().as_slice())
}
fn implementation(id: &str, version: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.into(),
        version: version.into(),
    }
}
fn valid_opaque_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
/// Supported iMathAS item identifiers are deliberately identifier-shaped,
/// rather than URLs or arbitrary provider path fragments. Numeric item IDs and
/// provider opaque IDs share this bounded grammar.
fn valid_item_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && !value.contains("..")
}
fn binding_payload(binding: &GradeBinding) -> Vec<u8> {
    let mut value = Vec::with_capacity(16 * 4 + 8);
    value.extend_from_slice(binding.tenant.as_uuid().as_bytes());
    value.extend_from_slice(binding.attempt.as_uuid().as_bytes());
    value.extend_from_slice(binding.problem.as_uuid().as_bytes());
    value.extend_from_slice(binding.version.as_uuid().as_bytes());
    value.extend_from_slice(&binding.seed.value().to_be_bytes());
    value
}
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}
fn hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut value, "{byte:02x}");
    }
    value
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use objects::memory::MemoryObjectStore;
    use question_model::generation::RandomizationDefinition;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{DraftQuestionSource, GradingDefinition, QuestionMetadata, WorkspaceId};

    use super::*;

    #[derive(Clone)]
    struct RecordedProvider {
        renders: Arc<AtomicUsize>,
        grades: Arc<AtomicUsize>,
        outage: bool,
        mismatch: Option<Mismatch>,
    }

    #[derive(Clone, Copy)]
    enum Mismatch {
        Tenant,
        Attempt,
        Problem,
        Version,
        Seed,
        Correlation,
    }

    impl sealed::ProviderSealed for RecordedProvider {}

    #[async_trait]
    impl ImathasProvider for RecordedProvider {
        async fn snapshot(
            &self,
            locator: &DraftLocator,
        ) -> Result<(Vec<u8>, SupportedProfile), ProviderFailure> {
            assert_eq!(locator.provider(), "recorded-provider");
            assert_eq!(locator.item_ref(), "item-17");
            Ok((b"{\"recorded\":true}".to_vec(), profile()))
        }

        async fn render(
            &self,
            request: ProviderRenderRequest<'_>,
        ) -> Result<SafeProviderRender, ProviderFailure> {
            self.renders.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            if self.outage {
                return Err(ProviderFailure::Unavailable);
            }
            assert_eq!(request.snapshot, b"{\"recorded\":true}");
            assert_eq!(request.profile, "recorded-v1");
            Ok(SafeProviderRender {
                title: "Recorded external question".into(),
                prompt: vec![ContentBlock::Text {
                    markdown: "Complete this iMathAS activity.".into(),
                }],
            })
        }

        async fn verify_grade(
            &self,
            request: ProviderGradeRequest<'_>,
        ) -> Result<VerifiedProviderGrade, ProviderFailure> {
            self.grades.fetch_add(1, Ordering::SeqCst);
            if self.outage {
                return Err(ProviderFailure::Timeout);
            }
            let mut verdict = VerifiedProviderGrade::verified(
                AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                request.tenant(),
                request.attempt(),
                request.problem(),
                request.version(),
                request.seed(),
                request.correlation(),
            );
            match self.mismatch {
                Some(Mismatch::Tenant) => verdict.tenant = TenantId::from_uuid(Uuid::from_u128(99)),
                Some(Mismatch::Attempt) => {
                    verdict.attempt = QuestionAttemptId::from_uuid(Uuid::from_u128(99))
                }
                Some(Mismatch::Problem) => {
                    verdict.problem = ProblemId::from_uuid(Uuid::from_u128(99))
                }
                Some(Mismatch::Version) => {
                    verdict.version = VersionId::from_uuid(Uuid::from_u128(99))
                }
                Some(Mismatch::Seed) => verdict.seed = Seed::new(99),
                Some(Mismatch::Correlation) => {
                    verdict.correlation = "wrong-server-correlation".into()
                }
                None => {}
            }
            Ok(verdict)
        }
    }

    fn profile() -> SupportedProfile {
        SupportedProfile::new("recorded-v1", true, true, true).unwrap()
    }

    fn provider() -> RecordedProvider {
        RecordedProvider {
            renders: Arc::new(AtomicUsize::new(0)),
            grades: Arc::new(AtomicUsize::new(0)),
            outage: false,
            mismatch: None,
        }
    }

    fn question(snapshot: ObjectId, digest: String) -> QuestionDefinition {
        QuestionDefinition {
            problem: ProblemId::from_uuid(Uuid::from_u128(1)),
            version: VersionId::from_uuid(Uuid::from_u128(2)),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(3)),
            source: QuestionSource::Imathas {
                provider: "recorded-provider".into(),
                item_ref: "item-17".into(),
                snapshot,
                snapshot_sha256: digest,
                integration_profile: "recorded-v1".into(),
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
        }
    }

    async fn stored_source(
        store: &MemoryObjectStore,
    ) -> (QuestionDefinition, ImathasSource, PublishedSourceArtifact) {
        let snapshot = ObjectId::from_uuid(Uuid::from_u128(4));
        let digest = hex(Sha256::digest(b"{\"recorded\":true}").as_slice());
        let question = question(snapshot, digest);
        let object = store
            .put(PutObject {
                key: ObjectKey::ProblemSource {
                    problem: question.problem,
                    version: question.version,
                    object: snapshot,
                },
                bytes: b"{\"recorded\":true}".to_vec(),
                media_type: "application/json".into(),
                license: "CC-BY-SA-4.0".into(),
                provenance: "recorded redacted iMathAS fixture".into(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .unwrap();
        let artifact = PublishedSourceArtifact {
            reference: question_model::ProblemVersionRef {
                problem: question.problem,
                version: question.version,
            },
            backend: question_model::QuestionBackend::Imathas,
            object,
        };
        let source = ImathasSource::resolve(store, &question, &artifact)
            .await
            .unwrap();
        (question, source, artifact)
    }

    #[tokio::test]
    async fn draft_snapshot_is_unversioned_and_publication_handoff_is_digest_pinned() {
        let provider = provider();
        let adapter = ImathasAdapter::new(MemoryObjectStore::default(), provider, [profile()]);
        let prepared = adapter
            .prepare_snapshot(&DraftQuestionSource::Imathas {
                provider: "recorded-provider".into(),
                item_ref: "item-17".into(),
            })
            .await
            .unwrap();
        assert_eq!(prepared.bytes(), b"{\"recorded\":true}");
        assert_eq!(prepared.profile().name(), "recorded-v1");
        assert!(!format!("{prepared:?}").contains("recorded\\\":true"));
        assert!(
            DraftLocator::from_draft(&DraftQuestionSource::Imathas {
                provider: "https://untrusted.example".into(),
                item_ref: "item-17".into(),
            })
            .is_err()
        );
        for item_ref in [
            "https://provider.example/item",
            "17?token=secret",
            "17#fragment",
            "item with-space",
            "item\n17",
            &"a".repeat(129),
        ] {
            assert!(
                DraftLocator::from_draft(&DraftQuestionSource::Imathas {
                    provider: "recorded-provider".into(),
                    item_ref: item_ref.into(),
                })
                .is_err()
            );
        }
        assert_eq!(
            format!(
                "{:?}",
                DraftLocator::from_draft(&DraftQuestionSource::Imathas {
                    provider: "recorded-provider".into(),
                    item_ref: "item-17".into(),
                })
                .unwrap()
            ),
            "DraftLocator(REDACTED)"
        );
    }

    #[tokio::test]
    async fn immutable_snapshot_cache_and_verified_grade_are_bound_to_exact_attempt() {
        let store = MemoryObjectStore::default();
        let recorded = provider();
        let renders = recorded.renders.clone();
        let adapter = ImathasAdapter::new(store.clone(), recorded, [profile()]);
        let (question, source, _) = stored_source(&store).await;
        let first = adapter
            .issue(
                &question,
                Seed::new(17),
                &source,
                ActivityTimestamp::from_unix_millis(2),
            )
            .await
            .unwrap();
        let second = adapter
            .issue(
                &question,
                Seed::new(17),
                &source,
                ActivityTimestamp::from_unix_millis(3),
            )
            .await
            .unwrap();
        assert!(!first.cache_hit);
        assert!(second.cache_hit);
        assert_eq!(renders.load(Ordering::SeqCst), 1);
        assert_eq!(first.envelope.title, "Recorded external question");
        assert_eq!(second.envelope.title, first.envelope.title);
        assert!(matches!(
            first.envelope.response,
            question_model::ResponseDefinition::ExternalTool {}
        ));
        let serialized = serde_json::to_string(&first.envelope).unwrap();
        for forbidden in ["token", "launch", "score", "correct", "recorded\\\":true"] {
            assert!(!serialized.contains(forbidden));
        }
        let result = adapter
            .grade(
                &question,
                &source,
                TenantId::from_uuid(Uuid::from_u128(5)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                Seed::new(17),
                &correlation(&question, Seed::new(17)),
            )
            .await
            .unwrap();
        assert!(result.result().correct);
    }

    #[tokio::test]
    async fn historical_invalid_metadata_title_is_refused_before_provider_or_cache() {
        let store = MemoryObjectStore::default();
        let recorded = provider();
        let renders = recorded.renders.clone();
        let adapter = ImathasAdapter::new(store.clone(), recorded, [profile()]);
        let (mut question, source, _) = stored_source(&store).await;
        question.metadata.title = " \n ".into();
        assert!(matches!(
            adapter
                .issue(
                    &question,
                    Seed::new(17),
                    &source,
                    ActivityTimestamp::from_unix_millis(2),
                )
                .await,
            Err(ImathasAdapterError::InvalidTitle(_))
        ));
        assert_eq!(renders.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn snapshot_mutation_wrong_binding_and_outage_refuse_without_fabricating_incorrectness() {
        let store = MemoryObjectStore::default();
        let (question, source, artifact) = stored_source(&store).await;
        let mut changed_source = question.clone();
        if let QuestionSource::Imathas {
            snapshot_sha256, ..
        } = &mut changed_source.source
        {
            *snapshot_sha256 = "00".repeat(32);
        }
        assert_eq!(
            ImathasSource::resolve(&store, &changed_source, &artifact)
                .await
                .unwrap_err(),
            ImathasAdapterError::UntrustedSource
        );
        let wrong = ImathasAdapter::new(
            store.clone(),
            RecordedProvider {
                mismatch: Some(Mismatch::Version),
                ..provider()
            },
            [profile()],
        );
        let error = wrong
            .grade(
                &question,
                &source,
                TenantId::from_uuid(Uuid::from_u128(5)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                Seed::new(17),
                &correlation(&question, Seed::new(17)),
            )
            .await
            .unwrap_err();
        assert_eq!(error, ImathasAdapterError::VerificationRefused);
        let outage = ImathasAdapter::new(
            store,
            RecordedProvider {
                outage: true,
                ..provider()
            },
            [profile()],
        );
        assert!(matches!(
            outage
                .issue(
                    &question,
                    Seed::new(18),
                    &source,
                    ActivityTimestamp::from_unix_millis(2)
                )
                .await,
            Err(ImathasAdapterError::Provider(ProviderFailure::Unavailable))
        ));
    }

    #[tokio::test]
    async fn every_verified_grade_binding_dimension_and_restored_handle_is_checked() {
        let store = MemoryObjectStore::default();
        let (question, source, _) = stored_source(&store).await;
        for mismatch in [
            Mismatch::Tenant,
            Mismatch::Attempt,
            Mismatch::Problem,
            Mismatch::Version,
            Mismatch::Seed,
            Mismatch::Correlation,
        ] {
            let adapter = ImathasAdapter::new(
                store.clone(),
                RecordedProvider {
                    mismatch: Some(mismatch),
                    ..provider()
                },
                [profile()],
            );
            assert_eq!(
                adapter
                    .grade(
                        &question,
                        &source,
                        TenantId::from_uuid(Uuid::from_u128(5)),
                        QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                        Seed::new(17),
                        &correlation(&question, Seed::new(17)),
                    )
                    .await
                    .unwrap_err(),
                ImathasAdapterError::VerificationRefused
            );
        }
        let issuer = CorrelationIssuer::from_server_secret([8; 32]);
        let binding = GradeBinding {
            tenant: TenantId::from_uuid(Uuid::from_u128(5)),
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
            problem: question.problem,
            version: question.version,
            seed: Seed::new(17),
        };
        let persisted = issuer.begin(binding);
        let stored_value = persisted.to_storage_value();
        let after_restart = PersistedCorrelation::from_storage_value(&stored_value).unwrap();
        let restored = issuer.restore(binding, &after_restart).unwrap();
        let adapter = ImathasAdapter::new(store.clone(), provider(), [profile()]);
        assert!(
            adapter
                .grade(
                    &question,
                    &source,
                    binding.tenant,
                    binding.attempt,
                    binding.seed,
                    &restored,
                )
                .await
                .unwrap()
                .result()
                .correct
        );
        let mut altered = stored_value.clone().into_bytes();
        altered[0] = if altered[0] == b'f' { b'e' } else { b'f' };
        let altered = String::from_utf8(altered).unwrap();
        let altered = PersistedCorrelation::from_storage_value(&altered).unwrap();
        assert!(issuer.restore(binding, &altered).is_err());
        let wrong_issuer = CorrelationIssuer::from_server_secret([9; 32]);
        assert!(wrong_issuer.restore(binding, &after_restart).is_err());
        assert!(
            PersistedCorrelation::from_storage_value(&stored_value[..stored_value.len() - 1])
                .is_err()
        );
        assert!(PersistedCorrelation::from_storage_value(&"a".repeat(1024)).is_err());
        assert!(
            issuer
                .restore(
                    GradeBinding {
                        seed: Seed::new(18),
                        ..binding
                    },
                    &persisted
                )
                .is_err()
        );
    }

    #[tokio::test]
    async fn malformed_stored_cache_and_grade_outage_remain_local_and_redacted() {
        let store = MemoryObjectStore::default();
        let (question, source, _) = stored_source(&store).await;
        let key = render_key(question.problem, question.version, Seed::new(31));
        store
            .put(PutObject {
                key,
                bytes: b"{malformed".to_vec(),
                media_type: "application/json".into(),
                license: "test".into(),
                provenance: "test".into(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .unwrap();
        let adapter = ImathasAdapter::new(store.clone(), provider(), [profile()]);
        assert_eq!(
            adapter
                .issue(
                    &question,
                    Seed::new(31),
                    &source,
                    ActivityTimestamp::from_unix_millis(2)
                )
                .await
                .unwrap_err(),
            ImathasAdapterError::InvalidCache
        );
        let outage = ImathasAdapter::new(
            store,
            RecordedProvider {
                outage: true,
                ..provider()
            },
            [profile()],
        );
        assert!(matches!(
            outage
                .grade(
                    &question,
                    &source,
                    TenantId::from_uuid(Uuid::from_u128(5)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                    Seed::new(17),
                    &correlation(&question, Seed::new(17)),
                )
                .await,
            Err(ImathasAdapterError::Provider(ProviderFailure::Timeout))
        ));
        let text = ImathasAdapterError::Provider(ProviderFailure::Unavailable).to_string();
        assert!(!text.contains("token"));
        assert!(text.len() < 100);
    }

    #[tokio::test]
    async fn concurrent_replicas_reuse_the_winning_immutable_render() {
        let store = MemoryObjectStore::default();
        let recorded = provider();
        let adapter = ImathasAdapter::new(store.clone(), recorded, [profile()]);
        let (question, source, _) = stored_source(&store).await;
        let first = adapter.issue(
            &question,
            Seed::new(41),
            &source,
            ActivityTimestamp::from_unix_millis(2),
        );
        let second = adapter.issue(
            &question,
            Seed::new(41),
            &source,
            ActivityTimestamp::from_unix_millis(2),
        );
        let (first, second) = tokio::join!(first, second);
        let first = first.unwrap();
        let second = second.unwrap();
        assert!(first.cache_hit || second.cache_hit);
        assert_eq!(first.envelope, second.envelope);
    }

    fn correlation(question: &QuestionDefinition, seed: Seed) -> ServerCorrelation {
        let issuer = CorrelationIssuer::from_server_secret([7; 32]);
        let binding = GradeBinding {
            tenant: TenantId::from_uuid(Uuid::from_u128(5)),
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
            problem: question.problem,
            version: question.version,
            seed,
        };
        let persisted = issuer.begin(binding);
        issuer.restore(binding, &persisted).unwrap()
    }
}
