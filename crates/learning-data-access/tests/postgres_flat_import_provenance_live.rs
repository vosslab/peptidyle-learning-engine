#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for atomic QTI-profile-to-flat provenance.

#[path = "postgres_flat_import_provenance_live/success.rs"]
mod success;

use learning_data_access::postgres::{
    PostgresGraderStore, PostgresStore, lazy_pool, verify_application_schema,
};
use learning_data_access::{
    CatalogStore, CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    DraftRecord, EnqueueJob, FlatImportChoiceMapPayload, FlatImportConversionVersion,
    FlatImportIntegrityDigests, FlatImportProvenanceStore, FlatImportPublicationPromotion,
    FlatQuestionGradingPayload, FlatQuestionGradingStore, FlatQuestionPublicationPromotion,
    FlatQuestionStore, JobClaimFilter, JobKind, JobLeaseDuration, JobPayload, JobStore,
    PersistedFlatImportProfile, PublishDraftCommand, PublishedSourceArtifact,
    QtiImportGradingPayload, QtiImportItem, QtiImportItemRegistration, QtiImportItemResult,
    QtiImportItemStatus, QtiImportProfileSummary, QtiImportRef, QtiImportRegistry, QtiImportStore,
    QtiProfileFlatConversionCommand, QtiProfileImportEvidence, QtiUnsupportedFeature, Store,
    StoreError, TenantContext, UpsertFlatQuestionCommand, WorkspaceDraftRevision,
    WorkspaceFlatImportOrigin,
};
use objects::{
    ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest, published_import_archive_object_id,
};
use question_model::response::ChoiceId;
use question_model::{
    ActivityTimestamp, BackendCapabilities, Capability, ObjectId, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionBackend, QuestionSource, TenantId, UserId, VersionId, WorkspaceId,
    WorkspaceImportId,
};
use sqlx::Row;
use uuid::Uuid;

const ITEM_ID: &str = "accepted-profile-item";
const FLAT_MEDIA_TYPE: &str = "application/vnd.peptidyle.flat-question+json";
const IMPORTED_FLAT_SOURCE: &str = r#"{"format":"pleFlatQuestion","version":2,"title":"Imported color","prompt":"Which color was imported?","response":{"kind":"singleChoice","choices":[{"id":"blue","text":"Blue"},{"id":"red","text":"Red"}],"correctChoice":"blue"},"feedback":{"correct":"Imported answer retained","incorrect":"Try the imported blue choice"},"points":1.0,"attemptPolicy":{"maxAttempts":null},"timingPolicy":{"kind":"untimed"},"license":{"kind":"cc0"},"language":"en-US"}"#;
const EDITED_FLAT_SOURCE: &str = r#"{"format":"pleFlatQuestion","version":2,"title":"Edited color","prompt":"Which color should the edited question use?","response":{"kind":"singleChoice","choices":[{"id":"blue","text":"Blue"},{"id":"red","text":"Red"}],"correctChoice":"red"},"feedback":{"correct":"Edited answer retained","incorrect":"Try the edited red choice"},"points":1.0,"attemptPolicy":{"maxAttempts":null},"timingPolicy":{"kind":"untimed"},"license":{"kind":"cc0"},"language":"en-US"}"#;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

#[derive(Clone)]
struct FlatFixture {
    draft: DraftRecord,
    source: ObjectRecord,
    grading: FlatQuestionGradingPayload,
}

fn flat_fixture(tenant: TenantId, workspace: WorkspaceId, source: &str) -> FlatFixture {
    let document = adapter_native::flat_question::FlatQuestionDocument::parse(source.as_bytes())
        .expect("live flat source parses");
    let canonical = document
        .canonical_bytes()
        .expect("live flat source canonicalizes");
    let compiled = document
        .compile(workspace)
        .expect("live flat source compiles");
    let (question, private) = compiled.into_parts();
    let object = ObjectId::from_uuid(id());
    let key = ObjectKey::WorkspaceQuestionSource {
        tenant,
        workspace,
        object,
    };
    FlatFixture {
        draft: DraftRecord {
            tenant,
            question,
            derived_from: None,
        },
        source: ObjectRecord {
            id: object,
            bucket: key.bucket(),
            key,
            sha256: Sha256Digest::compute(&canonical),
            size_bytes: u64::try_from(canonical.len()).expect("fixture source size fits"),
            media_type: FLAT_MEDIA_TYPE.to_string(),
            category: ObjectCategory::Source,
            version: None,
            license: "CC0-1.0".to_string(),
            provenance: "disposable PostgreSQL flat-import fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(2_000),
        },
        grading: FlatQuestionGradingPayload::from_private(&private)
            .expect("compiled private material becomes a grading payload"),
    }
}

fn workspace_archive(
    tenant: TenantId,
    workspace: WorkspaceId,
    import: WorkspaceImportId,
) -> ObjectRecord {
    let bytes = b"original disposable Canvas QTI archive";
    let object = ObjectId::from_uuid(id());
    let key = ObjectKey::WorkspaceSource {
        tenant,
        workspace,
        import,
        object,
    };
    ObjectRecord {
        id: object,
        bucket: key.bucket(),
        key,
        sha256: Sha256Digest::compute(bytes),
        size_bytes: u64::try_from(bytes.len()).expect("archive fixture size fits"),
        media_type: "application/zip".to_string(),
        category: ObjectCategory::Source,
        version: None,
        license: "CC BY 4.0".to_string(),
        provenance: "disposable Canvas profile archive".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1_000),
    }
}

fn import_command(
    reference: QtiImportRef,
    source: ObjectRecord,
    digests: FlatImportIntegrityDigests,
) -> CreateQtiImportCommand {
    let item = QtiImportItem {
        item_id: ITEM_ID.to_string(),
        model_sha256: Sha256Digest::compute(b"safe imported profile item"),
        assets: Vec::new(),
    };
    let grading_bytes =
        serde_json::to_vec(&ChoiceId::new("blue")).expect("fixture choice ID serializes");
    let profile_summary = QtiImportProfileSummary::new(
        PersistedFlatImportProfile::CanvasQti12V1,
        digests.profile_report_sha256,
        fixed_profile_defaults(),
    )
    .expect("recognized profile summary fixture is valid");
    CreateQtiImportCommand {
        registry: QtiImportRegistry {
            reference,
            source,
            source_format: "qti".to_string(),
            source_identifier: Some("imsmanifest.xml".to_string()),
            importer: "adapter_qti".to_string(),
            parse_schema: profile_summary.profile_id().to_string(),
            adapter_version: "live-test".to_string(),
            profile_summary: Some(profile_summary),
            items: vec![item.clone()],
            item_results: vec![QtiImportItemResult {
                source_identifier: ITEM_ID.to_string(),
                title: Some("Live imported profile item".to_string()),
                item_id: Some(ITEM_ID.to_string()),
                normalized_sha256: Some(digests.normalized_item_sha256),
                status: QtiImportItemStatus::Accepted,
                diagnostics: Vec::new(),
                defaults: Vec::new(),
                warnings: Vec::new(),
            }],
            assets: Vec::new(),
            unsupported_features: Vec::new(),
        },
        item_bindings: vec![QtiImportItemRegistration {
            item,
            grading: QtiImportGradingPayload::new(grading_bytes)
                .expect("bounded private QTI grading fixture"),
        }],
    }
}

fn fixed_profile_defaults() -> Vec<QtiUnsupportedFeature> {
    [
        "PLE default applied: unlimited attempts.",
        "PLE default applied: untimed.",
        "PLE default applied: en-US.",
        "PLE default applied: allRightsReserved.",
        "PLE default applied: empty tags.",
        "PLE default applied: empty taxonomy.",
        "PLE default applied: no feedback.",
    ]
    .into_iter()
    .map(|detail| QtiUnsupportedFeature {
        code: "policy".to_string(),
        location: "item".to_string(),
        detail: detail.to_string(),
    })
    .collect()
}

fn integrity_digests(choice_map: &FlatImportChoiceMapPayload) -> FlatImportIntegrityDigests {
    FlatImportIntegrityDigests {
        normalized_item_sha256: Sha256Digest::compute(b"normalized imported item"),
        profile_report_sha256: Sha256Digest::compute(b"closed Canvas profile report"),
        public_mapping_sha256: Sha256Digest::compute(b"answer-free mapping"),
        private_mapping_sha256: Sha256Digest::compute(b"private grading mapping"),
        mapping_sha256: Sha256Digest::compute(b"combined mapping"),
        warning_sha256: Sha256Digest::compute(b"empty warning report"),
        choice_map_sha256: choice_map.sha256(),
    }
}

fn profile_evidence(
    reference: QtiImportRef,
    digests: FlatImportIntegrityDigests,
) -> QtiProfileImportEvidence {
    QtiProfileImportEvidence::new(
        reference,
        ITEM_ID.to_string(),
        PersistedFlatImportProfile::CanvasQti12V1,
        digests,
    )
    .expect("valid closed profile-evidence fixture")
}

#[derive(Debug, PartialEq, Eq)]
struct ProfileEvidenceSnapshot {
    profile: Option<serde_json::Value>,
    item: Option<serde_json::Value>,
}

async fn profile_evidence_snapshot(
    pool: &sqlx::PgPool,
    reference: QtiImportRef,
) -> ProfileEvidenceSnapshot {
    let row = sqlx::query(
        "SELECT \
            (SELECT to_jsonb(profile) FROM public.workspace_qti_profile_import_evidence AS profile \
              WHERE profile.tenant_id = $1 AND profile.workspace_id = $2 \
                AND profile.import_id = $3) AS profile_row, \
            (SELECT to_jsonb(item) FROM public.workspace_qti_profile_item_evidence AS item \
              WHERE item.tenant_id = $1 AND item.workspace_id = $2 \
                AND item.import_id = $3 AND item.item_id = $4) AS item_row",
    )
    .bind(reference.tenant.as_uuid())
    .bind(reference.workspace.as_uuid())
    .bind(reference.import.as_uuid())
    .bind(ITEM_ID)
    .fetch_one(pool)
    .await
    .expect("inspect protected profile-evidence snapshot");
    ProfileEvidenceSnapshot {
        profile: row.get("profile_row"),
        item: row.get("item_row"),
    }
}

async fn assert_profile_evidence_refused_without_mutation(
    store: &PostgresStore,
    pool: &sqlx::PgPool,
    context: TenantContext,
    reference: QtiImportRef,
    evidence: QtiProfileImportEvidence,
    expected: &ProfileEvidenceSnapshot,
) {
    assert!(
        store
            .stage_qti_profile_import_evidence(context, evidence)
            .await
            .is_err(),
        "divergent prepared profile evidence must refuse"
    );
    assert_eq!(
        &profile_evidence_snapshot(pool, reference).await,
        expected,
        "divergent staging leaves the exact prepared evidence unchanged"
    );
}

async fn commit_import(
    store: &PostgresStore,
    pool: &sqlx::PgPool,
    context: TenantContext,
    command: &CreateQtiImportCommand,
    digests: FlatImportIntegrityDigests,
) {
    store
        .prepare_qti_import(context, command.clone())
        .await
        .expect("prepare profile import through Store API");
    let reference = command.registry.reference;
    let exact_evidence = profile_evidence(reference, digests);
    store
        .stage_qti_profile_import_evidence(context, exact_evidence.clone())
        .await
        .expect("stage exact accepted profile evidence through Store API");
    store
        .stage_qti_profile_import_evidence(context, exact_evidence)
        .await
        .expect("exact profile-evidence replay is idempotent");
    let before_divergence = profile_evidence_snapshot(pool, reference).await;
    assert!(before_divergence.profile.is_some());
    assert!(before_divergence.item.is_some());
    let mut divergent_digests = digests;
    divergent_digests.warning_sha256 = Sha256Digest::compute(b"divergent staged warnings");
    assert_profile_evidence_refused_without_mutation(
        store,
        pool,
        context,
        reference,
        profile_evidence(reference, divergent_digests),
        &before_divergence,
    )
    .await;

    let job = store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant: reference.tenant,
                payload: JobPayload::QtiImport {
                    workspace: reference.workspace,
                    import: reference.import,
                    source_object: command.registry.source.id,
                },
                max_attempts: 2,
            },
        )
        .await
        .expect("enqueue profile import commit job");
    let filter = JobClaimFilter::new([JobKind::QtiImport]).expect("nonempty QTI job filter");
    let claim = store
        .claim_next_job(
            &filter,
            JobLeaseDuration::from_seconds(60).expect("bounded live-test lease"),
        )
        .await
        .expect("claim profile import commit job")
        .expect("profile import commit job is ready");
    assert_eq!(claim.id, job);
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference,
                    source_object: command.registry.source.id,
                },
            )
            .await
            .expect("commit prepared profile import"),
        CommitPreparedQtiImportOutcome::Committed
    );
}

fn import_origin(
    command: &CreateQtiImportCommand,
    actor: UserId,
    mapped_source_sha256: Sha256Digest,
    digests: FlatImportIntegrityDigests,
    choice_map: FlatImportChoiceMapPayload,
) -> WorkspaceFlatImportOrigin {
    WorkspaceFlatImportOrigin::new(
        command.registry.reference,
        ITEM_ID.to_string(),
        PersistedFlatImportProfile::CanvasQti12V1,
        FlatImportConversionVersion::new("native-flat-v1").expect("conversion version fixture"),
        command.registry.source.clone(),
        digests,
        mapped_source_sha256,
        actor,
        ActivityTimestamp::from_unix_millis(3_000),
        choice_map,
    )
    .expect("valid workspace flat-import origin")
}

fn conversion_command(
    fixture: &FlatFixture,
    expected_revision: WorkspaceDraftRevision,
    origin: WorkspaceFlatImportOrigin,
) -> QtiProfileFlatConversionCommand {
    QtiProfileFlatConversionCommand::new(
        Some(expected_revision),
        fixture.draft.clone(),
        fixture.source.clone(),
        fixture.source.sha256.to_string(),
        fixture.grading.public_binding_sha256().to_string(),
        fixture.grading.clone(),
        origin,
    )
    .expect("valid atomic profile conversion command")
}

#[derive(Debug, PartialEq, Eq)]
struct WorkspaceMutationSnapshot {
    drafts: i64,
    access_bindings: i64,
    flat_sources: i64,
    current_gradings: i64,
    current_origins: i64,
    current_choice_maps: i64,
}

async fn workspace_mutation_snapshot(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    workspace: WorkspaceId,
) -> WorkspaceMutationSnapshot {
    let row = sqlx::query(
        "SELECT \
            (SELECT count(*) FROM public.workspace_draft \
              WHERE tenant_id = $1 AND workspace_id = $2) AS drafts, \
            (SELECT count(*) FROM public.workspace_draft_access \
              WHERE tenant_id = $1 AND workspace_id = $2) AS access_bindings, \
            (SELECT count(*) FROM public.workspace_flat_question_source \
              WHERE tenant_id = $1 AND workspace_id = $2) AS flat_sources, \
            (SELECT count(*) FROM public.workspace_flat_question_grading \
              WHERE tenant_id = $1 AND workspace_id = $2) AS current_gradings, \
            (SELECT count(*) FROM public.workspace_flat_import_origin \
              WHERE tenant_id = $1 AND workspace_id = $2) AS current_origins, \
            (SELECT count(*) FROM public.workspace_flat_import_choice_map \
              WHERE tenant_id = $1 AND workspace_id = $2) AS current_choice_maps",
    )
    .bind(tenant.as_uuid())
    .bind(workspace.as_uuid())
    .fetch_one(pool)
    .await
    .expect("inspect workspace atomic-mutation snapshot");
    WorkspaceMutationSnapshot {
        drafts: row.get("drafts"),
        access_bindings: row.get("access_bindings"),
        flat_sources: row.get("flat_sources"),
        current_gradings: row.get("current_gradings"),
        current_origins: row.get("current_origins"),
        current_choice_maps: row.get("current_choice_maps"),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CurrentGradingSnapshot {
    draft_revision: i64,
    source_object_id: Uuid,
    canonical_source_sha256: String,
    public_binding_sha256: String,
    key_sha256: String,
}

async fn current_grading_snapshot(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    workspace: WorkspaceId,
) -> Option<CurrentGradingSnapshot> {
    sqlx::query(
        "SELECT draft_revision, source_object_id, canonical_source_sha256, \
                public_binding_sha256, key_sha256 \
           FROM public.workspace_flat_question_grading \
          WHERE tenant_id = $1 AND workspace_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(workspace.as_uuid())
    .fetch_optional(pool)
    .await
    .expect("inspect protected current flat grading")
    .map(|row| CurrentGradingSnapshot {
        draft_revision: row.get("draft_revision"),
        source_object_id: row.get("source_object_id"),
        canonical_source_sha256: row
            .get::<String, _>("canonical_source_sha256")
            .trim_end()
            .to_string(),
        public_binding_sha256: row
            .get::<String, _>("public_binding_sha256")
            .trim_end()
            .to_string(),
        key_sha256: row.get::<String, _>("key_sha256").trim_end().to_string(),
    })
}

#[derive(Debug, PartialEq)]
struct WorkspacePublicationSnapshot {
    draft: Option<serde_json::Value>,
    source: Option<serde_json::Value>,
    origin: Option<serde_json::Value>,
    grading: Option<CurrentGradingSnapshot>,
}

async fn workspace_publication_snapshot(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    workspace: WorkspaceId,
) -> WorkspacePublicationSnapshot {
    let row = sqlx::query(
        "SELECT \
            (SELECT to_jsonb(draft) FROM public.workspace_draft AS draft \
              WHERE draft.tenant_id = $1 AND draft.workspace_id = $2) AS draft_row, \
            (SELECT to_jsonb(source) FROM public.workspace_flat_question_source AS source \
              WHERE source.tenant_id = $1 AND source.workspace_id = $2) AS source_row, \
            (SELECT to_jsonb(origin) FROM public.workspace_flat_import_origin AS origin \
              WHERE origin.tenant_id = $1 AND origin.workspace_id = $2) AS origin_row",
    )
    .bind(tenant.as_uuid())
    .bind(workspace.as_uuid())
    .fetch_one(pool)
    .await
    .expect("inspect publication rollback snapshot");
    WorkspacePublicationSnapshot {
        draft: row.get("draft_row"),
        source: row.get("source_row"),
        origin: row.get("origin_row"),
        grading: current_grading_snapshot(pool, tenant, workspace).await,
    }
}

#[derive(Clone, Copy)]
enum PrivateRelation {
    CurrentChoiceMap,
    PublishedChoiceMap,
    CurrentGrading,
}

async fn assert_private_relation_denied(
    pool: &sqlx::PgPool,
    role: &str,
    tenant: TenantId,
    relation: PrivateRelation,
) {
    let mut transaction = pool
        .begin()
        .await
        .expect("begin restricted role transaction");
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
        "ple_grader" => {
            sqlx::query("SET LOCAL ROLE ple_grader")
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
        .expect("set restricted-role tenant context");
    let result = match relation {
        PrivateRelation::PublishedChoiceMap => {
            sqlx::query("SELECT payload FROM public.published_flat_import_choice_map")
                .fetch_all(&mut *transaction)
                .await
        }
        PrivateRelation::CurrentChoiceMap => {
            sqlx::query("SELECT payload FROM public.workspace_flat_import_choice_map")
                .fetch_all(&mut *transaction)
                .await
        }
        PrivateRelation::CurrentGrading => {
            sqlx::query("SELECT key_payload FROM public.workspace_flat_question_grading")
                .fetch_all(&mut *transaction)
                .await
        }
    };
    let error = result.expect_err("restricted role must not directly read private staging");
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
        .expect("rollback restricted role transaction");
}

fn published_flat_source(
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
            provenance: "published edited flat-question source".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(4_000),
        },
    }
}

fn publication_command(
    fixture: &FlatFixture,
    expected_revision: WorkspaceDraftRevision,
    reference: ProblemVersionRef,
    flat_question_promotion: Option<FlatQuestionPublicationPromotion>,
    actor: UserId,
) -> PublishDraftCommand {
    PublishDraftCommand {
        expected_draft: fixture.draft.clone(),
        expected_revision,
        publication: reference,
        published_source: QuestionSource::Native {
            family: "flat_single_choice_v2".to_string(),
        },
        source_artifact: Some(published_flat_source(reference, &fixture.source)),
        qti_promotion: None,
        flat_question_promotion,
        publisher: actor,
        scope: PublicationScope::Institution,
        byline: question_model::PublicByline::new(vec![
            question_model::PublicAuthorName::new("PLE fixture".to_string())
                .expect("valid test byline"),
        ])
        .expect("valid test byline"),
        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
    }
}

fn published_import_archive(
    reference: ProblemVersionRef,
    origin: &WorkspaceFlatImportOrigin,
) -> ObjectRecord {
    let import = origin.import();
    let source = origin.source_archive();
    let object = published_import_archive_object_id(
        import.tenant,
        reference.problem,
        reference.version,
        import.import,
        source.sha256,
    );
    let key = ObjectKey::PublishedImportArchive {
        tenant: import.tenant,
        problem: reference.problem,
        version: reference.version,
        import: import.import,
        object,
    };
    assert!(!key.may_issue_signed_url());
    ObjectRecord {
        id: object,
        bucket: key.bucket(),
        key,
        sha256: source.sha256,
        size_bytes: source.size_bytes,
        media_type: source.media_type.clone(),
        category: ObjectCategory::Source,
        version: Some(reference.version),
        license: source.license.clone(),
        provenance: "published immutable Canvas archive".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(5_000),
    }
}

async fn assert_published_origin_immutable(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    reference: ProblemVersionRef,
) {
    for statement in [
        "UPDATE public.published_flat_import_origin SET conversion_version = 'tampered' \
         WHERE owner_tenant_id = $1 AND problem_id = $2 AND version_id = $3",
        "DELETE FROM public.published_flat_import_origin \
         WHERE owner_tenant_id = $1 AND problem_id = $2 AND version_id = $3",
        "UPDATE public.published_flat_import_choice_map SET payload = decode('00', 'hex') \
         WHERE owner_tenant_id = $1 AND problem_id = $2 AND version_id = $3",
    ] {
        let mut transaction = pool.begin().await.expect("begin immutability probe");
        let error = sqlx::query(statement)
            .bind(tenant.as_uuid())
            .bind(reference.problem.as_uuid())
            .bind(reference.version.as_uuid())
            .execute(&mut *transaction)
            .await
            .expect_err("published provenance must reject mutation");
        assert_eq!(
            error
                .as_database_error()
                .and_then(|value| value.code())
                .as_deref(),
            Some("55000")
        );
        transaction
            .rollback()
            .await
            .expect("rollback immutability probe");
    }
}
