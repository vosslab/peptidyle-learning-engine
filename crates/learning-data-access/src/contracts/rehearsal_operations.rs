//! Durable, route-shaped idempotency operations for instructor rehearsal.
//!
//! The values here deliberately carry only opaque server capabilities and
//! answer-free projections.  The execution coordinator owns issue and grade
//! work; this Store boundary owns the durable operation protocol around it.

use async_trait::async_trait;
use question_model::{RehearsalAttemptId, RehearsalFrozenItemEvidence, ResponseDefinition};
use serde_json::Value;
use uuid::Uuid;

use super::{RehearsalIssuedExecutionArtifactV1, SealedRehearsalDeliveryIssueWork};
use crate::{
    FlatGradingCapability, NativeExecutionEnvelopeCapability, QtiGradingCapability,
    WebworkGradingCapability,
};
use crate::{
    IssuedQuestionSnapshotV1, PrefetchedPrivateExecutionV1, RehearsalIdempotencyKey,
    RehearsalLocator, StoreError, TenantContext,
};

pub const MAX_REHEARSAL_OPERATION_RESPONSE_BYTES: usize = 64 * 1024;
pub const MAX_REHEARSAL_OPERATION_SCREEN_BYTES: usize = 256 * 1024;
pub const MAX_REHEARSAL_DELIVERY_PLAN_BYTES: usize = 512 * 1024;

/// Versioned, answer-free binding for one rehearsal delivery generation.
///
/// The descriptor is deliberately separate from the sealed family contract:
/// routing and presentation may validate this commitment without acquiring
/// answer keys, renderer state, or external-provider credentials.
#[derive(Clone, PartialEq)]
pub struct RehearsalDeliveryExecutionDescriptorV1 {
    attempt: RehearsalAttemptId,
    problem: question_model::ProblemVersionRef,
    response_definition: ResponseDefinition,
    frozen_content_digest: question_model::RehearsalEvidenceDigest,
    deterministic_seed: u64,
    selection_algorithm_version: u16,
}

impl RehearsalDeliveryExecutionDescriptorV1 {
    #[allow(dead_code)]
    pub(crate) fn from_frozen(
        frozen: RehearsalFrozenItemEvidence,
        deterministic_seed: u64,
        selection_algorithm_version: u16,
    ) -> Self {
        Self {
            attempt: frozen.attempt,
            problem: frozen.problem,
            response_definition: frozen.response_definition,
            frozen_content_digest: frozen.canonical_content_digest,
            deterministic_seed,
            selection_algorithm_version,
        }
    }

    pub fn attempt(&self) -> RehearsalAttemptId {
        self.attempt
    }
    pub fn problem(&self) -> question_model::ProblemVersionRef {
        self.problem
    }
    pub fn response_definition(&self) -> &ResponseDefinition {
        &self.response_definition
    }
    pub fn frozen_content_digest(&self) -> question_model::RehearsalEvidenceDigest {
        self.frozen_content_digest
    }
    pub fn deterministic_seed(&self) -> u64 {
        self.deterministic_seed
    }
    pub fn selection_algorithm_version(&self) -> u16 {
        self.selection_algorithm_version
    }

    /// Reconstructs only the answer-free descriptor emitted by the sealed
    /// broker.  The caller separately validates the embedded issued snapshot.
    #[cfg(feature = "postgres")]
    pub(crate) fn decode_persisted(value: &Value) -> Result<Self, StoreError> {
        let object = value.as_object().ok_or_else(|| {
            StoreError::Unavailable("sealed rehearsal descriptor is invalid".into())
        })?;
        let uuid = |name: &str| -> Result<Uuid, StoreError> {
            object
                .get(name)
                .and_then(Value::as_str)
                .and_then(|raw| Uuid::parse_str(raw).ok())
                .ok_or_else(|| {
                    StoreError::Unavailable(format!("sealed rehearsal {name} is invalid"))
                })
        };
        let seed = object
            .get("deterministicSeed")
            .and_then(Value::as_u64)
            .ok_or_else(|| StoreError::Unavailable("sealed rehearsal seed is invalid".into()))?;
        let selection_algorithm_version = object
            .get("selectionAlgorithmVersion")
            .and_then(Value::as_u64)
            .and_then(|raw| u16::try_from(raw).ok())
            .ok_or_else(|| {
                StoreError::Unavailable("sealed rehearsal selection version is invalid".into())
            })?;
        let response_definition =
            serde_json::from_value(object.get("responseDefinition").cloned().ok_or_else(|| {
                StoreError::Unavailable("sealed rehearsal response definition is absent".into())
            })?)
            .map_err(|_| {
                StoreError::Unavailable("sealed rehearsal response definition is invalid".into())
            })?;
        let digest = object
            .get("canonicalContentDigest")
            .and_then(Value::as_str)
            .and_then(|raw| question_model::RehearsalEvidenceDigest::parse_hex(raw).ok())
            .ok_or_else(|| {
                StoreError::Unavailable("sealed rehearsal content digest is invalid".into())
            })?;
        Ok(Self {
            attempt: RehearsalAttemptId::from_uuid(uuid("attemptId")?),
            problem: question_model::ProblemVersionRef {
                problem: question_model::ProblemId::from_uuid(uuid("problemId")?),
                version: question_model::VersionId::from_uuid(uuid("versionId")?),
            },
            response_definition,
            frozen_content_digest: digest,
            deterministic_seed: seed,
            selection_algorithm_version,
        })
    }
}

impl std::fmt::Debug for RehearsalDeliveryExecutionDescriptorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RehearsalDeliveryExecutionDescriptorV1")
            .field("attempt", &self.attempt)
            .field("problem", &self.problem)
            .field(
                "selection_algorithm_version",
                &self.selection_algorithm_version,
            )
            .finish()
    }
}

/// Answer-free, immutable material the coordinator may use to prepare an
/// issue/render operation.  It deliberately excludes family grader contracts.
#[derive(Clone, PartialEq)]
pub struct RehearsalDeliveryIssueDescriptorV1 {
    attempt: RehearsalAttemptId,
    problem: question_model::ProblemVersionRef,
    response_definition: ResponseDefinition,
    frozen_content_digest: question_model::RehearsalEvidenceDigest,
    deterministic_seed: u64,
    selection_algorithm_version: u16,
}

impl RehearsalDeliveryIssueDescriptorV1 {
    #[allow(dead_code)]
    pub(crate) fn from_frozen(
        frozen: RehearsalFrozenItemEvidence,
        deterministic_seed: u64,
        selection_algorithm_version: u16,
    ) -> Self {
        Self {
            attempt: frozen.attempt,
            problem: frozen.problem,
            response_definition: frozen.response_definition,
            frozen_content_digest: frozen.canonical_content_digest,
            deterministic_seed,
            selection_algorithm_version,
        }
    }

    pub fn attempt(&self) -> RehearsalAttemptId {
        self.attempt
    }
    pub fn problem(&self) -> question_model::ProblemVersionRef {
        self.problem
    }
    pub fn response_definition(&self) -> &ResponseDefinition {
        &self.response_definition
    }
    pub fn frozen_content_digest(&self) -> question_model::RehearsalEvidenceDigest {
        self.frozen_content_digest
    }
    pub fn deterministic_seed(&self) -> u64 {
        self.deterministic_seed
    }
    pub fn selection_algorithm_version(&self) -> u16 {
        self.selection_algorithm_version
    }
}

impl std::fmt::Debug for RehearsalDeliveryIssueDescriptorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RehearsalDeliveryIssueDescriptorV1")
            .field("attempt", &self.attempt)
            .field("problem", &self.problem)
            .field(
                "selection_algorithm_version",
                &self.selection_algorithm_version,
            )
            .finish()
    }
}

/// Closed execution-family authority retained only by the privileged execution
/// facade.  It is purposefully a capability tag, never a JSON bag of answers,
/// checker source, renderer state, or provider credentials.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RehearsalPrivateExecutionCapabilityV1 {
    Native(NativeExecutionEnvelopeCapability),
    Flat(FlatGradingCapability),
    Webwork(WebworkGradingCapability),
    Qti(QtiGradingCapability),
}

impl std::fmt::Debug for RehearsalPrivateExecutionCapabilityV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RehearsalPrivateExecutionCapabilityV1([REDACTED])")
    }
}

/// Versioned sealed delivery authority.  This internal value never derives
/// serde; persistence must use a canonical codec owned by the sealed facade.
#[allow(dead_code)]
pub struct RehearsalDeliveryExecutionContractV1 {
    issue: RehearsalDeliveryIssueDescriptorV1,
    private_execution: RehearsalPrivateExecutionCapabilityV1,
    contract_digest: RehearsalOperationDigest,
}

#[allow(dead_code)]
impl RehearsalDeliveryExecutionContractV1 {
    pub(crate) fn new(
        issue: RehearsalDeliveryIssueDescriptorV1,
        private_execution: RehearsalPrivateExecutionCapabilityV1,
        contract_digest: RehearsalOperationDigest,
    ) -> Self {
        Self {
            issue,
            private_execution,
            contract_digest,
        }
    }

    pub fn issue_descriptor(&self) -> &RehearsalDeliveryIssueDescriptorV1 {
        &self.issue
    }
    pub(crate) fn private_execution(&self) -> RehearsalPrivateExecutionCapabilityV1 {
        self.private_execution
    }
    pub(crate) fn contract_digest(&self) -> RehearsalOperationDigest {
        self.contract_digest
    }
}

impl std::fmt::Debug for RehearsalDeliveryExecutionContractV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RehearsalDeliveryExecutionContractV1([REDACTED])")
    }
}

/// A server-only operation identity.  It is never serialized into a screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalOperationId(Uuid);

impl RehearsalOperationId {
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// A transaction-bound broker nonce.  It cannot be used as browser state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalOperationNonce(Uuid);

impl RehearsalOperationNonce {
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// A fixed SHA-256 commitment supplied by the route/execution coordinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalOperationDigest([u8; 32]);

impl RehearsalOperationDigest {
    pub const fn from_bytes(value: [u8; 32]) -> Self {
        Self(value)
    }
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// A bounded, object-shaped projection that is already safe for the browser.
/// It is intentionally non-serializable as a wrapper: only the HTTP DTO layer
/// may turn its verified value into transport bytes.
#[derive(Clone, PartialEq)]
pub struct RehearsalSafeProjection(Value);

impl RehearsalSafeProjection {
    pub fn new(value: Value, limit: usize) -> Result<Self, StoreError> {
        if !value.is_object()
            || serde_json::to_vec(&value).map_or(true, |bytes| bytes.len() > limit)
        {
            return Err(StoreError::InvalidRecord(
                "invalid bounded rehearsal projection".into(),
            ));
        }
        Ok(Self(value))
    }
    pub fn as_value(&self) -> &Value {
        &self.0
    }
    pub fn into_value(self) -> Value {
        self.0
    }
}

// PostgreSQL consumes these fields; Memory intentionally fails closed until
// its atomic discard protocol is installed, so its production-only build does
// not otherwise read the command payload.
#[derive(Clone)]
#[cfg_attr(not(feature = "postgres"), allow(dead_code))]
pub struct RehearsalDiscardOperationCommand {
    pub locator: RehearsalLocator,
    pub idempotency_key: RehearsalIdempotencyKey,
    pub request_fingerprint: RehearsalOperationDigest,
    pub response: RehearsalSafeProjection,
    pub response_digest: RehearsalOperationDigest,
}

#[derive(Clone, PartialEq)]
pub enum RehearsalIdempotentProjectionResult {
    Applied(RehearsalSafeProjection),
    Replay(RehearsalSafeProjection),
    Conflict,
}

#[derive(Clone)]
pub struct RehearsalDeliveryRequest {
    pub locator: RehearsalLocator,
    pub idempotency_key: RehearsalIdempotencyKey,
    pub request_fingerprint: RehearsalOperationDigest,
}

pub enum RehearsalDeliveryClaimResult {
    /// A server-owned generation is ready for the coordinator to dispatch.
    /// This includes a recovered prepared generation and a fresh generation
    /// reclaimed after a proven pre-dispatch abandonment.
    Prepared {
        prepared: PreparedRehearsalDelivery,
    },
    Pending {
        dispatched: DispatchedRehearsalDelivery,
    },
    Replay(question_model::RehearsalActiveScreenV1),
    /// The latest issued generation crossed its server-owned expiry boundary.
    /// A caller must use the explicit retry operation; Continue never advances
    /// past an expired item.
    Expired,
    RunTimeExhausted {
        deadline: question_model::ActivityTimestamp,
    },
    Conflict,
}

pub struct RehearsalDeliveryCompletionCommand {
    pub dispatched: DispatchedRehearsalDelivery,
    pub screen: question_model::RehearsalActiveScreenV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalDeliveryPreDispatchAbandonReason {
    LocalPreparationFailed,
    NativeBackendAdmissionRejected,
    TrustedRendererAdmissionRejected,
}

pub struct PreparedRehearsalDelivery {
    locator: RehearsalLocator,
    operation: RehearsalOperationId,
    descriptor: RehearsalDeliveryExecutionDescriptorV1,
    capability: RehearsalDeliveryExecutionCapability,
}

/// Opaque proof that a prepared operation was created by the Store.  It is
/// intentionally useful only as part of the non-serializable prepared handle.
#[derive(Clone, Copy)]
pub struct RehearsalDeliveryExecutionCapability(());

impl std::fmt::Debug for RehearsalDeliveryExecutionCapability {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RehearsalDeliveryExecutionCapability([OPAQUE])")
    }
}

impl PreparedRehearsalDelivery {
    pub(crate) fn mint(
        locator: RehearsalLocator,
        operation: RehearsalOperationId,
        descriptor: RehearsalDeliveryExecutionDescriptorV1,
    ) -> Self {
        Self {
            locator,
            operation,
            descriptor,
            capability: RehearsalDeliveryExecutionCapability(()),
        }
    }
    /// Answer-free immutable selection data for trusted route orchestration.
    pub fn descriptor(&self) -> &RehearsalDeliveryExecutionDescriptorV1 {
        &self.descriptor
    }
    /// Opaque Store-minted capability paired with this prepared generation.
    pub fn capability(&self) -> &RehearsalDeliveryExecutionCapability {
        &self.capability
    }
    pub(crate) fn locator(&self) -> RehearsalLocator {
        self.locator
    }
    pub(crate) fn operation(&self) -> RehearsalOperationId {
        self.operation
    }
}
pub struct DispatchedRehearsalDelivery {
    locator: RehearsalLocator,
    operation: RehearsalOperationId,
}

pub enum RehearsalDeliveryDispatchResult {
    Dispatched {
        dispatched: DispatchedRehearsalDelivery,
    },
    RunTimeExhausted {
        deadline: question_model::ActivityTimestamp,
    },
}

/// Non-serializable, grader-facade-only material for an already committed
/// delivery dispatch.  The ordinary store never returns this value; its
/// private family contract is intentionally unavailable to browser and route
/// persistence capabilities.
pub struct SealedRehearsalDeliveryExecution {
    issued_snapshot: IssuedQuestionSnapshotV1,
    private_execution: PrefetchedPrivateExecutionV1,
    issued_artifact: crate::RehearsalIssuedExecutionArtifactV1,
}

/// One trusted grader invocation assembled only from a committed issued
/// artifact. It is deliberately non-serializable and non-cloneable; it is not
/// a route DTO and contains no artifact bytes or rendered-id mapping.
pub struct SealedRehearsalGradingParts {
    family: crate::RehearsalIssuedExecutionFamilyV1,
    issued_snapshot: IssuedQuestionSnapshotV1,
    envelope: question_model::QuestionEnvelope,
    parameter_hash: String,
    provenance: question_model::AttemptProvenance,
    private_execution: PrefetchedPrivateExecutionV1,
    response: question_model::StudentResponse,
}

impl SealedRehearsalGradingParts {
    pub fn family(&self) -> crate::RehearsalIssuedExecutionFamilyV1 {
        self.family
    }
    pub fn issued_snapshot(&self) -> &IssuedQuestionSnapshotV1 {
        &self.issued_snapshot
    }
    pub fn envelope(&self) -> &question_model::QuestionEnvelope {
        &self.envelope
    }
    pub fn deterministic_seed(&self) -> question_model::generation::Seed {
        self.envelope.seed
    }
    pub fn parameter_hash(&self) -> &str {
        &self.parameter_hash
    }
    pub fn provenance(&self) -> &question_model::AttemptProvenance {
        &self.provenance
    }
    pub fn private_execution(&self) -> &PrefetchedPrivateExecutionV1 {
        &self.private_execution
    }
    pub fn response(&self) -> &question_model::StudentResponse {
        &self.response
    }
    pub fn into_parts(
        self,
    ) -> (
        crate::RehearsalIssuedExecutionFamilyV1,
        IssuedQuestionSnapshotV1,
        question_model::QuestionEnvelope,
        String,
        question_model::AttemptProvenance,
        PrefetchedPrivateExecutionV1,
        question_model::StudentResponse,
    ) {
        (
            self.family,
            self.issued_snapshot,
            self.envelope,
            self.parameter_hash,
            self.provenance,
            self.private_execution,
            self.response,
        )
    }
}

impl std::fmt::Debug for SealedRehearsalGradingParts {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SealedRehearsalGradingParts([REDACTED])")
    }
}

impl SealedRehearsalDeliveryExecution {
    pub(crate) fn with_issued_artifact(
        issued_snapshot: IssuedQuestionSnapshotV1,
        private_execution: PrefetchedPrivateExecutionV1,
        issued_artifact: crate::RehearsalIssuedExecutionArtifactV1,
    ) -> Self {
        Self {
            issued_snapshot,
            private_execution,
            issued_artifact,
        }
    }

    /// The public half is answer-free and may be inspected by the trusted
    /// execution coordinator before it invokes its family adapter.
    pub fn issued_snapshot(&self) -> &IssuedQuestionSnapshotV1 {
        &self.issued_snapshot
    }

    /// True only when this value was hydrated from a canonical, committed
    /// per-generation issued-execution artifact.
    pub fn has_committed_artifact(&self) -> bool {
        !self.issued_artifact.bytes().is_empty()
    }
    /// Derives the only browser-safe rehearsal screen permitted by this
    /// immutable issued artifact.
    pub fn active_screen(&self) -> Result<question_model::RehearsalActiveScreenV1, StoreError> {
        self.issued_artifact.active_screen()
    }
    /// Consumes this sealed execution into exactly the checked inputs a
    /// deterministic family adapter needs. The response is translated through
    /// the artifact's reproduced presentation before any grader sees it.
    pub fn into_grading_parts(
        self,
        rendered_response: &question_model::StudentResponse,
    ) -> Result<SealedRehearsalGradingParts, StoreError> {
        let fields = self.issued_artifact.grading_fields(rendered_response)?;
        Ok(SealedRehearsalGradingParts {
            family: fields.family,
            issued_snapshot: self.issued_snapshot,
            envelope: fields.envelope,
            parameter_hash: fields.parameter_hash,
            provenance: fields.provenance,
            private_execution: self.private_execution,
            response: fields.response,
        })
    }
}

impl std::fmt::Debug for SealedRehearsalDeliveryExecution {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SealedRehearsalDeliveryExecution([REDACTED])")
    }
}
impl DispatchedRehearsalDelivery {
    pub(crate) fn mint(locator: RehearsalLocator, operation: RehearsalOperationId) -> Self {
        Self { locator, operation }
    }
    pub(crate) fn locator(&self) -> RehearsalLocator {
        self.locator
    }
    pub(crate) fn operation(&self) -> RehearsalOperationId {
        self.operation
    }
}

#[async_trait]
pub trait RehearsalOperationStore: Send + Sync {
    async fn discard_rehearsal_idempotent(
        &self,
        context: TenantContext,
        command: RehearsalDiscardOperationCommand,
    ) -> Result<RehearsalIdempotentProjectionResult, StoreError>;
    async fn claim_rehearsal_delivery(
        &self,
        context: TenantContext,
        request: RehearsalDeliveryRequest,
    ) -> Result<RehearsalDeliveryClaimResult, StoreError>;
    async fn mark_rehearsal_delivery_dispatched(
        &self,
        context: TenantContext,
        prepared: PreparedRehearsalDelivery,
    ) -> Result<RehearsalDeliveryDispatchResult, StoreError>;
    async fn complete_rehearsal_delivery(
        &self,
        context: TenantContext,
        command: RehearsalDeliveryCompletionCommand,
    ) -> Result<question_model::RehearsalActiveScreenV1, StoreError>;
}

#[async_trait]
pub trait RehearsalDeliveryPreDispatchCompensationStore: Send + Sync {
    async fn abandon_rehearsal_delivery_before_dispatch(
        &self,
        context: TenantContext,
        prepared: PreparedRehearsalDelivery,
        reason: RehearsalDeliveryPreDispatchAbandonReason,
    ) -> Result<(), StoreError>;
}

/// A separate least-privilege facade for post-dispatch execution material.
/// Implementations must verify the committed `issueDispatched` generation and
/// its checksums before returning a sealed value.  This is deliberately not a
/// supertrait of `RehearsalOperationStore`.
#[async_trait]
pub trait SealedRehearsalDeliveryExecutionStore: Send + Sync {
    /// Atomically returns committed issued material or the one sealed work
    /// item allowed to create it. Ordinary route Stores cannot call this.
    async fn prepare_or_resume_issued_execution(
        &self,
        _context: TenantContext,
        _dispatched: &DispatchedRehearsalDelivery,
    ) -> Result<crate::SealedRehearsalDeliveryIssuePreparation, StoreError> {
        Err(StoreError::Unavailable(
            "sealed issued rehearsal execution is not installed".into(),
        ))
    }
    /// Commits exact, canonical executor output. Repeating the same bytes is
    /// a replay; a different artifact for one generation is a conflict.
    async fn commit_issued_execution(
        &self,
        _context: TenantContext,
        _work: SealedRehearsalDeliveryIssueWork,
        _artifact: RehearsalIssuedExecutionArtifactV1,
    ) -> Result<SealedRehearsalDeliveryExecution, StoreError> {
        Err(StoreError::Unavailable(
            "sealed issued rehearsal commit is not installed".into(),
        ))
    }
    async fn prepare_sealed_rehearsal_delivery_execution(
        &self,
        context: TenantContext,
        dispatched: &DispatchedRehearsalDelivery,
    ) -> Result<SealedRehearsalDeliveryExecution, StoreError>;
}
