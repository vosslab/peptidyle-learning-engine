#![cfg(feature = "postgres")]

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand, DraftRecord,
    EnqueueJob, JobClaimFilter, JobLeaseDuration, JobPayload, JobStore, QtiImportGradingPayload,
    QtiImportItem, QtiImportItemRegistration, QtiImportItemResult, QtiImportItemStatus,
    QtiImportRef, QtiImportRegistry, QtiImportStore, QtiUnsupportedFeature, Store, TenantContext,
};
use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::{License, Tag};
use question_model::{
    ActivityTimestamp, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, ObjectId,
    QuestionMetadata, ResponseDefinition, TenantId, UserId, WorkspaceId, WorkspaceImportId,
};
use sqlx::Row;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn warning(code: &str, location: &str, detail: &str) -> QtiUnsupportedFeature {
    QtiUnsupportedFeature {
        code: code.to_string(),
        location: location.to_string(),
        detail: detail.to_string(),
    }
}

fn workspace_draft(tenant: TenantId, workspace: WorkspaceId) -> DraftRecord {
    DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: "molar_mass".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "What is the molar mass?".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "QTI import workspace".to_string(),
                tags: vec![Tag::new("qti")],
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    }
}

fn import_command(
    tenant: TenantId,
    workspace: WorkspaceId,
    import: WorkspaceImportId,
) -> CreateQtiImportCommand {
    let object = ObjectId::from_uuid(id());
    let source_key = ObjectKey::WorkspaceSource {
        tenant,
        workspace,
        import,
        object,
    };
    let source = ObjectRecord {
        id: object,
        bucket: source_key.bucket(),
        key: source_key,
        sha256: Sha256Digest::compute(b"original live QTI archive"),
        size_bytes: 25,
        media_type: "application/zip".to_string(),
        category: ObjectCategory::Source,
        version: None,
        license: "CC BY 4.0".to_string(),
        provenance: "live QTI partial-import fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1),
    };
    let item = QtiImportItem {
        item_id: "accepted-item".to_string(),
        model_sha256: Sha256Digest::compute(b"canonical accepted item"),
        assets: Vec::new(),
    };
    let exact_duplicate = warning(
        "exact-duplicate-item",
        "accepted-item",
        "the normalized item matches another source item in this batch",
    );
    let rejected = warning(
        "unsupported-interaction",
        "rejected-item",
        "the original package retains the unsupported item for correction",
    );
    CreateQtiImportCommand {
        registry: QtiImportRegistry {
            reference: QtiImportRef {
                tenant,
                workspace,
                import,
            },
            source,
            source_format: "qti".to_string(),
            source_identifier: Some("live-manifest".to_string()),
            importer: "adapter_qti".to_string(),
            parse_schema: "imsqti_v2_1".to_string(),
            adapter_version: "live-test".to_string(),
            profile_summary: None,
            items: vec![item.clone()],
            item_results: vec![
                QtiImportItemResult {
                    source_identifier: "resource-accepted".to_string(),
                    title: Some("Accepted live item".to_string()),
                    item_id: Some(item.item_id.clone()),
                    normalized_sha256: Some(Sha256Digest::compute(
                        b"normalized item and private grading semantics",
                    )),
                    status: QtiImportItemStatus::Accepted,
                    diagnostics: Vec::new(),
                    defaults: Vec::new(),
                    warnings: vec![exact_duplicate.clone()],
                },
                QtiImportItemResult {
                    source_identifier: "resource-rejected".to_string(),
                    title: Some("Rejected live item".to_string()),
                    item_id: None,
                    normalized_sha256: None,
                    status: QtiImportItemStatus::Rejected,
                    diagnostics: vec![rejected.clone()],
                    defaults: Vec::new(),
                    warnings: Vec::new(),
                },
            ],
            assets: Vec::new(),
            unsupported_features: vec![exact_duplicate, rejected],
        },
        item_bindings: vec![QtiImportItemRegistration {
            item,
            grading: QtiImportGradingPayload::new(b"live-secret-choice".to_vec())
                .expect("bounded private grading fixture"),
        }],
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_qti_import_preserves_partial_results_provenance_and_rls() {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let pool = lazy_pool(database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());

    let tenant = TenantId::from_uuid(id());
    let foreign = TenantId::from_uuid(id());
    let workspace = WorkspaceId::from_uuid(id());
    let import = WorkspaceImportId::from_uuid(id());
    let actor = UserId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let command = import_command(tenant, workspace, import);

    store
        .upsert_draft(context, actor, None, workspace_draft(tenant, workspace))
        .await
        .expect("save the author workspace before preparing its QTI import");

    store
        .prepare_qti_import(context, command.clone())
        .await
        .expect("prepare partial QTI import");
    store
        .prepare_qti_import(context, command.clone())
        .await
        .expect("exact preparation replay is idempotent");
    assert!(
        store
            .get_qti_import(context, workspace, import)
            .await
            .expect("prepared registry lookup")
            .is_none(),
        "prepared imports stay invisible"
    );

    let job = store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::QtiImport {
                    workspace,
                    import,
                    source_object: command.registry.source.id,
                },
                max_attempts: 2,
            },
        )
        .await
        .expect("enqueue QTI import job");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("lease"),
        )
        .await
        .expect("claim QTI import job")
        .expect("QTI import job is ready");
    assert_eq!(claim.id, job);
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: command.registry.reference,
                    source_object: command.registry.source.id,
                },
            )
            .await
            .expect("commit exact QTI preparation"),
        CommitPreparedQtiImportOutcome::Committed
    );

    let registry = store
        .get_qti_import(context, workspace, import)
        .await
        .expect("read committed QTI import")
        .expect("committed registry exists");
    assert_eq!(registry, command.registry);
    assert_eq!(registry.source_identifier.as_deref(), Some("live-manifest"));
    assert_eq!(registry.importer, "adapter_qti");
    assert_eq!(registry.item_results.len(), 2);
    assert!(matches!(
        registry.item_results[0].status,
        QtiImportItemStatus::Accepted
    ));
    assert!(matches!(
        registry.item_results[1].status,
        QtiImportItemStatus::Rejected
    ));
    assert!(
        store
            .get_qti_import(
                TenantContext::from_authenticated_session(foreign),
                workspace,
                import,
            )
            .await
            .expect("foreign QTI lookup")
            .is_none(),
        "foreign tenants cannot enumerate committed imports"
    );

    let rows = sqlx::query(
        "SELECT status, normalized_sha256, payload::text AS payload_text \
           FROM workspace_qti_import_result \
          WHERE tenant_id = $1 AND workspace_id = $2 AND import_id = $3 \
          ORDER BY ordinal",
    )
    .bind(tenant.as_uuid())
    .bind(workspace.as_uuid())
    .bind(import.as_uuid())
    .fetch_all(&pool)
    .await
    .expect("inspect normalized per-item persistence");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<String, _>("status"), "accepted");
    assert!(
        rows[0]
            .get::<Option<String>, _>("normalized_sha256")
            .is_some()
    );
    assert_eq!(rows[1].get::<String, _>("status"), "rejected");
    assert!(
        rows[1]
            .get::<Option<String>, _>("normalized_sha256")
            .is_none()
    );
    let persisted_results = rows
        .iter()
        .map(|row| row.get::<String, _>("payload_text"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(persisted_results.contains("unsupported-interaction"));
    assert!(persisted_results.contains("exact-duplicate-item"));
    assert!(!persisted_results.contains("live-secret-choice"));

    let safe_registry = serde_json::to_string(&registry).expect("serialize safe QTI registry");
    assert!(!safe_registry.contains("live-secret-choice"));
}
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
