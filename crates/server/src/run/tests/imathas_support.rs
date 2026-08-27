use super::*;

pub(super) struct CountingExternalRouteBackend {
    pub(super) inner: Arc<ContractedRouteBackend>,
    pub(super) create_calls: AtomicUsize,
    pub(super) proxy_calls: AtomicUsize,
    pub(super) submission_calls: AtomicUsize,
}

#[async_trait]
impl RunBackend for CountingExternalRouteBackend {
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        self.inner.issue(context, reference, question, seed).await
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        self.inner
            .reproduce(context, reference, question, attempt)
            .await
    }

    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError> {
        self.inner
            .grade(context, reference, question, attempt, response)
            .await
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        self.inner.submit(submission).await
    }
}

#[async_trait]
impl ExternalToolLaunchBackend for CountingExternalRouteBackend {
    async fn create_external_tool_launch(
        &self,
        context: TenantContext,
        actor: UserId,
        learner_work_binding: LearnerWorkRoutingBinding,
        issued_question_snapshot: &learning_data_access::IssuedQuestionSnapshotV1,
        attempt: &QuestionAttempt,
        aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<learning_data_access::CreatedExternalToolLaunchSession, RunBackendError> {
        self.create_calls.fetch_add(1, Ordering::SeqCst);
        self.inner
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

    async fn proxy_external_tool_activity(
        &self,
        context: TenantContext,
        actor: UserId,
        learner_work_binding: LearnerWorkRoutingBinding,
        issued_question_snapshot: &learning_data_access::IssuedQuestionSnapshotV1,
        attempt: &QuestionAttempt,
        session_id: Uuid,
        token: &learning_data_access::ExternalToolLaunchToken,
        method: adapter_imathas::broker_provider::ProxyMethod,
        body: &[u8],
        aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<adapter_imathas::broker_provider::ProxyResponse, RunBackendError> {
        self.proxy_calls.fetch_add(1, Ordering::SeqCst);
        self.inner
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
impl ExternalToolSubmissionBackend for CountingExternalRouteBackend {
    async fn submit_external_tool(
        &self,
        context: TenantContext,
        actor: UserId,
        learner_work_binding: LearnerWorkRoutingBinding,
        issued_question_snapshot: &learning_data_access::IssuedQuestionSnapshotV1,
        attempt: &QuestionAttempt,
        idempotency_key: learning_data_access::SubmissionIdempotencyKey,
        launch_proof: learning_data_access::ExternalToolLaunchProof,
        state_aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        self.submission_calls.fetch_add(1, Ordering::SeqCst);
        self.inner
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

pub(super) type ContractedRouteBackend = ImathasBackend<
    MemoryStore,
    objects::memory::MemoryObjectStore,
    adapter_imathas::broker_provider::ContractedScoredEmbedProvider<
        adapter_imathas::test_support::RecordedContractedTransport,
    >,
>;

pub(super) struct ContractedRouteFixture {
    pub(super) store: Arc<MemoryStore>,
    pub(super) objects: Arc<objects::memory::MemoryObjectStore>,
    pub(super) source_key: objects::ObjectKey,
    pub(super) backend: Arc<ContractedRouteBackend>,
    pub(super) route_backend: Arc<CountingExternalRouteBackend>,
    pub(super) transport: adapter_imathas::test_support::RecordedContractedTransport,
    pub(super) aead: Arc<crate::imathas_backend::LaunchStateAead>,
    pub(super) app: Router,
    pub(super) student_cookie: String,
    pub(super) outsider_cookie: String,
    pub(super) attempt: QuestionAttempt,
    pub(super) context: TenantContext,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) question: QuestionDefinition,
    pub(super) issued_question_snapshot: learning_data_access::IssuedQuestionSnapshotV1,
}

pub(super) async fn contracted_route_fixture(
    transport_mode: adapter_imathas::test_support::RecordedContractedTransportMode,
) -> ContractedRouteFixture {
    use adapter_imathas::test_support::RecordedContractedTransportFactory;
    use learning_data_access::{IssueQuestionAttemptCommand, PublishedSourceArtifact};
    use objects::{ObjectKey, ObjectStore, PutObject, Sha256Digest};
    use question_model::generation::RandomizationDefinition;

    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
        .expect("fixture clock");
    let objects = Arc::new(objects::memory::MemoryObjectStore::default());
    let tenant = TenantId::from_uuid(id(801));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(802));
    let actor = UserId::from_uuid(id(803));
    let outsider = UserId::from_uuid(id(804));
    let workspace = WorkspaceId::from_uuid(id(805));
    let problem = ProblemId::from_uuid(id(806));
    let version = VersionId::from_uuid(id(807));
    let snapshot = question_model::ObjectId::from_uuid(id(808));
    let source_bytes = br#"{"recorded":true}"#.to_vec();
    let source_sha256 = Sha256Digest::compute(&source_bytes).to_string();
    let source = QuestionSource::Imathas {
        provider: "institution-imathas".into(),
        item_ref: "17".into(),
        snapshot,
        snapshot_sha256: source_sha256,
        integration_profile: adapter_imathas::scored_embed::SCORED_EMBED_BROKER_PROFILE_ID.into(),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Imathas {
                provider: "institution-imathas".into(),
                item_ref: "17".into(),
            },
            prompt: Vec::new(),
            response: ResponseDefinition::ExternalTool {},
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Recorded contracted iMathAS question".into(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".into(),
            },
        },
        derived_from: None,
    };
    let reference = ProblemVersionRef { problem, version };
    let object_key = ObjectKey::ProblemSource {
        problem,
        version,
        object: snapshot,
    };
    objects
        .put(PutObject {
            key: object_key.clone(),
            bytes: source_bytes,
            media_type: "application/json".into(),
            license: "CC-BY-SA-4.0".into(),
            provenance: "recorded contracted route fixture".into(),
            created_at: ActivityTimestamp::from_unix_millis(10_000),
        })
        .await
        .expect("source object");
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("draft");
    let artifact = objects
        .get(&object_key)
        .await
        .expect("source record")
        .record;
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: source,
                source_artifact: Some(PublishedSourceArtifact {
                    reference,
                    backend: question_model::QuestionBackend::Imathas,
                    object: artifact,
                }),
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: BackendCapabilities::from_iter([
                    Capability::AlgorithmicGeneration,
                    Capability::ServerGrading,
                ]),
            },
        )
        .await
        .expect("publish");
    let question = store
        .get_catalog_problem(context, reference)
        .await
        .expect("catalog")
        .expect("published")
        .question;
    let course = CourseId::from_uuid(id(809));
    let assignment = AssignmentId::from_uuid(id(810));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Recorded course".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: crate::test_fixtures::sysadmin_course_creation_authority(
                    store.as_ref(),
                    tenant,
                    course,
                    instructor,
                )
                .await,
            },
        )
        .await
        .expect("course");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: actor,
                display_name: "Recorded learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student roster membership");
    store
        .create_assignment(
            context,
            learning_data_access::CreateAssignmentCommand {
                actor: instructor,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: question_model::AssignmentAudience::CourseWide,
                    title: "Recorded assignment".into(),
                    lifecycle: question_model::AssignmentLifecycle::Draft,
                    instructions: question_model::AssignmentInstructions::default(),
                    items: assignment_items(vec![reference, reference]),
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AllCorrect,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
                base_policy: question_model::BaseAssignmentPolicy::default(),
            },
        )
        .await
        .expect("assignment");
    crate::course::tests::fixtures::publish_assignment(
        store.as_ref(),
        context,
        instructor,
        course,
        assignment,
        question_model::AssignmentTeachingSettings {
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            base_policy: question_model::BaseAssignmentPolicy::default(),
        },
    )
    .await;
    let (provider, transport) = RecordedContractedTransportFactory::new(transport_mode)
        .contracted_provider_with_transport();
    let adapter = Arc::new(adapter_imathas::ImathasAdapter::new(
        objects.as_ref().clone(),
        provider,
        [adapter_imathas::SupportedProfile::new(
            adapter_imathas::scored_embed::SCORED_EMBED_BROKER_PROFILE_ID,
            true,
            true,
            true,
        )
        .expect("profile")],
    ));
    let backend = Arc::new(ImathasBackend::new(
        Arc::clone(&store),
        Arc::clone(&objects),
        adapter,
        Arc::new(adapter_imathas::CorrelationIssuer::from_server_secret(
            [83; 32],
        )),
        crate::imathas_backend::ExternalToolTiming::from_provider_timeout(
            std::time::Duration::from_secs(15),
        )
        .expect("bounded test timing"),
    ));
    let run = store
        .start_or_resume_run(
            context,
            actor,
            learning_data_access::LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(id(813)),
        )
        .await
        .expect("run");
    let issued = backend
        .issue(context, reference, &question, 17)
        .await
        .expect("issue");
    let issued_question_snapshot = learning_data_access::IssuedQuestionSnapshotV1::new(
        question.clone(),
        learning_data_access::IssuedQuestionFamilyWitnessV1::External {
            source_artifact: issued
                .provenance
                .source_artifact
                .clone()
                .expect("iMathAS issuance source artifact"),
            integration_profile_identity:
                adapter_imathas::scored_embed::SCORED_EMBED_BROKER_PROFILE_ID.into(),
        },
    )
    .expect("issued external snapshot");
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor,
                attempt: QuestionAttemptId::from_uuid(id(814)),
                run: run.id,
                binding: learning_data_access::LearnerWorkRoutingBinding::new(course, assignment),
                assignment_position: 0,
                problem,
                question_version: version,
                issued_question_snapshot: issued_question_snapshot.clone(),
                seed: 17,
                // iMathAS is an external-tool family, not a v1 presentation
                // family; its receipt explicitly records that no envelope is
                // required.
                presentation_capability:
                    learning_data_access::PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                native_execution_envelope_capability:
                    learning_data_access::NativeExecutionEnvelopeCapability::NotApplicable,
                flat_grading: None,
                flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: learning_data_access::QtiGradingCapability::NotApplicable,
                parameter_hash: issued.parameter_hash,
                provenance: issued.provenance,
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("attempt");
    let aead = Arc::new(
        crate::imathas_backend::LaunchStateAead::from_server_secret([84; 32]).expect("aead"),
    );
    let route_backend = Arc::new(CountingExternalRouteBackend {
        inner: Arc::clone(&backend),
        create_calls: AtomicUsize::new(0),
        proxy_calls: AtomicUsize::new(0),
        submission_calls: AtomicUsize::new(0),
    });
    let app = router(
        Arc::clone(&store),
        Arc::clone(&route_backend),
        sealed_memory(&store),
        learner_submission_status(&store),
        automated_grading(&store),
    )
    .merge(external_tool_router(
        Arc::clone(&store),
        Arc::clone(&route_backend),
        Arc::clone(&aead),
    ));
    ContractedRouteFixture {
        student_cookie: issued_cookie_for(store.as_ref(), tenant, actor, "Student").await,
        outsider_cookie: issued_cookie_for(store.as_ref(), tenant, outsider, "Outsider").await,
        store,
        objects,
        source_key: object_key,
        backend,
        route_backend,
        transport,
        aead,
        app,
        attempt,
        context,
        course,
        assignment,
        question,
        issued_question_snapshot,
    }
}
