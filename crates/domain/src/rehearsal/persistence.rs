//! Closed, private v1 persistence values for rehearsal storage.
//!
//! This module deliberately reconstructs private material through the domain
//! constructors. It is not a transport schema and none of its wire types are
//! exposed outside this module.

use std::io::{self, Write};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use uuid::Uuid;

use super::{
    RehearsalClaimRoot, RehearsalEvidencePayload, RehearsalPersistedClaimRoot,
    RehearsalSubjectFingerprint, RehearsalSubmissionRequestFingerprint,
    RehearsalValidatedSubmissionEvidence, RehearsalValidatedSubmissionRequest,
    frozen_response_schema_digest,
};

mod claim_input_codec;
mod receipt;

pub use claim_input_codec::{decode_claim_submission_input, encode_claim_submission_input};

use receipt::FeedbackWire;
pub use receipt::{
    decode_persisted_rehearsal_receipt, encode_persisted_rehearsal_receipt,
    persisted_rehearsal_receipt_digest,
};

pub const REHEARSAL_PERSISTENCE_CODEC_VERSION: u8 = 1;

/// The established course request ceiling for the identity-free subject JSON.
pub const MAX_REHEARSAL_SUBJECT_PAYLOAD_BYTES: usize = 64 * 1024;
/// The established flat-question publication body ceiling for a response definition.
pub const MAX_REHEARSAL_RESPONSE_DEFINITION_BYTES: usize = 256 * 1024;
/// Frozen-item envelope identifiers, digests, timestamp, and JSON punctuation above its definition.
pub const MAX_REHEARSAL_FROZEN_EVIDENCE_PAYLOAD_BYTES: usize =
    MAX_REHEARSAL_RESPONSE_DEFINITION_BYTES + 1024;
/// JSON can escape each private source byte as a six-byte `\\u00XX` sequence.
const MAX_REHEARSAL_PRIVATE_JSON_ESCAPED_BYTES: usize =
    6 * question_model::MAX_REHEARSAL_ACCEPTED_SUBMISSION_BYTES;
/// Sealed-request envelope overhead above the established private submission budget.
pub const MAX_REHEARSAL_SEALED_REQUEST_BYTES: usize =
    MAX_REHEARSAL_PRIVATE_JSON_ESCAPED_BYTES + 512;
/// Accepted-evidence envelope, UUID, timestamp, and grading discriminant overhead.
pub const MAX_REHEARSAL_ACCEPTED_EVIDENCE_PAYLOAD_BYTES: usize =
    MAX_REHEARSAL_PRIVATE_JSON_ESCAPED_BYTES + 2048;
/// Answer-safe receipt projection overhead above the same bounded feedback source material.
pub const MAX_REHEARSAL_RECEIPT_PROJECTION_BYTES: usize =
    MAX_REHEARSAL_PRIVATE_JSON_ESCAPED_BYTES + 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalPersistenceError {
    UnsupportedVersion,
    WrongPayloadKind,
    MalformedValue,
    BindingMismatch,
    TimestampMismatch,
    InvalidPrivateMaterial,
    ValueTooLarge,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SealedRequestWire {
    codec_version: u8,
    kind: String,
    submitted_response: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FrozenWire {
    codec_version: u8,
    kind: String,
    attempt_id: String,
    problem_id: String,
    version_id: String,
    response_definition: Value,
    canonical_content_digest: String,
    frozen_at_millis: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AcceptedWire {
    codec_version: u8,
    kind: String,
    claim_id: String,
    attempt_id: String,
    submitted_response: Value,
    grading: GradingWire,
    accepted_at_millis: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum GradingWire {
    Graded {
        attempt_result: Value,
        feedback: FeedbackWire,
        backend_receipt_reference: String,
    },
}

pub fn encode_sealed_request(request: &RehearsalValidatedSubmissionRequest) -> Value {
    to_value(&SealedRequestWire {
        codec_version: REHEARSAL_PERSISTENCE_CODEC_VERSION,
        kind: "sealedRequest".into(),
        submitted_response: to_value(request.submitted_response()),
    })
}

/// Strictly restores the identity-free subject retained with a rehearsal run.
pub fn decode_persisted_subject(
    value: &Value,
) -> Result<question_model::PreviewSubject, RehearsalPersistenceError> {
    decode_exact_limited(value, MAX_REHEARSAL_SUBJECT_PAYLOAD_BYTES)
}

pub fn decode_sealed_request(
    value: &Value,
    frozen: &question_model::RehearsalFrozenItemEvidence,
    expected_attempt: question_model::RehearsalAttemptId,
) -> Result<RehearsalValidatedSubmissionRequest, RehearsalPersistenceError> {
    let wire: SealedRequestWire = decode_exact_limited(value, MAX_REHEARSAL_SEALED_REQUEST_BYTES)?;
    require_header(wire.codec_version, &wire.kind, "sealedRequest")?;
    let response: question_model::StudentResponse =
        decode_exact_limited(&wire.submitted_response, MAX_REHEARSAL_SEALED_REQUEST_BYTES)?;
    RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(frozen, expected_attempt, response)
        .map_err(|_| RehearsalPersistenceError::InvalidPrivateMaterial)
}

pub fn encode_evidence_payload(payload: &RehearsalEvidencePayload) -> Value {
    match payload {
        RehearsalEvidencePayload::FrozenItem(item) => to_value(&FrozenWire {
            codec_version: REHEARSAL_PERSISTENCE_CODEC_VERSION,
            kind: "frozenItem".into(),
            attempt_id: item.attempt.as_uuid().to_string(),
            problem_id: item.problem.problem.as_uuid().to_string(),
            version_id: item.problem.version.as_uuid().to_string(),
            response_definition: to_value(&item.response_definition),
            canonical_content_digest: item.canonical_content_digest.to_hex(),
            frozen_at_millis: item.frozen_at.as_unix_millis(),
        }),
        RehearsalEvidencePayload::AcceptedSubmission(item) => to_value(&AcceptedWire {
            codec_version: REHEARSAL_PERSISTENCE_CODEC_VERSION,
            kind: "acceptedSubmission".into(),
            claim_id: item.claim_binding.claim.as_uuid().to_string(),
            attempt_id: item.attempt().as_uuid().to_string(),
            submitted_response: to_value(item.submitted_response()),
            grading: grading_to_wire(item.result()),
            accepted_at_millis: item.accepted_at().as_unix_millis(),
        }),
    }
}

pub fn decode_frozen_evidence_payload(
    value: &Value,
    frozen_row: &question_model::RehearsalFrozenItemEvidence,
    recorded_at: question_model::ActivityTimestamp,
) -> Result<RehearsalEvidencePayload, RehearsalPersistenceError> {
    let wire: FrozenWire =
        decode_exact_limited(value, MAX_REHEARSAL_FROZEN_EVIDENCE_PAYLOAD_BYTES)?;
    require_header(wire.codec_version, &wire.kind, "frozenItem")?;
    let decoded = question_model::RehearsalFrozenItemEvidence {
        attempt: parse_uuid(&wire.attempt_id).map(question_model::RehearsalAttemptId::from_uuid)?,
        problem: question_model::ProblemVersionRef {
            problem: parse_uuid(&wire.problem_id).map(question_model::ProblemId::from_uuid)?,
            version: parse_uuid(&wire.version_id).map(question_model::VersionId::from_uuid)?,
        },
        response_definition: decode_exact_limited(
            &wire.response_definition,
            MAX_REHEARSAL_RESPONSE_DEFINITION_BYTES,
        )?,
        canonical_content_digest: parse_digest(&wire.canonical_content_digest)?,
        frozen_at: question_model::ActivityTimestamp::from_unix_millis(wire.frozen_at_millis),
    };
    if decoded.frozen_at != recorded_at {
        return Err(RehearsalPersistenceError::TimestampMismatch);
    }
    if decoded != *frozen_row
        || frozen_response_schema_digest(&decoded.response_definition)
            != frozen_response_schema_digest(&frozen_row.response_definition)
    {
        return Err(RehearsalPersistenceError::BindingMismatch);
    }
    Ok(RehearsalEvidencePayload::FrozenItem(decoded))
}

pub fn decode_accepted_evidence_payload(
    value: &Value,
    root: &RehearsalClaimRoot,
    frozen: &question_model::RehearsalFrozenItemEvidence,
    recorded_at: question_model::ActivityTimestamp,
) -> Result<RehearsalEvidencePayload, RehearsalPersistenceError> {
    let wire: AcceptedWire =
        decode_exact_limited(value, MAX_REHEARSAL_ACCEPTED_EVIDENCE_PAYLOAD_BYTES)?;
    require_header(wire.codec_version, &wire.kind, "acceptedSubmission")?;
    if wire.accepted_at_millis != recorded_at.as_unix_millis() {
        return Err(RehearsalPersistenceError::TimestampMismatch);
    }
    if parse_uuid(&wire.claim_id)? != root.claim().as_uuid()
        || parse_uuid(&wire.attempt_id)? != frozen.attempt.as_uuid()
    {
        return Err(RehearsalPersistenceError::BindingMismatch);
    }
    let response: question_model::StudentResponse = decode_exact_limited(
        &wire.submitted_response,
        MAX_REHEARSAL_ACCEPTED_EVIDENCE_PAYLOAD_BYTES,
    )?;
    if response != *root.submission_input().original_response() {
        return Err(RehearsalPersistenceError::BindingMismatch);
    }
    let result = grading_from_wire(wire.grading)?;
    let evidence = RehearsalValidatedSubmissionEvidence::restore_with_verified_root(
        root,
        frozen,
        response,
        result,
        recorded_at,
    )
    .map_err(|_| RehearsalPersistenceError::InvalidPrivateMaterial)?;
    Ok(RehearsalEvidencePayload::AcceptedSubmission(evidence))
}

pub fn restore_subject_fingerprint(
    bytes: &[u8],
) -> Result<RehearsalSubjectFingerprint, RehearsalPersistenceError> {
    Ok(RehearsalSubjectFingerprint(copy_digest(bytes)?))
}

pub fn decode_persisted_claim_root(
    rehearsal: question_model::RehearsalRunId,
    claim: question_model::RehearsalSubmissionClaimId,
    fingerprint_bytes: &[u8],
    submission_input: &Value,
    frozen: &question_model::RehearsalFrozenItemEvidence,
    expected_attempt: question_model::RehearsalAttemptId,
) -> Result<RehearsalPersistedClaimRoot, RehearsalPersistenceError> {
    decode_persisted_claim_root_with_screen(
        rehearsal,
        claim,
        fingerprint_bytes,
        submission_input,
        frozen,
        expected_attempt,
        None,
    )
}

/// Restores a root with the exact authenticated screen required by a rendered
/// live claim. Durable internal/test-support rows remain decodable without a
/// screen through [`decode_persisted_claim_root`].
pub fn decode_persisted_claim_root_with_screen(
    rehearsal: question_model::RehearsalRunId,
    claim: question_model::RehearsalSubmissionClaimId,
    fingerprint_bytes: &[u8],
    submission_input: &Value,
    frozen: &question_model::RehearsalFrozenItemEvidence,
    expected_attempt: question_model::RehearsalAttemptId,
    screen: Option<&question_model::RehearsalActiveScreenV1>,
) -> Result<RehearsalPersistedClaimRoot, RehearsalPersistenceError> {
    let input = decode_claim_submission_input(submission_input, frozen, expected_attempt, screen)?;
    Ok(RehearsalPersistedClaimRoot::from_persisted(
        rehearsal,
        claim,
        RehearsalSubmissionRequestFingerprint(copy_digest(fingerprint_bytes)?),
        input,
    ))
}

/// Verifies the immutable private receipt witness against the domain result.
pub fn verify_persisted_receipt_witness(
    outcome: &question_model::RehearsalPublicOutcome,
    projection: &Value,
    digest: question_model::RehearsalEvidenceDigest,
) -> Result<(), RehearsalPersistenceError> {
    ensure_serialized_at_most(projection, MAX_REHEARSAL_RECEIPT_PROJECTION_BYTES)?;
    (encode_persisted_rehearsal_receipt(outcome) == *projection
        && persisted_rehearsal_receipt_digest(outcome) == digest)
        .then_some(())
        .ok_or(RehearsalPersistenceError::BindingMismatch)
}

fn grading_to_wire(value: &question_model::RehearsalPrivateGradingResult) -> GradingWire {
    let question_model::RehearsalPrivateGradingResult::Graded {
        result,
        feedback,
        backend_receipt_reference,
    } = value;
    GradingWire::Graded {
        attempt_result: to_value(result),
        feedback: FeedbackWire::from(feedback),
        backend_receipt_reference: backend_receipt_reference.as_str().into(),
    }
}

fn grading_from_wire(
    value: GradingWire,
) -> Result<question_model::RehearsalPrivateGradingResult, RehearsalPersistenceError> {
    match value {
        GradingWire::Graded {
            attempt_result,
            feedback,
            backend_receipt_reference,
        } => Ok(question_model::RehearsalPrivateGradingResult::Graded {
            result: decode_exact_limited(
                &attempt_result,
                MAX_REHEARSAL_ACCEPTED_EVIDENCE_PAYLOAD_BYTES,
            )?,
            feedback: feedback.into(),
            backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
                backend_receipt_reference,
            )
            .map_err(|_| RehearsalPersistenceError::InvalidPrivateMaterial)?,
        }),
    }
}

fn require_header(
    version: u8,
    actual: &str,
    expected: &str,
) -> Result<(), RehearsalPersistenceError> {
    if version != REHEARSAL_PERSISTENCE_CODEC_VERSION {
        return Err(RehearsalPersistenceError::UnsupportedVersion);
    }
    (actual == expected)
        .then_some(())
        .ok_or(RehearsalPersistenceError::WrongPayloadKind)
}

fn parse_uuid(value: &str) -> Result<Uuid, RehearsalPersistenceError> {
    let parsed = value
        .parse::<Uuid>()
        .map_err(|_| RehearsalPersistenceError::MalformedValue)?;
    (parsed.to_string() == value)
        .then_some(parsed)
        .ok_or(RehearsalPersistenceError::MalformedValue)
}

pub(super) fn parse_digest(
    value: &str,
) -> Result<question_model::RehearsalEvidenceDigest, RehearsalPersistenceError> {
    question_model::RehearsalEvidenceDigest::parse_hex(value)
        .map_err(|_| RehearsalPersistenceError::MalformedValue)
}

fn copy_digest(bytes: &[u8]) -> Result<[u8; 32], RehearsalPersistenceError> {
    bytes
        .try_into()
        .map_err(|_| RehearsalPersistenceError::MalformedValue)
}

pub(super) fn decode_exact_limited<T>(
    value: &Value,
    maximum_bytes: usize,
) -> Result<T, RehearsalPersistenceError>
where
    T: DeserializeOwned + Serialize,
{
    ensure_serialized_at_most(value, maximum_bytes)?;
    let decoded = serde_json::from_value(value.clone())
        .map_err(|_| RehearsalPersistenceError::MalformedValue)?;
    (to_value(&decoded) == *value)
        .then_some(decoded)
        .ok_or(RehearsalPersistenceError::MalformedValue)
}

/// Counts canonical JSON bytes without allocating a second serialized buffer.
fn ensure_serialized_at_most(
    value: &Value,
    maximum_bytes: usize,
) -> Result<(), RehearsalPersistenceError> {
    let mut writer = BoundedByteCounter::new(maximum_bytes);
    match serde_json::to_writer(&mut writer, value) {
        Ok(()) => Ok(()),
        Err(_) if writer.exceeded => Err(RehearsalPersistenceError::ValueTooLarge),
        Err(_) => Err(RehearsalPersistenceError::MalformedValue),
    }
}

struct BoundedByteCounter {
    maximum_bytes: usize,
    written: usize,
    exceeded: bool,
}

impl BoundedByteCounter {
    const fn new(maximum_bytes: usize) -> Self {
        Self {
            maximum_bytes,
            written: 0,
            exceeded: false,
        }
    }
}

impl Write for BoundedByteCounter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let Some(written) = self.written.checked_add(buffer.len()) else {
            self.exceeded = true;
            return Err(io::Error::other(
                "rehearsal persistence value exceeds byte ceiling",
            ));
        };
        if written > self.maximum_bytes {
            self.exceeded = true;
            return Err(io::Error::other(
                "rehearsal persistence value exceeds byte ceiling",
            ));
        }
        self.written = written;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn to_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("closed rehearsal persistence values serialize")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RehearsalGenesisContext, private_payload_digest, rehearsal_submission_request_fingerprint,
    };
    use question_model::{
        ActivityTimestamp, AssignmentReference, AttemptResult, CourseId, CourseMembershipId,
        ProblemId, ProblemVersionRef, RehearsalAttemptId, RehearsalRunId,
        RehearsalSubmissionClaimId, StudentResponse, TeachingOperationRevision, TenantId,
        VersionId,
    };

    fn frozen() -> question_model::RehearsalFrozenItemEvidence {
        question_model::RehearsalFrozenItemEvidence {
            attempt: RehearsalAttemptId::from_uuid(Uuid::from_u128(1)),
            problem: ProblemVersionRef {
                problem: ProblemId::from_uuid(Uuid::from_u128(2)),
                version: VersionId::from_uuid(Uuid::from_u128(3)),
            },
            response_definition: question_model::ResponseDefinition::Numeric {
                tolerance: question_model::answer::NumericTolerance::Exact,
                unit: None,
            },
            canonical_content_digest: question_model::RehearsalEvidenceDigest::from_bytes([4; 32]),
            frozen_at: ActivityTimestamp::from_unix_millis(10),
        }
    }

    fn context() -> RehearsalGenesisContext {
        RehearsalGenesisContext {
            rehearsal: RehearsalRunId::from_uuid(Uuid::from_u128(5)),
            tenant: TenantId::from_uuid(Uuid::from_u128(6)),
            course: CourseId::from_uuid(Uuid::from_u128(7)),
            assignment: AssignmentReference::new(8).unwrap(),
            direct_instructor_membership: CourseMembershipId::from_uuid(Uuid::from_u128(9)),
            revision: TeachingOperationRevision::new(1).unwrap(),
            subject_fingerprint: RehearsalSubjectFingerprint([10; 32]),
        }
    }

    fn request(
        item: &question_model::RehearsalFrozenItemEvidence,
    ) -> RehearsalValidatedSubmissionRequest {
        RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
            item,
            item.attempt,
            StudentResponse::Numeric { value: 2.0 },
        )
        .unwrap()
    }

    fn root(item: &question_model::RehearsalFrozenItemEvidence) -> RehearsalClaimRoot {
        let context = context();
        let sealed = request(item);
        let fingerprint = rehearsal_submission_request_fingerprint(context, item, &sealed).unwrap();
        RehearsalClaimRoot::verify_persisted(
            context,
            item,
            RehearsalPersistedClaimRoot::from_persisted(
                context.rehearsal,
                RehearsalSubmissionClaimId::from_uuid(Uuid::from_u128(11)),
                fingerprint,
                sealed,
            ),
        )
        .unwrap()
    }

    #[test]
    fn sealed_request_is_closed_and_reconstructed_against_frozen_item() {
        let item = frozen();
        let original = request(&item);
        let encoded = encode_sealed_request(&original);
        assert_eq!(
            encoded.as_object().unwrap().keys().collect::<Vec<_>>(),
            ["codecVersion", "kind", "submittedResponse"]
        );
        assert!(decode_sealed_request(&encoded, &item, item.attempt).unwrap() == original);
        for field in ["codecVersion", "kind", "submittedResponse", "unexpected"] {
            let mut tampered = encoded.clone();
            match field {
                "codecVersion" => tampered[field] = Value::from(2),
                "kind" => tampered[field] = Value::from("frozenItem"),
                "submittedResponse" => {
                    tampered[field] = serde_json::json!({ "kind": "shortText", "text": "3" })
                }
                _ => tampered[field] = Value::Null,
            }
            assert!(
                decode_sealed_request(&tampered, &item, item.attempt).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn frozen_payload_requires_exact_locked_row_and_record_timestamp() {
        let item = frozen();
        let encoded = encode_evidence_payload(&RehearsalEvidencePayload::FrozenItem(item.clone()));
        assert!(
            decode_frozen_evidence_payload(&encoded, &item, item.frozen_at).unwrap()
                == RehearsalEvidencePayload::FrozenItem(item.clone())
        );
        for field in [
            "attemptId",
            "problemId",
            "versionId",
            "responseDefinition",
            "canonicalContentDigest",
            "frozenAtMillis",
        ] {
            let mut tampered = encoded.clone();
            tampered[field] = match field {
                "canonicalContentDigest" => Value::from("a".repeat(64)),
                "frozenAtMillis" => Value::from(11),
                "responseDefinition" => serde_json::json!({
                    "kind": "shortText", "matchMode": "exact", "maxLength": 3
                }),
                _ => Value::from(Uuid::from_u128(99).to_string()),
            };
            assert!(
                decode_frozen_evidence_payload(&tampered, &item, item.frozen_at).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn accepted_payload_reconstructs_only_through_verified_root() {
        let item = frozen();
        let root = root(&item);
        let accepted_at = ActivityTimestamp::from_unix_millis(12);
        let evidence = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
            &root,
            root.submission_input().durable_request().unwrap().clone(),
            &item,
            question_model::RehearsalPrivateGradingResult::Graded {
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: question_model::DisclosedFeedback::empty(),
                backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
                    "native:1".into(),
                )
                .unwrap(),
            },
            accepted_at,
        )
        .unwrap();
        let encoded = encode_evidence_payload(&RehearsalEvidencePayload::AcceptedSubmission(
            evidence.clone(),
        ));
        assert!(
            decode_accepted_evidence_payload(&encoded, &root, &item, accepted_at).unwrap()
                == RehearsalEvidencePayload::AcceptedSubmission(evidence)
        );
        for field in [
            "claimId",
            "attemptId",
            "submittedResponse",
            "acceptedAtMillis",
            "grading",
        ] {
            let mut tampered = encoded.clone();
            tampered[field] = match field {
                "submittedResponse" => serde_json::json!({ "kind": "shortText", "text": "3" }),
                "acceptedAtMillis" => Value::from(13),
                "grading" => {
                    serde_json::json!({ "kind": "graded", "attemptResult": { "correct": true, "pointsEarned": 1.0, "pointsPossible": 1.0 }, "feedback": {}, "backendReceiptReference": "" })
                }
                _ => Value::from(Uuid::from_u128(99).to_string()),
            };
            assert!(
                decode_accepted_evidence_payload(&tampered, &root, &item, accepted_at).is_err(),
                "{field}"
            );
        }
        let mut stale_manual = encoded.clone();
        stale_manual["grading"] = serde_json::json!({ "kind": "needsManualGrading" });
        assert!(
            decode_accepted_evidence_payload(&stale_manual, &root, &item, accepted_at).is_err(),
            "stale manual grading persistence is rejected without a compatibility reader"
        );
        assert_eq!(
            encoded.as_object().unwrap().keys().collect::<Vec<_>>(),
            [
                "acceptedAtMillis",
                "attemptId",
                "claimId",
                "codecVersion",
                "grading",
                "kind",
                "submittedResponse"
            ]
        );
    }

    #[test]
    fn private_codec_rejects_bound_excess_before_deserialization() {
        let item = frozen();
        let original = request(&item);
        let mut sealed = encode_sealed_request(&original);
        sealed["submittedResponse"] = serde_json::json!({
            "kind": "shortText", "text": "x".repeat(MAX_REHEARSAL_SEALED_REQUEST_BYTES)
        });
        assert!(matches!(
            decode_sealed_request(&sealed, &item, item.attempt),
            Err(RehearsalPersistenceError::ValueTooLarge)
        ));

        let mut frozen_payload =
            encode_evidence_payload(&RehearsalEvidencePayload::FrozenItem(item.clone()));
        frozen_payload["responseDefinition"] = serde_json::json!({
            "kind": "numeric", "tolerance": "exact",
            "unit": "x".repeat(MAX_REHEARSAL_FROZEN_EVIDENCE_PAYLOAD_BYTES)
        });
        assert!(matches!(
            decode_frozen_evidence_payload(&frozen_payload, &item, item.frozen_at),
            Err(RehearsalPersistenceError::ValueTooLarge)
        ));

        let root = root(&item);
        let accepted_at = ActivityTimestamp::from_unix_millis(12);
        let evidence = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
            &root,
            root.submission_input().durable_request().unwrap().clone(),
            &item,
            question_model::RehearsalPrivateGradingResult::Graded {
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: question_model::DisclosedFeedback::empty(),
                backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
                    "native:1".into(),
                )
                .unwrap(),
            },
            accepted_at,
        )
        .unwrap();
        let accepted =
            encode_evidence_payload(&RehearsalEvidencePayload::AcceptedSubmission(evidence));
        for field in [
            "submittedResponse",
            "gradingFeedback",
            "backendReceiptReference",
        ] {
            let mut oversized = accepted.clone();
            match field {
                "submittedResponse" => {
                    oversized["submittedResponse"] = serde_json::json!({
                        "kind": "shortText", "text": "x".repeat(MAX_REHEARSAL_ACCEPTED_EVIDENCE_PAYLOAD_BYTES)
                    })
                }
                "gradingFeedback" => {
                    oversized["grading"]["feedback"] = serde_json::json!({
                        "hint": [{"kind": "text", "markdown": "x".repeat(MAX_REHEARSAL_ACCEPTED_EVIDENCE_PAYLOAD_BYTES)}]
                    })
                }
                _ => {
                    oversized["grading"]["backendReceiptReference"] =
                        Value::from("x".repeat(MAX_REHEARSAL_ACCEPTED_EVIDENCE_PAYLOAD_BYTES))
                }
            }
            assert!(
                matches!(
                    decode_accepted_evidence_payload(&oversized, &root, &item, accepted_at),
                    Err(RehearsalPersistenceError::ValueTooLarge)
                ),
                "{field}"
            );
        }
        let receipt = serde_json::json!({
            "kind": "submitted",
            "feedback": {"hint": [{"kind": "text", "markdown": "x".repeat(MAX_REHEARSAL_RECEIPT_PROJECTION_BYTES)}]}
        });
        assert_eq!(
            verify_persisted_receipt_witness(
                &question_model::RehearsalPublicOutcome::Submitted {
                    feedback: question_model::DisclosedFeedback::empty(),
                },
                &receipt,
                question_model::RehearsalEvidenceDigest::from_bytes([1; 32]),
            ),
            Err(RehearsalPersistenceError::ValueTooLarge)
        );
    }

    #[test]
    fn altered_private_values_require_their_own_evidence_chain_commitment() {
        let item = frozen();
        let root = root(&item);
        let accepted_at = ActivityTimestamp::from_unix_millis(12);
        let evidence = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
            &root,
            root.submission_input().durable_request().unwrap().clone(),
            &item,
            question_model::RehearsalPrivateGradingResult::Graded {
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: question_model::DisclosedFeedback::empty(),
                backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
                    "native:1".into(),
                )
                .unwrap(),
            },
            accepted_at,
        )
        .unwrap();
        let encoded = encode_evidence_payload(&RehearsalEvidencePayload::AcceptedSubmission(
            evidence.clone(),
        ));
        assert_eq!(
            encoded.as_object().unwrap().keys().collect::<Vec<_>>(),
            [
                "acceptedAtMillis",
                "attemptId",
                "claimId",
                "codecVersion",
                "grading",
                "kind",
                "submittedResponse"
            ]
        );
        let baseline =
            private_payload_digest(&RehearsalEvidencePayload::AcceptedSubmission(evidence));
        for (path, replacement) in [
            (
                "attemptResult",
                serde_json::json!({"correct": true, "pointsEarned": 0.5, "pointsPossible": 1.0}),
            ),
            (
                "feedback",
                serde_json::json!({"hint": [{"kind": "text", "markdown": "review"}]}),
            ),
            ("backendReceiptReference", Value::from("native:2")),
        ] {
            let mut changed = encoded.clone();
            changed["grading"][path] = replacement;
            let restored =
                decode_accepted_evidence_payload(&changed, &root, &item, accepted_at).unwrap();
            assert_ne!(private_payload_digest(&restored), baseline, "{path}");
        }

        let persisted = decode_persisted_claim_root(
            context().rehearsal,
            RehearsalSubmissionClaimId::from_uuid(Uuid::from_u128(11)),
            &[99; 32],
            &encode_claim_submission_input(root.submission_input()),
            &item,
            item.attempt,
        )
        .unwrap();
        assert!(RehearsalClaimRoot::verify_persisted(context(), &item, persisted).is_err());
    }

    #[test]
    fn receipt_witness_is_exact_and_subject_is_bounded() {
        let outcome = question_model::RehearsalPublicOutcome::Submitted {
            feedback: question_model::DisclosedFeedback::empty(),
        };
        let projection = encode_persisted_rehearsal_receipt(&outcome);
        let digest = persisted_rehearsal_receipt_digest(&outcome);
        assert!(verify_persisted_receipt_witness(&outcome, &projection, digest).is_ok());
        assert!(
            verify_persisted_receipt_witness(
                &outcome,
                &serde_json::json!({"kind": "attemptExpired"}),
                digest
            )
            .is_err()
        );
        assert!(
            verify_persisted_receipt_witness(
                &outcome,
                &projection,
                question_model::RehearsalEvidenceDigest::from_bytes([99; 32])
            )
            .is_err()
        );
        assert_eq!(
            decode_persisted_subject(
                &serde_json::json!({"x": "x".repeat(MAX_REHEARSAL_SUBJECT_PAYLOAD_BYTES)})
            ),
            Err(RehearsalPersistenceError::ValueTooLarge)
        );
    }

    #[test]
    fn maximum_deterministic_private_submission_round_trips() {
        let mut item = frozen();
        item.response_definition = question_model::ResponseDefinition::ShortText {
            match_mode: question_model::answer::TextMatchMode::Exact,
            max_length: question_model::MAX_REHEARSAL_ACCEPTED_SUBMISSION_BYTES as u32,
        };
        let sealed = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
            &item,
            item.attempt,
            StudentResponse::ShortText {
                text: "x".repeat(question_model::MAX_REHEARSAL_ACCEPTED_SUBMISSION_BYTES - 256),
            },
        )
        .unwrap();
        assert!(
            decode_sealed_request(&encode_sealed_request(&sealed), &item, item.attempt).unwrap()
                == sealed
        );
        let root = RehearsalClaimRoot::verify_persisted(
            context(),
            &item,
            RehearsalPersistedClaimRoot::from_persisted(
                context().rehearsal,
                RehearsalSubmissionClaimId::from_uuid(Uuid::from_u128(11)),
                rehearsal_submission_request_fingerprint(context(), &item, &sealed).unwrap(),
                sealed,
            ),
        )
        .unwrap();
        let accepted = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
            &root,
            root.submission_input().durable_request().unwrap().clone(),
            &item,
            question_model::RehearsalPrivateGradingResult::Graded {
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: question_model::DisclosedFeedback::empty(),
                backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
                    "native:maximum".into(),
                )
                .unwrap(),
            },
            ActivityTimestamp::from_unix_millis(12),
        )
        .unwrap();
        let encoded = encode_evidence_payload(&RehearsalEvidencePayload::AcceptedSubmission(
            accepted.clone(),
        ));
        assert!(
            decode_accepted_evidence_payload(
                &encoded,
                &root,
                &item,
                ActivityTimestamp::from_unix_millis(12),
            )
            .unwrap()
                == RehearsalEvidencePayload::AcceptedSubmission(accepted)
        );
    }

    #[test]
    fn fingerprints_and_persisted_receipt_digests_are_stable_private_values() {
        assert_eq!(
            restore_subject_fingerprint(&[1; 32]).unwrap().as_bytes(),
            [1; 32]
        );
        assert!(restore_subject_fingerprint(&[1; 31]).is_err());
        let outcome = question_model::RehearsalPublicOutcome::Submitted {
            feedback: question_model::DisclosedFeedback::empty(),
        };
        assert_eq!(
            persisted_rehearsal_receipt_digest(&outcome).to_hex(),
            "1ca820d2bc482730ff63185bd1b7c7e4e3eb5f7c832970bdf288ed9041b4abfa"
        );
    }

    #[test]
    fn persisted_receipt_wire_has_the_closed_v1_operation_result_shape() {
        let cases = [
            (
                question_model::RehearsalPublicOutcome::Submitted {
                    feedback: question_model::DisclosedFeedback::empty(),
                },
                serde_json::json!({"kind": "submitted", "feedback": {}}),
            ),
            (
                question_model::RehearsalPublicOutcome::AttemptExpired,
                serde_json::json!({"kind": "attemptExpired"}),
            ),
            (
                question_model::RehearsalPublicOutcome::SubmissionPending,
                serde_json::json!({"kind": "submissionPending"}),
            ),
            (
                question_model::RehearsalPublicOutcome::StaleRevision,
                serde_json::json!({"kind": "staleRevision"}),
            ),
            (
                question_model::RehearsalPublicOutcome::DeliveryUnsupported {
                    support: question_model::RehearsalBackendSupport::UnsupportedExternal,
                },
                serde_json::json!({
                    "kind": "deliveryUnsupported",
                    "support": "unsupportedExternal"
                }),
            ),
        ];

        for (outcome, expected) in cases {
            let projection = encode_persisted_rehearsal_receipt(&outcome);
            assert_eq!(projection, expected);
            assert_eq!(
                serde_json::to_vec(&projection).expect("JSON value serializes"),
                serde_json::to_vec(&expected).expect("JSON value serializes"),
            );
        }
    }
}
