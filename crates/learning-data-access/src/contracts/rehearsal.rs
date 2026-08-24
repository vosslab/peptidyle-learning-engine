//! Dedicated persistence contract for instructor-owned rehearsal runs.

use async_trait::async_trait;
use domain::{
    DispatchedClaimHandle, PreparedClaimHandle, RehearsalPreDispatchAbandonReason,
    RehearsalValidatedSubmissionRequest,
};
use question_model::{
    AssignmentReference, CourseId, RehearsalAttemptId, RehearsalFrozenItemEvidence,
    RehearsalPrivateGradingResult, RehearsalPublicOutcome, RehearsalReference, RehearsalRunReceipt,
    RehearsalSubjectStart, StudentResponse, TeachingOperationRevision, UserId,
};

use crate::{StoreError, TenantContext};

/// Rejects response families that cannot be delivered and graded entirely by
/// the deterministic rehearsal path. Call before freezing an item so an
/// unsupported family never becomes renderable evidence or accepts a body.
pub fn ensure_rehearsal_delivery_supported(
    definition: &question_model::ResponseDefinition,
) -> Result<(), StoreError> {
    match definition {
        question_model::ResponseDefinition::FileUpload { .. } => Err(StoreError::Unavailable(
            "rehearsal delivery unsupported: file upload".into(),
        )),
        question_model::ResponseDefinition::ExternalTool {} => Err(StoreError::Unavailable(
            "rehearsal delivery unsupported: external tool".into(),
        )),
        _ => Ok(()),
    }
}

/// Route-owned start command. The Store resolves this candidate under its lock.
#[derive(Debug, Clone)]
pub struct StartRehearsalCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub subject: RehearsalSubjectStart,
    pub start_new_after_completion: bool,
}

/// Exact aggregate binding used after route authorization and locator resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalLocator {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub rehearsal: RehearsalReference,
}

/// Trusted server-only item issue prepared by the isolated execution helper.
#[derive(Debug, Clone, PartialEq)]
pub struct AppendRehearsalFrozenItemCommand {
    pub locator: RehearsalLocator,
    pub frozen: RehearsalFrozenItemEvidence,
}

/// A bounded idempotency key retained only in rehearsal-local claim state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RehearsalSubmissionIdempotencyKey(String);

impl RehearsalSubmissionIdempotencyKey {
    pub const MAX_SCALARS: usize = 128;

    pub fn new(value: String) -> Result<Self, StoreError> {
        (!value.is_empty()
            && value.chars().count() <= Self::MAX_SCALARS
            && !value.chars().any(char::is_control))
        .then_some(Self(value))
        .ok_or_else(|| StoreError::InvalidRecord("invalid rehearsal idempotency key".into()))
    }

    /// The already-validated private value used for an exact SQL binding.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Browser input sealed by the Store before a server calls a grader.
#[derive(Clone, PartialEq)]
pub struct ClaimRehearsalSubmissionCommand {
    pub locator: RehearsalLocator,
    pub attempt: RehearsalAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: RehearsalSubmissionIdempotencyKey,
}

/// Server-private work granted to exactly one logical grading operation.
#[derive(PartialEq)]
pub struct ClaimedRehearsalSubmission {
    pub handle: PreparedClaimHandle,
    pub prepared: RehearsalValidatedSubmissionRequest,
}

/// Claim result observed before a grader executes.
#[derive(PartialEq)]
pub enum RehearsalSubmissionClaimResult {
    Claimed(ClaimedRehearsalSubmission),
    Replay(RehearsalSubmissionReceipt),
    Pending,
    Conflict,
}

/// Server-only completion after one deterministic grading operation.
#[derive(PartialEq)]
pub struct CompleteRehearsalSubmissionCommand {
    pub locator: RehearsalLocator,
    pub handle: DispatchedClaimHandle,
    pub grading: RehearsalPrivateGradingResult,
}

/// Private state transition that is committed before any grader can receive
/// the sealed response. It is not a browser DTO.
pub struct MarkRehearsalSubmissionDispatchedCommand {
    pub locator: RehearsalLocator,
    pub handle: PreparedClaimHandle,
}

/// A definite local failure before dispatch. The grading coordinator owns this
/// command: it has the in-process knowledge that a grader was never called.
/// It is deliberately absent from the general route-facing rehearsal Store.
pub struct AbandonRehearsalSubmissionBeforeDispatchCommand {
    pub locator: RehearsalLocator,
    pub handle: PreparedClaimHandle,
    pub reason: RehearsalPreDispatchAbandonReason,
}

/// Browser-safe result of an accepted or replayed rehearsal-local submission.
#[derive(Debug, Clone, PartialEq)]
pub struct RehearsalSubmissionReceipt {
    pub outcome: RehearsalPublicOutcome,
    pub replayed: bool,
}

/// The dedicated capability. It never exposes ordinary learner-run identities.
#[async_trait]
pub trait RehearsalStore: Send + Sync {
    async fn start_rehearsal(
        &self,
        context: TenantContext,
        command: StartRehearsalCommand,
    ) -> Result<RehearsalRunReceipt, StoreError>;
    async fn read_rehearsal(
        &self,
        context: TenantContext,
        locator: RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError>;
    async fn append_rehearsal_frozen_item(
        &self,
        context: TenantContext,
        command: AppendRehearsalFrozenItemCommand,
    ) -> Result<(), StoreError>;
    async fn claim_rehearsal_submission(
        &self,
        context: TenantContext,
        command: ClaimRehearsalSubmissionCommand,
    ) -> Result<RehearsalSubmissionClaimResult, StoreError>;
    async fn complete_rehearsal_submission(
        &self,
        context: TenantContext,
        command: CompleteRehearsalSubmissionCommand,
    ) -> Result<RehearsalSubmissionReceipt, StoreError>;
    async fn mark_rehearsal_submission_dispatched(
        &self,
        context: TenantContext,
        command: MarkRehearsalSubmissionDispatchedCommand,
    ) -> Result<DispatchedClaimHandle, StoreError>;
    async fn discard_rehearsal(
        &self,
        context: TenantContext,
        locator: RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError>;
    async fn complete_rehearsal(
        &self,
        context: TenantContext,
        locator: RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError>;
}

/// Internal composition capability for the deterministic grading coordinator.
///
/// The Prepared handle, closed reason, and durable claim phase prevent a
/// dispatched request from being reclaimed.  This trait additionally keeps
/// the pre-dispatch compensation authority out of route/general Store APIs:
/// only a coordinator that can establish the definite in-process fact that no
/// grader observed the request may receive this capability.
#[async_trait]
pub trait RehearsalPreDispatchCompensationStore: Send + Sync {
    async fn abandon_rehearsal_submission_before_dispatch(
        &self,
        context: TenantContext,
        command: AbandonRehearsalSubmissionBeforeDispatchCommand,
    ) -> Result<(), StoreError>;
}
