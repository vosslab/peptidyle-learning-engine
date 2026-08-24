use std::{
    collections::BTreeSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use learning_data_access::{
    EnqueueJob, ExportArtifactKind, ExportArtifactRecord, JobKind, JobLeaseDuration, JobState,
    TenantContext,
};
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::{ActivityTimestamp, ProblemId, ProblemVersionRef, TenantId, VersionId};
use uuid::Uuid;

use super::*;

#[derive(Clone)]
enum Behavior {
    Success,
    CooperativeSleep(Duration),
    UncooperativeFor(TenantId, Duration),
}

#[derive(Clone)]
struct RecordingHandler {
    behavior: Behavior,
    tenants: Arc<Mutex<Vec<TenantId>>>,
}

#[async_trait]
impl JobHandler for RecordingHandler {
    async fn prepare(
        &self,
        context: TenantContext,
        payload: JobPayload,
        execution: JobExecution,
    ) -> Result<PreparedJobEffect, JobFailureKind> {
        self.tenants
            .lock()
            .expect("test lock")
            .push(context.tenant_id());
        match self.behavior {
            Behavior::Success => Ok(effect_for(payload)),
            Behavior::CooperativeSleep(duration) => tokio::select! {
                () = tokio::time::sleep(duration) => Ok(effect_for(payload)),
                () = execution.cancelled() => Err(JobFailureKind::TimedOut),
            },
            Behavior::UncooperativeFor(expected, duration) if expected == context.tenant_id() => {
                tokio::time::sleep(duration).await;
                Ok(effect_for(payload))
            }
            Behavior::UncooperativeFor(_, _) => Ok(effect_for(payload)),
        }
    }
}

struct MemoryCommitter {
    store: Arc<learning_data_access::in_memory::MemoryStore>,
    visible: Arc<Mutex<BTreeSet<JobId>>>,
    pause_before_commit: Option<Arc<tokio::sync::Notify>>,
    release_commit: Option<Arc<tokio::sync::Notify>>,
    fail_after_effect_once: AtomicBool,
}

impl sealed::EffectCommitter for MemoryCommitter {}

#[async_trait]
impl EffectCommitter for MemoryCommitter {
    async fn commit(
        &self,
        claim: JobCommitClaim,
        _effect: PreparedJobEffect,
    ) -> Result<EffectCommitOutcome, StoreError> {
        if let Some(started) = &self.pause_before_commit {
            started.notify_one();
        }
        if let Some(release) = &self.release_commit {
            release.notified().await;
        }
        // This test sink models the production conditional transaction: its
        // visible idempotency record is keyed by JobId and it refuses to add
        // one unless the exact lease can still complete.
        if self.fail_after_effect_once.swap(false, Ordering::AcqRel) {
            self.visible
                .lock()
                .expect("test lock")
                .insert(claim.job_id());
            return Err(StoreError::Unavailable(
                "injected crash after durable effect".to_string(),
            ));
        }
        match self
            .store
            .complete_job(claim.job_id(), claim.lease_token())
            .await
        {
            Ok(()) => {
                self.visible
                    .lock()
                    .expect("test lock")
                    .insert(claim.job_id());
                Ok(EffectCommitOutcome::Committed)
            }
            Err(StoreError::Conflict) => Ok(EffectCommitOutcome::ClaimNoLongerActive),
            Err(error) => Err(error),
        }
    }
}

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
fn tenant(value: u128) -> TenantId {
    TenantId::from_uuid(id(value))
}
fn effect_for(payload: JobPayload) -> PreparedJobEffect {
    match payload {
        JobPayload::RecalculateAssignment {
            assignment,
            generation,
        } => PreparedJobEffect::AssignmentScoring {
            tenant: TenantId::from_uuid(id(1)),
            assignment,
            generation,
        },
        JobPayload::RecalculateCourseItemAnalysis {
            assignment,
            generation,
        } => PreparedJobEffect::CourseItemAnalysis {
            tenant: TenantId::from_uuid(id(1)),
            assignment,
            generation,
        },
        JobPayload::AutoSubmitAttempt {
            attempt,
            timing_generation,
        } => PreparedJobEffect::AttemptAutoSubmit {
            tenant: TenantId::from_uuid(id(1)),
            attempt,
            timing_generation,
        },
        JobPayload::Retention { .. } => {
            panic!("recording handler must not receive retention work")
        }
        JobPayload::Render { .. } => PreparedJobEffect::Test,
        JobPayload::Export { delivery_object } => PreparedJobEffect::Export {
            tenant: TenantId::from_uuid(id(1)),
            manifest: delivery_object,
            artifacts: Box::new(PreparedExportArtifacts {
                docx: export_artifact(ExportArtifactKind::Docx, id(9_001)),
                pdf: export_artifact(ExportArtifactKind::Pdf, id(9_002)),
                accessible_docx: export_artifact(ExportArtifactKind::AccessibleDocx, id(9_003)),
                accessible_pdf: export_artifact(ExportArtifactKind::AccessiblePdf, id(9_004)),
            }),
        },
        JobPayload::Import { .. } => PreparedJobEffect::Test,
        JobPayload::QtiImport {
            workspace,
            import,
            source_object,
        } => PreparedJobEffect::QtiImport {
            tenant: TenantId::from_uuid(id(1)),
            workspace,
            import,
            source_object,
        },
        JobPayload::PublishPublicAssets { .. } => PreparedJobEffect::Test,
    }
}

fn export_artifact(kind: ExportArtifactKind, object: Uuid) -> ExportArtifactRecord {
    let object = ObjectId::from_uuid(object);
    let key = ObjectKey::StudentRecord {
        tenant: TenantId::from_uuid(id(1)),
        object,
    };
    ExportArtifactRecord {
        kind,
        filename: "fixture".to_string(),
        object: ObjectRecord {
            id: object,
            bucket: Bucket::StudentRecords,
            key,
            sha256: Sha256Digest::compute(b"fixture"),
            size_bytes: 7,
            media_type: kind.media_type().to_string(),
            category: ObjectCategory::Export,
            version: None,
            license: "educational-record".to_string(),
            provenance: "fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        },
    }
}
fn render_job(tenant: TenantId, max_attempts: u16) -> EnqueueJob {
    EnqueueJob {
        tenant,
        payload: JobPayload::Render {
            reference: ProblemVersionRef {
                problem: ProblemId::from_uuid(id(31)),
                version: VersionId::from_uuid(id(32)),
            },
            seed: 4,
        },
        max_attempts,
    }
}
fn committer(store: Arc<learning_data_access::in_memory::MemoryStore>) -> Arc<MemoryCommitter> {
    Arc::new(MemoryCommitter {
        store,
        visible: Arc::new(Mutex::new(BTreeSet::new())),
        pause_before_commit: None,
        release_commit: None,
        fail_after_effect_once: AtomicBool::new(false),
    })
}
fn registry(
    behavior: Behavior,
    tenants: Arc<Mutex<Vec<TenantId>>>,
    committer: Arc<MemoryCommitter>,
) -> JobRegistry {
    let handler: Arc<dyn JobHandler> = Arc::new(RecordingHandler { behavior, tenants });
    let committer: Arc<dyn EffectCommitter> = committer;
    JobRegistry::new([JobRegistryEntry::new(JobKind::Render, handler, committer)])
        .expect("test registry")
}
fn worker(
    store: Arc<learning_data_access::in_memory::MemoryStore>,
    behavior: Behavior,
    tenants: Arc<Mutex<Vec<TenantId>>>,
    batch_size: usize,
) -> Worker<learning_data_access::in_memory::MemoryStore> {
    let commit = committer(Arc::clone(&store));
    Worker::new(
        store,
        registry(behavior, tenants, commit),
        WorkerSettings::new(2, Duration::from_millis(30), batch_size).expect("settings"),
    )
}
async fn enqueue(
    store: &learning_data_access::in_memory::MemoryStore,
    tenant: TenantId,
    attempts: u16,
) -> JobId {
    store
        .enqueue_job(
            TenantContext::from_authenticated_session(tenant),
            render_job(tenant, attempts),
        )
        .await
        .expect("enqueue")
}

#[tokio::test]
async fn registry_claims_only_complete_families_and_leaves_reserved_work_ready() {
    let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let tenant = tenant(7);
    let context = TenantContext::from_authenticated_session(tenant);
    let reserved = enqueue(&store, tenant, 2).await;
    let supported = store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::Export {
                    delivery_object: ObjectId::from_uuid(id(7_001)),
                },
                max_attempts: 2,
            },
        )
        .await
        .expect("supported export");
    let handler: Arc<dyn JobHandler> = Arc::new(RecordingHandler {
        behavior: Behavior::Success,
        tenants: Arc::new(Mutex::new(Vec::new())),
    });
    let committer: Arc<dyn EffectCommitter> = committer(Arc::clone(&store));
    let registry = JobRegistry::new([JobRegistryEntry::new(JobKind::Export, handler, committer)])
        .expect("export registry");
    let worker = Worker::new(
        Arc::clone(&store),
        registry,
        WorkerSettings::new(2, Duration::from_millis(30), 1).expect("settings"),
    );

    assert_eq!(worker.drain_once().await.expect("drain").completed, 1);
    assert_eq!(worker.ready_queue_depth().await.expect("depth").ready, 0);
    assert_eq!(
        store
            .get_job(context, supported)
            .await
            .expect("supported view")
            .expect("supported job")
            .state,
        JobState::Completed
    );
    assert_eq!(
        store
            .get_job(context, reserved)
            .await
            .expect("reserved view")
            .expect("reserved job")
            .state,
        JobState::Ready
    );
}

#[tokio::test]
async fn concurrent_workers_claim_distinct_jobs_and_drain_depth() {
    let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let seen = Arc::new(Mutex::new(Vec::new()));
    enqueue(&store, tenant(1), 2).await;
    enqueue(&store, tenant(2), 2).await;
    let left = worker(Arc::clone(&store), Behavior::Success, Arc::clone(&seen), 1);
    let right = worker(Arc::clone(&store), Behavior::Success, Arc::clone(&seen), 1);
    let (left, right) = tokio::join!(left.drain_once(), right.drain_once());
    assert_eq!(
        left.expect("left").completed + right.expect("right").completed,
        2
    );
    let mut tenants = seen.lock().expect("test lock").clone();
    tenants.sort();
    assert_eq!(tenants, vec![tenant(1), tenant(2)]);
    assert_eq!(
        worker(store, Behavior::Success, seen, 1)
            .ready_queue_depth()
            .await
            .expect("depth")
            .ready,
        0
    );
}

#[tokio::test(start_paused = true)]
async fn cooperative_timeout_stops_preparation_before_retry() {
    let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let job = enqueue(&store, tenant(1), 2).await;
    let report = worker(
        Arc::clone(&store),
        Behavior::CooperativeSleep(Duration::from_millis(100)),
        Arc::clone(&seen),
        1,
    )
    .drain_once()
    .await
    .expect("drain");
    assert_eq!(report.retrying, 1);
    assert_eq!(
        store
            .get_job(TenantContext::from_authenticated_session(tenant(1)), job)
            .await
            .expect("view")
            .expect("job")
            .state,
        JobState::Ready
    );
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("advance retry clock");
    let retry = worker(Arc::clone(&store), Behavior::Success, seen, 1)
        .drain_once()
        .await
        .expect("retry drain");
    assert_eq!(
        retry.completed, 1,
        "only the post-cancellation retry commits"
    );
}

#[tokio::test(start_paused = true)]
async fn unconfirmed_preparation_is_dead_and_adjacent_work_continues() {
    let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let bad = enqueue(&store, tenant(1), 2).await;
    let good = enqueue(&store, tenant(2), 2).await;
    let report = worker(
        Arc::clone(&store),
        Behavior::UncooperativeFor(tenant(1), Duration::from_millis(200)),
        seen,
        2,
    )
    .drain_once()
    .await
    .expect("drain");
    assert_eq!(report.dead, 1);
    assert_eq!(report.completed, 1, "adjacent job must still run");
    assert_eq!(
        store
            .get_job(TenantContext::from_authenticated_session(tenant(1)), bad)
            .await
            .expect("view")
            .expect("job")
            .state,
        JobState::Dead
    );
    assert_eq!(
        store
            .get_job(TenantContext::from_authenticated_session(tenant(2)), good)
            .await
            .expect("view")
            .expect("job")
            .state,
        JobState::Completed
    );
}

#[tokio::test]
async fn stale_commit_is_refused_after_lease_closes() {
    let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let job = enqueue(&store, tenant(1), 2).await;
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let commit = Arc::new(MemoryCommitter {
        store: Arc::clone(&store),
        visible: Arc::new(Mutex::new(BTreeSet::new())),
        pause_before_commit: Some(Arc::clone(&started)),
        release_commit: Some(Arc::clone(&release)),
        fail_after_effect_once: AtomicBool::new(false),
    });
    let worker = Worker::new(
        Arc::clone(&store),
        registry(
            Behavior::Success,
            Arc::new(Mutex::new(Vec::new())),
            Arc::clone(&commit),
        ),
        WorkerSettings::new(2, Duration::from_millis(30), 1).expect("settings"),
    );
    let drain = tokio::spawn(async move { worker.drain_once().await });
    started.notified().await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_001))
        .expect("advance");
    let reclaimed = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(2).expect("lease"),
        )
        .await
        .expect("claim")
        .expect("reclaimed");
    release.notify_one();
    let report = drain.await.expect("join").expect("drain");
    assert_eq!(report.finalization_failed, 1);
    assert!(
        commit.visible.lock().expect("test lock").is_empty(),
        "stale conditional commit must not make an effect visible"
    );
    store
        .complete_job(reclaimed.id, reclaimed.lease_token)
        .await
        .expect("current claim completes");
    assert_eq!(reclaimed.id, job);
}

#[tokio::test]
async fn crash_recovery_reuses_job_idempotency_key_without_duplicate_effect() {
    let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let job = enqueue(&store, tenant(1), 2).await;
    let commit = committer(Arc::clone(&store));
    commit.fail_after_effect_once.store(true, Ordering::Release);
    let settings = WorkerSettings::new(2, Duration::from_millis(30), 1).expect("settings");
    let first = Worker::new(
        Arc::clone(&store),
        registry(
            Behavior::Success,
            Arc::new(Mutex::new(Vec::new())),
            Arc::clone(&commit),
        ),
        settings,
    )
    .drain_once()
    .await
    .expect("first");
    assert_eq!(first.finalization_failed, 1);
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_001))
        .expect("advance");
    let second = Worker::new(
        Arc::clone(&store),
        registry(
            Behavior::Success,
            Arc::new(Mutex::new(Vec::new())),
            Arc::clone(&commit),
        ),
        settings,
    )
    .drain_once()
    .await
    .expect("second");
    assert_eq!(second.completed, 1);
    assert_eq!(commit.visible.lock().expect("test lock").len(), 1);
    assert!(commit.visible.lock().expect("test lock").contains(&job));
}

#[test]
fn settings_reject_timeout_that_cannot_leave_cancellation_grace() {
    assert!(WorkerSettings::new(2, Duration::from_secs(2), 1).is_err());
    assert!(WorkerSettings::new(2, Duration::ZERO, 1).is_err());
}

#[test]
fn registry_rejects_empty_and_duplicate_families() {
    assert!(JobRegistry::new([]).is_err());

    let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let commit = committer(store);
    let handler: Arc<dyn JobHandler> = Arc::new(RecordingHandler {
        behavior: Behavior::Success,
        tenants: Arc::new(Mutex::new(Vec::new())),
    });
    let first_committer: Arc<dyn EffectCommitter> = commit.clone();
    let second_committer: Arc<dyn EffectCommitter> = commit;
    assert!(
        JobRegistry::new([
            JobRegistryEntry::new(JobKind::Render, Arc::clone(&handler), first_committer),
            JobRegistryEntry::new(JobKind::Render, handler, second_committer),
        ])
        .is_err()
    );
}
