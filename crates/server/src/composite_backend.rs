use async_trait::async_trait;
use learning_data_access::{
    AssetStore, CatalogSourceStore, ExternalToolBrokerStore, Store, TenantContext,
};
use question_model::{
    ProblemVersionRef, QuestionAttempt, QuestionDefinition, QuestionEnvelope, StudentResponse,
};

use crate::imathas_backend::ExternalToolSubmissionBackend;
use crate::native_backend::NativeBackend;
use crate::qti_dispatch::QtiBackendSlot;
use crate::rehearsal::{RehearsalGradeBackend, RehearsalIssueBackend};
use crate::run::ExternalToolLaunchBackend;
use crate::run::{
    GradeReceipt, IssuedAttemptMetadata, RunBackend, RunBackendError, RunSubmission,
    SubmissionDisposition,
};
use crate::webwork_backend::WebworkBackend;
mod dispatch;

/// One trusted server backend that delegates by persisted source kind.
pub trait ConfiguredImathas:
    RunBackend + ExternalToolLaunchBackend + ExternalToolSubmissionBackend
{
    fn serves_provider(&self, provider: &str) -> bool;
}

pub struct CompositeBackend<S, O, R> {
    native: NativeBackend<S>,
    webwork: Option<WebworkBackend<S, O, R>>,
    imathas: Option<std::sync::Arc<dyn ConfiguredImathas>>,
    pub(crate) qti: QtiBackendSlot,
}

impl<S, O, R> CompositeBackend<S, O, R> {
    /// Combines already-configured backend boundaries without reading runtime
    /// configuration or changing generic route behavior.
    pub fn new(native: NativeBackend<S>, webwork: WebworkBackend<S, O, R>) -> Self {
        Self {
            native,
            webwork: Some(webwork),
            imathas: None,
            qti: QtiBackendSlot::empty(),
        }
    }
    /// Builds the normal native-only registry without manufacturing renderer
    /// credentials or advertising an unavailable WebWork capability.
    pub fn native_only(native: NativeBackend<S>) -> Self {
        Self {
            native,
            webwork: None,
            imathas: None,
            qti: QtiBackendSlot::empty(),
        }
    }
    pub fn with_imathas(mut self, imathas: std::sync::Arc<dyn ConfiguredImathas>) -> Self {
        self.imathas = Some(imathas);
        self
    }
    pub fn has_imathas(&self) -> bool {
        self.imathas.is_some()
    }
    fn imathas_for(
        &self,
        question: &QuestionDefinition,
    ) -> Result<&std::sync::Arc<dyn ConfiguredImathas>, RunBackendError> {
        let question_model::QuestionSource::Imathas { provider, .. } = &question.source else {
            return Err(RunBackendError::Unsupported(
                "published question is not iMathAS".into(),
            ));
        };
        let backend = self
            .imathas
            .as_ref()
            .ok_or_else(|| RunBackendError::Unsupported("iMathAS is not configured".into()))?;
        if !backend.serves_provider(provider) {
            return Err(RunBackendError::Unsupported(
                "iMathAS provider is not configured".into(),
            ));
        }
        Ok(backend)
    }

    fn webwork(&self) -> Result<&WebworkBackend<S, O, R>, RunBackendError> {
        self.webwork
            .as_ref()
            .ok_or_else(|| RunBackendError::Unsupported("WebWork is not configured".into()))
    }
}

#[async_trait]
impl<S, O, R> RehearsalIssueBackend for CompositeBackend<S, O, R>
where
    S: Send + Sync + 'static,
    O: Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    async fn issue_frozen_rehearsal(
        &self,
        work: &learning_data_access::SealedRehearsalDeliveryIssueWork,
    ) -> Result<learning_data_access::RehearsalIssuedExecutionArtifactV1, RunBackendError> {
        self.native.issue_frozen_rehearsal(work).await
    }
}

#[async_trait]
impl<S, O, R> RehearsalGradeBackend for CompositeBackend<S, O, R>
where
    S: Send + Sync + 'static,
    O: Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    async fn grade_frozen_rehearsal(
        &self,
        work: learning_data_access::SealedRehearsalGradingParts,
    ) -> Result<GradeReceipt, RunBackendError> {
        self.native.grade_frozen_rehearsal(work).await
    }
}

#[async_trait]
impl<S, O, R> RunBackend for CompositeBackend<S, O, R>
where
    S: AssetStore + CatalogSourceStore + ExternalToolBrokerStore + Store + Send + Sync + 'static,
    O: objects::ObjectStore + Send + Sync + 'static,
    R: adapter_webwork::renderer_contract::WebworkRenderer + Send + Sync + 'static,
{
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        match question.source {
            question_model::QuestionSource::Native { .. } => {
                self.native.issue(context, reference, question, seed).await
            }
            question_model::QuestionSource::Webwork { .. } => {
                let issued = self
                    .webwork()?
                    .issue(context, reference, question, seed)
                    .await?;
                Ok(IssuedAttemptMetadata {
                    envelope: issued.envelope,
                    parameter_hash: issued.parameter_hash,
                    provenance: issued.provenance,
                    webwork_replay: issued.replay,
                    flat_grading: None,
                    flat_grading_capability:
                        learning_data_access::FlatGradingCapability::NotApplicable,
                    webwork_grading: Some(
                        learning_data_access::IssuedWebworkGradingContract::new(question.clone())
                            .map_err(|error| RunBackendError::Invalid(error.to_string()))?,
                    ),
                    webwork_grading_capability:
                        learning_data_access::WebworkGradingCapability::Required,
                    qti_grading: None,
                    qti_grading_capability:
                        learning_data_access::QtiGradingCapability::NotApplicable,
                })
            }
            question_model::QuestionSource::Imathas { .. } => {
                self.imathas_for(question)?
                    .issue(context, reference, question, seed)
                    .await
            }
            question_model::QuestionSource::Qti { .. } => {
                crate::qti_dispatch::issue(&self.qti, context, reference, question, seed).await
            }
            _ => Err(RunBackendError::Unsupported(
                "published question backend is not registered".to_string(),
            )),
        }
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        match question.source {
            question_model::QuestionSource::Native { .. } => {
                self.native
                    .reproduce(context, reference, question, attempt)
                    .await
            }
            question_model::QuestionSource::Webwork { .. } => {
                let issued = self
                    .webwork()?
                    .reproduce(context, reference, question, attempt)
                    .await?;
                Ok(issued.envelope)
            }
            question_model::QuestionSource::Imathas { .. } => {
                self.imathas_for(question)?
                    .reproduce(context, reference, question, attempt)
                    .await
            }
            question_model::QuestionSource::Qti { .. } => {
                crate::qti_dispatch::reproduce(&self.qti, context, reference, question, attempt)
                    .await
            }
            _ => Err(RunBackendError::Unsupported(
                "published question backend is not registered".to_string(),
            )),
        }
    }

    async fn prepare_external_tool_launch(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<(), RunBackendError> {
        match question.source {
            question_model::QuestionSource::Imathas { .. } => {
                self.imathas_for(question)?
                    .prepare_external_tool_launch(context, reference, question, attempt)
                    .await
            }
            _ => Err(RunBackendError::Unsupported(
                "this question backend does not provide an external-tool launch".into(),
            )),
        }
    }

    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<grading::GradeOutcome, RunBackendError> {
        match question.source {
            question_model::QuestionSource::Native { .. } => {
                self.native
                    .grade(context, reference, question, attempt, response)
                    .await
            }
            question_model::QuestionSource::Webwork { .. } => Err(RunBackendError::Unsupported(
                "WeBWorK grading requires an actor-bound submission".into(),
            )),
            question_model::QuestionSource::Imathas { .. } => {
                self.imathas_for(question)?
                    .grade(context, reference, question, attempt, response)
                    .await
            }
            question_model::QuestionSource::Qti { .. } => {
                crate::qti_dispatch::grade(
                    &self.qti, context, reference, question, attempt, response,
                )
                .await
            }
            _ => Err(RunBackendError::Unsupported(
                "published question backend is not registered".to_string(),
            )),
        }
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        match submission.question().source {
            question_model::QuestionSource::Native { .. } => self.native.submit(submission).await,
            question_model::QuestionSource::Webwork { .. } => self
                .webwork()?
                .grade(
                    submission.context,
                    submission.actor,
                    submission.reference,
                    submission.attempt,
                    submission.issued_webwork_grading.ok_or_else(|| {
                        RunBackendError::Unavailable(
                            "WeBWorK issued grading contract is unavailable".to_string(),
                        )
                    })?,
                    submission.issued_presentation_binding.ok_or_else(|| {
                        RunBackendError::Unavailable(
                            "WeBWorK issued presentation binding is unavailable".to_string(),
                        )
                    })?,
                    submission.issued_webwork_replay.ok_or_else(|| {
                        RunBackendError::Unavailable(
                            "WeBWorK issued replay state is unavailable".to_string(),
                        )
                    })?,
                    submission.issued_presentation.ok_or_else(|| {
                        RunBackendError::Unavailable(
                            "WeBWorK issued presentation snapshot is unavailable".to_string(),
                        )
                    })?,
                    submission.issued_grading_envelope.ok_or_else(|| {
                        RunBackendError::Unavailable(
                            "WeBWorK issued grading envelope is unavailable".to_string(),
                        )
                    })?,
                    submission.response,
                )
                .await
                .and_then(|outcome| match outcome {
                    grading::GradeOutcome::Graded(result) => {
                        Ok(SubmissionDisposition::Grade(GradeReceipt::empty(result)))
                    }
                    grading::GradeOutcome::NeedsManualGrading => {
                        Ok(SubmissionDisposition::NeedsManualGrading)
                    }
                    grading::GradeOutcome::Ungraded => Err(RunBackendError::Unsupported(
                        "this run backend does not produce a server grade".to_string(),
                    )),
                }),
            question_model::QuestionSource::Imathas { .. } => {
                self.imathas_for(submission.question())?
                    .submit(submission)
                    .await
            }
            question_model::QuestionSource::Qti { .. } => {
                crate::qti_dispatch::submit(&self.qti, submission).await
            }
            _ => Err(RunBackendError::Unsupported(
                "published question backend is not registered".to_string(),
            )),
        }
    }
}

#[async_trait]
impl<S, O, R> ExternalToolLaunchBackend for CompositeBackend<S, O, R>
where
    S: AssetStore + CatalogSourceStore + ExternalToolBrokerStore + Send + Sync + 'static,
    O: objects::ObjectStore + Send + Sync + 'static,
    R: adapter_webwork::renderer_contract::WebworkRenderer + Send + Sync + 'static,
{
    async fn create_external_tool_launch(
        &self,
        context: TenantContext,
        actor: question_model::UserId,
        learner_work_binding: learning_data_access::LearnerWorkRoutingBinding,
        issued_question_snapshot: &learning_data_access::IssuedQuestionSnapshotV1,
        attempt: &QuestionAttempt,
        aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<learning_data_access::CreatedExternalToolLaunchSession, RunBackendError> {
        self.imathas_for(issued_question_snapshot.question())?
            .create_external_tool_launch(
                context,
                actor,
                learner_work_binding,
                issued_question_snapshot,
                attempt,
                aead,
            )
            .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn proxy_external_tool_activity(
        &self,
        context: TenantContext,
        actor: question_model::UserId,
        learner_work_binding: learning_data_access::LearnerWorkRoutingBinding,
        issued_question_snapshot: &learning_data_access::IssuedQuestionSnapshotV1,
        attempt: &QuestionAttempt,
        session_id: uuid::Uuid,
        token: &learning_data_access::ExternalToolLaunchToken,
        method: adapter_imathas::broker_provider::ProxyMethod,
        body: &[u8],
        aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<adapter_imathas::broker_provider::ProxyResponse, RunBackendError> {
        self.imathas_for(issued_question_snapshot.question())?
            .proxy_external_tool_activity(
                context,
                actor,
                learner_work_binding,
                issued_question_snapshot,
                attempt,
                session_id,
                token,
                method,
                body,
                aead,
            )
            .await
    }
}

#[async_trait]
impl<S, O, R> ExternalToolSubmissionBackend for CompositeBackend<S, O, R>
where
    S: AssetStore + CatalogSourceStore + ExternalToolBrokerStore + Send + Sync + 'static,
    O: objects::ObjectStore + Send + Sync + 'static,
    R: adapter_webwork::renderer_contract::WebworkRenderer + Send + Sync + 'static,
{
    async fn submit_external_tool(
        &self,
        context: TenantContext,
        actor: question_model::UserId,
        learner_work_binding: learning_data_access::LearnerWorkRoutingBinding,
        issued_question_snapshot: &learning_data_access::IssuedQuestionSnapshotV1,
        attempt: &QuestionAttempt,
        idempotency_key: learning_data_access::SubmissionIdempotencyKey,
        launch_proof: learning_data_access::ExternalToolLaunchProof,
        state_aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        self.imathas_for(issued_question_snapshot.question())?
            .submit_external_tool(
                context,
                actor,
                learner_work_binding,
                issued_question_snapshot,
                attempt,
                idempotency_key,
                launch_proof,
                state_aead,
            )
            .await
    }
}

#[cfg(test)]
#[path = "composite_backend/tests.rs"]
mod tests;
