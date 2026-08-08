//! Private QTI staging worker.
//!
//! Archive and answer material cross this module only through server-owned
//! types.  Preparation writes immutable object bytes first, then a hidden
//! registry; the sealed committer is the only visibility transition.

use std::sync::Arc;

use adapter_qti::{QtiImporter, qti_question_asset_checksums};
use async_trait::async_trait;
use objects::{ObjectCategory, ObjectKey, ObjectStore, ObjectStoreError, PutObject, Sha256Digest};
use question_model::{AssetId, ObjectId};
use sha2::{Digest, Sha256};
use store::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    JobFailureKind, JobPayload, QtiImportGradingPayload, QtiImportItem, QtiImportItemRegistration,
    QtiImportRef, QtiImportRegistry, QtiImportStore, QtiUnsupportedFeature, StoreError,
    TenantContext,
};

use crate::worker::{
    self, EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
    PreparedJobEffect,
};

/// Worker implementation for the closed QTI queue payload.
pub(crate) struct QtiImportHandler<S, O> {
    store: Arc<S>,
    objects: Arc<O>,
}

impl<S, O> QtiImportHandler<S, O> {
    #[allow(dead_code)] // The composition root opts in to worker families separately.
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
    S: QtiImportStore + Send + Sync + 'static,
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
            parse_schema: "qti-1.2-subset".to_string(),
            adapter_version: env!("CARGO_PKG_VERSION").to_string(),
            items: item_bindings
                .iter()
                .map(|binding| binding.item.clone())
                .collect(),
            assets,
            unsupported_features: package
                .unsupported
                .iter()
                .map(|feature| QtiUnsupportedFeature {
                    code: feature.feature.clone(),
                    location: feature.location.clone(),
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
            .map_err(|error| match error {
                StoreError::Unavailable(_) => JobFailureKind::Transient,
                _ => JobFailureKind::Permanent,
            })?;
        Ok(PreparedJobEffect::QtiImport {
            tenant,
            workspace,
            import,
            source_object,
        })
    }
}

/// The sole QTI visibility boundary; it contains no archive or answer bytes.
pub(crate) struct QtiImportCommitter<S> {
    store: Arc<S>,
}

impl<S> QtiImportCommitter<S> {
    #[allow(dead_code)] // The composition root opts in to worker families separately.
    pub(crate) fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)] // Committer implementation follows its type declaration.
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use objects::memory::MemoryObjectStore;
    use question_model::{ActivityTimestamp, TenantId, WorkspaceId, WorkspaceImportId};
    use store::memory::MemoryStore;

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

    fn id(value: u128) -> uuid::Uuid {
        uuid::Uuid::from_u128(value)
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
