//! Generic iMathAS attempt projection behind the `RunBackend` facade.
//!
//! This owner deliberately refuses grading and submission.  Contracted launch
//! and broker recovery live in sibling owners that require launch proof.

use adapter_imathas::ImathasProvider;
use async_trait::async_trait;
use learning_data_access::{CatalogSourceStore, ExternalToolBrokerStore, TenantContext};
use objects::ObjectStore;
use question_model::{
    ProblemVersionRef, QuestionAttempt, QuestionDefinition, QuestionEnvelope, StudentResponse,
};

use super::{
    ImathasBackend, IssuedAttemptMetadata, RunBackend, RunBackendError, RunSubmission,
    SubmissionDisposition, map_adapter_error,
};

#[async_trait]
impl<S, O, P> RunBackend for ImathasBackend<S, O, P>
where
    S: CatalogSourceStore + ExternalToolBrokerStore + Send + Sync + 'static,
    O: ObjectStore + Send + Sync + 'static,
    P: ImathasProvider + 'static,
{
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        let (source, artifact) = self.resolve_source(context, reference, question).await?;
        let issued = self
            .adapter
            .issue(
                question,
                question_model::generation::Seed::new(seed),
                &source,
                artifact.object.created_at,
            )
            .await
            .map_err(map_adapter_error)?;
        Ok(IssuedAttemptMetadata {
            envelope: issued.envelope,
            parameter_hash: issued.parameter_hash,
            provenance: issued.provenance,
            webwork_replay: None,
        })
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        Ok(self
            .reproduce_issued_attempt(context, reference, question, attempt)
            .await?
            .envelope)
    }

    async fn prepare_external_tool_launch(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<(), RunBackendError> {
        let _ = self
            .reproduce_issued_attempt(context, reference, question, attempt)
            .await?;
        Ok(())
    }

    async fn grade(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
        _question: &QuestionDefinition,
        _attempt: &QuestionAttempt,
        _response: &StudentResponse,
    ) -> Result<grading::GradeOutcome, RunBackendError> {
        Err(RunBackendError::Unsupported(
            "iMathAS grading requires the durable external-tool broker".into(),
        ))
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        let _ = submission;
        Err(RunBackendError::Unsupported(
            "iMathAS submission requires an authenticated launch session".into(),
        ))
    }
}
