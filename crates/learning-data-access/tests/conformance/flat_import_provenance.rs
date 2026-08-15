use super::*;
use learning_data_access::{
    FlatImportChoiceMapPayload, FlatImportConversionVersion, FlatImportIntegrityDigests,
    FlatImportProvenanceStore, FlatImportPublicationPromotion, FlatQuestionGradingPayload,
    FlatQuestionGradingStore, FlatQuestionPublicationPromotion, FlatQuestionStore,
    PersistedFlatImportProfile, QtiProfileFlatConversionCommand, QtiProfileImportEvidence,
    WorkspaceFlatImportOrigin, WorkspaceFlatQuestionSource,
};
use objects::published_import_archive_object_id;

const CONVERSION_VERSION: &str = "ple-qti-profile-flat-conversion/v1";
const FLAT_QUESTION_MEDIA_TYPE: &str = "application/vnd.peptidyle.flat-question+json";

struct ConversionFixture {
    tenant: TenantId,
    context: TenantContext,
    owner: UserId,
    workspace: WorkspaceId,
    import: WorkspaceImportId,
    draft: DraftRecord,
    source: ObjectRecord,
    grading: FlatQuestionGradingPayload,
    origin: WorkspaceFlatImportOrigin,
    import_command: CreateQtiImportCommand,
}

struct WorkspaceSnapshot {
    draft: Option<learning_data_access::WorkspaceDraft>,
    source: Option<WorkspaceFlatQuestionSource>,
    origin: Option<WorkspaceFlatImportOrigin>,
}

impl ConversionFixture {
    fn new(seed: u128, title: &str) -> Self {
        let tenant = TenantId::from_uuid(uuid(86_000));
        let workspace = WorkspaceId::from_uuid(uuid(86_001));
        let owner = UserId::from_uuid(uuid(86_002));
        let import = WorkspaceImportId::from_uuid(uuid(seed));
        let context = TenantContext::from_authenticated_session(tenant);
        let source_text = format!(
            r#"{{"format":"pleFlatQuestion","version":2,"title":"{title}","prompt":"Which choice is blue?","response":{{"kind":"singleChoice","choices":[{{"id":"blue","text":"Blue"}},{{"id":"red","text":"Red"}}],"correctChoice":"blue"}},"points":1.0,"attemptPolicy":{{"maxAttempts":null,"feedback":"immediateFull"}},"timingPolicy":{{"kind":"untimed"}},"license":{{"kind":"allRightsReserved"}},"language":"en-US"}}"#
        );
        let document =
            adapter_native::flat_question::FlatQuestionDocument::parse(source_text.as_bytes())
                .expect("flat-import conformance fixture parses");
        let canonical = document
            .canonical_bytes()
            .expect("flat-import conformance fixture canonicalizes");
        let (question, private) = document
            .compile(workspace)
            .expect("flat-import conformance fixture compiles")
            .into_parts();
        let draft = DraftRecord {
            tenant,
            question,
            derived_from: None,
        };
        let grading = FlatQuestionGradingPayload::from_private(&private)
            .expect("flat-import private grading fixture is valid");
        let source = source_record(tenant, workspace, seed + 1, &canonical);
        let archive = archive_record(
            tenant,
            workspace,
            import,
            seed + 2,
            b"PK\x03\x04profile-fixture",
        );
        let choice_map = FlatImportChoiceMapPayload::from_canonical_bytes(
            br#"{"schema":"ple-qti-private-choice-map/v1","choices":[["vendor-blue","blue"],["vendor-red","red"]]}"#.to_vec(),
        )
        .expect("bounded private choice map");
        let digests = FlatImportIntegrityDigests {
            normalized_item_sha256: Sha256Digest::compute(b"normalized mapped item"),
            profile_report_sha256: Sha256Digest::compute(b"profile report"),
            public_mapping_sha256: Sha256Digest::compute(b"public mapping"),
            private_mapping_sha256: Sha256Digest::compute(b"private mapping"),
            mapping_sha256: Sha256Digest::compute(b"combined mapping"),
            warning_sha256: Sha256Digest::compute(b"author-visible warnings"),
            choice_map_sha256: choice_map.sha256(),
        };
        let origin = WorkspaceFlatImportOrigin::new(
            QtiImportRef {
                tenant,
                workspace,
                import,
            },
            format!("canvas-item-{seed}"),
            PersistedFlatImportProfile::CanvasQti12V1,
            FlatImportConversionVersion::new(CONVERSION_VERSION).expect("fixture version"),
            archive.clone(),
            digests,
            source.sha256,
            owner,
            ActivityTimestamp::from_unix_millis(86_003),
            choice_map,
        )
        .expect("fixture origin is valid");
        let item = QtiImportItem {
            item_id: origin.source_item_identifier().to_string(),
            model_sha256: Sha256Digest::compute(b"answer-free imported item model"),
            assets: Vec::new(),
        };
        let import_command = CreateQtiImportCommand {
            registry: QtiImportRegistry {
                reference: origin.import(),
                source: archive,
                source_format: "qti".to_string(),
                source_identifier: Some("profile-fixture.zip".to_string()),
                importer: "adapter_qti".to_string(),
                parse_schema: "canvas-qti-1.2".to_string(),
                adapter_version: "v1".to_string(),
                profile_summary: None,
                items: vec![item.clone()],
                item_results: vec![QtiImportItemResult {
                    source_identifier: item.item_id.clone(),
                    title: Some("Imported profile item".to_string()),
                    item_id: Some(item.item_id.clone()),
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
                grading: QtiImportGradingPayload::new(b"private profile import grading".to_vec())
                    .expect("bounded QTI grading fixture"),
            }],
        };
        Self {
            tenant,
            context,
            owner,
            workspace,
            import,
            draft,
            source,
            grading,
            origin,
            import_command,
        }
    }

    fn conversion(
        &self,
        expected_revision: Option<learning_data_access::WorkspaceDraftRevision>,
    ) -> QtiProfileFlatConversionCommand {
        QtiProfileFlatConversionCommand::new(
            expected_revision,
            self.draft.clone(),
            self.source.clone(),
            self.source.sha256.to_string(),
            self.grading.public_binding_sha256().to_string(),
            self.grading.clone(),
            self.origin.clone(),
        )
        .expect("fixture conversion command is valid")
    }

    async fn save_workspace(&self, store: &MemoryStore) -> learning_data_access::WorkspaceDraft {
        if let Some(workspace) = store
            .get_draft(self.context, self.owner, self.workspace)
            .await
            .expect("workspace lookup succeeds")
        {
            return workspace;
        }
        store
            .upsert_draft(self.context, self.owner, None, self.draft.clone())
            .await
            .expect("profile-import workspace saves")
    }

    fn profile_evidence(&self) -> QtiProfileImportEvidence {
        QtiProfileImportEvidence::new(
            self.origin.import(),
            self.origin.source_item_identifier().to_string(),
            self.origin.profile(),
            self.origin.digests(),
        )
        .expect("fixture profile evidence is valid")
    }

    fn divergent_profile_evidence(&self) -> QtiProfileImportEvidence {
        QtiProfileImportEvidence::new(
            self.origin.import(),
            self.origin.source_item_identifier().to_string(),
            PersistedFlatImportProfile::BlackboardQti21V1,
            self.origin.digests(),
        )
        .expect("divergent profile evidence remains structurally valid")
    }

    fn divergent_digest_evidence(&self) -> QtiProfileImportEvidence {
        let mut digests = self.origin.digests();
        digests.warning_sha256 = Sha256Digest::compute(b"divergent author-visible warnings");
        QtiProfileImportEvidence::new(
            self.origin.import(),
            self.origin.source_item_identifier().to_string(),
            self.origin.profile(),
            digests,
        )
        .expect("divergent digest evidence remains structurally valid")
    }

    fn published_archive(&self, reference: ProblemVersionRef) -> ObjectRecord {
        let object = published_import_archive_object_id(
            self.tenant,
            reference.problem,
            reference.version,
            self.import,
            self.origin.source_archive().sha256,
        );
        let key = ObjectKey::PublishedImportArchive {
            tenant: self.tenant,
            problem: reference.problem,
            version: reference.version,
            import: self.import,
            object,
        };
        ObjectRecord {
            id: object,
            bucket: key.bucket(),
            key,
            sha256: self.origin.source_archive().sha256,
            size_bytes: self.origin.source_archive().size_bytes,
            media_type: "application/zip".to_string(),
            category: ObjectCategory::Source,
            version: Some(reference.version),
            license: "allRightsReserved".to_string(),
            provenance: "published flat-import conformance fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(86_004),
        }
    }
}

fn source_record(
    tenant: TenantId,
    workspace: WorkspaceId,
    seed: u128,
    bytes: &[u8],
) -> ObjectRecord {
    let object = ObjectId::from_uuid(uuid(seed));
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
        media_type: FLAT_QUESTION_MEDIA_TYPE.to_string(),
        category: ObjectCategory::Source,
        version: None,
        license: "allRightsReserved".to_string(),
        provenance: "flat-import conformance fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(86_005),
    }
}

fn archive_record(
    tenant: TenantId,
    workspace: WorkspaceId,
    import: WorkspaceImportId,
    seed: u128,
    bytes: &[u8],
) -> ObjectRecord {
    let object = ObjectId::from_uuid(uuid(seed));
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
        size_bytes: u64::try_from(bytes.len()).expect("fixture archive size fits"),
        media_type: "application/zip".to_string(),
        category: ObjectCategory::Source,
        version: None,
        license: "allRightsReserved".to_string(),
        provenance: "QTI profile import conformance fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(86_006),
    }
}

async fn commit_profile_import(
    store: &MemoryStore,
    fixture: &ConversionFixture,
) -> learning_data_access::WorkspaceDraft {
    let workspace = fixture.save_workspace(store).await;
    let mut import_command = fixture.import_command.clone();
    let profile_defaults = [
        "PLE default applied: unlimited attempts.",
        "PLE default applied: immediate full feedback.",
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
    .collect();
    let profile = fixture.origin.profile();
    let summary = learning_data_access::QtiImportProfileSummary::new(
        profile,
        fixture.origin.digests().profile_report_sha256,
        profile_defaults,
    )
    .expect("recognized profile summary is valid");
    import_command.registry.parse_schema = summary.profile_id().to_string();
    import_command.registry.profile_summary = Some(summary);
    let model_sha256 = fixture.import_command.registry.items[0].model_sha256;
    let normalized_sha256 = fixture.import_command.registry.item_results[0]
        .normalized_sha256
        .expect("accepted fixture result has a normalized digest");
    assert_ne!(
        model_sha256, normalized_sha256,
        "answer-free model and grading-bound normalized digests stay distinct"
    );
    store
        .prepare_qti_import(fixture.context, import_command.clone())
        .await
        .expect("profile import prepares");
    store
        .stage_qti_profile_import_evidence(fixture.context, fixture.profile_evidence())
        .await
        .expect("profile evidence stages before import commit");
    store
        .stage_qti_profile_import_evidence(fixture.context, fixture.profile_evidence())
        .await
        .expect("exact profile-evidence replay is idempotent");
    assert!(
        store
            .stage_qti_profile_import_evidence(
                fixture.context,
                fixture.divergent_profile_evidence(),
            )
            .await
            .is_err(),
        "a divergent profile cannot replace prepared evidence"
    );
    assert!(
        store
            .stage_qti_profile_import_evidence(fixture.context, fixture.divergent_digest_evidence())
            .await
            .is_err(),
        "a divergent digest cannot replace prepared evidence"
    );
    let job = store
        .enqueue_job(
            fixture.context,
            EnqueueJob {
                tenant: fixture.tenant,
                payload: JobPayload::QtiImport {
                    workspace: fixture.workspace,
                    import: fixture.import,
                    source_object: import_command.registry.source.id,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("profile import job enqueues");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("bounded lease"),
        )
        .await
        .expect("profile import job claims")
        .expect("profile import job is available");
    assert_eq!(claim.id, job);
    assert_eq!(
        store
            .commit_prepared_qti_import(
                fixture.context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: import_command.registry.reference,
                    source_object: import_command.registry.source.id,
                },
            )
            .await
            .expect("profile import commits"),
        CommitPreparedQtiImportOutcome::Committed
    );
    workspace
}

async fn snapshot_workspace(store: &MemoryStore, fixture: &ConversionFixture) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        draft: store
            .get_draft(fixture.context, fixture.owner, fixture.workspace)
            .await
            .expect("owner draft lookup"),
        source: store
            .flat_question_source(fixture.context, fixture.owner, fixture.workspace)
            .await
            .expect("owner source lookup"),
        origin: store
            .workspace_flat_import_origin(fixture.context, fixture.owner, fixture.workspace)
            .await
            .expect("owner origin lookup"),
    }
}

async fn assert_workspace_unchanged(
    store: &MemoryStore,
    fixture: &ConversionFixture,
    before: &WorkspaceSnapshot,
) {
    let after = snapshot_workspace(store, fixture).await;
    assert!(
        after.draft == before.draft
            && after.source == before.source
            && after.origin == before.origin,
        "a refused conversion leaves draft, source, and private origin unchanged"
    );
}

async fn convert_fixture(
    store: &MemoryStore,
    fixture: &ConversionFixture,
) -> WorkspaceFlatQuestionSource {
    let workspace = commit_profile_import(store, fixture).await;
    store
        .convert_qti_profile_item_to_flat(
            fixture.context,
            fixture.owner,
            fixture.conversion(Some(workspace.revision)),
        )
        .await
        .expect("committed profile evidence converts atomically")
}

fn published_source_artifact(
    reference: ProblemVersionRef,
    source: &ObjectRecord,
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
            key,
            sha256: source.sha256,
            size_bytes: source.size_bytes,
            media_type: source.media_type.clone(),
            category: ObjectCategory::Source,
            version: Some(reference.version),
            license: source.license.clone(),
            provenance: "published flat-import source fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(86_007),
        },
    }
}

fn publication_command(
    fixture: &ConversionFixture,
    staged: WorkspaceFlatQuestionSource,
    reference: ProblemVersionRef,
    import_origin: Option<FlatImportPublicationPromotion>,
) -> PublishDraftCommand {
    PublishDraftCommand {
        expected_draft: fixture.draft.clone(),
        expected_revision: staged.workspace_revision,
        publication: reference,
        published_source: QuestionSource::Native {
            family: "flat_single_choice_v2".to_string(),
        },
        source_artifact: Some(published_source_artifact(
            reference,
            &fixture.source,
            ObjectId::from_uuid(uuid(86_008)),
        )),
        qti_promotion: None,
        flat_question_promotion: Some(FlatQuestionPublicationPromotion {
            source: staged,
            import_origin,
            published_question: fixture.draft.question.clone(),
            assets: Vec::new(),
        }),
        publisher: fixture.owner,
        scope: PublicationScope::Public,
        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
    }
}

#[tokio::test]
async fn memory_flat_import_conversion_commits_source_grading_and_origin_together() {
    let (store, _) = MemoryStore::with_flat_question_grader();
    let fixture = ConversionFixture::new(86_010, "Imported favorite color");
    let staged = convert_fixture(&store, &fixture).await;

    assert_eq!(staged.source_record, fixture.source);
    assert_eq!(
        store
            .get_draft(fixture.context, fixture.owner, fixture.workspace)
            .await
            .expect("owner draft lookup")
            .expect("conversion creates a draft")
            .record,
        fixture.draft
    );
    assert_eq!(
        store
            .flat_question_source(fixture.context, fixture.owner, fixture.workspace)
            .await
            .expect("owner source lookup"),
        Some(staged)
    );
    assert!(
        store
            .workspace_flat_import_origin(fixture.context, fixture.owner, fixture.workspace)
            .await
            .expect("owner origin lookup")
            == Some(fixture.origin.clone()),
        "conversion installs the exact private origin"
    );
}

#[tokio::test]
async fn memory_generic_qti_import_cannot_stage_or_convert_profile_evidence() {
    let (store, _) = MemoryStore::with_flat_question_grader();
    let fixture = ConversionFixture::new(86_015, "Generic QTI is not profile-convertible");
    let workspace = fixture.save_workspace(&store).await;
    store
        .prepare_qti_import(fixture.context, fixture.import_command.clone())
        .await
        .expect("generic import prepares without profile evidence");
    assert!(matches!(
        store
            .stage_qti_profile_import_evidence(fixture.context, fixture.profile_evidence())
            .await,
        Err(StoreError::Conflict)
    ));
    let job = store
        .enqueue_job(
            fixture.context,
            EnqueueJob {
                tenant: fixture.tenant,
                payload: JobPayload::QtiImport {
                    workspace: fixture.workspace,
                    import: fixture.import,
                    source_object: fixture.import_command.registry.source.id,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("generic import job enqueues");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("bounded lease"),
        )
        .await
        .expect("generic import job claims")
        .expect("generic import job is available");
    assert_eq!(
        store
            .commit_prepared_qti_import(
                fixture.context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: fixture.import_command.registry.reference,
                    source_object: fixture.import_command.registry.source.id,
                },
            )
            .await
            .expect("generic import keeps its existing commit behavior"),
        CommitPreparedQtiImportOutcome::Committed
    );
    assert!(matches!(
        store
            .convert_qti_profile_item_to_flat(
                fixture.context,
                fixture.owner,
                fixture.conversion(Some(workspace.revision)),
            )
            .await,
        Err(StoreError::Conflict)
    ));
}

#[tokio::test]
async fn memory_flat_import_refusals_leave_workspace_state_unchanged() {
    let (store, grader) = MemoryStore::with_flat_question_grader();
    let fixture = ConversionFixture::new(86_020, "Refusal baseline");
    let staged = convert_fixture(&store, &fixture).await;
    let before = snapshot_workspace(&store, &fixture).await;

    assert!(
        store
            .convert_qti_profile_item_to_flat(
                fixture.context,
                fixture.owner,
                fixture.conversion(None)
            )
            .await
            .is_err(),
        "a stale conversion CAS refuses"
    );
    assert_workspace_unchanged(&store, &fixture, &before).await;

    let missing = ConversionFixture::new(86_021, "Missing committed import");
    assert!(
        store
            .convert_qti_profile_item_to_flat(
                fixture.context,
                fixture.owner,
                missing.conversion(Some(staged.workspace_revision)),
            )
            .await
            .is_err(),
        "an origin whose import was never committed refuses"
    );
    assert_workspace_unchanged(&store, &fixture, &before).await;

    let mut changed = ConversionFixture::new(86_022, "Changed committed evidence");
    let changed_workspace = commit_profile_import(&store, &changed).await;
    let altered_archive = archive_record(
        changed.tenant,
        changed.workspace,
        changed.import,
        86_024,
        b"PK\x03\x04different archive bytes",
    );
    let map = FlatImportChoiceMapPayload::from_canonical_bytes(b"changed private map".to_vec())
        .expect("bounded changed choice map");
    let mut digests = changed.origin.digests();
    digests.choice_map_sha256 = map.sha256();
    changed.origin = WorkspaceFlatImportOrigin::new(
        changed.origin.import(),
        changed.origin.source_item_identifier().to_string(),
        changed.origin.profile(),
        changed.origin.conversion_version().clone(),
        altered_archive,
        digests,
        changed.source.sha256,
        changed.owner,
        changed.origin.acknowledged_at(),
        map,
    )
    .expect("changed-evidence origin remains structurally valid");
    assert!(
        store
            .convert_qti_profile_item_to_flat(
                fixture.context,
                fixture.owner,
                changed.conversion(Some(changed_workspace.revision)),
            )
            .await
            .is_err(),
        "changed archive evidence refuses after the committed-import check"
    );
    assert_workspace_unchanged(&store, &fixture, &before).await;

    let foreign_context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(86_023)));
    assert!(
        store
            .convert_qti_profile_item_to_flat(
                foreign_context,
                fixture.owner,
                fixture.conversion(Some(staged.workspace_revision))
            )
            .await
            .is_err(),
        "a foreign tenant cannot convert a private workspace"
    );
    assert_workspace_unchanged(&store, &fixture, &before).await;

    // The private workspace grading payload has no read API. Publishing the
    // untouched staging through the dedicated grader proves every refusal left
    // its original private binding in place without widening that boundary.
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(86_025)),
        version: VersionId::from_uuid(uuid(86_026)),
    };
    let current = before
        .origin
        .as_ref()
        .expect("successful baseline conversion has a current origin");
    let promotion = FlatImportPublicationPromotion::new(
        current,
        reference,
        fixture.published_archive(reference),
    )
    .expect("matching publication selector is valid");
    store
        .publish_draft(
            fixture.context,
            fixture.owner,
            publication_command(&fixture, staged, reference, Some(promotion)),
        )
        .await
        .expect("unchanged staging still publishes with its original private grading");
    assert_eq!(
        grader
            .flat_question_published_grading(fixture.context, reference)
            .await
            .expect("published grading lookup"),
        Some(fixture.grading.clone()),
        "refused conversions do not replace the private grading payload"
    );
}

#[tokio::test]
async fn memory_flat_import_ordinary_flat_save_preserves_origin_and_replacement_switches_it() {
    let store = MemoryStore::default();
    let first = ConversionFixture::new(86_030, "Original imported question");
    let staged = convert_fixture(&store, &first).await;
    let original = store
        .workspace_flat_import_origin(first.context, first.owner, first.workspace)
        .await
        .expect("origin lookup")
        .expect("conversion creates current origin");

    let edited = ConversionFixture::new(86_031, "Ordinary author edit");
    let saved = store
        .upsert_flat_question(
            first.context,
            first.owner,
            learning_data_access::UpsertFlatQuestionCommand {
                expected_revision: Some(staged.workspace_revision),
                draft: edited.draft.clone(),
                source: edited.source.clone(),
                canonical_source_sha256: edited.source.sha256.to_string(),
                public_binding_sha256: edited.grading.public_binding_sha256().to_string(),
                grading: edited.grading.clone(),
            },
        )
        .await
        .expect("ordinary flat editor save succeeds");
    assert!(
        store
            .workspace_flat_import_origin(first.context, first.owner, first.workspace)
            .await
            .expect("origin lookup after ordinary save")
            == Some(original.clone()),
        "ordinary flat editing retains the opaque origin byte-for-byte"
    );

    let replacement = ConversionFixture::new(86_032, "Replacement imported question");
    commit_profile_import(&store, &replacement).await;
    let replaced = store
        .convert_qti_profile_item_to_flat(
            first.context,
            first.owner,
            replacement.conversion(Some(saved.workspace_revision)),
        )
        .await
        .expect("a matching replacement converts atomically");
    assert_eq!(replaced.source_record, replacement.source);
    assert!(
        store
            .workspace_flat_import_origin(first.context, first.owner, first.workspace)
            .await
            .expect("origin lookup after replacement")
            == Some(replacement.origin),
        "replacement installs exactly the new current origin"
    );
}

#[tokio::test]
async fn memory_flat_import_edit_preserves_origin_and_publishes_new_stored_grading() {
    let (store, grader) = MemoryStore::with_flat_question_grader();
    let imported = ConversionFixture::new(86_033, "Original imported question");
    let staged = convert_fixture(&store, &imported).await;
    let current_origin = store
        .workspace_flat_import_origin(imported.context, imported.owner, imported.workspace)
        .await
        .expect("origin lookup")
        .expect("conversion creates current origin");
    let edited = ConversionFixture::new(86_034, "Edited imported question");
    let saved = store
        .upsert_flat_question(
            imported.context,
            imported.owner,
            learning_data_access::UpsertFlatQuestionCommand {
                expected_revision: Some(staged.workspace_revision),
                draft: edited.draft.clone(),
                source: edited.source.clone(),
                canonical_source_sha256: edited.source.sha256.to_string(),
                public_binding_sha256: edited.grading.public_binding_sha256().to_string(),
                grading: edited.grading.clone(),
            },
        )
        .await
        .expect("ordinary edit should replace current grading");
    assert!(
        store
            .workspace_flat_import_origin(imported.context, imported.owner, imported.workspace)
            .await
            .expect("origin lookup after edit")
            == Some(current_origin.clone()),
        "ordinary editing preserves the imported origin"
    );

    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(86_035)),
        version: VersionId::from_uuid(uuid(86_036)),
    };
    let promotion = FlatImportPublicationPromotion::new(
        &current_origin,
        reference,
        imported.published_archive(reference),
    )
    .expect("preserved origin should produce a publication selector");
    store
        .publish_draft(
            imported.context,
            imported.owner,
            publication_command(&edited, saved, reference, Some(promotion)),
        )
        .await
        .expect("edited imported question should publish");
    assert_eq!(
        grader
            .flat_question_published_grading(imported.context, reference)
            .await
            .expect("published grading lookup"),
        Some(edited.grading),
        "publication copies the grading staged by the edit, not the imported original"
    );
}

#[tokio::test]
async fn memory_flat_import_publication_requires_matching_origin_and_cleans_workspace() {
    let (store, grader) = MemoryStore::with_flat_question_grader();
    let fixture = ConversionFixture::new(86_040, "Publication provenance");
    let staged = convert_fixture(&store, &fixture).await;
    let current = store
        .workspace_flat_import_origin(fixture.context, fixture.owner, fixture.workspace)
        .await
        .expect("owner origin lookup")
        .expect("converted workspace has an origin");
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(86_041)),
        version: VersionId::from_uuid(uuid(86_042)),
    };
    let before_omitted = snapshot_workspace(&store, &fixture).await;

    assert!(
        store
            .publish_draft(
                fixture.context,
                fixture.owner,
                publication_command(&fixture, staged.clone(), reference, None),
            )
            .await
            .is_err(),
        "a converted draft cannot omit its locked current-origin selector"
    );
    assert_workspace_unchanged(&store, &fixture, &before_omitted).await;

    let mismatched = ConversionFixture::new(86_043, "Mismatched selector");
    let mismatched_promotion = FlatImportPublicationPromotion::new(
        &mismatched.origin,
        reference,
        mismatched.published_archive(reference),
    )
    .expect("structurally valid mismatched selector");
    let before = snapshot_workspace(&store, &fixture).await;
    assert!(
        store
            .publish_draft(
                fixture.context,
                fixture.owner,
                publication_command(
                    &fixture,
                    staged.clone(),
                    reference,
                    Some(mismatched_promotion),
                ),
            )
            .await
            .is_err(),
        "a selector for another current origin refuses"
    );
    assert_workspace_unchanged(&store, &fixture, &before).await;

    let matching = FlatImportPublicationPromotion::new(
        &current,
        reference,
        fixture.published_archive(reference),
    )
    .expect("matching archive candidate is valid");
    let published = store
        .publish_draft(
            fixture.context,
            fixture.owner,
            publication_command(&fixture, staged, reference, Some(matching)),
        )
        .await
        .expect("matching origin promotion publishes atomically");
    assert_eq!(published.problem, reference.problem);
    assert_eq!(
        grader
            .flat_question_published_grading(fixture.context, reference)
            .await
            .expect("grader lookup after publication"),
        Some(fixture.grading.clone()),
        "publication copies the immutable server-only grading binding"
    );
    let after_publication = snapshot_workspace(&store, &fixture).await;
    assert!(
        after_publication.draft.is_none()
            && after_publication.source.is_none()
            && after_publication.origin.is_none(),
        "matching publication removes workspace staging and its current origin only"
    );

    let foreign = TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(86_044)));
    assert!(
        store
            .workspace_flat_import_origin(foreign, fixture.owner, fixture.workspace)
            .await
            .expect("foreign lineage lookup is non-enumerating")
            .is_none(),
        "published lineage has no browser-visible or foreign-tenant read path"
    );
}
