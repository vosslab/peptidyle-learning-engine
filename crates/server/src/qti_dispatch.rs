//! QTI-specific composition helpers for the installed run backend registry.
//!
//! Keeping this closed dispatch seam separate makes the QTI issue-only
//! authority visible without coupling native, WeBWorK, or external-tool
//! provider composition to its private grader contract.

use std::sync::Arc;

use learning_data_access::TenantContext;
use question_model::{
    ProblemVersionRef, QuestionAttempt, QuestionDefinition, QuestionEnvelope, StudentResponse,
};

use crate::composite_backend::CompositeBackend;
use crate::run::{
    IssuedAttemptMetadata, RunBackend, RunBackendError, RunSubmission, SubmissionDisposition,
};

/// A separately configured QTI execution boundary.
///
/// QTI has no browser launch protocol. Its implementation receives immutable
/// public source/object handles plus a private grader handle only at trusted
/// production composition time.
pub trait ConfiguredQti: RunBackend {}

impl<T> ConfiguredQti for T where T: RunBackend + ?Sized {}

/// Optional QTI composition slot kept beside the QTI dispatch boundary.
pub struct QtiBackendSlot(Option<Arc<dyn ConfiguredQti>>);

impl QtiBackendSlot {
    pub const fn empty() -> Self {
        Self(None)
    }

    pub fn with(mut self, backend: Arc<dyn ConfiguredQti>) -> Self {
        self.0 = Some(backend);
        self
    }

    pub fn is_configured(&self) -> bool {
        self.0.is_some()
    }

    fn configured(&self) -> &Option<Arc<dyn ConfiguredQti>> {
        &self.0
    }
}

impl<S, O, R> CompositeBackend<S, O, R> {
    pub fn with_qti(mut self, backend: Arc<dyn ConfiguredQti>) -> Self {
        self.qti = self.qti.with(backend);
        self
    }

    pub fn has_qti(&self) -> bool {
        self.qti.is_configured()
    }
}

/// Selects the QTI backend from the immutable issued question family.
pub fn configured_qti_for<'a>(
    configured: &'a QtiBackendSlot,
    question: &QuestionDefinition,
) -> Result<&'a Arc<dyn ConfiguredQti>, RunBackendError> {
    if !matches!(question.source, question_model::QuestionSource::Qti { .. }) {
        return Err(RunBackendError::Unsupported(
            "published question is not QTI".into(),
        ));
    }
    configured
        .configured()
        .as_ref()
        .ok_or_else(|| RunBackendError::Unsupported("QTI is not configured".into()))
}

/// Issues QTI only through the configured private authority.
pub async fn issue(
    configured: &QtiBackendSlot,
    context: TenantContext,
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
    seed: u64,
) -> Result<IssuedAttemptMetadata, RunBackendError> {
    configured_qti_for(configured, question)?
        .issue(context, reference, question, seed)
        .await
}

/// Reproduces only a configured QTI backend's non-replay path.
pub async fn reproduce(
    configured: &QtiBackendSlot,
    context: TenantContext,
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
    attempt: &QuestionAttempt,
) -> Result<QuestionEnvelope, RunBackendError> {
    configured_qti_for(configured, question)?
        .reproduce(context, reference, question, attempt)
        .await
}

/// Invokes the legacy backend grade seam only for callers that do not own a
/// prepared submission. QTI's concrete backend refuses this seam.
pub async fn grade(
    configured: &QtiBackendSlot,
    context: TenantContext,
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
    attempt: &QuestionAttempt,
    response: &StudentResponse,
) -> Result<grading::GradeOutcome, RunBackendError> {
    configured_qti_for(configured, question)?
        .grade(context, reference, question, attempt, response)
        .await
}

/// Submits a QTI response through the prepared issued-contract path.
pub async fn submit(
    configured: &QtiBackendSlot,
    submission: RunSubmission<'_>,
) -> Result<SubmissionDisposition, RunBackendError> {
    configured_qti_for(configured, submission.question())?
        .submit(submission)
        .await
}
