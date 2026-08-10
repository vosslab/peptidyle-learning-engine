//! Trusted server composition for the first-party native adapter.
//!
//! This bridge resolves immutable catalog asset bindings under the authenticated
//! tenant context before handing an attempt to `adapter_native`. It never
//! accepts asset mappings, implementation versions, seeds, or answer material
//! from a browser.

use std::sync::Arc;

use async_trait::async_trait;
use grading::GradeOutcome;
use learning_data_access::{AssetStore, FlatQuestionGradingStore, StoreError, TenantContext};
use question_model::generation::Seed;
use question_model::{
    BackendCapabilities, DraftQuestionSource, ProblemVersionRef, QuestionAttempt,
    QuestionDefinition, QuestionEnvelope, QuestionSource, StudentResponse,
};

use crate::catalog::{BackendRegistry, BackendRegistryError};
use crate::run::{
    GradeReceipt, IssuedAttemptMetadata, RunBackend, RunBackendError, RunSubmission,
    SubmissionDisposition,
};

/// Composition bridge from server-owned persistence to native question logic.
pub struct NativeBackend<S> {
    adapter: Arc<adapter_native::NativeAdapter>,
    assets: Arc<S>,
    // This deliberately remains a separate, optional capability. The normal
    // application store can resolve public catalog assets, but cannot recover
    // flat-question answers or teaching feedback.
    flat_grader: Option<Arc<dyn FlatQuestionGradingStore>>,
}

impl<S> NativeBackend<S> {
    /// Creates a bridge with one immutable native adapter registry and asset store.
    pub fn new(adapter: Arc<adapter_native::NativeAdapter>, assets: Arc<S>) -> Self {
        Self {
            adapter,
            assets,
            flat_grader: None,
        }
    }

    /// Creates a native bridge with the isolated capability required to grade
    /// published flat questions. Callers that use [`Self::new`] instead retain
    /// normal native behavior but flat submissions fail closed.
    pub fn with_flat_grader(
        adapter: Arc<adapter_native::NativeAdapter>,
        assets: Arc<S>,
        flat_grader: Arc<dyn FlatQuestionGradingStore>,
    ) -> Self {
        Self {
            adapter,
            assets,
            flat_grader: Some(flat_grader),
        }
    }

    /// Returns the shared adapter registry used by this server composition.
    pub fn adapter(&self) -> &Arc<adapter_native::NativeAdapter> {
        &self.adapter
    }
}

impl<S> BackendRegistry for NativeBackend<S>
where
    S: Send + Sync,
{
    fn capabilities(
        &self,
        source: &DraftQuestionSource,
    ) -> Result<BackendCapabilities, BackendRegistryError> {
        let source = match source {
            DraftQuestionSource::Native { family } => QuestionSource::Native {
                family: family.clone(),
            },
            _ => return Err(BackendRegistryError::Unsupported),
        };
        self.adapter
            .capabilities(&source)
            .map_err(map_capability_error)
    }
}

#[async_trait]
impl<S> RunBackend for NativeBackend<S>
where
    S: AssetStore + Send + Sync + 'static,
{
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        validate_definition_reference(reference, question)?;
        let bindings = self.asset_bindings(context, reference).await?;
        let issued = self
            .adapter
            .issue(question, Seed::new(seed), &bindings)
            .map_err(map_native_error)?;
        Ok(IssuedAttemptMetadata {
            envelope: issued.envelope,
            parameter_hash: issued.parameter_hash,
            provenance: issued.provenance,
        })
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        validate_attempt_reference(reference, question, attempt)?;
        let bindings = self.asset_bindings(context, reference).await?;
        self.adapter
            .reproduce(
                question,
                Seed::new(attempt.seed),
                &attempt.parameter_hash,
                &attempt.provenance,
                &bindings,
            )
            .map_err(map_native_error)
    }

    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError> {
        validate_attempt_reference(reference, question, attempt)?;
        let bindings = self.asset_bindings(context, reference).await?;
        if is_flat_question(question) {
            self.validate_flat_attempt(question, attempt, &bindings)?;
            return self
                .flat_evaluate(context, reference, question, response)
                .await
                .map(|evaluation| evaluation.outcome);
        }
        self.adapter
            .grade(
                question,
                Seed::new(attempt.seed),
                &attempt.parameter_hash,
                &attempt.provenance,
                &bindings,
                response,
            )
            .map_err(map_native_error)
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        validate_attempt_reference(
            submission.reference,
            submission.question,
            submission.attempt,
        )?;
        let bindings = self
            .asset_bindings(submission.context, submission.reference)
            .await?;
        if is_flat_question(submission.question) {
            self.validate_flat_attempt(submission.question, submission.attempt, &bindings)?;
            let evaluation = self
                .flat_evaluate(
                    submission.context,
                    submission.reference,
                    submission.question,
                    submission.response,
                )
                .await?;
            return match evaluation.outcome {
                grading::GradeOutcome::Graded(result) => {
                    Ok(SubmissionDisposition::Grade(GradeReceipt {
                        result,
                        feedback: evaluation.feedback,
                    }))
                }
                grading::GradeOutcome::NeedsManualGrading => {
                    Ok(SubmissionDisposition::NeedsManualGrading)
                }
                grading::GradeOutcome::Ungraded => Err(RunBackendError::Unsupported(
                    "flat question did not produce a server grade".to_string(),
                )),
            };
        }
        let (outcome, feedback) = self
            .adapter
            .grade_with_feedback(
                submission.question,
                Seed::new(submission.attempt.seed),
                &submission.attempt.parameter_hash,
                &submission.attempt.provenance,
                &bindings,
                submission.response,
            )
            .map_err(map_native_error)?;
        match outcome {
            grading::GradeOutcome::Graded(result) => {
                Ok(SubmissionDisposition::Grade(GradeReceipt {
                    result,
                    feedback,
                }))
            }
            grading::GradeOutcome::NeedsManualGrading => {
                Ok(SubmissionDisposition::NeedsManualGrading)
            }
            grading::GradeOutcome::Ungraded => Err(RunBackendError::Unsupported(
                "native question did not produce a server grade".to_string(),
            )),
        }
    }
}

impl<S> NativeBackend<S>
where
    S: AssetStore + Send + Sync + 'static,
{
    async fn asset_bindings(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Vec<adapter_native::AssetObjectBinding>, RunBackendError> {
        self.assets
            .catalog_asset_bindings(context, reference)
            .await
            .map_err(map_store_error)
            .map(|bindings| {
                bindings
                    .into_iter()
                    .map(|binding| adapter_native::AssetObjectBinding {
                        asset: binding.asset,
                        object: binding.object,
                    })
                    .collect()
            })
    }

    /// Reuses the public native replay path to prove the stored attempt was
    /// issued for this immutable question before the private capability is
    /// consulted. Flat questions are static, but their adapter/version and
    /// rendered-question provenance are still security-relevant.
    fn validate_flat_attempt(
        &self,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        bindings: &[adapter_native::AssetObjectBinding],
    ) -> Result<(), RunBackendError> {
        self.adapter
            .reproduce(
                question,
                Seed::new(attempt.seed),
                &attempt.parameter_hash,
                &attempt.provenance,
                bindings,
            )
            .map(|_| ())
            .map_err(map_native_error)
    }

    async fn flat_evaluate(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        response: &StudentResponse,
    ) -> Result<adapter_native::flat_question::FlatQuestionEvaluation, RunBackendError> {
        let grader = self.flat_grader.as_ref().ok_or_else(|| {
            RunBackendError::Unavailable("flat-question private grading is unavailable".to_string())
        })?;
        let payload = grader
            .flat_question_published_grading(context, reference)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| {
                RunBackendError::Unavailable(
                    "flat-question private grading is unavailable".to_string(),
                )
            })?;
        let private = payload.decode_private().map_err(|_| {
            RunBackendError::Invalid("flat-question private grading is invalid".to_string())
        })?;
        private.evaluate(question, response).map_err(|_| {
            RunBackendError::Invalid("flat-question private grading is invalid".to_string())
        })
    }
}

fn is_flat_question(question: &QuestionDefinition) -> bool {
    matches!(
        &question.source,
        QuestionSource::Native { family }
            if adapter_native::flat_question::is_flat_question_family(family)
    )
}

fn validate_definition_reference(
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
) -> Result<(), RunBackendError> {
    if question.version != reference.version || question.problem != reference.problem {
        return Err(RunBackendError::Invalid(
            "published question does not match immutable problem version reference".to_string(),
        ));
    }
    Ok(())
}

fn validate_attempt_reference(
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
    attempt: &QuestionAttempt,
) -> Result<(), RunBackendError> {
    validate_definition_reference(reference, question)?;
    if attempt.problem != reference.problem || attempt.question_version != reference.version {
        return Err(RunBackendError::Invalid(
            "attempt does not match immutable problem version reference".to_string(),
        ));
    }
    Ok(())
}

fn map_capability_error(error: adapter_native::NativeAdapterError) -> BackendRegistryError {
    match error {
        adapter_native::NativeAdapterError::UnsupportedSource
        | adapter_native::NativeAdapterError::UnknownFamily(_) => BackendRegistryError::Unsupported,
        other => BackendRegistryError::Unavailable(other.to_string()),
    }
}

fn map_native_error(error: adapter_native::NativeAdapterError) -> RunBackendError {
    match error {
        adapter_native::NativeAdapterError::UnsupportedSource
        | adapter_native::NativeAdapterError::UnknownFamily(_)
        | adapter_native::NativeAdapterError::UnknownGenerator { .. } => {
            RunBackendError::Unsupported(error.to_string())
        }
        other => RunBackendError::Invalid(other.to_string()),
    }
}

fn map_store_error(error: StoreError) -> RunBackendError {
    match error {
        StoreError::Unavailable(message) => RunBackendError::Unavailable(message),
        other => RunBackendError::Invalid(other.to_string()),
    }
}

#[cfg(test)]
mod tests;
