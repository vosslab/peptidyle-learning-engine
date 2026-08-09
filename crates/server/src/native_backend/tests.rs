use std::collections::BTreeMap;

use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, CatalogStore, DraftRecord,
    FlatQuestionGradingPayload, FlatQuestionPublicationPromotion, FlatQuestionStore,
    PublishDraftCommand, PublishedSourceArtifact, Store, UpsertFlatQuestionCommand,
};
use objects::{ObjectKey, ObjectRecord, Sha256Digest};
use question_model::answer::SelectionCardinality;
use question_model::capability::Capability;
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssetId, BackendCapabilities, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, ProblemId, QuestionAttemptId, QuestionBackend, QuestionMetadata,
    QuestionSource, RunId, TenantId, UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

use super::*;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn choice(id: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
        body: vec![ContentBlock::Text {
            markdown: id.to_string(),
        }],
    }
}

fn draft_question(workspace: WorkspaceId, image: AssetId) -> DraftQuestionDefinition {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "residue".to_string(),
        ParameterSpec::Choice {
            options: vec!["alanine".to_string(), "glycine".to_string()],
        },
    );
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
        },
        prompt: vec![
            ContentBlock::Text {
                markdown: "In a peptide containing {{residue}}, which linkage is planar?"
                    .to_string(),
            },
            ContentBlock::Image {
                asset: AssetRef {
                    asset: image,
                    checksum: "bridge-fixture".to_string(),
                },
                description: "A peptide bond diagram.".to_string(),
            },
        ],
        response: ResponseDefinition::MultipleChoice {
            choices: vec![choice("ester"), choice("amide"), choice("ether")],
            selection: SelectionCardinality::ExactlyOne,
        },
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::ImmediateCorrectness,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: adapter_native::peptide_bond_geometry::GENERATOR_ID.to_string(),
                version: adapter_native::peptide_bond_geometry::GENERATOR_VERSION.to_string(),
            },
            parameters,
        },
        grading: GradingDefinition::AllOrNothing { points: 2.0 },
        metadata: QuestionMetadata {
            title: "Peptide bond geometry".to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    }
}

fn asset_record(
    reference: ProblemVersionRef,
    asset: AssetId,
    object: question_model::ObjectId,
) -> AssetDeliveryRecord {
    let key = ObjectKey::ProblemAsset {
        problem: reference.problem,
        version: reference.version,
        asset,
        object,
    };
    AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(asset),
        object: ObjectRecord {
            id: object,
            bucket: key.bucket(),
            key,
            sha256: Sha256Digest::compute(b"native bridge asset"),
            size_bytes: 19,
            media_type: "image/svg+xml".to_string(),
            category: objects::ObjectCategory::Asset,
            version: Some(reference.version),
            license: "CC BY 4.0".to_string(),
            provenance: "native bridge test".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        },
        scope: AssetDeliveryScope::Catalog { asset, reference },
    }
}

const FLAT_SOURCE: &str = r#"{
        "format":"pleFlatQuestion","version":1,"kind":"singleChoice",
        "title":"Favorite color","prompt":"What is my favorite color?",
        "choices":[
            {"id":"blue","text":"Blue","feedback":"Blue feedback."},
            {"id":"red","text":"Red","feedback":"Red feedback."}
        ],"correctChoice":"blue",
        "feedback":{"correct":"Correct feedback.","incorrect":"Incorrect feedback."},
        "points":10.0,"attemptPolicy":{"maxAttempts":null,"feedback":"immediateFull"},
        "timingPolicy":{"kind":"untimed"},"license":{"kind":"cc0"},"language":"en-US"
    }"#;

fn flat_source_record(
    tenant: TenantId,
    workspace: WorkspaceId,
    object: question_model::ObjectId,
    bytes: &[u8],
) -> ObjectRecord {
    let key = ObjectKey::WorkspaceQuestionSource {
        tenant,
        workspace,
        object,
    };
    ObjectRecord {
        id: object,
        bucket: key.bucket(),
        key,
        sha256: Sha256Digest::compute(bytes),
        size_bytes: bytes.len() as u64,
        media_type: adapter_native::flat_question::FLAT_QUESTION_MEDIA_TYPE.to_string(),
        category: objects::ObjectCategory::Source,
        version: None,
        license: "CC0-1.0".to_string(),
        provenance: "native backend flat fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1_000),
    }
}

async fn published_flat_fixture() -> (
    NativeBackend<MemoryStore>,
    Arc<MemoryStore>,
    TenantContext,
    ProblemVersionRef,
    QuestionDefinition,
    QuestionAttempt,
    ChoiceId,
    ChoiceId,
) {
    let (store, grader) = MemoryStore::with_flat_question_grader();
    let store = Arc::new(store);
    let tenant = TenantId::from_uuid(uuid(101));
    let context = TenantContext::from_authenticated_session(tenant);
    let owner = UserId::from_uuid(uuid(102));
    let workspace = WorkspaceId::from_uuid(uuid(103));
    let source = adapter_native::flat_question::FlatQuestionDocument::parse(FLAT_SOURCE.as_bytes())
        .expect("fixture source parses");
    let source_bytes = source
        .canonical_bytes()
        .expect("fixture source canonicalizes");
    let compiled = source.compile(workspace).expect("fixture source compiles");
    let (draft_question, private) = compiled.into_parts();
    let draft = DraftRecord {
        tenant,
        question: draft_question.clone(),
        revises: None,
        derived_from: None,
    };
    let staged_source = flat_source_record(
        tenant,
        workspace,
        question_model::ObjectId::from_uuid(uuid(104)),
        &source_bytes,
    );
    let staged = store
        .upsert_flat_question(
            context,
            owner,
            UpsertFlatQuestionCommand {
                expected_revision: None,
                draft: draft.clone(),
                source: staged_source.clone(),
                canonical_source_sha256: staged_source.sha256.to_string(),
                public_binding_sha256: private.public_binding_sha256().to_string(),
                grading: FlatQuestionGradingPayload::from_private(&private)
                    .expect("private material validates"),
            },
        )
        .await
        .expect("flat source stages");
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(105)),
        version: VersionId::from_uuid(uuid(106)),
    };
    let published_object = question_model::ObjectId::from_uuid(uuid(107));
    let key = ObjectKey::ProblemSource {
        problem: reference.problem,
        version: reference.version,
        object: published_object,
    };
    let artifact = PublishedSourceArtifact {
        reference,
        backend: QuestionBackend::Native,
        object: ObjectRecord {
            id: published_object,
            bucket: key.bucket(),
            key,
            sha256: staged_source.sha256,
            size_bytes: staged_source.size_bytes,
            media_type: staged_source.media_type.clone(),
            category: objects::ObjectCategory::Source,
            version: Some(reference.version),
            license: staged_source.license.clone(),
            provenance: "published native backend flat fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_001),
        },
    };
    store
        .publish_draft(
            context,
            owner,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: staged.workspace_revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: adapter_native::flat_question::FLAT_SINGLE_CHOICE_FAMILY.to_string(),
                },
                source_artifact: Some(artifact),
                qti_promotion: None,
                flat_question_promotion: Some(FlatQuestionPublicationPromotion {
                    source: staged,
                    import_origin: None,
                }),
                publisher: owner,
                scope: question_model::PublicationScope::Institution,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("flat source publishes");
    let question = QuestionDefinition::from_draft(
        draft_question,
        reference.problem,
        reference.version,
        QuestionSource::Native {
            family: adapter_native::flat_question::FLAT_SINGLE_CHOICE_FAMILY.to_string(),
        },
    );
    let backend = NativeBackend::with_flat_grader(
        Arc::new(adapter_native::NativeAdapter::new()),
        Arc::clone(&store),
        Arc::new(grader),
    );
    let issued = backend
        .issue(context, reference, &question, 108)
        .await
        .expect("flat question issues without private material");
    let attempt = QuestionAttempt {
        id: QuestionAttemptId::from_uuid(uuid(109)),
        tenant,
        run: RunId::from_uuid(uuid(110)),
        problem: reference.problem,
        question_version: reference.version,
        assignment_position: 0,
        seed: 108,
        parameter_hash: issued.parameter_hash,
        response: None,
        status: question_model::AttemptStatus::InProgress,
        result: None,
        timer: question_model::AttemptTimerRecord {
            issued_at: ActivityTimestamp::from_unix_millis(1_000),
            deadline: None,
            submitted_at: None,
        },
        provenance: issued.provenance,
    };
    (
        backend,
        store,
        context,
        reference,
        question,
        attempt,
        ChoiceId::new("blue"),
        ChoiceId::new("red"),
    )
}

#[tokio::test]
async fn native_bridge_reproduces_only_with_exact_memory_catalog_assets() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(uuid(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let problem = ProblemId::from_uuid(uuid(2));
    let version = VersionId::from_uuid(uuid(3));
    let workspace = WorkspaceId::from_uuid(uuid(4));
    let asset = AssetId::from_uuid(uuid(5));
    let object = question_model::ObjectId::from_uuid(uuid(6));
    let publisher = UserId::from_uuid(uuid(7));
    let draft = DraftRecord {
        tenant,
        question: draft_question(workspace, asset),
        revises: None,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("draft saves before publication");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: ProblemVersionRef { problem, version },
                published_source: QuestionSource::Native {
                    family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: question_model::PublicationScope::Institution,
                capabilities: BackendCapabilities::from_iter([
                    Capability::AlgorithmicGeneration,
                    Capability::ClientRendering,
                    Capability::ServerGrading,
                ]),
            },
        )
        .await
        .expect("native question publishes");
    let reference = ProblemVersionRef { problem, version };
    store
        .register_asset_delivery(context, asset_record(reference, asset, object))
        .await
        .expect("exact version asset registers");

    let backend = NativeBackend::new(Arc::new(adapter_native::NativeAdapter::new()), store);
    let published = QuestionDefinition::from_draft(
        draft_question(workspace, asset),
        problem,
        version,
        QuestionSource::Native {
            family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
        },
    );
    let issued = backend
        .issue(context, reference, &published, 37)
        .await
        .expect("native issue resolves only exact catalog assets");
    assert_eq!(issued.provenance.asset_objects, vec![object]);
    let attempt = QuestionAttempt {
        id: QuestionAttemptId::from_uuid(uuid(8)),
        tenant,
        run: RunId::from_uuid(uuid(9)),
        problem,
        question_version: version,
        assignment_position: 0,
        seed: 37,
        parameter_hash: issued.parameter_hash,
        response: None,
        status: question_model::AttemptStatus::InProgress,
        result: None,
        timer: question_model::AttemptTimerRecord {
            issued_at: ActivityTimestamp::from_unix_millis(1_000),
            deadline: None,
            submitted_at: None,
        },
        provenance: issued.provenance,
    };
    let envelope = backend
        .reproduce(context, reference, &published, &attempt)
        .await
        .expect("stored native attempt reproduces through memory asset resolver");
    let body = serde_json::to_string(&envelope).expect("envelope serializes");
    assert!(!body.contains("{{residue}}"));
    assert!(!body.contains("correct"));
    assert!(body.contains("alanine") || body.contains("glycine"));
    let outcome = backend
        .grade(
            context,
            reference,
            &published,
            &attempt,
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("amide")],
            },
        )
        .await
        .expect("native server grading remains behind the bridge");
    assert!(matches!(outcome, GradeOutcome::Graded(result) if result.correct));

    let wrong_reference = ProblemVersionRef {
        problem,
        version: VersionId::from_uuid(uuid(10)),
    };
    assert!(matches!(
        backend
            .reproduce(context, wrong_reference, &published, &attempt)
            .await,
        Err(RunBackendError::Invalid(_))
    ));

    let foreign_context = TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(11)));
    assert!(matches!(
        backend
            .reproduce(foreign_context, reference, &published, &attempt)
            .await,
        Err(RunBackendError::Invalid(_))
    ));

    let mut tampered = attempt.clone();
    tampered.provenance.asset_objects = vec![question_model::ObjectId::from_uuid(uuid(12))];
    assert!(matches!(
        backend
            .reproduce(context, reference, &published, &tampered)
            .await,
        Err(RunBackendError::Invalid(_))
    ));
}

#[tokio::test]
async fn flat_question_grades_from_isolated_memory_grader_and_keeps_issue_answer_free() {
    let (backend, _store, context, reference, question, attempt, correct, incorrect) =
        published_flat_fixture().await;
    let envelope = backend
        .reproduce(context, reference, &question, &attempt)
        .await
        .expect("flat question reproduces through its public adapter path");
    let serialized = serde_json::to_string(&envelope).expect("public envelope serializes");
    assert!(!serialized.contains("correctChoice"));
    assert!(!serialized.contains("Correct feedback."));

    let right = backend
        .grade(
            context,
            reference,
            &question,
            &attempt,
            &StudentResponse::MultipleChoice {
                selected: vec![correct],
            },
        )
        .await
        .expect("private grader grades the correct choice");
    assert!(matches!(right, GradeOutcome::Graded(result) if result.correct));
    let wrong_response = StudentResponse::MultipleChoice {
        selected: vec![incorrect],
    };
    let wrong = backend
        .grade(context, reference, &question, &attempt, &wrong_response)
        .await
        .expect("private grader grades the incorrect choice");
    assert!(matches!(wrong, GradeOutcome::Graded(result) if !result.correct));

    let disposition = backend
        .submit(RunSubmission {
            context,
            actor: UserId::from_uuid(uuid(111)),
            idempotency_key: learning_data_access::SubmissionIdempotencyKey::parse("flat-test")
                .expect("fixture key is valid"),
            reference,
            question: &question,
            attempt: &attempt,
            response: &wrong_response,
        })
        .await
        .expect("flat submission prepares trusted feedback");
    let SubmissionDisposition::Grade(receipt) = disposition else {
        panic!("flat question should return a numerical receipt");
    };
    assert!(!receipt.result.correct);
    assert!(
        receipt.feedback.hint.is_some(),
        "trusted receipt keeps teaching feedback for the run policy projection"
    );
    assert!(receipt.feedback.correct_response.is_some());
}

#[tokio::test]
async fn flat_question_without_injected_grader_or_with_foreign_tenant_fails_closed() {
    let (backend, _store, context, reference, question, attempt, _correct, incorrect) =
        published_flat_fixture().await;
    let response = StudentResponse::MultipleChoice {
        selected: vec![incorrect],
    };
    let foreign = TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(112)));
    assert!(matches!(
        backend
            .grade(foreign, reference, &question, &attempt, &response)
            .await,
        Err(RunBackendError::Unavailable(_))
    ));

    let no_grader = NativeBackend::new(
        Arc::new(adapter_native::NativeAdapter::new()),
        Arc::new(MemoryStore::default()),
    );
    assert!(matches!(
        no_grader
            .grade(context, reference, &question, &attempt, &response)
            .await,
        Err(RunBackendError::Invalid(_) | RunBackendError::Unavailable(_))
    ));
}

#[derive(Clone)]
struct FixedFlatGrader(FlatQuestionGradingPayload);

#[async_trait]
impl learning_data_access::FlatQuestionGradingStore for FixedFlatGrader {
    async fn flat_question_published_grading(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
    ) -> Result<Option<FlatQuestionGradingPayload>, StoreError> {
        Ok(Some(self.0.clone()))
    }
}

#[tokio::test]
async fn flat_question_rejects_private_material_bound_to_another_public_model() {
    let (_backend, store, context, reference, question, attempt, _correct, incorrect) =
        published_flat_fixture().await;
    let mismatched_source = FLAT_SOURCE.replace("Favorite color", "Different favorite color");
    let document =
        adapter_native::flat_question::FlatQuestionDocument::parse(mismatched_source.as_bytes())
            .expect("fixture source parses");
    let compiled = document
        .compile(WorkspaceId::from_uuid(uuid(113)))
        .expect("fixture source compiles");
    let (_, private) = compiled.into_parts();
    let mismatched = FlatQuestionGradingPayload::from_private(&private)
        .expect("different private material validates");
    let backend = NativeBackend::with_flat_grader(
        Arc::new(adapter_native::NativeAdapter::new()),
        store,
        Arc::new(FixedFlatGrader(mismatched)),
    );
    assert!(matches!(
        backend
            .grade(
                context,
                reference,
                &question,
                &attempt,
                &StudentResponse::MultipleChoice {
                    selected: vec![incorrect],
                },
            )
            .await,
        Err(RunBackendError::Invalid(_))
    ));
}
