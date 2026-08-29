//! Bounded execution for the closed durable worker queue.
//!
//! Work is deliberately split in two. A handler performs only cancellable,
//! replay-safe preparation and returns an immutable prepared-effect reference.
//! A sealed server-owned committer then makes that effect visible and completes
//! the exact job claim in one durable conditional transaction. Preparation
//! never writes educational records or externally visible results.

pub(crate) mod runtime;

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use learning_data_access::{
    ClaimedJob, ExportArtifactRecord, JobClaimFilter, JobFailureKind, JobId, JobKind,
    JobLeaseDuration, JobLeaseToken, JobPayload, JobStore, QueueDepth, StoreError, TenantContext,
};
use question_model::{
    AssignmentId, ObjectId, QuestionAttemptId, ScoringGeneration, TenantId, WorkspaceId,
    WorkspaceImportId,
};

use crate::accepted_submission_worker::AcceptedSubmissionExecutionWorkerReport;

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

    #[cfg(test)]
    pub(crate) fn with_test_claim(self, claim: JobCommitClaim) -> Self {
        self.with_claim(claim)
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
    /// Current course-local item-analysis rows staged after scoring has published.
    CourseItemAnalysis {
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
        command: learning_data_access::RetentionWorkerCommand,
    },
    /// Four immutable export artifacts awaiting one target-record linkage.
    ///
    /// A bundle is indivisible: a partial print set is never a visible student
    /// record. The records contain only verified object metadata, never bytes.
    Export {
        tenant: TenantId,
        manifest: ObjectId,
        artifacts: Box<PreparedExportArtifacts>,
    },
    /// Private QTI registry awaiting the exact queue-claim commit.
    QtiImport {
        tenant: TenantId,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        source_object: ObjectId,
    },
    /// Committed-publication asset bytes have been materialized at the exact
    /// immutable public keys and await the lease-conditional registry flip.
    PublicAssetPublication {
        reference: question_model::ProblemVersionRef,
    },
    /// Opaque effect used only to exercise the generic worker lifecycle.
    #[cfg(test)]
    Test,
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

/// One complete worker family. A family cannot be claimable without both
/// preparation and durable finalization behavior.
#[derive(Clone)]
struct JobFamily {
    handler: Arc<dyn JobHandler>,
    committer: Arc<dyn EffectCommitter>,
}

/// One composition-root registration for a complete job family.
pub(crate) struct JobRegistryEntry {
    kind: JobKind,
    family: JobFamily,
}

impl JobRegistryEntry {
    pub(crate) fn new(
        kind: JobKind,
        handler: Arc<dyn JobHandler>,
        committer: Arc<dyn EffectCommitter>,
    ) -> Self {
        Self {
            kind,
            family: JobFamily { handler, committer },
        }
    }
}

/// Closed registry whose keys are also the queue broker's mandatory filter.
/// Reserved payload variants remain queued until a complete family is added.
#[derive(Clone)]
pub(crate) struct JobRegistry {
    families: BTreeMap<JobKind, JobFamily>,
    filter: JobClaimFilter,
}

impl JobRegistry {
    pub(crate) fn new(
        entries: impl IntoIterator<Item = JobRegistryEntry>,
    ) -> Result<Self, StoreError> {
        let mut families = BTreeMap::new();
        for entry in entries {
            if families.insert(entry.kind, entry.family).is_some() {
                return Err(StoreError::InvalidRecord(format!(
                    "worker registry contains duplicate family {}",
                    entry.kind.database_name()
                )));
            }
        }
        let filter = JobClaimFilter::new(families.keys().copied())?;
        Ok(Self { families, filter })
    }

    fn for_payload(&self, payload: &JobPayload) -> Option<JobFamily> {
        self.families.get(&payload.kind()).cloned()
    }

    fn claim_filter(&self) -> &JobClaimFilter {
        &self.filter
    }
}

/// Validated bounds for one worker process.
#[derive(Debug, Clone, Copy)]
pub struct WorkerSettings {
    lease: JobLeaseDuration,
    preparation_timeout: Duration,
    cancellation_grace: Duration,
    batch_size: usize,
}

impl WorkerSettings {
    /// Creates bounds leaving a cancellation confirmation interval inside lease.
    pub fn new(
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

    /// Returns the validated lease shared by capability-scoped worker families.
    pub(crate) fn lease(&self) -> JobLeaseDuration {
        self.lease
    }

    /// Returns the validated direct-handler deadline within that lease.
    pub(crate) fn execution_deadline(&self) -> Duration {
        self.preparation_timeout
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

impl DrainReport {
    pub(crate) fn claimed(&self) -> bool {
        self.completed + self.rescheduled + self.retrying + self.dead + self.finalization_failed > 0
    }

    fn include(&mut self, other: Self) {
        self.completed += other.completed;
        self.rescheduled += other.rescheduled;
        self.retrying += other.retrying;
        self.dead += other.dead;
        self.finalization_failed += other.finalization_failed;
    }
}

/// One bounded generic claim attempt used by the fair seven-family dispatcher.
#[cfg_attr(not(test), allow(dead_code))]
#[async_trait]
pub(crate) trait GenericOneClaimDrain: Send + Sync {
    async fn drain_one(&self) -> Result<DrainReport, StoreError>;
}

/// One bounded sealed claim attempt used by the fair seven-family dispatcher.
#[cfg_attr(not(test), allow(dead_code))]
#[async_trait]
pub(crate) trait AcceptedOneClaimDrain: Send + Sync {
    async fn drain_one(&self) -> Result<AcceptedSubmissionExecutionWorkerReport, StoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
enum DispatchFamily {
    Generic,
    Accepted,
}

#[cfg_attr(not(test), allow(dead_code))]
impl DispatchFamily {
    fn other(self) -> Self {
        match self {
            Self::Generic => Self::Accepted,
            Self::Accepted => Self::Generic,
        }
    }
}

/// Answer-free reports for one fair seven-family dispatch pass.
///
/// Each family retains its native report shape. `None` means that family was
/// not attempted during this pass; an attempted idle side reports its normal
/// empty result. At most one report can describe an actual claim.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct FairWorkerDispatchReport {
    pub(crate) generic: Option<DrainReport>,
    pub(crate) accepted: Option<AcceptedSubmissionExecutionWorkerReport>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl FairWorkerDispatchReport {
    fn claimed_count(&self) -> u32 {
        let generic_claimed = self.generic.as_ref().is_some_and(DrainReport::claimed);
        let accepted_claimed = self
            .accepted
            .as_ref()
            .is_some_and(|report| report.no_claim == 0);
        u32::from(generic_claimed) + u32::from(accepted_claimed)
    }
}

/// Private dispatcher that shares fair claim opportunity between six generic
/// families and the sealed accepted-submission family.
///
/// The next preferred family is process-local state. It flips before the
/// asynchronous claim attempt so a settled pass always advances preference,
/// including an idle or failed pass. A store error ends the pass immediately:
/// it is never interpreted as an idle side and cannot cause a fallback claim.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct FairWorkerDispatcher<G, A> {
    generic: G,
    accepted: A,
    next_preference: std::sync::Mutex<DispatchFamily>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl<G, A> FairWorkerDispatcher<G, A> {
    pub(crate) fn new(generic: G, accepted: A) -> Self {
        Self {
            generic,
            accepted,
            next_preference: std::sync::Mutex::new(DispatchFamily::Generic),
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
impl<G, A> FairWorkerDispatcher<G, A>
where
    G: GenericOneClaimDrain,
    A: AcceptedOneClaimDrain,
{
    /// Attempts the preferred family once, then the other family once only
    /// when the preferred attempt was idle.
    pub(crate) async fn drain_once(&self) -> Result<FairWorkerDispatchReport, StoreError> {
        let preferred = {
            let mut next_preference = self
                .next_preference
                .lock()
                .expect("fair dispatcher preference lock must not be poisoned");
            let preferred = *next_preference;
            *next_preference = preferred.other();
            preferred
        };
        let first = self.drain_family(preferred).await?;
        if first.claimed_count() == 1 {
            return Ok(first);
        }

        let second = self.drain_family(preferred.other()).await?;
        let report = FairWorkerDispatchReport {
            generic: first.generic.or(second.generic),
            accepted: first.accepted.or(second.accepted),
        };
        debug_assert!(report.claimed_count() <= 1);
        Ok(report)
    }

    async fn drain_family(
        &self,
        family: DispatchFamily,
    ) -> Result<FairWorkerDispatchReport, StoreError> {
        match family {
            DispatchFamily::Generic => {
                let report = self.generic.drain_one().await?;
                Ok(FairWorkerDispatchReport {
                    generic: Some(report),
                    accepted: None,
                })
            }
            DispatchFamily::Accepted => {
                let report = self.accepted.drain_one().await?;
                Ok(FairWorkerDispatchReport {
                    generic: None,
                    accepted: Some(report),
                })
            }
        }
    }
}

/// Stateless worker; competing instances rely on the broker's atomic claim.
#[derive(Clone)]
pub(crate) struct Worker<S> {
    store: Arc<S>,
    registry: JobRegistry,
    settings: WorkerSettings,
}

impl<S> Worker<S>
where
    S: JobStore + 'static,
{
    pub(crate) fn new(store: Arc<S>, registry: JobRegistry, settings: WorkerSettings) -> Self {
        Self {
            store,
            registry,
            settings,
        }
    }

    /// Claims and processes at most one bounded batch, with no heartbeat.
    pub(crate) async fn drain_once(&self) -> Result<DrainReport, StoreError> {
        let mut report = DrainReport::default();
        for _ in 0..self.settings.batch_size {
            let one = self.drain_one().await?;
            let claimed = one.claimed();
            report.include(one);
            if !claimed {
                break;
            }
        }
        Ok(report)
    }

    /// Claims and processes exactly one generic queue item at most.
    ///
    /// This operation deliberately ignores the configured batch size. The
    /// fair dispatcher uses it so one sealed accepted claim cannot be delayed
    /// behind a generic batch.
    pub(crate) async fn drain_one(&self) -> Result<DrainReport, StoreError> {
        let Some(claimed) = self
            .store
            .claim_next_job(self.registry.claim_filter(), self.settings.lease)
            .await?
        else {
            return Ok(DrainReport::default());
        };
        self.execute_claimed(claimed).await
    }

    /// Claims and processes one known generic queue identity at most.
    ///
    /// Synchronous server-owned convergence paths use the same preparation,
    /// cancellation, staging, and atomic commit behavior as the background
    /// worker while avoiding unrelated ready work.
    pub(crate) async fn drain_exact(
        &self,
        job: JobId,
        kind: JobKind,
    ) -> Result<DrainReport, StoreError> {
        let Some(claimed) = self
            .store
            .claim_exact_job(job, kind, self.settings.lease)
            .await?
        else {
            return Ok(DrainReport::default());
        };
        if claimed.id != job {
            return Err(StoreError::Unavailable(
                "exact queue broker returned another job identity".to_string(),
            ));
        }
        self.execute_claimed(claimed).await
    }

    async fn execute_claimed(&self, claimed: ClaimedJob) -> Result<DrainReport, StoreError> {
        let mut report = DrainReport::default();
        let family = self.registry.for_payload(&claimed.payload).ok_or_else(|| {
            StoreError::Unavailable(
                "queue broker returned a family outside the worker registry".to_string(),
            )
        })?;
        let claim = JobCommitClaim::new(claimed.id, claimed.lease_token);
        let execution = JobExecution::new().with_claim(claim);
        let handler = family.handler;
        let context = TenantContext::from_authenticated_session(claimed.tenant);
        let payload = claimed.payload.clone();
        let handler_execution = execution.clone();
        let mut task =
            tokio::spawn(async move { handler.prepare(context, payload, handler_execution).await });

        let prepared =
            match tokio::time::timeout(self.settings.preparation_timeout, &mut task).await {
                Ok(Ok(Ok(effect))) => Some(effect),
                Ok(Ok(Err(failure))) => {
                    self.finalize_failure(
                        &mut report,
                        context,
                        claimed.id,
                        claimed.lease_token,
                        failure,
                    )
                    .await;
                    None
                }
                Ok(Err(_panic)) => {
                    self.finalize_failure(
                        &mut report,
                        context,
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
                                context,
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
                                context,
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
            return Ok(report);
        };

        // This is the only visibility path. The committer owns one atomic
        // conditional transaction: active claim token + effect + completion.
        match family.committer.commit(claim, effect).await {
            Ok(EffectCommitOutcome::Committed) => report.completed += 1,
            Ok(EffectCommitOutcome::Rescheduled) => report.rescheduled += 1,
            Ok(EffectCommitOutcome::ClaimNoLongerActive) | Err(_) => {
                // An uncertain commit is not failed/retried here: recovery uses
                // the durable idempotency key after the lease resolves.
                report.finalization_failed += 1;
            }
        }
        Ok(report)
    }

    async fn finalize_failure(
        &self,
        report: &mut DrainReport,
        context: TenantContext,
        id: JobId,
        token: JobLeaseToken,
        failure: JobFailureKind,
    ) {
        match self.store.fail_job(context, id, token, failure).await {
            Ok(learning_data_access::JobFailureDisposition::Retrying) => report.retrying += 1,
            Ok(learning_data_access::JobFailureDisposition::Dead) => report.dead += 1,
            Err(_) => report.finalization_failed += 1,
        }
    }

    pub(crate) async fn ready_queue_depth(&self) -> Result<QueueDepth, StoreError> {
        self.store
            .ready_queue_depth(self.registry.claim_filter())
            .await
    }
}

#[async_trait]
impl<S> GenericOneClaimDrain for Worker<S>
where
    S: JobStore + 'static,
{
    async fn drain_one(&self) -> Result<DrainReport, StoreError> {
        Worker::drain_one(self).await
    }
}

#[async_trait]
impl<G, A> runtime::BoundedWorkerDispatch for FairWorkerDispatcher<G, A>
where
    G: GenericOneClaimDrain,
    A: AcceptedOneClaimDrain,
{
    async fn drain_once(&self) -> Result<u32, StoreError> {
        let report = FairWorkerDispatcher::drain_once(self).await?;
        Ok(report.claimed_count())
    }
}

#[cfg(test)]
#[path = "worker/tests.rs"]
mod tests;
