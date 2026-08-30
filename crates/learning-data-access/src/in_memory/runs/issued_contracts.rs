//! Immutable issue-contract readers shared by submission, receipt, and GET.

use question_model::QuestionAttempt;

use crate::{
    FlatGradingCapability, IssuedQuestionSnapshotV1, QtiGradingCapability,
    ReceiptPresentationSnapshot, StoreError, SubmissionReceiptRead, WebworkGradingCapability,
};

use super::super::State;

/// Validates immutable issued-source evidence before Memory exposes it to any
/// first-effect path.  The corresponding PostgreSQL decoder performs the
/// same identity and capability checks after checksum verification.
pub(crate) fn validate_issued_question_snapshot(
    snapshot: &IssuedQuestionSnapshotV1,
    attempt: &QuestionAttempt,
    flat_grading: FlatGradingCapability,
    webwork_grading: WebworkGradingCapability,
    qti_grading: QtiGradingCapability,
    presentation: Option<&ReceiptPresentationSnapshot>,
) -> Result<(), StoreError> {
    snapshot.validate_for_attempt(attempt.problem, attempt.question_version)?;
    snapshot.validate_for_issuance_context(flat_grading, webwork_grading, qti_grading, presentation)
}

/// The immutable issue record, rather than current catalog state, decides
/// whether a first submission must carry an answer-free native envelope.
pub(crate) fn load_issued_presentation(
    state: &State,
    attempt: &QuestionAttempt,
) -> Result<Option<ReceiptPresentationSnapshot>, StoreError> {
    let snapshot = load_issued_receipt_evidence(state, attempt)?;
    load_issued_flat_grading(state, attempt)?;
    load_issued_webwork_grading(state, attempt)?;
    Ok(snapshot)
}

/// Reads only the immutable common receipt tuple.  Submitted delivery uses
/// this seam after active-only grading and replay authority has been removed;
/// neither route can ask mutable catalog/renderer state to replace it.
pub(crate) fn load_issued_receipt_evidence(
    state: &State,
    attempt: &QuestionAttempt,
) -> Result<Option<ReceiptPresentationSnapshot>, StoreError> {
    let capability = state
        .attempt_presentation_capabilities
        .get(&attempt.id)
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt presentation capability is missing".to_string())
        })?;
    let flat_capability = state
        .attempt_flat_grading_capabilities
        .get(&attempt.id)
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt flat grading capability is missing".to_string())
        })?;
    let webwork_capability = state
        .attempt_webwork_grading_capabilities
        .get(&attempt.id)
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt WeBWorK grading capability is missing".to_string())
        })?;
    let qti_capability = state
        .attempt_qti_grading_capabilities
        .get(&attempt.id)
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt QTI grading capability is missing".to_string())
        })?;
    crate::validate_attempt_issuance_capability(
        attempt,
        capability,
        flat_capability,
        webwork_capability,
        qti_capability,
    )?;
    let binding = state.attempt_presentations.get(&attempt.id).copied();
    let snapshot = state.attempt_presentation_snapshots.get(&attempt.id);
    let grading_envelope = state.attempt_grading_envelopes.get(&attempt.id);
    let snapshot = crate::validate_issued_presentation(
        capability,
        attempt,
        binding,
        snapshot,
        grading_envelope,
    )?;
    Ok(snapshot)
}

pub(crate) fn load_issued_qti_grading(
    state: &State,
    attempt: &QuestionAttempt,
    snapshot: &IssuedQuestionSnapshotV1,
) -> Result<Option<crate::IssuedQtiGradingContractV1>, StoreError> {
    let capability = state
        .attempt_qti_grading_capabilities
        .get(&attempt.id)
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt QTI grading capability is missing".to_string())
        })?;
    let contract = state.attempt_qti_grading.get(&attempt.id);
    let expected = matches!(
        attempt.issued_capability,
        question_model::IssuedAttemptCapabilityV1::QtiPresentation
    );
    if capability.requires_contract() != expected {
        return Err(StoreError::Unavailable(
            "stored QTI grading capability disagrees with its checksummed attempt".to_string(),
        ));
    }
    match (capability, contract) {
        (QtiGradingCapability::NotApplicable, None) => Ok(None),
        (QtiGradingCapability::Required, Some(contract)) => {
            contract.validate_for_question(snapshot.question())?;
            Ok(Some(contract.clone()))
        }
        _ => Err(StoreError::Unavailable(
            "stored QTI grading capability and payload disagree".to_string(),
        )),
    }
}

/// Reads the explicit family obligation and validates the retained private
/// flat authority. The nullable payload never decides whether it was required.
pub(crate) fn load_issued_flat_grading(
    state: &State,
    attempt: &QuestionAttempt,
) -> Result<Option<crate::IssuedFlatGradingContract>, StoreError> {
    let capability = state
        .attempt_flat_grading_capabilities
        .get(&attempt.id)
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt flat grading capability is missing".to_string())
        })?;
    let contract = state.attempt_flat_grading.get(&attempt.id);
    let expected_required = matches!(
        attempt.issued_capability,
        question_model::IssuedAttemptCapabilityV1::FlatPresentation
    );
    if capability.requires_contract() != expected_required {
        return Err(StoreError::Unavailable(
            "stored flat grading capability disagrees with its checksummed attempt".to_string(),
        ));
    }
    match (capability, contract) {
        (crate::FlatGradingCapability::NotApplicable, None) => Ok(None),
        (crate::FlatGradingCapability::Required, Some(contract)) => {
            contract.validate()?;
            if contract.question().problem != attempt.problem
                || contract.question().version != attempt.question_version
            {
                return Err(StoreError::Unavailable(
                    "stored private flat grading disagrees with its attempt".to_string(),
                ));
            }
            Ok(Some(contract.clone()))
        }
        _ => Err(StoreError::Unavailable(
            "stored private flat grading capability and payload disagree".to_string(),
        )),
    }
}

/// Reads the explicit WeBWorK obligation and validates the retained immutable
/// definition. It never consults current catalog state.
pub(crate) fn load_issued_webwork_grading(
    state: &State,
    attempt: &QuestionAttempt,
) -> Result<Option<crate::IssuedWebworkGradingContract>, StoreError> {
    let capability = state
        .attempt_webwork_grading_capabilities
        .get(&attempt.id)
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt WeBWorK grading capability is missing".to_string())
        })?;
    let contract = state.attempt_webwork_grading.get(&attempt.id);
    let expected_required = matches!(
        attempt.issued_capability,
        question_model::IssuedAttemptCapabilityV1::WebworkPresentation
    );
    if capability.requires_contract() != expected_required {
        return Err(StoreError::Unavailable(
            "stored WeBWorK grading capability disagrees with its checksummed attempt".to_string(),
        ));
    }
    match (capability, contract) {
        (crate::WebworkGradingCapability::NotApplicable, None) => Ok(None),
        (crate::WebworkGradingCapability::Required, Some(contract)) => {
            contract.validate_for_attempt(attempt)?;
            Ok(Some(contract.clone()))
        }
        _ => Err(StoreError::Unavailable(
            "stored WeBWorK grading capability and payload disagree".to_string(),
        )),
    }
}

/// Returns a durable receipt only when its answer-free view is identical to
/// the immutable issue snapshot. Both receipt GET and idempotent replay use
/// this seam, so neither can turn a checksum-valid partial write into a new
/// learner-visible presentation.
pub(crate) fn load_submission_record(
    state: &State,
    attempt: &QuestionAttempt,
) -> Result<SubmissionReceiptRead, StoreError> {
    let Some(stored) = state.submissions.get(&attempt.id) else {
        return Ok(SubmissionReceiptRead::Missing);
    };
    if stored.accepted_pending().is_some() {
        return Ok(SubmissionReceiptRead::AcceptedPending(
            crate::AcceptedSubmissionPending::new(attempt.id),
        ));
    }
    let issued_presentation = load_issued_presentation(state, attempt)?;
    let completed = stored.completed_record_opt().ok_or_else(|| {
        StoreError::Unavailable("completed submission receipt is missing".to_string())
    })?;
    if completed.presentation != issued_presentation {
        return Err(StoreError::Unavailable(
            "submission receipt presentation does not match its issued snapshot".to_string(),
        ));
    }
    let run = state.runs.get(&attempt.run).ok_or(StoreError::NotFound)?;
    let enrollment = super::super::enrollment_record(state, run.enrollment)?;
    let assignment = super::super::assignment_record(state, enrollment.assignment)?;
    let disclosure = super::super::feedback::current_disclosure_input(
        state,
        &assignment,
        attempt.id,
        completed.attempt.timer.submitted_at,
    )?;
    Ok(SubmissionReceiptRead::Completed(Box::new(
        completed.clone().into_submission_record(disclosure),
    )))
}

#[cfg(test)]
mod tests {
    use question_model::{
        ActivityTimestamp, AssignmentId, AttemptProvenance, AttemptStatus, AttemptTimerRecord,
        CourseId, ImplementationVersion, ProblemId, QuestionAttemptId, RunId, StudentResponse,
        UserId, VersionId,
    };
    use uuid::Uuid;

    use super::*;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn attempt() -> QuestionAttempt {
        QuestionAttempt {
            id: QuestionAttemptId::from_uuid(id(1)),
            run: RunId::from_uuid(id(2)),
            problem: ProblemId::from_uuid(id(3)),
            question_version: VersionId::from_uuid(id(4)),
            assignment_position: 0,
            seed: 5,
            parameter_hash: "parameter-hash".to_string(),
            response: None,
            status: AttemptStatus::InProgress,
            result: None,
            timer: AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(1),
                deadline: None,
                submitted_at: None,
            },
            provenance: AttemptProvenance {
                adapter: ImplementationVersion {
                    id: "native".to_string(),
                    version: "1".to_string(),
                },
                renderer: None,
                generator: None,
                source_artifact: None,
                asset_objects: Vec::new(),
                grading: ImplementationVersion {
                    id: "flat".to_string(),
                    version: "1".to_string(),
                },
                rendered_question_sha256: "rendered-hash".to_string(),
            },
            issued_capability: question_model::IssuedAttemptCapabilityV1::FlatPresentation,
        }
    }

    #[test]
    fn checksummed_capability_refuses_a_downgraded_private_contract_column() {
        let attempt = attempt();
        let mut state = State::default();
        state
            .attempt_presentation_capabilities
            .insert(attempt.id, crate::PresentationCapability::EnvelopeV1);
        state
            .attempt_flat_grading_capabilities
            .insert(attempt.id, crate::FlatGradingCapability::NotApplicable);
        state
            .attempt_webwork_grading_capabilities
            .insert(attempt.id, crate::WebworkGradingCapability::NotApplicable);

        assert!(matches!(
            load_issued_presentation(&state, &attempt),
            Err(StoreError::Unavailable(_))
        ));
    }

    #[test]
    fn accepted_pending_receipt_read_is_typed_and_redacted() {
        let attempt = attempt();
        let key = crate::SubmissionIdempotencyKey::parse("pending-replay-key")
            .expect("bounded idempotency key");
        let response = StudentResponse::Numeric { value: 88.0 };
        let mut state = State::default();
        state.submissions.insert(
            attempt.id,
            super::super::StoredSubmission {
                key: key.clone(),
                state: super::super::StoredSubmissionState::AcceptedPending(
                    crate::AcceptedSubmission {
                        course: CourseId::from_uuid(id(21)),
                        assignment: AssignmentId::from_uuid(id(22)),
                        attempt: attempt.id,
                        submission: crate::AcceptedSubmissionId::from_uuid(attempt.id.as_uuid()),
                        actor: UserId::from_uuid(id(23)),
                        idempotency_key: key,
                        request_sha256: objects::Sha256Digest::compute(
                            &serde_json::to_vec(&response).expect("serializable response"),
                        ),
                        accepted_at: ActivityTimestamp::from_unix_millis(24),
                    },
                ),
            },
        );

        let receipt = load_submission_record(&state, &attempt)
            .expect("accepted input has a typed receipt state");
        assert_eq!(
            receipt,
            SubmissionReceiptRead::AcceptedPending(crate::AcceptedSubmissionPending::new(
                attempt.id,
            ))
        );
        let debug = format!("{receipt:?}");
        assert!(!debug.contains("pending-replay-key") && !debug.contains("88"));
    }
}
