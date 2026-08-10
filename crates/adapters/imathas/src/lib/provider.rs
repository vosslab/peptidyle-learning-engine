//! Provider-facing, answer-safe contracts and draft snapshot preparation.

use async_trait::async_trait;
use question_model::envelope::ContentBlock;
use question_model::generation::Seed;
use question_model::{ProblemId, QuestionAttemptId, TenantId, VersionId};

use crate::{
    ImathasAdapterError, ProviderFailure, ServerCorrelation, VerifiedProviderGrade,
    cache::{valid_item_ref, valid_opaque_key},
};

/// A provider's publication-safe integration profile.
///
/// No endpoint, credential, accepted origin, or launch protocol is carried in
/// this value. Those belong to deployment configuration behind `provider`.
#[derive(Clone, PartialEq, Eq)]
pub struct SupportedProfile {
    pub(crate) name: String,
    pub(crate) deterministic_seeded_render: bool,
    pub(crate) verified_server_grading: bool,
    pub(crate) partial_credit: bool,
}

impl std::fmt::Debug for SupportedProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SupportedProfile")
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
        if name.is_empty() || name.len() > 128 || (partial_credit && !verified_server_grading) {
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
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DraftLocator(REDACTED)")
    }
}

/// Server-private immutable bytes prepared before publication. It has no
/// `ProblemId`/`VersionId`; the publication transaction alone owns identity.
#[derive(Clone, PartialEq, Eq)]
pub struct PreparedSnapshot {
    pub(crate) bytes: Vec<u8>,
    pub(crate) sha256: String,
    pub(crate) profile: SupportedProfile,
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
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedSnapshot")
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

/// Implementation seal: external crates cannot install a provider that
/// constructs a grade proof without the adapter-owned verifier.
pub(crate) mod sealed {
    pub trait ProviderSealed {}
}

/// Server-side provider client. Implementations keep provider URLs,
/// credentials, network timeout policy, and trust verification private.
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
    pub(crate) snapshot: &'a [u8],
    pub(crate) profile: &'a str,
    pub(crate) tenant: TenantId,
    pub(crate) attempt: QuestionAttemptId,
    pub(crate) problem: ProblemId,
    pub(crate) version: VersionId,
    pub(crate) seed: Seed,
    pub(crate) correlation: &'a ServerCorrelation,
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
