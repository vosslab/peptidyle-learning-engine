//! Closed durable receipt projection for internal rehearsal operation results.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::{Encoder, digest, encode_feedback};
use question_model::envelope::ContentBlock;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FeedbackWire {
    #[serde(skip_serializing_if = "Option::is_none")]
    correctness: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    points_earned: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    points_possible: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<Vec<ContentBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    correct_response: Option<Vec<ContentBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rationale: Option<Vec<ContentBlock>>,
}

/// The closed v1 durable projection. It preserves the JSON written by the
/// former serializable model enum while remaining inaccessible to route DTOs.
#[derive(Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum PersistedRehearsalOutcomeWireV1 {
    Submitted {
        feedback: FeedbackWire,
    },
    AttemptExpired,
    SubmissionPending,
    StaleRevision,
    DeliveryUnsupported {
        support: question_model::RehearsalBackendSupport,
    },
}

impl From<&question_model::RehearsalPublicOutcome> for PersistedRehearsalOutcomeWireV1 {
    fn from(value: &question_model::RehearsalPublicOutcome) -> Self {
        match value {
            question_model::RehearsalPublicOutcome::Submitted { feedback } => Self::Submitted {
                feedback: FeedbackWire::from(feedback),
            },
            question_model::RehearsalPublicOutcome::AttemptExpired => Self::AttemptExpired,
            question_model::RehearsalPublicOutcome::SubmissionPending => Self::SubmissionPending,
            question_model::RehearsalPublicOutcome::StaleRevision => Self::StaleRevision,
            question_model::RehearsalPublicOutcome::DeliveryUnsupported { support } => {
                Self::DeliveryUnsupported { support: *support }
            }
        }
    }
}

impl From<&question_model::DisclosedFeedback> for FeedbackWire {
    fn from(value: &question_model::DisclosedFeedback) -> Self {
        Self {
            correctness: value.correctness,
            points_earned: value.points_earned,
            points_possible: value.points_possible,
            hint: value.hint.clone(),
            correct_response: value.correct_response.clone(),
            rationale: value.rationale.clone(),
        }
    }
}

impl From<FeedbackWire> for question_model::DisclosedFeedback {
    fn from(value: FeedbackWire) -> Self {
        Self {
            correctness: value.correctness,
            points_earned: value.points_earned,
            points_possible: value.points_possible,
            hint: value.hint,
            correct_response: value.correct_response,
            rationale: value.rationale,
        }
    }
}

/// Projects an internal result into its private durable receipt wire.
pub fn encode_persisted_rehearsal_receipt(
    outcome: &question_model::RehearsalPublicOutcome,
) -> Value {
    serde_json::to_value(PersistedRehearsalOutcomeWireV1::from(outcome))
        .expect("closed rehearsal receipt wire serializes")
}

/// Strictly restores the closed receipt projection retained by a completed
/// claim.  Route replay uses this before it considers frozen material or a
/// grader, then recomputes its independent digest witness.
pub fn decode_persisted_rehearsal_receipt(
    value: &Value,
) -> Result<question_model::RehearsalPublicOutcome, super::RehearsalPersistenceError> {
    let wire: PersistedRehearsalOutcomeWireV1 = serde_json::from_value(value.clone())
        .map_err(|_| super::RehearsalPersistenceError::MalformedValue)?;
    match wire {
        PersistedRehearsalOutcomeWireV1::Submitted { feedback } => {
            Ok(question_model::RehearsalPublicOutcome::Submitted {
                feedback: feedback.into(),
            })
        }
        PersistedRehearsalOutcomeWireV1::AttemptExpired
        | PersistedRehearsalOutcomeWireV1::SubmissionPending
        | PersistedRehearsalOutcomeWireV1::StaleRevision
        | PersistedRehearsalOutcomeWireV1::DeliveryUnsupported { .. } => {
            Err(super::RehearsalPersistenceError::WrongPayloadKind)
        }
    }
}

/// Canonically digests the private durable receipt for one accepted submission.
pub fn persisted_rehearsal_receipt_digest(
    outcome: &question_model::RehearsalPublicOutcome,
) -> question_model::RehearsalEvidenceDigest {
    let mut encoder = Encoder::new();
    match outcome {
        question_model::RehearsalPublicOutcome::Submitted { feedback } => {
            encoder.u8(1);
            encode_feedback(&mut encoder, feedback);
        }
        _ => unreachable!("accepted rehearsal evidence has only submitted receipts"),
    }
    digest(b"ple:rehearsal:persisted-receipt:v1\0", encoder.finish())
}
