//! Append-only, root-bound rehearsal submission-claim transitions.

use std::collections::BTreeSet;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RehearsalClaimGeneration(std::num::NonZeroU32);

impl RehearsalClaimGeneration {
    pub const fn first() -> Self {
        Self(std::num::NonZeroU32::MIN)
    }
    pub const fn value(self) -> u32 {
        self.0.get()
    }
    pub fn from_persisted(value: u32) -> Option<Self> {
        std::num::NonZeroU32::new(value).map(Self)
    }
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Closed server-private reasons for a definite failure before a grader sees input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalPreDispatchAbandonReason {
    LocalPreparationFailed,
    NativeBackendAdmissionRejected,
    TrustedRendererAdmissionRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalSubmissionClaimPhase {
    Prepared,
    GradingDispatched,
    Completed,
    AbandonedBeforeDispatch,
    RevokedStaleRevision,
    RevokedTerminalLifecycle,
    RevokedSourceContextRemoved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ClaimRootBinding {
    pub(super) rehearsal: question_model::RehearsalRunId,
    pub(super) claim: question_model::RehearsalSubmissionClaimId,
    pub(super) fingerprint: RehearsalSubmissionRequestFingerprint,
}

/// The exact private input committed by an idempotent rehearsal submission claim.
///
/// Live route claims retain the browser's authenticated rendered response until
/// the sealed grading boundary translates it.  Durable requests exist only for
/// generic test-support and internal claims that already own durable IDs.
/// This is deliberately non-serde: persistence uses the closed codec in
/// [`super::persistence`].
pub enum RehearsalClaimSubmissionInput {
    Rendered(question_model::ValidatedRehearsalRenderedSubmissionV1),
    Durable(RehearsalValidatedSubmissionRequest),
}

impl PartialEq for RehearsalClaimSubmissionInput {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Rendered(left), Self::Rendered(right)) => {
                left.response() == right.response()
                    && left.presentation_commitment() == right.presentation_commitment()
            }
            (Self::Durable(left), Self::Durable(right)) => left == right,
            _ => false,
        }
    }
}

impl From<RehearsalValidatedSubmissionRequest> for RehearsalClaimSubmissionInput {
    fn from(value: RehearsalValidatedSubmissionRequest) -> Self {
        Self::Durable(value)
    }
}

impl RehearsalClaimSubmissionInput {
    pub fn rendered(value: question_model::ValidatedRehearsalRenderedSubmissionV1) -> Self {
        Self::Rendered(value)
    }

    pub fn durable(value: RehearsalValidatedSubmissionRequest) -> Self {
        Self::Durable(value)
    }

    /// The exact original response retained as private claim evidence.
    pub fn original_response(&self) -> &question_model::StudentResponse {
        match self {
            Self::Rendered(value) => value.response(),
            Self::Durable(value) => value.submitted_response(),
        }
    }

    pub fn presentation_commitment(&self) -> Option<question_model::RehearsalPresentationDigestV1> {
        match self {
            Self::Rendered(value) => Some(value.presentation_commitment()),
            Self::Durable(_) => None,
        }
    }

    pub fn durable_request(&self) -> Option<&RehearsalValidatedSubmissionRequest> {
        match self {
            Self::Rendered(_) => None,
            Self::Durable(value) => Some(value),
        }
    }

    pub(super) fn validate_for_completion(
        &self,
        durable_request: &RehearsalValidatedSubmissionRequest,
        frozen: &RehearsalFrozenItemEvidence,
    ) -> Result<(), question_model::RehearsalEvidenceValidationError> {
        durable_request.validate_frozen_attempt(frozen)?;
        match self {
            Self::Rendered(_) => Ok(()),
            Self::Durable(value) if value == durable_request => Ok(()),
            Self::Durable(_) => {
                Err(question_model::RehearsalEvidenceValidationError::ResponseDefinitionMismatch)
            }
        }
    }
}

/// Raw private material decoded from or retained for the locked claim-root row.
///
/// This is deliberately not a usable aggregate capability: it cannot restore
/// transitions, hydrate history, or produce a grading handle.  Persistence may
/// move this value across a Store boundary, but it must be verified against the
/// locked aggregate genesis and exact frozen attempt before domain operations
/// can use it.
#[derive(PartialEq)]
pub struct RehearsalPersistedClaimRoot {
    pub(super) binding: ClaimRootBinding,
    submission_input: RehearsalClaimSubmissionInput,
}

impl RehearsalPersistedClaimRoot {
    /// Decodes a private claim-root row. This does not grant any operational
    /// capability; call [`RehearsalClaimRoot::verify_persisted`] under the
    /// aggregate lock before restoring a claim history.
    pub fn from_persisted(
        rehearsal: question_model::RehearsalRunId,
        claim: question_model::RehearsalSubmissionClaimId,
        fingerprint: RehearsalSubmissionRequestFingerprint,
        submission_input: impl Into<RehearsalClaimSubmissionInput>,
    ) -> Self {
        Self {
            binding: ClaimRootBinding {
                rehearsal,
                claim,
                fingerprint,
            },
            submission_input: submission_input.into(),
        }
    }
    pub const fn rehearsal(&self) -> question_model::RehearsalRunId {
        self.binding.rehearsal
    }
    pub const fn claim(&self) -> question_model::RehearsalSubmissionClaimId {
        self.binding.claim
    }
    pub const fn fingerprint(&self) -> RehearsalSubmissionRequestFingerprint {
        self.binding.fingerprint
    }
    pub const fn submission_input(&self) -> &RehearsalClaimSubmissionInput {
        &self.submission_input
    }
}

/// Verified private root bound to one locked aggregate and frozen attempt.
///
/// The only constructor recomputes the sealed request fingerprint. This type
/// alone can restore root-scoped transitions and therefore form grading handles.
#[derive(PartialEq)]
pub struct RehearsalClaimRoot {
    pub(super) binding: ClaimRootBinding,
    submission_input: RehearsalClaimSubmissionInput,
    verified_attempt: RehearsalFrozenAttemptCommitment,
}

impl RehearsalClaimRoot {
    /// Verifies persistence material before it becomes operational.
    ///
    /// The caller supplies only Store-locked aggregate genesis and the exact
    /// immutable frozen attempt. A stored fingerprint is never trusted: this
    /// recomputes it from the sealed request and rejects any mismatch before a
    /// Prepared or Dispatched handle can exist.
    pub fn verify_persisted(
        context: RehearsalGenesisContext,
        frozen: &RehearsalFrozenItemEvidence,
        persisted: RehearsalPersistedClaimRoot,
    ) -> Result<Self, RehearsalClaimRootVerificationError> {
        if persisted.binding.rehearsal != context.rehearsal {
            return Err(RehearsalClaimRootVerificationError::ContextRunMismatch);
        }
        let fingerprint = rehearsal_claim_submission_input_fingerprint(
            context,
            frozen,
            &persisted.submission_input,
        )
        .map_err(RehearsalClaimRootVerificationError::SubmissionInputMismatch)?;
        if fingerprint != persisted.binding.fingerprint {
            return Err(RehearsalClaimRootVerificationError::FingerprintMismatch);
        }
        Ok(Self {
            binding: persisted.binding,
            submission_input: persisted.submission_input,
            verified_attempt: RehearsalFrozenAttemptCommitment::from_frozen(frozen),
        })
    }

    /// Returns persistence material after this verified root is no longer
    /// needed as an operational capability.
    pub fn into_persisted(self) -> RehearsalPersistedClaimRoot {
        RehearsalPersistedClaimRoot {
            binding: self.binding,
            submission_input: self.submission_input,
        }
    }
    pub const fn rehearsal(&self) -> question_model::RehearsalRunId {
        self.binding.rehearsal
    }
    pub const fn claim(&self) -> question_model::RehearsalSubmissionClaimId {
        self.binding.claim
    }
    pub const fn fingerprint(&self) -> RehearsalSubmissionRequestFingerprint {
        self.binding.fingerprint
    }
    pub const fn submission_input(&self) -> &RehearsalClaimSubmissionInput {
        &self.submission_input
    }

    /// Confirms that restored accepted evidence is tied to the exact frozen
    /// attempt authenticated while this root became operational.
    pub(super) fn verified_attempt_matches(&self, frozen: &RehearsalFrozenItemEvidence) -> bool {
        self.verified_attempt.matches_frozen(frozen)
    }

    /// Restores one transition that is physically scoped by this exact root.
    #[allow(clippy::too_many_arguments)]
    pub const fn restore_transition(
        &self,
        sequence: u64,
        operation: question_model::RehearsalGradeOperationId,
        generation: RehearsalClaimGeneration,
        phase: RehearsalSubmissionClaimPhase,
        recorded_at: question_model::ActivityTimestamp,
        abandon_reason: Option<RehearsalPreDispatchAbandonReason>,
        completion_material: Option<RehearsalClaimCompletionMaterial>,
    ) -> RehearsalClaimTransitionEvent {
        RehearsalClaimTransitionEvent {
            sequence,
            binding: self.binding,
            operation,
            generation,
            phase,
            recorded_at,
            abandon_reason,
            completion_material,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalClaimCompletionMaterial {
    accepted_evidence_sequence: u64,
    accepted_evidence_digest: RehearsalEvidenceDigest,
    receipt_digest: RehearsalEvidenceDigest,
}
impl RehearsalClaimCompletionMaterial {
    /// Restores the immutable material named by a completed transition.
    /// Hydration still requires a verified proof that independently recomputes
    /// all three values from the aggregate-bound evidence chain.
    pub fn from_persisted(
        accepted_evidence_sequence: u64,
        accepted_evidence_digest: RehearsalEvidenceDigest,
        receipt_digest: RehearsalEvidenceDigest,
    ) -> Option<Self> {
        (accepted_evidence_sequence != 0).then_some(Self {
            accepted_evidence_sequence,
            accepted_evidence_digest,
            receipt_digest,
        })
    }
    pub const fn accepted_evidence_sequence(self) -> u64 {
        self.accepted_evidence_sequence
    }
    pub const fn receipt_digest(self) -> RehearsalEvidenceDigest {
        self.receipt_digest
    }
    pub const fn accepted_evidence_digest(self) -> RehearsalEvidenceDigest {
        self.accepted_evidence_digest
    }
}

/// One append-only persistence transition. It has no replacement API.
///
/// `sequence` is the sole causal authority for claim history. `recorded_at` is
/// integrity-hashed observational audit metadata and may move backward when a
/// host clock is corrected; it is never used to establish phase ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalClaimTransitionEvent {
    sequence: u64,
    binding: ClaimRootBinding,
    operation: question_model::RehearsalGradeOperationId,
    generation: RehearsalClaimGeneration,
    phase: RehearsalSubmissionClaimPhase,
    recorded_at: question_model::ActivityTimestamp,
    abandon_reason: Option<RehearsalPreDispatchAbandonReason>,
    completion_material: Option<RehearsalClaimCompletionMaterial>,
}
impl RehearsalClaimTransitionEvent {
    pub const fn sequence(self) -> u64 {
        self.sequence
    }
    pub const fn rehearsal(self) -> question_model::RehearsalRunId {
        self.binding.rehearsal
    }
    pub const fn claim(self) -> question_model::RehearsalSubmissionClaimId {
        self.binding.claim
    }
    pub const fn fingerprint(self) -> RehearsalSubmissionRequestFingerprint {
        self.binding.fingerprint
    }
    pub const fn operation(self) -> question_model::RehearsalGradeOperationId {
        self.operation
    }
    pub const fn generation(self) -> RehearsalClaimGeneration {
        self.generation
    }
    pub const fn phase(self) -> RehearsalSubmissionClaimPhase {
        self.phase
    }
    pub const fn recorded_at(self) -> question_model::ActivityTimestamp {
        self.recorded_at
    }
    pub const fn abandon_reason(self) -> Option<RehearsalPreDispatchAbandonReason> {
        self.abandon_reason
    }
    pub const fn completion_material(self) -> Option<RehearsalClaimCompletionMaterial> {
        self.completion_material
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClaimIdentity {
    binding: ClaimRootBinding,
    operation: question_model::RehearsalGradeOperationId,
    generation: RehearsalClaimGeneration,
}

/// A hydrated phase-specific handle. It is intentionally neither `Clone` nor `Copy`.
#[derive(Debug, PartialEq, Eq)]
pub struct PreparedClaimHandle(ClaimIdentity);
/// A hydrated phase-specific handle. It is intentionally neither `Clone` nor `Copy`.
#[derive(Debug, PartialEq, Eq)]
pub struct DispatchedClaimHandle(ClaimIdentity);

macro_rules! handle_accessors {
    ($name:ident) => {
        impl $name {
            pub const fn rehearsal(&self) -> question_model::RehearsalRunId {
                self.0.binding.rehearsal
            }
            pub const fn claim(&self) -> question_model::RehearsalSubmissionClaimId {
                self.0.binding.claim
            }
            pub const fn fingerprint(&self) -> RehearsalSubmissionRequestFingerprint {
                self.0.binding.fingerprint
            }
            pub const fn operation(&self) -> question_model::RehearsalGradeOperationId {
                self.0.operation
            }
            pub const fn generation(&self) -> RehearsalClaimGeneration {
                self.0.generation
            }
        }
    };
}
handle_accessors!(PreparedClaimHandle);
handle_accessors!(DispatchedClaimHandle);

#[derive(Debug, PartialEq, Eq)]
pub struct RehearsalPreDispatchAbandonment {
    identity: ClaimIdentity,
    reason: RehearsalPreDispatchAbandonReason,
}
impl RehearsalPreDispatchAbandonment {
    pub const fn rehearsal(&self) -> question_model::RehearsalRunId {
        self.identity.binding.rehearsal
    }
    pub const fn claim(&self) -> question_model::RehearsalSubmissionClaimId {
        self.identity.binding.claim
    }
    pub const fn operation(&self) -> question_model::RehearsalGradeOperationId {
        self.identity.operation
    }
    pub const fn generation(&self) -> RehearsalClaimGeneration {
        self.identity.generation
    }
    pub const fn reason(&self) -> RehearsalPreDispatchAbandonReason {
        self.reason
    }
}

pub fn mark_rehearsal_submission_dispatched(
    prepared: PreparedClaimHandle,
) -> DispatchedClaimHandle {
    DispatchedClaimHandle(prepared.0)
}

/// Restores a dispatched handle only after a sealed persistence capability has
/// verified the claim root, immutable delivery binding, and append-only phase.
/// It is intentionally not a route constructor: callers must keep all values
/// server-private and perform that verification in the same database call.
pub fn restore_sealed_dispatched_claim_handle(
    rehearsal: question_model::RehearsalRunId,
    claim: question_model::RehearsalSubmissionClaimId,
    fingerprint: RehearsalSubmissionRequestFingerprint,
    operation: question_model::RehearsalGradeOperationId,
    generation: RehearsalClaimGeneration,
) -> DispatchedClaimHandle {
    DispatchedClaimHandle(ClaimIdentity {
        binding: ClaimRootBinding {
            rehearsal,
            claim,
            fingerprint,
        },
        operation,
        generation,
    })
}
pub fn abandon_rehearsal_submission_before_dispatch(
    prepared: PreparedClaimHandle,
    reason: RehearsalPreDispatchAbandonReason,
) -> RehearsalPreDispatchAbandonment {
    RehearsalPreDispatchAbandonment {
        identity: prepared.0,
        reason,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalSubmissionClaimState {
    Prepared,
    GradingDispatched,
    Completed,
    AbandonedBeforeDispatch,
    RevokedStaleRevision,
    RevokedTerminalLifecycle,
    RevokedSourceContextRemoved,
}
impl From<RehearsalSubmissionClaimPhase> for RehearsalSubmissionClaimState {
    fn from(value: RehearsalSubmissionClaimPhase) -> Self {
        match value {
            RehearsalSubmissionClaimPhase::Prepared => Self::Prepared,
            RehearsalSubmissionClaimPhase::GradingDispatched => Self::GradingDispatched,
            RehearsalSubmissionClaimPhase::Completed => Self::Completed,
            RehearsalSubmissionClaimPhase::AbandonedBeforeDispatch => Self::AbandonedBeforeDispatch,
            RehearsalSubmissionClaimPhase::RevokedStaleRevision => Self::RevokedStaleRevision,
            RehearsalSubmissionClaimPhase::RevokedTerminalLifecycle => {
                Self::RevokedTerminalLifecycle
            }
            RehearsalSubmissionClaimPhase::RevokedSourceContextRemoved => {
                Self::RevokedSourceContextRemoved
            }
        }
    }
}

/// Opaque witness returned only after aggregate-bound evidence verification.
#[derive(Debug, PartialEq)]
pub struct VerifiedRehearsalClaimCompletionProof {
    binding: ClaimRootBinding,
    material: RehearsalClaimCompletionMaterial,
    receipt: question_model::RehearsalPublicOutcome,
}
impl VerifiedRehearsalClaimCompletionProof {
    pub const fn completion_material(&self) -> RehearsalClaimCompletionMaterial {
        self.material
    }
    pub fn replay_receipt(&self) -> question_model::RehearsalPublicOutcome {
        self.receipt.clone()
    }
}

/// A fully hydrated claim history. It is the sole source of phase-specific handles.
#[derive(Debug, PartialEq)]
pub struct RehearsalSubmissionClaimSnapshot {
    binding: ClaimRootBinding,
    identity: ClaimIdentity,
    state: RehearsalSubmissionClaimState,
    used_operations: BTreeSet<question_model::RehearsalGradeOperationId>,
    completion_proof: Option<VerifiedRehearsalClaimCompletionProof>,
}
impl RehearsalSubmissionClaimSnapshot {
    pub const fn fingerprint(&self) -> RehearsalSubmissionRequestFingerprint {
        self.binding.fingerprint
    }
    pub const fn rehearsal(&self) -> question_model::RehearsalRunId {
        self.binding.rehearsal
    }
    pub const fn claim(&self) -> question_model::RehearsalSubmissionClaimId {
        self.binding.claim
    }
    pub const fn operation(&self) -> question_model::RehearsalGradeOperationId {
        self.identity.operation
    }
    pub const fn generation(&self) -> RehearsalClaimGeneration {
        self.identity.generation
    }
    pub const fn state(&self) -> RehearsalSubmissionClaimState {
        self.state
    }
    pub fn into_prepared_handle(self) -> Result<PreparedClaimHandle, RehearsalClaimHandleError> {
        (self.state == RehearsalSubmissionClaimState::Prepared)
            .then_some(PreparedClaimHandle(self.identity))
            .ok_or(RehearsalClaimHandleError::NotPrepared)
    }
    pub fn into_dispatched_handle(
        self,
    ) -> Result<DispatchedClaimHandle, RehearsalClaimHandleError> {
        (self.state == RehearsalSubmissionClaimState::GradingDispatched)
            .then_some(DispatchedClaimHandle(self.identity))
            .ok_or(RehearsalClaimHandleError::NotDispatched)
    }
    fn completion_proof(&self) -> Option<&VerifiedRehearsalClaimCompletionProof> {
        self.completion_proof.as_ref()
    }
    fn has_used_operation(&self, operation: question_model::RehearsalGradeOperationId) -> bool {
        self.used_operations.contains(&operation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalClaimHandleError {
    NotPrepared,
    NotDispatched,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalClaimReclaimError {
    ReusedOperation,
    GenerationExhausted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalClaimHydrationError {
    EmptyHistory,
    RootMismatch,
    SequenceNotContiguous,
    FirstEventNotPreparedGenerationOne,
    SameGenerationOperationMismatch,
    GenerationNotNext,
    ReusedOperation,
    IllegalTransition,
    PhaseMaterialMismatch,
    MissingCompletionProof,
    CompletionProofMismatch,
    UnexpectedCompletionProof,
}
/// Verification failure while converting persistence-only root material into
/// the capability required for claim restoration and grading preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalClaimRootVerificationError {
    ContextRunMismatch,
    SubmissionInputMismatch(question_model::RehearsalEvidenceValidationError),
    FingerprintMismatch,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalClaimCompletionProofError {
    ContextRunMismatch,
    EvidenceIntegrity(RehearsalIntegrityError),
    FingerprintMismatch,
    FrozenAttemptMissing,
    DuplicateFrozenAttempt,
    FrozenAttemptNotBeforeAcceptedSubmission,
    AcceptedSubmissionMissing,
    DuplicateAcceptedSubmission,
    AcceptedSubmissionRequestMismatch,
}

/// Verifies actual immutable evidence before a completed transition can name it.
pub fn verify_rehearsal_claim_completion_proof(
    context: RehearsalGenesisContext,
    expected_head: RehearsalEvidenceHead,
    root: &RehearsalClaimRoot,
    entries: &[RehearsalEvidenceChainEntry],
) -> Result<VerifiedRehearsalClaimCompletionProof, RehearsalClaimCompletionProofError> {
    if context.rehearsal != root.rehearsal() {
        return Err(RehearsalClaimCompletionProofError::ContextRunMismatch);
    }
    verify_evidence_chain(context, expected_head, entries)
        .map_err(RehearsalClaimCompletionProofError::EvidenceIntegrity)?;
    let accepted: Vec<(
        &RehearsalEvidenceChainEntry,
        &RehearsalValidatedSubmissionEvidence,
    )> = entries
        .iter()
        .filter_map(|entry| match &entry.payload {
            RehearsalEvidencePayload::AcceptedSubmission(value)
                if value.claim_binding_matches(root.binding) =>
            {
                Some((entry, value))
            }
            _ => None,
        })
        .collect();
    let Some((entry, value)) = accepted.first().copied() else {
        return Err(RehearsalClaimCompletionProofError::AcceptedSubmissionMissing);
    };
    if accepted.len() != 1 {
        return Err(RehearsalClaimCompletionProofError::DuplicateAcceptedSubmission);
    }
    let frozen: Vec<(&RehearsalEvidenceChainEntry, &RehearsalFrozenItemEvidence)> = entries
        .iter()
        .filter_map(|entry| match &entry.payload {
            RehearsalEvidencePayload::FrozenItem(item) if item.attempt == value.attempt() => {
                Some((entry, item))
            }
            _ => None,
        })
        .collect();
    let frozen_count = frozen.len();
    let Some((frozen_entry, frozen)) = frozen.first().copied() else {
        return Err(RehearsalClaimCompletionProofError::FrozenAttemptMissing);
    };
    if frozen_count != 1 {
        return Err(RehearsalClaimCompletionProofError::DuplicateFrozenAttempt);
    }
    let fingerprint =
        rehearsal_claim_submission_input_fingerprint(context, frozen, root.submission_input())
            .map_err(|_| RehearsalClaimCompletionProofError::FingerprintMismatch)?;
    if fingerprint != root.fingerprint() {
        return Err(RehearsalClaimCompletionProofError::FingerprintMismatch);
    }
    if frozen_entry.record.sequence >= entry.record.sequence {
        return Err(RehearsalClaimCompletionProofError::FrozenAttemptNotBeforeAcceptedSubmission);
    }
    if value.attempt() != frozen.attempt
        || value.submitted_response() != root.submission_input().original_response()
    {
        return Err(RehearsalClaimCompletionProofError::AcceptedSubmissionRequestMismatch);
    }
    let receipt = value.browser_safe_receipt();
    Ok(VerifiedRehearsalClaimCompletionProof {
        binding: root.binding,
        material: RehearsalClaimCompletionMaterial {
            accepted_evidence_sequence: u64::from(entry.record.sequence),
            accepted_evidence_digest: entry.record.digest,
            receipt_digest: persistence::persisted_rehearsal_receipt_digest(&receipt),
        },
        receipt,
    })
}

/// Fully validates an arbitrary ordered append-only root-bound claim history.
pub fn hydrate_claim_history(
    root: &RehearsalClaimRoot,
    events: &[RehearsalClaimTransitionEvent],
    completion_proof: Option<VerifiedRehearsalClaimCompletionProof>,
) -> Result<RehearsalSubmissionClaimSnapshot, RehearsalClaimHydrationError> {
    let Some(first) = events.first().copied() else {
        return Err(RehearsalClaimHydrationError::EmptyHistory);
    };
    if first.binding != root.binding {
        return Err(RehearsalClaimHydrationError::RootMismatch);
    }
    if first.sequence != 1
        || first.generation != RehearsalClaimGeneration::first()
        || first.phase != RehearsalSubmissionClaimPhase::Prepared
    {
        return Err(RehearsalClaimHydrationError::FirstEventNotPreparedGenerationOne);
    }
    validate_phase_material(first)?;
    let mut used_operations = BTreeSet::from([first.operation]);
    let mut prior = first;
    for (index, event) in events.iter().copied().enumerate().skip(1) {
        if event.sequence
            != u64::try_from(index + 1)
                .map_err(|_| RehearsalClaimHydrationError::SequenceNotContiguous)?
        {
            return Err(RehearsalClaimHydrationError::SequenceNotContiguous);
        }
        if event.binding != root.binding {
            return Err(RehearsalClaimHydrationError::RootMismatch);
        }
        validate_transition(prior, event, &mut used_operations)?;
        prior = event;
    }
    let completion_proof = match prior.phase {
        RehearsalSubmissionClaimPhase::Completed => {
            let Some(proof) = completion_proof else {
                return Err(RehearsalClaimHydrationError::MissingCompletionProof);
            };
            if proof.binding != root.binding || prior.completion_material != Some(proof.material) {
                return Err(RehearsalClaimHydrationError::CompletionProofMismatch);
            }
            Some(proof)
        }
        _ if completion_proof.is_some() => {
            return Err(RehearsalClaimHydrationError::UnexpectedCompletionProof);
        }
        _ => None,
    };
    Ok(RehearsalSubmissionClaimSnapshot {
        binding: root.binding,
        identity: ClaimIdentity {
            binding: root.binding,
            operation: prior.operation,
            generation: prior.generation,
        },
        state: prior.phase.into(),
        used_operations,
        completion_proof,
    })
}

fn validate_transition(
    prior: RehearsalClaimTransitionEvent,
    event: RehearsalClaimTransitionEvent,
    used_operations: &mut BTreeSet<question_model::RehearsalGradeOperationId>,
) -> Result<(), RehearsalClaimHydrationError> {
    validate_phase_material(event)?;
    if event.generation == prior.generation {
        if event.operation != prior.operation {
            return Err(RehearsalClaimHydrationError::SameGenerationOperationMismatch);
        }
        return match (prior.phase, event.phase) {
            (
                RehearsalSubmissionClaimPhase::Prepared,
                RehearsalSubmissionClaimPhase::GradingDispatched,
            )
            | (
                RehearsalSubmissionClaimPhase::Prepared,
                RehearsalSubmissionClaimPhase::AbandonedBeforeDispatch,
            )
            | (
                RehearsalSubmissionClaimPhase::Prepared,
                RehearsalSubmissionClaimPhase::RevokedStaleRevision,
            )
            | (
                RehearsalSubmissionClaimPhase::Prepared,
                RehearsalSubmissionClaimPhase::RevokedTerminalLifecycle,
            )
            | (
                RehearsalSubmissionClaimPhase::Prepared,
                RehearsalSubmissionClaimPhase::RevokedSourceContextRemoved,
            )
            | (
                RehearsalSubmissionClaimPhase::GradingDispatched,
                RehearsalSubmissionClaimPhase::Completed,
            )
            | (
                RehearsalSubmissionClaimPhase::GradingDispatched,
                RehearsalSubmissionClaimPhase::RevokedStaleRevision,
            )
            | (
                RehearsalSubmissionClaimPhase::GradingDispatched,
                RehearsalSubmissionClaimPhase::RevokedTerminalLifecycle,
            )
            | (
                RehearsalSubmissionClaimPhase::GradingDispatched,
                RehearsalSubmissionClaimPhase::RevokedSourceContextRemoved,
            ) => Ok(()),
            _ => Err(RehearsalClaimHydrationError::IllegalTransition),
        };
    }
    let expected = prior
        .generation
        .next()
        .ok_or(RehearsalClaimHydrationError::GenerationNotNext)?;
    if event.generation != expected {
        return Err(RehearsalClaimHydrationError::GenerationNotNext);
    }
    if prior.phase != RehearsalSubmissionClaimPhase::AbandonedBeforeDispatch
        || event.phase != RehearsalSubmissionClaimPhase::Prepared
    {
        return Err(RehearsalClaimHydrationError::IllegalTransition);
    }
    if !used_operations.insert(event.operation) {
        return Err(RehearsalClaimHydrationError::ReusedOperation);
    }
    Ok(())
}

fn validate_phase_material(
    event: RehearsalClaimTransitionEvent,
) -> Result<(), RehearsalClaimHydrationError> {
    match event.phase {
        RehearsalSubmissionClaimPhase::AbandonedBeforeDispatch
            if event.abandon_reason.is_some() && event.completion_material.is_none() =>
        {
            Ok(())
        }
        RehearsalSubmissionClaimPhase::Completed
            if event.abandon_reason.is_none() && event.completion_material.is_some() =>
        {
            Ok(())
        }
        RehearsalSubmissionClaimPhase::AbandonedBeforeDispatch
        | RehearsalSubmissionClaimPhase::Completed => {
            Err(RehearsalClaimHydrationError::PhaseMaterialMismatch)
        }
        _ if event.abandon_reason.is_some() || event.completion_material.is_some() => {
            Err(RehearsalClaimHydrationError::PhaseMaterialMismatch)
        }
        _ => Ok(()),
    }
}

#[derive(Debug, PartialEq)]
pub enum RehearsalSubmissionClaimDecision {
    New {
        handle: PreparedClaimHandle,
    },
    Reclaimed {
        handle: PreparedClaimHandle,
    },
    Replay {
        receipt: question_model::RehearsalPublicOutcome,
    },
    Pending,
    Conflict,
    ReclaimRefused(RehearsalClaimReclaimError),
    StaleRevision,
    TerminalLifecycle,
}

pub fn decide_submission_claim(
    lifecycle: RehearsalLifecycle,
    revision_is_current: bool,
    existing: Option<&RehearsalSubmissionClaimSnapshot>,
    requested: RehearsalSubmissionRequestFingerprint,
    new_root: &RehearsalClaimRoot,
    new_operation: question_model::RehearsalGradeOperationId,
) -> RehearsalSubmissionClaimDecision {
    if let Some(existing) = existing {
        if existing.fingerprint() != requested {
            return RehearsalSubmissionClaimDecision::Conflict;
        }
        if existing.state() == RehearsalSubmissionClaimState::Completed {
            return RehearsalSubmissionClaimDecision::Replay {
                receipt: existing
                    .completion_proof()
                    .expect("completed snapshot has proof")
                    .replay_receipt(),
            };
        }
    }
    if !revision_is_current {
        return RehearsalSubmissionClaimDecision::StaleRevision;
    }
    if lifecycle.is_terminal() {
        return RehearsalSubmissionClaimDecision::TerminalLifecycle;
    }
    let Some(existing) = existing else {
        if requested != new_root.fingerprint() {
            return RehearsalSubmissionClaimDecision::Conflict;
        }
        return RehearsalSubmissionClaimDecision::New {
            handle: PreparedClaimHandle(ClaimIdentity {
                binding: new_root.binding,
                operation: new_operation,
                generation: RehearsalClaimGeneration::first(),
            }),
        };
    };
    match existing.state() {
        RehearsalSubmissionClaimState::Completed => unreachable!("completed replay returns first"),
        RehearsalSubmissionClaimState::Prepared
        | RehearsalSubmissionClaimState::GradingDispatched => {
            RehearsalSubmissionClaimDecision::Pending
        }
        RehearsalSubmissionClaimState::AbandonedBeforeDispatch => {
            match existing.generation().next() {
                Some(generation) if !existing.has_used_operation(new_operation) => {
                    RehearsalSubmissionClaimDecision::Reclaimed {
                        handle: PreparedClaimHandle(ClaimIdentity {
                            binding: existing.binding,
                            operation: new_operation,
                            generation,
                        }),
                    }
                }
                Some(_) => RehearsalSubmissionClaimDecision::ReclaimRefused(
                    RehearsalClaimReclaimError::ReusedOperation,
                ),
                None => RehearsalSubmissionClaimDecision::ReclaimRefused(
                    RehearsalClaimReclaimError::GenerationExhausted,
                ),
            }
        }
        RehearsalSubmissionClaimState::RevokedStaleRevision => {
            RehearsalSubmissionClaimDecision::StaleRevision
        }
        RehearsalSubmissionClaimState::RevokedTerminalLifecycle => {
            RehearsalSubmissionClaimDecision::TerminalLifecycle
        }
        RehearsalSubmissionClaimState::RevokedSourceContextRemoved => {
            RehearsalSubmissionClaimDecision::TerminalLifecycle
        }
    }
}

pub fn validate_claim_completion(
    lifecycle: RehearsalLifecycle,
    revision_is_current: bool,
    _dispatched: DispatchedClaimHandle,
) -> Result<(), RehearsalClaimCompletionError> {
    if !revision_is_current {
        return Err(RehearsalClaimCompletionError::StaleRevision);
    }
    if lifecycle.is_terminal() {
        return Err(RehearsalClaimCompletionError::TerminalLifecycle);
    }
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalClaimCompletionError {
    StaleRevision,
    TerminalLifecycle,
}
