use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use learning_data_access::{
    AcceptedSubmission, AcceptedSubmissionCommitError, AcceptedSubmissionExecution,
    AcceptedSubmissionExecutionClaim, AcceptedSubmissionExecutionDisposition,
    AcceptedSubmissionExecutionFastPathClaimStore, AcceptedSubmissionExecutionLoadError,
    AcceptedSubmissionExecutionOutcome, AcceptedSubmissionExecutionRecoveryClaimStore,
    AcceptedSubmissionExecutionStore, AcceptedSubmissionExecutionTarget,
    GradingExecutionGeneration, IssuedQuestionFamilyWitnessV1, IssuedQuestionSnapshotV1, JobId,
    JobLeaseDuration, JobLeaseToken, PreparedQuestionSubmission, SubmissionIdempotencyKey,
    WorkerId,
};
use question_model::answer::TextMatchMode;
use question_model::definition::{GradingDefinition, QuestionMetadata};
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AttemptProvenance, AttemptStatus, AttemptTimerRecord, ImplementationVersion,
    IssuedAttemptCapabilityV1, QuestionAttempt, QuestionAttemptId, QuestionDefinition,
    QuestionSource, RunId, StudentResponse, TenantId, UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

use super::*;

const EXECUTION_DEADLINE: Duration = Duration::from_secs(5);

#[derive(Clone)]
struct FakeStore {
    claim_result: Result<Option<AcceptedSubmissionExecutionClaim>, StoreError>,
    claim_requests: Arc<Mutex<Vec<(WorkerId, JobLeaseDuration)>>>,
    load_claims: Arc<Mutex<Vec<AcceptedSubmissionExecutionClaim>>>,
    execution: Arc<Mutex<Option<AcceptedSubmissionExecution>>>,
    load_error: Option<AcceptedSubmissionExecutionLoadError>,
    commit_result: Result<AcceptedSubmissionExecutionDisposition, AcceptedSubmissionCommitError>,
    commits: Arc<Mutex<Vec<AcceptedSubmissionExecutionOutcome>>>,
    cancellation_before_commit: Option<Arc<AtomicBool>>,
}

#[async_trait]
impl AcceptedSubmissionExecutionRecoveryClaimStore for FakeStore {
    async fn claim_next_accepted_submission_execution(
        &self,
        worker: WorkerId,
        lease: JobLeaseDuration,
    ) -> Result<Option<AcceptedSubmissionExecutionClaim>, StoreError> {
        self.claim_requests
            .lock()
            .expect("fake claim requests lock")
            .push((worker, lease));
        self.claim_result.clone()
    }
}

#[async_trait]
impl AcceptedSubmissionExecutionFastPathClaimStore for FakeStore {
    async fn claim_exact_accepted_submission_execution(
        &self,
        _: AcceptedSubmissionExecutionTarget,
        worker: WorkerId,
        lease: JobLeaseDuration,
    ) -> Result<Option<AcceptedSubmissionExecutionClaim>, StoreError> {
        self.claim_requests
            .lock()
            .expect("fake claim requests lock")
            .push((worker, lease));
        self.claim_result.clone()
    }
}

#[async_trait]
impl AcceptedSubmissionExecutionStore for FakeStore {
    async fn load_accepted_submission_for_execution(
        &self,
        _: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
    ) -> Result<AcceptedSubmissionExecution, AcceptedSubmissionExecutionLoadError> {
        self.load_claims
            .lock()
            .expect("fake load claims lock")
            .push(claim);
        if let Some(error) = &self.load_error {
            return Err(error.clone());
        }
        self.execution
            .lock()
            .expect("fake execution lock")
            .take()
            .ok_or(AcceptedSubmissionExecutionLoadError::NotFound)
    }

    async fn commit_or_fail_accepted_submission_execution(
        &self,
        _: TenantContext,
        _: AcceptedSubmissionExecutionClaim,
        outcome: AcceptedSubmissionExecutionOutcome,
    ) -> Result<AcceptedSubmissionExecutionDisposition, AcceptedSubmissionCommitError> {
        if let Some(cancelled) = &self.cancellation_before_commit {
            assert!(
                cancelled.load(Ordering::SeqCst),
                "timed-out backend future must be cancelled before commit"
            );
        }
        self.commits
            .lock()
            .expect("fake commits lock")
            .push(outcome);
        self.commit_result.clone()
    }
}

struct FakeBackend {
    result: Result<SubmissionDisposition, RunBackendError>,
    submit_calls: Arc<AtomicUsize>,
    verify_sealed_reconstruction: bool,
}

#[async_trait]
impl RunBackend for FakeBackend {
    async fn issue(
        &self,
        _: TenantContext,
        _: ProblemVersionRef,
        _: &QuestionDefinition,
        _: u64,
    ) -> Result<crate::run::IssuedAttemptMetadata, RunBackendError> {
        Err(RunBackendError::Unsupported("fixture".to_string()))
    }

    async fn reproduce(
        &self,
        _: TenantContext,
        _: ProblemVersionRef,
        _: &QuestionDefinition,
        _: &QuestionAttempt,
    ) -> Result<question_model::QuestionEnvelope, RunBackendError> {
        Err(RunBackendError::Unsupported("fixture".to_string()))
    }

    async fn grade(
        &self,
        _: TenantContext,
        _: ProblemVersionRef,
        _: &QuestionDefinition,
        _: &QuestionAttempt,
        _: &StudentResponse,
    ) -> Result<grading::GradeOutcome, RunBackendError> {
        Err(RunBackendError::Unsupported("fixture".to_string()))
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        self.submit_calls.fetch_add(1, Ordering::SeqCst);
        if self.verify_sealed_reconstruction {
            assert_eq!(submission.context.tenant_id(), tenant_id());
            assert_eq!(submission.actor, actor_id());
            assert_eq!(submission.idempotency_key.as_str(), "sealed-worker-key");
            assert_eq!(submission.reference.problem, problem_id());
            assert_eq!(submission.reference.version, version_id());
            assert_eq!(submission.attempt.id, attempt_id());
            assert_eq!(
                submission.issued_question_snapshot.question().problem,
                problem_id()
            );
            assert_eq!(
                submission.issued_question_snapshot.question().version,
                version_id()
            );
            assert!(submission.issued_grading_envelope.is_none());
            assert!(submission.issued_flat_grading.is_none());
            assert!(submission.issued_webwork_grading.is_none());
            assert!(submission.issued_qti_grading.is_none());
            assert!(submission.issued_webwork_replay.is_none());
            assert!(submission.issued_presentation_binding.is_none());
            assert!(submission.issued_presentation.is_none());
        }
        self.result.clone()
    }
}

struct PendingBackend {
    cancelled: Arc<AtomicBool>,
    submit_calls: Arc<AtomicUsize>,
}

struct CancellationSignal(Arc<AtomicBool>);

impl Drop for CancellationSignal {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[async_trait]
impl RunBackend for PendingBackend {
    async fn issue(
        &self,
        _: TenantContext,
        _: ProblemVersionRef,
        _: &QuestionDefinition,
        _: u64,
    ) -> Result<crate::run::IssuedAttemptMetadata, RunBackendError> {
        unreachable!("timeout fixture only submits")
    }

    async fn reproduce(
        &self,
        _: TenantContext,
        _: ProblemVersionRef,
        _: &QuestionDefinition,
        _: &QuestionAttempt,
    ) -> Result<question_model::QuestionEnvelope, RunBackendError> {
        unreachable!("timeout fixture only submits")
    }

    async fn grade(
        &self,
        _: TenantContext,
        _: ProblemVersionRef,
        _: &QuestionDefinition,
        _: &QuestionAttempt,
        _: &StudentResponse,
    ) -> Result<grading::GradeOutcome, RunBackendError> {
        unreachable!("timeout fixture only submits")
    }

    async fn submit(&self, _: RunSubmission<'_>) -> Result<SubmissionDisposition, RunBackendError> {
        self.submit_calls.fetch_add(1, Ordering::SeqCst);
        let _signal = CancellationSignal(Arc::clone(&self.cancelled));
        std::future::pending().await
    }
}

type HandlerFixture = (
    AcceptedSubmissionExecutionHandler<FakeStore, FakeBackend>,
    Arc<AtomicUsize>,
    Arc<Mutex<Vec<AcceptedSubmissionExecutionOutcome>>>,
);

fn handler(
    backend_result: Result<SubmissionDisposition, RunBackendError>,
    commit_result: Result<AcceptedSubmissionExecutionDisposition, AcceptedSubmissionCommitError>,
) -> HandlerFixture {
    let calls = Arc::new(AtomicUsize::new(0));
    let commits = Arc::new(Mutex::new(Vec::new()));
    let store = FakeStore {
        claim_result: Ok(None),
        claim_requests: Arc::new(Mutex::new(Vec::new())),
        load_claims: Arc::new(Mutex::new(Vec::new())),
        execution: Arc::new(Mutex::new(Some(execution()))),
        load_error: None,
        commit_result,
        commits: Arc::clone(&commits),
        cancellation_before_commit: None,
    };
    let backend = FakeBackend {
        result: backend_result,
        submit_calls: Arc::clone(&calls),
        verify_sealed_reconstruction: true,
    };
    (
        AcceptedSubmissionExecutionHandler::new(store, backend, EXECUTION_DEADLINE)
            .expect("positive test deadline"),
        calls,
        commits,
    )
}

fn tenant_id() -> TenantId {
    TenantId::from_uuid(Uuid::from_u128(1))
}

fn actor_id() -> UserId {
    UserId::from_uuid(Uuid::from_u128(2))
}

fn problem_id() -> question_model::ProblemId {
    question_model::ProblemId::from_uuid(Uuid::from_u128(7))
}

fn version_id() -> VersionId {
    VersionId::from_uuid(Uuid::from_u128(8))
}

fn attempt_id() -> QuestionAttemptId {
    QuestionAttemptId::from_uuid(Uuid::from_u128(10))
}

fn claim() -> AcceptedSubmissionExecutionClaim {
    AcceptedSubmissionExecutionClaim {
        tenant: tenant_id(),
        job: JobId::from_uuid(Uuid::from_u128(3)),
        lease_token: JobLeaseToken::generate().expect("lease token"),
        submission: learning_data_access::AcceptedSubmissionId::from_uuid(Uuid::from_u128(5)),
        execution_generation: GradingExecutionGeneration::INITIAL,
        worker: WorkerId::from_uuid(Uuid::from_u128(6)),
    }
}

fn target() -> AcceptedSubmissionExecutionTarget {
    let claim = claim();
    AcceptedSubmissionExecutionTarget {
        tenant: claim.tenant,
        attempt: attempt_id(),
        submission: claim.submission,
        job: claim.job,
    }
}

fn execution() -> AcceptedSubmissionExecution {
    let question = QuestionDefinition {
        problem: problem_id(),
        version: version_id(),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(9)),
        source: QuestionSource::Native {
            family: "fixture".to_string(),
        },
        prompt: Vec::new(),
        response: question_model::ResponseDefinition::ShortText {
            match_mode: TextMatchMode::Normalized,
            max_length: 24,
        },
        attempt_policy: AttemptPolicy {
            max_attempts: Some(1),
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "fixture".to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    };
    let attempt = QuestionAttempt {
        id: attempt_id(),
        tenant: tenant_id(),
        run: RunId::from_uuid(Uuid::from_u128(11)),
        problem: question.problem,
        question_version: question.version,
        assignment_position: 0,
        seed: 12,
        parameter_hash: "a".repeat(64),
        response: None,
        status: AttemptStatus::InProgress,
        result: None,
        timer: AttemptTimerRecord {
            issued_at: ActivityTimestamp::from_unix_millis(13),
            deadline: None,
            submitted_at: None,
        },
        provenance: AttemptProvenance {
            adapter: ImplementationVersion {
                id: "fixture".to_string(),
                version: "1".to_string(),
            },
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: ImplementationVersion {
                id: "fixture".to_string(),
                version: "1".to_string(),
            },
            rendered_question_sha256: "b".repeat(64),
        },
        issued_capability: IssuedAttemptCapabilityV1::NotApplicable,
    };
    AcceptedSubmissionExecution {
        accepted: AcceptedSubmission {
            tenant: tenant_id(),
            course: question_model::CourseId::from_uuid(Uuid::from_u128(14)),
            assignment: question_model::AssignmentId::from_uuid(Uuid::from_u128(15)),
            attempt: attempt.id,
            submission: claim().submission,
            actor: actor_id(),
            idempotency_key: SubmissionIdempotencyKey::parse("sealed-worker-key").expect("key"),
            request_sha256: objects::Sha256Digest::compute(b"response"),
            accepted_at: ActivityTimestamp::from_unix_millis(16),
        },
        response: StudentResponse::ShortText {
            text: "peptide".to_string(),
        },
        prepared: Box::new(PreparedQuestionSubmission {
            attempt,
            issued_question_snapshot: IssuedQuestionSnapshotV1::new(
                question,
                IssuedQuestionFamilyWitnessV1::Native {
                    physical_asset_bindings: Vec::new(),
                },
            )
            .expect("snapshot"),
            presentation_binding: None,
            presentation: None,
            grading_envelope: None,
            flat_grading: None,
            webwork_grading: None,
            issued_qti_grading: None,
            webwork_replay: None,
        }),
    }
}

fn grade_disposition() -> Result<SubmissionDisposition, RunBackendError> {
    Ok(SubmissionDisposition::Grade(
        crate::run::GradeReceipt::empty(question_model::AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 1.0,
        }),
    ))
}

type SealedWorkerFixture = (
    AcceptedSubmissionExecutionWorker<FakeStore, FakeBackend>,
    Arc<Mutex<Vec<(WorkerId, JobLeaseDuration)>>>,
    Arc<Mutex<Vec<AcceptedSubmissionExecutionClaim>>>,
    Arc<AtomicUsize>,
);

fn sealed_worker(
    claim_result: Result<Option<AcceptedSubmissionExecutionClaim>, StoreError>,
    commit_result: Result<AcceptedSubmissionExecutionDisposition, AcceptedSubmissionCommitError>,
) -> SealedWorkerFixture {
    let claim_requests = Arc::new(Mutex::new(Vec::new()));
    let load_claims = Arc::new(Mutex::new(Vec::new()));
    let submit_calls = Arc::new(AtomicUsize::new(0));
    let store = FakeStore {
        claim_result,
        claim_requests: Arc::clone(&claim_requests),
        load_claims: Arc::clone(&load_claims),
        execution: Arc::new(Mutex::new(Some(execution()))),
        load_error: None,
        commit_result,
        commits: Arc::new(Mutex::new(Vec::new())),
        cancellation_before_commit: None,
    };
    let backend = FakeBackend {
        result: grade_disposition(),
        submit_calls: Arc::clone(&submit_calls),
        verify_sealed_reconstruction: true,
    };
    let settings = WorkerSettings::new(60, EXECUTION_DEADLINE, 100).expect("worker settings");
    let worker = AcceptedSubmissionExecutionWorker::new(store, backend, claim().worker, settings)
        .expect("sealed worker");
    (worker, claim_requests, load_claims, submit_calls)
}

#[tokio::test]
async fn sealed_worker_claims_once_with_the_exact_process_and_execution_identity() {
    let expected_claim = claim();
    let (worker, claim_requests, load_claims, submit_calls) = sealed_worker(
        Ok(Some(expected_claim)),
        Ok(AcceptedSubmissionExecutionDisposition::Committed),
    );

    assert_eq!(worker.worker_id(), expected_claim.worker);
    assert_eq!(
        worker.drain_one().await.expect("sealed worker pass"),
        AcceptedSubmissionExecutionWorkerReport {
            committed: 1,
            ..AcceptedSubmissionExecutionWorkerReport::default()
        }
    );
    assert_eq!(
        claim_requests.lock().expect("claim requests").as_slice(),
        [(
            expected_claim.worker,
            JobLeaseDuration::from_seconds(60).expect("lease")
        )]
    );
    assert_eq!(
        load_claims.lock().expect("load claims").as_slice(),
        [expected_claim]
    );
    assert_eq!(submit_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn exact_fast_path_claim_uses_the_common_handler_and_commits_once() {
    let expected_claim = claim();
    let (worker, claim_requests, load_claims, submit_calls) = sealed_worker(
        Ok(Some(expected_claim)),
        Ok(AcceptedSubmissionExecutionDisposition::Committed),
    );

    assert_eq!(
        worker
            .execute_accepted_submission(target())
            .await
            .expect("exact fast path"),
        AcceptedSubmissionHandlerResult::Committed
    );
    assert_eq!(
        claim_requests.lock().expect("claim requests").as_slice(),
        [(
            expected_claim.worker,
            JobLeaseDuration::from_seconds(60).expect("lease")
        )]
    );
    assert_eq!(
        load_claims.lock().expect("load claims").as_slice(),
        [expected_claim]
    );
    assert_eq!(submit_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn exact_fast_path_claim_loss_is_answered_without_loading_or_grading() {
    let (worker, claim_requests, load_claims, submit_calls) = sealed_worker(
        Ok(None),
        Ok(AcceptedSubmissionExecutionDisposition::Committed),
    );

    assert_eq!(
        worker
            .execute_accepted_submission(target())
            .await
            .expect("exact fast path"),
        AcceptedSubmissionHandlerResult::ClaimNoLongerActive
    );
    assert_eq!(claim_requests.lock().expect("claim requests").len(), 1);
    assert!(load_claims.lock().expect("load claims").is_empty());
    assert_eq!(submit_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn sealed_worker_reports_an_empty_pass_without_loading_or_grading() {
    let (worker, claim_requests, load_claims, submit_calls) = sealed_worker(
        Ok(None),
        Ok(AcceptedSubmissionExecutionDisposition::Committed),
    );

    assert_eq!(
        worker.drain_one().await.expect("empty sealed pass"),
        AcceptedSubmissionExecutionWorkerReport {
            no_claim: 1,
            ..AcceptedSubmissionExecutionWorkerReport::default()
        }
    );
    assert_eq!(claim_requests.lock().expect("claim requests").len(), 1);
    assert!(load_claims.lock().expect("load claims").is_empty());
    assert_eq!(submit_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn sealed_worker_counts_each_durable_handler_disposition() {
    for (disposition, expected) in [
        (
            AcceptedSubmissionExecutionDisposition::Committed,
            AcceptedSubmissionExecutionWorkerReport {
                committed: 1,
                ..AcceptedSubmissionExecutionWorkerReport::default()
            },
        ),
        (
            AcceptedSubmissionExecutionDisposition::Rescheduled,
            AcceptedSubmissionExecutionWorkerReport {
                rescheduled: 1,
                ..AcceptedSubmissionExecutionWorkerReport::default()
            },
        ),
        (
            AcceptedSubmissionExecutionDisposition::Terminal,
            AcceptedSubmissionExecutionWorkerReport {
                terminal: 1,
                ..AcceptedSubmissionExecutionWorkerReport::default()
            },
        ),
        (
            AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive,
            AcceptedSubmissionExecutionWorkerReport {
                stale_claim: 1,
                ..AcceptedSubmissionExecutionWorkerReport::default()
            },
        ),
    ] {
        let (worker, claim_requests, load_claims, submit_calls) =
            sealed_worker(Ok(Some(claim())), Ok(disposition));
        assert_eq!(
            worker.drain_one().await.expect("sealed worker pass"),
            expected
        );
        assert_eq!(claim_requests.lock().expect("claim requests").len(), 1);
        assert_eq!(load_claims.lock().expect("load claims").len(), 1);
        assert_eq!(submit_calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn sealed_worker_counts_an_ambiguous_outcome_without_regrading() {
    let (worker, claim_requests, load_claims, submit_calls) = sealed_worker(
        Ok(Some(claim())),
        Err(AcceptedSubmissionCommitError::OutcomeUnknown),
    );

    assert_eq!(
        worker.drain_one().await.expect("sealed worker pass"),
        AcceptedSubmissionExecutionWorkerReport {
            outcome_unknown: 1,
            ..AcceptedSubmissionExecutionWorkerReport::default()
        }
    );
    assert_eq!(claim_requests.lock().expect("claim requests").len(), 1);
    assert_eq!(load_claims.lock().expect("load claims").len(), 1);
    assert_eq!(submit_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn sealed_worker_propagates_claim_and_known_handler_errors() {
    let (claim_error_worker, claim_requests, load_claims, submit_calls) = sealed_worker(
        Err(StoreError::Unavailable("claim unavailable".to_string())),
        Ok(AcceptedSubmissionExecutionDisposition::Committed),
    );
    assert!(matches!(
        claim_error_worker.drain_one().await,
        Err(StoreError::Unavailable(message)) if message == "claim unavailable"
    ));
    assert_eq!(claim_requests.lock().expect("claim requests").len(), 1);
    assert!(load_claims.lock().expect("load claims").is_empty());
    assert_eq!(submit_calls.load(Ordering::SeqCst), 0);

    let (handler_error_worker, claim_requests, load_claims, submit_calls) = sealed_worker(
        Ok(Some(claim())),
        Err(AcceptedSubmissionCommitError::Known(
            StoreError::Unavailable("outcome unavailable".to_string()),
        )),
    );
    assert!(matches!(
        handler_error_worker.drain_one().await,
        Err(StoreError::Unavailable(message)) if message == "outcome unavailable"
    ));
    assert_eq!(claim_requests.lock().expect("claim requests").len(), 1);
    assert_eq!(load_claims.lock().expect("load claims").len(), 1);
    assert_eq!(submit_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn graded_success_reconstructs_the_sealed_submission_and_commits_once() {
    let (handler, calls, commits) = handler(
        grade_disposition(),
        Ok(AcceptedSubmissionExecutionDisposition::Committed),
    );
    assert_eq!(
        handler.execute_claim(claim()).await.expect("handler"),
        AcceptedSubmissionHandlerResult::Committed
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        commits.lock().expect("commits").as_slice(),
        [AcceptedSubmissionExecutionOutcome::Evaluated { .. }]
    ));
}

#[tokio::test]
async fn deterministic_error_is_committed_without_a_retry() {
    let (handler, calls, commits) = handler(
        Err(RunBackendError::Deterministic(
            crate::run::DeterministicGraderFailure::Contract,
        )),
        Ok(AcceptedSubmissionExecutionDisposition::Terminal),
    );
    assert_eq!(
        handler.execute_claim(claim()).await.expect("handler"),
        AcceptedSubmissionHandlerResult::Terminal
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        commits.lock().expect("commits").as_slice(),
        [AcceptedSubmissionExecutionOutcome::DeterministicFailure {
            reason: GradingOperationReason::GraderContractFailure
        }]
    ));
}

#[cfg(feature = "e2e-grader-fault")]
#[tokio::test]
async fn feature_fault_backend_commits_the_closed_execution_exception_once() {
    let (prototype, _, commits) = handler(
        grade_disposition(),
        Ok(AcceptedSubmissionExecutionDisposition::Terminal),
    );
    let handler = AcceptedSubmissionExecutionHandler::new(
        prototype.store,
        crate::accepted_submission_worker::DeterministicGraderExceptionBackend,
        EXECUTION_DEADLINE,
    )
    .expect("positive deadline");
    assert_eq!(
        handler
            .execute_claim(claim())
            .await
            .expect("feature fault handler"),
        AcceptedSubmissionHandlerResult::Terminal
    );
    assert!(matches!(
        commits.lock().expect("commits").as_slice(),
        [AcceptedSubmissionExecutionOutcome::DeterministicFailure {
            reason: GradingOperationReason::GraderExecutionFailure
        }]
    ));
}

#[cfg(feature = "e2e-grader-fault")]
#[tokio::test]
async fn feature_fault_fast_path_leaves_durable_work_for_recovery_without_a_claim() {
    assert_eq!(
        RecoveryOnlyAcceptedSubmissionFastPath
            .execute_accepted_submission(target())
            .await
            .expect("recovery-only facade"),
        AcceptedSubmissionHandlerResult::RecoveryQueued
    );
}

#[tokio::test]
async fn unavailable_backend_becomes_one_transient_outcome() {
    let (handler, calls, commits) = handler(
        Err(RunBackendError::Unavailable("temporary".to_string())),
        Ok(AcceptedSubmissionExecutionDisposition::Rescheduled),
    );
    assert_eq!(
        handler.execute_claim(claim()).await.expect("handler"),
        AcceptedSubmissionHandlerResult::Rescheduled
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        commits.lock().expect("commits").as_slice(),
        [AcceptedSubmissionExecutionOutcome::TransientFailure]
    ));
}

#[tokio::test]
async fn unsupported_submission_is_committed_once_as_terminal() {
    let (handler, calls, commits) = handler(
        Err(RunBackendError::Unsupported(
            "no deterministic grader".to_string(),
        )),
        Ok(AcceptedSubmissionExecutionDisposition::Terminal),
    );
    assert_eq!(
        handler.execute_claim(claim()).await.expect("handler"),
        AcceptedSubmissionHandlerResult::Terminal
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        commits.lock().expect("commits").as_slice(),
        [AcceptedSubmissionExecutionOutcome::TerminalFailure]
    ));
}

#[tokio::test]
async fn load_claim_loss_does_not_invoke_the_backend() {
    let (handler, calls, commits) = handler(
        grade_disposition(),
        Ok(AcceptedSubmissionExecutionDisposition::Committed),
    );
    let handler = AcceptedSubmissionExecutionHandler::new(
        FakeStore {
            load_error: Some(AcceptedSubmissionExecutionLoadError::Conflict),
            ..handler.store
        },
        handler.backend,
        EXECUTION_DEADLINE,
    )
    .expect("positive deadline");
    assert_eq!(
        handler.execute_claim(claim()).await.expect("handler"),
        AcceptedSubmissionHandlerResult::ClaimNoLongerActive
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(commits.lock().expect("commits").is_empty());
}

#[tokio::test]
async fn issued_evidence_integrity_finalizes_the_active_lease_without_grading() {
    let (prototype, calls, commits) = handler(
        grade_disposition(),
        Ok(AcceptedSubmissionExecutionDisposition::Terminal),
    );
    let handler = AcceptedSubmissionExecutionHandler::new(
        FakeStore {
            load_error: Some(AcceptedSubmissionExecutionLoadError::IssuedEvidenceIntegrity),
            ..prototype.store
        },
        prototype.backend,
        EXECUTION_DEADLINE,
    )
    .expect("positive deadline");

    assert_eq!(
        handler
            .execute_claim(claim())
            .await
            .expect("integrity transition"),
        AcceptedSubmissionHandlerResult::Terminal
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(matches!(
        commits.lock().expect("commits").as_slice(),
        [AcceptedSubmissionExecutionOutcome::DeterministicFailure {
            reason: GradingOperationReason::IssuedEvidenceIntegrity
        }]
    ));
}

#[tokio::test]
async fn commit_side_claim_loss_is_returned_after_one_submission() {
    let (handler, calls, commits) = handler(
        grade_disposition(),
        Ok(AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive),
    );
    assert_eq!(
        handler.execute_claim(claim()).await.expect("handler"),
        AcceptedSubmissionHandlerResult::ClaimNoLongerActive
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        commits.lock().expect("commits").as_slice(),
        [AcceptedSubmissionExecutionOutcome::Evaluated { .. }]
    ));
}

#[tokio::test]
async fn known_commit_error_propagates_after_one_outcome_request() {
    let (handler, calls, commits) = handler(
        grade_disposition(),
        Err(AcceptedSubmissionCommitError::Known(
            StoreError::Unavailable("known commit failure".to_string()),
        )),
    );
    assert!(matches!(
        handler.execute_claim(claim()).await,
        Err(StoreError::Unavailable(message)) if message == "known commit failure"
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        commits.lock().expect("commits").as_slice(),
        [AcceptedSubmissionExecutionOutcome::Evaluated { .. }]
    ));
}

#[tokio::test]
async fn explicit_acknowledgement_ambiguity_returns_outcome_unknown() {
    let (handler, calls, commits) = handler(
        grade_disposition(),
        Err(AcceptedSubmissionCommitError::OutcomeUnknown),
    );
    assert_eq!(
        handler.execute_claim(claim()).await.expect("handler"),
        AcceptedSubmissionHandlerResult::OutcomeUnknown
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        commits.lock().expect("commits").as_slice(),
        [AcceptedSubmissionExecutionOutcome::Evaluated { .. }]
    ));
}

#[tokio::test(start_paused = true)]
async fn timeout_cancels_the_backend_before_one_timed_out_commit() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let submit_calls = Arc::new(AtomicUsize::new(0));
    let commits = Arc::new(Mutex::new(Vec::new()));
    let handler = AcceptedSubmissionExecutionHandler::new(
        FakeStore {
            claim_result: Ok(None),
            claim_requests: Arc::new(Mutex::new(Vec::new())),
            load_claims: Arc::new(Mutex::new(Vec::new())),
            execution: Arc::new(Mutex::new(Some(execution()))),
            load_error: None,
            commit_result: Ok(AcceptedSubmissionExecutionDisposition::Rescheduled),
            commits: Arc::clone(&commits),
            cancellation_before_commit: Some(Arc::clone(&cancelled)),
        },
        PendingBackend {
            cancelled: Arc::clone(&cancelled),
            submit_calls: Arc::clone(&submit_calls),
        },
        EXECUTION_DEADLINE,
    )
    .expect("positive deadline");
    let task = tokio::spawn(async move { handler.execute_claim(claim()).await });
    tokio::task::yield_now().await;
    tokio::time::advance(EXECUTION_DEADLINE).await;
    assert_eq!(
        task.await.expect("handler task").expect("handler"),
        AcceptedSubmissionHandlerResult::Rescheduled
    );
    assert!(cancelled.load(Ordering::SeqCst));
    assert_eq!(submit_calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        commits.lock().expect("commits").as_slice(),
        [AcceptedSubmissionExecutionOutcome::TimedOut]
    ));
}

#[test]
fn zero_execution_deadline_is_rejected() {
    assert!(matches!(
        AcceptedSubmissionExecutionHandler::new((), (), Duration::ZERO),
        Err(InvalidAcceptedSubmissionExecutionDeadline)
    ));
}
