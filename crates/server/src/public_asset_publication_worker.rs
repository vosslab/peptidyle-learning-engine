//! Durable, post-commit public-asset publication.
//!
//! The catalog transaction records a `Pending` delivery and queues this work
//! in the same commit. This worker alone materializes the final public key;
//! it always re-resolves the target and private source from the durable
//! registry, never from browser input or queue payload bytes.

use std::{future::Future, sync::Arc};

use async_trait::async_trait;
use learning_data_access::{
    AssetDeliveryRecord, AssetDeliveryScope, AssetPublication, JobFailureKind, JobPayload,
    PublicAssetPublicationStore, StoreError,
};
use objects::{ObjectKey, ObjectStore, ObjectStoreError, PutObject, Sha256Digest};
use question_model::ProblemVersionRef;

use crate::worker::{
    self, EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
    PreparedJobEffect,
};

/// Cancellable materializer for committed public catalog assets.
pub(crate) struct PublicAssetPublicationHandler<S, O> {
    store: Arc<S>,
    objects: Arc<O>,
}

impl<S, O> PublicAssetPublicationHandler<S, O> {
    pub(crate) fn new(store: Arc<S>, objects: Arc<O>) -> Self {
        Self { store, objects }
    }
}

/// Lease-conditional registry activator for published public assets.
pub(crate) struct PublicAssetPublicationCommitter<S> {
    store: Arc<S>,
}

impl<S> PublicAssetPublicationCommitter<S> {
    pub(crate) fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

fn store_failure(error: StoreError) -> JobFailureKind {
    match error {
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => JobFailureKind::Transient,
        StoreError::NotFound
        | StoreError::AlreadyExists
        | StoreError::OwnershipMismatch
        | StoreError::Conflict
        | StoreError::Forbidden
        | StoreError::InvalidRecord(_)
        | StoreError::RunModel(_)
        | StoreError::TimedOut => JobFailureKind::Permanent,
    }
}

fn object_failure(error: ObjectStoreError) -> JobFailureKind {
    match error {
        ObjectStoreError::Unavailable(_) => JobFailureKind::Transient,
        ObjectStoreError::NotFound
        | ObjectStoreError::ChecksumMismatch
        | ObjectStoreError::AlreadyExists
        | ObjectStoreError::NotSignable
        | ObjectStoreError::NumericOverflow => JobFailureKind::Permanent,
    }
}

async fn cancellable<F, T, E>(execution: &JobExecution, operation: F) -> Result<T, JobFailureKind>
where
    F: Future<Output = Result<T, E>>,
    E: IntoFailure,
{
    tokio::select! {
        value = operation => value.map_err(IntoFailure::into_failure),
        () = execution.cancelled() => Err(JobFailureKind::TimedOut),
    }
}

trait IntoFailure {
    fn into_failure(self) -> JobFailureKind;
}

impl IntoFailure for StoreError {
    fn into_failure(self) -> JobFailureKind {
        store_failure(self)
    }
}

impl IntoFailure for ObjectStoreError {
    fn into_failure(self) -> JobFailureKind {
        object_failure(self)
    }
}

fn validate_pending_record(
    record: &AssetDeliveryRecord,
    reference: ProblemVersionRef,
) -> Result<(), JobFailureKind> {
    let AssetDeliveryScope::Catalog {
        asset,
        reference: recorded_reference,
    } = record.scope
    else {
        return Err(JobFailureKind::Permanent);
    };
    let ObjectKey::ProblemAsset {
        problem,
        version,
        asset: key_asset,
        object,
    } = &record.object.key
    else {
        return Err(JobFailureKind::Permanent);
    };
    let Some(source) = &record.pending_source else {
        return Err(JobFailureKind::Permanent);
    };
    if record.publication != AssetPublication::Pending
        || recorded_reference != reference
        || problem != &reference.problem
        || version != &reference.version
        || key_asset != &asset
        || object != &record.object.id
        || record.object.bucket != objects::Bucket::PublicAssets
        || record.object.sha256 != source.sha256
        || record.object.size_bytes != source.size_bytes
        || record.object.media_type != source.media_type
        || source.key.bucket() == objects::Bucket::PublicAssets
    {
        return Err(JobFailureKind::Permanent);
    }
    Ok(())
}

async fn materialize_one<O: ObjectStore>(
    objects: &O,
    execution: &JobExecution,
    record: AssetDeliveryRecord,
    reference: ProblemVersionRef,
) -> Result<(), JobFailureKind> {
    validate_pending_record(&record, reference)?;
    let source = record
        .pending_source
        .as_ref()
        .expect("validated pending source");
    let stored = cancellable(execution, objects.get(&source.key)).await?;
    if stored.record != *source || Sha256Digest::compute(&stored.bytes) != source.sha256 {
        return Err(JobFailureKind::Permanent);
    }
    let candidate = PutObject {
        key: record.object.key.clone(),
        bytes: stored.bytes,
        media_type: record.object.media_type.clone(),
        license: record.object.license.clone(),
        provenance: record.object.provenance.clone(),
        created_at: record.object.created_at,
    };
    match cancellable(execution, objects.put(candidate)).await {
        Ok(written) if written == record.object => Ok(()),
        Ok(_) => Err(JobFailureKind::Permanent),
        Err(JobFailureKind::Permanent) => {
            // A retry after a worker crash finds the immutable key already
            // present. Its exact metadata must agree before activation.
            let existing = cancellable(execution, objects.get(&record.object.key)).await?;
            if existing.record == record.object
                && Sha256Digest::compute(&existing.bytes) == record.object.sha256
            {
                Ok(())
            } else {
                Err(JobFailureKind::Permanent)
            }
        }
        Err(error) => Err(error),
    }
}

#[async_trait]
impl<S, O> JobHandler for PublicAssetPublicationHandler<S, O>
where
    S: PublicAssetPublicationStore + Send + Sync + 'static,
    O: ObjectStore + 'static,
{
    async fn prepare(
        &self,
        payload: JobPayload,
        execution: JobExecution,
    ) -> Result<PreparedJobEffect, JobFailureKind> {
        let JobPayload::PublishPublicAssets { reference } = payload else {
            return Err(JobFailureKind::Permanent);
        };
        let claim = execution.claim().ok_or(JobFailureKind::Permanent)?;
        let records = cancellable(
            &execution,
            self.store.pending_public_asset_publication(
                claim.job_id(),
                claim.lease_token(),
                reference,
            ),
        )
        .await?;
        for record in records {
            materialize_one(self.objects.as_ref(), &execution, record, reference).await?;
        }
        Ok(PreparedJobEffect::PublicAssetPublication { reference })
    }
}

impl<S> worker::sealed::EffectCommitter for PublicAssetPublicationCommitter<S> where
    S: Send + Sync + 'static
{
}

#[async_trait]
impl<S> EffectCommitter for PublicAssetPublicationCommitter<S>
where
    S: PublicAssetPublicationStore + Send + Sync + 'static,
{
    async fn commit(
        &self,
        claim: JobCommitClaim,
        effect: PreparedJobEffect,
    ) -> Result<EffectCommitOutcome, StoreError> {
        let PreparedJobEffect::PublicAssetPublication { reference } = effect else {
            return Err(StoreError::InvalidRecord(
                "public-asset committer received another effect family".to_string(),
            ));
        };
        self.store
            .activate_public_asset_publication(claim.job_id(), claim.lease_token(), reference)
            .await?;
        Ok(EffectCommitOutcome::Committed)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use learning_data_access::{AssetDeliveryId, JobId, JobLeaseToken};
    use objects::{ObjectCategory, ObjectRecord, memory::MemoryObjectStore};
    use question_model::{ActivityTimestamp, AssetId, ObjectId, ProblemId, VersionId};

    use super::*;

    #[derive(Default)]
    struct PendingStore {
        records: Mutex<Vec<AssetDeliveryRecord>>,
        activated: Mutex<Vec<ProblemVersionRef>>,
        active: bool,
    }

    #[async_trait]
    impl PublicAssetPublicationStore for PendingStore {
        async fn pending_public_asset_publication(
            &self,
            _job: JobId,
            _lease: JobLeaseToken,
            reference: ProblemVersionRef,
        ) -> Result<Vec<AssetDeliveryRecord>, StoreError> {
            Ok(self
                .records
                .lock()
                .expect("test records lock")
                .iter()
                .filter(|record| {
                    matches!(record.scope, AssetDeliveryScope::Catalog { reference: value, .. } if value == reference)
                })
                .cloned()
                .collect())
        }

        async fn activate_public_asset_publication(
            &self,
            _job: JobId,
            _lease: JobLeaseToken,
            reference: ProblemVersionRef,
        ) -> Result<(), StoreError> {
            if !self.active {
                return Err(StoreError::Conflict);
            }
            self.activated
                .lock()
                .expect("test activation lock")
                .push(reference);
            Ok(())
        }
    }

    fn id(value: u128) -> uuid::Uuid {
        uuid::Uuid::from_u128(value)
    }

    #[tokio::test]
    async fn publisher_materializes_only_after_pending_registry_is_resolved() {
        let objects = Arc::new(MemoryObjectStore::default());
        let reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(id(1)),
            version: VersionId::from_uuid(id(2)),
        };
        let asset = AssetId::from_uuid(id(3));
        let source_key = ObjectKey::WorkspaceQuestionAsset {
            workspace: question_model::WorkspaceId::from_uuid(id(5)),
            asset,
            object: ObjectId::from_uuid(id(6)),
        };
        let source = objects
            .put(PutObject {
                key: source_key,
                bytes: b"verified source".to_vec(),
                media_type: "image/png".to_string(),
                license: "CC0-1.0".to_string(),
                provenance: "test source".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .expect("source stores privately");
        let target_key = ObjectKey::ProblemAsset {
            problem: reference.problem,
            version: reference.version,
            asset,
            object: ObjectId::from_uuid(id(7)),
        };
        let target = ObjectRecord {
            id: target_key.object_id(),
            bucket: objects::Bucket::PublicAssets,
            key: target_key.clone(),
            sha256: source.sha256,
            size_bytes: source.size_bytes,
            media_type: source.media_type.clone(),
            category: ObjectCategory::Asset,
            version: Some(reference.version),
            license: "CC0-1.0".to_string(),
            provenance: "published test asset".to_string(),
            created_at: source.created_at,
        };
        let record = AssetDeliveryRecord {
            id: AssetDeliveryId::from_asset(asset),
            object: target.clone(),
            intrinsic_width: None,
            intrinsic_height: None,
            scope: AssetDeliveryScope::Catalog { asset, reference },
            publication: AssetPublication::Pending,
            pending_source: Some(source),
        };
        let store = Arc::new(PendingStore {
            records: Mutex::new(vec![record]),
            activated: Mutex::default(),
            active: true,
        });
        assert!(matches!(
            objects.get(&target_key).await,
            Err(ObjectStoreError::NotFound)
        ));
        let handler = PublicAssetPublicationHandler::new(Arc::clone(&store), Arc::clone(&objects));
        let effect = handler
            .prepare(
                JobPayload::PublishPublicAssets { reference },
                JobExecution::new().with_test_claim(JobCommitClaim::new(
                    JobId::generate().expect("job"),
                    JobLeaseToken::generate().expect("lease"),
                )),
            )
            .await
            .expect("committed pending record materializes");
        assert_eq!(
            objects
                .get(&target_key)
                .await
                .expect("public target exists")
                .record,
            target
        );
        let retry = handler
            .prepare(
                JobPayload::PublishPublicAssets { reference },
                JobExecution::new().with_test_claim(JobCommitClaim::new(
                    JobId::generate().expect("job"),
                    JobLeaseToken::generate().expect("lease"),
                )),
            )
            .await
            .expect("retry verifies the exact existing immutable target");
        assert_eq!(retry, effect);
        let committer = PublicAssetPublicationCommitter::new(Arc::clone(&store));
        assert_eq!(
            committer
                .commit(
                    JobCommitClaim::new(
                        JobId::generate().expect("job"),
                        JobLeaseToken::generate().expect("lease"),
                    ),
                    effect,
                )
                .await
                .expect("activation"),
            EffectCommitOutcome::Committed
        );
        assert_eq!(
            *store.activated.lock().expect("activation lock"),
            vec![reference]
        );
    }

    #[tokio::test]
    async fn stale_activation_lease_fails_closed() {
        let store = Arc::new(PendingStore::default());
        let committer = PublicAssetPublicationCommitter::new(store);
        let reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(id(11)),
            version: VersionId::from_uuid(id(12)),
        };
        assert_eq!(
            committer
                .commit(
                    JobCommitClaim::new(
                        JobId::generate().expect("job"),
                        JobLeaseToken::generate().expect("lease"),
                    ),
                    PreparedJobEffect::PublicAssetPublication { reference },
                )
                .await,
            Err(StoreError::Conflict)
        );
    }
}
