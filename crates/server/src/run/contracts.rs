//! Trusted backend contract for assignment-run issuance and grading.
//!
//! Route capabilities depend on these shared server-only types directly. The
//! parent `run` module re-exports the public API for callers outside this
//! module tree without becoming an internal dependency of its children.

use async_trait::async_trait;
use grading::GradeOutcome;
use learning_data_access::{SubmissionIdempotencyKey, SubmissionRecord, TenantContext};
use question_model::{
    AttemptProvenance, AttemptResult, FeedbackContent, ProblemVersionRef, QuestionAttempt,
    QuestionDefinition, QuestionEnvelope, StudentResponse,
};

/// Key-free metadata produced while a trusted adapter issues one instance.
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
    pub question: &'a QuestionDefinition,
    pub attempt: &'a QuestionAttempt,
    pub response: &'a StudentResponse,
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

    /// Rebuilds the exact key-free envelope that was issued for an attempt.
    ///
    /// The backend verifies the persisted seed, parameter hash, provenance,
    /// and immutable version before this browser-facing representation leaves
    /// the server.
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
            submission.question,
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

/// Failure from the selected trusted adapter or grading implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunBackendError {
    /// The selected backend does not implement the requested behavior.
    Unsupported(String),
    /// The published definition or private backend material is invalid.
    Invalid(String),
    /// A renderer or backend dependency is temporarily unavailable.
    Unavailable(String),
}

impl std::fmt::Display for RunBackendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(message) => write!(formatter, "unsupported run behavior: {message}"),
            Self::Invalid(message) => write!(formatter, "invalid run backend data: {message}"),
            Self::Unavailable(message) => write!(formatter, "run backend unavailable: {message}"),
        }
    }
}

impl std::error::Error for RunBackendError {}
