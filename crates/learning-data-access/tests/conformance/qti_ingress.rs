use super::*;
use learning_data_access::{
    FlatImportIntegrityDigests, FlatImportProvenanceStore, PersistedFlatImportProfile,
    QtiImportApiState, QtiImportApiStore, QtiImportApiView, QtiImportProfileSummary,
    QtiProfileImportEvidence, QueueQtiImportCommand,
};
use objects::workspace_qti_archive_object_id;

fn archive_record(reference: QtiImportRef, bytes: &[u8]) -> ObjectRecord {
    let object =
        workspace_qti_archive_object_id(reference.tenant, reference.workspace, reference.import);
    let key = ObjectKey::WorkspaceSource {
        tenant: reference.tenant,
        workspace: reference.workspace,
        import: reference.import,
        object,
    };
    ObjectRecord {
        id: object,
        bucket: key.bucket(),
        key,
        sha256: Sha256Digest::compute(bytes),
        size_bytes: u64::try_from(bytes.len()).expect("fixture length fits"),
        media_type: "application/zip".to_string(),
        category: ObjectCategory::Source,
        version: None,
        license: "allRightsReserved".to_string(),
        provenance: "QTI ingress conformance fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(91_000),
    }
}

fn queue_command(reference: QtiImportRef, bytes: &[u8]) -> QueueQtiImportCommand {
    QueueQtiImportCommand {
        reference,
        source: archive_record(reference, bytes),
        max_attempts: 2,
    }
}

async fn create_workspace<S: Store>(
    store: &S,
    reference: QtiImportRef,
    owner: UserId,
) -> learning_data_access::WorkspaceDraft {
    store
        .upsert_draft(
            TenantContext::from_authenticated_session(reference.tenant),
            owner,
            None,
            DraftRecord {
                tenant: reference.tenant,
                question: draft_question(reference.workspace),
                derived_from: None,
            },
        )
        .await
        .expect("workspace fixture saves")
}

fn assert_private_state(view: &QtiImportApiView, expected: QtiImportApiState) {
    assert_eq!(view.state, expected);
    assert!(view.registry.is_none());
}

fn committed_import(command: &QueueQtiImportCommand) -> CreateQtiImportCommand {
    let item = QtiImportItem {
        item_id: "canvas-item-1".to_string(),
        model_sha256: Sha256Digest::compute(b"answer-free canonical item"),
        assets: Vec::new(),
    };
    CreateQtiImportCommand {
        registry: QtiImportRegistry {
            reference: command.reference,
            source: command.source.clone(),
            source_format: "qti".to_string(),
            source_identifier: Some("chapter-1.zip".to_string()),
            importer: "adapter_qti".to_string(),
            parse_schema: "canvas-qti-1.2".to_string(),
            adapter_version: "v1".to_string(),
            profile_summary: None,
            items: vec![item.clone()],
            item_results: vec![QtiImportItemResult {
                source_identifier: item.item_id.clone(),
                title: Some("Imported item".to_string()),
                item_id: Some(item.item_id.clone()),
                normalized_sha256: Some(Sha256Digest::compute(b"normalized imported item")),
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
            grading: QtiImportGradingPayload::new(br#""choice-1""#.to_vec())
                .expect("bounded grading fixture"),
        }],
    }
}

fn profiled_import(
    command: &QueueQtiImportCommand,
    profile_report: &[u8],
) -> (CreateQtiImportCommand, QtiProfileImportEvidence) {
    let mut import = committed_import(command);
    let summary = QtiImportProfileSummary::new(
        PersistedFlatImportProfile::CanvasQti12V1,
        Sha256Digest::compute(profile_report),
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
        .collect(),
    )
    .expect("profile fixture is valid");
    let normalized = Sha256Digest::compute(profile_report);
    import.registry.parse_schema = summary.profile_id().to_string();
    import.registry.profile_summary = Some(summary.clone());
    let accepted = &mut import.registry.item_results[0];
    accepted.source_identifier = accepted.item_id.clone().expect("accepted item has an ID");
    accepted.normalized_sha256 = Some(normalized);

    let evidence = QtiProfileImportEvidence::new(
        import.registry.reference,
        accepted.source_identifier.clone(),
        PersistedFlatImportProfile::CanvasQti12V1,
        FlatImportIntegrityDigests {
            normalized_item_sha256: normalized,
            profile_report_sha256: summary.profile_report_sha256(),
            public_mapping_sha256: Sha256Digest::compute(b"lifecycle public mapping"),
            private_mapping_sha256: Sha256Digest::compute(b"lifecycle private mapping"),
            mapping_sha256: Sha256Digest::compute(b"lifecycle mapping"),
            warning_sha256: Sha256Digest::compute(b"lifecycle warnings"),
            choice_map_sha256: Sha256Digest::compute(b"lifecycle choice map"),
        },
    )
    .expect("profile evidence fixture is valid");
    (import, evidence)
}

#[tokio::test]
async fn memory_qti_ingress_exact_replay_failure_and_nonenumeration_conform() {
    let store = MemoryStore::default();
    let reference = QtiImportRef {
        tenant: TenantId::from_uuid(uuid(91_001)),
        workspace: WorkspaceId::from_uuid(uuid(91_002)),
        import: WorkspaceImportId::from_uuid(uuid(91_003)),
    };
    let owner = UserId::from_uuid(uuid(91_004));
    let stranger = UserId::from_uuid(uuid(91_005));
    let context = TenantContext::from_authenticated_session(reference.tenant);
    create_workspace(&store, reference, owner).await;
    let command = queue_command(reference, b"PK\x03\x04canvas fixture");

    let mut invalid_source = command.clone();
    invalid_source.source.media_type = "application/xml".to_string();
    assert!(matches!(
        store.queue_qti_import(context, owner, invalid_source).await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .ready_queue_depth(&JobClaimFilter::new([JobKind::QtiImport]).expect("filter"))
            .await
            .expect("queue depth")
            .ready,
        0,
        "invalid archive metadata must not create work"
    );

    let created = store
        .queue_qti_import(context, owner, command.clone())
        .await
        .expect("first queue succeeds");
    assert_private_state(&created, QtiImportApiState::Queued);
    let replay = store
        .queue_qti_import(context, owner, command.clone())
        .await
        .expect("exact replay succeeds");
    assert_private_state(&replay, QtiImportApiState::Queued);
    assert_eq!(
        store
            .ready_queue_depth(&JobClaimFilter::new([JobKind::QtiImport]).expect("filter"))
            .await
            .expect("queue depth")
            .ready,
        1,
        "exact replay must retain one worker job"
    );

    let mut divergent = command.clone();
    divergent.max_attempts = 3;
    assert!(matches!(
        store.queue_qti_import(context, owner, divergent).await,
        Err(StoreError::Conflict)
    ));
    assert!(
        store
            .qti_import_view(context, stranger, reference.workspace, reference.import)
            .await
            .expect("unbound actor lookup")
            .is_none()
    );
    assert!(
        store
            .qti_import_view(
                TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(91_006))),
                owner,
                reference.workspace,
                reference.import,
            )
            .await
            .expect("foreign tenant lookup")
            .is_none()
    );

    let claim = store
        .claim_next_job(
            &JobClaimFilter::new([JobKind::QtiImport]).expect("filter"),
            JobLeaseDuration::from_seconds(60).expect("lease"),
        )
        .await
        .expect("claim succeeds")
        .expect("queued import exists");
    let processing = store
        .qti_import_view(context, owner, reference.workspace, reference.import)
        .await
        .expect("processing lookup")
        .expect("request remains visible");
    assert_private_state(&processing, QtiImportApiState::Processing);
    store
        .fail_job(claim.id, claim.lease_token, JobFailureKind::Permanent)
        .await
        .expect("permanent refusal persists");
    let failed = store
        .qti_import_view(context, owner, reference.workspace, reference.import)
        .await
        .expect("failed lookup")
        .expect("failed request remains visible");
    assert_private_state(&failed, QtiImportApiState::Failed);
}

#[tokio::test]
async fn memory_qti_ingress_becomes_ready_only_with_atomic_registry_commit() {
    let store = MemoryStore::default();
    let reference = QtiImportRef {
        tenant: TenantId::from_uuid(uuid(91_101)),
        workspace: WorkspaceId::from_uuid(uuid(91_102)),
        import: WorkspaceImportId::from_uuid(uuid(91_103)),
    };
    let owner = UserId::from_uuid(uuid(91_104));
    let context = TenantContext::from_authenticated_session(reference.tenant);
    create_workspace(&store, reference, owner).await;
    let command = queue_command(reference, b"PK\x03\x04blackboard fixture");
    store
        .queue_qti_import(context, owner, command.clone())
        .await
        .expect("queue succeeds");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::new([JobKind::QtiImport]).expect("filter"),
            JobLeaseDuration::from_seconds(60).expect("lease"),
        )
        .await
        .expect("claim succeeds")
        .expect("queued import exists");
    let prepared = committed_import(&command);
    store
        .prepare_qti_import(context, prepared.clone())
        .await
        .expect("worker preparation succeeds");
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job: claim.id,
                    lease: claim.lease_token,
                    reference,
                    source_object: command.source.id,
                },
            )
            .await
            .expect("atomic worker commit succeeds"),
        CommitPreparedQtiImportOutcome::Committed
    );
    let ready = store
        .qti_import_view(context, owner, reference.workspace, reference.import)
        .await
        .expect("ready lookup")
        .expect("committed request remains visible");
    assert_eq!(ready.state, QtiImportApiState::Ready);
    assert_eq!(ready.registry, Some(prepared.registry));
}

#[tokio::test]
async fn memory_draft_deletion_removes_its_queued_qti_request() {
    let store = MemoryStore::default();
    let reference = QtiImportRef {
        tenant: TenantId::from_uuid(uuid(91_201)),
        workspace: WorkspaceId::from_uuid(uuid(91_202)),
        import: WorkspaceImportId::from_uuid(uuid(91_203)),
    };
    let owner = UserId::from_uuid(uuid(91_204));
    let context = TenantContext::from_authenticated_session(reference.tenant);
    let draft = create_workspace(&store, reference, owner).await;
    store
        .queue_qti_import(
            context,
            owner,
            queue_command(reference, b"PK\x03\x04deletion fixture"),
        )
        .await
        .expect("queue succeeds");

    assert!(
        store
            .delete_draft(context, owner, reference.workspace, draft.revision)
            .await
            .expect("owner deletion succeeds")
    );
    assert!(
        store
            .qti_import_view(context, owner, reference.workspace, reference.import)
            .await
            .expect("deleted workspace lookup")
            .is_none()
    );
    assert_eq!(
        store
            .ready_queue_depth(&JobClaimFilter::new([JobKind::QtiImport]).expect("filter"))
            .await
            .expect("queue depth")
            .ready,
        0
    );
}

#[tokio::test]
async fn memory_draft_deletion_discards_prepared_qti_state_and_fences_late_preparation() {
    let store = MemoryStore::default();
    let reference = QtiImportRef {
        tenant: TenantId::from_uuid(uuid(91_301)),
        workspace: WorkspaceId::from_uuid(uuid(91_302)),
        import: WorkspaceImportId::from_uuid(uuid(91_303)),
    };
    let owner = UserId::from_uuid(uuid(91_304));
    let context = TenantContext::from_authenticated_session(reference.tenant);
    let draft = create_workspace(&store, reference, owner).await;
    let queued = queue_command(reference, b"PK\x03\x04prepared lifecycle fixture");
    let (prepared, evidence) = profiled_import(&queued, b"first profile report");

    store
        .prepare_qti_import(context, prepared)
        .await
        .expect("existing draft permits preparation");
    store
        .stage_qti_profile_import_evidence(context, evidence)
        .await
        .expect("prepared profile evidence stages");
    assert!(
        store
            .delete_draft(context, owner, reference.workspace, draft.revision)
            .await
            .expect("owner deletion succeeds")
    );

    let (late_prepare, _) = profiled_import(&queued, b"first profile report");
    assert!(matches!(
        store.prepare_qti_import(context, late_prepare).await,
        Err(StoreError::NotFound)
    ));

    create_workspace(&store, reference, owner).await;
    let (reprepared, replacement_evidence) =
        profiled_import(&queued, b"replacement profile report");
    store
        .prepare_qti_import(context, reprepared)
        .await
        .expect("recreated draft can prepare the same import identity");
    store
        .stage_qti_profile_import_evidence(context, replacement_evidence)
        .await
        .expect("prepared-only evidence was removed with the deleted draft");
}
