//! Server-only persistence boundary for accepted automated-grading input.
//!
//! The Student response enters this capability exactly once. Public route and
//! worker packages receive metadata-only records and resolve private input
//! only inside a server-owned execution capability.

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{
    ActivityTimestamp, AssignmentId, AttemptResult, CourseId, GradingOperationAction,
    GradingOperationReason, GradingOperationReference, GradingOperationState, QuestionAttemptId,
    ScoringGeneration, StudentResponse, SubmissionEvaluationStatus, TenantId, UserId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    JobId, JobLeaseToken, PreparedQuestionSubmission, StoreError, SubmissionIdempotencyKey,
    TenantContext,
};

/// Worker-attempt budget shared by initial and Instructor-retried accepted submissions.
///
/// Store adapters use this public contract constant so every backend applies
/// the same bounded recovery policy.
pub const ACCEPTED_SUBMISSION_JOB_MAX_ATTEMPTS: u16 = 3;

mod instructor;

pub use instructor::MAX_INSTRUCTOR_GRADING_RETRY_COUNT;
pub use instructor::{
    GradingOperationActionReceipt, GradingOperationGroup, GradingOperationGroupBy,
    GradingOperationStore, GradingOperationTrustGeneration, InstructorGradingOperationProjection,
    InstructorGradingOperationRow, ListInstructorGradingOperationsCommand,
    RecalculateAssignmentCommand, RetryGradingOperationCommand,
};
pub(crate) use instructor::{GradingOperationCursor, operation_group_key};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AcceptedSubmissionId(Uuid);

impl AcceptedSubmissionId {
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Derives the one assignment-recalculation job owned by a successful
/// accepted-submission completion.
///
/// Keeping this identity derivation beside [`AcceptedSubmissionId`] gives the
/// in-memory adapter, PostgreSQL adapter, and server composition one canonical
/// replay key for the same durable follow-on work.
pub fn accepted_submission_recalculation_job(submission: AcceptedSubmissionId) -> JobId {
    JobId::from_uuid(Uuid::from_u128(submission.as_uuid().as_u128() ^ u128::MAX))
}

/// Positive execution fence, independent of assignment scoring generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GradingExecutionGeneration(u64);

impl GradingExecutionGeneration {
    pub const INITIAL: Self = Self(1);
    pub fn from_u64(value: u64) -> Option<Self> {
        (value > 0).then_some(Self(value))
    }
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Opaque identity of one server worker process.
///
/// This is deliberately separate from [`UserId`]: execution evidence records
/// the server process that held the lease, never an Instructor or Student.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId(Uuid);

impl WorkerId {
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradingExecutionState {
    Ready,
    Running,
    Completed,
    Exception,
    RetryWait,
    Superseded,
}

/// Closed, answer-free category for one immutable execution receipt.
///
/// A category is separate from the mutable execution state: it preserves why
/// that state was reached without recording private Student material or a
/// provider diagnostic.  The database validates its exact state and identity
/// pairing on every append.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradingExecutionReceiptSafeCategory {
    AcceptedSubmission,
    InstructorRetry,
    WorkerClaim,
    Graded,
    DependencyRetry,
    GraderContractFailure,
    GraderExecutionFailure,
    IssuedEvidenceIntegrity,
    RetryExhausted,
}

/// Closed, answer-free category for one immutable Instructor-action receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradingOperationReceiptSafeCategory {
    InstructorRetry,
    InstructorRecalculation,
}

/// Immutable accepted-input metadata. The private response is intentionally absent.
#[derive(Clone, PartialEq)]
pub struct AcceptedSubmission {
    pub tenant: TenantId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub attempt: QuestionAttemptId,
    pub submission: AcceptedSubmissionId,
    pub actor: UserId,
    pub idempotency_key: SubmissionIdempotencyKey,
    /// Canonical digest derived by the store from the accepted response.
    pub request_sha256: Sha256Digest,
    pub accepted_at: ActivityTimestamp,
}

impl std::fmt::Debug for AcceptedSubmission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcceptedSubmission")
            .field("tenant", &self.tenant)
            .field("course", &self.course)
            .field("assignment", &self.assignment)
            .field("attempt", &self.attempt)
            .field("submission", &self.submission)
            .field("actor", &self.actor)
            .field("idempotency_key", &"[REDACTED]")
            .field("request_sha256", &self.request_sha256)
            .field("accepted_at", &self.accepted_at)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradingExecution {
    pub submission: AcceptedSubmissionId,
    pub generation: GradingExecutionGeneration,
    pub state: GradingExecutionState,
    pub job: JobId,
    pub retry_count: u16,
}

/// Closed, data-bearing target of an Instructor recovery thread.
///
/// Submission identity and scoring generation stay distinct because their
/// respective commits are fenced by separate state machines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradingOperationTarget {
    SubmissionRecovery {
        submission: AcceptedSubmissionId,
    },
    AssignmentScoringGeneration {
        requested_generation: ScoringGeneration,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GradingOperationRevision(u64);

impl GradingOperationRevision {
    pub const INITIAL: Self = Self(1);
    pub fn from_u64(value: u64) -> Option<Self> {
        (value > 0).then_some(Self(value))
    }
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GradingOperationActionId(Uuid);

impl GradingOperationActionId {
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Metadata-only current recovery-thread projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradingOperation {
    pub tenant: TenantId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub reference: GradingOperationReference,
    pub target: GradingOperationTarget,
    pub reason: GradingOperationReason,
    pub state: GradingOperationState,
    pub revision: GradingOperationRevision,
    pub next_action: Option<GradingOperationAction>,
}

/// Exact private queue claim required to reload one accepted Student input.
///
/// The worker identity is recorded by W4's committer. It is deliberately not
/// an authenticated user identity. The lease capability is redacted from all
/// diagnostics (ASVS V2.3: bind multi-step work to its exact claim).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AcceptedSubmissionExecutionClaim {
    /// Tenant fence carried through every private worker operation.
    pub tenant: TenantId,
    pub job: JobId,
    pub lease_token: JobLeaseToken,
    pub submission: AcceptedSubmissionId,
    pub execution_generation: GradingExecutionGeneration,
    pub worker: WorkerId,
}

/// Exact identity supplied by the synchronous accepted-submission path.
///
/// The target is intentionally smaller than a leased claim: it names durable
/// work before the store atomically decides whether that work is currently
/// eligible.  The store supplies the lease token and execution generation only
/// after it wins the shared state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptedSubmissionExecutionTarget {
    pub tenant: TenantId,
    pub attempt: QuestionAttemptId,
    pub submission: AcceptedSubmissionId,
    pub job: JobId,
}

impl std::fmt::Debug for AcceptedSubmissionExecutionClaim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcceptedSubmissionExecutionClaim")
            .field("tenant", &self.tenant)
            .field("job", &self.job)
            .field("lease_token", &"[REDACTED]")
            .field("submission", &self.submission)
            .field("execution_generation", &self.execution_generation)
            .field("worker", &self.worker)
            .finish()
    }
}

/// Canonical immutable evidence produced by deterministic automated grading.
///
/// Construction is intentionally centralized so Memory and PostgreSQL receive
/// the exact same UTF-8 JSON bytes and digest. The value remains inside the
/// worker/persistence boundary; it is not a browser receipt. ASVS V1.5 and
/// V2.3: durable evidence is typed, canonical, and bound to one execution.
#[derive(Clone, PartialEq)]
pub struct CanonicalAttemptResult {
    pub result: AttemptResult,
    pub canonical_json_version: u16,
    pub canonical_json: String,
    pub sha256: Sha256Digest,
}

impl std::fmt::Debug for CanonicalAttemptResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CanonicalAttemptResult")
            .field("result", &"[SERVER-ONLY]")
            .field("canonical_json_version", &self.canonical_json_version)
            .field("canonical_json", &"[SERVER-ONLY]")
            .field("sha256", &self.sha256)
            .finish()
    }
}

/// Derives canonical evidence from one typed deterministic grading result.
pub fn canonical_attempt_result_json(
    result: AttemptResult,
) -> Result<CanonicalAttemptResult, StoreError> {
    let canonical = crate::canonical_json::canonical_json_bytes_v1("attempt result", &result)?;
    Ok(CanonicalAttemptResult {
        result,
        canonical_json_version: canonical.version,
        canonical_json: canonical.source,
        sha256: canonical.sha256,
    })
}

/// Server-private grade material returned by one accepted grading execution.
///
/// The feedback value deliberately remains outside browser contracts. Its
/// custom diagnostics marker keeps a worker failure from logging teaching
/// material or a Student's result evidence.
#[derive(Clone, PartialEq)]
pub struct AcceptedSubmissionGrade {
    pub evidence: CanonicalAttemptResult,
    pub feedback: question_model::FeedbackContent,
}

impl std::fmt::Debug for AcceptedSubmissionGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcceptedSubmissionGrade")
            .field("evidence", &"[CANONICAL RESULT]")
            .field("feedback", &"[SERVER-ONLY]")
            .finish()
    }
}

/// Closed worker outcome for one active accepted-submission claim.
#[derive(Debug, Clone, PartialEq)]
pub enum AcceptedSubmissionExecutionOutcome {
    Evaluated { grade: AcceptedSubmissionGrade },
    DeterministicFailure { reason: GradingOperationReason },
    TransientFailure,
    TimedOut,
    TerminalFailure,
}

/// Durable result of a commit-or-fail request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptedSubmissionExecutionDisposition {
    Committed,
    Rescheduled,
    Terminal,
    ClaimNoLongerActive,
}

/// Typed outcome of reloading immutable accepted work for an active lease.
///
/// `NotFound` and `Conflict` retain the established stale-claim meaning. A
/// failed private-input or issued-evidence verification is closed and safe to
/// route to `issued_evidence_integrity`; all other store failures remain
/// operational errors and keep their original handling.
#[derive(Debug, Clone, PartialEq)]
pub enum AcceptedSubmissionExecutionLoadError {
    NotFound,
    Conflict,
    IssuedEvidenceIntegrity,
    Store(StoreError),
}

impl std::fmt::Display for AcceptedSubmissionExecutionLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "accepted-submission execution is absent"),
            Self::Conflict => write!(formatter, "accepted-submission execution claim is stale"),
            Self::IssuedEvidenceIntegrity => {
                write!(
                    formatter,
                    "issued execution evidence failed integrity verification"
                )
            }
            Self::Store(error) => write!(
                formatter,
                "accepted-submission execution load failed: {error}"
            ),
        }
    }
}

impl std::error::Error for AcceptedSubmissionExecutionLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Store(error) => Some(error),
            Self::NotFound | Self::Conflict | Self::IssuedEvidenceIntegrity => None,
        }
    }
}

impl From<StoreError> for AcceptedSubmissionExecutionLoadError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::NotFound => Self::NotFound,
            StoreError::Conflict => Self::Conflict,
            error => Self::Store(error),
        }
    }
}

/// Result of a worker completion request whose acknowledgement may be uncertain.
///
/// `Known` preserves a failure that prevented the requested transition. `OutcomeUnknown`
/// means the final durable acknowledgement was lost after the request reached the store; callers
/// must inspect durable state rather than submit or commit again.
#[derive(Debug, Clone, PartialEq)]
pub enum AcceptedSubmissionCommitError {
    Known(StoreError),
    OutcomeUnknown,
}

impl std::fmt::Display for AcceptedSubmissionCommitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Known(error) => write!(formatter, "accepted-submission commit failed: {error}"),
            Self::OutcomeUnknown => write!(
                formatter,
                "accepted-submission commit acknowledgement is unknown; inspect durable state before retrying"
            ),
        }
    }
}

impl std::error::Error for AcceptedSubmissionCommitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Known(error) => Some(error),
            Self::OutcomeUnknown => None,
        }
    }
}

impl From<StoreError> for AcceptedSubmissionCommitError {
    fn from(error: StoreError) -> Self {
        Self::Known(error)
    }
}

/// Server-private reloaded work for the common automated-grading handler.
///
/// It combines the exact immutable accepted response with the issued grading
/// witnesses that were valid at issue time. Neither field is browser data.
#[derive(Clone, PartialEq)]
pub struct AcceptedSubmissionExecution {
    pub accepted: AcceptedSubmission,
    pub response: StudentResponse,
    pub prepared: Box<PreparedQuestionSubmission>,
}

impl std::fmt::Debug for AcceptedSubmissionExecution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcceptedSubmissionExecution")
            .field("accepted", &self.accepted)
            .field("response", &"[SERVER-ONLY]")
            .field("prepared", &"[SERVER-ONLY]")
            .finish()
    }
}

/// Append-only execution transition evidence without response or score data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradingExecutionReceipt {
    pub submission: AcceptedSubmissionId,
    pub generation: GradingExecutionGeneration,
    pub resulting_state: GradingExecutionState,
    pub safe_category: GradingExecutionReceiptSafeCategory,
    /// The authenticated Student or Instructor that initiated a ready
    /// execution generation. Exactly one of `actor` and `worker` is present.
    pub actor: Option<UserId>,
    /// The server worker that made a claimed, completed, retry, or exception
    /// transition. Exactly one of `actor` and `worker` is present.
    pub worker: Option<WorkerId>,
    pub occurred_at: ActivityTimestamp,
}

/// All data required to atomically accept input and create initial projections.
#[derive(Clone, PartialEq)]
pub struct AcceptedSubmissionCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub attempt: QuestionAttemptId,
    pub idempotency_key: SubmissionIdempotencyKey,
    /// Server-private accepted Student input. Browser DTOs never receive it.
    pub response: StudentResponse,
    pub execution_job: JobId,
}

impl std::fmt::Debug for AcceptedSubmissionCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcceptedSubmissionCommand")
            .field("actor", &self.actor)
            .field("course", &self.course)
            .field("assignment", &self.assignment)
            .field("attempt", &self.attempt)
            .field("idempotency_key", &"[REDACTED]")
            .field("response", &"[SERVER-ONLY]")
            .field("execution_job", &self.execution_job)
            .finish()
    }
}

impl AcceptedSubmissionCommand {
    /// Derives the immutable accepted-input digest from the canonical response
    /// bytes used by every store implementation.
    pub fn trusted_response_sha256(&self) -> Result<Sha256Digest, StoreError> {
        let canonical = canonical_student_response_json(&self.response)?;
        Ok(Sha256Digest::compute(canonical.as_bytes()))
    }
}

/// Serializes one typed closed response for immutable accepted-input storage.
///
/// This is the sole response representation used for an accepted-input
/// digest. PostgreSQL receives these exact UTF-8 bytes, validates their JSON
/// shape in the broker, and stores the parsed `jsonb` separately. Keeping the
/// serialization here prevents an in-memory path, an SQL adapter, or a replay
/// check from silently choosing a different normal form. ASVS 1.5.3 and
/// 2.3.1: one typed value has one durable replay identity.
pub fn canonical_student_response_json(response: &StudentResponse) -> Result<String, StoreError> {
    serde_json::to_string(response).map_err(|error| {
        StoreError::InvalidRecord(format!("accepted response serialization failed: {error}"))
    })
}

/// Automated persistence owns accepted-submission scoring without a second mutation path.
#[async_trait]
pub trait AutomatedGradingStore: Send + Sync {
    async fn accept_automated_submission(
        &self,
        context: TenantContext,
        command: AcceptedSubmissionCommand,
    ) -> Result<AcceptedSubmission, StoreError>;
    async fn automated_grading_execution(
        &self,
        context: TenantContext,
        submission: AcceptedSubmissionId,
    ) -> Result<Option<GradingExecution>, StoreError>;
    async fn record_automated_grading_execution_receipt(
        &self,
        context: TenantContext,
        receipt: GradingExecutionReceipt,
        resulting_evaluation: SubmissionEvaluationStatus,
    ) -> Result<(), StoreError>;
}

/// Server-private capability for lease-bound accepted-input reload and its
/// single durable completion transition.
///
/// Claim selection is deliberately outside this trait.  Both recovery and the
/// exact synchronous path hand the same already-won claim to this shared
/// execution capability, preserving one load/commit state machine.
#[async_trait]
pub trait AcceptedSubmissionExecutionStore: Send + Sync {
    async fn load_accepted_submission_for_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
    ) -> Result<AcceptedSubmissionExecution, AcceptedSubmissionExecutionLoadError>;

    async fn commit_or_fail_accepted_submission_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
        outcome: AcceptedSubmissionExecutionOutcome,
    ) -> Result<AcceptedSubmissionExecutionDisposition, AcceptedSubmissionCommitError>;
}

/// Recovery-worker authority to select the next eligible accepted submission.
///
/// This capability has no private-input loader or completion operation.  It
/// supplies one claim for the shared execution capability to process.
#[async_trait]
pub trait AcceptedSubmissionExecutionRecoveryClaimStore: Send + Sync {
    async fn claim_next_accepted_submission_execution(
        &self,
        worker: WorkerId,
        lease: crate::JobLeaseDuration,
    ) -> Result<Option<AcceptedSubmissionExecutionClaim>, StoreError>;
}

/// Exact-claim authority used by the synchronous accepted-submission path.
///
/// The target and process identity are explicit so a browser request cannot
/// select another Student's work.  The same eligible-state transition used by
/// recovery decides whether this call wins (ASVS V2.3.1 and V2.3.4).
#[async_trait]
pub trait AcceptedSubmissionExecutionFastPathClaimStore: Send + Sync {
    async fn claim_exact_accepted_submission_execution(
        &self,
        target: AcceptedSubmissionExecutionTarget,
        worker: WorkerId,
        lease: crate::JobLeaseDuration,
    ) -> Result<Option<AcceptedSubmissionExecutionClaim>, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::{
        AcceptedSubmissionCommand, AcceptedSubmissionCommitError, CanonicalAttemptResult,
        GradingExecutionGeneration, GradingOperationRevision, canonical_attempt_result_json,
        canonical_student_response_json,
    };
    use objects::Sha256Digest;
    use question_model::response::{ChoiceId, TextEntryAnswer};
    use question_model::{AssignmentId, CourseId, QuestionAttemptId, StudentResponse, UserId};
    use uuid::Uuid;

    use crate::{JobId, StoreError, SubmissionIdempotencyKey};

    #[test]
    fn commit_error_preserves_known_failures_and_redacts_unknown_outcomes() {
        assert_eq!(
            AcceptedSubmissionCommitError::from(StoreError::Conflict),
            AcceptedSubmissionCommitError::Known(StoreError::Conflict)
        );
        assert_eq!(
            AcceptedSubmissionCommitError::OutcomeUnknown.to_string(),
            "accepted-submission commit acknowledgement is unknown; inspect durable state before retrying"
        );
    }

    #[test]
    fn generation_and_revision_reject_zero() {
        assert_eq!(GradingExecutionGeneration::INITIAL.as_u64(), 1);
        assert_eq!(GradingOperationRevision::INITIAL.as_u64(), 1);
        assert!(GradingExecutionGeneration::from_u64(0).is_none());
        assert!(GradingOperationRevision::from_u64(0).is_none());
    }

    #[test]
    fn trusted_digest_is_derived_from_response() {
        let command = AcceptedSubmissionCommand {
            actor: UserId::from_uuid(Uuid::from_u128(1)),
            course: CourseId::from_uuid(Uuid::from_u128(2)),
            assignment: AssignmentId::from_uuid(Uuid::from_u128(3)),
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(4)),
            idempotency_key: SubmissionIdempotencyKey::parse("submission-4").expect("key"),
            response: StudentResponse::Numeric { value: 4.0 },
            execution_job: JobId::from_uuid(Uuid::from_u128(5)),
        };
        let expected = Sha256Digest::compute(
            canonical_student_response_json(&StudentResponse::Numeric { value: 4.0 })
                .expect("response")
                .as_bytes(),
        );
        assert_eq!(command.trusted_response_sha256().expect("digest"), expected);
    }

    #[test]
    fn canonical_response_text_and_digest_are_stable_for_closed_variants() {
        let responses = [
            StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("alpha"), ChoiceId::new("beta")],
            },
            StudentResponse::MultiBlank {
                answers: vec![
                    TextEntryAnswer {
                        slot: ChoiceId::new("protein"),
                        text: "collagen".to_string(),
                    },
                    TextEntryAnswer {
                        slot: ChoiceId::new("signal"),
                        text: "phosphorylation".to_string(),
                    },
                ],
            },
        ];

        for response in responses {
            let first = canonical_student_response_json(&response).expect("canonical response");
            let second = canonical_student_response_json(&response).expect("canonical response");
            assert_eq!(first, second);
            assert_eq!(
                Sha256Digest::compute(first.as_bytes()),
                Sha256Digest::compute(second.as_bytes())
            );
        }
    }

    #[test]
    fn command_debug_redacts_replay_key_and_response() {
        let command = AcceptedSubmissionCommand {
            actor: UserId::from_uuid(Uuid::from_u128(1)),
            course: CourseId::from_uuid(Uuid::from_u128(2)),
            assignment: AssignmentId::from_uuid(Uuid::from_u128(3)),
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(4)),
            idempotency_key: SubmissionIdempotencyKey::parse("replay-key-private").expect("key"),
            response: StudentResponse::Numeric { value: 919.0 },
            execution_job: JobId::from_uuid(Uuid::from_u128(5)),
        };

        let debug = format!("{command:?}");
        assert!(!debug.contains("replay-key-private"));
        assert!(!debug.contains("919"));
        assert!(debug.contains("[REDACTED]"));
        assert!(debug.contains("[SERVER-ONLY]"));
    }

    #[test]
    fn canonical_attempt_result_uses_exact_utf8_and_redacts_diagnostics() {
        let evidence = canonical_attempt_result_json(question_model::AttemptResult {
            correct: true,
            points_earned: 8.5,
            points_possible: 10.0,
        })
        .expect("closed result serializes");
        assert_eq!(
            evidence.canonical_json,
            r#"{"correct":true,"pointsEarned":8.5,"pointsPossible":10.0}"#
        );
        assert_eq!(evidence.canonical_json_version, 1);
        assert_eq!(
            evidence.sha256,
            Sha256Digest::compute(evidence.canonical_json.as_bytes())
        );
        let debug = format!("{evidence:?}");
        assert!(!debug.contains("8.5"));
        assert!(!debug.contains("pointsEarned"));
        assert!(debug.contains("[SERVER-ONLY]"));
        let copied = CanonicalAttemptResult {
            result: evidence.result,
            canonical_json_version: evidence.canonical_json_version,
            canonical_json: evidence.canonical_json.clone(),
            sha256: evidence.sha256,
        };
        assert_eq!(copied, evidence);
    }
}
