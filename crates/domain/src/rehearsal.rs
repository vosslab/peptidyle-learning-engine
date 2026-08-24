//! Pure WP-PROF-T4 rehearsal lifecycle, authorization-bound evidence, and integrity rules.

use sha2::{Digest, Sha256};

use crate::validation::{ResponseFormatViolation, validate_response_format};
use question_model::envelope::ContentBlock;
use question_model::{
    AssignmentReference, CourseId, CourseMembershipId, DisclosedFeedback, PreviewSubject,
    RehearsalEvidenceDigest, RehearsalEvidenceKind, RehearsalEvidenceRecord,
    RehearsalEvidenceValidationError, RehearsalFrozenItemEvidence, RehearsalLifecycle,
    RehearsalPrivateGradingResult, RehearsalRunId, TeachingOperationRevision, TenantId,
};

const SUBJECT_FINGERPRINT_DOMAIN: &[u8] = b"ple:rehearsal:subject:v1\0";
const EVIDENCE_GENESIS_DOMAIN: &[u8] = b"ple:rehearsal:evidence:genesis:v2\0";
const EVIDENCE_PAYLOAD_DOMAIN: &[u8] = b"ple:rehearsal:evidence:payload:v2\0";
const EVIDENCE_ENTRY_DOMAIN: &[u8] = b"ple:rehearsal:evidence:entry:v3\0";
const FROZEN_RESPONSE_SCHEMA_DOMAIN: &[u8] = b"ple:rehearsal:frozen-response-schema:v1\0";
const SUBMISSION_REQUEST_DOMAIN: &[u8] = b"ple:rehearsal:submission-request:v1\0";

mod claims;
mod inventory;
/// Private, versioned persistence reconstruction for rehearsal evidence.
pub mod persistence;

pub use claims::{
    DispatchedClaimHandle, PreparedClaimHandle, RehearsalClaimCompletionError,
    RehearsalClaimCompletionMaterial, RehearsalClaimCompletionProofError, RehearsalClaimGeneration,
    RehearsalClaimHandleError, RehearsalClaimHydrationError, RehearsalClaimReclaimError,
    RehearsalClaimRoot, RehearsalClaimRootVerificationError, RehearsalClaimTransitionEvent,
    RehearsalPersistedClaimRoot, RehearsalPreDispatchAbandonReason,
    RehearsalPreDispatchAbandonment, RehearsalSubmissionClaimDecision,
    RehearsalSubmissionClaimPhase, RehearsalSubmissionClaimSnapshot, RehearsalSubmissionClaimState,
    VerifiedRehearsalClaimCompletionProof, abandon_rehearsal_submission_before_dispatch,
    decide_submission_claim, hydrate_claim_history, mark_rehearsal_submission_dispatched,
    validate_claim_completion, verify_rehearsal_claim_completion_proof,
};
pub use inventory::{
    RehearsalFrozenInventoryEntry, RehearsalInventoryError, VerifiedRehearsalAcceptedEvidenceOwner,
    rehearsal_accepted_evidence_owner, verify_rehearsal_inventory,
};

/// A canonical, private digest of exactly one validated grading input.
///
/// This identifies a proposed submission, not an acceptance event. It is not
/// serde and never belongs in browser transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RehearsalSubmissionRequestFingerprint([u8; 32]);

impl RehearsalSubmissionRequestFingerprint {
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(self) -> String {
        RehearsalEvidenceDigest::from_bytes(self.0).to_hex()
    }
}

/// Private, schema-bound submission input sealed before a grader runs.
///
/// It intentionally has no grading result or timestamp. The Store constructs
/// it only from an aggregate-frozen attempt and the raw response it decoded.
#[derive(Clone, PartialEq)]
pub struct RehearsalValidatedSubmissionRequest {
    commitment: RehearsalFrozenAttemptCommitment,
    submitted_response: question_model::StudentResponse,
}

/// Exact immutable provenance and schema commitment for one frozen attempt.
///
/// This remains private because it is part of the grader input, not browser
/// transport.  Retaining it prevents a request validated against one frozen
/// item from later being fingerprinted or graded against another same-attempt
/// record.
#[derive(Clone, PartialEq, Eq)]
struct RehearsalFrozenAttemptCommitment {
    attempt: question_model::RehearsalAttemptId,
    problem: question_model::ProblemVersionRef,
    canonical_content_digest: RehearsalEvidenceDigest,
    response_schema_digest: RehearsalEvidenceDigest,
}

impl RehearsalFrozenAttemptCommitment {
    fn from_frozen(frozen: &RehearsalFrozenItemEvidence) -> Self {
        Self {
            attempt: frozen.attempt,
            problem: frozen.problem,
            canonical_content_digest: frozen.canonical_content_digest,
            response_schema_digest: frozen_response_schema_digest(&frozen.response_definition),
        }
    }

    fn matches_frozen(&self, frozen: &RehearsalFrozenItemEvidence) -> bool {
        self.attempt == frozen.attempt
            && self.problem == frozen.problem
            && self.canonical_content_digest == frozen.canonical_content_digest
            && self.response_schema_digest
                == frozen_response_schema_digest(&frozen.response_definition)
    }
}

impl RehearsalValidatedSubmissionRequest {
    pub fn try_from_frozen_attempt(
        frozen: &RehearsalFrozenItemEvidence,
        submitted_attempt: question_model::RehearsalAttemptId,
        submitted_response: question_model::StudentResponse,
    ) -> Result<Self, RehearsalEvidenceValidationError> {
        if frozen.attempt != submitted_attempt {
            return Err(RehearsalEvidenceValidationError::ResponseDefinitionMismatch);
        }
        validate_rehearsal_response(&frozen.response_definition, &submitted_response)?;
        if response_bytes(&submitted_response)
            > question_model::MAX_REHEARSAL_ACCEPTED_SUBMISSION_BYTES
        {
            return Err(RehearsalEvidenceValidationError::AcceptedSubmissionTooLarge);
        }
        if response_entries(&submitted_response)
            > question_model::MAX_REHEARSAL_ACCEPTED_SUBMISSION_ENTRIES
        {
            return Err(RehearsalEvidenceValidationError::TooManyAcceptedSubmissionEntries);
        }
        Ok(Self {
            commitment: RehearsalFrozenAttemptCommitment::from_frozen(frozen),
            submitted_response,
        })
    }

    pub fn attempt(&self) -> question_model::RehearsalAttemptId {
        self.commitment.attempt
    }

    pub fn submitted_response(&self) -> &question_model::StudentResponse {
        &self.submitted_response
    }

    /// Fails closed unless this sealed request still matches the Store-loaded
    /// frozen attempt that will be prepared or completed.
    pub fn validate_frozen_attempt(
        &self,
        frozen: &RehearsalFrozenItemEvidence,
    ) -> Result<(), RehearsalEvidenceValidationError> {
        self.commitment
            .matches_frozen(frozen)
            .then_some(())
            .ok_or(RehearsalEvidenceValidationError::ResponseDefinitionMismatch)
    }
}

/// Domain-owned, schema-bound private submission evidence.
///
/// The only constructor validates a raw response against the exact definition
/// frozen with the Store-retrieved attempt.  It is intentionally non-serde and
/// exposes only persistence reads; no caller can manufacture accepted evidence.
#[derive(Clone, PartialEq)]
pub struct RehearsalValidatedSubmissionEvidence {
    claim_binding: claims::ClaimRootBinding,
    attempt: question_model::RehearsalAttemptId,
    submitted_response: question_model::StudentResponse,
    result: RehearsalPrivateGradingResult,
    accepted_at: question_model::ActivityTimestamp,
}

impl RehearsalValidatedSubmissionEvidence {
    /// Completes a sealed grading request only against the exact frozen
    /// evidence record locked by the Store for this attempt.
    pub fn try_complete_with_frozen_attempt(
        root: &RehearsalClaimRoot,
        request: RehearsalValidatedSubmissionRequest,
        frozen: &RehearsalFrozenItemEvidence,
        result: RehearsalPrivateGradingResult,
        accepted_at: question_model::ActivityTimestamp,
    ) -> Result<Self, RehearsalEvidenceValidationError> {
        request.validate_frozen_attempt(frozen)?;
        validate_grading_result(&result)?;
        if private_submission_bytes(request.submitted_response(), &result)
            > question_model::MAX_REHEARSAL_ACCEPTED_SUBMISSION_BYTES
        {
            return Err(RehearsalEvidenceValidationError::AcceptedSubmissionTooLarge);
        }
        if private_submission_entries(request.submitted_response(), &result)
            > question_model::MAX_REHEARSAL_ACCEPTED_SUBMISSION_ENTRIES
        {
            return Err(RehearsalEvidenceValidationError::TooManyAcceptedSubmissionEntries);
        }
        Ok(Self {
            claim_binding: root.binding,
            attempt: request.attempt(),
            submitted_response: request.submitted_response,
            result,
            accepted_at,
        })
    }

    pub fn attempt(&self) -> question_model::RehearsalAttemptId {
        self.attempt
    }
    pub fn submitted_response(&self) -> &question_model::StudentResponse {
        &self.submitted_response
    }
    pub fn result(&self) -> &RehearsalPrivateGradingResult {
        &self.result
    }
    pub fn accepted_at(&self) -> question_model::ActivityTimestamp {
        self.accepted_at
    }

    fn claim_binding_matches(&self, binding: claims::ClaimRootBinding) -> bool {
        self.claim_binding == binding
    }

    fn browser_safe_receipt(&self) -> question_model::RehearsalPublicOutcome {
        let RehearsalPrivateGradingResult::Graded { feedback, .. } = &self.result;
        question_model::RehearsalPublicOutcome::Submitted {
            feedback: feedback.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RehearsalSubjectFingerprint([u8; 32]);
impl RehearsalSubjectFingerprint {
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
    pub fn to_hex(self) -> String {
        RehearsalEvidenceDigest::from_bytes(self.0).to_hex()
    }
}

/// Immutable context loaded from the authorized aggregate row, never request input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalGenesisContext {
    pub rehearsal: RehearsalRunId,
    pub tenant: TenantId,
    pub course: CourseId,
    pub assignment: AssignmentReference,
    pub direct_instructor_membership: CourseMembershipId,
    pub revision: TeachingOperationRevision,
    pub subject_fingerprint: RehearsalSubjectFingerprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalLifecycleSnapshot {
    pub lifecycle: RehearsalLifecycle,
    pub revision: TeachingOperationRevision,
    pub subject_fingerprint: RehearsalSubjectFingerprint,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalStartDecision {
    Create,
    Resume,
    DiscardByNewSubjectThenCreate,
    RequireExplicitRestart,
    DiscardStaleRevision,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalTerminalTransition {
    Complete,
    DiscardByInstructor,
    DiscardByNewSubject,
    DiscardStaleRevision,
    DiscardSourceContextRemoved,
}
/// The exact private evidence payload persisted beside an append-only chain row.
#[derive(Clone, PartialEq)]
pub enum RehearsalEvidencePayload {
    FrozenItem(RehearsalFrozenItemEvidence),
    AcceptedSubmission(RehearsalValidatedSubmissionEvidence),
}
impl RehearsalEvidencePayload {
    /// Returns the immutable evidence variant for persistence verification.
    pub fn kind(&self) -> RehearsalEvidenceKind {
        match self {
            Self::FrozenItem(_) => RehearsalEvidenceKind::FrozenItem,
            Self::AcceptedSubmission(_) => RehearsalEvidenceKind::AcceptedSubmission,
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct RehearsalEvidenceChainEntry {
    pub record: RehearsalEvidenceRecord,
    pub payload: RehearsalEvidencePayload,
}

/// Aggregate-owned commitment to the complete private evidence sequence.
///
/// This is persistence state, not a browser DTO.  It prevents a rewritten and
/// rehashed evidence collection from certifying itself: verification begins at
/// genesis and must arrive at this independently stored terminal commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalEvidenceHead {
    digest: RehearsalEvidenceDigest,
    length: u32,
}

impl RehearsalEvidenceHead {
    /// Decodes the private durable representation owned by a Store.
    pub const fn from_persisted(digest: RehearsalEvidenceDigest, length: u32) -> Self {
        Self { digest, length }
    }

    pub const fn digest(self) -> RehearsalEvidenceDigest {
        self.digest
    }

    pub const fn length(self) -> u32 {
        self.length
    }

    /// Advances the head only over the next correctly linked evidence record.
    pub fn advance(
        self,
        record: &RehearsalEvidenceRecord,
    ) -> Result<Self, RehearsalIntegrityError> {
        let expected = self
            .length
            .checked_add(1)
            .ok_or(RehearsalIntegrityError::SequenceGap)?;
        if record.sequence != expected || record.previous_digest != Some(self.digest) {
            return Err(RehearsalIntegrityError::HeadMismatch);
        }
        Ok(Self::from_persisted(record.digest, record.sequence))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalIntegrityError {
    NonCanonicalSubject,
    SubjectBindingMismatch,
    SubjectNotAuthorized,
    TerminalLifecycle,
    InvalidEvidenceKind,
    SequenceGap,
    PreviousDigestMismatch,
    DigestMismatch,
    HeadMismatch,
}

/// Hashes a Store-resolved subject after its assignment/revision binding is
/// checked. This validates representation, not authority: Store resolution is
/// the only authority boundary for synthetic and derived start candidates.
pub fn fingerprint_resolved_preview_subject(
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    subject: &PreviewSubject,
) -> Result<RehearsalSubjectFingerprint, RehearsalIntegrityError> {
    validate_subject_binding(assignment, revision, subject)?;
    let bytes = serde_json::to_vec(subject).expect("validated preview subject serializes");
    Ok(RehearsalSubjectFingerprint(
        digest(SUBJECT_FINGERPRINT_DOMAIN, framed(bytes)).as_bytes(),
    ))
}
pub fn validate_subject_binding(
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    subject: &PreviewSubject,
) -> Result<(), RehearsalIntegrityError> {
    (subject.assignment == assignment && subject.revision == revision)
        .then_some(())
        .ok_or(RehearsalIntegrityError::SubjectBindingMismatch)
}

pub fn decide_start(
    current: Option<RehearsalLifecycleSnapshot>,
    requested_revision: TeachingOperationRevision,
    requested_subject: RehearsalSubjectFingerprint,
    start_new_after_completion: bool,
) -> RehearsalStartDecision {
    let Some(current) = current else {
        return RehearsalStartDecision::Create;
    };
    if current.revision != requested_revision {
        return RehearsalStartDecision::DiscardStaleRevision;
    }
    match current.lifecycle {
        RehearsalLifecycle::Active if current.subject_fingerprint == requested_subject => {
            RehearsalStartDecision::Resume
        }
        RehearsalLifecycle::Active => RehearsalStartDecision::DiscardByNewSubjectThenCreate,
        RehearsalLifecycle::Completed if start_new_after_completion => {
            RehearsalStartDecision::Create
        }
        RehearsalLifecycle::Completed => RehearsalStartDecision::RequireExplicitRestart,
        _ => RehearsalStartDecision::Create,
    }
}
pub fn apply_terminal_transition(
    lifecycle: RehearsalLifecycle,
    transition: RehearsalTerminalTransition,
) -> Result<RehearsalLifecycle, RehearsalIntegrityError> {
    if lifecycle.is_terminal() {
        return Err(RehearsalIntegrityError::TerminalLifecycle);
    }
    Ok(match transition {
        RehearsalTerminalTransition::Complete => RehearsalLifecycle::Completed,
        RehearsalTerminalTransition::DiscardByInstructor => {
            RehearsalLifecycle::DiscardedByInstructor
        }
        RehearsalTerminalTransition::DiscardByNewSubject => {
            RehearsalLifecycle::DiscardedByNewSubject
        }
        RehearsalTerminalTransition::DiscardStaleRevision => {
            RehearsalLifecycle::DiscardedStaleRevision
        }
        RehearsalTerminalTransition::DiscardSourceContextRemoved => {
            RehearsalLifecycle::DiscardedSourceContextRemoved
        }
    })
}
/// Canonically fingerprints the aggregate-bound, frozen input that a grader
/// receives. Acceptance time, grader output, feedback, and idempotency key are
/// deliberately absent because they identify an event, not an input.
pub fn rehearsal_submission_request_fingerprint(
    context: RehearsalGenesisContext,
    frozen: &RehearsalFrozenItemEvidence,
    request: &RehearsalValidatedSubmissionRequest,
) -> Result<RehearsalSubmissionRequestFingerprint, RehearsalEvidenceValidationError> {
    request.validate_frozen_attempt(frozen)?;
    let mut e = Encoder::new();
    e.raw(&evidence_genesis_digest(context).as_bytes());
    e.uuid(frozen.attempt.as_uuid().as_bytes());
    e.uuid(frozen.problem.problem.as_uuid().as_bytes());
    e.uuid(frozen.problem.version.as_uuid().as_bytes());
    e.raw(&frozen.canonical_content_digest.as_bytes());
    e.raw(&frozen_response_schema_digest(&frozen.response_definition).as_bytes());
    encode_response(&mut e, request.submitted_response());
    Ok(RehearsalSubmissionRequestFingerprint(
        digest(SUBMISSION_REQUEST_DOMAIN, e.finish()).as_bytes(),
    ))
}

/// Binds genesis to immutable run, tenant, course, direct-Instructor membership, revision, and subject.
pub fn evidence_genesis_digest(context: RehearsalGenesisContext) -> RehearsalEvidenceDigest {
    let mut e = Encoder::new();
    e.uuid(context.rehearsal.as_uuid().as_bytes());
    e.uuid(context.tenant.as_uuid().as_bytes());
    e.uuid(context.course.as_uuid().as_bytes());
    e.u32(context.assignment.number());
    e.uuid(context.direct_instructor_membership.as_uuid().as_bytes());
    e.u64(context.revision.value());
    e.raw(&context.subject_fingerprint.as_bytes());
    digest(EVIDENCE_GENESIS_DOMAIN, e.finish())
}

/// The only valid aggregate head before the first evidence append.
pub fn evidence_genesis_head(context: RehearsalGenesisContext) -> RehearsalEvidenceHead {
    RehearsalEvidenceHead::from_persisted(evidence_genesis_digest(context), 0)
}
pub fn evidence_entry_digest(
    sequence: u32,
    kind: RehearsalEvidenceKind,
    previous: RehearsalEvidenceDigest,
    payload: RehearsalEvidenceDigest,
    recorded_at: question_model::ActivityTimestamp,
) -> RehearsalEvidenceDigest {
    let mut e = Encoder::new();
    e.u32(sequence);
    e.u8(kind_tag(kind));
    e.raw(&previous.as_bytes());
    e.raw(&payload.as_bytes());
    e.i64(recorded_at.as_unix_millis());
    digest(EVIDENCE_ENTRY_DOMAIN, e.finish())
}

/// Canonically digests every persisted private payload field. No generic serde hashing is used.
pub fn private_payload_digest(payload: &RehearsalEvidencePayload) -> RehearsalEvidenceDigest {
    let mut e = Encoder::new();
    match payload {
        RehearsalEvidencePayload::FrozenItem(v) => {
            e.u8(1);
            e.uuid(v.attempt.as_uuid().as_bytes());
            e.uuid(v.problem.problem.as_uuid().as_bytes());
            e.uuid(v.problem.version.as_uuid().as_bytes());
            e.raw(&frozen_response_schema_digest(&v.response_definition).as_bytes());
            e.raw(&v.canonical_content_digest.as_bytes());
            e.i64(v.frozen_at.as_unix_millis());
        }
        RehearsalEvidencePayload::AcceptedSubmission(v) => {
            e.u8(2);
            e.uuid(v.claim_binding.rehearsal.as_uuid().as_bytes());
            e.uuid(v.claim_binding.claim.as_uuid().as_bytes());
            e.raw(&v.claim_binding.fingerprint.as_bytes());
            e.uuid(v.attempt().as_uuid().as_bytes());
            encode_response(&mut e, v.submitted_response());
            encode_result(&mut e, v.result());
            e.i64(v.accepted_at().as_unix_millis());
        }
    }
    digest(EVIDENCE_PAYLOAD_DOMAIN, e.finish())
}

/// Canonically commits the browser-safe completion projection only. Private
/// response, provider receipt, and timestamps are deliberately absent.
pub fn rehearsal_public_receipt_digest(
    receipt: &question_model::RehearsalPublicOutcome,
) -> RehearsalEvidenceDigest {
    let mut e = Encoder::new();
    match receipt {
        question_model::RehearsalPublicOutcome::Submitted { feedback } => {
            e.u8(1);
            encode_feedback(&mut e, feedback);
        }
        _ => unreachable!("accepted rehearsal evidence has only terminal receipt projections"),
    }
    digest(b"ple:rehearsal:browser-receipt:v1\0", e.finish())
}

/// A collision-resistant digest of the exact response schema persisted with an
/// issued attempt. The definition remains stored as evidence; this digest gives
/// the chain a fixed-size canonical commitment to every schema field.
pub fn frozen_response_schema_digest(
    definition: &question_model::ResponseDefinition,
) -> RehearsalEvidenceDigest {
    let bytes = serde_json::to_vec(definition)
        .expect("response definitions are closed serializable rehearsal evidence");
    digest(FROZEN_RESPONSE_SCHEMA_DOMAIN, framed(bytes))
}

fn validate_grading_result(
    result: &RehearsalPrivateGradingResult,
) -> Result<(), RehearsalEvidenceValidationError> {
    let RehearsalPrivateGradingResult::Graded {
        result, feedback, ..
    } = result;
    if !result.points_earned.is_finite() || !result.points_possible.is_finite() {
        return Err(RehearsalEvidenceValidationError::NonFiniteAttemptResult);
    }
    [feedback.points_earned, feedback.points_possible]
        .into_iter()
        .flatten()
        .all(f64::is_finite)
        .then_some(())
        .ok_or(RehearsalEvidenceValidationError::NonFiniteFeedback)
}

fn validate_rehearsal_response(
    definition: &question_model::ResponseDefinition,
    response: &question_model::StudentResponse,
) -> Result<(), RehearsalEvidenceValidationError> {
    match definition {
        question_model::ResponseDefinition::FileUpload { .. } => {
            return Err(RehearsalEvidenceValidationError::FileUploadUnsupported);
        }
        question_model::ResponseDefinition::ExternalTool {} => {
            return Err(RehearsalEvidenceValidationError::ExternalToolUnsupported);
        }
        _ => {}
    }
    let report = validate_response_format(definition, response);
    if report.is_valid() {
        return Ok(());
    }
    Err(
        if report
            .violations
            .contains(&ResponseFormatViolation::NumericNotFinite)
        {
            RehearsalEvidenceValidationError::NonFiniteNumericResponse
        } else {
            RehearsalEvidenceValidationError::InvalidResponseShape
        },
    )
}

fn private_submission_bytes(
    response: &question_model::StudentResponse,
    result: &RehearsalPrivateGradingResult,
) -> usize {
    response_bytes(response) + grading_result_bytes(result)
}

fn private_submission_entries(
    response: &question_model::StudentResponse,
    result: &RehearsalPrivateGradingResult,
) -> usize {
    response_entries(response) + grading_result_entries(result)
}

fn response_entries(response: &question_model::StudentResponse) -> usize {
    use question_model::StudentResponse;
    match response {
        StudentResponse::Numeric { .. } | StudentResponse::ShortText { .. } => 1,
        StudentResponse::MultipleChoice { selected }
        | StudentResponse::Ordering { order: selected } => selected.len(),
        StudentResponse::MultiBlank { answers } => answers.len(),
        StudentResponse::Matching { matches } => matches.len(),
        StudentResponse::Hotspot { points } => points.len(),
        StudentResponse::FileUpload { .. } | StudentResponse::ExternalTool {} => 0,
    }
}

fn response_bytes(response: &question_model::StudentResponse) -> usize {
    use question_model::StudentResponse;
    match response {
        StudentResponse::Numeric { .. } => 8,
        StudentResponse::MultipleChoice { selected }
        | StudentResponse::Ordering { order: selected } => {
            selected.iter().map(|choice| choice.as_str().len()).sum()
        }
        StudentResponse::ShortText { text } => text.len(),
        StudentResponse::MultiBlank { answers } => answers
            .iter()
            .map(|answer| answer.slot.as_str().len() + answer.text.len())
            .sum(),
        StudentResponse::Matching { matches } => matches
            .iter()
            .map(|pair| pair.prompt.as_str().len() + pair.choice.as_str().len())
            .sum(),
        StudentResponse::Hotspot { points } => points.len() * 4,
        StudentResponse::FileUpload { object_key } => object_key.len(),
        StudentResponse::ExternalTool {} => 0,
    }
}

fn grading_result_bytes(result: &RehearsalPrivateGradingResult) -> usize {
    let RehearsalPrivateGradingResult::Graded {
        feedback,
        backend_receipt_reference,
        ..
    } = result;
    17 + backend_receipt_reference.as_str().len() + feedback_bytes(feedback)
}

fn grading_result_entries(result: &RehearsalPrivateGradingResult) -> usize {
    let RehearsalPrivateGradingResult::Graded { feedback, .. } = result;
    1 + feedback_entries(feedback)
}

fn feedback_bytes(feedback: &DisclosedFeedback) -> usize {
    feedback.hint.as_deref().map(blocks_bytes).unwrap_or(0)
        + feedback
            .correct_response
            .as_deref()
            .map(blocks_bytes)
            .unwrap_or(0)
        + feedback.rationale.as_deref().map(blocks_bytes).unwrap_or(0)
        + 17
}

fn feedback_entries(feedback: &DisclosedFeedback) -> usize {
    feedback.hint.as_deref().map(blocks_entries).unwrap_or(0)
        + feedback
            .correct_response
            .as_deref()
            .map(blocks_entries)
            .unwrap_or(0)
        + feedback
            .rationale
            .as_deref()
            .map(blocks_entries)
            .unwrap_or(0)
}

fn blocks_bytes(blocks: &[ContentBlock]) -> usize {
    blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { markdown } => markdown.len(),
            ContentBlock::Math { latex, description } => latex.len() + description.len(),
            ContentBlock::Image { asset, description } => {
                16 + asset.checksum.len() + description.len()
            }
            ContentBlock::Code { language, source } => language.len() + source.len(),
            ContentBlock::Table {
                headers,
                rows,
                description,
            } => {
                description.len()
                    + headers.iter().map(String::len).sum::<usize>()
                    + rows
                        .iter()
                        .flat_map(|row| row.iter())
                        .map(String::len)
                        .sum::<usize>()
            }
        })
        .sum()
}

fn blocks_entries(blocks: &[ContentBlock]) -> usize {
    blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Table { headers, rows, .. } => {
                1 + headers.len() + rows.len() + rows.iter().map(Vec::len).sum::<usize>()
            }
            _ => 1,
        })
        .sum()
}
/// Recomputes from private payload bytes loaded with each persisted row; supplied digests are never trusted.
///
/// Evidence `sequence` is the causal authority. `recorded_at` is hashed
/// observational audit metadata and deliberately need not be monotonic: host
/// clocks can move backward without changing append order or phase validity.
pub fn verify_evidence_chain(
    context: RehearsalGenesisContext,
    expected_head: RehearsalEvidenceHead,
    entries: &[RehearsalEvidenceChainEntry],
) -> Result<(), RehearsalIntegrityError> {
    let mut head = evidence_genesis_head(context);
    for (index, entry) in entries.iter().enumerate() {
        let expected_sequence =
            u32::try_from(index + 1).map_err(|_| RehearsalIntegrityError::SequenceGap)?;
        if entry.record.sequence != expected_sequence {
            return Err(RehearsalIntegrityError::SequenceGap);
        }
        if entry.record.kind != entry.payload.kind()
            || entry.record.kind == RehearsalEvidenceKind::Genesis
        {
            return Err(RehearsalIntegrityError::InvalidEvidenceKind);
        }
        if entry.record.previous_digest != Some(head.digest) {
            return Err(RehearsalIntegrityError::PreviousDigestMismatch);
        }
        let expected = evidence_entry_digest(
            entry.record.sequence,
            entry.record.kind,
            head.digest,
            private_payload_digest(&entry.payload),
            entry.record.recorded_at,
        );
        if entry.record.digest != expected {
            return Err(RehearsalIntegrityError::DigestMismatch);
        }
        head = head.advance(&entry.record)?;
    }
    (head == expected_head)
        .then_some(())
        .ok_or(RehearsalIntegrityError::HeadMismatch)
}

fn digest(domain: &[u8], bytes: Vec<u8>) -> RehearsalEvidenceDigest {
    let mut h = Sha256::new();
    h.update(domain);
    h.update(bytes);
    RehearsalEvidenceDigest::from_bytes(h.finalize().into())
}
fn framed(bytes: Vec<u8>) -> Vec<u8> {
    let mut e = Encoder::new();
    e.bytes(&bytes);
    e.finish()
}
const fn kind_tag(kind: RehearsalEvidenceKind) -> u8 {
    match kind {
        RehearsalEvidenceKind::Genesis => 0,
        RehearsalEvidenceKind::FrozenItem => 1,
        RehearsalEvidenceKind::AcceptedSubmission => 2,
    }
}
struct Encoder(Vec<u8>);
impl Encoder {
    fn new() -> Self {
        Self(Vec::new())
    }
    fn finish(self) -> Vec<u8> {
        self.0
    }
    fn raw(&mut self, v: &[u8]) {
        self.0.extend_from_slice(v);
    }
    fn bytes(&mut self, v: &[u8]) {
        self.u64(u64::try_from(v.len()).expect("usize fits u64"));
        self.raw(v);
    }
    fn text(&mut self, v: &str) {
        self.bytes(v.as_bytes())
    }
    fn u8(&mut self, v: u8) {
        self.0.push(v)
    }
    fn u16(&mut self, v: u16) {
        self.raw(&v.to_be_bytes())
    }
    fn u32(&mut self, v: u32) {
        self.raw(&v.to_be_bytes())
    }
    fn u64(&mut self, v: u64) {
        self.raw(&v.to_be_bytes())
    }
    fn i64(&mut self, v: i64) {
        self.raw(&v.to_be_bytes())
    }
    fn f64(&mut self, v: f64) {
        self.u64(v.to_bits())
    }
    fn uuid(&mut self, v: &[u8; 16]) {
        self.raw(v)
    }
    fn option<T>(&mut self, v: &Option<T>, f: impl Fn(&mut Self, &T)) {
        match v {
            Some(x) => {
                self.u8(1);
                f(self, x)
            }
            None => self.u8(0),
        }
    }
    fn list<T>(&mut self, v: &[T], f: impl Fn(&mut Self, &T)) {
        self.u64(u64::try_from(v.len()).expect("usize fits u64"));
        for x in v {
            f(self, x);
        }
    }
}
fn encode_response(e: &mut Encoder, v: &question_model::StudentResponse) {
    use question_model::StudentResponse;

    match v {
        StudentResponse::Numeric { value } => {
            e.u8(1);
            e.f64(*value)
        }
        StudentResponse::MultipleChoice { selected } => {
            e.u8(2);
            e.list(selected, |x, y| x.text(y.as_str()))
        }
        StudentResponse::ShortText { text } => {
            e.u8(3);
            e.text(text)
        }
        StudentResponse::MultiBlank { answers } => {
            e.u8(4);
            e.list(answers, |x, y| {
                x.text(y.slot.as_str());
                x.text(&y.text)
            })
        }
        StudentResponse::Matching { matches } => {
            e.u8(5);
            e.list(matches, |x, y| {
                x.text(y.prompt.as_str());
                x.text(y.choice.as_str())
            })
        }
        StudentResponse::Ordering { order } => {
            e.u8(6);
            e.list(order, |x, y| x.text(y.as_str()))
        }
        StudentResponse::Hotspot { points } => {
            e.u8(7);
            e.list(points, |x, y| {
                x.u16(y.x);
                x.u16(y.y)
            })
        }
        StudentResponse::FileUpload { .. } | StudentResponse::ExternalTool {} => {
            unreachable!("validated rehearsal evidence excludes unsupported response families")
        }
    }
}
fn encode_result(e: &mut Encoder, v: &RehearsalPrivateGradingResult) {
    let RehearsalPrivateGradingResult::Graded {
        result,
        feedback,
        backend_receipt_reference,
    } = v;
    e.u8(1);
    e.u8(u8::from(result.correct));
    e.f64(result.points_earned);
    e.f64(result.points_possible);
    encode_feedback(e, feedback);
    e.text(backend_receipt_reference.as_str())
}
fn encode_feedback(e: &mut Encoder, v: &DisclosedFeedback) {
    e.option(&v.correctness, |x, y| x.u8(u8::from(*y)));
    e.option(&v.points_earned, |x, y| x.f64(*y));
    e.option(&v.points_possible, |x, y| x.f64(*y));
    e.option(&v.hint, |x, y| encode_blocks(x, y));
    e.option(&v.correct_response, |x, y| encode_blocks(x, y));
    e.option(&v.rationale, |x, y| encode_blocks(x, y));
}
fn encode_blocks(e: &mut Encoder, values: &[ContentBlock]) {
    e.list(values, |x, v| match v {
        ContentBlock::Text { markdown } => {
            x.u8(1);
            x.text(markdown)
        }
        ContentBlock::Math { latex, description } => {
            x.u8(2);
            x.text(latex);
            x.text(description)
        }
        ContentBlock::Image { asset, description } => {
            x.u8(3);
            x.uuid(asset.asset.as_uuid().as_bytes());
            x.text(&asset.checksum);
            x.text(description)
        }
        ContentBlock::Code { language, source } => {
            x.u8(4);
            x.text(language);
            x.text(source)
        }
        ContentBlock::Table {
            headers,
            rows,
            description,
        } => {
            x.u8(5);
            x.list(headers, |y, z| y.text(z));
            x.list(rows, |y, z| y.list(z, |a, b| a.text(b)));
            x.text(description)
        }
    })
}

#[cfg(test)]
mod claims_tests;
#[cfg(test)]
mod inventory_tests;
#[cfg(test)]
mod tests;
