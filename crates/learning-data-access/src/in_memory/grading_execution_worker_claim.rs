//! One locked eligibility and lease transition for accepted-submission work.

use crate::{
    AcceptedSubmissionExecutionClaim, AcceptedSubmissionExecutionTarget, GradingExecutionReceipt,
    JobLeaseDuration, JobLeaseToken, JobPayload, JobState, StoreError, WorkerId,
};

use super::*;

impl MemoryStore {
    /// Selects and leases one eligible execution under the sole Memory lock.
    ///
    /// Recovery supplies no target; the synchronous path supplies its durable
    /// target. Both callers therefore share eligibility, lease expiry, and
    /// transition behavior (ASVS V2.3.1 and V2.3.4).
    pub(super) fn claim_accepted_submission_execution(
        &self,
        target: Option<AcceptedSubmissionExecutionTarget>,
        worker: WorkerId,
        lease: JobLeaseDuration,
    ) -> Result<Option<AcceptedSubmissionExecutionClaim>, StoreError> {
        let token = JobLeaseToken::generate()?;
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        super::grading_execution_worker::converge_expired_exhausted_claims(&mut state, now)?;
        let candidate = state.jobs.iter().find_map(|(job_id, job)| {
            let JobPayload::GradeAcceptedSubmission {
                attempt,
                submission,
                execution_generation,
            } = job.payload
            else {
                return None;
            };
            let execution = state.automated_grading_executions.get(&attempt)?;
            let identity_matches = execution.submission == submission
                && execution.generation == execution_generation
                && execution.job == *job_id;
            let target_matches = target.is_none_or(|target| {
                target.attempt == attempt
                    && target.submission == submission
                    && target.job == *job_id
            });
            let eligible = match job.state {
                JobState::Ready => {
                    job.available_at <= now
                        && matches!(
                            execution.state,
                            crate::GradingExecutionState::Ready
                                | crate::GradingExecutionState::RetryWait
                        )
                }
                JobState::Leased => {
                    job.lease_expires_at.is_some_and(|expiry| expiry <= now)
                        && job.attempt_count < job.max_attempts
                        && execution.state == crate::GradingExecutionState::Running
                }
                JobState::Completed | JobState::Dead => false,
            };
            (identity_matches && target_matches && eligible).then_some((*job_id, attempt))
        });
        let Some((job_id, attempt)) = candidate else {
            return Ok(None);
        };
        let job = state.jobs.get_mut(&job_id).ok_or(StoreError::NotFound)?;
        let JobPayload::GradeAcceptedSubmission {
            submission,
            execution_generation,
            ..
        } = job.payload
        else {
            return Err(StoreError::Conflict);
        };
        job.state = JobState::Leased;
        job.attempt_count = job
            .attempt_count
            .checked_add(1)
            .ok_or_else(|| StoreError::InvalidRecord("job attempts overflow".to_string()))?;
        job.lease_token = Some(token);
        job.lease_expires_at = Some(super::queue::add_job_seconds(now, lease.seconds())?);
        job.failure = None;
        let execution = state
            .automated_grading_executions
            .get_mut(&attempt)
            .expect("eligible execution retains its record");
        execution.state = crate::GradingExecutionState::Running;
        state
            .automated_grading_execution_workers
            .insert(attempt, worker);
        state
            .automated_grading_execution_receipts
            .entry(attempt)
            .or_default()
            .push(GradingExecutionReceipt {
                submission,
                generation: execution_generation,
                resulting_state: crate::GradingExecutionState::Running,
                safe_category: crate::GradingExecutionReceiptSafeCategory::WorkerClaim,
                actor: None,
                worker: Some(worker),
                occurred_at: now,
            });
        Ok(Some(AcceptedSubmissionExecutionClaim {
            job: job_id,
            lease_token: token,
            submission,
            execution_generation,
            worker,
        }))
    }
}
