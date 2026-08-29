//! Current-score recalculation worker with staged, atomic publication.

use std::sync::Arc;

use async_trait::async_trait;
use learning_data_access::{
    AssignmentScoringCommitOutcome, AssignmentScoringPreparationOutcome,
    AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore, JobFailureKind, JobId, JobKind,
    JobPayload, JobState, JobStore, StoreError, TenantContext,
};

use crate::worker::{
    self, EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
    JobRegistry, JobRegistryEntry, PreparedJobEffect, Worker, WorkerSettings,
};

/// Result of asking the ordinary scoring worker to converge one exact job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactAssignmentScoringOutcome {
    /// The generation is durably published and its queue job is complete.
    Completed,
    /// Another worker owns the lease or the job is waiting to become ready.
    Pending,
}

/// Runs one known recalculation through the canonical staged scoring worker.
///
/// This is the synchronous host-composition entry point. It retains the same
/// lease, cancellation, preparation, and atomic publication boundaries as the
/// background worker while selecting only the caller's durable job identity.
pub async fn execute_exact_assignment_scoring<S>(
    store: Arc<S>,
    context: TenantContext,
    job: JobId,
    settings: WorkerSettings,
) -> Result<ExactAssignmentScoringOutcome, StoreError>
where
    S: AssignmentScoringWorkerStore + JobStore + Send + Sync + 'static,
{
    let handler: Arc<dyn JobHandler> = Arc::new(AssignmentScoringHandler::new(Arc::clone(&store)));
    let committer: Arc<dyn EffectCommitter> =
        Arc::new(AssignmentScoringCommitter::new(Arc::clone(&store)));
    let registry = JobRegistry::new([JobRegistryEntry::new(
        JobKind::RecalculateAssignment,
        handler,
        committer,
    )])?;
    let worker = Worker::new(Arc::clone(&store), registry, settings);
    let report = worker
        .drain_exact(job, JobKind::RecalculateAssignment)
        .await?;
    if report.completed == 1
        && report.rescheduled == 0
        && report.retrying == 0
        && report.dead == 0
        && report.finalization_failed == 0
    {
        return Ok(ExactAssignmentScoringOutcome::Completed);
    }
    if report.claimed() {
        return Err(StoreError::Unavailable(
            "exact assignment scoring did not publish successfully".to_string(),
        ));
    }
    match store.get_job(context, job).await? {
        Some(view) if view.state == JobState::Completed => {
            Ok(ExactAssignmentScoringOutcome::Completed)
        }
        Some(view) if matches!(view.state, JobState::Ready | JobState::Leased) => {
            Ok(ExactAssignmentScoringOutcome::Pending)
        }
        Some(_) => Err(StoreError::Unavailable(
            "exact assignment scoring reached a terminal queue state".to_string(),
        )),
        None => Err(StoreError::InvalidRecord(
            "exact assignment scoring job is unavailable".to_string(),
        )),
    }
}

/// Stages one still-current assignment scoring generation.
pub(crate) struct AssignmentScoringHandler<S> {
    store: Arc<S>,
}

impl<S> AssignmentScoringHandler<S> {
    pub(crate) fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S> JobHandler for AssignmentScoringHandler<S>
where
    S: AssignmentScoringWorkerStore + Send + Sync + 'static,
{
    async fn prepare(
        &self,
        context: TenantContext,
        payload: JobPayload,
        execution: JobExecution,
    ) -> Result<PreparedJobEffect, JobFailureKind> {
        let JobPayload::RecalculateAssignment {
            assignment,
            generation,
        } = payload
        else {
            return Err(JobFailureKind::Permanent);
        };
        let claim = execution.claim().ok_or(JobFailureKind::Permanent)?;
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        match self
            .store
            .prepare_assignment_scoring(
                context,
                AssignmentScoringWorkerCommand {
                    job: claim.job_id(),
                    lease: claim.lease_token(),
                    assignment,
                    generation,
                },
            )
            .await
            .map_err(scoring_failure)?
        {
            AssignmentScoringPreparationOutcome::Prepared
            | AssignmentScoringPreparationOutcome::Superseded => {}
        }
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        Ok(PreparedJobEffect::AssignmentScoring {
            tenant: context.tenant_id(),
            assignment,
            generation,
        })
    }
}

fn scoring_failure(error: StoreError) -> JobFailureKind {
    match error {
        StoreError::Unavailable(_) => JobFailureKind::Transient,
        StoreError::Conflict => JobFailureKind::Transient,
        _ => JobFailureKind::Permanent,
    }
}

/// Sole visibility boundary for a staged scoring generation.
pub(crate) struct AssignmentScoringCommitter<S> {
    store: Arc<S>,
}

impl<S> AssignmentScoringCommitter<S> {
    pub(crate) fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

impl<S> worker::sealed::EffectCommitter for AssignmentScoringCommitter<S> where
    S: Send + Sync + 'static
{
}

#[async_trait]
impl<S> EffectCommitter for AssignmentScoringCommitter<S>
where
    S: AssignmentScoringWorkerStore + Send + Sync + 'static,
{
    async fn commit(
        &self,
        claim: JobCommitClaim,
        effect: PreparedJobEffect,
    ) -> Result<EffectCommitOutcome, StoreError> {
        let PreparedJobEffect::AssignmentScoring {
            tenant,
            assignment,
            generation,
        } = effect
        else {
            return Err(StoreError::InvalidRecord(
                "assignment scoring committer received another effect family".to_string(),
            ));
        };
        match self
            .store
            .commit_assignment_scoring(
                TenantContext::from_authenticated_session(tenant),
                AssignmentScoringWorkerCommand {
                    job: claim.job_id(),
                    lease: claim.lease_token(),
                    assignment,
                    generation,
                },
            )
            .await?
        {
            AssignmentScoringCommitOutcome::Committed
            | AssignmentScoringCommitOutcome::Superseded => Ok(EffectCommitOutcome::Committed),
            AssignmentScoringCommitOutcome::ClaimNoLongerActive => {
                Ok(EffectCommitOutcome::ClaimNoLongerActive)
            }
        }
    }
}
