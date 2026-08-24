//! Route-shaped mutation boundary for live instructor rehearsal operations.
//!
//! HTTP routes hold public course, assignment, rehearsal, and revision values.
//! They never reconstruct [`RehearsalLocator`]; implementations resolve that
//! private aggregate binding while the authorization/operation effect is live.

use async_trait::async_trait;
use question_model::{
    ActivityTimestamp, AssignmentReference, CourseId, RehearsalReference, StudentResponse,
    TeachingOperationRevision, UserId,
};

use crate::{
    DispatchedRehearsalDelivery, PreparedRehearsalDelivery, RehearsalDeliveryClaimResult,
    RehearsalDeliveryDispatchResult, RehearsalDeliveryPreDispatchAbandonReason,
    RehearsalIdempotencyKey, RehearsalIdempotentProjectionResult, RehearsalOperationDigest,
    RehearsalSafeProjection, RehearsalSubmissionClaimResult, StoreError, TenantContext,
};
use domain::{DispatchedClaimHandle, PreparedClaimHandle, RehearsalPreDispatchAbandonReason};

/// The complete public identity accepted by a mutable rehearsal route.
/// `expected_revision` is the HTTP If-Match witness and is checked before a
/// planner or grader receives any work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalRouteIdentity {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentReference,
    pub rehearsal: RehearsalReference,
    pub expected_revision: TeachingOperationRevision,
}

#[derive(Clone, PartialEq)]
pub struct ClaimRehearsalDeliveryRouteCommand {
    pub route: RehearsalRouteIdentity,
    pub idempotency_key: RehearsalIdempotencyKey,
    pub request_fingerprint: RehearsalOperationDigest,
}

/// Reconciles a dispatched generation against the server-owned clock. The
/// route carries no item, attempt, generation, or browser timestamp.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ReconcileRehearsalDeliveryExpiryRouteCommand {
    pub route: RehearsalRouteIdentity,
}

/// Explicit same-item retry request. Its key is independent of Continue and
/// replays the Store-created successor for an exact request fingerprint.
#[derive(Clone, PartialEq)]
pub struct RetryRehearsalDeliveryRouteCommand {
    pub route: RehearsalRouteIdentity,
    pub idempotency_key: RehearsalIdempotencyKey,
    pub request_fingerprint: RehearsalOperationDigest,
}

/// Browser-safe availability result derived from immutable delivery evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalDeliveryTimingResult {
    pub verdict: domain::RehearsalTimingVerdictV1,
    pub deadline: Option<ActivityTimestamp>,
    pub expires_at: Option<ActivityTimestamp>,
    pub retry_disposition: RehearsalDeliveryRetryDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalDeliveryRetryDisposition {
    NotApplicable,
    Available,
    RunTimeExhausted,
}

pub enum RetryRehearsalDeliveryResult {
    Prepared {
        prepared: PreparedRehearsalDelivery,
    },
    Pending {
        dispatched: DispatchedRehearsalDelivery,
    },
    Replay(question_model::RehearsalActiveScreenV1),
    RunTimeExhausted {
        deadline: ActivityTimestamp,
    },
    Conflict,
}

#[derive(Clone, PartialEq)]
pub struct ClaimRehearsalSubmissionRouteCommand {
    pub route: RehearsalRouteIdentity,
    pub response: StudentResponse,
    /// Exact commitment from the issued answer-free screen.  The Store
    /// compares this to immutable delivery evidence before grading work is
    /// minted; a browser never names an attempt or delivery operation.
    pub presentation_digest: question_model::PresentationDigestTokenV1,
    pub idempotency_key: RehearsalIdempotencyKey,
}

#[derive(Clone, PartialEq)]
pub struct DiscardRehearsalRouteCommand {
    pub route: RehearsalRouteIdentity,
    pub idempotency_key: RehearsalIdempotencyKey,
    pub request_fingerprint: RehearsalOperationDigest,
    pub response: RehearsalSafeProjection,
    pub response_digest: RehearsalOperationDigest,
}

pub struct CompleteRehearsalDeliveryRouteCommand {
    pub route: RehearsalRouteIdentity,
    pub dispatched: DispatchedRehearsalDelivery,
    pub screen: question_model::RehearsalActiveScreenV1,
}

/// Narrow server facade. Every method begins with a route identity; opaque
/// Store-minted handles, not browser identifiers, drive follow-up effects.
#[async_trait]
pub trait RehearsalRouteMutationStore: Send + Sync {
    async fn claim_rehearsal_delivery_from_route(
        &self,
        context: TenantContext,
        command: ClaimRehearsalDeliveryRouteCommand,
    ) -> Result<RehearsalDeliveryClaimResult, StoreError>;
    async fn reconcile_rehearsal_delivery_expiry_from_route(
        &self,
        context: TenantContext,
        command: ReconcileRehearsalDeliveryExpiryRouteCommand,
    ) -> Result<RehearsalDeliveryTimingResult, StoreError>;
    async fn retry_rehearsal_delivery_from_route(
        &self,
        context: TenantContext,
        command: RetryRehearsalDeliveryRouteCommand,
    ) -> Result<RetryRehearsalDeliveryResult, StoreError>;
    async fn mark_rehearsal_delivery_dispatched_from_route(
        &self,
        context: TenantContext,
        route: RehearsalRouteIdentity,
        prepared: PreparedRehearsalDelivery,
    ) -> Result<RehearsalDeliveryDispatchResult, StoreError>;
    async fn complete_rehearsal_delivery_from_route(
        &self,
        context: TenantContext,
        command: CompleteRehearsalDeliveryRouteCommand,
    ) -> Result<question_model::RehearsalActiveScreenV1, StoreError>;
    async fn abandon_rehearsal_delivery_before_dispatch_from_route(
        &self,
        context: TenantContext,
        route: RehearsalRouteIdentity,
        prepared: PreparedRehearsalDelivery,
        reason: RehearsalDeliveryPreDispatchAbandonReason,
    ) -> Result<(), StoreError>;
    async fn claim_rehearsal_submission_from_route(
        &self,
        context: TenantContext,
        command: ClaimRehearsalSubmissionRouteCommand,
    ) -> Result<RehearsalSubmissionClaimResult, StoreError>;
    async fn mark_rehearsal_submission_dispatched_from_route(
        &self,
        context: TenantContext,
        route: RehearsalRouteIdentity,
        handle: PreparedClaimHandle,
    ) -> Result<DispatchedClaimHandle, StoreError>;
    /// Server-private recovery operation keyed by the original route request.
    /// It closes the crash window after a Prepared claim: dispatch is atomic,
    /// exact-key replay returns the same opaque handle, and browsers continue
    /// to observe only the status-only Pending claim result.
    async fn dispatch_rehearsal_submission_from_route(
        &self,
        _context: TenantContext,
        _route: RehearsalRouteIdentity,
        _idempotency_key: RehearsalIdempotencyKey,
    ) -> Result<DispatchedClaimHandle, StoreError> {
        Err(StoreError::Unavailable(
            "route-keyed rehearsal submission dispatch is not installed".into(),
        ))
    }
    async fn abandon_rehearsal_submission_before_dispatch_from_route(
        &self,
        context: TenantContext,
        route: RehearsalRouteIdentity,
        handle: PreparedClaimHandle,
        reason: RehearsalPreDispatchAbandonReason,
    ) -> Result<(), StoreError>;
    async fn discard_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: DiscardRehearsalRouteCommand,
    ) -> Result<RehearsalIdempotentProjectionResult, StoreError>;
}
