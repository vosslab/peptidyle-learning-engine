//! Private execution of one lease-bound accepted automated submission.
//!
//! This module deliberately bridges only the sealed execution worker store to
//! [`crate::run::RunBackend`]. It has no browser DTO, generic queue, or score
//! publication dependency: migration 1830 enqueues scoring work and migration
//! 1831 remains the only current-score writer.

use std::time::Duration;

use learning_data_access::{
    AcceptedSubmissionCommitError, AcceptedSubmissionExecution, AcceptedSubmissionExecutionClaim,
    AcceptedSubmissionExecutionDisposition, AcceptedSubmissionExecutionFastPathClaimStore,
    AcceptedSubmissionExecutionLoadError, AcceptedSubmissionExecutionOutcome,
    AcceptedSubmissionExecutionRecoveryClaimStore, AcceptedSubmissionExecutionStore,
    AcceptedSubmissionExecutionTarget, AcceptedSubmissionGrade, StoreError, TenantContext,
    WorkerId, canonical_attempt_result_json,
};
use question_model::{GradingOperationReason, ProblemVersionRef, StudentResponse};

use crate::run::{RunBackend, RunBackendError, RunSubmission, SubmissionDisposition};
use crate::worker::{AcceptedOneClaimDrain, WorkerSettings};

/// The server-local result of handling one exact execution claim.
///
/// It intentionally carries no response, prepared evidence, result, feedback,
/// lease token, or browser projection. `OutcomeUnknown` means the single
/// commit-or-fail request was ambiguous; callers must read later durable state
/// rather than submit the response again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AcceptedSubmissionHandlerResult {
    Committed,
    Rescheduled,
    Terminal,
    /// Durable acceptance remains queued for the sealed recovery worker.
    ///
    /// The acceptance-only fault profile deliberately selects this result
    /// without claiming, loading, or grading learner work in the API process.
    /// It is an explicit outcome rather than borrowing claim-loss semantics,
    /// so a route's pending projection accurately reflects the ownership
    /// boundary.
    #[cfg(feature = "e2e-grader-fault")]
    RecoveryQueued,
    ClaimNoLongerActive,
    OutcomeUnknown,
}

/// Server-internal capability for one exact accepted-submission execution.
///
/// Browser routes receive this opaque facade rather than an execution store or
/// a [`RunBackend`]. The facade owns the claim-before-load sequence, while the
/// common handler remains the sole owner of private input and outcome writes.
/// ASVS 2.3.1 and 8.3.1: the target originates from durable acceptance and the
/// route cannot select, load, or grade work on its own.
#[async_trait::async_trait]
pub(crate) trait AcceptedSubmissionFastPath: Send + Sync {
    async fn execute_accepted_submission(
        &self,
        target: AcceptedSubmissionExecutionTarget,
    ) -> Result<AcceptedSubmissionHandlerResult, StoreError>;
}

/// Explicit composition placeholder until the typed fast-path PostgreSQL pool
/// is supplied. It preserves durable acceptance and leaves recovery available.
pub(crate) struct UnavailableAcceptedSubmissionFastPath;

#[async_trait::async_trait]
impl AcceptedSubmissionFastPath for UnavailableAcceptedSubmissionFastPath {
    async fn execute_accepted_submission(
        &self,
        _: AcceptedSubmissionExecutionTarget,
    ) -> Result<AcceptedSubmissionHandlerResult, StoreError> {
        Err(StoreError::Unavailable(
            "accepted-submission fast path is not configured".to_string(),
        ))
    }
}

/// Acceptance-only facade for the isolated deterministic-fault stack.
///
/// This type exists only in a purpose-built acceptance binary.  It preserves
/// the ordinary durable accept transition while leaving the execution claim to
/// the feature-only one-claim worker.  The API cannot observe private work or
/// cause grading through this facade.
#[cfg(feature = "e2e-grader-fault")]
pub(crate) struct RecoveryOnlyAcceptedSubmissionFastPath;

#[cfg(feature = "e2e-grader-fault")]
#[async_trait::async_trait]
impl AcceptedSubmissionFastPath for RecoveryOnlyAcceptedSubmissionFastPath {
    async fn execute_accepted_submission(
        &self,
        _: AcceptedSubmissionExecutionTarget,
    ) -> Result<AcceptedSubmissionHandlerResult, StoreError> {
        Ok(AcceptedSubmissionHandlerResult::RecoveryQueued)
    }
}

/// A feature-only grader that produces the one closed exception class used by
/// the connected recovery journey.
///
/// It is composed solely by the dedicated one-claim process mode.  Its submit
/// implementation does not inspect or serialize any accepted input, so the
/// connected evidence exercises the production handler and durable failure
/// path without a browser-side answer channel.
#[cfg(feature = "e2e-grader-fault")]
pub(crate) struct DeterministicGraderExceptionBackend;

#[cfg(feature = "e2e-grader-fault")]
#[async_trait::async_trait]
impl RunBackend for DeterministicGraderExceptionBackend {
    async fn issue(
        &self,
        _: TenantContext,
        _: ProblemVersionRef,
        _: &question_model::QuestionDefinition,
        _: u64,
    ) -> Result<crate::run::IssuedAttemptMetadata, RunBackendError> {
        Err(RunBackendError::Unsupported(
            "deterministic grader exception backend only executes accepted submissions".to_string(),
        ))
    }

    async fn reproduce(
        &self,
        _: TenantContext,
        _: ProblemVersionRef,
        _: &question_model::QuestionDefinition,
        _: &question_model::QuestionAttempt,
    ) -> Result<question_model::QuestionEnvelope, RunBackendError> {
        Err(RunBackendError::Unsupported(
            "deterministic grader exception backend only executes accepted submissions".to_string(),
        ))
    }

    async fn grade(
        &self,
        _: TenantContext,
        _: ProblemVersionRef,
        _: &question_model::QuestionDefinition,
        _: &question_model::QuestionAttempt,
        _: &StudentResponse,
    ) -> Result<grading::GradeOutcome, RunBackendError> {
        Err(RunBackendError::Unsupported(
            "deterministic grader exception backend only executes accepted submissions".to_string(),
        ))
    }

    async fn submit(&self, _: RunSubmission<'_>) -> Result<SubmissionDisposition, RunBackendError> {
        Err(RunBackendError::Deterministic(
            crate::run::DeterministicGraderFailure::Execution,
        ))
    }
}

/// Rejects a handler deadline that cannot bound an execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InvalidAcceptedSubmissionExecutionDeadline;

impl std::fmt::Display for InvalidAcceptedSubmissionExecutionDeadline {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("accepted-submission execution deadline must be positive")
    }
}

impl std::error::Error for InvalidAcceptedSubmissionExecutionDeadline {}

/// Common server-private handler shared by the fast and recovery worker paths.
pub(crate) struct AcceptedSubmissionExecutionHandler<S, B> {
    store: S,
    backend: B,
    execution_deadline: Duration,
}

impl<S, B> AcceptedSubmissionExecutionHandler<S, B> {
    /// Builds the single owner of a bounded accepted-submission execution.
    pub(crate) fn new(
        store: S,
        backend: B,
        execution_deadline: Duration,
    ) -> Result<Self, InvalidAcceptedSubmissionExecutionDeadline> {
        if execution_deadline.is_zero() {
            return Err(InvalidAcceptedSubmissionExecutionDeadline);
        }
        Ok(Self {
            store,
            backend,
            execution_deadline,
        })
    }
}

impl<S, B> AcceptedSubmissionExecutionHandler<S, B>
where
    S: AcceptedSubmissionExecutionStore,
    B: RunBackend,
{
    /// Executes exactly one previously-won claim.
    ///
    /// A stale load is not an error for a worker: another fenced transition is
    /// authoritative. Once `submit` has been reached, this function performs
    /// exactly one durable outcome request and never retries grading or commit.
    pub(crate) async fn execute_claim(
        &self,
        claim: AcceptedSubmissionExecutionClaim,
    ) -> Result<AcceptedSubmissionHandlerResult, StoreError> {
        let context = TenantContext::from_authenticated_session(claim.tenant);
        let execution = match self
            .store
            .load_accepted_submission_for_execution(context, claim)
            .await
        {
            Ok(execution) => execution,
            Err(
                AcceptedSubmissionExecutionLoadError::NotFound
                | AcceptedSubmissionExecutionLoadError::Conflict,
            ) => {
                return Ok(AcceptedSubmissionHandlerResult::ClaimNoLongerActive);
            }
            // The load capability has already rejected the private material
            // and rolled back its read transaction. The claim is still live,
            // so finalize the safe, closed integrity outcome through the same
            // lease-fenced fail transition as a deterministic grader fault.
            // ASVS 1.5.2 and 2.3.1: malformed issued evidence cannot be
            // interpreted or leave a running execution stranded.
            Err(AcceptedSubmissionExecutionLoadError::IssuedEvidenceIntegrity) => {
                let outcome = AcceptedSubmissionExecutionOutcome::DeterministicFailure {
                    reason: GradingOperationReason::IssuedEvidenceIntegrity,
                };
                return match self
                    .store
                    .commit_or_fail_accepted_submission_execution(context, claim, outcome)
                    .await
                {
                    Ok(disposition) => Ok(handler_result(disposition)),
                    Err(AcceptedSubmissionCommitError::Known(error)) => Err(error),
                    Err(AcceptedSubmissionCommitError::OutcomeUnknown) => {
                        Ok(AcceptedSubmissionHandlerResult::OutcomeUnknown)
                    }
                };
            }
            Err(AcceptedSubmissionExecutionLoadError::Store(error)) => return Err(error),
        };
        let outcome = self.evaluate(context, execution).await;
        match self
            .store
            .commit_or_fail_accepted_submission_execution(context, claim, outcome)
            .await
        {
            Ok(disposition) => Ok(handler_result(disposition)),
            Err(AcceptedSubmissionCommitError::Known(error)) => Err(error),
            // The grader has already run. Retrying could duplicate an external
            // side effect, so later durable state is authoritative.
            Err(AcceptedSubmissionCommitError::OutcomeUnknown) => {
                Ok(AcceptedSubmissionHandlerResult::OutcomeUnknown)
            }
        }
    }

    async fn evaluate(
        &self,
        context: TenantContext,
        execution: AcceptedSubmissionExecution,
    ) -> AcceptedSubmissionExecutionOutcome {
        let AcceptedSubmissionExecution {
            accepted,
            response,
            prepared,
        } = execution;
        let response = match translate_accepted_response(&prepared, &response) {
            Ok(response) => response,
            Err(reason) => {
                return AcceptedSubmissionExecutionOutcome::DeterministicFailure { reason };
            }
        };
        let reference = ProblemVersionRef {
            problem: prepared.attempt.problem,
            version: prepared.attempt.question_version,
        };
        let submission = RunSubmission {
            context,
            actor: accepted.actor,
            idempotency_key: accepted.idempotency_key,
            reference,
            issued_question_snapshot: &prepared.issued_question_snapshot,
            attempt: &prepared.attempt,
            issued_grading_envelope: prepared.grading_envelope.as_ref(),
            issued_flat_grading: prepared.flat_grading.as_ref(),
            issued_webwork_grading: prepared.webwork_grading.as_ref(),
            issued_qti_grading: prepared.issued_qti_grading.as_ref(),
            issued_webwork_replay: prepared.webwork_replay.as_ref(),
            issued_presentation_binding: prepared.presentation_binding,
            issued_presentation: prepared.presentation.as_ref(),
            response: &response,
        };
        match tokio::time::timeout(self.execution_deadline, self.backend.submit(submission)).await {
            Err(_) => AcceptedSubmissionExecutionOutcome::TimedOut,
            Ok(backend_result) => map_backend_result(backend_result),
        }
    }
}

/// Answer-free outcome counters for one sealed recovery pass.
///
/// A pass claims at most one execution, so exactly one counter is incremented
/// unless the claim or handler returns a known store error. The report does
/// not contain a response, result, feedback, lease token, or claim identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct AcceptedSubmissionExecutionWorkerReport {
    pub(crate) no_claim: u32,
    pub(crate) committed: u32,
    pub(crate) rescheduled: u32,
    pub(crate) terminal: u32,
    pub(crate) stale_claim: u32,
    pub(crate) outcome_unknown: u32,
}

/// Background recovery owner for the sealed accepted-submission family.
///
/// `worker_id` is supplied once at process composition and retained for every
/// pass. The generic `JobStore` and its finalization methods are deliberately
/// absent: this type can only use the private execution-store capability.
pub(crate) struct AcceptedSubmissionExecutionWorker<S, B> {
    handler: AcceptedSubmissionExecutionHandler<S, B>,
    worker_id: WorkerId,
    lease: learning_data_access::JobLeaseDuration,
}

impl<S, B> AcceptedSubmissionExecutionWorker<S, B> {
    /// Builds a worker from the existing validated process bounds.
    pub(crate) fn new(
        store: S,
        backend: B,
        worker_id: WorkerId,
        settings: WorkerSettings,
    ) -> Result<Self, InvalidAcceptedSubmissionExecutionDeadline> {
        Ok(Self {
            handler: AcceptedSubmissionExecutionHandler::new(
                store,
                backend,
                settings.execution_deadline(),
            )?,
            worker_id,
            lease: settings.lease(),
        })
    }

    /// Returns the stable process identity used for every claim.
    pub(crate) fn worker_id(&self) -> WorkerId {
        self.worker_id
    }
}

impl<S, B> AcceptedSubmissionExecutionWorker<S, B>
where
    S: AcceptedSubmissionExecutionStore + AcceptedSubmissionExecutionRecoveryClaimStore,
    B: RunBackend,
{
    /// Claims and handles at most one sealed execution.
    ///
    /// ASVS 2.3.1 and 8.4.1: the store receives the process-stable worker ID
    /// and validates the full tenant/job/lease/submission/generation tuple.
    /// This direct bounded future remains the handler's sole grading call; no
    /// task is detached and no generic queue finalization is available here.
    pub(crate) async fn drain_one(
        &self,
    ) -> Result<AcceptedSubmissionExecutionWorkerReport, StoreError> {
        let Some(claim) = self
            .handler
            .store
            .claim_next_accepted_submission_execution(self.worker_id, self.lease)
            .await?
        else {
            return Ok(AcceptedSubmissionExecutionWorkerReport {
                no_claim: 1,
                ..AcceptedSubmissionExecutionWorkerReport::default()
            });
        };

        let mut report = AcceptedSubmissionExecutionWorkerReport::default();
        match self.handler.execute_claim(claim).await? {
            AcceptedSubmissionHandlerResult::Committed => report.committed = 1,
            AcceptedSubmissionHandlerResult::Rescheduled => report.rescheduled = 1,
            AcceptedSubmissionHandlerResult::Terminal => report.terminal = 1,
            #[cfg(feature = "e2e-grader-fault")]
            AcceptedSubmissionHandlerResult::RecoveryQueued => {
                unreachable!("the recovery worker always claims before invoking the handler")
            }
            AcceptedSubmissionHandlerResult::ClaimNoLongerActive => report.stale_claim = 1,
            AcceptedSubmissionHandlerResult::OutcomeUnknown => report.outcome_unknown = 1,
        }
        Ok(report)
    }
}

impl<S, B> AcceptedSubmissionExecutionWorker<S, B>
where
    S: AcceptedSubmissionExecutionStore + AcceptedSubmissionExecutionFastPathClaimStore,
    B: RunBackend,
{
    /// Claims and handles one exact accepted submission through the same
    /// private handler used by background recovery.
    pub(crate) async fn execute_accepted_submission(
        &self,
        target: AcceptedSubmissionExecutionTarget,
    ) -> Result<AcceptedSubmissionHandlerResult, StoreError> {
        let Some(claim) = self
            .handler
            .store
            .claim_exact_accepted_submission_execution(target, self.worker_id, self.lease)
            .await?
        else {
            return Ok(AcceptedSubmissionHandlerResult::ClaimNoLongerActive);
        };

        self.handler.execute_claim(claim).await
    }
}

#[async_trait::async_trait]
impl<S, B> AcceptedSubmissionFastPath for AcceptedSubmissionExecutionWorker<S, B>
where
    S: AcceptedSubmissionExecutionStore
        + AcceptedSubmissionExecutionFastPathClaimStore
        + Send
        + Sync
        + 'static,
    B: RunBackend + Send + Sync + 'static,
{
    async fn execute_accepted_submission(
        &self,
        target: AcceptedSubmissionExecutionTarget,
    ) -> Result<AcceptedSubmissionHandlerResult, StoreError> {
        AcceptedSubmissionExecutionWorker::execute_accepted_submission(self, target).await
    }
}

#[async_trait::async_trait]
impl<S, B> AcceptedOneClaimDrain for AcceptedSubmissionExecutionWorker<S, B>
where
    S: AcceptedSubmissionExecutionStore
        + AcceptedSubmissionExecutionRecoveryClaimStore
        + Send
        + Sync
        + 'static,
    B: RunBackend + Send + Sync + 'static,
{
    async fn drain_one(&self) -> Result<AcceptedSubmissionExecutionWorkerReport, StoreError> {
        AcceptedSubmissionExecutionWorker::drain_one(self).await
    }
}

fn map_backend_result(
    backend_result: Result<SubmissionDisposition, RunBackendError>,
) -> AcceptedSubmissionExecutionOutcome {
    match backend_result {
        Ok(SubmissionDisposition::Grade(receipt)) => {
            match canonical_attempt_result_json(receipt.result) {
                Ok(evidence) => AcceptedSubmissionExecutionOutcome::Evaluated {
                    grade: AcceptedSubmissionGrade {
                        evidence,
                        feedback: receipt.feedback,
                    },
                },
                Err(_) => AcceptedSubmissionExecutionOutcome::DeterministicFailure {
                    reason: GradingOperationReason::GraderExecutionFailure,
                },
            }
        }
        // Manual and backend-owned external-tool paths cannot enter this
        // sealed deterministic worker. Record a safe terminal recovery.
        Ok(SubmissionDisposition::NeedsManualGrading | SubmissionDisposition::Committed(_))
        | Err(RunBackendError::Unsupported(_)) => {
            AcceptedSubmissionExecutionOutcome::TerminalFailure
        }
        Err(RunBackendError::Deterministic(failure)) => {
            AcceptedSubmissionExecutionOutcome::DeterministicFailure {
                reason: failure.operation_reason(),
            }
        }
        Err(RunBackendError::Invalid(_)) => {
            AcceptedSubmissionExecutionOutcome::DeterministicFailure {
                reason: GradingOperationReason::IssuedEvidenceIntegrity,
            }
        }
        Err(RunBackendError::Unavailable(_)) => {
            AcceptedSubmissionExecutionOutcome::TransientFailure
        }
    }
}

fn handler_result(
    disposition: AcceptedSubmissionExecutionDisposition,
) -> AcceptedSubmissionHandlerResult {
    match disposition {
        AcceptedSubmissionExecutionDisposition::Committed => {
            AcceptedSubmissionHandlerResult::Committed
        }
        AcceptedSubmissionExecutionDisposition::Rescheduled => {
            AcceptedSubmissionHandlerResult::Rescheduled
        }
        AcceptedSubmissionExecutionDisposition::Terminal => {
            AcceptedSubmissionHandlerResult::Terminal
        }
        AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive => {
            AcceptedSubmissionHandlerResult::ClaimNoLongerActive
        }
    }
}

fn translate_accepted_response(
    prepared: &learning_data_access::PreparedQuestionSubmission,
    response: &StudentResponse,
) -> Result<StudentResponse, GradingOperationReason> {
    match (
        prepared.presentation.as_ref(),
        prepared.grading_envelope.as_ref(),
        prepared.presentation_binding,
    ) {
        (Some(presentation), Some(envelope), Some(binding)) => {
            let public_report = domain::validation::validate_presentation_response_format(
                &presentation.envelope.response,
                response,
            );
            if !public_report.is_valid() {
                return Err(GradingOperationReason::IssuedEvidenceIntegrity);
            }
            let issued = question_model::presentation::reproduce_presentation_v1(
                envelope,
                &presentation.asset_bindings,
                binding,
            )
            .map_err(|_| GradingOperationReason::IssuedEvidenceIntegrity)?;
            if issued.envelope != presentation.envelope {
                return Err(GradingOperationReason::IssuedEvidenceIntegrity);
            }
            let translated =
                question_model::presentation::translate_rendered_response_v1(response, &issued)
                    .map_err(|_| GradingOperationReason::IssuedEvidenceIntegrity)?;
            let private_report =
                domain::validation::validate_response_format(&envelope.response, &translated);
            if private_report.is_valid() {
                Ok(translated)
            } else {
                Err(GradingOperationReason::IssuedEvidenceIntegrity)
            }
        }
        (None, None, None) => {
            let report = domain::validation::validate_response_format(
                &prepared.question().response,
                response,
            );
            if report.is_valid() {
                Ok(response.clone())
            } else {
                Err(GradingOperationReason::IssuedEvidenceIntegrity)
            }
        }
        _ => Err(GradingOperationReason::IssuedEvidenceIntegrity),
    }
}

#[cfg(test)]
#[path = "accepted_submission_worker_tests.rs"]
mod tests;
