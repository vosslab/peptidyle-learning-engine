use std::sync::Arc;

use super::*;

use adapter_imathas::test_support::{
    RecordedImathasProvider, RecordedImathasProviderFactory, RecordedProviderMode,
};
use adapter_imathas::{
    CorrelationIssuer, GradeBinding, PersistedCorrelation as AdapterCorrelation,
};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AssignmentRecord, BeginExternalToolGradeCommand, CatalogSourceStore, CatalogStore,
    CourseRecord, DraftRecord, ExternalToolBegin, ExternalToolBrokerStore,
    IssueQuestionAttemptCommand, PersistedCorrelation, PublishDraftCommand,
    StageExternalToolVerificationCommand, Store,
};
use objects::memory::MemoryObjectStore;
use objects::{ObjectKey, ObjectStore, PutObject, Sha256Digest};
use question_model::capability::Capability;
use question_model::generation::{RandomizationDefinition, Seed};
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AttemptResult, BackendCapabilities,
    CompletionRequirement, ContinuedPractice, CourseId, CourseMembership, CourseMembershipRole,
    DraftQuestionDefinition, DraftQuestionSource, EnrollmentId, GradePolicy, GradingDefinition,
    PresentationBindingV1, PresentationDigestV1, PresentationNonceV1, ProblemId, QuestionAttemptId,
    QuestionMetadata, QuestionSource, RunId, RunPolicies, StudentId, StudentResponse, TenantId,
    UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

#[test]
fn launch_state_aead_binds_each_issued_identity() {
    let aead = LaunchStateAead::from_server_secret([9; 32]).expect("aead");
    let aad = b"tenant\0actor\0attempt\0problem\0version\0seed\0provider\0source\0profile\0";
    let sealed = aead.seal(b"adapter-private-session", aad).expect("seal");
    assert_ne!(sealed, b"adapter-private-session");
    assert_eq!(
        aead.open(&sealed, aad).expect("open"),
        b"adapter-private-session"
    );
    assert!(aead.open(&sealed, b"other identity").is_err());
    let mut altered = sealed;
    *altered.last_mut().expect("ciphertext") ^= 1;
    assert!(aead.open(&altered, aad).is_err());
}

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn presentation_binding(marker: u8) -> PresentationBindingV1 {
    PresentationBindingV1::new(
        PresentationNonceV1::from_bytes([marker; 16]),
        PresentationDigestV1::compute(&[marker]),
    )
}

type TestBackend = ImathasBackend<MemoryStore, MemoryObjectStore, RecordedImathasProvider>;

struct Fixture {
    store: Arc<MemoryStore>,
    backend: TestBackend,
    provider: RecordedImathasProvider,
    context: TenantContext,
    actor: UserId,
    reference: ProblemVersionRef,
    question: QuestionDefinition,
    attempt: QuestionAttempt,
}

async fn fixture() -> Fixture {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
        .expect("fixture clock");
    let objects = Arc::new(MemoryObjectStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let actor = UserId::from_uuid(id(2));
    let instructor = UserId::from_uuid(id(13));
    let workspace = WorkspaceId::from_uuid(id(3));
    let problem = ProblemId::from_uuid(id(4));
    let version = VersionId::from_uuid(id(5));
    let snapshot = question_model::ObjectId::from_uuid(id(6));
    let digest = Sha256Digest::compute(br#"{"recorded":true}"#).to_string();
    let question_source = QuestionSource::Imathas {
        provider: "recorded-provider".into(),
        item_ref: "item-17".into(),
        snapshot,
        snapshot_sha256: digest,
        integration_profile: "recorded-v1".into(),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Imathas {
                provider: "recorded-provider".into(),
                item_ref: "item-17".into(),
            },
            prompt: Vec::new(),
            response: question_model::ResponseDefinition::ExternalTool {},
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Recorded iMathAS question".into(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".into(),
            },
        },
        revises: None,
        derived_from: None,
    };
    objects
        .put(PutObject {
            key: ObjectKey::ProblemSource {
                problem,
                version,
                object: snapshot,
            },
            bytes: br#"{"recorded":true}"#.to_vec(),
            media_type: "application/json".into(),
            license: "CC-BY-SA-4.0".into(),
            provenance: "recorded fixture".into(),
            created_at: ActivityTimestamp::from_unix_millis(10_000),
        })
        .await
        .expect("source object");
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("draft");
    let reference = ProblemVersionRef { problem, version };
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: question_source,
                source_artifact: Some(learning_data_access::PublishedSourceArtifact {
                    reference,
                    backend: question_model::QuestionBackend::Imathas,
                    object: objects
                        .get(&ObjectKey::ProblemSource {
                            problem,
                            version,
                            object: snapshot,
                        })
                        .await
                        .expect("stored source")
                        .record,
                }),
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: instructor,
                scope: question_model::PublicationScope::Public,
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
    let course = CourseId::from_uuid(id(7));
    let assignment = AssignmentId::from_uuid(id(8));
    let enrollment = EnrollmentId::from_uuid(id(9));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Recorded course".into(),
                members: vec![
                    CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: actor,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("course");
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Recorded assignment".into(),
                items: vec![question_model::AssignmentItem {
                    id: question_model::AssignmentItemId::from_uuid(id(10)),
                    reference,
                    position: 0,
                    points_possible: question_model::PointValue::from_whole(1),
                    delivery_state: question_model::AssignmentDeliveryState::Active,
                    scoring_mode: question_model::AssignmentScoringMode::Normal,
                }],
                selection_groups: Vec::new(),
                policies: RunPolicies {
                    completion: CompletionRequirement::AllCorrect,
                    grade: GradePolicy::Highest,
                    continued_practice: ContinuedPractice::Unlimited,
                    variation: question_model::VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment,
                tenant,
                assignment,
                user: actor,
                student: StudentId::from_uuid(id(10)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("enrollment");
    let run = store
        .start_or_resume_run(context, actor, assignment, RunId::from_uuid(id(11)))
        .await
        .expect("run");
    let provider = RecordedImathasProviderFactory::new(RecordedProviderMode::Verified).build();
    let adapter = Arc::new(ImathasAdapter::new(
        objects.as_ref().clone(),
        provider.clone(),
        [
            adapter_imathas::SupportedProfile::new("recorded-v1", true, true, true)
                .expect("profile"),
        ],
    ));
    let backend = ImathasBackend::new(
        Arc::clone(&store),
        Arc::clone(&objects),
        adapter,
        Arc::new(CorrelationIssuer::from_server_secret([3; 32])),
    );
    let issued = backend
        .issue(context, reference, &question, 17)
        .await
        .expect("issue");
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor,
                attempt: QuestionAttemptId::from_uuid(id(12)),
                run: run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: 17,
                presentation: presentation_binding(17),
                parameter_hash: issued.parameter_hash,
                provenance: issued.provenance,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("attempt");
    Fixture {
        store,
        backend,
        provider,
        context,
        actor,
        reference,
        question,
        attempt,
    }
}

fn submission<'a>(
    fixture: &'a Fixture,
    key: &str,
    response: &'a StudentResponse,
) -> RunSubmission<'a> {
    RunSubmission {
        context: fixture.context,
        actor: fixture.actor,
        idempotency_key: learning_data_access::SubmissionIdempotencyKey::parse(key).expect("key"),
        reference: fixture.reference,
        question: &fixture.question,
        attempt: &fixture.attempt,
        response,
    }
}

#[tokio::test]
async fn generic_submission_refuses_without_an_authenticated_launch_session() {
    let fixture = fixture().await;
    let envelope = fixture
        .backend
        .reproduce(
            fixture.context,
            fixture.reference,
            &fixture.question,
            &fixture.attempt,
        )
        .await
        .expect("exact reproduction");
    assert_eq!(envelope.seed, Seed::new(fixture.attempt.seed));
    assert_eq!(
        envelope.response,
        question_model::ResponseDefinition::ExternalTool {}
    );
    let response = StudentResponse::ExternalTool {};
    let refused = fixture
        .backend
        .submit(submission(&fixture, "server-imathas-first", &response))
        .await
        .expect_err("generic submission must not bypass launch ownership");
    assert!(matches!(refused, RunBackendError::Unsupported(_)));
    assert_eq!(fixture.provider.grade_calls(), 0);
}

#[tokio::test]
async fn generic_submission_never_reaches_a_provider_without_launch_ownership() {
    let fixture = fixture().await;
    let response = StudentResponse::ExternalTool {};
    let mut parameter_tamper = fixture.attempt.clone();
    parameter_tamper.parameter_hash.push('x');
    let parameter = fixture
        .backend
        .submit(RunSubmission {
            context: fixture.context,
            actor: fixture.actor,
            idempotency_key: learning_data_access::SubmissionIdempotencyKey::parse(
                "server-imathas-param",
            )
            .expect("key"),
            reference: fixture.reference,
            question: &fixture.question,
            attempt: &parameter_tamper,
            response: &response,
        })
        .await;
    assert!(matches!(parameter, Err(RunBackendError::Unsupported(_))));
    assert_eq!(fixture.provider.grade_calls(), 0);

    let mut provenance_tamper = fixture.attempt.clone();
    provenance_tamper
        .provenance
        .rendered_question_sha256
        .push('x');
    let provenance = fixture
        .backend
        .prepare_external_tool_launch(
            fixture.context,
            fixture.reference,
            &fixture.question,
            &provenance_tamper,
        )
        .await;
    assert!(matches!(provenance, Err(RunBackendError::Invalid(_))));
    assert_eq!(fixture.provider.grade_calls(), 0);

    let mut source_tamper = fixture.question.clone();
    if let QuestionSource::Imathas {
        snapshot_sha256, ..
    } = &mut source_tamper.source
    {
        snapshot_sha256.replace_range(..1, "0");
    }
    let source = fixture
        .backend
        .submit(RunSubmission {
            context: fixture.context,
            actor: fixture.actor,
            idempotency_key: learning_data_access::SubmissionIdempotencyKey::parse(
                "server-imathas-source",
            )
            .expect("key"),
            reference: fixture.reference,
            question: &source_tamper,
            attempt: &fixture.attempt,
            response: &response,
        })
        .await;
    assert!(matches!(source, Err(RunBackendError::Unsupported(_))));
    assert_eq!(fixture.provider.grade_calls(), 0);
}

async fn binding_for(fixture: &Fixture, response: &StudentResponse) -> ExternalToolBinding {
    let artifact = fixture
        .store
        .catalog_source_artifact(fixture.context, fixture.reference)
        .await
        .expect("source lookup")
        .expect("source artifact");
    TestBackend::binding(&fixture.question, &fixture.attempt, &artifact, response).expect("binding")
}

#[tokio::test]
async fn generic_submission_refuses_even_when_a_broker_exchange_exists() {
    let tampered_fixture = fixture().await;
    let response = StudentResponse::ExternalTool {};
    let binding = binding_for(&tampered_fixture, &response).await;
    let key = learning_data_access::SubmissionIdempotencyKey::parse("server-imathas-correlation")
        .expect("key");
    tampered_fixture
        .store
        .begin_or_resume_external_grade(
            tampered_fixture.context,
            BeginExternalToolGradeCommand {
                actor: tampered_fixture.actor,
                attempt: tampered_fixture.attempt.id,
                response: response.clone(),
                idempotency_key: key.clone(),
                binding: binding.clone(),
                proposed_correlation: PersistedCorrelation::new(b"corrupted".to_vec())
                    .expect("bounded corruption"),
                lease_millis: EXTERNAL_TOOL_LEASE_MILLIS,
            },
        )
        .await
        .expect("tampered lease setup");
    tampered_fixture
        .store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(41_000))
        .expect("expire fixture lease");
    let tampered = tampered_fixture
        .backend
        .submit(submission(
            &tampered_fixture,
            "server-imathas-correlation",
            &response,
        ))
        .await;
    assert!(matches!(tampered, Err(RunBackendError::Unsupported(_))));
    assert_eq!(tampered_fixture.provider.grade_calls(), 0);

    let in_progress = fixture().await;
    let binding = binding_for(&in_progress, &response).await;
    let grade_binding = TestBackend::correlation_binding(in_progress.context, &in_progress.attempt);
    in_progress
        .store
        .begin_or_resume_external_grade(
            in_progress.context,
            BeginExternalToolGradeCommand {
                actor: in_progress.actor,
                attempt: in_progress.attempt.id,
                response: response.clone(),
                idempotency_key: learning_data_access::SubmissionIdempotencyKey::parse(
                    "server-imathas-busy",
                )
                .expect("key"),
                binding,
                proposed_correlation: in_progress
                    .backend
                    .persisted_correlation(grade_binding)
                    .expect("correlation"),
                lease_millis: EXTERNAL_TOOL_LEASE_MILLIS,
            },
        )
        .await
        .expect("active lease setup");
    let busy = in_progress
        .backend
        .submit(submission(&in_progress, "server-imathas-busy", &response))
        .await;
    assert!(matches!(busy, Err(RunBackendError::Unsupported(_))));
    assert_eq!(in_progress.provider.grade_calls(), 0);
}

#[tokio::test]
async fn generic_submission_cannot_commit_verified_pending_without_launch_proof() {
    let fixture = fixture().await;
    let response = StudentResponse::ExternalTool {};
    let binding = binding_for(&fixture, &response).await;
    let key = learning_data_access::SubmissionIdempotencyKey::parse("server-imathas-verified")
        .expect("key");
    let grade_binding = TestBackend::correlation_binding(fixture.context, &fixture.attempt);
    let lease = fixture
        .store
        .begin_or_resume_external_grade(
            fixture.context,
            BeginExternalToolGradeCommand {
                actor: fixture.actor,
                attempt: fixture.attempt.id,
                response: response.clone(),
                idempotency_key: key.clone(),
                binding: binding.clone(),
                proposed_correlation: fixture
                    .backend
                    .persisted_correlation(grade_binding)
                    .expect("correlation"),
                lease_millis: EXTERNAL_TOOL_LEASE_MILLIS,
            },
        )
        .await
        .expect("lease setup");
    let ExternalToolBegin::Lease(lease) = lease else {
        panic!("new exchange must lease")
    };
    fixture
        .store
        .stage_external_tool_verification(
            fixture.context,
            StageExternalToolVerificationCommand {
                actor: fixture.actor,
                attempt: fixture.attempt.id,
                response: response.clone(),
                idempotency_key: key,
                binding: binding.clone(),
                correlation: lease.correlation,
                lease_token: lease.token,
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
            },
        )
        .await
        .expect("staged verified receipt");
    let recovered = fixture
        .backend
        .submit(submission(&fixture, "server-imathas-verified", &response))
        .await
        .expect_err("generic submission has no authenticated launch proof");
    assert!(matches!(recovered, RunBackendError::Unsupported(_)));
    assert_eq!(fixture.provider.grade_calls(), 0);
}

#[test]
fn stored_correlation_round_trips_only_with_its_exact_mac_binding() {
    let issuer = CorrelationIssuer::from_server_secret([7; 32]);
    let binding = GradeBinding {
        tenant: TenantId::from_uuid(id(1)),
        attempt: QuestionAttemptId::from_uuid(id(2)),
        problem: ProblemId::from_uuid(id(3)),
        version: VersionId::from_uuid(id(4)),
        seed: Seed::new(5),
    };
    let adapter_value = issuer.begin(binding);
    let stored = PersistedCorrelation::new(adapter_value.to_storage_value().into_bytes())
        .expect("bounded adapter correlation persists");
    let stored_bytes = stored.to_storage_bytes();
    let encoded = std::str::from_utf8(&stored_bytes).expect("adapter correlation is UTF-8");
    let restored = AdapterCorrelation::from_storage_value(encoded)
        .expect("canonical adapter correlation restores");
    assert!(issuer.restore(binding, &restored).is_ok());

    let altered = GradeBinding {
        seed: Seed::new(6),
        ..binding
    };
    assert!(issuer.restore(altered, &restored).is_err());
}
