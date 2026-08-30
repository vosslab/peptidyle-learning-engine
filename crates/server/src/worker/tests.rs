use std::{
    collections::{BTreeSet, VecDeque},
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
use question_model::{
    ActivityTimestamp, CourseId, ProblemId, ProblemVersionRef, TenantId, VersionId,
};
use uuid::Uuid;

use super::*;

#[derive(Clone)]
enum Behavior {
    Success,
    CooperativeSleep(Duration),
    UncooperativeFor(Duration),
}

#[derive(Clone)]
struct RecordingHandler {
    behavior: Behavior,
    claims: Arc<Mutex<Vec<JobId>>>,
}

#[async_trait]
impl JobHandler for RecordingHandler {
    async fn prepare(
        &self,
        payload: JobPayload,
        execution: JobExecution,
    ) -> Result<PreparedJobEffect, JobFailureKind> {
        let claim = execution.claim().expect("worker supplies a lease claim");
        self.claims.lock().expect("test lock").push(claim.job_id());
        match self.behavior {
            Behavior::Success => Ok(effect_for(payload)),
            Behavior::CooperativeSleep(duration) => tokio::select! {
                () = tokio::time::sleep(duration) => Ok(effect_for(payload)),
                () = execution.cancelled() => Err(JobFailureKind::TimedOut),
            },
            Behavior::UncooperativeFor(duration) => {
                tokio::time::sleep(duration).await;
                Ok(effect_for(payload))
            }
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
        JobPayload::GradeAcceptedSubmission { .. } => {
            panic!("recording handler must not receive accepted-submission grading work")
        }
        JobPayload::RecalculateAssignment {
            assignment,
            generation,
        } => PreparedJobEffect::AssignmentScoring {
            assignment,
            generation,
        },
        JobPayload::RecalculateCourseItemAnalysis {
            assignment,
            generation,
        } => PreparedJobEffect::CourseItemAnalysis {
            assignment,
            generation,
        },
        JobPayload::AutoSubmitAttempt {
            attempt,
            timing_generation,
        } => PreparedJobEffect::AttemptAutoSubmit {
            attempt,
            timing_generation,
        },
        JobPayload::Retention { .. } => {
            panic!("recording handler must not receive retention work")
        }
        JobPayload::Render { .. } => PreparedJobEffect::Test,
        JobPayload::Export { delivery_object } => PreparedJobEffect::Export {
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
        course: CourseId::from_uuid(id(1)),
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
fn render_job(max_attempts: u16) -> EnqueueJob {
    EnqueueJob {
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
    claims: Arc<Mutex<Vec<JobId>>>,
    committer: Arc<MemoryCommitter>,
) -> JobRegistry {
    let handler: Arc<dyn JobHandler> = Arc::new(RecordingHandler { behavior, claims });
    let committer: Arc<dyn EffectCommitter> = committer;
    JobRegistry::new([JobRegistryEntry::new(JobKind::Render, handler, committer)])
        .expect("test registry")
}
fn worker(
    store: Arc<learning_data_access::in_memory::MemoryStore>,
    behavior: Behavior,
    claims: Arc<Mutex<Vec<JobId>>>,
    batch_size: usize,
) -> Worker<learning_data_access::in_memory::MemoryStore> {
    let commit = committer(Arc::clone(&store));
    Worker::new(
        store,
        registry(behavior, claims, commit),
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
            render_job(attempts),
        )
        .await
        .expect("enqueue")
}

#[derive(Clone, Copy)]
enum GenericDispatchOutcome {
    Idle,
    Claimed,
    Error,
}

struct RecordingGenericDispatch {
    outcomes: Mutex<VecDeque<GenericDispatchOutcome>>,
    calls: Arc<Mutex<Vec<&'static str>>>,
}

#[async_trait]
impl GenericOneClaimDrain for RecordingGenericDispatch {
    async fn drain_one(&self) -> Result<DrainReport, StoreError> {
        self.calls.lock().expect("call log").push("generic");
        match self
            .outcomes
            .lock()
            .expect("generic outcomes")
            .pop_front()
            .expect("generic outcome")
        {
            GenericDispatchOutcome::Idle => Ok(DrainReport::default()),
            GenericDispatchOutcome::Claimed => Ok(DrainReport {
                completed: 1,
                ..DrainReport::default()
            }),
            GenericDispatchOutcome::Error => {
                Err(StoreError::Unavailable("generic claim failed".to_string()))
            }
        }
    }
}

#[derive(Clone, Copy)]
enum AcceptedDispatchOutcome {
    Claimed,
    OutcomeUnknown,
}

struct RecordingAcceptedDispatch {
    outcomes: Mutex<VecDeque<AcceptedDispatchOutcome>>,
    calls: Arc<Mutex<Vec<&'static str>>>,
}

#[async_trait]
impl AcceptedOneClaimDrain for RecordingAcceptedDispatch {
    async fn drain_one(&self) -> Result<AcceptedSubmissionExecutionWorkerReport, StoreError> {
        self.calls.lock().expect("call log").push("accepted");
        let report = match self
            .outcomes
            .lock()
            .expect("accepted outcomes")
            .pop_front()
            .expect("accepted outcome")
        {
            AcceptedDispatchOutcome::Claimed => AcceptedSubmissionExecutionWorkerReport {
                committed: 1,
                ..AcceptedSubmissionExecutionWorkerReport::default()
            },
            AcceptedDispatchOutcome::OutcomeUnknown => AcceptedSubmissionExecutionWorkerReport {
                outcome_unknown: 1,
                ..AcceptedSubmissionExecutionWorkerReport::default()
            },
        };
        Ok(report)
    }
}

fn fair_dispatcher(
    generic: impl IntoIterator<Item = GenericDispatchOutcome>,
    accepted: impl IntoIterator<Item = AcceptedDispatchOutcome>,
) -> (
    FairWorkerDispatcher<RecordingGenericDispatch, RecordingAcceptedDispatch>,
    Arc<Mutex<Vec<&'static str>>>,
) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let generic = RecordingGenericDispatch {
        outcomes: Mutex::new(generic.into_iter().collect()),
        calls: Arc::clone(&calls),
    };
    let accepted = RecordingAcceptedDispatch {
        outcomes: Mutex::new(accepted.into_iter().collect()),
        calls: Arc::clone(&calls),
    };
    (FairWorkerDispatcher::new(generic, accepted), calls)
}

#[tokio::test]
async fn fair_dispatcher_alternates_its_preferred_family_across_passes() {
    let (dispatcher, calls) = fair_dispatcher(
        [
            GenericDispatchOutcome::Claimed,
            GenericDispatchOutcome::Claimed,
        ],
        [AcceptedDispatchOutcome::Claimed],
    );

    let first = dispatcher.drain_once().await.expect("first pass");
    let second = dispatcher.drain_once().await.expect("second pass");
    let third = dispatcher.drain_once().await.expect("third pass");

    assert_eq!(first.generic.expect("generic claim").completed, 1);
    assert_eq!(second.accepted.expect("accepted claim").committed, 1);
    assert_eq!(third.generic.expect("generic claim").completed, 1);
    assert_eq!(
        calls.lock().expect("call log").as_slice(),
        ["generic", "accepted", "generic"]
    );
}

#[tokio::test]
async fn fair_dispatcher_falls_back_once_when_the_preferred_family_is_idle() {
    let (dispatcher, calls) = fair_dispatcher(
        [GenericDispatchOutcome::Idle],
        [AcceptedDispatchOutcome::Claimed],
    );

    let report = dispatcher.drain_once().await.expect("fallback pass");

    assert_eq!(report.generic.expect("idle generic").completed, 0);
    assert_eq!(report.accepted.expect("accepted claim").committed, 1);
    assert_eq!(
        calls.lock().expect("call log").as_slice(),
        ["generic", "accepted"]
    );
}

#[tokio::test]
async fn fair_dispatcher_stops_on_a_store_error_without_fallback() {
    let (dispatcher, calls) = fair_dispatcher(
        [GenericDispatchOutcome::Error],
        [AcceptedDispatchOutcome::Claimed],
    );

    assert!(matches!(
        dispatcher.drain_once().await,
        Err(StoreError::Unavailable(message)) if message == "generic claim failed"
    ));
    assert_eq!(calls.lock().expect("call log").as_slice(), ["generic"]);
}

#[tokio::test]
async fn fair_dispatcher_treats_an_ambiguous_accepted_outcome_as_its_one_claim() {
    let (dispatcher, calls) = fair_dispatcher(
        [GenericDispatchOutcome::Claimed],
        [AcceptedDispatchOutcome::OutcomeUnknown],
    );
    let _first = dispatcher.drain_once().await.expect("generic pass");
    let second = dispatcher.drain_once().await.expect("accepted pass");

    assert_eq!(second.accepted.expect("accepted result").outcome_unknown, 1);
    assert_eq!(
        calls.lock().expect("call log").as_slice(),
        ["generic", "accepted"]
    );
}

#[tokio::test]
async fn generic_drain_one_ignores_the_configured_batch_size() {
    let store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let first = enqueue(&store, tenant(41), 2).await;
    let second = enqueue(&store, tenant(42), 2).await;
    let worker = worker(
        Arc::clone(&store),
        Behavior::Success,
        Arc::new(Mutex::new(Vec::new())),
        100,
    );

    assert_eq!(worker.drain_one().await.expect("one claim").completed, 1);
    let first_state = store
        .get_job(first)
        .await
        .expect("first view")
        .expect("first job")
        .state;
    let second_state = store
        .get_job(second)
        .await
        .expect("second view")
        .expect("second job")
        .state;
    assert!(matches!(
        (first_state, second_state),
        (JobState::Completed, JobState::Ready) | (JobState::Ready, JobState::Completed)
    ));
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
        claims: Arc::new(Mutex::new(Vec::new())),
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
            .get_job(supported)
            .await
            .expect("supported view")
            .expect("supported job")
            .state,
        JobState::Completed
    );
    assert_eq!(
        store
            .get_job(reserved)
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
    let claims = seen.lock().expect("test lock").clone();
    assert_eq!(claims.len(), 2);
    assert_ne!(claims[0], claims[1]);
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
        store.get_job(job).await.expect("view").expect("job").state,
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
        Behavior::UncooperativeFor(Duration::from_millis(200)),
        seen,
        2,
    )
    .drain_once()
    .await
    .expect("drain");
    assert_eq!(report.dead, 1);
    assert_eq!(report.completed, 1, "adjacent job must still run");
    assert_eq!(
        store.get_job(bad).await.expect("view").expect("job").state,
        JobState::Dead
    );
    assert_eq!(
        store.get_job(good).await.expect("view").expect("job").state,
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
        claims: Arc::new(Mutex::new(Vec::new())),
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
