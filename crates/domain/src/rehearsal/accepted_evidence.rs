//! Verified restoration of immutable accepted rehearsal evidence.

use super::*;

impl RehearsalValidatedSubmissionEvidence {
    /// Restores accepted evidence only from a root authenticated against this
    /// exact frozen attempt. Rendered input retains the original browser
    /// response; durable translation remains a sealed transient.
    pub fn restore_with_verified_root(
        root: &RehearsalClaimRoot,
        frozen: &RehearsalFrozenItemEvidence,
        submitted_response: question_model::StudentResponse,
        result: RehearsalPrivateGradingResult,
        accepted_at: question_model::ActivityTimestamp,
    ) -> Result<Self, RehearsalEvidenceValidationError> {
        if !root.verified_attempt_matches(frozen)
            || submitted_response != *root.submission_input().original_response()
        {
            return Err(RehearsalEvidenceValidationError::ResponseDefinitionMismatch);
        }
        if root.submission_input().durable_request().is_some() {
            let restored = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
                frozen,
                frozen.attempt,
                submitted_response.clone(),
            )?;
            root.submission_input()
                .validate_for_completion(&restored, frozen)?;
        }
        validate_grading_result(&result)?;
        if private_submission_bytes(&submitted_response, &result)
            > question_model::MAX_REHEARSAL_ACCEPTED_SUBMISSION_BYTES
        {
            return Err(RehearsalEvidenceValidationError::AcceptedSubmissionTooLarge);
        }
        if private_submission_entries(&submitted_response, &result)
            > question_model::MAX_REHEARSAL_ACCEPTED_SUBMISSION_ENTRIES
        {
            return Err(RehearsalEvidenceValidationError::TooManyAcceptedSubmissionEntries);
        }
        Ok(Self {
            claim_binding: root.binding,
            attempt: frozen.attempt,
            submitted_response,
            result,
            accepted_at,
        })
    }
}
