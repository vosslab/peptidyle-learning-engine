//! Private QTI staging worker.
//!
//! Archive and answer material cross this module only through server-owned
//! types.  Preparation writes immutable object bytes first, then a hidden
//! registry; the sealed committer is the only visibility transition.

use std::sync::Arc;

use adapter_qti::{QtiImporter, qti_question_asset_checksums};
use async_trait::async_trait;
use learning_data_access::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    FlatImportIntegrityDigests, FlatImportProvenanceStore, JobFailureKind, JobPayload,
    QtiImportGradingPayload, QtiImportItem, QtiImportItemRegistration, QtiImportItemResult,
    QtiImportItemStatus, QtiImportProfileSummary, QtiImportRef, QtiImportRegistry, QtiImportStore,
    QtiProfileImportEvidence, QtiUnsupportedFeature, StoreError, TenantContext,
};
use objects::{ObjectCategory, ObjectKey, ObjectStore, ObjectStoreError, PutObject, Sha256Digest};
use question_model::{AssetId, ObjectId, response::ChoiceId};
use sha2::{Digest, Sha256};

use crate::worker::{
    self, EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
    PreparedJobEffect,
};

pub(crate) mod profile;
use profile::prepare_qti_profile_package;

/// Worker implementation for the closed QTI queue payload.
pub(crate) struct QtiImportHandler<S, O> {
    store: Arc<S>,
    objects: Arc<O>,
}

impl<S, O> QtiImportHandler<S, O> {
    pub(crate) fn new(store: Arc<S>, objects: Arc<O>) -> Self {
        Self { store, objects }
    }
}

fn object_failure(error: ObjectStoreError) -> JobFailureKind {
    match error {
        ObjectStoreError::Unavailable(_) => JobFailureKind::Transient,
        ObjectStoreError::NotFound | ObjectStoreError::ChecksumMismatch => {
            JobFailureKind::Permanent
        }
        ObjectStoreError::AlreadyExists
        | ObjectStoreError::NotSignable
        | ObjectStoreError::NumericOverflow => JobFailureKind::Permanent,
    }
}

/// Replays use the same immutable key, so a crash after object persistence is
/// an orphan reconciliation case rather than a duplicate asset publication.
fn staged_asset_object(import: question_model::WorkspaceImportId, asset: AssetId) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(b"ple:qti-workspace-asset:v1");
    hasher.update(import.as_uuid().as_bytes());
    hasher.update(asset.as_uuid().as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(uuid::Uuid::from_bytes(bytes))
}

#[async_trait]
impl<S, O> JobHandler for QtiImportHandler<S, O>
where
    S: FlatImportProvenanceStore + QtiImportStore + Send + Sync + 'static,
    O: ObjectStore + Send + Sync + 'static,
{
    async fn prepare(
        &self,
        context: TenantContext,
        payload: JobPayload,
        execution: JobExecution,
    ) -> Result<PreparedJobEffect, JobFailureKind> {
        let JobPayload::QtiImport {
            workspace,
            import,
            source_object,
        } = payload
        else {
            return Err(JobFailureKind::Permanent);
        };
        let tenant = context.tenant_id();
        let source_key = ObjectKey::WorkspaceSource {
            tenant,
            workspace,
            import,
            object: source_object,
        };
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        let source = self
            .objects
            .get(&source_key)
            .await
            .map_err(object_failure)?;
        if source.record.id != source_object
            || source.record.key != source_key
            || source.record.category != ObjectCategory::Source
            || source.record.media_type != "application/zip"
            || source.record.sha256 != Sha256Digest::compute(&source.bytes)
            || source.record.size_bytes != source.bytes.len() as u64
        {
            return Err(JobFailureKind::Permanent);
        }
        if let Some(profile_package) =
            prepare_qti_profile_package(&source.bytes, adapter_qti::QtiImportLimits::default())
                .map_err(|_| JobFailureKind::Permanent)?
        {
            let reference = QtiImportRef {
                tenant,
                workspace,
                import,
            };
            let profile = profile_package.profile();
            let profile_summary = QtiImportProfileSummary::new(
                profile,
                profile_package.profile_report_sha256(),
                profile_package.package_defaults().to_vec(),
            )
            .map_err(|_| JobFailureKind::Permanent)?;
            let item_results = profile_package.item_results().to_vec();
            let mut item_bindings = Vec::new();
            let mut evidences = Vec::new();
            for item in profile_package.into_items() {
                let integrity = item.integrity();
                let parts = item.into_mapped_item().into_server_parts();
                if integrity.profile_report_sha256 != profile_summary.profile_report_sha256()
                    || parts.profile().as_str() != profile.profile_id()
                    || parts.profile_version().as_str() != profile.profile_version()
                    || parts.mapping_version().as_str() != profile.mapping_version()
                {
                    return Err(JobFailureKind::Permanent);
                }
                let source_identifier = parts.public_mapping().source_identifier.clone();
                let grading_choice = ChoiceId::new(parts.server_correct_ple_choice_id());
                let grading = QtiImportGradingPayload::new(
                    serde_json::to_vec(&grading_choice).map_err(|_| JobFailureKind::Permanent)?,
                )
                .map_err(|_| JobFailureKind::Permanent)?;
                let choice_map_sha256 = parts.server_choice_map_payload().server_sha256();
                item_bindings.push(QtiImportItemRegistration {
                    item: QtiImportItem {
                        item_id: source_identifier.clone(),
                        model_sha256: integrity.public_mapping_sha256,
                        assets: Vec::new(),
                    },
                    grading,
                });
                evidences.push(
                    QtiProfileImportEvidence::new(
                        reference,
                        source_identifier,
                        profile,
                        FlatImportIntegrityDigests {
                            normalized_item_sha256: parts.normalized_profile_item_sha256(),
                            profile_report_sha256: integrity.profile_report_sha256,
                            public_mapping_sha256: integrity.public_mapping_sha256,
                            private_mapping_sha256: integrity.private_mapping_sha256,
                            mapping_sha256: integrity.mapping_sha256,
                            warning_sha256: integrity.warning_sha256,
                            choice_map_sha256,
                        },
                    )
                    .map_err(|_| JobFailureKind::Permanent)?,
                );
            }
            let registry = QtiImportRegistry {
                reference,
                source: source.record,
                source_format: "qti".to_string(),
                source_identifier: None,
                importer: "adapter_qti".to_string(),
                parse_schema: profile.profile_id().to_string(),
                adapter_version: env!("CARGO_PKG_VERSION").to_string(),
                profile_summary: Some(profile_summary),
                items: item_bindings
                    .iter()
                    .map(|binding| binding.item.clone())
                    .collect(),
                item_results,
                assets: Vec::new(),
                unsupported_features: Vec::new(),
            };
            self.store
                .prepare_qti_import(
                    context,
                    CreateQtiImportCommand {
                        registry,
                        item_bindings,
                    },
                )
                .await
                .map_err(store_failure)?;
            for evidence in evidences {
                self.store
                    .stage_qti_profile_import_evidence(context, evidence)
                    .await
                    .map_err(store_failure)?;
            }
            return Ok(PreparedJobEffect::QtiImport {
                tenant,
                workspace,
                import,
                source_object,
            });
        }

        let package = QtiImporter::default()
            .import(&source.bytes)
            .map_err(|_| JobFailureKind::Permanent)?;
        if package.worker_original_sha256() != source.record.sha256.to_string()
            || package.worker_original_size_bytes() != source.record.size_bytes
            || package.worker_original_bytes() != source.bytes.as_slice()
        {
            return Err(JobFailureKind::Permanent);
        }

        let mut assets = Vec::with_capacity(package.worker_assets().len());
        for asset in package.worker_assets() {
            if execution.cancellation_requested() {
                return Err(JobFailureKind::TimedOut);
            }
            let object = staged_asset_object(import, asset.worker_asset_id());
            let key = ObjectKey::WorkspaceAsset {
                tenant,
                workspace,
                import,
                asset: asset.worker_asset_id(),
                object,
            };
            let record = match self
                .objects
                .put(PutObject {
                    key: key.clone(),
                    bytes: asset.worker_bytes().to_vec(),
                    media_type: asset.worker_media_type().to_string(),
                    license: "private-workspace-import".to_string(),
                    provenance: "QTI extracted asset".to_string(),
                    created_at: source.record.created_at,
                })
                .await
            {
                Ok(record) => record,
                Err(ObjectStoreError::AlreadyExists) => {
                    self.objects.get(&key).await.map_err(object_failure)?.record
                }
                Err(error) => return Err(object_failure(error)),
            };
            if record.id != object
                || record.key != key
                || record.category != ObjectCategory::Asset
                || record.sha256.to_string() != asset.worker_sha256()
            {
                return Err(JobFailureKind::Permanent);
            }
            assets.push(record);
        }

        let mut item_bindings = Vec::with_capacity(package.questions.len());
        for question in &package.questions {
            let asset_checksums =
                qti_question_asset_checksums(question).map_err(|_| JobFailureKind::Permanent)?;
            for (asset, checksum) in &asset_checksums {
                if !assets
                    .iter()
                    .any(|record| matches!(&record.key, ObjectKey::WorkspaceAsset { asset: stored, .. } if stored == asset)
                        && record.sha256.to_string() == *checksum)
                {
                    return Err(JobFailureKind::Permanent);
                }
            }
            let assets_for_item: Vec<AssetId> = asset_checksums.into_keys().collect();
            let model = serde_json::to_vec(question).map_err(|_| JobFailureKind::Permanent)?;
            let choice = package
                .worker_correct_choice(&question.item_id)
                .ok_or(JobFailureKind::Permanent)?;
            let grading = QtiImportGradingPayload::new(
                serde_json::to_vec(&choice).map_err(|_| JobFailureKind::Permanent)?,
            )
            .map_err(|_| JobFailureKind::Permanent)?;
            item_bindings.push(QtiImportItemRegistration {
                item: QtiImportItem {
                    item_id: question.item_id.clone(),
                    model_sha256: Sha256Digest::compute(&model),
                    assets: assets_for_item,
                },
                grading,
            });
        }
        let registry = QtiImportRegistry {
            reference: QtiImportRef {
                tenant,
                workspace,
                import,
            },
            source: source.record,
            source_format: "qti".to_string(),
            source_identifier: package.manifest.identifier.clone(),
            importer: "adapter_qti".to_string(),
            parse_schema: adapter_qti::QtiProfileId::GENERIC.to_string(),
            adapter_version: env!("CARGO_PKG_VERSION").to_string(),
            profile_summary: None,
            items: item_bindings
                .iter()
                .map(|binding| binding.item.clone())
                .collect(),
            item_results: package
                .item_results
                .iter()
                .map(|result| QtiImportItemResult {
                    source_identifier: result.source_identifier.clone(),
                    title: None,
                    item_id: result.item_id.clone(),
                    normalized_sha256: result.normalized_sha256,
                    status: match result.status {
                        adapter_qti::QtiItemImportStatus::Accepted => QtiImportItemStatus::Accepted,
                        adapter_qti::QtiItemImportStatus::Rejected => QtiImportItemStatus::Rejected,
                    },
                    diagnostics: Vec::new(),
                    defaults: Vec::new(),
                    warnings: result
                        .warnings
                        .iter()
                        .map(|warning| QtiUnsupportedFeature {
                            code: warning.feature.clone(),
                            location: warning.location.clone(),
                            detail: warning.detail.clone(),
                        })
                        .collect(),
                })
                .collect(),
            assets,
            unsupported_features: package
                .unsupported
                .iter()
                .map(|feature| QtiUnsupportedFeature {
                    code: feature.feature.clone(),
                    location: feature.location.clone(),
                    detail: feature.detail.clone(),
                })
                .collect(),
        };
        self.store
            .prepare_qti_import(
                context,
                CreateQtiImportCommand {
                    registry,
                    item_bindings,
                },
            )
            .await
            .map_err(store_failure)?;
        Ok(PreparedJobEffect::QtiImport {
            tenant,
            workspace,
            import,
            source_object,
        })
    }
}

fn store_failure(error: StoreError) -> JobFailureKind {
    match error {
        StoreError::Unavailable(_) => JobFailureKind::Transient,
        _ => JobFailureKind::Permanent,
    }
}

/// The sole QTI visibility boundary; it contains no archive or answer bytes.
pub(crate) struct QtiImportCommitter<S> {
    store: Arc<S>,
}

impl<S> QtiImportCommitter<S> {
    pub(crate) fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)] // Committer implementation follows its type declaration.
mod tests {
    use std::io::{Cursor, Write};

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use learning_data_access::in_memory::MemoryStore;
    use learning_data_access::{
        DraftRecord, EnqueueJob, JobLeaseDuration, JobStore, QtiGradingStore, QtiImportItemStatus,
        Store,
    };
    use objects::memory::MemoryObjectStore;
    use question_model::envelope::ContentBlock;
    use question_model::response::{ChoiceId, ChoiceOption};
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::{
        ActivityTimestamp, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition,
        QuestionMetadata, ResponseDefinition, TenantId, UserId, WorkspaceId, WorkspaceImportId,
    };

    use super::*;

    const VALID_PACKAGE: &str = concat!(
        "UEsDBBQAAAAIAHS7B13yXbGdXwAAAIsAAAAPAAAAaW1zbWFuaWZlc3QueG1sVY5RDkAwEESv0uwBNHxXryLClg2l",
        "uku4vYoIfiYvM5PJGF9P5JBFUYuTkCOMJYShA2si8rzGBvnFX4sEPSg5Aib2vAhVl1XtftyKkIPqI7q7xvrSLCWg",
        "rdGfZf0csCdQSwMEFAAAAAgAdLsHXcJKi+S6AAAAiwEAAA4AAABpdGVtcy9pdGVtLnhtbH2QSw7CMAxErxLlAETs",
        "XUu0sOgGUDlBCEaN1CZVHH63J7QgKEXsrPEbe2zQzMTckotlpFbYQ6rs0VLIpE2CRAjEnXdMSzKNDjpa70ZYtdpt",
        "N+vdKqHGh0AmVk8Hwlk3J8I9qKEANSHUj/EIj9W5P9wQOixq75lErEk83UI7vlCYgerSztpbQ6WLFLTpw70mlr9C",
        "ilZfi97CmZynzGzbrqFBGt2lJS5Afbb/wHuJ+TesJtGS9r5MjV+Pd1BLAQIUAxQAAAAIAHS7B13yXbGdXwAAAIsA",
        "AAAPAAAAAAAAAAAAAACAAQAAAABpbXNtYW5pZmVzdC54bWxQSwECFAMUAAAACAB0uwddwkqL5LoAAACLAQAADgAA",
        "AAAAAAAAAAAAgAGMAAAAaXRlbXMvaXRlbS54bWxQSwUGAAAAAAIAAgB5AAAAcgEAAAAA",
    );
    const CANVAS_MANIFEST: &str =
        include_str!("../../adapters/qti/tests/fixtures/profiles/canvas_positive_manifest.xml");
    const CANVAS_META: &str =
        include_str!("../../adapters/qti/tests/fixtures/profiles/canvas_assessment_meta.xml");
    const CANVAS_ITEM: &str =
        include_str!("../../adapters/qti/tests/fixtures/profiles/canvas_positive_item.xml");

    fn id(value: u128) -> uuid::Uuid {
        uuid::Uuid::from_u128(value)
    }

    fn canvas_archive(item: &str) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for (path, contents) in [
            ("imsmanifest.xml", CANVAS_MANIFEST),
            ("canvas_qti12_questions/assessment_meta.xml", CANVAS_META),
            ("canvas_qti12_questions/canvas-1.xml", item),
        ] {
            zip.start_file(path, options).expect("fixture entry");
            zip.write_all(contents.as_bytes()).expect("fixture body");
        }
        zip.finish().expect("fixture archive").into_inner()
    }

    fn mixed_canvas_item() -> String {
        let start = CANVAS_ITEM.find("      <item ").expect("item start");
        let end = CANVAS_ITEM[start..]
            .find("      </item>")
            .map(|offset| start + offset + "      </item>".len())
            .expect("item end");
        let rejected = CANVAS_ITEM[start..end]
            .replace("canvas-1", "canvas-2")
            .replacen("rcardinality=\"Single\"", "rcardinality=\"Multiple\"", 1);
        CANVAS_ITEM.replacen("    </section>", &format!("{rejected}\n    </section>"), 1)
    }

    async fn source(
        objects: &MemoryObjectStore,
        tenant: TenantId,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        object: ObjectId,
        bytes: Vec<u8>,
    ) {
        objects
            .put(PutObject {
                key: ObjectKey::WorkspaceSource {
                    tenant,
                    workspace,
                    import,
                    object,
                },
                bytes,
                media_type: "application/zip".to_string(),
                license: "private-workspace-import".to_string(),
                provenance: "QTI test source".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .expect("source object persists");
    }

    async fn save_workspace(store: &MemoryStore, tenant: TenantId, workspace: WorkspaceId) {
        store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                UserId::from_uuid(id(900)),
                None,
                DraftRecord {
                    tenant,
                    question: DraftQuestionDefinition {
                        workspace,
                        source: DraftQuestionSource::Qti {
                            item_id: "staging-fixture".to_string(),
                            import_id: WorkspaceImportId::from_uuid(id(901)),
                        },
                        prompt: vec![ContentBlock::Text {
                            markdown: "Fixture".to_string(),
                        }],
                        response: ResponseDefinition::MultipleChoice {
                            choices: vec![ChoiceOption {
                                id: ChoiceId::new("a"),
                                body: vec![ContentBlock::Text {
                                    markdown: "A".to_string(),
                                }],
                            }],
                            selection: question_model::answer::SelectionCardinality::ExactlyOne,
                        },
                        attempt_policy: AttemptPolicy {
                            max_attempts: None,
                            feedback: FeedbackDisclosure::ImmediateCorrectness,
                        },
                        timing_policy: TimingPolicy::Untimed,
                        randomization: question_model::generation::RandomizationDefinition::Static,
                        grading: GradingDefinition::AllOrNothing { points: 1.0 },
                        metadata: QuestionMetadata {
                            title: "QTI staging fixture".to_string(),
                            tags: Vec::new(),
                            taxonomy: Vec::new(),
                            license: question_model::taxonomy::License::CcBy,
                            language: "en-US".to_string(),
                        },
                    },
                    revises: None,
                    derived_from: None,
                },
            )
            .await
            .expect("worker fixture workspace saves");
    }

    #[tokio::test]
    async fn qti_worker_prepares_hidden_registry_and_retries_exact_objects() {
        let tenant = TenantId::from_uuid(id(1));
        let workspace = WorkspaceId::from_uuid(id(2));
        let import = WorkspaceImportId::from_uuid(id(3));
        let object = ObjectId::from_uuid(id(4));
        let bytes = STANDARD
            .decode(VALID_PACKAGE.trim())
            .expect("fixture base64");
        let store = Arc::new(MemoryStore::default());
        save_workspace(store.as_ref(), tenant, workspace).await;
        let objects = Arc::new(MemoryObjectStore::default());
        source(objects.as_ref(), tenant, workspace, import, object, bytes).await;
        let handler = QtiImportHandler::new(Arc::clone(&store), Arc::clone(&objects));
        let _committer = QtiImportCommitter::new(Arc::clone(&store));
        let payload = JobPayload::QtiImport {
            workspace,
            import,
            source_object: object,
        };
        let context = TenantContext::from_authenticated_session(tenant);
        let first = handler
            .prepare(context, payload.clone(), JobExecution::new())
            .await
            .expect("valid QTI prepares");
        let retry = handler
            .prepare(context, payload, JobExecution::new())
            .await
            .expect("retry reuses prepared import and objects");
        assert_eq!(first, retry);
        assert_eq!(
            store
                .get_qti_import(context, workspace, import)
                .await
                .expect("hidden registry lookup"),
            None
        );
        let job = store
            .enqueue_job(
                context,
                EnqueueJob {
                    tenant,
                    payload: JobPayload::QtiImport {
                        workspace,
                        import,
                        source_object: object,
                    },
                    max_attempts: 1,
                },
            )
            .await
            .expect("fixture job enqueues");
        let claim = store
            .claim_next_job(
                &learning_data_access::JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(60).expect("bounded lease"),
            )
            .await
            .expect("fixture claim query")
            .expect("fixture job claims");
        assert_eq!(claim.id, job);
        assert_eq!(
            store
                .commit_prepared_qti_import(
                    context,
                    CommitPreparedQtiImport {
                        job,
                        lease: claim.lease_token,
                        reference: QtiImportRef {
                            tenant,
                            workspace,
                            import,
                        },
                        source_object: object,
                    },
                )
                .await
                .expect("exact preparation commits"),
            CommitPreparedQtiImportOutcome::Committed
        );
        let registry = store
            .get_qti_import(context, workspace, import)
            .await
            .expect("committed registry lookup")
            .expect("committed registry exists");
        assert_eq!(registry.source_format, "qti");
        assert_eq!(registry.importer, "adapter_qti");
        assert_eq!(
            registry.parse_schema,
            "ple-qti-assessment-item-single-choice/v1"
        );
        assert_eq!(registry.source.provenance, "QTI test source");
        assert!(registry.profile_summary.is_none());
        assert_eq!(registry.item_results.len(), 1);
        assert_eq!(
            registry.item_results[0].status,
            QtiImportItemStatus::Accepted
        );
        assert!(registry.item_results[0].normalized_sha256.is_some());
        assert!(registry.item_results[0].title.is_none());
        assert!(registry.item_results[0].diagnostics.is_empty());
        assert!(registry.item_results[0].defaults.is_empty());
    }

    #[tokio::test]
    async fn qti_profile_worker_stages_safe_registry_and_private_ple_choice() {
        let tenant = TenantId::from_uuid(id(21));
        let workspace = WorkspaceId::from_uuid(id(22));
        let import = WorkspaceImportId::from_uuid(id(23));
        let object = ObjectId::from_uuid(id(24));
        let bytes = canvas_archive(&mixed_canvas_item());
        let expected_public_mapping =
            prepare_qti_profile_package(&bytes, adapter_qti::QtiImportLimits::default())
                .expect("profile preparation succeeds")
                .expect("Canvas is recognized")
                .into_items()
                .pop()
                .expect("accepted mapping")
                .integrity()
                .public_mapping_sha256;
        let (store, grader) = MemoryStore::with_qti_grader();
        let store = Arc::new(store);
        save_workspace(store.as_ref(), tenant, workspace).await;
        let objects = Arc::new(MemoryObjectStore::default());
        source(objects.as_ref(), tenant, workspace, import, object, bytes).await;
        let handler = QtiImportHandler::new(Arc::clone(&store), objects);
        let payload = JobPayload::QtiImport {
            workspace,
            import,
            source_object: object,
        };
        let context = TenantContext::from_authenticated_session(tenant);
        let first = handler
            .prepare(context, payload.clone(), JobExecution::new())
            .await
            .expect("profile import prepares");
        let retry = handler
            .prepare(context, payload.clone(), JobExecution::new())
            .await
            .expect("exact profile retry is idempotent");
        assert_eq!(first, retry);
        assert_eq!(
            store
                .get_qti_import(context, workspace, import)
                .await
                .expect("hidden registry lookup"),
            None
        );

        let job = store
            .enqueue_job(
                context,
                EnqueueJob {
                    tenant,
                    payload,
                    max_attempts: 1,
                },
            )
            .await
            .expect("fixture job enqueues");
        let claim = store
            .claim_next_job(
                &learning_data_access::JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(60).expect("bounded lease"),
            )
            .await
            .expect("fixture claim query")
            .expect("fixture job claims");
        assert_eq!(claim.id, job);
        assert_eq!(
            store
                .commit_prepared_qti_import(
                    context,
                    CommitPreparedQtiImport {
                        job,
                        lease: claim.lease_token,
                        reference: QtiImportRef {
                            tenant,
                            workspace,
                            import,
                        },
                        source_object: object,
                    },
                )
                .await
                .expect("exact profile preparation commits"),
            CommitPreparedQtiImportOutcome::Committed
        );
        let registry = store
            .get_qti_import(context, workspace, import)
            .await
            .expect("committed registry lookup")
            .expect("committed registry exists");
        let summary = registry
            .profile_summary
            .as_ref()
            .expect("recognized profile summary");
        assert_eq!(
            summary.profile_id(),
            "canvas-qti-1.2-static-single-choice/v1"
        );
        assert!(!summary.defaults().is_empty());
        assert!(registry.assets.is_empty());
        assert!(registry.items[0].assets.is_empty());
        assert_eq!(registry.items[0].item_id, "canvas-1");
        assert_eq!(registry.items[0].model_sha256, expected_public_mapping);
        assert_eq!(
            registry.item_results[0].item_id.as_deref(),
            Some("canvas-1")
        );
        assert_eq!(registry.item_results[1].source_identifier, "canvas-2");
        assert_eq!(
            registry.item_results[1].status,
            QtiImportItemStatus::Rejected
        );
        assert!(registry.item_results[1].item_id.is_none());
        assert!(!registry.item_results[1].diagnostics.is_empty());
        let safe_registry = serde_json::to_string(&registry).expect("safe registry serializes");
        assert!(!safe_registry.contains("blue"));
        assert!(!safe_registry.contains("red"));

        let grading = grader
            .qti_import_grading(context, workspace, import, "canvas-1")
            .await
            .expect("private grading lookup")
            .expect("private grading exists");
        assert_eq!(
            grading
                .server_correct_choice()
                .expect("PLE ChoiceId grading payload")
                .as_str(),
            "blue"
        );
    }

    #[tokio::test]
    async fn qti_profile_worker_commits_an_all_rejected_safe_report() {
        let tenant = TenantId::from_uuid(id(31));
        let workspace = WorkspaceId::from_uuid(id(32));
        let import = WorkspaceImportId::from_uuid(id(33));
        let object = ObjectId::from_uuid(id(34));
        let rejected =
            CANVAS_ITEM.replacen("rcardinality=\"Single\"", "rcardinality=\"Multiple\"", 1);
        let bytes = canvas_archive(&rejected);
        let expected_report =
            prepare_qti_profile_package(&bytes, adapter_qti::QtiImportLimits::default())
                .expect("recognized refusal remains a report")
                .expect("Canvas remains selected")
                .profile_report_sha256();
        let store = Arc::new(MemoryStore::default());
        save_workspace(store.as_ref(), tenant, workspace).await;
        let objects = Arc::new(MemoryObjectStore::default());
        source(objects.as_ref(), tenant, workspace, import, object, bytes).await;
        let handler = QtiImportHandler::new(Arc::clone(&store), objects);
        let payload = JobPayload::QtiImport {
            workspace,
            import,
            source_object: object,
        };
        let context = TenantContext::from_authenticated_session(tenant);
        handler
            .prepare(context, payload.clone(), JobExecution::new())
            .await
            .expect("all-rejected profile prepares safely");
        assert_eq!(
            store
                .get_qti_import(context, workspace, import)
                .await
                .expect("prepared report stays hidden"),
            None
        );
        let job = store
            .enqueue_job(
                context,
                EnqueueJob {
                    tenant,
                    payload,
                    max_attempts: 1,
                },
            )
            .await
            .expect("fixture job enqueues");
        let claim = store
            .claim_next_job(
                &learning_data_access::JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(60).expect("bounded lease"),
            )
            .await
            .expect("fixture claim query")
            .expect("fixture job claims");
        assert_eq!(claim.id, job);
        assert_eq!(
            store
                .commit_prepared_qti_import(
                    context,
                    CommitPreparedQtiImport {
                        job,
                        lease: claim.lease_token,
                        reference: QtiImportRef {
                            tenant,
                            workspace,
                            import,
                        },
                        source_object: object,
                    },
                )
                .await
                .expect("all-rejected report commits"),
            CommitPreparedQtiImportOutcome::Committed
        );
        let registry = store
            .get_qti_import(context, workspace, import)
            .await
            .expect("committed report lookup")
            .expect("committed report exists");
        let summary = registry.profile_summary.expect("recognized summary");
        assert_eq!(summary.profile_report_sha256(), expected_report);
        assert!(!summary.defaults().is_empty());
        assert!(registry.items.is_empty());
        assert!(matches!(
            registry.item_results.as_slice(),
            [rejected]
                if rejected.status == QtiImportItemStatus::Rejected
                    && rejected.item_id.is_none()
                    && !rejected.diagnostics.is_empty()
        ));
    }

    #[tokio::test]
    async fn qti_worker_refuses_malformed_or_misbinding_source_before_registry() {
        let tenant = TenantId::from_uuid(id(11));
        let workspace = WorkspaceId::from_uuid(id(12));
        let import = WorkspaceImportId::from_uuid(id(13));
        let object = ObjectId::from_uuid(id(14));
        let store = Arc::new(MemoryStore::default());
        let objects = Arc::new(MemoryObjectStore::default());
        source(
            objects.as_ref(),
            tenant,
            workspace,
            import,
            object,
            b"not a ZIP".to_vec(),
        )
        .await;
        let handler = QtiImportHandler::new(Arc::clone(&store), objects);
        let context = TenantContext::from_authenticated_session(tenant);
        assert_eq!(
            handler
                .prepare(
                    context,
                    JobPayload::QtiImport {
                        workspace,
                        import,
                        source_object: object,
                    },
                    JobExecution::new(),
                )
                .await,
            Err(JobFailureKind::Permanent)
        );
        assert_eq!(
            handler
                .prepare(
                    context,
                    JobPayload::QtiImport {
                        workspace,
                        import,
                        source_object: ObjectId::from_uuid(id(15)),
                    },
                    JobExecution::new(),
                )
                .await,
            Err(JobFailureKind::Permanent)
        );
        assert_eq!(
            store
                .get_qti_import(context, workspace, import)
                .await
                .expect("failed import remains absent"),
            None
        );
    }
}

impl<S> worker::sealed::EffectCommitter for QtiImportCommitter<S> where S: Send + Sync + 'static {}

#[async_trait]
impl<S> EffectCommitter for QtiImportCommitter<S>
where
    S: QtiImportStore + Send + Sync + 'static,
{
    async fn commit(
        &self,
        claim: JobCommitClaim,
        effect: PreparedJobEffect,
    ) -> Result<EffectCommitOutcome, StoreError> {
        let PreparedJobEffect::QtiImport {
            tenant,
            workspace,
            import,
            source_object,
        } = effect
        else {
            return Err(StoreError::InvalidRecord(
                "QTI committer received another effect family".to_string(),
            ));
        };
        match self
            .store
            .commit_prepared_qti_import(
                TenantContext::from_authenticated_session(tenant),
                CommitPreparedQtiImport {
                    job: claim.job_id(),
                    lease: claim.lease_token(),
                    reference: QtiImportRef {
                        tenant,
                        workspace,
                        import,
                    },
                    source_object,
                },
            )
            .await?
        {
            CommitPreparedQtiImportOutcome::Committed => Ok(EffectCommitOutcome::Committed),
            CommitPreparedQtiImportOutcome::ClaimNoLongerActive => {
                Ok(EffectCommitOutcome::ClaimNoLongerActive)
            }
        }
    }
}
