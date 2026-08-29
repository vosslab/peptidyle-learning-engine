use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::catalog::{BackendRegistry, BackendRegistryError};
use async_trait::async_trait;
use learning_data_access::{
    ExternalToolLaunchProof, ExternalToolLaunchToken, IssuedQuestionFamilyWitnessV1,
    IssuedQuestionSnapshotV1, SubmissionIdempotencyKey,
};
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
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
    async fn submit(&self, _: RunSubmission<'_>) -> Result<SubmissionDisposition, RunBackendError> {
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
        _: learning_data_access::StudentWorkRoutingBinding,
        _: &learning_data_access::IssuedQuestionSnapshotV1,
        _: &QuestionAttempt,
        _: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<learning_data_access::CreatedExternalToolLaunchSession, RunBackendError> {
        self.touch();
        Err(RunBackendError::Unavailable("recorded provider".into()))
    }
    async fn proxy_external_tool_activity(
        &self,
        _: TenantContext,
        _: UserId,
        _: learning_data_access::StudentWorkRoutingBinding,
        _: &learning_data_access::IssuedQuestionSnapshotV1,
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
        _: learning_data_access::StudentWorkRoutingBinding,
        _: &learning_data_access::IssuedQuestionSnapshotV1,
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

    async fn submit(&self, _: RunSubmission<'_>) -> Result<SubmissionDisposition, RunBackendError> {
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
        attempt_policy: AttemptPolicy { max_attempts: None },
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
        attempt_policy: AttemptPolicy { max_attempts: None },
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
        issued_capability: question_model::IssuedAttemptCapabilityV1::NotApplicable,
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
    let student_work_binding = learning_data_access::StudentWorkRoutingBinding::new(
        question_model::CourseId::from_uuid(id(11)),
        question_model::AssignmentId::from_uuid(id(12)),
    );
    let aead = crate::imathas_backend::LaunchStateAead::from_server_secret([9; 32]).expect("aead");
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
    let token =
        ExternalToolLaunchToken::parse_cookie_value("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
            .expect("token");
    let proof = ExternalToolLaunchProof {
        session_id: id(9),
        token: token.clone(),
    };
    let question_b_snapshot = IssuedQuestionSnapshotV1::new(
        question_b.clone(),
        IssuedQuestionFamilyWitnessV1::External {
            source_artifact: question_model::SourceArtifact {
                object: match &question_b.source {
                    question_model::QuestionSource::Imathas { snapshot, .. } => *snapshot,
                    _ => panic!("fixture is iMathAS"),
                },
                sha256: match &question_b.source {
                    question_model::QuestionSource::Imathas {
                        snapshot_sha256, ..
                    } => snapshot_sha256.clone(),
                    _ => panic!("fixture is iMathAS"),
                },
            },
            integration_profile_identity: match &question_b.source {
                question_model::QuestionSource::Imathas {
                    integration_profile,
                    ..
                } => integration_profile.clone(),
                _ => panic!("fixture is iMathAS"),
            },
        },
    )
    .expect("issued iMathAS snapshot");
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
                issued_question_snapshot: &question_b_snapshot,
                attempt: &attempt,
                issued_grading_envelope: None,
                issued_flat_grading: None,
                issued_webwork_grading: None,
                issued_qti_grading: None,
                issued_webwork_replay: None,
                issued_presentation_binding: None,
                issued_presentation: None,
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
                student_work_binding,
                &question_b_snapshot,
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
                student_work_binding,
                &question_b_snapshot,
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
                student_work_binding,
                &question_b_snapshot,
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
