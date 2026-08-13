//! Immutable issue-contract readers shared by submission, receipt, and GET.

use question_model::{QuestionAttempt, TenantId};

use crate::{ReceiptPresentationSnapshot, StoreError, SubmissionRecord};

use super::super::State;

/// The immutable issue record, rather than current catalog state, decides
/// whether a first submission must carry an answer-free native envelope.
pub(crate) fn load_issued_presentation(
    state: &State,
    tenant: TenantId,
    attempt: &QuestionAttempt,
) -> Result<Option<ReceiptPresentationSnapshot>, StoreError> {
    let capability = state
        .attempt_presentation_capabilities
        .get(&(tenant, attempt.id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt presentation capability is missing".to_string())
        })?;
    let flat_capability = state
        .attempt_flat_grading_capabilities
        .get(&(tenant, attempt.id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt flat grading capability is missing".to_string())
        })?;
    let webwork_capability = state
        .attempt_webwork_grading_capabilities
        .get(&(tenant, attempt.id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt WeBWorK grading capability is missing".to_string())
        })?;
    crate::validate_attempt_issuance_capability(
        attempt,
        capability,
        flat_capability,
        webwork_capability,
    )?;
    let binding = state
        .attempt_presentations
        .get(&(tenant, attempt.id))
        .copied();
    let snapshot = state
        .attempt_presentation_snapshots
        .get(&(tenant, attempt.id));
    let grading_envelope = state.attempt_grading_envelopes.get(&(tenant, attempt.id));
    let snapshot = crate::validate_issued_presentation(
        capability,
        attempt,
        binding,
        snapshot,
        grading_envelope,
    )?;
    load_issued_flat_grading(state, tenant, attempt)?;
    load_issued_webwork_grading(state, tenant, attempt)?;
    Ok(snapshot)
}

/// Reads the explicit family obligation and validates the retained private
/// flat authority. The nullable payload never decides whether it was required.
pub(crate) fn load_issued_flat_grading(
    state: &State,
    tenant: TenantId,
    attempt: &QuestionAttempt,
) -> Result<Option<crate::IssuedFlatGradingContract>, StoreError> {
    let capability = state
        .attempt_flat_grading_capabilities
        .get(&(tenant, attempt.id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt flat grading capability is missing".to_string())
        })?;
    let contract = state.attempt_flat_grading.get(&(tenant, attempt.id));
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
    tenant: TenantId,
    attempt: &QuestionAttempt,
) -> Result<Option<crate::IssuedWebworkGradingContract>, StoreError> {
    let capability = state
        .attempt_webwork_grading_capabilities
        .get(&(tenant, attempt.id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt WeBWorK grading capability is missing".to_string())
        })?;
    let contract = state.attempt_webwork_grading.get(&(tenant, attempt.id));
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
    tenant: TenantId,
    attempt: &QuestionAttempt,
) -> Result<Option<SubmissionRecord>, StoreError> {
    let Some(stored) = state.submissions.get(&(tenant, attempt.id)) else {
        return Ok(None);
    };
    let issued_presentation = load_issued_presentation(state, tenant, attempt)?;
    if stored.record.presentation != issued_presentation {
        return Err(StoreError::Unavailable(
            "submission receipt presentation does not match its issued snapshot".to_string(),
        ));
    }
    Ok(Some(stored.record.clone()))
}

#[cfg(test)]
mod tests {
    use question_model::{
        ActivityTimestamp, AttemptProvenance, AttemptStatus, AttemptTimerRecord,
        ImplementationVersion, ProblemId, QuestionAttemptId, RunId, TenantId, VersionId,
    };
    use uuid::Uuid;

    use super::*;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn attempt(tenant: TenantId) -> QuestionAttempt {
        QuestionAttempt {
            id: QuestionAttemptId::from_uuid(id(1)),
            tenant,
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
        let tenant = TenantId::from_uuid(id(10));
        let attempt = attempt(tenant);
        let mut state = State::default();
        state.attempt_presentation_capabilities.insert(
            (tenant, attempt.id),
            crate::PresentationCapability::EnvelopeV1,
        );
        state.attempt_flat_grading_capabilities.insert(
            (tenant, attempt.id),
            crate::FlatGradingCapability::NotApplicable,
        );
        state.attempt_webwork_grading_capabilities.insert(
            (tenant, attempt.id),
            crate::WebworkGradingCapability::NotApplicable,
        );

        assert!(matches!(
            load_issued_presentation(&state, tenant, &attempt),
            Err(StoreError::Unavailable(_))
        ));
    }
}
