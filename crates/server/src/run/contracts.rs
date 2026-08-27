//! Trusted backend contract for assignment-run issuance and grading.
//!
//! Route capabilities depend on these shared server-only types directly. The
//! parent `run` module re-exports the public API for callers outside this
//! module tree without becoming an internal dependency of its children.

use async_trait::async_trait;
use grading::GradeOutcome;
use learning_data_access::{
    FlatGradingCapability, IssuedFlatGradingContract, IssuedQtiGradingContractV1,
    IssuedQuestionSnapshotV1, IssuedWebworkGradingContract, QtiGradingCapability,
    SubmissionIdempotencyKey, SubmissionRecord, TenantContext, WebworkGradeReplayStateV1,
    WebworkGradingCapability,
};
use question_model::{
    AttemptProvenance, AttemptResult, FeedbackContent, GradingOperationReason, ProblemVersionRef,
    QuestionAttempt, QuestionDefinition, QuestionEnvelope, StudentResponse,
};
use std::sync::Arc;

/// Server-only metadata produced while a trusted adapter issues one instance.
///
/// The rendered envelope and provenance are answer-free, but native flat
/// questions also carry the protected grading contract that must never enter
/// a browser DTO or a public cache.
#[derive(Clone, PartialEq)]
pub struct IssuedAttemptMetadata {
    /// The key-free rendered envelope prepared by the trusted backend.
    pub envelope: QuestionEnvelope,
    /// SHA-256 of generated parameter values.
    pub parameter_hash: String,
    /// Complete reproducibility record without an answer or key.
    ///
    /// The backend owns the canonical rendered artifact covered by
    /// `rendered_question_sha256`. For example, WeBWorK includes its
    /// sanitized renderer markup in addition to the shared envelope.
    pub provenance: AttemptProvenance,
    /// Private answer-free WeBWorK controls keyed by durable item identity.
    ///
    /// The run issuance path converts these to presentation-scoped rendered
    /// IDs before persistence. This value has no serialization surface.
    pub webwork_replay: Option<adapter_webwork::renderer_contract::WebworkReplayMappingV1>,
    /// Server-only flat-question definition/key pair frozen at issue. It is
    /// absent for every other backend family.
    pub flat_grading: Option<IssuedFlatGradingContract>,
    /// Immutable obligation for private flat grading. Receipt readers never
    /// infer it from whether the private payload happens to be present.
    pub flat_grading_capability: FlatGradingCapability,
    /// Server-only WebWork definition frozen at issue for first grading.
    pub webwork_grading: Option<IssuedWebworkGradingContract>,
    /// Explicit obligation for the frozen WebWork definition.
    pub webwork_grading_capability: WebworkGradingCapability,
    /// Server-only QTI private contract frozen at issue.  It is never a
    /// browser DTO and first grade consumes this exact issued evidence.
    pub qti_grading: Option<IssuedQtiGradingContractV1>,
    /// Explicit obligation for the frozen QTI contract.
    pub qti_grading_capability: QtiGradingCapability,
}

impl std::fmt::Debug for IssuedAttemptMetadata {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedAttemptMetadata")
            .field("envelope", &self.envelope)
            .field("parameter_hash", &self.parameter_hash)
            .field("provenance", &self.provenance)
            .field(
                "webwork_replay",
                &self.webwork_replay.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "flat_grading",
                &self.flat_grading.as_ref().map(|_| "[REDACTED]"),
            )
            .field("flat_grading_capability", &self.flat_grading_capability)
            .field(
                "webwork_grading",
                &self.webwork_grading.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "webwork_grading_capability",
                &self.webwork_grading_capability,
            )
            .field(
                "qti_grading",
                &self.qti_grading.as_ref().map(|_| "[REDACTED]"),
            )
            .field("qti_grading_capability", &self.qti_grading_capability)
            .finish()
    }
}

/// The durable disposition of a response chosen by its trusted backend.
///
/// Most backends return [`Self::Grade`] and let the generic run route persist
/// it. A server-mediated external tool instead owns an all-or-nothing broker
/// transaction and returns [`Self::Committed`]. Keeping that distinction in
/// this server-only seam prevents a provider grade from being observed before
/// its attempt record is durably committed.
#[derive(Clone, PartialEq)]
pub enum SubmissionDisposition {
    /// A normal server-only grade that the generic attempt store must commit.
    Grade(GradeReceipt),
    /// A valid response whose trusted backend requires an instructor's
    /// server-side evaluation before a numeric result exists.
    NeedsManualGrading,
    /// A record already atomically committed by a backend-owned broker.
    Committed(Box<SubmissionRecord>),
}

impl std::fmt::Debug for SubmissionDisposition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Grade(_) => formatter.debug_tuple("Grade").field(&"[redacted]").finish(),
            Self::NeedsManualGrading => formatter.write_str("NeedsManualGrading"),
            Self::Committed(record) => formatter.debug_tuple("Committed").field(record).finish(),
        }
    }
}

/// One server-only grade and its private, sanitized teaching material.
///
/// This deliberately has no wire traits and no debug implementation: answer
/// keys and feedback live only long enough to enter the trusted store command.
#[derive(Clone, PartialEq)]
pub struct GradeReceipt {
    pub result: AttemptResult,
    pub feedback: FeedbackContent,
}

impl GradeReceipt {
    pub fn empty(result: AttemptResult) -> Self {
        Self {
            result,
            feedback: FeedbackContent::default(),
        }
    }
}

/// Complete trusted input to a backend-owned submission transition.
///
/// This is intentionally constructed only after the route has authenticated
/// the actor, completed replay lookup, loaded the tenant-visible attempt, and
/// validated the browser response shape.
pub struct RunSubmission<'a> {
    pub context: TenantContext,
    pub actor: question_model::UserId,
    pub idempotency_key: SubmissionIdempotencyKey,
    pub reference: ProblemVersionRef,
    /// Checksummed definition and family witness retained at issuance. This
    /// is the only definition authority available to first-grade backends.
    pub issued_question_snapshot: &'a IssuedQuestionSnapshotV1,
    pub attempt: &'a QuestionAttempt,
    /// Server-only answer-free envelope frozen with this issued attempt.
    /// Presentation-bearing families use it to translate public rendered IDs
    /// before grading without consulting a mutable renderer or catalog view.
    pub issued_grading_envelope: Option<&'a QuestionEnvelope>,
    /// Private flat-question definition/key pair retained with the attempt.
    /// This is the sole flat first-grade authority after issue.
    pub issued_flat_grading: Option<&'a IssuedFlatGradingContract>,
    /// Immutable WebWork definition retained with the attempt. It supplies
    /// the first-grade source path and policy without a current catalog read.
    pub issued_webwork_grading: Option<&'a IssuedWebworkGradingContract>,
    /// Private QTI contract copied at issuance.  This is the only QTI
    /// first-grade authority; no current catalog or source lookup is valid.
    pub issued_qti_grading: Option<&'a IssuedQtiGradingContractV1>,
    /// Exact attempt-bound private WeBWorK replay state from preparation.
    pub issued_webwork_replay: Option<&'a WebworkGradeReplayStateV1>,
    /// Presentation binding from the same coherent preparation snapshot.
    pub issued_presentation_binding: Option<question_model::PresentationBindingV1>,
    /// Answer-free presentation snapshot from that preparation snapshot.
    pub issued_presentation: Option<&'a learning_data_access::ReceiptPresentationSnapshot>,
    pub response: &'a StudentResponse,
}

impl RunSubmission<'_> {
    /// Projects the immutable definition from the required issued evidence.
    pub fn question(&self) -> &QuestionDefinition {
        self.issued_question_snapshot.question()
    }
}

/// Adapter-owned server boundary used by the generic run routes.
#[async_trait]
pub trait RunBackend: Send + Sync {
    /// Generates or renders one fresh instance from the server-owned seed.
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError>;

    /// Rebuilds a key-free envelope for an envelope-less active family.
    ///
    /// Presentation-bearing attempt GET and submission format validation read
    /// their owned issuance snapshot instead. Implementations must therefore
    /// never reintroduce this mutable reconstruction on a receipt path.
    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError>;

    /// Confirms that this exact issued external-tool attempt may use the
    /// server-owned launch broker. It deliberately returns no provider data.
    async fn prepare_external_tool_launch(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
        _question: &QuestionDefinition,
        _attempt: &QuestionAttempt,
    ) -> Result<(), RunBackendError> {
        Err(RunBackendError::Unsupported(
            "this question backend does not provide an external-tool launch".to_string(),
        ))
    }

    /// Grades one response without returning or serializing its key.
    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError>;

    /// Submits one response after route-level replay and format validation.
    ///
    /// The actor and idempotency key deliberately cross this boundary so an
    /// external backend can bind its provider exchange to the exact tenant
    /// record that will be committed. The default preserves the ordinary
    /// Native and WeBWorK grade-then-store behavior.
    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        let _ = (submission.actor, &submission.idempotency_key);
        self.grade(
            submission.context,
            submission.reference,
            submission.question(),
            submission.attempt,
            submission.response,
        )
        .await
        .and_then(|outcome| match outcome {
            GradeOutcome::Graded(result) => {
                Ok(SubmissionDisposition::Grade(GradeReceipt::empty(result)))
            }
            GradeOutcome::NeedsManualGrading => Ok(SubmissionDisposition::NeedsManualGrading),
            GradeOutcome::Ungraded => Err(RunBackendError::Unsupported(
                "this run backend does not produce a server grade".to_string(),
            )),
        })
    }
}

/// Shares one configured backend between request routing and a server-owned
/// execution worker without introducing a second backend configuration.
///
/// The worker and router both need the same immutable adapter capabilities and
/// therefore should observe one handle. Keeping this forwarding implementation
/// on the trusted backend contract makes that composition explicit while
/// preserving the trait's `Send + Sync` boundary.
#[async_trait]
impl<B> RunBackend for Arc<B>
where
    B: RunBackend + ?Sized,
{
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        self.as_ref()
            .issue(context, reference, question, seed)
            .await
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        self.as_ref()
            .reproduce(context, reference, question, attempt)
            .await
    }

    async fn prepare_external_tool_launch(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<(), RunBackendError> {
        self.as_ref()
            .prepare_external_tool_launch(context, reference, question, attempt)
            .await
    }

    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError> {
        self.as_ref()
            .grade(context, reference, question, attempt, response)
            .await
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        self.as_ref().submit(submission).await
    }
}

/// Closed deterministic failure category from a trusted grader.
///
/// This contains no backend detail because callers may safely convert it to an
/// Instructor-facing operation reason after durable accepted input exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterministicGraderFailure {
    Contract,
    Execution,
    IssuedEvidenceIntegrity,
}

impl DeterministicGraderFailure {
    pub const fn operation_reason(self) -> GradingOperationReason {
        match self {
            Self::Contract => GradingOperationReason::GraderContractFailure,
            Self::Execution => GradingOperationReason::GraderExecutionFailure,
            Self::IssuedEvidenceIntegrity => GradingOperationReason::IssuedEvidenceIntegrity,
        }
    }
}

/// Failure from the selected trusted adapter or grading implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunBackendError {
    /// The selected backend does not implement the requested behavior.
    Unsupported(String),
    /// The published definition or private backend material is invalid.
    Invalid(String),
    /// A renderer or backend dependency is temporarily unavailable.
    Unavailable(String),
    /// A post-validation deterministic grader failure with a safe closed category.
    Deterministic(DeterministicGraderFailure),
}

impl std::fmt::Display for RunBackendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(message) => write!(formatter, "unsupported run behavior: {message}"),
            Self::Invalid(message) => write!(formatter, "invalid run backend data: {message}"),
            Self::Unavailable(message) => write!(formatter, "run backend unavailable: {message}"),
            Self::Deterministic(failure) => {
                write!(formatter, "deterministic grader failure: {failure:?}")
            }
        }
    }
}

impl std::error::Error for RunBackendError {}

#[cfg(test)]
mod tests {
    use super::{DeterministicGraderFailure, GradingOperationReason};

    #[test]
    fn deterministic_grader_failures_have_only_safe_operation_reasons() {
        assert_eq!(
            DeterministicGraderFailure::Contract.operation_reason(),
            GradingOperationReason::GraderContractFailure
        );
        assert_eq!(
            DeterministicGraderFailure::Execution.operation_reason(),
            GradingOperationReason::GraderExecutionFailure
        );
        assert_eq!(
            DeterministicGraderFailure::IssuedEvidenceIntegrity.operation_reason(),
            GradingOperationReason::IssuedEvidenceIntegrity
        );
    }
}
