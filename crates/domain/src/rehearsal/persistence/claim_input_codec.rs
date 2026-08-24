//! Closed persistence codec for the private tagged rehearsal claim input.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    MAX_REHEARSAL_SEALED_REQUEST_BYTES, REHEARSAL_PERSISTENCE_CODEC_VERSION,
    RehearsalPersistenceError, decode_exact_limited, parse_digest, to_value,
};
use crate::{RehearsalClaimSubmissionInput, RehearsalValidatedSubmissionRequest};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ClaimSubmissionInputWire {
    codec_version: u8,
    kind: ClaimSubmissionInputKindWire,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum ClaimSubmissionInputKindWire {
    Rendered {
        presentation_commitment: String,
        response: Value,
    },
    Durable {
        submitted_response: Value,
    },
}

/// Encodes the closed private claim-input union. This is storage-only data,
/// never a browser contract and never a rendered-to-durable mapping.
pub fn encode_claim_submission_input(input: &RehearsalClaimSubmissionInput) -> Value {
    let kind = match input {
        RehearsalClaimSubmissionInput::Rendered(value) => ClaimSubmissionInputKindWire::Rendered {
            presentation_commitment: question_model::RehearsalEvidenceDigest::from_bytes(
                value.presentation_commitment().as_bytes(),
            )
            .to_hex(),
            response: to_value(value.response()),
        },
        RehearsalClaimSubmissionInput::Durable(value) => ClaimSubmissionInputKindWire::Durable {
            submitted_response: to_value(value.submitted_response()),
        },
    };
    to_value(&ClaimSubmissionInputWire {
        codec_version: REHEARSAL_PERSISTENCE_CODEC_VERSION,
        kind,
    })
}

/// Restores a private input. Rendered input needs the authenticated active
/// screen because its full presentation commitment and public response shape
/// must be re-established before it becomes a claim capability.
pub fn decode_claim_submission_input(
    value: &Value,
    frozen: &question_model::RehearsalFrozenItemEvidence,
    expected_attempt: question_model::RehearsalAttemptId,
    screen: Option<&question_model::RehearsalActiveScreenV1>,
) -> Result<RehearsalClaimSubmissionInput, RehearsalPersistenceError> {
    let wire: ClaimSubmissionInputWire =
        decode_exact_limited(value, MAX_REHEARSAL_SEALED_REQUEST_BYTES)?;
    if wire.codec_version != REHEARSAL_PERSISTENCE_CODEC_VERSION {
        return Err(RehearsalPersistenceError::UnsupportedVersion);
    }
    match wire.kind {
        ClaimSubmissionInputKindWire::Durable { submitted_response } => {
            let response =
                decode_exact_limited(&submitted_response, MAX_REHEARSAL_SEALED_REQUEST_BYTES)?;
            let request = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
                frozen,
                expected_attempt,
                response,
            )
            .map_err(|_| RehearsalPersistenceError::InvalidPrivateMaterial)?;
            Ok(RehearsalClaimSubmissionInput::durable(request))
        }
        ClaimSubmissionInputKindWire::Rendered {
            presentation_commitment,
            response,
        } => {
            let screen = screen.ok_or(RehearsalPersistenceError::BindingMismatch)?;
            let commitment = screen
                .commitment()
                .map_err(|_| RehearsalPersistenceError::InvalidPrivateMaterial)?;
            if parse_digest(&presentation_commitment)?.as_bytes() != commitment.as_bytes() {
                return Err(RehearsalPersistenceError::BindingMismatch);
            }
            let response = decode_exact_limited(&response, MAX_REHEARSAL_SEALED_REQUEST_BYTES)?;
            let request = question_model::RehearsalSubmissionRequestV1 {
                presentation_digest: screen.presentation_digest.clone(),
                response,
            };
            let rendered =
                question_model::ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(
                    request, screen,
                )
                .map_err(|_| RehearsalPersistenceError::InvalidPrivateMaterial)?;
            Ok(RehearsalClaimSubmissionInput::rendered(rendered))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rehearsal::persistence::{
        decode_accepted_evidence_payload, encode_evidence_payload, restore_subject_fingerprint,
    };
    use crate::{
        RehearsalClaimRoot, RehearsalGenesisContext, RehearsalPersistedClaimRoot,
        rehearsal_claim_submission_input_fingerprint,
    };
    use question_model::{
        ActivityTimestamp, AssignmentReference, AttemptResult, CourseId, CourseMembershipId,
        ProblemId, ProblemVersionRef, RehearsalContentBlockV1, RehearsalPresentedChoiceV1,
        RehearsalQuestionPresentationV1, RehearsalResponseSchemaV1, RehearsalRunId,
        RehearsalSubmissionClaimId, StudentResponse, TeachingOperationRevision, TenantId,
        VersionId,
    };
    use uuid::Uuid;

    fn screen() -> question_model::RehearsalActiveScreenV1 {
        question_model::RehearsalActiveScreenV1::new(RehearsalQuestionPresentationV1 {
            title: "Rendered claim".into(),
            prompt: vec![RehearsalContentBlockV1::Text {
                markdown: "Choose one".into(),
            }],
            response: RehearsalResponseSchemaV1::SingleChoice {
                choices: ["0001", "0002"]
                    .into_iter()
                    .map(|id| RehearsalPresentedChoiceV1 {
                        id: question_model::RenderedItemIdV1::parse(id).unwrap(),
                        body: vec![RehearsalContentBlockV1::Text {
                            markdown: id.into(),
                        }],
                    })
                    .collect(),
            },
        })
        .unwrap()
    }

    fn frozen() -> question_model::RehearsalFrozenItemEvidence {
        question_model::RehearsalFrozenItemEvidence {
            attempt: question_model::RehearsalAttemptId::from_uuid(Uuid::from_u128(1)),
            problem: ProblemVersionRef {
                problem: ProblemId::from_uuid(Uuid::from_u128(2)),
                version: VersionId::from_uuid(Uuid::from_u128(3)),
            },
            response_definition: question_model::ResponseDefinition::Numeric {
                tolerance: question_model::answer::NumericTolerance::Exact,
                unit: None,
            },
            canonical_content_digest: question_model::RehearsalEvidenceDigest::from_bytes([4; 32]),
            frozen_at: ActivityTimestamp::from_unix_millis(5),
        }
    }

    fn context() -> RehearsalGenesisContext {
        RehearsalGenesisContext {
            rehearsal: RehearsalRunId::from_uuid(Uuid::from_u128(6)),
            tenant: TenantId::from_uuid(Uuid::from_u128(7)),
            course: CourseId::from_uuid(Uuid::from_u128(8)),
            assignment: AssignmentReference::new(1).unwrap(),
            direct_instructor_membership: CourseMembershipId::from_uuid(Uuid::from_u128(9)),
            revision: TeachingOperationRevision::new(1).unwrap(),
            subject_fingerprint: restore_subject_fingerprint(&[10; 32]).unwrap(),
        }
    }

    #[test]
    fn rendered_input_requires_the_authenticated_screen_and_exact_commitment() {
        let screen = screen();
        let frozen = frozen();
        let input = RehearsalClaimSubmissionInput::rendered(
            question_model::ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(
                question_model::RehearsalSubmissionRequestV1 {
                    presentation_digest: screen.presentation_digest.clone(),
                    response: StudentResponse::MultipleChoice {
                        selected: vec![question_model::response::ChoiceId::new("0002")],
                    },
                },
                &screen,
            )
            .unwrap(),
        );
        let encoded = encode_claim_submission_input(&input);
        assert!(matches!(
            decode_claim_submission_input(&encoded, &frozen, frozen.attempt, None),
            Err(RehearsalPersistenceError::BindingMismatch)
        ));
        let restored =
            decode_claim_submission_input(&encoded, &frozen, frozen.attempt, Some(&screen))
                .unwrap();
        assert!(restored == input);
        let mut tampered = encoded;
        tampered["kind"]["presentationCommitment"] = Value::from("a".repeat(64));
        assert!(
            decode_claim_submission_input(&tampered, &frozen, frozen.attempt, Some(&screen))
                .is_err()
        );
    }

    #[test]
    fn rendered_completed_evidence_hydrates_without_persisting_translation() {
        let screen = screen();
        let frozen = frozen();
        let input = RehearsalClaimSubmissionInput::rendered(
            question_model::ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(
                question_model::RehearsalSubmissionRequestV1 {
                    presentation_digest: screen.presentation_digest.clone(),
                    response: StudentResponse::MultipleChoice {
                        selected: vec![question_model::response::ChoiceId::new("0002")],
                    },
                },
                &screen,
            )
            .unwrap(),
        );
        let fingerprint =
            rehearsal_claim_submission_input_fingerprint(context(), &frozen, &input).unwrap();
        let root = RehearsalClaimRoot::verify_persisted(
            context(),
            &frozen,
            RehearsalPersistedClaimRoot::from_persisted(
                context().rehearsal,
                RehearsalSubmissionClaimId::from_uuid(Uuid::from_u128(11)),
                fingerprint,
                input,
            ),
        )
        .unwrap();
        let result = question_model::RehearsalPrivateGradingResult::Graded {
            result: AttemptResult {
                correct: true,
                points_earned: 1.0,
                points_possible: 1.0,
            },
            feedback: question_model::DisclosedFeedback::empty(),
            backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
                "native:rendered".into(),
            )
            .unwrap(),
        };
        let evidence = crate::RehearsalValidatedSubmissionEvidence::restore_with_verified_root(
            &root,
            &frozen,
            root.submission_input().original_response().clone(),
            result,
            ActivityTimestamp::from_unix_millis(12),
        )
        .unwrap();
        let encoded = encode_evidence_payload(
            &crate::RehearsalEvidencePayload::AcceptedSubmission(evidence.clone()),
        );
        assert!(
            decode_accepted_evidence_payload(
                &encoded,
                &root,
                &frozen,
                ActivityTimestamp::from_unix_millis(12),
            )
            .unwrap()
                == crate::RehearsalEvidencePayload::AcceptedSubmission(evidence)
        );
        let mut response_tamper = encoded.clone();
        response_tamper["submittedResponse"] = serde_json::json!({
            "kind": "multipleChoice", "selected": ["0001"]
        });
        assert!(
            decode_accepted_evidence_payload(
                &response_tamper,
                &root,
                &frozen,
                ActivityTimestamp::from_unix_millis(12),
            )
            .is_err()
        );
        let mut attempt_tamper = encoded;
        attempt_tamper["attemptId"] = Value::from(Uuid::from_u128(99).to_string());
        assert!(
            decode_accepted_evidence_payload(
                &attempt_tamper,
                &root,
                &frozen,
                ActivityTimestamp::from_unix_millis(12),
            )
            .is_err()
        );
    }
}
