use super::assets::source_artifact;
use super::*;
use learning_data_access::{
    FlatImportIntegrityDigests, FlatImportProvenanceStore, PersistedFlatImportProfile,
    QtiProfileImportEvidence,
};

fn qti_import_command(
    tenant: TenantId,
    workspace: WorkspaceId,
    import: WorkspaceImportId,
) -> CreateQtiImportCommand {
    let source_key = ObjectKey::WorkspaceSource {
        tenant,
        workspace,
        import,
        object: ObjectId::from_uuid(uuid(9_011)),
    };
    let source = ObjectRecord {
        id: source_key.object_id(),
        bucket: source_key.bucket(),
        key: source_key,
        sha256: Sha256Digest::compute(b"qti zip fixture"),
        size_bytes: 15,
        media_type: "application/zip".to_string(),
        category: objects::ObjectCategory::Source,
        version: None,
        license: "CC BY-SA 4.0".to_string(),
        provenance: "QTI import conformance fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1),
    };
    let logical_asset = AssetId::from_uuid(uuid(9_012));
    let asset_key = ObjectKey::WorkspaceAsset {
        tenant,
        workspace,
        import,
        asset: logical_asset,
        object: ObjectId::from_uuid(uuid(9_013)),
    };
    let asset = object_record(asset_key, b"<svg/>", 1);
    let item = QtiImportItem {
        item_id: "item-1".to_string(),
        model_sha256: Sha256Digest::compute(b"canonical item model"),
        assets: vec![logical_asset],
    };
    CreateQtiImportCommand {
        registry: QtiImportRegistry {
            reference: QtiImportRef {
                tenant,
                workspace,
                import,
            },
            source,
            source_format: "qti".to_string(),
            source_identifier: Some("package-1".to_string()),
            importer: "adapter_qti".to_string(),
            parse_schema: "imsqti_v2_1".to_string(),
            adapter_version: "1".to_string(),
            profile_summary: None,
            items: vec![item.clone()],
            item_results: vec![
                QtiImportItemResult {
                    source_identifier: "resource-1".to_string(),
                    title: Some("Accepted source item".to_string()),
                    item_id: Some(item.item_id.clone()),
                    normalized_sha256: Some(Sha256Digest::compute(b"normalized item and grading")),
                    status: QtiImportItemStatus::Accepted,
                    diagnostics: Vec::new(),
                    defaults: vec![QtiUnsupportedFeature {
                        code: "policy".to_string(),
                        location: "item".to_string(),
                        detail: "PLE default applied: unlimited attempts.".to_string(),
                    }],
                    warnings: vec![QtiUnsupportedFeature {
                        code: "choiceInteraction.shuffle".to_string(),
                        location: "item-1".to_string(),
                        detail: "shuffle is retained in the source package".to_string(),
                    }],
                },
                QtiImportItemResult {
                    source_identifier: "resource-2".to_string(),
                    title: Some("Rejected source item".to_string()),
                    item_id: None,
                    normalized_sha256: None,
                    status: QtiImportItemStatus::Rejected,
                    diagnostics: vec![QtiUnsupportedFeature {
                        code: "unsupported-interaction".to_string(),
                        location: "item-2".to_string(),
                        detail: "the source item remains available for correction".to_string(),
                    }],
                    defaults: Vec::new(),
                    warnings: Vec::new(),
                },
            ],
            assets: vec![asset],
            unsupported_features: vec![
                QtiUnsupportedFeature {
                    code: "choiceInteraction.shuffle".to_string(),
                    location: "item-1".to_string(),
                    detail: "shuffle is retained in the source package".to_string(),
                },
                QtiUnsupportedFeature {
                    code: "unsupported-interaction".to_string(),
                    location: "item-2".to_string(),
                    detail: "the source item remains available for correction".to_string(),
                },
            ],
        },
        item_bindings: vec![QtiImportItemRegistration {
            item,
            grading: QtiImportGradingPayload::new(b"correct-choice=2".to_vec())
                .expect("bounded test grading binding"),
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

fn recognized_profile_summary() -> learning_data_access::QtiImportProfileSummary {
    learning_data_access::QtiImportProfileSummary::new(
        learning_data_access::PersistedFlatImportProfile::CanvasQti12V1,
        Sha256Digest::compute(b"canonical safe Canvas profile report"),
        fixed_profile_defaults(),
    )
    .expect("recognized profile summary fixture is valid")
}

fn recognized_profile_command(
    tenant: TenantId,
    workspace: WorkspaceId,
    import: WorkspaceImportId,
) -> (CreateQtiImportCommand, QtiProfileImportEvidence) {
    let mut command = qti_import_command(tenant, workspace, import);
    let summary = recognized_profile_summary();
    let normalized = Sha256Digest::compute(b"recognized profile normalized item");
    let report = summary.profile_report_sha256();
    command.registry.parse_schema = summary.profile_id().to_string();
    command.registry.profile_summary = Some(summary);
    let accepted = &mut command.registry.item_results[0];
    accepted.source_identifier = "item-1".to_string();
    accepted.normalized_sha256 = Some(normalized);

    let digests = FlatImportIntegrityDigests {
        normalized_item_sha256: normalized,
        profile_report_sha256: report,
        public_mapping_sha256: Sha256Digest::compute(b"recognized profile public map"),
        private_mapping_sha256: Sha256Digest::compute(b"recognized profile private map"),
        mapping_sha256: Sha256Digest::compute(b"recognized profile map"),
        warning_sha256: Sha256Digest::compute(b"recognized profile warnings"),
        choice_map_sha256: Sha256Digest::compute(b"recognized profile choice map"),
    };
    let evidence = QtiProfileImportEvidence::new(
        command.registry.reference,
        "item-1".to_string(),
        PersistedFlatImportProfile::CanvasQti12V1,
        digests,
    )
    .expect("recognized profile evidence is structurally valid");
    (command, evidence)
}

async fn save_workspace<S: Store>(
    store: &S,
    tenant: TenantId,
    workspace: WorkspaceId,
    owner: UserId,
) -> learning_data_access::WorkspaceDraft {
    store
        .upsert_draft(
            TenantContext::from_authenticated_session(tenant),
            owner,
            None,
            DraftRecord {
                tenant,
                question: draft_question(workspace),
                derived_from: None,
            },
        )
        .await
        .expect("QTI fixture workspace saves")
}

#[test]
fn qti_profile_summary_round_trips_only_closed_safe_evidence() {
    let mut registry = qti_import_command(
        TenantId::from_uuid(uuid(9_201)),
        WorkspaceId::from_uuid(uuid(9_202)),
        WorkspaceImportId::from_uuid(uuid(9_203)),
    )
    .registry;
    let expected = recognized_profile_summary();
    registry.profile_summary = Some(expected.clone());

    let value = serde_json::to_value(&registry).expect("registry serializes");
    let decoded: QtiImportRegistry =
        serde_json::from_value(value.clone()).expect("persisted registry decodes");

    assert_eq!(decoded.profile_summary.as_ref(), Some(&expected));
    assert_eq!(
        value["profileSummary"],
        serde_json::json!({
            "profileId": "canvas-qti-1.2-static-single-choice/v1",
            "profileVersion": "v1",
            "mappingVersion": "v1",
            "profileReportSha256": expected.profile_report_sha256().to_string(),
            "defaults": expected.defaults()
        })
    );
}

#[test]
fn qti_registry_decodes_legacy_generic_payload_without_profile_summary() {
    let registry = qti_import_command(
        TenantId::from_uuid(uuid(9_211)),
        WorkspaceId::from_uuid(uuid(9_212)),
        WorkspaceImportId::from_uuid(uuid(9_213)),
    )
    .registry;
    let mut value = serde_json::to_value(&registry).expect("legacy registry serializes");
    value
        .as_object_mut()
        .expect("registry JSON object")
        .remove("profileSummary");

    let decoded: QtiImportRegistry =
        serde_json::from_value(value).expect("legacy generic registry decodes");

    assert_eq!(decoded.profile_summary, None);
}

#[test]
fn qti_profile_summary_persisted_decode_rejects_open_profile_tuple() {
    let mut value = serde_json::to_value(recognized_profile_summary()).expect("summary serializes");
    value["profileVersion"] = serde_json::Value::String("v2".to_string());
    assert!(
        serde_json::from_value::<learning_data_access::QtiImportProfileSummary>(value).is_err()
    );
}

#[test]
fn qti_profile_summary_refuses_more_than_32_safe_defaults() {
    let too_many_defaults = vec![
        QtiUnsupportedFeature {
            code: "policy".to_string(),
            location: "package".to_string(),
            detail: "bounded default".to_string(),
        };
        33
    ];
    assert!(
        learning_data_access::QtiImportProfileSummary::new(
            learning_data_access::PersistedFlatImportProfile::BlackboardQti21V1,
            Sha256Digest::compute(b"Blackboard report"),
            too_many_defaults,
        )
        .is_err()
    );
}

#[test]
fn qti_profile_summary_refuses_bounded_but_noncanonical_defaults() {
    let mut defaults = fixed_profile_defaults();
    defaults[0].detail = "PLE default applied: unlimited attempts!".to_string();

    assert!(
        learning_data_access::QtiImportProfileSummary::new(
            learning_data_access::PersistedFlatImportProfile::CanvasQti12V1,
            Sha256Digest::compute(b"Canvas report"),
            defaults,
        )
        .is_err()
    );
}

#[tokio::test]
async fn memory_qti_import_refuses_profile_summary_with_generic_registry_schema() {
    let tenant = TenantId::from_uuid(uuid(9_221));
    let workspace = WorkspaceId::from_uuid(uuid(9_222));
    let import = WorkspaceImportId::from_uuid(uuid(9_223));
    let mut command = qti_import_command(tenant, workspace, import);
    command.registry.profile_summary = Some(recognized_profile_summary());

    assert!(matches!(
        MemoryStore::default()
            .prepare_qti_import(TenantContext::from_authenticated_session(tenant), command)
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

#[tokio::test]
async fn memory_recognized_qti_commit_requires_exact_complete_private_evidence() {
    let tenant = TenantId::from_uuid(uuid(9_231));
    let workspace = WorkspaceId::from_uuid(uuid(9_232));
    let import = WorkspaceImportId::from_uuid(uuid(9_233));
    let context = TenantContext::from_authenticated_session(tenant);
    let (command, evidence) = recognized_profile_command(tenant, workspace, import);
    let store = MemoryStore::default();
    save_workspace(&store, tenant, workspace, UserId::from_uuid(uuid(9_234))).await;
    store
        .prepare_qti_import(context, command.clone())
        .await
        .expect("recognized import prepares invisibly");
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
                max_attempts: 1,
            },
        )
        .await
        .expect("recognized import job enqueues");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("bounded lease"),
        )
        .await
        .expect("recognized import job claims")
        .expect("recognized import job is available");

    let commit = CommitPreparedQtiImport {
        job,
        lease: claim.lease_token,
        reference: command.registry.reference,
        source_object: command.registry.source.id,
    };
    assert_eq!(
        store
            .commit_prepared_qti_import(context, commit)
            .await
            .expect("incomplete recognized evidence is refused without exposure"),
        CommitPreparedQtiImportOutcome::ClaimNoLongerActive
    );
    assert_eq!(
        store
            .get_qti_import(context, workspace, import)
            .await
            .expect("incomplete recognized import lookup"),
        None
    );

    let report_mismatch = QtiProfileImportEvidence::new(
        command.registry.reference,
        "item-1".to_string(),
        PersistedFlatImportProfile::CanvasQti12V1,
        FlatImportIntegrityDigests {
            profile_report_sha256: Sha256Digest::compute(b"different safe report"),
            normalized_item_sha256: Sha256Digest::compute(b"recognized profile normalized item"),
            public_mapping_sha256: Sha256Digest::compute(b"recognized profile public map"),
            private_mapping_sha256: Sha256Digest::compute(b"recognized profile private map"),
            mapping_sha256: Sha256Digest::compute(b"recognized profile map"),
            warning_sha256: Sha256Digest::compute(b"recognized profile warnings"),
            choice_map_sha256: Sha256Digest::compute(b"recognized profile choice map"),
        },
    )
    .expect("report mismatch remains structurally valid");
    assert!(matches!(
        store
            .stage_qti_profile_import_evidence(context, report_mismatch)
            .await,
        Err(StoreError::Conflict)
    ));
    let profile_mismatch = QtiProfileImportEvidence::new(
        command.registry.reference,
        "item-1".to_string(),
        PersistedFlatImportProfile::BlackboardQti21V1,
        FlatImportIntegrityDigests {
            normalized_item_sha256: Sha256Digest::compute(b"recognized profile normalized item"),
            profile_report_sha256: Sha256Digest::compute(b"canonical safe Canvas profile report"),
            public_mapping_sha256: Sha256Digest::compute(b"recognized profile public map"),
            private_mapping_sha256: Sha256Digest::compute(b"recognized profile private map"),
            mapping_sha256: Sha256Digest::compute(b"recognized profile map"),
            warning_sha256: Sha256Digest::compute(b"recognized profile warnings"),
            choice_map_sha256: Sha256Digest::compute(b"recognized profile choice map"),
        },
    )
    .expect("profile mismatch remains structurally valid");
    assert!(matches!(
        store
            .stage_qti_profile_import_evidence(context, profile_mismatch)
            .await,
        Err(StoreError::Conflict)
    ));
    store
        .stage_qti_profile_import_evidence(context, evidence)
        .await
        .expect("exact evidence stages");
    assert_eq!(
        store
            .commit_prepared_qti_import(context, commit)
            .await
            .expect("complete recognized evidence commits"),
        CommitPreparedQtiImportOutcome::Committed
    );
}

#[tokio::test]
async fn memory_recognized_qti_commit_allows_a_zero_accepted_item_report() {
    let tenant = TenantId::from_uuid(uuid(9_241));
    let workspace = WorkspaceId::from_uuid(uuid(9_242));
    let import = WorkspaceImportId::from_uuid(uuid(9_243));
    let context = TenantContext::from_authenticated_session(tenant);
    let (mut command, _) = recognized_profile_command(tenant, workspace, import);
    command.registry.items.clear();
    command.item_bindings.clear();
    let result = &mut command.registry.item_results[0];
    result.item_id = None;
    result.normalized_sha256 = None;
    result.status = QtiImportItemStatus::Rejected;
    result.diagnostics.push(QtiUnsupportedFeature {
        code: "unsupported-interaction".to_string(),
        location: "item-1".to_string(),
        detail: "The recognized profile cannot map this item.".to_string(),
    });

    let store = MemoryStore::default();
    save_workspace(&store, tenant, workspace, UserId::from_uuid(uuid(9_244))).await;
    store
        .prepare_qti_import(context, command.clone())
        .await
        .expect("recognized all-rejected import prepares");
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
                max_attempts: 1,
            },
        )
        .await
        .expect("recognized all-rejected import job enqueues");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("bounded lease"),
        )
        .await
        .expect("recognized all-rejected import job claims")
        .expect("recognized all-rejected import job is available");

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
            .expect("zero accepted items need no private item evidence"),
        CommitPreparedQtiImportOutcome::Committed
    );
}

async fn exercise_qti_import_store<S, G>(store: &S, grader: &G)
where
    S: Store + QtiImportStore + JobStore,
    G: QtiGradingStore,
{
    let tenant = TenantId::from_uuid(uuid(9_001));
    let foreign = TenantId::from_uuid(uuid(9_002));
    let workspace = WorkspaceId::from_uuid(uuid(9_003));
    let other_workspace = WorkspaceId::from_uuid(uuid(9_004));
    let import = WorkspaceImportId::from_uuid(uuid(9_005));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign);
    save_workspace(store, tenant, workspace, UserId::from_uuid(uuid(9_008))).await;
    let command = qti_import_command(tenant, workspace, import);
    store
        .prepare_qti_import(context, command.clone())
        .await
        .expect("private registry prepares");
    let direct_job = store
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
        .expect("QTI job enqueue");
    let direct_claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("lease"),
        )
        .await
        .expect("QTI job claim")
        .expect("QTI job ready");
    assert_eq!(direct_claim.id, direct_job);
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job: direct_job,
                    lease: direct_claim.lease_token,
                    reference: command.registry.reference,
                    source_object: command.registry.source.id,
                },
            )
            .await
            .expect("exact initial QTI commit"),
        CommitPreparedQtiImportOutcome::Committed
    );
    let registry = store
        .get_qti_import(context, workspace, import)
        .await
        .expect("owner lookup")
        .expect("registry exists");
    assert_eq!(registry, command.registry);
    assert_eq!(
        store
            .get_qti_import(foreign_context, workspace, import)
            .await
            .expect("foreign lookup"),
        None
    );
    assert_eq!(
        store
            .get_qti_import(context, other_workspace, import)
            .await
            .expect("foreign workspace lookup"),
        None
    );
    let grading = grader
        .qti_import_grading(context, workspace, import, "item-1")
        .await
        .expect("grader lookup")
        .expect("only injected grader handle reads the private binding");
    assert_eq!(grading.sha256(), Sha256Digest::compute(b"correct-choice=2"));
    assert_eq!(
        grader
            .qti_import_grading(foreign_context, workspace, import, "item-1")
            .await
            .expect("foreign grading lookup"),
        None
    );
    assert!(matches!(
        store.prepare_qti_import(context, command).await,
        Err(StoreError::Conflict)
    ));

    let bad_import = WorkspaceImportId::from_uuid(uuid(9_006));
    let mut invalid = qti_import_command(tenant, workspace, bad_import);
    invalid.registry.assets.clear();
    assert!(matches!(
        store.prepare_qti_import(context, invalid).await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .get_qti_import(context, workspace, bad_import)
            .await
            .expect("failed creation lookup"),
        None
    );

    // Preparation is deliberately invisible, including to the dedicated
    // grader, until the exact durable QTI claim is committed.
    let staged_import = WorkspaceImportId::from_uuid(uuid(9_007));
    let staged = qti_import_command(tenant, workspace, staged_import);
    let source_object = staged.registry.source.id;
    store
        .prepare_qti_import(context, staged.clone())
        .await
        .expect("hidden QTI registry prepares atomically");
    store
        .prepare_qti_import(context, staged.clone())
        .await
        .expect("replayed QTI preparation is idempotent by import");
    let mut divergent_retry = staged.clone();
    divergent_retry.item_bindings[0].grading =
        QtiImportGradingPayload::new(b"a different server-only correct choice".to_vec())
            .expect("bounded divergent grading fixture");
    assert!(matches!(
        store.prepare_qti_import(context, divergent_retry).await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store
            .get_qti_import(context, workspace, staged_import)
            .await
            .expect("prepared registry lookup"),
        None
    );
    assert_eq!(
        grader
            .qti_import_grading(context, workspace, staged_import, "item-1")
            .await
            .expect("prepared grading lookup"),
        None
    );
    let job = store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::QtiImport {
                    workspace,
                    import: staged_import,
                    source_object,
                },
                max_attempts: 2,
            },
        )
        .await
        .expect("QTI job enqueue");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("lease"),
        )
        .await
        .expect("QTI job claim")
        .expect("QTI job is ready");
    assert_eq!(claim.id, job);
    let wrong_source = ObjectId::from_uuid(uuid(9_099));
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: staged.registry.reference,
                    source_object: wrong_source,
                },
            )
            .await
            .expect("wrong QTI source must be safely refused"),
        CommitPreparedQtiImportOutcome::ClaimNoLongerActive
    );
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: staged.registry.reference,
                    source_object,
                },
            )
            .await
            .expect("exact claim atomically exposes prepared QTI import"),
        CommitPreparedQtiImportOutcome::Committed
    );
    assert_eq!(
        store
            .get_qti_import(context, workspace, staged_import)
            .await
            .expect("committed registry lookup"),
        Some(staged.registry)
    );
    assert!(
        grader
            .qti_import_grading(context, workspace, staged_import, "item-1")
            .await
            .expect("committed grading lookup")
            .is_some()
    );
}

#[tokio::test]
async fn memory_qti_import_registry_is_private_complete_and_secret_redacted() {
    let (store, grader) = MemoryStore::with_qti_grader();
    exercise_qti_import_store(&store, &grader).await;
    let redacted = format!(
        "{:?}",
        QtiImportGradingPayload::new(b"never-in-debug".to_vec()).expect("fixture payload")
    );
    assert!(!redacted.contains("never-in-debug"));
}

async fn exercise_qti_publication_grading_visibility<S, G>(store: &S, grader: &G)
where
    S: Store + CatalogStore + AssetStore + QtiImportStore + JobStore,
    G: QtiGradingStore,
{
    let tenant = TenantId::from_uuid(uuid(9_100));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(uuid(9_101));
    let workspace = WorkspaceId::from_uuid(uuid(9_102));
    let import = WorkspaceImportId::from_uuid(uuid(9_103));
    let command = qti_import_command(tenant, workspace, import);
    let initial_draft = save_workspace(store, tenant, workspace, publisher).await;
    store
        .prepare_qti_import(context, command.clone())
        .await
        .expect("private QTI staging prepares");
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
        .expect("QTI promotion fixture job enqueues");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("bounded lease"),
        )
        .await
        .expect("QTI promotion fixture job claims")
        .expect("queued fixture is available");
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
            .expect("committed private staging"),
        CommitPreparedQtiImportOutcome::Committed
    );

    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(9_104)),
        version: VersionId::from_uuid(uuid(9_105)),
    };
    let mut draft_question = draft_question(workspace);
    draft_question.source = DraftQuestionSource::Qti {
        item_id: "item-1".to_string(),
        import_id: import,
    };
    let draft = DraftRecord {
        tenant,
        question: draft_question,
        derived_from: None,
    };
    let saved_draft = store
        .upsert_draft(
            context,
            publisher,
            Some(initial_draft.revision),
            draft.clone(),
        )
        .await
        .expect("QTI draft saves before promotion");
    let mut artifact = source_artifact(
        reference,
        QuestionBackend::Qti,
        ObjectId::from_uuid(uuid(9_106)),
    );
    artifact.object.sha256 = command.registry.source.sha256;
    artifact.object.size_bytes = command.registry.source.size_bytes;
    artifact.object.media_type = command.registry.source.media_type.clone();
    let staged_asset = &command.registry.assets[0];
    let logical_asset = command.registry.items[0].assets[0];
    let object = ObjectId::from_uuid(uuid(9_107));
    let asset_key = ObjectKey::ProblemAsset {
        problem: reference.problem,
        version: reference.version,
        asset: logical_asset,
        object,
    };
    let delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(logical_asset),
        object: ObjectRecord {
            id: object,
            bucket: asset_key.bucket(),
            key: asset_key,
            sha256: staged_asset.sha256,
            size_bytes: staged_asset.size_bytes,
            media_type: staged_asset.media_type.clone(),
            category: objects::ObjectCategory::Asset,
            version: Some(reference.version),
            license: staged_asset.license.clone(),
            provenance: "published QTI asset fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(2),
        },
        intrinsic_width: None,
        intrinsic_height: None,
        scope: AssetDeliveryScope::Catalog {
            asset: logical_asset,
            reference,
        },
        publication: AssetPublication::Ready,
        pending_source: None,
    };
    let collaborator = UserId::from_uuid(uuid(9_109));
    store
        .grant_draft_collaborator(context, publisher, workspace, collaborator)
        .await
        .expect("owner grants the QTI collaborator");
    let mut intervening_draft = draft.clone();
    intervening_draft.question.metadata.title = "Intervening workspace save".to_string();
    let intervening_save = store
        .upsert_draft(
            context,
            collaborator,
            Some(saved_draft.revision),
            intervening_draft,
        )
        .await
        .expect("collaborator save advances the workspace revision");
    let reverted_save = store
        .upsert_draft(
            context,
            publisher,
            Some(intervening_save.revision),
            draft.clone(),
        )
        .await
        .expect("owner may restore content while advancing the revision");
    let promotion = QtiPublicationPromotion {
        staging: command.registry.reference,
        assets: vec![delivery],
    };
    assert_eq!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft.clone(),
                    expected_revision: saved_draft.revision,
                    publication: reference,
                    published_source: QuestionSource::Qti {
                        item_id: "item-1".to_string(),
                        package_object: artifact.object.id,
                        package_sha256: artifact.object.sha256.to_string(),
                    },
                    source_artifact: Some(artifact.clone()),
                    qti_promotion: Some(promotion.clone()),
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    byline: reviewed_byline(),
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a content-equivalent reverted draft cannot publish with its stale revision"
    );
    assert_eq!(
        store.get_catalog_problem(context, reference).await,
        Ok(None),
        "a stale revision cannot mint the catalog version"
    );
    assert!(
        grader
            .qti_publication_grading(context, reference, "item-1")
            .await
            .expect("stale QTI grading lookup")
            .is_none(),
        "a stale publication must leave private QTI staging unconsumed"
    );
    let published = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: reverted_save.revision,
                publication: reference,
                published_source: QuestionSource::Qti {
                    item_id: "item-1".to_string(),
                    package_object: artifact.object.id,
                    package_sha256: artifact.object.sha256.to_string(),
                },
                source_artifact: Some(artifact),
                qti_promotion: Some(promotion),
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                byline: reviewed_byline(),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("exact committed QTI staging atomically publishes");
    assert_eq!(published.problem, reference.problem);
    assert!(
        grader
            .qti_publication_grading(context, reference, "item-1")
            .await
            .expect("published grading read")
            .is_some(),
        "the grader receives the copied server-only binding"
    );
    assert!(
        grader
            .qti_publication_grading(
                TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(9_108))),
                reference,
                "item-1",
            )
            .await
            .expect("public published grading read")
            .is_some(),
        "public QTI content remains gradeable from another tenant"
    );
    assert_eq!(
        store.catalog_asset_bindings(context, reference).await,
        Ok(vec![learning_data_access::CatalogAssetBinding {
            asset: logical_asset,
            object,
            key: ObjectKey::ProblemAsset {
                problem: reference.problem,
                version: reference.version,
                asset: logical_asset,
                object,
            },
            rendition_checksum: staged_asset.sha256,
            media_type: staged_asset.media_type.clone(),
            intrinsic_width: None,
            intrinsic_height: None,
        }])
    );
}

#[tokio::test]
async fn memory_qti_publication_time_grading_respects_catalog_visibility() {
    let (store, grader) = MemoryStore::with_qti_grader();
    exercise_qti_publication_grading_visibility(&store, &grader).await;
}
