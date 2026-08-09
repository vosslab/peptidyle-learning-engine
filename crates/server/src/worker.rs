//! Bounded execution for the closed durable worker queue.
//!
//! Work is deliberately split in two. A handler performs only cancellable,
//! replay-safe preparation and returns an immutable prepared-effect reference.
//! A sealed server-owned committer then makes that effect visible and completes
//! the exact job claim in one durable conditional transaction. Preparation
//! never writes educational records or externally visible results.

#![allow(dead_code)] // Composition wiring follows the worker contract package.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use question_model::{
    AssignmentId, ObjectId, QuestionAttemptId, ScoringGeneration, TenantId, WorkspaceId,
    WorkspaceImportId,
};
use store::{
    ExportArtifactRecord, JobFailureKind, JobId, JobLeaseDuration, JobLeaseToken, JobPayload,
    JobStore, QueueDepth, StoreError, TenantContext,
};

/// The only server-owned types allowed to implement the durable commit sink.
///
/// A production implementation belongs beside the storage transaction that
/// writes the effect. It verifies the active `jobs` row's exact lease token,
/// records the effect with `JobId` as its idempotency key, and marks that same
/// job completed in the *same* transaction.
pub(crate) mod sealed {
    pub trait EffectCommitter {}
}

/// Cancellable preparation of one closed worker payload family.
#[async_trait]
pub(crate) trait JobHandler: Send + Sync + 'static {
    /// Resolves immutable inputs and creates only replay-safe private output.
    ///
    /// Implementations select on [`JobExecution::cancelled`] around I/O,
    /// terminate child processes/requests on cancellation, and never make the
    /// result visible here. Blocking worker threads and detached work are not
    /// permitted. The returned object must already be immutable and safe to
    /// orphan if the claim is later lost.
    async fn prepare(
        &self,
        context: TenantContext,
        payload: JobPayload,
        execution: JobExecution,
    ) -> Result<PreparedJobEffect, JobFailureKind>;
}

/// Cooperative cancellation state for preparation only.
#[derive(Clone)]
pub(crate) struct JobExecution {
    cancellation_requested: Arc<AtomicBool>,
    cancellation_notice: Arc<tokio::sync::Notify>,
    claim: Option<JobCommitClaim>,
}

impl JobExecution {
    pub(crate) fn new() -> Self {
        Self {
            cancellation_requested: Arc::new(AtomicBool::new(false)),
            cancellation_notice: Arc::new(tokio::sync::Notify::new()),
            claim: None,
        }
    }

    fn with_claim(mut self, claim: JobCommitClaim) -> Self {
        self.claim = Some(claim);
        self
    }

    pub(crate) fn claim(&self) -> Option<JobCommitClaim> {
        self.claim
    }

    /// Resolves once the worker has requested a cooperative stop.
    pub(crate) async fn cancelled(&self) {
        while !self.cancellation_requested() {
            self.cancellation_notice.notified().await;
        }
    }

    /// Lets cancellable I/O take a cheap nonblocking path.
    pub(crate) fn cancellation_requested(&self) -> bool {
        self.cancellation_requested.load(Ordering::Acquire)
    }

    fn request_cancellation(&self) {
        self.cancellation_requested.store(true, Ordering::Release);
        self.cancellation_notice.notify_one();
    }
}

/// A safe, immutable prepared result; it is not yet visible to learners.
///
/// Raw bytes, credentials, answer keys, and arbitrary URLs are prohibited.
/// The concrete producer later resolves this object in its commit transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreparedJobEffect {
    /// Current-score rows staged for one assignment generation.
    AssignmentScoring {
        tenant: TenantId,
        assignment: AssignmentId,
        generation: ScoringGeneration,
    },
    /// Server-owned deadline transition requiring no external preparation.
    AttemptAutoSubmit {
        tenant: TenantId,
        attempt: QuestionAttemptId,
        timing_generation: u64,
    },
    /// A retention effect whose external cleanup has completed; the Store
    /// still owns the lease-conditional lifecycle and queue finalization.
    Retention {
        command: store::RetentionWorkerCommand,
    },
    /// Immutable renderer output awaiting cache registration.
    Render { artifact: ObjectId },
    /// Four immutable export artifacts awaiting one target-record linkage.
    ///
    /// A bundle is indivisible: a partial print set is never a visible student
    /// record. The records contain only verified object metadata, never bytes.
    Export {
        tenant: TenantId,
        manifest: ObjectId,
        artifacts: Box<PreparedExportArtifacts>,
    },
    /// Immutable parsed/imported artifact awaiting publication linkage.
    Import { artifact: ObjectId },
    /// Private QTI registry awaiting the exact queue-claim commit.
    QtiImport {
        tenant: TenantId,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        source_object: ObjectId,
    },
}

/// The closed four-artifact effect produced by one assignment export job.
///
/// Named fields make omission and duplication unrepresentable before the
/// storage committer performs its independent exact-set validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedExportArtifacts {
    pub(crate) docx: ExportArtifactRecord,
    pub(crate) pdf: ExportArtifactRecord,
    pub(crate) accessible_docx: ExportArtifactRecord,
    pub(crate) accessible_pdf: ExportArtifactRecord,
}

impl PreparedExportArtifacts {
    /// Returns the canonical closed artifact order consumed by the Store.
    pub(crate) fn into_records(self) -> Vec<ExportArtifactRecord> {
        vec![
            self.docx,
            self.pdf,
            self.accessible_docx,
            self.accessible_pdf,
        ]
    }
}

/// Exact broker capability supplied only to the durable finalization boundary.
#[derive(Clone, Copy)]
pub(crate) struct JobCommitClaim {
    id: JobId,
    token: JobLeaseToken,
}

impl JobCommitClaim {
    pub(crate) fn new(id: JobId, token: JobLeaseToken) -> Self {
        Self { id, token }
    }
    /// Immutable idempotency key for the visible effect.
    pub(crate) fn job_id(self) -> JobId {
        self.id
    }

    /// Opaque active-lease capability. Never serialize or expose to a browser.
    pub(crate) fn lease_token(self) -> JobLeaseToken {
        self.token
    }
}

/// Result of a single durable effect-and-complete transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectCommitOutcome {
    /// Effect became visible and this exact claim was completed atomically.
    Committed,
    /// The claimed execution safely moved the same durable job into the future.
    Rescheduled,
    /// Claim expired/reclaimed before the conditional transaction could commit.
    ClaimNoLongerActive,
}

/// Server-owned final visibility boundary for prepared effects.
///
/// `commit` is not a check-then-act API. An implementation must use the exact
/// `JobCommitClaim` in the same transaction/conditional write that both makes
/// the effect visible and marks the job complete. An unavailable/unknown
/// result is left leased for safe recovery; the worker never releases it for a
/// second preparation on an ambiguous commit.
#[async_trait]
pub(crate) trait EffectCommitter: sealed::EffectCommitter + Send + Sync + 'static {
    async fn commit(
        &self,
        claim: JobCommitClaim,
        effect: PreparedJobEffect,
    ) -> Result<EffectCommitOutcome, StoreError>;
}

/// Closed registry for initial preparation families.
#[derive(Clone)]
pub(crate) struct JobHandlers {
    scoring: Arc<dyn JobHandler>,
    timing: Arc<dyn JobHandler>,
    render: Arc<dyn JobHandler>,
    export: Arc<dyn JobHandler>,
    import: Arc<dyn JobHandler>,
    qti_import: Arc<dyn JobHandler>,
    retention: Arc<dyn JobHandler>,
}

impl JobHandlers {
    pub(crate) fn new(
        scoring: Arc<dyn JobHandler>,
        timing: Arc<dyn JobHandler>,
        render: Arc<dyn JobHandler>,
        export: Arc<dyn JobHandler>,
        import: Arc<dyn JobHandler>,
        qti_import: Arc<dyn JobHandler>,
        retention: Arc<dyn JobHandler>,
    ) -> Self {
        Self {
            scoring,
            timing,
            render,
            export,
            import,
            qti_import,
            retention,
        }
    }

    fn for_payload(&self, payload: &JobPayload) -> Arc<dyn JobHandler> {
        match payload {
            JobPayload::RecalculateAssignment { .. } => Arc::clone(&self.scoring),
            JobPayload::AutoSubmitAttempt { .. } => Arc::clone(&self.timing),
            JobPayload::Retention { .. } => Arc::clone(&self.retention),
            JobPayload::Render { .. } => Arc::clone(&self.render),
            JobPayload::Export { .. } => Arc::clone(&self.export),
            JobPayload::Import { .. } => Arc::clone(&self.import),
            JobPayload::QtiImport { .. } => Arc::clone(&self.qti_import),
        }
    }
}

/// Validated bounds for one worker process.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WorkerSettings {
    lease: JobLeaseDuration,
    preparation_timeout: Duration,
    cancellation_grace: Duration,
    batch_size: usize,
}

impl WorkerSettings {
    /// Creates bounds leaving a cancellation confirmation interval inside lease.
    pub(crate) fn new(
        lease_seconds: u32,
        preparation_timeout: Duration,
        batch_size: usize,
    ) -> Result<Self, StoreError> {
        let lease = JobLeaseDuration::from_seconds(lease_seconds)?;
        if preparation_timeout.is_zero()
            || preparation_timeout >= Duration::from_secs(u64::from(lease_seconds))
        {
            return Err(StoreError::InvalidRecord(
                "worker preparation timeout must be positive and shorter than its lease"
                    .to_string(),
            ));
        }
        if !(1..=100).contains(&batch_size) {
            return Err(StoreError::InvalidRecord(
                "worker batch size must be between 1 and 100".to_string(),
            ));
        }
        let remaining = Duration::from_secs(u64::from(lease_seconds)) - preparation_timeout;
        Ok(Self {
            lease,
            preparation_timeout,
            cancellation_grace: remaining.div_f64(2.0).min(Duration::from_millis(50)),
            batch_size,
        })
    }
}

/// Outcome counters for one bounded polling pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DrainReport {
    pub(crate) completed: u32,
    pub(crate) rescheduled: u32,
    pub(crate) retrying: u32,
    pub(crate) dead: u32,
    /// Ambiguous or stale finalization was intentionally not retried locally.
    pub(crate) finalization_failed: u32,
}

/// Stateless worker; competing instances rely on the broker's atomic claim.
#[derive(Clone)]
pub(crate) struct Worker<S, C> {
    store: Arc<S>,
    handlers: JobHandlers,
    committer: Arc<C>,
    settings: WorkerSettings,
}

impl<S, C> Worker<S, C>
where
    S: JobStore + 'static,
    C: EffectCommitter,
{
    pub(crate) fn new(
        store: Arc<S>,
        handlers: JobHandlers,
        committer: Arc<C>,
        settings: WorkerSettings,
    ) -> Self {
        Self {
            store,
            handlers,
            committer,
            settings,
        }
    }

    /// Claims and processes at most one bounded batch, with no heartbeat.
    pub(crate) async fn drain_once(&self) -> Result<DrainReport, StoreError> {
        let mut report = DrainReport::default();
        for _ in 0..self.settings.batch_size {
            let Some(claimed) = self.store.claim_next_job(self.settings.lease).await? else {
                break;
            };
            let claim = JobCommitClaim::new(claimed.id, claimed.lease_token);
            let execution = JobExecution::new().with_claim(claim);
            let handler = self.handlers.for_payload(&claimed.payload);
            let context = TenantContext::from_authenticated_session(claimed.tenant);
            let payload = claimed.payload.clone();
            let handler_execution = execution.clone();
            let mut task =
                tokio::spawn(
                    async move { handler.prepare(context, payload, handler_execution).await },
                );

            let prepared = match tokio::time::timeout(self.settings.preparation_timeout, &mut task)
                .await
            {
                Ok(Ok(Ok(effect))) => Some(effect),
                Ok(Ok(Err(failure))) => {
                    self.finalize_failure(&mut report, claimed.id, claimed.lease_token, failure)
                        .await;
                    None
                }
                Ok(Err(_panic)) => {
                    self.finalize_failure(
                        &mut report,
                        claimed.id,
                        claimed.lease_token,
                        JobFailureKind::Transient,
                    )
                    .await;
                    None
                }
                Err(_elapsed) => {
                    execution.request_cancellation();
                    match tokio::time::timeout(self.settings.cancellation_grace, &mut task).await {
                        // Preparation has stopped; it made no visible effect, so retry is safe.
                        Ok(_) => {
                            self.finalize_failure(
                                &mut report,
                                claimed.id,
                                claimed.lease_token,
                                JobFailureKind::TimedOut,
                            )
                            .await
                        }
                        Err(_) => {
                            // Do not release an unconfirmed preparer. Its outer task is awaited
                            // after abort, and the active lease is made terminal.
                            task.abort();
                            let _ = task.await;
                            self.finalize_failure(
                                &mut report,
                                claimed.id,
                                claimed.lease_token,
                                JobFailureKind::Permanent,
                            )
                            .await;
                        }
                    }
                    None
                }
            };
            let Some(effect) = prepared else {
                continue;
            };

            // This is the only visibility path. The committer owns one atomic
            // conditional transaction: active claim token + effect + completion.
            match self.committer.commit(claim, effect).await {
                Ok(EffectCommitOutcome::Committed) => report.completed += 1,
                Ok(EffectCommitOutcome::Rescheduled) => report.rescheduled += 1,
                Ok(EffectCommitOutcome::ClaimNoLongerActive) | Err(_) => {
                    // An uncertain commit is not failed/retried here: recovery uses
                    // the durable idempotency key after the lease resolves.
                    report.finalization_failed += 1;
                }
            }
        }
        Ok(report)
    }

    async fn finalize_failure(
        &self,
        report: &mut DrainReport,
        id: JobId,
        token: JobLeaseToken,
        failure: JobFailureKind,
    ) {
        match self.store.fail_job(id, token, failure).await {
            Ok(store::JobFailureDisposition::Retrying) => report.retrying += 1,
            Ok(store::JobFailureDisposition::Dead) => report.dead += 1,
            Err(_) => report.finalization_failed += 1,
        }
    }

    pub(crate) async fn ready_queue_depth(&self) -> Result<QueueDepth, StoreError> {
        self.store.ready_queue_depth().await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
    };

    use objects::{Bucket, ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
    use question_model::{ActivityTimestamp, ProblemId, ProblemVersionRef, TenantId, VersionId};
    use store::{
        EnqueueJob, ExportArtifactKind, ExportArtifactRecord, JobLeaseDuration, JobState,
        TenantContext,
    };
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
                Behavior::UncooperativeFor(expected, duration)
                    if expected == context.tenant_id() =>
                {
                    tokio::time::sleep(duration).await;
                    Ok(effect_for(payload))
                }
                Behavior::UncooperativeFor(_, _) => Ok(effect_for(payload)),
            }
        }
    }

    struct MemoryCommitter {
        store: Arc<store::memory::MemoryStore>,
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
                    "simulated crash after durable effect".to_string(),
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
        let artifact = ObjectId::from_uuid(id(9_000));
        match payload {
            JobPayload::RecalculateAssignment {
                assignment,
                generation,
            } => PreparedJobEffect::AssignmentScoring {
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
            JobPayload::Render { .. } => PreparedJobEffect::Render { artifact },
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
            JobPayload::Import { .. } => PreparedJobEffect::Import { artifact },
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
    fn committer(store: Arc<store::memory::MemoryStore>) -> Arc<MemoryCommitter> {
        Arc::new(MemoryCommitter {
            store,
            visible: Arc::new(Mutex::new(BTreeSet::new())),
            pause_before_commit: None,
            release_commit: None,
            fail_after_effect_once: AtomicBool::new(false),
        })
    }
    fn handlers(behavior: Behavior, tenants: Arc<Mutex<Vec<TenantId>>>) -> JobHandlers {
        let handler: Arc<dyn JobHandler> = Arc::new(RecordingHandler { behavior, tenants });
        JobHandlers::new(
            Arc::clone(&handler),
            Arc::clone(&handler),
            Arc::clone(&handler),
            Arc::clone(&handler),
            Arc::clone(&handler),
            Arc::clone(&handler),
            handler,
        )
    }
    fn worker(
        store: Arc<store::memory::MemoryStore>,
        behavior: Behavior,
        tenants: Arc<Mutex<Vec<TenantId>>>,
        batch_size: usize,
    ) -> Worker<store::memory::MemoryStore, MemoryCommitter> {
        let commit = committer(Arc::clone(&store));
        Worker::new(
            store,
            handlers(behavior, tenants),
            commit,
            WorkerSettings::new(2, Duration::from_millis(30), batch_size).expect("settings"),
        )
    }
    async fn enqueue(store: &store::memory::MemoryStore, tenant: TenantId, attempts: u16) -> JobId {
        store
            .enqueue_job(
                TenantContext::from_authenticated_session(tenant),
                render_job(tenant, attempts),
            )
            .await
            .expect("enqueue")
    }

    #[tokio::test]
    async fn concurrent_workers_claim_distinct_jobs_and_drain_depth() {
        let store = Arc::new(store::memory::MemoryStore::default());
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

    #[tokio::test]
    async fn cooperative_timeout_stops_preparation_before_retry() {
        let store = Arc::new(store::memory::MemoryStore::default());
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

    #[tokio::test]
    async fn unconfirmed_preparation_is_dead_and_adjacent_work_continues() {
        let store = Arc::new(store::memory::MemoryStore::default());
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
        let store = Arc::new(store::memory::MemoryStore::default());
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
            handlers(Behavior::Success, Arc::new(Mutex::new(Vec::new()))),
            Arc::clone(&commit),
            WorkerSettings::new(2, Duration::from_millis(30), 1).expect("settings"),
        );
        let drain = tokio::spawn(async move { worker.drain_once().await });
        started.notified().await;
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_001))
            .expect("advance");
        let reclaimed = store
            .claim_next_job(JobLeaseDuration::from_seconds(2).expect("lease"))
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
        let store = Arc::new(store::memory::MemoryStore::default());
        let job = enqueue(&store, tenant(1), 2).await;
        let commit = committer(Arc::clone(&store));
        commit.fail_after_effect_once.store(true, Ordering::Release);
        let handlers = handlers(Behavior::Success, Arc::new(Mutex::new(Vec::new())));
        let settings = WorkerSettings::new(2, Duration::from_millis(30), 1).expect("settings");
        let first = Worker::new(
            Arc::clone(&store),
            handlers.clone(),
            Arc::clone(&commit),
            settings,
        )
        .drain_once()
        .await
        .expect("first");
        assert_eq!(first.finalization_failed, 1);
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_001))
            .expect("advance");
        let second = Worker::new(Arc::clone(&store), handlers, Arc::clone(&commit), settings)
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
}
