//! iMathAS Question Backend-facing, answer-safe contracts and draft snapshot preparation.

use async_trait::async_trait;
use question_model::QuestionContentBlock;
use question_model::generation::QuestionSeed;
use question_model::{
    ImathasDeploymentReference, ImathasItemReference, ImathasProfile, QuestionRevisionReference,
};

use crate::{ImathasAdapterError, ImathasQuestionBackendFailure, VerifiedImathasResult};

/// An iMathAS deployment's publication-safe integration profile.
///
/// No endpoint, credential, accepted origin, or launch protocol is carried in
/// this value. Those belong to its iMathAS Deployment Reference.
#[derive(Clone, PartialEq, Eq)]
pub struct SupportedImathasProfile {
    pub(crate) profile: ImathasProfile,
    pub(crate) deterministic_seeded_render: bool,
    pub(crate) verified_server_grading: bool,
    pub(crate) partial_credit: bool,
}

impl std::fmt::Debug for SupportedImathasProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SupportedImathasProfile")
            .field("profile", &self.profile)
            .field(
                "deterministic_seeded_render",
                &self.deterministic_seeded_render,
            )
            .field("verified_server_grading", &self.verified_server_grading)
            .field("partial_credit", &self.partial_credit)
            .finish()
    }
}

impl SupportedImathasProfile {
    /// Constructs an explicitly supported protocol profile.
    pub fn new(
        profile: ImathasProfile,
        deterministic_seeded_render: bool,
        verified_server_grading: bool,
        partial_credit: bool,
    ) -> Result<Self, ImathasAdapterError> {
        if partial_credit && !verified_server_grading {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        Ok(Self {
            profile,
            deterministic_seeded_render,
            verified_server_grading,
            partial_credit,
        })
    }

    /// Pinned profile name persisted with a published source.
    pub fn profile(&self) -> &ImathasProfile {
        &self.profile
    }
}

/// The configured iMathAS Deployment Reference and iMathAS Item Reference used
/// to retrieve a Draft Question snapshot.
/// It contains no source bytes, endpoint, or credential.
#[derive(Clone, PartialEq, Eq)]
pub struct ImathasQuestionLocation {
    deployment_reference: ImathasDeploymentReference,
    item: ImathasItemReference,
}

impl ImathasQuestionLocation {
    /// Creates the iMathAS location from its exact editable binding.
    pub fn from_draft_imathas_question_backend_binding(
        binding: &question_model::DraftImathasQuestionBackendBinding,
    ) -> Self {
        Self {
            deployment_reference: binding.deployment_reference().clone(),
            item: binding.item_reference().clone(),
        }
    }

    /// Opaque deployment configuration selector.
    pub fn deployment_reference(&self) -> &ImathasDeploymentReference {
        &self.deployment_reference
    }

    /// iMathAS-backend-local item reference.
    pub fn item_reference(&self) -> &ImathasItemReference {
        &self.item
    }
}

impl std::fmt::Debug for ImathasQuestionLocation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasQuestionLocation(REDACTED)")
    }
}

/// Server-private immutable bytes prepared before publication. It has no
/// Question ID or Question Revision Number; publication alone owns identity.
#[derive(Clone, PartialEq, Eq)]
pub struct PreparedSnapshot {
    pub(crate) bytes: Vec<u8>,
    pub(crate) sha256: String,
    pub(crate) profile: SupportedImathasProfile,
}

impl PreparedSnapshot {
    /// Exact source bytes for the trusted worker/object-store handoff.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Pinned digest to put into the published Question Source.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Validated integration profile.
    pub fn profile(&self) -> &SupportedImathasProfile {
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

/// Browser-safe iMathAS Question Backend render.
///
/// It contains a Question Prompt and cannot contain iframe markup, launch URLs,
/// tokens, callbacks, answers, or scores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeImathasQuestionRender {
    /// Plain prompt blocks, already constrained by the adapter boundary.
    pub prompt: Vec<QuestionContentBlock>,
    /// A browser-safe Question title.
    pub title: String,
}

/// Implementation seal: external crates cannot install an iMathAS Question Backend that
/// constructs a grade proof without the adapter-owned verifier.
pub(crate) mod sealed {
    pub trait QuestionBackendSealed {}
}

/// Server-side iMathAS Question Backend client. Implementations keep deployment URLs,
/// credentials, network timeout policy, and trust verification private.
#[async_trait]
pub trait QuestionBackend: sealed::QuestionBackendSealed + Send + Sync {
    /// Fetches exact source bytes and an explicitly supported profile for an unversioned draft.
    async fn snapshot(
        &self,
        locator: &ImathasQuestionLocation,
    ) -> Result<(Vec<u8>, SupportedImathasProfile), ImathasQuestionBackendFailure>;

    /// Produces only a browser-safe Question Prompt from archived source bytes.
    async fn render(
        &self,
        request: ImathasRenderRequest<'_>,
    ) -> Result<SafeImathasQuestionRender, ImathasQuestionBackendFailure>;

    /// Authenticates and correlates an upstream grade server-to-server.
    async fn verify_result(
        &self,
        request: ImathasResultRequest<'_>,
    ) -> Result<VerifiedImathasResult, ImathasQuestionBackendFailure>;
}

/// Immutable inputs for one iMathAS render. No browser data is present.
pub struct ImathasRenderRequest<'a> {
    /// Exact archived source bytes.
    pub snapshot: &'a [u8],
    /// Pinned source profile.
    pub profile: &'a str,
    /// Exact immutable Question Revision.
    pub question_revision: QuestionRevisionReference,
    /// Deterministic Question Seed.
    pub seed: QuestionSeed,
}

/// Server-held, attempt-bound iMathAS result request. Private fields prevent a
/// browser request from constructing a launch_session_authentication or score payload.
pub struct ImathasResultRequest<'a> {
    pub(crate) snapshot: &'a [u8],
    pub(crate) profile: &'a str,
    pub(crate) grading_context: &'a learning_data_access::ImathasGradingContext,
    pub(crate) launch_session_authentication:
        &'a learning_data_access::ImathasQuestionBackendSessionAuthentication,
}

impl<'a> ImathasResultRequest<'a> {
    pub fn snapshot(&self) -> &'a [u8] {
        self.snapshot
    }
    pub fn profile(&self) -> &'a str {
        self.profile
    }
    /// Exact server-owned grading identity for this iMathAS request.
    pub fn grading_context(&self) -> &learning_data_access::ImathasGradingContext {
        self.grading_context
    }

    /// Opaque server-held value transmitted only by the iMathAS Question Backend client.
    pub fn launch_session_authentication(
        &self,
    ) -> &learning_data_access::ImathasQuestionBackendSessionAuthentication {
        self.launch_session_authentication
    }
}
