//! Dedicated persistence contract for instructor-owned rehearsal runs.

use async_trait::async_trait;
use domain::{DispatchedClaimHandle, PreparedClaimHandle, RehearsalPreDispatchAbandonReason};
use question_model::{
    AssignmentReference, CourseId, RehearsalPublicOutcome, RehearsalReference, RehearsalRunReceipt,
    RehearsalSubjectStart, TeachingOperationRevision, UserId,
};
#[cfg(feature = "test-support")]
use question_model::{RehearsalAttemptId, RehearsalPrivateGradingResult, StudentResponse};
#[cfg(all(not(feature = "test-support"), feature = "postgres"))]
use question_model::{RehearsalAttemptId, StudentResponse};

use crate::{RehearsalOperationDigest, StoreError, TenantContext};

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

/// Applies the closed T4 source-family boundary before a normal assignment
/// item becomes immutable rehearsal material.  The start broker treats an
/// unsupported item as a whole-operation refusal; it never silently omits an
/// item from an otherwise live assignment.
pub fn ensure_rehearsal_question_source_supported(
    question: &question_model::QuestionDefinition,
) -> Result<(), StoreError> {
    if matches!(
        question.grading,
        question_model::GradingDefinition::Ungraded
    ) {
        return Err(StoreError::Unavailable(
            "rehearsal delivery unsupported: source is not deterministically gradeable".into(),
        ));
    }
    match &question.source {
        question_model::QuestionSource::Native { family } if !family.trim().is_empty() => {
            ensure_rehearsal_delivery_supported(&question.response)
        }
        question_model::QuestionSource::Native { .. } => Err(StoreError::Unavailable(
            "rehearsal delivery unsupported: empty native family".into(),
        )),
        question_model::QuestionSource::Webwork { .. } => Err(StoreError::Unavailable(
            "rehearsal delivery unsupported: WebWork sealed contract is not installed".into(),
        )),
        question_model::QuestionSource::Qti { .. } => Err(StoreError::Unavailable(
            "rehearsal delivery unsupported: QTI sealed contract is not installed".into(),
        )),
        question_model::QuestionSource::H5p { .. } => Err(StoreError::Unavailable(
            "rehearsal delivery unsupported: H5P is browser-evaluated".into(),
        )),
        question_model::QuestionSource::Imathas { .. } => Err(StoreError::Unavailable(
            "rehearsal delivery unsupported: iMathAS sealed contract is not installed".into(),
        )),
    }
}

/// Public-route start command.  It deliberately contains only the values an
/// authenticated Instructor route can receive or derive from HTTP: the Store
/// resolves the direct membership, current source, internal run identity,
/// aggregate witness, and immutable response projection under its lock.
#[derive(Debug, Clone)]
pub struct StartRehearsalRouteCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentReference,
    pub expected_revision: TeachingOperationRevision,
    pub subject: RehearsalSubjectStart,
    pub start_new_after_completion: bool,
    pub idempotency_key: RehearsalIdempotencyKey,
    pub request_fingerprint: RehearsalOperationDigest,
}

/// Exact, answer-free persisted start response.  A replay returns the same
/// receipt rather than recalculating the caller's start decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRehearsalRouteResult {
    pub receipt: RehearsalRunReceipt,
    pub replayed: bool,
}

/// Public-reference read command.  Unlike the internal locator, a browser
/// never supplies an internal run id or revision witness.  The Store binds the
/// reference to the route source and verifies the persisted revision itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadRehearsalRouteCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentReference,
    pub rehearsal: RehearsalReference,
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

/// A bounded idempotency key retained only in rehearsal-local claim state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RehearsalIdempotencyKey(String);

impl RehearsalIdempotencyKey {
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

/// Compatibility spelling for sealed submission-claim call sites.  Submission
/// remains a distinct protocol; it shares only the bounded key value.
pub type RehearsalSubmissionIdempotencyKey = RehearsalIdempotencyKey;

/// Browser input sealed by the Store before a server calls a grader.
#[derive(Clone, PartialEq)]
#[cfg(feature = "test-support")]
pub struct ClaimRehearsalSubmissionCommand {
    pub locator: RehearsalLocator,
    pub attempt: RehearsalAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: RehearsalIdempotencyKey,
}
/// Crate-private durable admission input retained for persistence adapters.
/// Production routes construct authenticated rendered input instead; this type
/// cannot be named by a server or browser caller.
#[derive(Clone, PartialEq)]
#[cfg(all(not(feature = "test-support"), feature = "postgres"))]
pub(crate) struct ClaimRehearsalSubmissionCommand {
    pub(crate) locator: RehearsalLocator,
    pub(crate) attempt: RehearsalAttemptId,
    pub(crate) response: StudentResponse,
    pub(crate) idempotency_key: RehearsalIdempotencyKey,
}

/// Server-private work granted to exactly one logical grading operation.
#[derive(PartialEq)]
#[cfg(feature = "test-support")]
pub struct ClaimedRehearsalSubmission {
    pub handle: PreparedClaimHandle,
}
#[derive(PartialEq)]
#[cfg(not(feature = "test-support"))]
pub struct ClaimedRehearsalSubmission {
    pub handle: PreparedClaimHandle,
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
#[cfg(feature = "test-support")]
pub struct CompleteRehearsalSubmissionCommand {
    pub locator: RehearsalLocator,
    pub handle: DispatchedClaimHandle,
    pub grading: RehearsalPrivateGradingResult,
}

/// Private state transition that is committed before any grader can receive
/// the sealed response. It is not a browser DTO.
#[cfg(feature = "test-support")]
pub struct MarkRehearsalSubmissionDispatchedCommand {
    pub locator: RehearsalLocator,
    pub handle: PreparedClaimHandle,
}
#[cfg(not(feature = "test-support"))]
pub(crate) struct MarkRehearsalSubmissionDispatchedCommand {
    pub(crate) locator: RehearsalLocator,
    pub(crate) handle: PreparedClaimHandle,
}

/// A definite local failure before dispatch. The grading coordinator owns this
/// command: it has the in-process knowledge that a grader was never called.
/// It is deliberately absent from the general route-facing rehearsal Store.
#[cfg(feature = "test-support")]
pub struct AbandonRehearsalSubmissionBeforeDispatchCommand {
    pub locator: RehearsalLocator,
    pub handle: PreparedClaimHandle,
    pub reason: RehearsalPreDispatchAbandonReason,
}
#[cfg(not(feature = "test-support"))]
pub(crate) struct AbandonRehearsalSubmissionBeforeDispatchCommand {
    pub(crate) locator: RehearsalLocator,
    pub(crate) handle: PreparedClaimHandle,
    pub(crate) reason: RehearsalPreDispatchAbandonReason,
}

/// Browser-safe result of an accepted or replayed rehearsal-local submission.
#[derive(Debug, Clone, PartialEq)]
pub struct RehearsalSubmissionReceipt {
    pub outcome: RehearsalPublicOutcome,
    pub replayed: bool,
}

/// The dedicated route capability. It never exposes ordinary learner-run
/// identities or attempt-supplied mutation commands in a normal build.
#[async_trait]
pub trait RehearsalStore: Send + Sync {
    async fn start_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: StartRehearsalRouteCommand,
    ) -> Result<StartRehearsalRouteResult, StoreError>;
    async fn read_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: ReadRehearsalRouteCommand,
    ) -> Result<RehearsalRunReceipt, StoreError>;
    async fn read_rehearsal(
        &self,
        context: TenantContext,
        locator: RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError>;
}

/// Store-private implementation seam.  Route brokers use these methods only
/// after binding a complete public route and, for submissions, deriving the
/// sole issued attempt.  Keeping the seam `pub(crate)` prevents production
/// callers from naming attempt-supplied commands (ASVS 8.2.1, 8.3.1).
#[async_trait]
pub(crate) trait RehearsalInternalStore: Send + Sync {
    async fn start_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: StartRehearsalRouteCommand,
    ) -> Result<StartRehearsalRouteResult, StoreError>;
    async fn read_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: ReadRehearsalRouteCommand,
    ) -> Result<RehearsalRunReceipt, StoreError>;
    async fn read_rehearsal(
        &self,
        context: TenantContext,
        locator: RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError>;
    #[cfg(feature = "test-support")]
    async fn claim_rehearsal_submission(
        &self,
        context: TenantContext,
        command: ClaimRehearsalSubmissionCommand,
    ) -> Result<RehearsalSubmissionClaimResult, StoreError>;
    #[cfg(feature = "test-support")]
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
    #[cfg(feature = "test-support")]
    async fn discard_rehearsal(
        &self,
        context: TenantContext,
        locator: RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError>;
    #[cfg(feature = "test-support")]
    async fn complete_rehearsal(
        &self,
        context: TenantContext,
        locator: RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError>;
}

#[async_trait]
impl<T: RehearsalInternalStore + ?Sized> RehearsalStore for T {
    async fn start_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: StartRehearsalRouteCommand,
    ) -> Result<StartRehearsalRouteResult, StoreError> {
        RehearsalInternalStore::start_rehearsal_from_route(self, context, command).await
    }

    async fn read_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: ReadRehearsalRouteCommand,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        RehearsalInternalStore::read_rehearsal_from_route(self, context, command).await
    }

    async fn read_rehearsal(
        &self,
        context: TenantContext,
        locator: RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        RehearsalInternalStore::read_rehearsal(self, context, locator).await
    }
}

/// Explicit test-only access to the legacy attempt-supplied conformance seam.
/// The trait and its command types are absent from normal crate exports.
#[cfg(feature = "test-support")]
#[async_trait]
pub trait RehearsalTestSupportStore: Send + Sync {
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

#[cfg(feature = "test-support")]
#[async_trait]
impl<T: RehearsalInternalStore + ?Sized> RehearsalTestSupportStore for T {
    async fn claim_rehearsal_submission(
        &self,
        context: TenantContext,
        command: ClaimRehearsalSubmissionCommand,
    ) -> Result<RehearsalSubmissionClaimResult, StoreError> {
        RehearsalInternalStore::claim_rehearsal_submission(self, context, command).await
    }

    async fn complete_rehearsal_submission(
        &self,
        context: TenantContext,
        command: CompleteRehearsalSubmissionCommand,
    ) -> Result<RehearsalSubmissionReceipt, StoreError> {
        RehearsalInternalStore::complete_rehearsal_submission(self, context, command).await
    }

    async fn mark_rehearsal_submission_dispatched(
        &self,
        context: TenantContext,
        command: MarkRehearsalSubmissionDispatchedCommand,
    ) -> Result<DispatchedClaimHandle, StoreError> {
        RehearsalInternalStore::mark_rehearsal_submission_dispatched(self, context, command).await
    }

    async fn discard_rehearsal(
        &self,
        context: TenantContext,
        locator: RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        RehearsalInternalStore::discard_rehearsal(self, context, locator).await
    }

    async fn complete_rehearsal(
        &self,
        context: TenantContext,
        locator: RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        RehearsalInternalStore::complete_rehearsal(self, context, locator).await
    }
}

/// Internal composition capability for the deterministic grading coordinator.
///
/// The Prepared handle, closed reason, and durable claim phase prevent a
/// dispatched request from being reclaimed.  This trait additionally keeps
/// the pre-dispatch compensation authority out of route/general Store APIs:
/// only a coordinator that can establish the definite in-process fact that no
/// grader observed the request may receive this capability.
#[async_trait]
#[cfg(feature = "test-support")]
pub trait RehearsalPreDispatchCompensationStore: Send + Sync {
    async fn abandon_rehearsal_submission_before_dispatch(
        &self,
        context: TenantContext,
        command: AbandonRehearsalSubmissionBeforeDispatchCommand,
    ) -> Result<(), StoreError>;
}
#[async_trait]
#[cfg(not(feature = "test-support"))]
pub(crate) trait RehearsalPreDispatchCompensationStore: Send + Sync {
    async fn abandon_rehearsal_submission_before_dispatch(
        &self,
        context: TenantContext,
        command: AbandonRehearsalSubmissionBeforeDispatchCommand,
    ) -> Result<(), StoreError>;
}
