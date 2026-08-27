//! Server-only issue, prefetch, and first-grade contracts.

use super::*;

/// Server-owned data needed to issue or resume one question instance.
#[derive(Debug, Clone, PartialEq)]
pub struct IssueQuestionAttemptCommand {
    pub actor: UserId,
    pub binding: LearnerWorkRoutingBinding,
    pub attempt: QuestionAttemptId,
    pub run: RunId,
    pub assignment_position: u32,
    pub problem: ProblemId,
    pub question_version: VersionId,
    /// Complete immutable source/execution evidence fixed at issuance.
    pub issued_question_snapshot: IssuedQuestionSnapshotV1,
    pub seed: u64,
    pub parameter_hash: String,
    pub provenance: AttemptProvenance,
    pub presentation_capability: PresentationCapability,
    pub presentation: Option<PresentationBindingV1>,
    pub presentation_snapshot: Option<ReceiptPresentationSnapshot>,
    pub grading_envelope: Option<QuestionEnvelope>,
    pub native_execution_envelope_capability: NativeExecutionEnvelopeCapability,
    pub flat_grading: Option<crate::IssuedFlatGradingContract>,
    pub flat_grading_capability: FlatGradingCapability,
    pub webwork_replay: Option<WebworkReplayMappingV1>,
    pub webwork_grading: Option<IssuedWebworkGradingContract>,
    pub webwork_grading_capability: WebworkGradingCapability,
    pub qti_grading: Option<IssuedQtiGradingContractV1>,
    pub qti_grading_capability: QtiGradingCapability,
    pub prefetched: Option<PrefetchedQuestionDescriptorV1>,
    pub predecessor_submission: Option<QuestionAttemptId>,
}

/// Immutable successor state for one committed submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmissionNextAttempt {
    Pending,
    None,
    Issued(ReceiptNextAttempt),
}

/// Immutable presentation obligation selected at issue time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PresentationCapability {
    EnvelopeV1,
    NotApplicable,
}

/// Immutable server-only execution-envelope obligation for non-flat native
/// items. This is intentionally separate from browser presentation capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeExecutionEnvelopeCapability {
    Required,
    NotApplicable,
}

impl PresentationCapability {
    #[cfg(feature = "postgres")]
    pub(crate) fn requires_snapshot(self) -> bool {
        matches!(self, Self::EnvelopeV1)
    }
}

/// Immutable private-flat-grading obligation selected at issue time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlatGradingCapability {
    Required,
    NotApplicable,
}

impl FlatGradingCapability {
    pub(crate) fn requires_contract(self) -> bool {
        matches!(self, Self::Required)
    }
}

/// Immutable WeBWorK private-grading obligation selected at issue time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WebworkGradingCapability {
    Required,
    NotApplicable,
}

/// Immutable private-QTI-grading obligation selected at issue time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QtiGradingCapability {
    Required,
    NotApplicable,
}

impl QtiGradingCapability {
    pub(crate) fn requires_contract(self) -> bool {
        matches!(self, Self::Required)
    }
}

impl WebworkGradingCapability {
    pub(crate) fn requires_contract(self) -> bool {
        matches!(self, Self::Required)
    }
}

/// Derives the checksummed compact issued capability tag.
pub(crate) fn issued_attempt_capability_from_issue(
    presentation: PresentationCapability,
    flat_grading: FlatGradingCapability,
    webwork_grading: WebworkGradingCapability,
    qti_grading: QtiGradingCapability,
) -> Result<question_model::IssuedAttemptCapabilityV1, StoreError> {
    use question_model::IssuedAttemptCapabilityV1 as Capability;

    match (presentation, flat_grading, webwork_grading, qti_grading) {
        (
            PresentationCapability::EnvelopeV1,
            FlatGradingCapability::Required,
            WebworkGradingCapability::NotApplicable,
            QtiGradingCapability::NotApplicable,
        ) => Ok(Capability::FlatPresentation),
        (
            PresentationCapability::EnvelopeV1,
            FlatGradingCapability::NotApplicable,
            WebworkGradingCapability::Required,
            QtiGradingCapability::NotApplicable,
        ) => Ok(Capability::WebworkPresentation),
        (
            PresentationCapability::EnvelopeV1,
            FlatGradingCapability::NotApplicable,
            WebworkGradingCapability::NotApplicable,
            QtiGradingCapability::Required,
        ) => Ok(Capability::QtiPresentation),
        (
            PresentationCapability::EnvelopeV1,
            FlatGradingCapability::NotApplicable,
            WebworkGradingCapability::NotApplicable,
            QtiGradingCapability::NotApplicable,
        ) => Ok(Capability::PresentationEnvelope),
        (
            PresentationCapability::NotApplicable,
            FlatGradingCapability::NotApplicable,
            WebworkGradingCapability::NotApplicable,
            QtiGradingCapability::NotApplicable,
        ) => Ok(Capability::NotApplicable),
        _ => Err(StoreError::InvalidRecord(
            "issued presentation and grading capabilities disagree".to_string(),
        )),
    }
}

/// Refuses protected-column damage that could otherwise prompt catalog recovery.
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn validate_attempt_issuance_capability(
    attempt: &QuestionAttempt,
    presentation: PresentationCapability,
    flat_grading: FlatGradingCapability,
    webwork_grading: WebworkGradingCapability,
    qti_grading: QtiGradingCapability,
) -> Result<(), StoreError> {
    let expected = issued_attempt_capability_from_issue(
        presentation,
        flat_grading,
        webwork_grading,
        qti_grading,
    )
    .map_err(|error| StoreError::Unavailable(error.to_string()))?;
    if attempt.issued_capability != expected {
        return Err(StoreError::Unavailable(
            "stored issuance capability disagrees with its checksummed attempt".to_string(),
        ));
    }
    Ok(())
}

/// Browser-safe successor metadata frozen with the predecessor receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptNextAttempt {
    pub id: QuestionAttemptId,
    pub run: RunId,
    pub question_version: VersionId,
    pub seed: u64,
    pub deadline: Option<ActivityTimestamp>,
    pub assignment_position: u32,
    pub rendered_question_sha256: String,
}

impl ReceiptNextAttempt {
    pub(crate) fn from_attempt(attempt: &QuestionAttempt) -> Self {
        Self {
            id: attempt.id,
            run: attempt.run,
            question_version: attempt.question_version,
            seed: attempt.seed,
            deadline: attempt.timer.deadline,
            assignment_position: attempt.assignment_position,
            rendered_question_sha256: attempt.provenance.rendered_question_sha256.clone(),
        }
    }
}

/// Serializable, non-secret reservation descriptor returned by ordinary Store
/// lookup methods. It deliberately never contains an answer-bearing family
/// contract or replay mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefetchedQuestionDescriptorV1 {
    pub tenant: TenantId,
    pub run: RunId,
    pub predecessor: QuestionAttemptId,
    pub assignment_position: u32,
    pub problem: ProblemId,
    pub question_version: VersionId,
    /// Exact V1 evidence promoted byte-for-byte into the real attempt.
    pub issued_question_snapshot: IssuedQuestionSnapshotV1,
    pub seed: u64,
    pub parameter_hash: String,
    pub provenance: AttemptProvenance,
    pub presentation_capability: PresentationCapability,
    pub presentation: PresentationBindingV1,
    pub presentation_snapshot: ReceiptPresentationSnapshot,
    pub grading_envelope: QuestionEnvelope,
    pub native_execution_envelope_capability: NativeExecutionEnvelopeCapability,
    pub flat_grading_capability: FlatGradingCapability,
    pub webwork_grading_capability: WebworkGradingCapability,
    pub qti_grading_capability: QtiGradingCapability,
}

/// Private reservation execution authority, deliberately nonserializable and
/// redacted. It is stored separately from the ordinary descriptor.
#[derive(Clone, PartialEq)]
pub struct PrefetchedPrivateExecutionV1 {
    pub flat_grading: Option<crate::IssuedFlatGradingContract>,
    pub webwork_replay: Option<WebworkReplayMappingV1>,
    pub webwork_grading: Option<IssuedWebworkGradingContract>,
    pub qti_grading: Option<IssuedQtiGradingContractV1>,
}

impl std::fmt::Debug for PrefetchedPrivateExecutionV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrefetchedPrivateExecutionV1")
            .field("flat_grading", &"[REDACTED]")
            .field("webwork_replay", &"[REDACTED]")
            .field("webwork_grading", &"[REDACTED]")
            .field("qti_grading", &"[REDACTED]")
            .finish()
    }
}

/// Trusted server request to create or resume a prefetch reservation.
#[derive(Debug, Clone, PartialEq)]
pub struct ReservePrefetchedQuestionCommand {
    pub actor: UserId,
    pub binding: LearnerWorkRoutingBinding,
    pub reservation: PrefetchedQuestionDescriptorV1,
    pub private_execution: PrefetchedPrivateExecutionV1,
}

/// Trusted server result to persist for one student response.
#[derive(Clone, PartialEq)]
pub struct SubmitQuestionAttemptCommand {
    pub actor: UserId,
    pub binding: LearnerWorkRoutingBinding,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub result: AttemptResult,
    pub feedback: FeedbackContent,
    pub idempotency_key: SubmissionIdempotencyKey,
}

/// Complete owned server-only context for grading one prepared response.
#[derive(Clone, PartialEq)]
pub struct PreparedQuestionSubmission {
    pub attempt: QuestionAttempt,
    pub issued_question_snapshot: IssuedQuestionSnapshotV1,
    pub presentation_binding: Option<PresentationBindingV1>,
    pub presentation: Option<ReceiptPresentationSnapshot>,
    pub grading_envelope: Option<QuestionEnvelope>,
    pub flat_grading: Option<crate::IssuedFlatGradingContract>,
    pub webwork_grading: Option<IssuedWebworkGradingContract>,
    pub issued_qti_grading: Option<IssuedQtiGradingContractV1>,
    pub webwork_replay: Option<WebworkGradeReplayStateV1>,
}

impl PreparedQuestionSubmission {
    /// The one immutable definition retained with this issued attempt.
    ///
    /// Keeping this as a projection prevents a prepared submission from
    /// carrying a second mutable definition that can diverge from its
    /// checksummed evidence.
    pub fn question(&self) -> &QuestionDefinition {
        self.issued_question_snapshot.question()
    }
}

impl std::fmt::Debug for PreparedQuestionSubmission {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedQuestionSubmission")
            .field("attempt", &self.attempt)
            .field("issued_question_snapshot", &"[SERVER-ONLY]")
            .field("presentation_binding", &self.presentation_binding)
            .field("presentation", &self.presentation)
            .field("grading_envelope", &"[SERVER-ONLY]")
            .field("flat_grading", &"[REDACTED]")
            .field("webwork_grading", &"[SERVER-ONLY]")
            .field("issued_qti_grading", &"[REDACTED]")
            .field("webwork_replay", &"[REDACTED]")
            .finish()
    }
}

/// Answer-free first-effect intent.  This is the only fresh result exposed by
/// the ordinary application Store: it proves the locked route/attempt shape,
/// but intentionally cannot construct a grader invocation.
#[derive(Clone, PartialEq)]
pub struct AuthorizedSubmissionIntent {
    pub attempt: QuestionAttempt,
    pub issued_question_snapshot: IssuedQuestionSnapshotV1,
    pub presentation_binding: Option<PresentationBindingV1>,
    pub presentation: Option<ReceiptPresentationSnapshot>,
    pub grading_envelope: Option<QuestionEnvelope>,
}

impl std::fmt::Debug for AuthorizedSubmissionIntent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthorizedSubmissionIntent")
            .field("attempt", &self.attempt)
            .field("issued_question_snapshot", &"[SERVER-ONLY]")
            .field("presentation_binding", &self.presentation_binding)
            .field("presentation", &self.presentation)
            .field("grading_envelope", &"[SERVER-ONLY]")
            .finish()
    }
}

/// The sealed grader-only result.  It is deliberately a distinct closed
/// union from [`SubmissionPreparation`], so a browser-adjacent Store result
/// cannot be reused as private grading authority.
#[derive(Debug, Clone, PartialEq)]
pub enum SealedPrivateExecutionPreparation {
    Replay(Box<SubmissionRecord>),
    Grade(Box<PreparedQuestionSubmission>),
}

/// Replay-or-first-effect result of ordinary, answer-free authorization.
#[derive(Debug, Clone, PartialEq)]
pub enum SubmissionPreparation {
    Replay(Box<SubmissionRecord>),
    /// The exact request is already durable and awaits its server-owned
    /// grading execution. This public-safe state deliberately contains no
    /// response, idempotency credential, feedback, or grade.
    AcceptedPending(AcceptedSubmissionPending),
    FirstEffect(Box<AuthorizedSubmissionIntent>),
}

/// Metadata-only proof that a learner submission has been accepted but has no
/// completed receipt yet.
///
/// The opaque attempt identity lets the owning route bind its current
/// projection to the request it already authorized. It is not a grading
/// capability and intentionally carries no response or replay credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptedSubmissionPending {
    attempt: QuestionAttemptId,
}

impl AcceptedSubmissionPending {
    pub(crate) const fn new(attempt: QuestionAttemptId) -> Self {
        Self { attempt }
    }

    /// Returns the attempt whose immutable accepted input is pending.
    pub const fn attempt(self) -> QuestionAttemptId {
        self.attempt
    }
}

/// Exact receipt-read state for an owned attempt.
///
/// A missing submission, an accepted pending input, and a completed receipt
/// are distinct durable states. Callers must not decode the accepted v2
/// response payload as a completed [`QuestionAttempt`].
#[derive(Debug, Clone, PartialEq)]
pub enum SubmissionReceiptRead {
    Missing,
    AcceptedPending(AcceptedSubmissionPending),
    Completed(Box<SubmissionRecord>),
}

/// Answer-free learner status for an owned automated submission.
///
/// The route-bound store capability establishes current learner authority and
/// validates the durable execution/evaluation/receipt aggregate before it
/// returns this closed projection.  Pending and attention states deliberately
/// carry only the opaque route attempt, so no response, job, execution,
/// feedback, result, reason, or score can cross this persistence boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum LearnerSubmissionStatusRead {
    Completed(Box<SubmissionRecord>),
    AcceptedPending(AcceptedSubmissionPending),
    InstructorAttention(AcceptedSubmissionPending),
}
