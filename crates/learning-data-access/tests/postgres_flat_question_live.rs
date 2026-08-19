#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for flat-question publication and private grading.

use learning_data_access::postgres::{
    PostgresGraderStore, PostgresStore, lazy_pool, verify_application_schema,
};
use learning_data_access::{
    CatalogSourceStore, CatalogStore, DraftRecord, FlatQuestionGradingPayload,
    FlatQuestionGradingStore, FlatQuestionPublicationPromotion, FlatQuestionStore,
    PublishDraftCommand, PublishedSourceArtifact, QuestionIdCodec, Store, StoreError,
    TenantContext, UpsertFlatQuestionCommand,
};
use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::response::{ChoiceId, MatchPair, StudentResponse};
use question_model::{
    ActivityTimestamp, BackendCapabilities, Capability, ObjectId, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionBackend, QuestionSource, TenantId, UserId, VersionId, WorkspaceId,
};
use sqlx::Row;
use uuid::Uuid;

const FLAT_MEDIA_TYPE: &str = "application/vnd.peptidyle.flat-question+json";
const PRIVATE_FEEDBACK_MARKER: &str = "private feedback must remain grader-only";
const FLAT_SOURCE: &str = r#"{"format":"pleFlatQuestion","version":2,"title":"Favorite color","prompt":"What is my favorite color?","response":{"kind":"singleChoice","choices":[{"id":"blue","text":"Blue","feedback":"Blue choice feedback"},{"id":"red","text":"Red","feedback":"Red choice feedback"}],"correctChoice":"blue"},"feedback":{"correct":"Correct feedback","incorrect":"private feedback must remain grader-only"},"points":1.0,"attemptPolicy":{"maxAttempts":null,"feedback":"immediateFull"},"timingPolicy":{"kind":"untimed"},"license":{"kind":"cc0"},"language":"en-US"}"#;
const FLAT_MATCHING_V2_SOURCE: &str = r#"{"format":"pleFlatQuestion","version":2,"title":"Match inheritance terms","prompt":"Match each term to its description.","response":{"kind":"matching","prompts":[{"id":"p1","text":"Two different alleles"},{"id":"p2","text":"Two identical alleles"}],"choices":[{"id":"c1","text":"Heterozygous"},{"id":"c2","text":"Homozygous"}],"matches":[{"prompt":"p1","choice":"c1"},{"prompt":"p2","choice":"c2"}]},"feedback":{"correct":"Correct.","incorrect":"Review the allele pairs."},"points":2.0,"attemptPolicy":{"maxAttempts":null,"feedback":"immediateFull"},"timingPolicy":{"kind":"untimed"},"license":{"kind":"cc0"},"language":"en-US"}"#;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn flat_source(tenant: TenantId, workspace: WorkspaceId, bytes: &[u8]) -> ObjectRecord {
    let object = ObjectId::from_uuid(id());
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
        size_bytes: u64::try_from(bytes.len()).expect("fixture source size fits"),
        media_type: FLAT_MEDIA_TYPE.to_string(),
        category: ObjectCategory::Source,
        version: None,
        license: "CC BY-SA 4.0".to_string(),
        provenance: "disposable PostgreSQL flat-question fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1),
    }
}

fn published_artifact(
    reference: ProblemVersionRef,
    source: &ObjectRecord,
) -> PublishedSourceArtifact {
    let object = ObjectId::from_uuid(id());
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
            key,
            sha256: source.sha256,
            size_bytes: source.size_bytes,
            media_type: source.media_type.clone(),
            category: ObjectCategory::Source,
            version: Some(reference.version),
            license: source.license.clone(),
            provenance: "published disposable flat-question fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(2),
        },
    }
}

fn publication_command(
    draft: DraftRecord,
    revision: learning_data_access::WorkspaceDraftRevision,
    reference: ProblemVersionRef,
    source: learning_data_access::WorkspaceFlatQuestionSource,
    artifact: PublishedSourceArtifact,
    owner: UserId,
) -> PublishDraftCommand {
    let family = match &draft.question.source {
        question_model::DraftQuestionSource::Native { family } => family.clone(),
        _ => panic!("flat publication fixture must use a native family"),
    };
    let published_question = draft.question.clone();
    PublishDraftCommand {
        expected_draft: draft,
        expected_revision: revision,
        publication: reference,
        published_source: QuestionSource::Native { family },
        source_artifact: Some(artifact),
        qti_promotion: None,
        flat_question_promotion: Some(FlatQuestionPublicationPromotion {
            source,
            import_origin: None,
            published_question,
            assets: Vec::new(),
        }),
        publisher: owner,
        scope: PublicationScope::Institution,
        byline: question_model::PublicByline::new(vec![
            question_model::PublicAuthorName::new("PLE fixture".to_string())
                .expect("valid test byline"),
        ])
        .expect("valid test byline"),
        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
    }
}

async fn assert_answer_key_denied(pool: &sqlx::PgPool, role: &str, tenant: TenantId) {
    let mut transaction = pool.begin().await.expect("start direct role transaction");
    match role {
        "ple_app" => {
            sqlx::query("SET LOCAL ROLE ple_app")
                .execute(&mut *transaction)
                .await
        }
        "ple_student" => {
            sqlx::query("SET LOCAL ROLE ple_student")
                .execute(&mut *transaction)
                .await
        }
        _ => panic!("unsupported restricted role fixture"),
    }
    .expect("assume restricted direct role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.as_uuid().to_string())
        .execute(&mut *transaction)
        .await
        .expect("set tenant for restricted direct role");
    let error = sqlx::query("SELECT key_payload FROM public.answer_key")
        .fetch_all(&mut *transaction)
        .await
        .expect_err("restricted role must not directly select answer keys");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|value| value.code())
            .as_deref(),
        Some("42501")
    );
    transaction
        .rollback()
        .await
        .expect("rollback direct role transaction");
}

async fn seed_non_flat_answer_key(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    backend: &str,
    family: Option<&str>,
) -> ProblemVersionRef {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let question_id = QuestionIdCodec::from_server_secret([0x42; 32])
        .issue()
        .expect("live fixture Question ID issues");
    let source = match family {
        Some(family) => serde_json::json!({ "backend": "native", "family": family }),
        None => serde_json::json!({ "backend": backend }),
    };
    sqlx::query(
        "INSERT INTO public.problem \
         (problem_id, question_id, owner_tenant_id, owner_user_id, visibility, license) \
         VALUES ($1, $2, $3, $4, 'public', 'CC0-1.0')",
    )
    .bind(reference.problem.as_uuid())
    .bind(question_id.compact())
    .bind(tenant.as_uuid())
    .bind(Uuid::from_u128(1))
    .execute(pool)
    .await
    .expect("seed non-flat problem");
    sqlx::query(
        "INSERT INTO public.problem_version \
         (problem_id, version_id, content_sha256, workspace_id, title, backend, publication_scope, author_ids, public_byline) \
         VALUES ($1, $2, $3, $4, 'non-flat grading filter fixture', $5, 'public', '[\"fixture\"]'::jsonb, ARRAY['Fixture author'])",
    )
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .bind("a".repeat(64))
    .bind(Uuid::from_u128(2))
    .bind(backend)
    .execute(pool)
    .await
    .expect("seed non-flat version");
    sqlx::query(
        "INSERT INTO public.problem_version_payload (problem_id, version_id, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .bind(serde_json::json!({ "question": { "source": source } }))
    .bind("b".repeat(64))
    .execute(pool)
    .await
    .expect("seed non-flat public payload");
    sqlx::query(
        "INSERT INTO public.answer_key (problem_id, version_id, key_payload, key_sha256) \
         VALUES ($1, $2, '{}'::jsonb, $3)",
    )
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .bind("c".repeat(64))
    .execute(pool)
    .await
    .expect("seed non-flat answer key");
    reference
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_flat_question_publication_preserves_private_grading_boundary() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let grader_url = std::env::var("PLE_TEST_GRADER_DATABASE_URL")
        .expect("PLE_TEST_GRADER_DATABASE_URL must name the disposable grader connection");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x42; 32]);
    let grader = PostgresGraderStore::connect_local_development(&grader_url)
        .await
        .expect("dedicated grader credentials are accepted");

    let tenant_a = TenantId::from_uuid(id());
    let tenant_b = TenantId::from_uuid(id());
    let context_a = TenantContext::from_authenticated_session(tenant_a);
    let context_b = TenantContext::from_authenticated_session(tenant_b);
    let owner = UserId::from_uuid(id());
    let unrelated = UserId::from_uuid(id());
    let workspace = WorkspaceId::from_uuid(id());
    let document =
        adapter_native::flat_question::FlatQuestionDocument::parse(FLAT_SOURCE.as_bytes())
            .expect("fixture flat source parses");
    let source_bytes = document
        .canonical_bytes()
        .expect("fixture flat source canonicalizes");
    let compiled = document
        .compile(workspace)
        .expect("fixture flat source compiles");
    let (question, private_before_storage) = compiled.into_parts();
    let draft = DraftRecord {
        tenant: tenant_a,
        question,
        derived_from: None,
    };
    let source_record = flat_source(tenant_a, workspace, &source_bytes);
    let grading_before_storage = FlatQuestionGradingPayload::from_private(&private_before_storage)
        .expect("compiled private material becomes a grading payload");
    let private_bytes = private_before_storage
        .canonical_bytes()
        .expect("compiled private material canonicalizes");
    let public_sha256 = private_before_storage.public_binding_sha256().to_string();
    let staged = store
        .upsert_flat_question(
            context_a,
            owner,
            UpsertFlatQuestionCommand {
                expected_revision: None,
                draft: draft.clone(),
                source: source_record.clone(),
                canonical_source_sha256: source_record.sha256.to_string(),
                public_binding_sha256: public_sha256.clone(),
                grading: grading_before_storage.clone(),
            },
        )
        .await
        .expect("first flat save atomically creates draft and source staging");
    assert_eq!(staged.source_record, source_record);
    assert_eq!(
        store
            .flat_question_source(context_a, unrelated, workspace)
            .await
            .expect("unrelated lookup is non-enumerating"),
        None
    );
    assert_eq!(
        store
            .flat_question_source(context_b, owner, workspace)
            .await
            .expect("foreign lookup is non-enumerating"),
        None
    );
    assert_eq!(
        store
            .upsert_flat_question(
                context_a,
                owner,
                UpsertFlatQuestionCommand {
                    expected_revision: None,
                    draft: draft.clone(),
                    source: flat_source(tenant_a, workspace, b"stale"),
                    canonical_source_sha256: Sha256Digest::compute(b"stale").to_string(),
                    public_binding_sha256: public_sha256.clone(),
                    grading: grading_before_storage.clone(),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "stale source save leaves the staged source unchanged"
    );

    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let artifact = published_artifact(reference, &source_record);
    let mut mismatched_source = staged.clone();
    mismatched_source.public_binding_sha256 =
        Sha256Digest::compute(b"wrong public model").to_string();
    assert_eq!(
        store
            .publish_draft(
                context_a,
                owner,
                publication_command(
                    draft.clone(),
                    staged.workspace_revision,
                    reference,
                    mismatched_source,
                    artifact.clone(),
                    owner,
                ),
            )
            .await,
        Err(StoreError::Conflict),
        "mismatched private binding leaves draft and staging intact"
    );
    assert!(
        store
            .get_catalog_problem(context_a, reference)
            .await
            .expect("failed publication lookup")
            .is_none()
    );
    assert_eq!(
        store
            .flat_question_source(context_a, owner, workspace)
            .await
            .expect("failed publication retains staging"),
        Some(staged.clone())
    );

    let published = store
        .publish_draft(
            context_a,
            owner,
            publication_command(
                draft.clone(),
                staged.workspace_revision,
                reference,
                staged.clone(),
                artifact.clone(),
                owner,
            ),
        )
        .await
        .expect("matching flat promotion uses the security-definer capability");
    let public_json = serde_json::to_string(&published).expect("public record serializes");
    assert!(!public_json.contains(PRIVATE_FEEDBACK_MARKER));
    assert_eq!(
        store
            .catalog_source_artifact(context_a, reference)
            .await
            .expect("published source artifact lookup"),
        Some(artifact)
    );
    assert!(
        store
            .get_draft(context_a, owner, workspace)
            .await
            .expect("draft lookup after promotion")
            .is_none()
    );
    assert!(
        store
            .flat_question_source(context_a, owner, workspace)
            .await
            .expect("source lookup after promotion")
            .is_none()
    );

    assert_answer_key_denied(&pool, "ple_app", tenant_a).await;
    assert_answer_key_denied(&pool, "ple_student", tenant_a).await;
    let private = grader
        .flat_question_published_grading(context_a, reference)
        .await
        .expect("grader reads published flat payload")
        .expect("tenant-A institution grant exposes flat payload to grader");
    assert_eq!(
        private.sha256(),
        Sha256Digest::compute(&private_bytes),
        "grader payload preserves the canonical private-byte checksum"
    );
    assert_eq!(private.sha256(), grading_before_storage.sha256());
    assert_eq!(private.public_binding_sha256(), public_sha256);
    let decoded = private
        .decode_private()
        .expect("grader material decodes into the private flat model");
    assert_eq!(
        decoded
            .canonical_bytes()
            .expect("decoded private material canonicalizes"),
        private_bytes
    );
    assert_eq!(decoded.public_binding_sha256(), public_sha256);
    let correct = decoded
        .evaluate(
            &published.question,
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("blue")],
            },
        )
        .expect("decoded private material grades the correct choice");
    let grading::GradeOutcome::Graded(correct_result) = correct.outcome else {
        panic!("flat correct response must produce a numerical result");
    };
    assert!(correct_result.correct);
    assert!(correct.feedback.hint.is_some());
    assert!(correct.feedback.correct_response.is_some());
    let incorrect = decoded
        .evaluate(
            &published.question,
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("red")],
            },
        )
        .expect("decoded private material grades the incorrect choice");
    let grading::GradeOutcome::Graded(incorrect_result) = incorrect.outcome else {
        panic!("flat incorrect response must produce a numerical result");
    };
    assert!(!incorrect_result.correct);
    assert!(incorrect.feedback.hint.is_some());
    assert!(incorrect.feedback.correct_response.is_some());
    assert!(
        grader
            .flat_question_published_grading(context_b, reference)
            .await
            .expect("foreign grader lookup")
            .is_none(),
        "tenant-B cannot retrieve tenant-A institution-only private material"
    );

    let matching_workspace = WorkspaceId::from_uuid(id());
    let matching_document = adapter_native::flat_question::FlatQuestionDocument::parse(
        FLAT_MATCHING_V2_SOURCE.as_bytes(),
    )
    .expect("v2 matching source parses");
    let matching_source_bytes = matching_document
        .canonical_bytes()
        .expect("v2 matching source canonicalizes");
    let (matching_question, matching_private) = matching_document
        .compile(matching_workspace)
        .expect("v2 matching source compiles")
        .into_parts();
    let matching_draft = DraftRecord {
        tenant: tenant_a,
        question: matching_question,
        derived_from: None,
    };
    let matching_source = flat_source(tenant_a, matching_workspace, &matching_source_bytes);
    let matching_grading = FlatQuestionGradingPayload::from_private(&matching_private)
        .expect("v2 matching private material persists");
    let matching_staged = store
        .upsert_flat_question(
            context_a,
            owner,
            UpsertFlatQuestionCommand {
                expected_revision: None,
                draft: matching_draft.clone(),
                source: matching_source.clone(),
                canonical_source_sha256: matching_source.sha256.to_string(),
                public_binding_sha256: matching_grading.public_binding_sha256().to_string(),
                grading: matching_grading,
            },
        )
        .await
        .expect("PostgreSQL stages v2 matching source and grading together");
    let matching_reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let matching_published = store
        .publish_draft(
            context_a,
            owner,
            publication_command(
                matching_draft,
                matching_staged.workspace_revision,
                matching_reference,
                matching_staged,
                published_artifact(matching_reference, &matching_source),
                owner,
            ),
        )
        .await
        .expect("PostgreSQL publishes v2 matching through the grader capability");
    let matching_private = grader
        .flat_question_published_grading(context_a, matching_reference)
        .await
        .expect("v2 matching grader lookup")
        .expect("v2 matching private material is available to the grader")
        .decode_private()
        .expect("v2 matching private material decodes");
    let matching_outcome = matching_private
        .evaluate(
            &matching_published.question,
            &StudentResponse::Matching {
                matches: vec![
                    MatchPair {
                        prompt: ChoiceId::new("p1"),
                        choice: ChoiceId::new("c1"),
                    },
                    MatchPair {
                        prompt: ChoiceId::new("p2"),
                        choice: ChoiceId::new("c2"),
                    },
                ],
            },
        )
        .expect("v2 matching grades from protected PostgreSQL material");
    let grading::GradeOutcome::Graded(matching_result) = matching_outcome.outcome else {
        panic!("v2 matching must produce a numerical result");
    };
    assert!(matching_result.correct);

    let other_native =
        seed_non_flat_answer_key(&pool, tenant_a, "native", Some("other_native_v1")).await;
    let qti = seed_non_flat_answer_key(&pool, tenant_a, "qti", None).await;
    for non_flat in [other_native, qti] {
        assert!(
            grader
                .flat_question_published_grading(context_a, non_flat)
                .await
                .expect("filtered grading lookup")
                .is_none(),
            "the flat grading capability never returns another backend or family"
        );
    }

    let answer_count: i64 =
        sqlx::query("SELECT count(*) AS count FROM public.answer_key WHERE problem_id = $1")
            .bind(reference.problem.as_uuid())
            .fetch_one(&pool)
            .await
            .expect("migration owner verifies promotion output")
            .try_get("count")
            .expect("answer count is integer");
    assert_eq!(
        answer_count, 1,
        "matching promotion inserted one private answer key"
    );
}
