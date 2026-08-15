use super::*;
use learning_data_access::{
    FlatQuestionGradingPayload, FlatQuestionGradingStore, FlatQuestionPublicationPromotion,
    FlatQuestionStore,
};

const SECRET: &str = "flat-answer-token-must-not-reach-public-data";
const FLAT_QUESTION_MEDIA_TYPE: &str = "application/vnd.peptidyle.flat-question+json";

fn compiled_flat(
    workspace: WorkspaceId,
    title: &str,
) -> (
    question_model::DraftQuestionDefinition,
    grading::flat_question::FlatQuestionPrivate,
    Vec<u8>,
) {
    let source = format!(
        r#"{{"format":"pleFlatQuestion","version":2,"title":"{title}","prompt":"What is my favorite color?","response":{{"kind":"singleChoice","choices":[{{"id":"blue","text":"Blue","feedback":"Blue is correct."}},{{"id":"red","text":"Red","feedback":"Red is not correct."}}],"correctChoice":"blue"}},"feedback":{{"correct":"Correct.","incorrect":"Try again."}},"points":1.0,"attemptPolicy":{{"maxAttempts":null,"feedback":"immediateFull"}},"timingPolicy":{{"kind":"untimed"}},"license":{{"kind":"cc0"}},"language":"en-US"}}"#
    );
    let document = adapter_native::flat_question::FlatQuestionDocument::parse(source.as_bytes())
        .expect("flat conformance fixture should parse");
    let canonical = document
        .canonical_bytes()
        .expect("flat conformance fixture should canonicalize");
    let (draft, private) = document
        .compile(workspace)
        .expect("flat conformance fixture should compile")
        .into_parts();
    (draft, private, canonical)
}

fn flat_source_record(
    tenant: TenantId,
    workspace: WorkspaceId,
    object: ObjectId,
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
        sha256: Sha256Digest::compute(bytes),
        size_bytes: u64::try_from(bytes.len()).expect("flat fixture size fits"),
        media_type: FLAT_QUESTION_MEDIA_TYPE.to_string(),
        category: ObjectCategory::Source,
        version: None,
        license: "CC BY-SA 4.0".to_string(),
        provenance: "flat-question conformance fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1),
        key,
    }
}

fn published_flat_source_artifact(
    reference: ProblemVersionRef,
    staged: &ObjectRecord,
    object: ObjectId,
) -> PublishedSourceArtifact {
    let key = ObjectKey::ProblemSource {
        problem: reference.problem,
        version: reference.version,
        object,
    };
    PublishedSourceArtifact {
        reference,
        backend: QuestionBackend::Native,
        object: ObjectRecord {
            id: object,
            bucket: key.bucket(),
            sha256: staged.sha256,
            size_bytes: staged.size_bytes,
            media_type: staged.media_type.clone(),
            category: ObjectCategory::Source,
            version: Some(reference.version),
            license: staged.license.clone(),
            provenance: "published flat-question conformance fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(2),
            key,
        },
    }
}

fn flat_grading(
    private: &grading::flat_question::FlatQuestionPrivate,
) -> FlatQuestionGradingPayload {
    FlatQuestionGradingPayload::from_private(private)
        .expect("compiled private flat fixture is valid")
}

fn flat_command(
    draft: DraftRecord,
    expected_revision: Option<learning_data_access::WorkspaceDraftRevision>,
    source: ObjectRecord,
    grading: FlatQuestionGradingPayload,
) -> learning_data_access::UpsertFlatQuestionCommand {
    let canonical_source_sha256 = source.sha256.to_string();
    let public_binding_sha256 = grading.public_binding_sha256().to_string();
    learning_data_access::UpsertFlatQuestionCommand {
        expected_revision,
        draft,
        source,
        canonical_source_sha256,
        public_binding_sha256,
        grading,
    }
}

async fn exercise_flat_question_store(
    store: &MemoryStore,
    grader: &learning_data_access::in_memory::MemoryFlatQuestionGraderStore,
) {
    let tenant = TenantId::from_uuid(uuid(80_001));
    let foreign_tenant = TenantId::from_uuid(uuid(80_002));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let owner = UserId::from_uuid(uuid(80_003));
    let unrelated = UserId::from_uuid(uuid(80_004));
    let workspace = WorkspaceId::from_uuid(uuid(80_005));
    let (question, private_model, source_bytes) = compiled_flat(workspace, "Favorite color");
    let draft = DraftRecord {
        tenant,
        question,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, owner, None, draft.clone())
        .await
        .expect("owner stages flat draft");
    let source = flat_source_record(
        tenant,
        workspace,
        ObjectId::from_uuid(uuid(80_006)),
        &source_bytes,
    );
    let public_sha256 = private_model.public_binding_sha256().to_string();
    let staged = store
        .upsert_flat_question(
            context,
            owner,
            flat_command(
                draft.clone(),
                Some(saved.revision),
                source.clone(),
                flat_grading(&private_model),
            ),
        )
        .await
        .expect("owner stages source bound to exact draft revision");
    assert!(
        staged.workspace_revision.value() > saved.revision.value(),
        "flat-source staging advances the draft CAS revision"
    );
    assert_eq!(staged.source_record, source);

    assert_eq!(
        store
            .flat_question_source(context, unrelated, workspace)
            .await
            .expect("unrelated same-tenant lookup is non-enumerating"),
        None
    );
    assert_eq!(
        store
            .flat_question_source(foreign_context, owner, workspace)
            .await
            .expect("foreign-tenant lookup is non-enumerating"),
        None
    );

    let stale_source = flat_source_record(
        tenant,
        workspace,
        ObjectId::from_uuid(uuid(80_007)),
        b"stale flat source",
    );
    assert_eq!(
        store
            .upsert_flat_question(
                context,
                owner,
                flat_command(
                    draft.clone(),
                    Some(saved.revision),
                    stale_source,
                    flat_grading(&private_model),
                ),
            )
            .await,
        Err(StoreError::Conflict),
        "a stale replacement cannot overwrite the current source binding"
    );
    assert_eq!(
        store
            .flat_question_source(context, owner, workspace)
            .await
            .expect("owner source lookup"),
        Some(staged.clone()),
        "stale replacement preserves the staged binding"
    );

    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(80_008)),
        version: VersionId::from_uuid(uuid(80_009)),
    };
    let artifact =
        published_flat_source_artifact(reference, &source, ObjectId::from_uuid(uuid(80_010)));
    let promotion = FlatQuestionPublicationPromotion {
        source: staged.clone(),
        import_origin: None,
        published_question: draft.question.clone(),
        assets: Vec::new(),
    };
    let command = |promotion: FlatQuestionPublicationPromotion,
                   source_artifact: PublishedSourceArtifact,
                   expected_revision| PublishDraftCommand {
        expected_draft: draft.clone(),
        expected_revision,
        publication: reference,
        published_source: QuestionSource::Native {
            family: "flat_single_choice_v2".to_string(),
        },
        source_artifact: Some(source_artifact),
        qti_promotion: None,
        flat_question_promotion: Some(promotion),
        publisher: owner,
        scope: PublicationScope::Public,
        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
    };
    let mut bad_artifact = artifact.clone();
    bad_artifact.object.sha256 = Sha256Digest::compute(b"wrong immutable source");
    assert_eq!(
        store
            .publish_draft(
                context,
                owner,
                command(promotion.clone(), bad_artifact, staged.workspace_revision),
            )
            .await,
        Err(StoreError::Conflict),
        "a copied source artifact must exactly match staged source metadata"
    );
    assert_eq!(
        store
            .publish_draft(
                context,
                owner,
                command(promotion.clone(), artifact.clone(), saved.revision),
            )
            .await,
        Err(StoreError::Conflict),
        "a stale draft revision cannot consume the staged flat source"
    );
    assert_eq!(
        store
            .get_catalog_problem(context, reference)
            .await
            .expect("failed publication lookup"),
        None
    );
    assert_eq!(
        store
            .flat_question_source(context, owner, workspace)
            .await
            .expect("failed publication keeps staging"),
        Some(staged.clone())
    );
    assert!(
        grader
            .flat_question_published_grading(context, reference)
            .await
            .expect("failed publication grading lookup")
            .is_none(),
        "failed publication must not expose private grading material"
    );

    let published = store
        .publish_draft(
            context,
            owner,
            command(promotion, artifact.clone(), staged.workspace_revision),
        )
        .await
        .expect("matching source and private binding publish atomically");
    assert_eq!(published.problem, reference.problem);
    let public_json = serde_json::to_string(&published).expect("public record serializes");
    assert!(
        !public_json.contains(SECRET),
        "browser-safe published data must not contain the answer token"
    );
    let catalog_json = serde_json::to_string(
        &store
            .get_catalog_problem(context, reference)
            .await
            .expect("published catalog lookup")
            .expect("published catalog record"),
    )
    .expect("catalog record serializes");
    assert!(!catalog_json.contains(SECRET));
    assert_eq!(
        store
            .catalog_source_artifact(context, reference)
            .await
            .expect("published source metadata lookup"),
        Some(artifact),
        "publication retains only immutable source metadata"
    );
    assert_eq!(
        store
            .get_draft(context, owner, workspace)
            .await
            .expect("published workspace lookup"),
        None
    );
    assert_eq!(
        store
            .flat_question_source(context, owner, workspace)
            .await
            .expect("published source staging lookup"),
        None
    );
    let private = grader
        .flat_question_published_grading(context, reference)
        .await
        .expect("injected grader reads private payload")
        .expect("published flat grading exists");
    assert_eq!(
        private
            .decode_private()
            .expect("grader payload must decode through the opaque boundary")
            .public_binding_sha256(),
        public_sha256
    );
    assert!(
        grader
            .flat_question_published_grading(foreign_context, reference)
            .await
            .expect("public catalog grading lookup")
            .is_some(),
        "public catalog content remains gradeable for another tenant"
    );
}

#[tokio::test]
async fn memory_flat_question_staging_and_publication_conform() {
    let (store, grader) = MemoryStore::with_flat_question_grader();
    exercise_flat_question_store(&store, &grader).await;
}
