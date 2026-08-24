//! Sealed recovery boundary for a rehearsal submission already admitted by a route.

use async_trait::async_trait;
use domain::{DispatchedClaimHandle, RehearsalValidatedSubmissionRequest};
use question_model::{
    RehearsalAttemptId, RehearsalBackendReceiptReference, RehearsalFrozenItemEvidence,
    RehearsalPresentationDigestV1,
};

use crate::{
    RehearsalRouteIdentity, RehearsalSubmissionIdempotencyKey, RehearsalSubmissionReceipt,
    StoreError, TenantContext,
};

/// Move-only authority to commit a deterministic result for one authenticated
/// sealed submission preparation.
///
/// This is intentionally neither serializable nor cloneable.  It retains the
/// route and exact immutable witnesses which authorized rendered-ID
/// translation, but exposes none of them outside this crate.  A dispatched
/// claim handle is consequently *dispatch identity*, never completion
/// authority (ASVS V4.1.2, V8.3.1).
pub struct SealedRehearsalSubmissionCompletion {
    context: TenantContext,
    route: RehearsalRouteIdentity,
    handle: DispatchedClaimHandle,
    root: domain::RehearsalClaimRoot,
    attempt: RehearsalAttemptId,
    frozen: RehearsalFrozenItemEvidence,
    expected_evidence_head: domain::RehearsalEvidenceHead,
    presentation_commitment: RehearsalPresentationDigestV1,
    durable_request: RehearsalValidatedSubmissionRequest,
}

/// Crate-private sealed preparation output. Keeping the constructor input
/// named prevents later authentication facts from being silently omitted as
/// the sealed execution protocol evolves.
pub(crate) struct SealedRehearsalSubmissionCompletionParts {
    pub(crate) context: TenantContext,
    pub(crate) route: RehearsalRouteIdentity,
    pub(crate) handle: DispatchedClaimHandle,
    pub(crate) root: domain::RehearsalClaimRoot,
    pub(crate) attempt: RehearsalAttemptId,
    pub(crate) frozen: RehearsalFrozenItemEvidence,
    pub(crate) expected_evidence_head: domain::RehearsalEvidenceHead,
    pub(crate) presentation_commitment: RehearsalPresentationDigestV1,
    pub(crate) durable_request: RehearsalValidatedSubmissionRequest,
}

impl SealedRehearsalSubmissionCompletion {
    pub(crate) fn new(parts: SealedRehearsalSubmissionCompletionParts) -> Self {
        Self {
            context: parts.context,
            route: parts.route,
            handle: parts.handle,
            root: parts.root,
            attempt: parts.attempt,
            frozen: parts.frozen,
            expected_evidence_head: parts.expected_evidence_head,
            presentation_commitment: parts.presentation_commitment,
            durable_request: parts.durable_request,
        }
    }

    #[allow(dead_code)] // consumed by the optional PostgreSQL sealed adapter
    pub(crate) const fn context(&self) -> TenantContext {
        self.context
    }

    #[allow(dead_code)] // consumed by the optional PostgreSQL sealed adapter
    pub(crate) const fn route(&self) -> RehearsalRouteIdentity {
        self.route
    }

    #[allow(dead_code)] // consumed by the optional PostgreSQL sealed adapter
    pub(crate) fn handle(&self) -> &DispatchedClaimHandle {
        &self.handle
    }

    #[allow(dead_code)] // consumed by the optional PostgreSQL sealed adapter
    pub(crate) fn root(&self) -> &domain::RehearsalClaimRoot {
        &self.root
    }

    #[allow(dead_code)] // consumed by the optional PostgreSQL sealed adapter
    pub(crate) fn frozen(&self) -> &RehearsalFrozenItemEvidence {
        &self.frozen
    }

    pub(crate) const fn expected_evidence_head(&self) -> domain::RehearsalEvidenceHead {
        self.expected_evidence_head
    }

    #[allow(dead_code)] // consumed by the optional PostgreSQL sealed adapter
    pub(crate) const fn presentation_commitment(&self) -> RehearsalPresentationDigestV1 {
        self.presentation_commitment
    }

    #[allow(dead_code)] // consumed by the optional PostgreSQL sealed adapter
    pub(crate) fn durable_request(&self) -> &RehearsalValidatedSubmissionRequest {
        &self.durable_request
    }

    pub(crate) fn into_internal_parts(
        self,
    ) -> (
        TenantContext,
        RehearsalRouteIdentity,
        DispatchedClaimHandle,
        domain::RehearsalClaimRoot,
        RehearsalAttemptId,
        RehearsalFrozenItemEvidence,
        domain::RehearsalEvidenceHead,
        RehearsalPresentationDigestV1,
        RehearsalValidatedSubmissionRequest,
    ) {
        (
            self.context,
            self.route,
            self.handle,
            self.root,
            self.attempt,
            self.frozen,
            self.expected_evidence_head,
            self.presentation_commitment,
            self.durable_request,
        )
    }

    /// Stable, bounded correlation value for a deterministic grader backend.
    /// It is derived from the Store-minted grade operation and intentionally
    /// reveals no claim, route, response, screen, or durable identifiers.
    pub fn backend_receipt_reference(
        &self,
    ) -> Result<RehearsalBackendReceiptReference, StoreError> {
        RehearsalBackendReceiptReference::new(format!(
            "rehearsal-grade-v1:{}",
            self.handle.operation().as_uuid()
        ))
        .map_err(|_| StoreError::InvalidRecord("invalid rehearsal receipt reference".into()))
    }
}

impl std::fmt::Debug for SealedRehearsalSubmissionCompletion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SealedRehearsalSubmissionCompletion([REDACTED])")
    }
}

/// Opaque, server-only grading work coupled to the one capability that can
/// accept its result. It is deliberately neither serializable nor cloneable.
pub struct SealedRehearsalSubmissionExecutionWork {
    grading: crate::SealedRehearsalGradingParts,
    completion: SealedRehearsalSubmissionCompletion,
}

impl SealedRehearsalSubmissionExecutionWork {
    pub(crate) fn new(
        grading: crate::SealedRehearsalGradingParts,
        completion: SealedRehearsalSubmissionCompletion,
    ) -> Self {
        Self {
            grading,
            completion,
        }
    }
    /// Returns the sole already-translated deterministic grading input.
    ///
    /// The rendered browser response and artifact's rendered-ID mapping have
    /// both been consumed at the sealed Store boundary.  A coordinator can
    /// therefore grade this work without gaining a second translation path.
    pub fn grading(&self) -> &crate::SealedRehearsalGradingParts {
        &self.grading
    }
    pub fn into_grading_and_completion(
        self,
    ) -> (
        crate::SealedRehearsalGradingParts,
        SealedRehearsalSubmissionCompletion,
    ) {
        (self.grading, self.completion)
    }
}

impl std::fmt::Debug for SealedRehearsalSubmissionExecutionWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SealedRehearsalSubmissionExecutionWork([REDACTED])")
    }
}

pub enum SealedRehearsalSubmissionExecutionPreparation {
    Work(Box<SealedRehearsalSubmissionExecutionWork>),
    Receipt(RehearsalSubmissionReceipt),
    /// A claim exists but grading has not been durably dispatched. The route
    /// coordinator may dispatch it, but may not grade it from this result.
    PendingPreparation,
}

#[async_trait]
pub trait SealedRehearsalSubmissionExecutionStore: Send + Sync {
    async fn prepare_or_resume_sealed_rehearsal_submission_execution(
        &self,
        context: TenantContext,
        route: RehearsalRouteIdentity,
        idempotency_key: RehearsalSubmissionIdempotencyKey,
    ) -> Result<SealedRehearsalSubmissionExecutionPreparation, StoreError>;

    /// Commits a trusted deterministic grader result through the one
    /// capability minted by sealed preparation. A route/app caller cannot
    /// synthesize this authority from a dispatched handle.
    async fn complete_sealed_rehearsal_submission_execution(
        &self,
        context: TenantContext,
        completion: SealedRehearsalSubmissionCompletion,
        grading: question_model::RehearsalPrivateGradingResult,
    ) -> Result<RehearsalSubmissionReceipt, StoreError>;
}
