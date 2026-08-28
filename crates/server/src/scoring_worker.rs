//! Current-score recalculation worker with staged, atomic publication.

use std::sync::Arc;

use async_trait::async_trait;
use learning_data_access::{
    AssignmentScoringCommitOutcome, AssignmentScoringPreparationOutcome,
    AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore, JobFailureKind, JobPayload,
    StoreError, TenantContext,
};

use crate::worker::{
    self, EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
    PreparedJobEffect,
};

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
