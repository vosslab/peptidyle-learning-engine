//! Closed contracts for instructor-owned rehearsal runs (WP-PROF-T4).
//!
//! These types deliberately model a dedicated aggregate.  They do not extend
//! an ordinary learner run and do not carry a learner identity.  Public DTOs
//! remain answer-free; private evidence is intentionally not serializable.

use std::{num::NonZeroU32, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ActivityTimestamp, AssignmentReference, AttemptResult, DisclosedFeedback,
    PreviewSelectedMoment, PreviewSubject, PreviewSyntheticGroupReferences, ProblemVersionRef,
    SyntheticPreviewModifiers, TeachingOperationRevision,
};

/// Largest public rehearsal-route number accepted by every product layer.
pub const MAX_REHEARSAL_REFERENCE_NUMBER: u32 = i32::MAX as u32;

/// A server-owned rehearsal aggregate identity. It is never a browser locator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RehearsalRunId(Uuid);

/// A server-owned issued rehearsal item identity. It is never a learner attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RehearsalAttemptId(Uuid);

/// A server-owned submission-claim identity. It is private Store material,
/// never a browser locator or a learner attempt identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RehearsalSubmissionClaimId(Uuid);

/// A server-owned grader-operation identity. It is private Store material and
/// allows a backend with an equivalent operation replay contract to seal one
/// logical grading operation across a retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RehearsalGradeOperationId(Uuid);

macro_rules! impl_rehearsal_identifier {
    ($name:ident) => {
        impl $name {
            /// Restores the storage-only identity after the Store has authorized its row.
            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            /// Returns the storage-only UUID. Never use this value in transport or UI.
            pub fn as_uuid(self) -> Uuid {
                self.0
            }

            /// Mints a fresh server-owned identity.
            #[cfg(feature = "generate")]
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }
        }
    };
}

impl_rehearsal_identifier!(RehearsalRunId);
impl_rehearsal_identifier!(RehearsalAttemptId);
impl_rehearsal_identifier!(RehearsalSubmissionClaimId);
impl_rehearsal_identifier!(RehearsalGradeOperationId);

/// Opaque route locator for a rehearsal aggregate. It is a locator, never authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RehearsalReference(NonZeroU32);

impl RehearsalReference {
    /// Creates a public reference from the Store's positive sequence value.
    pub fn new(value: u64) -> Option<Self> {
        u32::try_from(value)
            .ok()
            .filter(|value| *value <= MAX_REHEARSAL_REFERENCE_NUMBER)
            .and_then(NonZeroU32::new)
            .map(Self)
    }

    /// Returns the persistence sequence number after authorization.
    pub fn number(self) -> u32 {
        self.0.get()
    }
}

impl std::fmt::Display for RehearsalReference {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "RH-{}", self.number())
    }
}

impl FromStr for RehearsalReference {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some(digits) = value.strip_prefix("RH-") else {
            return Err("rehearsal reference must look like RH-123");
        };
        if digits.is_empty()
            || digits.len() > 10
            || digits.starts_with('0')
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("rehearsal reference must look like RH-123");
        }
        digits
            .parse::<u64>()
            .ok()
            .and_then(Self::new)
            .ok_or("rehearsal reference must be a positive 31-bit value")
    }
}

impl TryFrom<String> for RehearsalReference {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<RehearsalReference> for String {
    fn from(value: RehearsalReference) -> Self {
        value.to_string()
    }
}

/// The only lifecycle states that can be persisted for a rehearsal aggregate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RehearsalLifecycle {
    Active,
    Completed,
    DiscardedByInstructor,
    DiscardedByNewSubject,
    DiscardedStaleRevision,
    DiscardedSourceContextRemoved,
}

impl RehearsalLifecycle {
    /// Whether the aggregate may resume or accept a new issue request.
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    /// Whether the aggregate cannot transition again.
    pub const fn is_terminal(self) -> bool {
        !self.is_active()
    }
}

/// A server-visible issue outcome that does not disclose answer material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RehearsalAttemptState {
    Ready,
    Issued,
    Submitted,
    Expired,
}

/// A closed delivery decision before rendering or accepting a response body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RehearsalBackendSupport {
    SupportedNative,
    SupportedTrustedRenderer,
    UnsupportedExternal,
    UnsupportedUpload,
}

impl RehearsalBackendSupport {
    /// Whether this backend may enter the isolated rehearsal issue transaction.
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::SupportedNative | Self::SupportedTrustedRenderer)
    }
}

/// Closed, answer-free failure codes used by the rehearsal HTTP boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RehearsalErrorCode {
    StaleRevision,
    DeliveryUnsupported,
    AttemptExpired,
    InvalidLifecycle,
    IdempotencyConflict,
    SubmissionPending,
    EvidenceIntegrityFailure,
}

/// The only identity-free subject inputs a rehearsal start may carry.
///
/// The route already owns the course and assignment.  This union is a
/// candidate input, not an authorization credential: the Store resolves its
/// current policy source before it can create or resume a rehearsal aggregate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum RehearsalSubjectStart {
    Synthetic {
        request: RehearsalSyntheticSubjectRequest,
    },
    Derived {
        candidate: PreviewSubject,
    },
}

/// Strict, route-bound synthetic subject input for a rehearsal start.
///
/// Course, assignment, revision, actor, and membership are already bound by
/// the authenticated route.  The Store converts this candidate into the T3
/// `SyntheticPreviewSubjectRequest` only after it has supplied those trusted
/// route values inside its locked authorization transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalSyntheticSubjectRequest {
    pub selected_moment: PreviewSelectedMoment,
    pub groups: PreviewSyntheticGroupReferences,
    pub modifiers: SyntheticPreviewModifiers,
}

/// An answer-free request to start or explicitly restart a rehearsal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalStartRequest {
    pub subject: RehearsalSubjectStart,
    pub start_new_after_completion: bool,
}

/// An answer-free request to discard the instructor's current rehearsal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalDiscardRequest {
    pub revision: TeachingOperationRevision,
}

/// A browser-safe summary of one rehearsal run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalRunReceipt {
    pub rehearsal: RehearsalReference,
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub lifecycle: RehearsalLifecycle,
    pub subject: PreviewSubject,
    pub started_at: ActivityTimestamp,
    pub updated_at: ActivityTimestamp,
}

/// Frozen item provenance appended before an item is delivered.
#[derive(Debug, Clone, PartialEq)]
pub struct RehearsalFrozenItemEvidence {
    pub attempt: RehearsalAttemptId,
    pub problem: ProblemVersionRef,
    /// The exact native response definition rendered and issued for this
    /// rehearsal attempt.  This is private evidence, never browser data.
    pub response_definition: crate::ResponseDefinition,
    pub canonical_content_digest: RehearsalEvidenceDigest,
    pub frozen_at: ActivityTimestamp,
}

/// A bounded private receipt produced by a native or trusted-renderer grader.
///
/// It never crosses the browser boundary.  Its explicit bound prevents an
/// adapter from turning rehearsal evidence into an unbounded provider log.
///
/// The type intentionally does not implement [`Debug`], because formatting a
/// private provider receipt into diagnostics could disclose backend material.
///
/// ```compile_fail
/// use question_model::RehearsalBackendReceiptReference;
///
/// let receipt = RehearsalBackendReceiptReference::new("provider-receipt".into())
///     .expect("valid receipt");
/// let _ = format!("{receipt:?}");
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct RehearsalBackendReceiptReference(String);

/// Maximum Unicode scalar count retained for a private grader receipt reference.
pub const MAX_REHEARSAL_BACKEND_RECEIPT_REFERENCE_SCALARS: usize = 512;

impl RehearsalBackendReceiptReference {
    pub fn new(value: String) -> Result<Self, RehearsalEvidenceValidationError> {
        (!value.is_empty()
            && value.chars().count() <= MAX_REHEARSAL_BACKEND_RECEIPT_REFERENCE_SCALARS
            && !value.chars().any(char::is_control))
        .then_some(Self(value))
        .ok_or(RehearsalEvidenceValidationError::InvalidBackendReceiptReference)
    }

    /// Private persistence only; never serialize this value into a DTO.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Maximum bytes retained for one accepted response and its private grading receipt.
///
/// This matches the established 64 KiB native submission transport ceiling.
/// It also protects direct Store callers, which do not pass through HTTP.
pub const MAX_REHEARSAL_ACCEPTED_SUBMISSION_BYTES: usize = 64 * 1024;

/// Maximum collection members retained with one accepted private submission.
///
/// A direct Store caller could otherwise provide many empty identifiers that
/// fit a byte-only tally. The bound shares the established 64 KiB transport
/// budget: every serialized collection member consumes at least one byte.
pub const MAX_REHEARSAL_ACCEPTED_SUBMISSION_ENTRIES: usize =
    MAX_REHEARSAL_ACCEPTED_SUBMISSION_BYTES;

/// Typed refusal for response material that cannot be isolated in rehearsal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalEvidenceValidationError {
    FileUploadUnsupported,
    ExternalToolUnsupported,
    ResponseDefinitionMismatch,
    InvalidResponseShape,
    NonFiniteNumericResponse,
    NonFiniteAttemptResult,
    NonFiniteFeedback,
    AcceptedSubmissionTooLarge,
    TooManyAcceptedSubmissionEntries,
    InvalidBackendReceiptReference,
}

/// Server-owned grading outcome retained only in rehearsal-local evidence.
#[derive(Clone, PartialEq)]
pub enum RehearsalPrivateGradingResult {
    Graded {
        result: AttemptResult,
        feedback: DisclosedFeedback,
        backend_receipt_reference: RehearsalBackendReceiptReference,
    },
}

/// A full SHA-256 digest used only by private rehearsal evidence and Store verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RehearsalEvidenceDigest([u8; 32]);

impl RehearsalEvidenceDigest {
    /// Restores full digest bytes from trusted storage after length validation by the type.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns full bytes for private persistence and chain verification.
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Returns canonical lowercase hexadecimal for private persistence.
    pub fn to_hex(self) -> String {
        let mut value = String::with_capacity(64);
        for byte in self.0 {
            use std::fmt::Write as _;
            let _ = write!(&mut value, "{byte:02x}");
        }
        value
    }

    /// Parses exact lowercase hexadecimal private persistence material.
    pub fn parse_hex(value: &str) -> Result<Self, &'static str> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("rehearsal evidence digest must be 64 lowercase hexadecimal characters");
        }
        let mut bytes = [0_u8; 32];
        for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
            let text =
                std::str::from_utf8(chunk).map_err(|_| "invalid rehearsal evidence digest")?;
            bytes[index] =
                u8::from_str_radix(text, 16).map_err(|_| "invalid rehearsal evidence digest")?;
        }
        Ok(Self(bytes))
    }
}

/// Private append-only evidence event names. No replacement or deletion event exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalEvidenceKind {
    Genesis,
    FrozenItem,
    AcceptedSubmission,
}

/// One private tamper-evident evidence record stored in rehearsal sequence order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RehearsalEvidenceRecord {
    pub sequence: u32,
    pub kind: RehearsalEvidenceKind,
    pub previous_digest: Option<RehearsalEvidenceDigest>,
    pub digest: RehearsalEvidenceDigest,
    pub recorded_at: ActivityTimestamp,
}

/// Browser-safe terminal or recoverable attempt outcome.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RehearsalPublicOutcome {
    Submitted { feedback: DisclosedFeedback },
    AttemptExpired,
    SubmissionPending,
    StaleRevision,
    DeliveryUnsupported { support: RehearsalBackendSupport },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rehearsal_reference_is_canonical_and_never_a_uuid() {
        let reference: RehearsalReference = "RH-42".parse().expect("valid reference");
        assert_eq!(reference.to_string(), "RH-42");
        assert_eq!(
            serde_json::to_value(reference).unwrap(),
            serde_json::json!("RH-42")
        );
        for invalid in ["RH-0", "RH-042", "R-42", "RH-2147483648"] {
            assert!(invalid.parse::<RehearsalReference>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn public_outcomes_are_closed_and_answer_free() {
        let submitted = serde_json::to_string(&RehearsalPublicOutcome::Submitted {
            feedback: DisclosedFeedback::empty(),
        })
        .unwrap();
        assert!(!submitted.contains("needsManualGrading"));
        assert!(
            serde_json::from_value::<RehearsalStartRequest>(serde_json::json!({
                "assignment": "A-1",
                "revision": "1",
                "subject": {},
                "startNewAfterCompletion": false,
                "unexpected": true
            }))
            .is_err()
        );
    }

    #[test]
    fn source_context_lifecycle_serializes_as_a_closed_camel_case_state() {
        assert_eq!(
            serde_json::to_value(RehearsalLifecycle::DiscardedSourceContextRemoved).unwrap(),
            serde_json::json!("discardedSourceContextRemoved")
        );
        assert_eq!(
            serde_json::from_value::<RehearsalLifecycle>(serde_json::json!(
                "discardedSourceContextRemoved"
            ))
            .unwrap(),
            RehearsalLifecycle::DiscardedSourceContextRemoved
        );
    }

    #[test]
    fn synthetic_rehearsal_subject_is_route_bound_and_strict() {
        let candidate = serde_json::json!({
            "selectedMoment": {
                "value": "2026-08-20T09:00:00.000",
                "timeZone": "America/Chicago"
            },
            "groups": ["G-2", "G-1"],
            "modifiers": {
                "mode": "extendOnly",
                "patch": {
                    "availableAt": {"kind": "inherit"},
                    "dueAt": {"kind": "inherit"},
                    "closesAt": {"kind": "inherit"},
                    "timeLimitSeconds": {"kind": "inherit"},
                    "attemptLimit": {"kind": "inherit"}
                }
            }
        });
        let request: RehearsalSyntheticSubjectRequest =
            serde_json::from_value(candidate.clone()).expect("closed candidate");
        assert_eq!(request.groups.as_slice()[0].to_string(), "G-1");
        for forbidden in [
            "assignment",
            "revision",
            "course",
            "actor",
            "membership",
            "learner",
            "capability",
            "unexpected",
        ] {
            let mut invalid = candidate.clone();
            invalid[forbidden] = serde_json::json!("forbidden");
            assert!(
                serde_json::from_value::<RehearsalSyntheticSubjectRequest>(invalid).is_err(),
                "{forbidden} must not cross the rehearsal boundary"
            );
        }
        let start = RehearsalSubjectStart::Synthetic { request };
        let encoded = serde_json::to_value(start).expect("serializable request");
        assert!(encoded.get("assignment").is_none());
        assert!(encoded.get("revision").is_none());
    }

    #[test]
    fn evidence_digest_requires_canonical_lowercase_sha256() {
        assert!(RehearsalEvidenceDigest::parse_hex(&"a".repeat(64)).is_ok());
        assert!(RehearsalEvidenceDigest::parse_hex(&"A".repeat(64)).is_err());
        assert!(RehearsalEvidenceDigest::parse_hex("not-a-digest").is_err());
    }

    #[test]
    fn backend_receipt_reference_has_closed_scalar_and_control_bounds() {
        assert!(RehearsalBackendReceiptReference::new(String::new()).is_err());
        assert!(RehearsalBackendReceiptReference::new("native\nreceipt".into()).is_err());
        assert!(
            RehearsalBackendReceiptReference::new(
                "x".repeat(MAX_REHEARSAL_BACKEND_RECEIPT_REFERENCE_SCALARS + 1)
            )
            .is_err()
        );
        assert!(
            RehearsalBackendReceiptReference::new(
                "x".repeat(MAX_REHEARSAL_BACKEND_RECEIPT_REFERENCE_SCALARS)
            )
            .is_ok()
        );
    }

    #[test]
    fn backend_receipt_reference_is_read_only_through_explicit_accessor() {
        let receipt = RehearsalBackendReceiptReference::new("provider-receipt".into())
            .expect("valid receipt");

        assert_eq!(receipt.as_str(), "provider-receipt");
    }
}
