//! Server-only dispatch across installed question backends.
//!
//! Dispatch derives solely from the immutable draft/published source model;
//! no request or browser value can select a backend.

use async_trait::async_trait;
use learning_data_access::{
    AssetStore, CatalogSourceStore, ExternalToolBrokerStore, Store, TenantContext,
};
use question_model::{
    BackendCapabilities, DraftQuestionSource, ProblemVersionRef, QuestionAttempt,
    QuestionDefinition, QuestionEnvelope, StudentResponse,
};

use crate::catalog::{BackendRegistry, BackendRegistryError};
use crate::imathas_backend::ExternalToolSubmissionBackend;
use crate::native_backend::NativeBackend;
use crate::run::ExternalToolLaunchBackend;
use crate::run::{
    GradeReceipt, IssuedAttemptMetadata, RunBackend, RunBackendError, RunSubmission,
    SubmissionDisposition,
};
use crate::webwork_backend::WebworkBackend;

/// One trusted server backend that delegates by persisted source kind.
pub trait ConfiguredImathas:
    RunBackend + ExternalToolLaunchBackend + ExternalToolSubmissionBackend
{
    fn serves_provider(&self, provider: &str) -> bool;
}

/// A separately configured QTI execution boundary.
///
/// Unlike external-tool providers, QTI has no browser launch protocol.  Its
/// implementation receives immutable public source/object handles plus a
/// private grader handle only at production composition time.
pub trait ConfiguredQti: RunBackend {}

impl<T> ConfiguredQti for T where T: RunBackend + ?Sized {}

pub struct CompositeBackend<S, O, R> {
    native: NativeBackend<S>,
    webwork: Option<WebworkBackend<S, O, R>>,
    imathas: Option<std::sync::Arc<dyn ConfiguredImathas>>,
    qti: Option<std::sync::Arc<dyn ConfiguredQti>>,
}

impl<S, O, R> CompositeBackend<S, O, R> {
    /// Combines already-configured backend boundaries without reading runtime
    /// configuration or changing generic route behavior.
    pub fn new(native: NativeBackend<S>, webwork: WebworkBackend<S, O, R>) -> Self {
        Self {
            native,
            webwork: Some(webwork),
            imathas: None,
            qti: None,
        }
    }
    /// Builds the normal native-only registry without manufacturing renderer
    /// credentials or advertising an unavailable WebWork capability.
    pub fn native_only(native: NativeBackend<S>) -> Self {
        Self {
            native,
            webwork: None,
            imathas: None,
            qti: None,
        }
    }
    pub fn with_imathas(mut self, imathas: std::sync::Arc<dyn ConfiguredImathas>) -> Self {
        self.imathas = Some(imathas);
        self
    }
    pub fn has_imathas(&self) -> bool {
        self.imathas.is_some()
    }
    pub fn with_qti(mut self, qti: std::sync::Arc<dyn ConfiguredQti>) -> Self {
        self.qti = Some(qti);
        self
    }
    pub fn has_qti(&self) -> bool {
        self.qti.is_some()
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

    fn qti_for(
        &self,
        question: &QuestionDefinition,
    ) -> Result<&std::sync::Arc<dyn ConfiguredQti>, RunBackendError> {
        if !matches!(question.source, question_model::QuestionSource::Qti { .. }) {
            return Err(RunBackendError::Unsupported(
                "published question is not QTI".into(),
            ));
        }
        self.qti
            .as_ref()
            .ok_or_else(|| RunBackendError::Unsupported("QTI is not configured".into()))
    }

    fn webwork(&self) -> Result<&WebworkBackend<S, O, R>, RunBackendError> {
        self.webwork
            .as_ref()
            .ok_or_else(|| RunBackendError::Unsupported("WebWork is not configured".into()))
    }
}

impl<S, O, R> BackendRegistry for CompositeBackend<S, O, R>
where
    S: Send + Sync,
    O: Send + Sync,
    R: Send + Sync,
{
    fn capabilities(
        &self,
        source: &DraftQuestionSource,
    ) -> Result<BackendCapabilities, BackendRegistryError> {
        match source {
            DraftQuestionSource::Native { .. } => self.native.capabilities(source),
            DraftQuestionSource::Webwork { .. } if self.webwork.is_some() => {
                Ok(BackendCapabilities::from_iter([
                    question_model::Capability::AlgorithmicGeneration,
                    question_model::Capability::ServerGrading,
                ]))
            }
            DraftQuestionSource::Imathas { provider, .. }
                if self
                    .imathas
                    .as_ref()
                    .is_some_and(|backend| backend.serves_provider(provider)) =>
            {
                Ok(BackendCapabilities::from_iter([
                    question_model::Capability::AlgorithmicGeneration,
                    question_model::Capability::ServerGrading,
                    question_model::Capability::PartialCredit,
                ]))
            }
            DraftQuestionSource::Qti { .. } if self.qti.is_some() => {
                Ok(BackendCapabilities::from_iter([
                    question_model::Capability::ServerGrading,
                ]))
            }
            _ => Err(BackendRegistryError::Unsupported),
        }
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
                })
            }
            question_model::QuestionSource::Imathas { .. } => {
                self.imathas_for(question)?
                    .issue(context, reference, question, seed)
                    .await
            }
            question_model::QuestionSource::Qti { .. } => {
                self.qti_for(question)?
                    .issue(context, reference, question, seed)
                    .await
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
                self.qti_for(question)?
                    .reproduce(context, reference, question, attempt)
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
                self.qti_for(question)?
                    .grade(context, reference, question, attempt, response)
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
        match submission.question.source {
            question_model::QuestionSource::Native { .. } => self.native.submit(submission).await,
            question_model::QuestionSource::Webwork { .. } => self
                .webwork()?
                .grade(
                    submission.context,
                    submission.actor,
                    submission.reference,
                    submission.question,
                    submission.attempt,
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
                self.imathas_for(submission.question)?
                    .submit(submission)
                    .await
            }
            question_model::QuestionSource::Qti { .. } => {
                self.qti_for(submission.question)?.submit(submission).await
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
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<learning_data_access::CreatedExternalToolLaunchSession, RunBackendError> {
        self.imathas_for(question)?
            .create_external_tool_launch(context, actor, reference, question, attempt, aead)
            .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn proxy_external_tool_activity(
        &self,
        context: TenantContext,
        actor: question_model::UserId,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        session_id: uuid::Uuid,
        token: &learning_data_access::ExternalToolLaunchToken,
        method: adapter_imathas::broker_provider::ProxyMethod,
        body: &[u8],
        aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<adapter_imathas::broker_provider::ProxyResponse, RunBackendError> {
        self.imathas_for(question)?
            .proxy_external_tool_activity(
                context, actor, reference, question, attempt, session_id, token, method, body, aead,
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
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        idempotency_key: learning_data_access::SubmissionIdempotencyKey,
        launch_proof: learning_data_access::ExternalToolLaunchProof,
        state_aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        self.imathas_for(question)?
            .submit_external_tool(
                context,
                actor,
                reference,
                question,
                attempt,
                idempotency_key,
                launch_proof,
                state_aead,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use learning_data_access::{
        ExternalToolLaunchProof, ExternalToolLaunchToken, SubmissionIdempotencyKey,
    };
    use question_model::generation::RandomizationDefinition;
    use question_model::response::ResponseDefinition;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, AttemptProvenance, AttemptTimerRecord, DraftQuestionSource,
        GradingDefinition, ImplementationVersion, ObjectId, ProblemId, QuestionAttempt,
        QuestionAttemptId, QuestionDefinition, QuestionMetadata, QuestionSource, RunId,
        StudentResponse, TenantId, UserId, VersionId, WorkspaceId, WorkspaceImportId,
    };
    use uuid::Uuid;

    use super::*;

    #[derive(Default)]
    struct CountingConfiguredProvider {
        catalog: AtomicUsize,
        objects: AtomicUsize,
        transport: AtomicUsize,
    }

    impl CountingConfiguredProvider {
        fn touch(&self) {
            self.catalog.fetch_add(1, Ordering::SeqCst);
            self.objects.fetch_add(1, Ordering::SeqCst);
            self.transport.fetch_add(1, Ordering::SeqCst);
        }
        fn assert_untouched(&self) {
            assert_eq!(self.catalog.load(Ordering::SeqCst), 0);
            assert_eq!(self.objects.load(Ordering::SeqCst), 0);
            assert_eq!(self.transport.load(Ordering::SeqCst), 0);
        }
    }

    #[async_trait]
    impl RunBackend for CountingConfiguredProvider {
        async fn issue(
            &self,
            _: TenantContext,
            _: ProblemVersionRef,
            _: &QuestionDefinition,
            _: u64,
        ) -> Result<IssuedAttemptMetadata, RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded provider".into()))
        }
        async fn reproduce(
            &self,
            _: TenantContext,
            _: ProblemVersionRef,
            _: &QuestionDefinition,
            _: &QuestionAttempt,
        ) -> Result<QuestionEnvelope, RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded provider".into()))
        }
        async fn grade(
            &self,
            _: TenantContext,
            _: ProblemVersionRef,
            _: &QuestionDefinition,
            _: &QuestionAttempt,
            _: &StudentResponse,
        ) -> Result<grading::GradeOutcome, RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded provider".into()))
        }
        async fn submit(
            &self,
            _: RunSubmission<'_>,
        ) -> Result<SubmissionDisposition, RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded provider".into()))
        }
        async fn prepare_external_tool_launch(
            &self,
            _: TenantContext,
            _: ProblemVersionRef,
            _: &QuestionDefinition,
            _: &QuestionAttempt,
        ) -> Result<(), RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded provider".into()))
        }
    }

    #[async_trait]
    impl ExternalToolLaunchBackend for CountingConfiguredProvider {
        async fn create_external_tool_launch(
            &self,
            _: TenantContext,
            _: UserId,
            _: ProblemVersionRef,
            _: &QuestionDefinition,
            _: &QuestionAttempt,
            _: &crate::imathas_backend::LaunchStateAead,
        ) -> Result<learning_data_access::CreatedExternalToolLaunchSession, RunBackendError>
        {
            self.touch();
            Err(RunBackendError::Unavailable("recorded provider".into()))
        }
        async fn proxy_external_tool_activity(
            &self,
            _: TenantContext,
            _: UserId,
            _: ProblemVersionRef,
            _: &QuestionDefinition,
            _: &QuestionAttempt,
            _: Uuid,
            _: &ExternalToolLaunchToken,
            _: adapter_imathas::broker_provider::ProxyMethod,
            _: &[u8],
            _: &crate::imathas_backend::LaunchStateAead,
        ) -> Result<adapter_imathas::broker_provider::ProxyResponse, RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded provider".into()))
        }
    }

    #[async_trait]
    impl ExternalToolSubmissionBackend for CountingConfiguredProvider {
        async fn submit_external_tool(
            &self,
            _: TenantContext,
            _: UserId,
            _: ProblemVersionRef,
            _: &QuestionDefinition,
            _: &QuestionAttempt,
            _: SubmissionIdempotencyKey,
            _: ExternalToolLaunchProof,
            _: &crate::imathas_backend::LaunchStateAead,
        ) -> Result<SubmissionDisposition, RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded provider".into()))
        }
    }

    impl ConfiguredImathas for CountingConfiguredProvider {
        fn serves_provider(&self, provider: &str) -> bool {
            provider == "provider-a"
        }
    }

    #[derive(Default)]
    struct CountingConfiguredQti {
        calls: AtomicUsize,
    }

    impl CountingConfiguredQti {
        fn touch(&self) {
            self.calls.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl RunBackend for CountingConfiguredQti {
        async fn issue(
            &self,
            _: TenantContext,
            _: ProblemVersionRef,
            _: &QuestionDefinition,
            _: u64,
        ) -> Result<IssuedAttemptMetadata, RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded QTI grader".into()))
        }

        async fn reproduce(
            &self,
            _: TenantContext,
            _: ProblemVersionRef,
            _: &QuestionDefinition,
            _: &QuestionAttempt,
        ) -> Result<QuestionEnvelope, RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded QTI grader".into()))
        }

        async fn grade(
            &self,
            _: TenantContext,
            _: ProblemVersionRef,
            _: &QuestionDefinition,
            _: &QuestionAttempt,
            _: &StudentResponse,
        ) -> Result<grading::GradeOutcome, RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded QTI grader".into()))
        }

        async fn submit(
            &self,
            _: RunSubmission<'_>,
        ) -> Result<SubmissionDisposition, RunBackendError> {
            self.touch();
            Err(RunBackendError::Unavailable("recorded QTI grader".into()))
        }
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }
    fn question(provider: &str) -> QuestionDefinition {
        QuestionDefinition {
            problem: ProblemId::from_uuid(id(1)),
            version: VersionId::from_uuid(id(2)),
            workspace: WorkspaceId::from_uuid(id(3)),
            source: QuestionSource::Imathas {
                provider: provider.into(),
                item_ref: "item-1".into(),
                snapshot: ObjectId::from_uuid(id(4)),
                snapshot_sha256: "a".repeat(64),
                integration_profile: "imathas_scored_embed_broker_v1".into(),
            },
            prompt: vec![],
            response: ResponseDefinition::ExternalTool {},
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Recorded external question".into(),
                tags: vec![],
                taxonomy: vec![],
                license: License::CcBySa,
                language: "en-US".into(),
            },
        }
    }

    fn qti_question() -> QuestionDefinition {
        QuestionDefinition {
            problem: ProblemId::from_uuid(id(1)),
            version: VersionId::from_uuid(id(2)),
            workspace: WorkspaceId::from_uuid(id(3)),
            source: QuestionSource::Qti {
                item_id: "choice-1".into(),
                package_object: ObjectId::from_uuid(id(4)),
                package_sha256: "a".repeat(64),
            },
            prompt: vec![],
            response: ResponseDefinition::ExternalTool {},
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Recorded QTI question".into(),
                tags: vec![],
                taxonomy: vec![],
                license: License::CcBySa,
                language: "en-US".into(),
            },
        }
    }

    fn composite_for_qti_tests() -> CompositeBackend<
        learning_data_access::in_memory::MemoryStore,
        objects::memory::MemoryObjectStore,
        adapter_webwork::HttpWebworkRenderer,
    > {
        let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
        let objects = Arc::new(objects::memory::MemoryObjectStore::default());
        let renderer = adapter_webwork::HttpWebworkRenderer::new(
            adapter_webwork::HttpWebworkRendererConfig::new(
                "http://renderer.internal/",
                std::time::Duration::from_secs(1),
                1_024,
                adapter_webwork::renderer_contract::RendererIdentity {
                    id: "recorded".into(),
                    version: "1".into(),
                },
            )
            .expect("renderer config"),
        )
        .expect("renderer");
        let native = NativeBackend::new(
            Arc::new(adapter_native::NativeAdapter::new()),
            Arc::clone(&store),
        );
        let webwork = WebworkBackend::new(
            store,
            Arc::clone(&objects),
            Arc::new(adapter_webwork::WebworkAdapter::new(
                objects.as_ref().clone(),
                renderer,
            )),
        );
        CompositeBackend::new(native, webwork)
    }

    #[test]
    fn native_only_registry_does_not_advertise_webwork() {
        let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
        let native = NativeBackend::new(
            Arc::new(adapter_native::NativeAdapter::new()),
            Arc::clone(&store),
        );
        let composite = CompositeBackend::<
            learning_data_access::in_memory::MemoryStore,
            objects::memory::MemoryObjectStore,
            adapter_webwork::HttpWebworkRenderer,
        >::native_only(native);

        assert!(matches!(
            composite.capabilities(&DraftQuestionSource::Webwork {
                pg_path: "Library/PLE/example.pg".into(),
            }),
            Err(BackendRegistryError::Unsupported)
        ));
    }

    fn attempt() -> QuestionAttempt {
        QuestionAttempt {
            id: QuestionAttemptId::from_uuid(id(5)),
            tenant: TenantId::from_uuid(id(6)),
            run: RunId::from_uuid(id(7)),
            problem: ProblemId::from_uuid(id(1)),
            question_version: VersionId::from_uuid(id(2)),
            assignment_position: 0,
            seed: 1,
            parameter_hash: "p".into(),
            response: None,
            status: question_model::AttemptStatus::InProgress,
            result: None,
            timer: AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(1),
                deadline: None,
                submitted_at: None,
            },
            provenance: AttemptProvenance {
                adapter: ImplementationVersion {
                    id: "test".into(),
                    version: "1".into(),
                },
                renderer: None,
                generator: None,
                source_artifact: None,
                asset_objects: vec![],
                grading: ImplementationVersion {
                    id: "test".into(),
                    version: "1".into(),
                },
                rendered_question_sha256: "a".repeat(64),
            },
        }
    }

    #[tokio::test]
    async fn foreign_imathas_provider_refuses_every_dispatch_before_any_delegate() {
        let provider = Arc::new(CountingConfiguredProvider::default());
        let question_b = question("provider-b");
        let reference = ProblemVersionRef {
            problem: question_b.problem,
            version: question_b.version,
        };
        let attempt = attempt();
        let context = TenantContext::from_authenticated_session(attempt.tenant);
        let actor = UserId::from_uuid(id(8));
        let aead =
            crate::imathas_backend::LaunchStateAead::from_server_secret([9; 32]).expect("aead");
        let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
        let objects = Arc::new(objects::memory::MemoryObjectStore::default());
        let renderer = adapter_webwork::HttpWebworkRenderer::new(
            adapter_webwork::HttpWebworkRendererConfig::new(
                "http://renderer.internal/",
                std::time::Duration::from_secs(1),
                1_024,
                adapter_webwork::renderer_contract::RendererIdentity {
                    id: "recorded".into(),
                    version: "1".into(),
                },
            )
            .expect("renderer config"),
        )
        .expect("renderer");
        let native = NativeBackend::new(
            Arc::new(adapter_native::NativeAdapter::new()),
            Arc::clone(&store),
        );
        let webwork = WebworkBackend::new(
            Arc::clone(&store),
            Arc::clone(&objects),
            Arc::new(adapter_webwork::WebworkAdapter::new(
                objects.as_ref().clone(),
                renderer,
            )),
        );
        let composite = CompositeBackend::new(native, webwork).with_imathas(provider.clone());
        let response = StudentResponse::ExternalTool {};
        let key = SubmissionIdempotencyKey::parse("recorded-key").expect("key");
        let token = ExternalToolLaunchToken::parse_cookie_value(
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        )
        .expect("token");
        let proof = ExternalToolLaunchProof {
            session_id: id(9),
            token: token.clone(),
        };
        assert!(matches!(
            composite.capabilities(&DraftQuestionSource::Imathas {
                provider: "provider-b".into(),
                item_ref: "item-1".into()
            }),
            Err(BackendRegistryError::Unsupported)
        ));
        assert!(matches!(
            composite.issue(context, reference, &question_b, 1).await,
            Err(RunBackendError::Unsupported(_))
        ));
        assert!(matches!(
            composite
                .reproduce(context, reference, &question_b, &attempt)
                .await,
            Err(RunBackendError::Unsupported(_))
        ));
        assert!(matches!(
            composite
                .grade(context, reference, &question_b, &attempt, &response)
                .await,
            Err(RunBackendError::Unsupported(_))
        ));
        assert!(matches!(
            composite
                .prepare_external_tool_launch(context, reference, &question_b, &attempt)
                .await,
            Err(RunBackendError::Unsupported(_))
        ));
        assert!(matches!(
            composite
                .submit(RunSubmission {
                    context,
                    actor,
                    idempotency_key: key.clone(),
                    reference,
                    question: &question_b,
                    attempt: &attempt,
                    response: &response
                })
                .await,
            Err(RunBackendError::Unsupported(_))
        ));
        assert!(matches!(
            composite
                .create_external_tool_launch(
                    context,
                    actor,
                    reference,
                    &question_b,
                    &attempt,
                    &aead
                )
                .await,
            Err(RunBackendError::Unsupported(_))
        ));
        assert!(matches!(
            composite
                .proxy_external_tool_activity(
                    context,
                    actor,
                    reference,
                    &question_b,
                    &attempt,
                    id(10),
                    &token,
                    adapter_imathas::broker_provider::ProxyMethod::Get,
                    &[],
                    &aead
                )
                .await,
            Err(RunBackendError::Unsupported(_))
        ));
        assert!(matches!(
            composite
                .submit_external_tool(
                    context,
                    actor,
                    reference,
                    &question_b,
                    &attempt,
                    key,
                    proof,
                    &aead
                )
                .await,
            Err(RunBackendError::Unsupported(_))
        ));
        provider.assert_untouched();
        assert!(matches!(
            composite
                .issue(context, reference, &question("provider-a"), 1)
                .await,
            Err(RunBackendError::Unavailable(_))
        ));
        assert_eq!(
            provider.transport.load(Ordering::SeqCst),
            1,
            "configured provider dispatches"
        );
    }

    #[tokio::test]
    async fn qti_dispatch_is_explicit_and_non_qti_sources_never_touch_its_grader() {
        let published = qti_question();
        let reference = ProblemVersionRef {
            problem: published.problem,
            version: published.version,
        };
        let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(6)));

        let disabled = composite_for_qti_tests();
        assert!(!disabled.has_qti());
        assert!(matches!(
            disabled.capabilities(&DraftQuestionSource::Qti {
                item_id: "choice-1".into(),
                import_id: WorkspaceImportId::from_uuid(id(44)),
            }),
            Err(BackendRegistryError::Unsupported)
        ));
        assert!(matches!(
            disabled.issue(context, reference, &published, 1).await,
            Err(RunBackendError::Unsupported(_))
        ));

        let grader = Arc::new(CountingConfiguredQti::default());
        let configured = composite_for_qti_tests().with_qti(grader.clone());
        assert!(configured.has_qti());
        let capabilities = configured
            .capabilities(&DraftQuestionSource::Qti {
                item_id: "choice-1".into(),
                import_id: WorkspaceImportId::from_uuid(id(44)),
            })
            .expect("configured QTI capabilities");
        assert!(capabilities.supports(question_model::Capability::ServerGrading));
        assert_eq!(
            capabilities.declared().collect::<Vec<_>>(),
            vec![question_model::Capability::ServerGrading],
            "QTI declares only implemented capabilities"
        );
        assert!(matches!(
            configured.issue(context, reference, &published, 1).await,
            Err(RunBackendError::Unavailable(_))
        ));
        assert_eq!(grader.calls.load(Ordering::SeqCst), 1);

        let mut native = published.clone();
        native.source = QuestionSource::Native {
            family: "unregistered-native-family".into(),
        };
        assert!(matches!(
            configured.issue(context, reference, &native, 1).await,
            Err(RunBackendError::Unsupported(_))
        ));
        assert_eq!(
            grader.calls.load(Ordering::SeqCst),
            1,
            "non-QTI dispatch cannot reach the QTI grading boundary"
        );
    }
}
