//! Sealed accepted-submission recovery used by the connected W7 receipt oracle.

use acceptance_runtime::AcceptanceRuntime;
use learning_data_access::postgres::{
    PostgresAcceptedSubmissionRecoveryStore, local_accepted_submission_recovery_pool,
};
use learning_data_access::{
    AcceptedSubmissionExecutionDisposition, AcceptedSubmissionExecutionOutcome,
    AcceptedSubmissionExecutionRecoveryClaimStore, AcceptedSubmissionExecutionStore,
    AcceptedSubmissionId, JobId, JobLeaseDuration, TenantContext, WorkerId,
};
use question_model::{GradingOperationReason, TenantId};

use super::fresh_uuid;

/// One recovery-worker capability with a private, attested database login.
///
/// The adapter controls claim selection and terminal state transitions, so the
/// connected oracle only checks the durable evidence that follows.
pub(super) struct RecoveryWorker {
    store: PostgresAcceptedSubmissionRecoveryStore,
    tenant: TenantId,
}

impl RecoveryWorker {
    pub(super) async fn connect(runtime: &AcceptanceRuntime, tenant: TenantId) -> Self {
        let pool = local_accepted_submission_recovery_pool(runtime.recovery_url().expose())
            .await
            .expect("attest disposable accepted-submission recovery pool");
        Self {
            store: PostgresAcceptedSubmissionRecoveryStore::from_recovery_pool(pool),
            tenant,
        }
    }

    pub(super) async fn fail_deterministically(
        &self,
        expected_job: JobId,
        expected_submission: AcceptedSubmissionId,
    ) -> WorkerId {
        self.claim_and_fail(
            expected_job,
            expected_submission,
            AcceptedSubmissionExecutionOutcome::DeterministicFailure {
                reason: GradingOperationReason::GraderExecutionFailure,
            },
        )
        .await
    }

    pub(super) async fn fail_terminally(
        &self,
        expected_job: JobId,
        expected_submission: AcceptedSubmissionId,
    ) -> WorkerId {
        self.claim_and_fail(
            expected_job,
            expected_submission,
            AcceptedSubmissionExecutionOutcome::TerminalFailure,
        )
        .await
    }

    async fn claim_and_fail(
        &self,
        expected_job: JobId,
        expected_submission: AcceptedSubmissionId,
        outcome: AcceptedSubmissionExecutionOutcome,
    ) -> WorkerId {
        let worker = WorkerId::from_uuid(fresh_uuid());
        let claim = self
            .store
            .claim_next_accepted_submission_execution(
                worker,
                JobLeaseDuration::from_seconds(60).expect("bounded recovery-worker lease"),
            )
            .await
            .expect("claim accepted submission through sealed recovery adapter")
            .expect("expected accepted submission remains recoverable");
        assert_eq!(
            claim.tenant, self.tenant,
            "recovery claim retains its tenant"
        );
        assert_eq!(
            claim.job, expected_job,
            "recovery claim retains its queued job"
        );
        assert_eq!(
            claim.submission, expected_submission,
            "recovery claim retains its accepted submission"
        );
        assert_eq!(
            self.store
                .commit_or_fail_accepted_submission_execution(
                    TenantContext::from_authenticated_session(self.tenant),
                    claim,
                    outcome,
                )
                .await
                .expect("commit terminal recovery through sealed worker adapter"),
            AcceptedSubmissionExecutionDisposition::Terminal,
            "recovery worker records the terminal transition"
        );
        worker
    }
}
