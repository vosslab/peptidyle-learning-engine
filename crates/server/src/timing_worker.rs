//! Generation-fenced server auto-submit for current attempt deadlines.

use std::sync::Arc;

use async_trait::async_trait;
use learning_data_access::{
    AttemptAutoSubmitCommitOutcome, AttemptAutoSubmitWorkerCommand, AttemptAutoSubmitWorkerStore,
    JobFailureKind, JobPayload, StoreError, TenantContext,
};

use crate::worker::{
    self, EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
    PreparedJobEffect,
};

/// Deadline work has no external preparation; the Store re-resolves all
/// mutable state in the final lease-conditional transaction.
#[derive(Debug, Default)]
pub(crate) struct AttemptAutoSubmitHandler;

impl AttemptAutoSubmitHandler {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait]
impl JobHandler for AttemptAutoSubmitHandler {
    async fn prepare(
        &self,
        context: TenantContext,
        payload: JobPayload,
        execution: JobExecution,
    ) -> Result<PreparedJobEffect, JobFailureKind> {
        let JobPayload::AutoSubmitAttempt {
            attempt,
            timing_generation,
        } = payload
        else {
            return Err(JobFailureKind::Permanent);
        };
        if execution.claim().is_none() {
            return Err(JobFailureKind::Permanent);
        }
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        Ok(PreparedJobEffect::AttemptAutoSubmit {
            tenant: context.tenant_id(),
            attempt,
            timing_generation,
        })
    }
}

/// Sole state transition for one current attempt deadline.
pub(crate) struct AttemptAutoSubmitCommitter<S> {
    store: Arc<S>,
}

impl<S> AttemptAutoSubmitCommitter<S> {
    pub(crate) fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

impl<S> worker::sealed::EffectCommitter for AttemptAutoSubmitCommitter<S> where
    S: Send + Sync + 'static
{
}

#[async_trait]
impl<S> EffectCommitter for AttemptAutoSubmitCommitter<S>
where
    S: AttemptAutoSubmitWorkerStore + Send + Sync + 'static,
{
    async fn commit(
        &self,
        claim: JobCommitClaim,
        effect: PreparedJobEffect,
    ) -> Result<EffectCommitOutcome, StoreError> {
        let PreparedJobEffect::AttemptAutoSubmit {
            tenant,
            attempt,
            timing_generation,
        } = effect
        else {
            return Err(StoreError::InvalidRecord(
                "attempt auto-submit committer received another effect family".to_string(),
            ));
        };
        match self
            .store
            .commit_attempt_auto_submit(
                TenantContext::from_authenticated_session(tenant),
                AttemptAutoSubmitWorkerCommand {
                    job: claim.job_id(),
                    lease: claim.lease_token(),
                    attempt,
                    timing_generation,
                },
            )
            .await?
        {
            AttemptAutoSubmitCommitOutcome::AutoSubmitted
            | AttemptAutoSubmitCommitOutcome::Superseded => Ok(EffectCommitOutcome::Committed),
            AttemptAutoSubmitCommitOutcome::Rescheduled => Ok(EffectCommitOutcome::Rescheduled),
            AttemptAutoSubmitCommitOutcome::ClaimNoLongerActive => {
                Ok(EffectCommitOutcome::ClaimNoLongerActive)
            }
        }
    }
}
