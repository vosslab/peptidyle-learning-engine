//! Server-private immutable learner-response authority for Memory conformance.

use question_model::{QuestionAttemptId, StudentResponse, TenantId};

use super::{State, StoreError};

/// Private response authority paired with one immutable submission parent.
///
/// The typed value makes deterministic Memory replay a useful conformance
/// model, while the canonical text and digest preserve the same response
/// identity as PostgreSQL's private accepted-response relation. ASVS 1.5.3,
/// 2.2.1, and 2.3.1: one closed response has one validated replay identity.
#[derive(Clone)]
pub(super) struct StoredPrivateSubmissionResponse {
    pub(super) canonical_text: String,
    pub(super) sha256: objects::Sha256Digest,
    pub(super) response: StudentResponse,
}

impl StoredPrivateSubmissionResponse {
    pub(super) fn from_response(response: StudentResponse) -> Result<Self, StoreError> {
        let canonical_text = crate::canonical_student_response_json(&response)?;
        let sha256 = objects::Sha256Digest::compute(canonical_text.as_bytes());
        Ok(Self {
            canonical_text,
            sha256,
            response,
        })
    }

    fn matches(
        &self,
        response: &StudentResponse,
        canonical_text: &str,
        sha256: objects::Sha256Digest,
    ) -> bool {
        self.response == *response && self.canonical_text == canonical_text && self.sha256 == sha256
    }
}

impl std::fmt::Debug for StoredPrivateSubmissionResponse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StoredPrivateSubmissionResponse")
            .field("canonical_text", &"[SERVER-ONLY]")
            .field("sha256", &"[SERVER-ONLY]")
            .field("response", &"[SERVER-ONLY]")
            .finish()
    }
}

/// Compares a replay request against private response authority without
/// exposing that authority to receipt or attempt callers.
pub(super) fn stored_submission_matches_response(
    state: &State,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    response: &StudentResponse,
) -> Result<bool, StoreError> {
    let canonical_text = crate::canonical_student_response_json(response)?;
    let sha256 = objects::Sha256Digest::compute(canonical_text.as_bytes());
    stored_submission_matches_canonical(state, tenant, attempt, response, &canonical_text, sha256)
}

/// Validates a response against already-canonicalized replay identity.
///
/// Automated acceptance calls this after serializing its closed response once;
/// generic completed-submission paths use `stored_submission_matches_response`.
pub(super) fn stored_submission_matches_canonical(
    state: &State,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    response: &StudentResponse,
    canonical_text: &str,
    sha256: objects::Sha256Digest,
) -> Result<bool, StoreError> {
    let private = state
        .private_submission_responses
        .get(&(tenant, attempt))
        .ok_or_else(|| {
            StoreError::Unavailable("submission response authority is missing".to_string())
        })?;
    Ok(private.matches(response, canonical_text, sha256))
}
