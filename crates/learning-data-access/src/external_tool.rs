//! Backend-neutral contract for server-mediated external learning tools.
//!
//! The browser-facing launch capability, provider correlation, verification
//! lease, and submission state machine stay non-serializable and redact their
//! diagnostics. Backend implementations live in their respective
//! `memory::external_tool` and `postgres::external_tool` modules.

use async_trait::async_trait;
use base64::Engine;
use objects::Sha256Digest;
use question_model::{
    ActivityTimestamp, AttemptResult, ObjectId, ProblemId, QuestionAttemptId, StudentResponse,
    UserId, VersionId,
};
use uuid::Uuid;

use crate::{StoreError, SubmissionIdempotencyKey, SubmissionRecord, TenantContext};

/// Exact immutable binding for one server-mediated external-tool exchange.
///
/// This is deliberately a store-private/server-core contract, rather than a
/// catalog or browser DTO. In particular, `provider` is a configured opaque
/// deployment key, never an endpoint.
#[derive(Clone, PartialEq, Eq)]
pub struct ExternalToolBinding {
    pub provider: String,
    pub problem: ProblemId,
    pub version: VersionId,
    pub seed: u64,
    pub source_object: ObjectId,
    pub source_sha256: String,
    pub integration_profile: String,
    pub response_sha256: Sha256Digest,
}

impl std::fmt::Debug for ExternalToolBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolBinding([redacted])")
    }
}

impl ExternalToolBinding {
    /// Rejects unbounded opaque deployment values before they reach storage.
    pub fn validate(&self) -> Result<(), StoreError> {
        for (name, value, max) in [
            ("provider", self.provider.as_str(), 160usize),
            ("source checksum", self.source_sha256.as_str(), 64),
            (
                "integration profile",
                self.integration_profile.as_str(),
                160,
            ),
        ] {
            if value.is_empty()
                || value.len() > max
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            {
                return Err(StoreError::InvalidRecord(format!(
                    "external tool {name} is invalid"
                )));
            }
        }
        if self.source_sha256.len() != 64
            || !self
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(StoreError::InvalidRecord(
                "external tool source checksum is invalid".to_string(),
            ));
        }
        Ok(())
    }
}

/// Opaque persisted provider correlation. It cannot be serialized or logged.
#[derive(Clone, PartialEq, Eq)]
pub struct PersistedCorrelation(Vec<u8>);

impl PersistedCorrelation {
    pub fn new(value: Vec<u8>) -> Result<Self, StoreError> {
        if value.is_empty() || value.len() > 512 {
            return Err(StoreError::InvalidRecord(
                "external-tool correlation must be 1 to 512 bytes".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// Returns an owned copy for a server-only adapter codec.
    ///
    /// This type remains non-serde and redacts diagnostics; callers must
    /// validate their own authenticated correlation format before sending
    /// anything to an external provider.
    pub fn to_storage_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn from_stored(value: Vec<u8>) -> Result<Self, StoreError> {
        Self::new(value)
    }
}

impl std::fmt::Debug for PersistedCorrelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PersistedCorrelation([redacted])")
    }
}

/// Opaque short-lived lease proving that a replica owns verification work.
#[derive(Clone, PartialEq, Eq)]
pub struct ExternalToolLeaseToken([u8; 32]);

impl ExternalToolLeaseToken {
    pub(crate) fn generate() -> Result<Self, StoreError> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|error| {
            StoreError::Unavailable(format!("external-tool lease entropy unavailable: {error}"))
        })?;
        Ok(Self(bytes))
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub(crate) fn hash(&self) -> Sha256Digest {
        Sha256Digest::compute(&self.0)
    }
}

impl std::fmt::Debug for ExternalToolLeaseToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolLeaseToken([redacted])")
    }
}

/// Work a single replica may send to a configured external provider.
#[derive(Clone)]
pub struct ExternalToolLease {
    pub binding: ExternalToolBinding,
    pub correlation: PersistedCorrelation,
    pub token: ExternalToolLeaseToken,
    pub expires_at: ActivityTimestamp,
}

impl std::fmt::Debug for ExternalToolLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalToolLease")
            .field("binding", &self.binding)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// A server-verified grade retained until its ordinary attempt submission commits.
#[derive(Clone, PartialEq)]
pub struct ExternalToolVerifiedPending {
    /// Immutable server-only binding restored after a verifier crash.
    pub binding: ExternalToolBinding,
    /// Opaque persisted correlation reused for the exact recovery commit.
    pub correlation: PersistedCorrelation,
    pub result: AttemptResult,
    pub result_sha256: Sha256Digest,
}

impl std::fmt::Debug for ExternalToolVerifiedPending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolVerifiedPending([redacted])")
    }
}

/// Result of atomically claiming or resuming a provider exchange.
#[derive(Clone)]
pub enum ExternalToolBegin {
    Committed(Box<SubmissionRecord>),
    VerifiedPending(ExternalToolVerifiedPending),
    Lease(ExternalToolLease),
    InProgress,
}

impl std::fmt::Debug for ExternalToolBegin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = match self {
            Self::Committed(_) => "Committed",
            Self::VerifiedPending(_) => "VerifiedPending",
            Self::Lease(_) => "Lease",
            Self::InProgress => "InProgress",
        };
        write!(f, "ExternalToolBegin::{state}([redacted])")
    }
}

/// Input to the atomic exchange claim.
#[derive(Clone)]
pub struct BeginExternalToolGradeCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: SubmissionIdempotencyKey,
    pub binding: ExternalToolBinding,
    pub proposed_correlation: PersistedCorrelation,
    pub lease_millis: u32,
}

/// Authenticated result staged by the lease holder before final commit.
#[derive(Clone)]
pub struct StageExternalToolVerificationCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: SubmissionIdempotencyKey,
    pub binding: ExternalToolBinding,
    pub correlation: PersistedCorrelation,
    pub lease_token: ExternalToolLeaseToken,
    pub result: AttemptResult,
}

/// Commits the staged verified grade through the ordinary attempt transition.
#[derive(Clone)]
pub struct CommitExternalToolSubmissionCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: SubmissionIdempotencyKey,
    pub binding: ExternalToolBinding,
    pub correlation: PersistedCorrelation,
    pub lease_token: ExternalToolLeaseToken,
    /// One-time same-origin frame capability consumed with the grade commit.
    pub launch_proof: ExternalToolLaunchProof,
}

/// Commits a previously staged provider verdict after the original verifier
/// lost its lease or process. This is server-only and intentionally carries no
/// lease token: exact binding, response, key, and correlation select the one
/// immutable `verified_pending` record.
#[derive(Clone)]
pub struct CommitVerifiedExternalToolSubmissionCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: SubmissionIdempotencyKey,
    pub binding: ExternalToolBinding,
    pub correlation: PersistedCorrelation,
    /// One-time same-origin frame capability consumed with the recovered commit.
    pub launch_proof: ExternalToolLaunchProof,
}

/// Durable external-grade state machine. This is intentionally not serde.
#[async_trait]
pub trait ExternalToolBrokerStore: Send + Sync {
    async fn begin_or_resume_external_grade(
        &self,
        context: TenantContext,
        command: BeginExternalToolGradeCommand,
    ) -> Result<ExternalToolBegin, StoreError>;

    async fn stage_external_tool_verification(
        &self,
        context: TenantContext,
        command: StageExternalToolVerificationCommand,
    ) -> Result<(), StoreError>;

    async fn commit_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError>;

    async fn commit_verified_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitVerifiedExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError>;
}

/// Browser cookie material minted once for a short-lived same-origin launch.
/// It is intentionally non-serde and redacts itself in diagnostics.
#[derive(Clone, PartialEq, Eq)]
pub struct ExternalToolLaunchToken([u8; 32]);

impl ExternalToolLaunchToken {
    pub(crate) fn generate() -> Result<Self, StoreError> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|error| {
            StoreError::Unavailable(format!("external-tool launch entropy unavailable: {error}"))
        })?;
        Ok(Self(bytes))
    }

    pub(crate) fn hash(&self) -> Sha256Digest {
        Sha256Digest::compute(&self.0)
    }

    /// Canonical cookie representation for the server-owned launch route.
    /// This remains opaque, non-serde, and never appears in a DTO.
    pub fn encode_cookie_value(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.0)
    }

    /// Parses only the exact 32-byte unpadded base64url cookie value.
    pub fn parse_cookie_value(value: &str) -> Result<Self, StoreError> {
        if value.len() != 43
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(StoreError::InvalidRecord(
                "external-tool launch token is invalid".into(),
            ));
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| {
                StoreError::InvalidRecord("external-tool launch token is invalid".into())
            })?;
        let array: [u8; 32] = bytes.try_into().map_err(|_| {
            StoreError::InvalidRecord("external-tool launch token is invalid".into())
        })?;
        if base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(array) != value {
            return Err(StoreError::InvalidRecord(
                "external-tool launch token is invalid".into(),
            ));
        }
        Ok(Self(array))
    }
}

pub(crate) fn fresh_external_tool_launch_id() -> Result<Uuid, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!("external-tool launch entropy unavailable: {error}"))
    })?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(Uuid::from_bytes(bytes))
}

impl std::fmt::Debug for ExternalToolLaunchToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolLaunchToken([redacted])")
    }
}

/// Server-only proof that the learner still owns the exact same-origin launch
/// session used for this external-tool submission. It is non-serde and redacts
/// cookie material in diagnostics.
#[derive(Clone)]
pub struct ExternalToolLaunchProof {
    pub session_id: Uuid,
    pub token: ExternalToolLaunchToken,
}

impl std::fmt::Debug for ExternalToolLaunchProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolLaunchProof([redacted])")
    }
}

/// Input to a server-created frame launch session. Provider state is already
/// encrypted by server configuration; it is never a browser token or URL.
#[derive(Clone)]
pub struct CreateExternalToolLaunchSessionCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub binding: ExternalToolBinding,
    pub encrypted_provider_state: Option<Vec<u8>>,
    pub lifetime_millis: u32,
}

/// The only time raw cookie bytes leave the Store boundary.
#[derive(Clone)]
pub struct CreatedExternalToolLaunchSession {
    pub id: Uuid,
    pub token: ExternalToolLaunchToken,
    pub expires_at: ActivityTimestamp,
}

impl std::fmt::Debug for CreatedExternalToolLaunchSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreatedExternalToolLaunchSession")
            .field("id", &self.id)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Server-only resolved launch state. It is never serialized into a route.
#[derive(Clone)]
pub struct ResolvedExternalToolLaunchSession {
    pub binding: ExternalToolBinding,
    pub encrypted_provider_state: Option<Vec<u8>>,
}

impl std::fmt::Debug for ResolvedExternalToolLaunchSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedExternalToolLaunchSession")
            .field("binding", &self.binding)
            .field(
                "encrypted_provider_state",
                &self.encrypted_provider_state.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

#[async_trait]
pub trait ExternalToolLaunchSessionStore: Send + Sync {
    async fn create_external_tool_launch_session(
        &self,
        context: TenantContext,
        command: CreateExternalToolLaunchSessionCommand,
    ) -> Result<CreatedExternalToolLaunchSession, StoreError>;

    async fn resolve_external_tool_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolLaunchToken,
    ) -> Result<Option<ResolvedExternalToolLaunchSession>, StoreError>;

    async fn revoke_external_tool_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
    ) -> Result<(), StoreError>;
}
