//! Private retention-worker effects: exact object deletion after Store revocation.
//!
//! No route uses this module. The closed queue payload carries only a course,
//! stage, and schedule generation; `RetentionWorkerStore` resolves the
//! protected notification and exact student-record manifest under its lease.

use std::sync::Arc;

use async_trait::async_trait;
use learning_data_access::{
    JobFailureKind, JobPayload, RetentionWork, RetentionWorkerCommand, RetentionWorkerStore,
    StoreError,
};
use objects::{ObjectKey, ObjectStore, ObjectStoreError};

use crate::worker::{
    self, EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
    PreparedJobEffect,
};

/// Server-only handler for the retention queue family.
pub struct RetentionJobHandler<S, O> {
    store: Arc<S>,
    objects: Arc<O>,
}

impl<S, O> RetentionJobHandler<S, O> {
    pub fn new(store: Arc<S>, objects: Arc<O>) -> Self {
        Self { store, objects }
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

fn validate_cleanup_key(
    key: &ObjectKey,
    course: question_model::CourseId,
) -> Result<(), JobFailureKind> {
    match key {
        ObjectKey::StudentRecord {
            course: key_course, ..
        } if *key_course == course => Ok(()),
        _ => Err(JobFailureKind::Permanent),
    }
}

#[async_trait]
impl<S, O> JobHandler for RetentionJobHandler<S, O>
where
    S: RetentionWorkerStore + Send + Sync + 'static,
    O: ObjectStore + Send + Sync + 'static,
{
    async fn prepare(
        &self,
        payload: JobPayload,
        execution: JobExecution,
    ) -> Result<PreparedJobEffect, JobFailureKind> {
        let JobPayload::Retention {
            course,
            stage,
            generation,
        } = payload
        else {
            return Err(JobFailureKind::Permanent);
        };
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        let claim = execution.claim().ok_or(JobFailureKind::Permanent)?;
        let command = RetentionWorkerCommand {
            course,
            stage,
            generation,
            job: claim.job_id(),
            lease: claim.lease_token(),
        };
        match self
            .store
            .prepare_retention_work(command)
            .await
            .map_err(store_failure)?
        {
            RetentionWork::Notify => {}
            RetentionWork::Cleanup(manifest) => {
                for key in manifest.objects() {
                    if execution.cancellation_requested() {
                        return Err(JobFailureKind::TimedOut);
                    }
                    validate_cleanup_key(key, course)?;
                    match self.objects.delete(key).await {
                        Ok(()) | Err(ObjectStoreError::NotFound) => {}
                        Err(ObjectStoreError::Unavailable(_)) => {
                            return Err(JobFailureKind::Transient);
                        }
                        Err(_) => return Err(JobFailureKind::Permanent),
                    }
                }
            }
        }
        Ok(PreparedJobEffect::Retention { command })
    }
}

/// Atomic Store finalizer for a prepared retention effect.
pub struct RetentionJobCommitter<S> {
    store: Arc<S>,
}
impl<S> RetentionJobCommitter<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}
impl<S> worker::sealed::EffectCommitter for RetentionJobCommitter<S> {}

#[async_trait]
impl<S> EffectCommitter for RetentionJobCommitter<S>
where
    S: RetentionWorkerStore + Send + Sync + 'static,
{
    async fn commit(
        &self,
        claim: JobCommitClaim,
        effect: PreparedJobEffect,
    ) -> Result<EffectCommitOutcome, StoreError> {
        let PreparedJobEffect::Retention { command } = effect else {
            return Err(StoreError::InvalidRecord(
                "retention committer received another effect family".to_string(),
            ));
        };
        if claim.job_id() != command.job || claim.lease_token() != command.lease {
            return Ok(EffectCommitOutcome::ClaimNoLongerActive);
        }
        self.store
            .commit_retention_work(command)
            .await
            .map(|()| EffectCommitOutcome::Committed)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use learning_data_access::{
        ClaimedJob, EnqueueJob, JobClaimFilter, JobFailureDisposition, JobId, JobKind,
        JobLeaseDuration, JobLeaseToken, JobStore, JobView, QueueDepth,
    };
    use objects::{
        ObjectRecord, ObjectStoreError, PutObject, SignedUrl, StoredObject,
        memory::MemoryObjectStore,
    };
    use question_model::{ActivityTimestamp, CourseId, ObjectId};
    use uuid::Uuid;

    use super::*;

    #[derive(Default)]
    struct FixtureStore {
        claim: Mutex<Option<ClaimedJob>>,
        committed: Mutex<Option<RetentionWorkerCommand>>,
    }

    #[async_trait]
    impl JobStore for FixtureStore {
        async fn enqueue_job(&self, _: TenantContext, _: EnqueueJob) -> Result<JobId, StoreError> {
            unreachable!("fixture never enqueues")
        }
        async fn claim_next_job(
            &self,
            _: &JobClaimFilter,
            _: JobLeaseDuration,
        ) -> Result<Option<ClaimedJob>, StoreError> {
            Ok(self.claim.lock().expect("claim lock").take())
        }
        async fn complete_job(&self, _: JobId, _: JobLeaseToken) -> Result<(), StoreError> {
            unreachable!("retention committer owns completion")
        }
        async fn fail_job(
            &self,
            _: JobId,
            _: JobLeaseToken,
            _: JobFailureKind,
        ) -> Result<JobFailureDisposition, StoreError> {
            unreachable!("fixture succeeds")
        }
        async fn get_job(&self, _: JobId) -> Result<Option<JobView>, StoreError> {
            Ok(None)
        }
        async fn ready_queue_depth(&self, _: &JobClaimFilter) -> Result<QueueDepth, StoreError> {
            Ok(QueueDepth { ready: 0 })
        }
    }

    #[async_trait]
    impl RetentionWorkerStore for FixtureStore {
        async fn prepare_retention_work(
            &self,
            command: RetentionWorkerCommand,
        ) -> Result<RetentionWork, StoreError> {
            assert_eq!(command.stage, learning_data_access::RetentionStage::Notify);
            Ok(RetentionWork::Notify)
        }
        async fn commit_retention_work(
            &self,
            command: RetentionWorkerCommand,
        ) -> Result<(), StoreError> {
            *self.committed.lock().expect("commit lock") = Some(command);
            Ok(())
        }
    }

    struct NoObjects;
    #[async_trait]
    impl ObjectStore for NoObjects {
        async fn put(&self, _: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
            unreachable!()
        }
        async fn get(&self, _: &ObjectKey) -> Result<StoredObject, ObjectStoreError> {
            unreachable!()
        }
        async fn delete(&self, _: &ObjectKey) -> Result<(), ObjectStoreError> {
            unreachable!()
        }
        async fn signed_url(
            &self,
            _: &ObjectKey,
            _: ActivityTimestamp,
        ) -> Result<SignedUrl, ObjectStoreError> {
            unreachable!()
        }
    }

    struct FlakyObjects {
        inner: Arc<MemoryObjectStore>,
        deletes: AtomicUsize,
    }
    #[async_trait]
    impl ObjectStore for FlakyObjects {
        async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
            self.inner.put(request).await
        }
        async fn get(&self, key: &ObjectKey) -> Result<StoredObject, ObjectStoreError> {
            self.inner.get(key).await
        }
        async fn delete(&self, key: &ObjectKey) -> Result<(), ObjectStoreError> {
            if self.deletes.fetch_add(1, Ordering::SeqCst) == 1 {
                return Err(ObjectStoreError::Unavailable("fixture".to_string()));
            }
            self.inner.delete(key).await
        }
        async fn signed_url(
            &self,
            key: &ObjectKey,
            now: ActivityTimestamp,
        ) -> Result<SignedUrl, ObjectStoreError> {
            self.inner.signed_url(key, now).await
        }
    }

    #[tokio::test]
    async fn worker_dispatches_closed_retention_payload_to_lease_bound_committer() {
        let course = CourseId::from_uuid(Uuid::from_u128(70_002));
        let job = JobId::from_uuid(Uuid::from_u128(70_003));
        let lease = JobLeaseToken::generate().expect("lease");
        let store = Arc::new(FixtureStore {
            claim: Mutex::new(Some(ClaimedJob {
                id: job,
                payload: JobPayload::Retention {
                    course,
                    stage: learning_data_access::RetentionStage::Notify,
                    generation: 1,
                },
                lease_token: lease,
                attempt_count: 1,
            })),
            committed: Mutex::new(None),
        });
        let handler: Arc<dyn JobHandler> = Arc::new(RetentionJobHandler::new(
            Arc::clone(&store),
            Arc::new(NoObjects),
        ));
        let committer: Arc<dyn EffectCommitter> =
            Arc::new(RetentionJobCommitter::new(Arc::clone(&store)));
        let registry = worker::JobRegistry::new([worker::JobRegistryEntry::new(
            JobKind::Retention,
            handler,
            committer,
        )])
        .expect("registry");
        let worker = worker::Worker::new(
            Arc::clone(&store),
            registry,
            worker::WorkerSettings::new(10, std::time::Duration::from_secs(1), 1)
                .expect("settings"),
        );
        assert_eq!(worker.drain_once().await.expect("drain").completed, 1);
        assert_eq!(
            store
                .committed
                .lock()
                .expect("commit lock")
                .as_ref()
                .map(|command| (command.course, command.generation, command.job)),
            Some((course, 1, job))
        );
    }

    #[tokio::test]
    async fn actual_memory_store_cleanup_retries_exact_keys_and_treats_absence_as_success() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(71_001));
        let course = CourseId::from_uuid(Uuid::from_u128(71_002));
        let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(3_000_000))
            .expect("clock");
        let ids = (0..4)
            .map(|offset| ObjectId::from_uuid(Uuid::from_u128(71_010 + offset)))
            .collect::<Vec<_>>();
        let keys = store
            .seed_retention_cleanup_for_test(tenant, course, ids)
            .expect("seed");
        let inner = Arc::new(MemoryObjectStore::default());
        for key in &keys {
            inner
                .put(PutObject {
                    key: key.clone(),
                    bytes: b"record".to_vec(),
                    media_type: "application/octet-stream".to_string(),
                    license: "educational-record".to_string(),
                    provenance: "test".to_string(),
                    created_at: ActivityTimestamp::from_unix_millis(3_000_000),
                })
                .await
                .expect("put");
        }
        let objects = Arc::new(FlakyObjects {
            inner: Arc::clone(&inner),
            deletes: AtomicUsize::new(0),
        });
        let handler: Arc<dyn JobHandler> = Arc::new(RetentionJobHandler::new(
            Arc::clone(&store),
            Arc::clone(&objects),
        ));
        let committer: Arc<dyn EffectCommitter> =
            Arc::new(RetentionJobCommitter::new(Arc::clone(&store)));
        let registry = worker::JobRegistry::new([worker::JobRegistryEntry::new(
            JobKind::Retention,
            handler,
            committer,
        )])
        .expect("registry");
        let worker = worker::Worker::new(
            Arc::clone(&store),
            registry,
            worker::WorkerSettings::new(10, std::time::Duration::from_secs(1), 1)
                .expect("settings"),
        );
        assert_eq!(worker.drain_once().await.expect("first").retrying, 1);
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(3_001_000))
            .expect("advance");
        assert_eq!(worker.drain_once().await.expect("retry").completed, 1);
        for key in keys {
            assert!(matches!(
                inner.get(&key).await,
                Err(ObjectStoreError::NotFound)
            ));
        }
    }

    #[test]
    fn hostile_cleanup_targets_are_refused_before_any_object_operation() {
        let course = CourseId::from_uuid(Uuid::from_u128(72_001));
        let foreign = CourseId::from_uuid(Uuid::from_u128(72_002));
        let object = ObjectId::from_uuid(Uuid::from_u128(72_003));
        assert!(validate_cleanup_key(&ObjectKey::StudentRecord { course, object }, course).is_ok());
        assert_eq!(
            validate_cleanup_key(
                &ObjectKey::StudentRecord {
                    course: foreign,
                    object,
                },
                course,
            ),
            Err(JobFailureKind::Permanent)
        );
        assert_eq!(
            validate_cleanup_key(&ObjectKey::Temporary { object }, course),
            Err(JobFailureKind::Permanent)
        );
    }
}
